use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use trushell::dotfiles::{import_dotfiles, lint_dotfile, DotfileImport};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

fn unique_temp_dir() -> PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!("trushell-dotfiles-{id}-{}", std::process::id()));
    fs::create_dir_all(&base).unwrap();
    base
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

#[test]
fn lints_common_bash_and_zsh_syntax() {
    let warnings = lint_dotfile("/tmp/example.bashrc", "export PATH=\"$HOME/bin:$PATH\"\nalias ll='ls -la'\nsource ~/.bashrc\nset -o vi\n");

    let messages: Vec<_> = warnings.iter().map(|warning| warning.message.clone()).collect();
    assert!(messages.iter().any(|message| message.contains("export")));
    assert!(messages.iter().any(|message| message.contains("alias")));
    assert!(messages.iter().any(|message| message.contains("source")));
    assert!(messages.iter().any(|message| message.contains("set -o")));
}

#[test]
fn imports_commands_from_startup_files() {
    let home = unique_temp_dir();
    write_file(&home.join(".bashrc"), "echo loaded\n# comment\nexport PATH=$PATH\n");

    let imports = import_dotfiles(&home).unwrap();
    let bashrc_import: Option<&DotfileImport> = imports.iter().find(|import| import.path.ends_with(".bashrc"));

    let import = bashrc_import.expect("expected .bashrc import");
    assert_eq!(import.commands, vec!["echo loaded".to_string(), "export PATH=$PATH".to_string()]);
    assert!(import.warnings.iter().any(|warning| warning.message.contains("export")));
}
