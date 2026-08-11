mod dotfiles;
mod parser;
mod job_control;
mod terminal;
mod wasm_host;

use crate::dotfiles::import_dotfiles;
use crate::wasm_host::{load_plugin_manifest, WasmPlugin};
use std::io::{self, Write};

use reedline::{DefaultPrompt, Reedline, Signal};
use reedline::history::FileBackedHistory;
use std::path::PathBuf;

fn main() {
    println!("Welcome to TruShell Native Engine");

    // Initialize job control and signal handlers
    job_control::init_signal_handlers();
    if let Ok(home) = std::env::var("HOME") {
        match import_dotfiles(std::path::Path::new(&home)) {
            Ok(imports) => {
                for import in &imports {
                    if !import.commands.is_empty() {
                        println!("Imported {} ({} commands)", import.path, import.commands.len());
                    }
                    for warning in &import.warnings {
                        eprintln!("compat: {}", warning.message);
                    }
                }
            }
            Err(err) => eprintln!("dotfiles: failed to import startup files: {err}"),
        }
    }

    let mut terminal = terminal::Terminal::new(24, 80);

    // --- reedline setup ---
    // Use the same ANSI-colored prompt returned by terminal.prompt()
    let prompt_text = terminal.prompt();
    let prompt = DefaultPrompt::new(prompt_text.clone());
    let mut line_editor = Reedline::create();

    // Setup history file at $HOME/.trushell_history
    if let Ok(home) = std::env::var("HOME") {
        let hist_path = PathBuf::from(home).join(".trushell_history");
        if let Ok(history) = FileBackedHistory::with_file(1000, hist_path) {
            line_editor.set_history(Box::new(history));
        }
    }
    // -----------------------

    loop {
        match line_editor.read_line(&prompt) {
            Ok(Signal::Success(buffer)) => {
                let trimmed_input = buffer.trim();
                if trimmed_input.is_empty() {
                    continue;
                }

                if trimmed_input == "exit" {
                    println!("Goodbye!");
                    break;
                }

                let parts = split_posix_words(trimmed_input);
                if parts.first().map(String::as_str) == Some("cd") {
                    let new_dir = parts
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
                    if let Err(e) = std::env::set_current_dir(new_dir.as_str()) {
                        eprintln!("trushell: cd: {}: {}", new_dir, e);
                    }
                    continue;
                }

                if parts.first().map(String::as_str) == Some("plugin") {
                    match parts.get(1).map(String::as_str) {
                        Some("run") => {
                            let module_path = parts.get(2).cloned();
                            let input = parts.get(3).cloned().unwrap_or_default();
                            if let Some(path) = module_path {
                                match WasmPlugin::load(&path) {
                                    Ok(mut plugin) => match plugin.run(&input) {
                                        Ok(logs) => {
                                            for line in logs {
                                                println!("plugin: {line}");
                                            }
                                        }
                                        Err(err) => eprintln!("Plugin execution failed: {}", err),
                                    },
                                    Err(err) => eprintln!("Failed to load plugin: {}", err),
                                }
                            } else {
                                eprintln!("Usage: plugin run <module.wasm|module.wat> [input]");
                            }
                        }
                        Some("manifest") => {
                            if let Some(path) = parts.get(2) {
                                match load_plugin_manifest(path) {
                                    Ok(manifest) => {
                                        println!("Plugin: {}@{}", manifest.name, manifest.version);
                                        println!("API version: {}", manifest.api_version);
                                    }
                                    Err(err) => eprintln!("Failed to load plugin manifest: {}", err),
                                }
                            } else {
                                eprintln!("Usage: plugin manifest <manifest.json>");
                            }
                        }
                        _ => {
                            eprintln!("Unknown plugin command");
                        }
                    }
                    continue;
                }

                // Try to parse as TruShell AST -> execution (existing code path)
                match parser::parse_line(trimmed_input) {
                    Ok(ast) => {
                        if let Some((cmd, args)) = probable_cli_from_ast(&ast) {
                            execute_system_command(&cmd, &args);
                        } else {
                            // Placeholder: existing executor should handle AST execution here.
                            // For now we print the parsed AST (retain current behavior).
                            println!("Parsed AST: {:#?}", ast);
                        }
                    }
                    Err(_) => {
                        // Fallback to running as a system command if parsing fails
                        execute_system_command_from_input(trimmed_input);
                    }
                }
            }
            Ok(Signal::CtrlC) => {
                // interrupt — print a newline and continue
                println!();
                continue;
            }
            Ok(Signal::CtrlD) => {
                // EOF — exit cleanly
                println!();
                break;
            }
            Err(err) => {
                eprintln!("Input error: {}", err);
                break;
            }
        }
    }
}

// Heuristic: if AST is a chain of subtraction operations where the leftmost
// node is an identifier (the command) and the rest are identifiers or
// string-like literals, treat it as a CLI invocation and extract command+args.
fn probable_cli_from_ast(ast: &parser::ASTNode) -> Option<(String, Vec<String>)> {
    use parser::{ASTNode, BinaryOperator};

    fn collect_subtract_parts(node: &ASTNode, parts: &mut Vec<ASTNode>) -> bool {
        match node {
            ASTNode::BinaryOp { left, op, right } if *op == BinaryOperator::Subtract => {
                if !collect_subtract_parts(left, parts) {
                    return false;
                }
                parts.push((**right).clone());
                true
            }
            other => {
                parts.push(other.clone());
                true
            }
        }
    }

    let mut parts: Vec<ASTNode> = Vec::new();
    if !collect_subtract_parts(ast, &mut parts) {
        return None;
    }

    if parts.is_empty() {
        return None;
    }

    // first must be an identifier (command name)
    let cmd = match &parts[0] {
        ASTNode::Identifier(name) => name.clone(),
        _ => return None,
    };

    let mut args: Vec<String> = Vec::new();
    for part in parts.into_iter().skip(1) {
        match part {
            ASTNode::Identifier(s) => args.push(s),
            ASTNode::Literal(parser::Literal::String(s)) => args.push(s),
            _ => return None,
        }
    }

    Some((cmd, args))
}

fn split_posix_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut quote_mode: Option<char> = None;

    while let Some(ch) = chars.next() {
        match quote_mode {
            Some('"') => match ch {
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                '"' => quote_mode = None,
                _ => current.push(ch),
            },
            Some('\'') => {
                if ch == '\'' {
                    quote_mode = None;
                } else {
                    current.push(ch);
                }
            }
            None => match ch {
                '\'' => quote_mode = Some('\''),
                '"' => quote_mode = Some('"'),
                '\\' => {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                ch if ch.is_whitespace() => {
                    if !current.is_empty() {
                        words.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            },
            Some(other) => {
                current.push(other);
            }
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
}

fn execute_system_command(cmd: &str, args: &[String]) {
    let command_name = if cmd.is_empty() {
        return;
    } else {
        cmd
    };

    job_control::spawn_and_wait(command_name, args);
}

fn execute_system_command_from_input(input: &str) {
    let parts = split_posix_words(input);
    if parts.is_empty() {
        return;
    }

    let cmd = parts[0].clone();
    let args = parts.into_iter().skip(1).collect::<Vec<_>>();
    job_control::spawn_and_wait(&cmd, &args);
}
