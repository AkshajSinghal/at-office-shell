use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DotfileWarning {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DotfileImport {
    pub path: String,
    pub commands: Vec<String>,
    pub warnings: Vec<DotfileWarning>,
}

pub fn import_dotfiles(home_dir: &Path) -> Result<Vec<DotfileImport>, io::Error> {
    let candidate_files = [
        home_dir.join(".bashrc"),
        home_dir.join(".bash_profile"),
        home_dir.join(".profile"),
        home_dir.join(".zshrc"),
        home_dir.join(".zprofile"),
    ];

    let mut imported = Vec::new();
    for path in candidate_files {
        if !path.exists() {
            continue;
        }

        let contents = fs::read_to_string(&path)?;
        let warnings = lint_dotfile(&path.to_string_lossy(), &contents);
        let commands = parse_commands(&contents);

        imported.push(DotfileImport {
            path: path.display().to_string(),
            commands,
            warnings,
        });
    }

    Ok(imported)
}

pub fn lint_dotfile(path: &str, contents: &str) -> Vec<DotfileWarning> {
    contents
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }

            let lowered = trimmed.to_ascii_lowercase();
            if lowered.starts_with("export ")
                || lowered.starts_with("alias ")
                || lowered.starts_with("source ")
                || lowered.starts_with("set -o")
            {
                Some(DotfileWarning {
                    message: format!(
                        "{}:{} uses shell syntax that may need a compatibility shim: {}",
                        path,
                        index + 1,
                        trimmed
                    ),
                })
            } else {
                None
            }
        })
        .collect()
}

fn parse_commands(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}
