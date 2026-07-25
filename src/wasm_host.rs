use serde::Deserialize;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use wasmtime::{Caller, Engine, Instance, Linker, Memory, Module, Store};

type HostResult<T> = Result<T, anyhow::Error>;

const SUPPORTED_PLUGIN_API_VERSION: &str = "1.0";
const INPUT_OFFSET: usize = 1024;
const MAX_INPUT_SIZE: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    Logging,
    EnvironmentGet,
}

#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub capabilities: Vec<Capability>,
}

impl PluginManifest {
    pub fn load_for_module(module_path: &Path) -> Result<Self, PluginError> {
        let manifest_path = module_path.with_extension("json");
        let raw = fs::read_to_string(&manifest_path)
            .map_err(|_| PluginError::ManifestMissing(manifest_path.clone()))?;
        let manifest: PluginManifest = serde_json::from_str(&raw)
            .map_err(|err| PluginError::InvalidManifest(err.to_string()))?;

        if manifest.api_version != SUPPORTED_PLUGIN_API_VERSION {
            return Err(PluginError::UnsupportedApiVersion(
                manifest.api_version.clone(),
            ));
        }

        Ok(manifest)
    }
}

pub struct WasmPlugin {
    manifest: PluginManifest,
    store: Store<PluginState>,
    instance: Instance,
}

impl WasmPlugin {
    pub fn load<P: AsRef<Path>>(module_path: P) -> Result<Self, PluginError> {
        let module_path = module_path.as_ref();
        let manifest = PluginManifest::load_for_module(module_path)?;
        let engine = Engine::default();
        let module_bytes = if module_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("wat"))
            .unwrap_or(false)
        {
            wat::parse_file(module_path).map_err(|err| PluginError::WasmCompile(err.to_string()))?
        } else {
            fs::read(module_path).map_err(|err| PluginError::WasmRead(err.to_string()))?
        };

        let module = Module::new(&engine, &module_bytes)
            .map_err(|err| PluginError::WasmCompile(err.to_string()))?;

        let state = PluginState::new(manifest.capabilities.iter().cloned().collect());
        let mut store = Store::new(&engine, state);
        let mut linker = Linker::new(&engine);

        register_host_functions(&mut linker, &manifest)?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|err| PluginError::Instantiation(err.to_string()))?;

        if instance.get_memory(&mut store, "memory").is_none() {
            return Err(PluginError::MissingMemory);
        }

        Ok(Self {
            manifest,
            store,
            instance,
        })
    }

    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    pub fn run(&mut self, input: &str) -> Result<Vec<String>, PluginError> {
        let input_bytes = input.as_bytes();
        if input_bytes.len() > MAX_INPUT_SIZE {
            return Err(PluginError::InputTooLarge(input_bytes.len()));
        }

        let memory = self
            .instance
            .get_memory(&mut self.store, "memory")
            .ok_or(PluginError::MissingMemory)?;

        memory
            .write(&mut self.store, INPUT_OFFSET, input_bytes)
            .map_err(|err| PluginError::MemoryWrite(err.to_string()))?;

        self.store.data_mut().logs.clear();
        let run_func = self
            .instance
            .get_typed_func::<(i32, i32), i32>(&mut self.store, "run")
            .map_err(|_| PluginError::MissingExport("run".to_string()))?;

        run_func
            .call(&mut self.store, (INPUT_OFFSET as i32, input_bytes.len() as i32))
            .map_err(|err| PluginError::PluginTrap(err.to_string()))?;

        Ok(self.store.data().logs.clone())
    }
}

pub fn load_plugin_manifest<P: AsRef<Path>>(module_path: P) -> Result<PluginManifest, PluginError> {
    PluginManifest::load_for_module(module_path.as_ref())
}

fn register_host_functions(linker: &mut Linker<PluginState>, manifest: &PluginManifest) -> Result<(), PluginError> {
    if manifest.capabilities.contains(&Capability::Logging) {
        linker
            .func_wrap("env", "host_log", |mut caller: Caller<'_, PluginState>, ptr: i32, len: i32| {
                let message = read_guest_string(&mut caller, ptr, len)?;
                caller.data_mut().logs.push(message);
                Ok(())
            })
            .map_err(|err| PluginError::HostLink(err.to_string()))?;
    }

    if manifest.capabilities.contains(&Capability::EnvironmentGet) {
        linker
            .func_wrap(
                "env",
                "host_get_env",
                |mut caller: Caller<'_, PluginState>, key_ptr: i32, key_len: i32, out_ptr: i32, out_len: i32| {
                    let key = read_guest_string(&mut caller, key_ptr, key_len)?;
                    let value = std::env::var(&key).unwrap_or_default();
                    let bytes = value.as_bytes();
                    let bytes_to_write = std::cmp::min(bytes.len(), out_len as usize);
                    write_guest_bytes(&mut caller, out_ptr, &bytes[..bytes_to_write])?;
                    Ok(bytes_to_write as i32)
                },
            )
            .map_err(|err| PluginError::HostLink(err.to_string()))?;
    }

    Ok(())
}

fn read_guest_string(caller: &mut Caller<'_, PluginState>, ptr: i32, len: i32) -> HostResult<String> {
    let memory = guest_memory(caller)?;
    let ptr = ptr
        .try_into()
        .map_err(|_| anyhow::anyhow!("negative pointer passed from plugin"))?;
    let len = len
        .try_into()
        .map_err(|_| anyhow::anyhow!("negative string length passed from plugin"))?;

    let mut buffer = vec![0u8; len];
    memory
        .read(caller, ptr, &mut buffer)
        .map_err(|_| anyhow::anyhow!("failed to read memory from plugin"))?;
    String::from_utf8(buffer).map_err(|_| anyhow::anyhow!("plugin string is not valid UTF-8"))
}

fn write_guest_bytes(caller: &mut Caller<'_, PluginState>, ptr: i32, bytes: &[u8]) -> HostResult<()> {
    let memory = guest_memory(caller)?;
    let ptr = ptr
        .try_into()
        .map_err(|_| anyhow::anyhow!("negative pointer passed from plugin"))?;
    memory
        .write(caller, ptr, bytes)
        .map_err(|_| anyhow::anyhow!("failed to write memory to plugin"))
}

fn guest_memory(caller: &mut Caller<'_, PluginState>) -> HostResult<Memory> {
    caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or_else(|| anyhow::anyhow!("failed to find exported memory"))
}

#[derive(Debug)]
pub enum PluginError {
    ManifestMissing(PathBuf),
    InvalidManifest(String),
    UnsupportedApiVersion(String),
    WasmRead(String),
    WasmCompile(String),
    Instantiation(String),
    HostLink(String),
    MissingMemory,
    MissingExport(String),
    MemoryWrite(String),
    PluginTrap(String),
    InputTooLarge(usize),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::ManifestMissing(path) => {
                write!(f, "Plugin manifest missing: {}", path.display())
            }
            PluginError::InvalidManifest(err) => write!(f, "Invalid plugin manifest: {}", err),
            PluginError::UnsupportedApiVersion(version) => {
                write!(f, "Unsupported plugin API version: {}", version)
            }
            PluginError::WasmRead(err) => write!(f, "Failed to read plugin module: {}", err),
            PluginError::WasmCompile(err) => write!(f, "Failed to compile WASM plugin: {}", err),
            PluginError::Instantiation(err) => write!(f, "Failed to instantiate plugin: {}", err),
            PluginError::HostLink(err) => write!(f, "Failed to link host functions: {}", err),
            PluginError::MissingMemory => write!(f, "Plugin module does not export linear memory named 'memory'"),
            PluginError::MissingExport(export) => write!(f, "Plugin module missing required export: {}", export),
            PluginError::MemoryWrite(err) => write!(f, "Failed to write plugin memory: {}", err),
            PluginError::PluginTrap(err) => write!(f, "Plugin trap: {}", err),
            PluginError::InputTooLarge(size) => write!(f, "Plugin input is too large: {} bytes", size),
        }
    }
}

impl std::error::Error for PluginError {}

#[derive(Debug)]
struct PluginState {
    pub capabilities: HashSet<Capability>,
    pub logs: Vec<String>,
}

impl PluginState {
    fn new(capabilities: HashSet<Capability>) -> Self {
        Self {
            capabilities,
            logs: Vec::new(),
        }
    }
}
