# TruShell

A general-purpose shell written in Rust. Like Bash and Zsh, but with a modern expression language and task management features planned.

Status: Alpha (actively developing)

---

## Quick start

### Install from source

```bash
$ git clone https://github.com/TruFoundation/TruShell.git
$ cd TruShell
$ cargo build --release
$ ./target/release/trushell
```

Or run directly for development:

```bash
$ cargo run
```

Notes:
- Rust 1.70+ is required (Edition 2021).
- If a prebuilt release is published in the future, you can download that instead of building locally.

---

## What is TruShell?

TruShell is a modern, extensible shell that:

- Runs standard Unix commands (ls, cat, grep, cd, etc.)
- Exposes a small expression language (variables, arithmetic, comparisons)
- Supports pipelines, redirects, and shell-style subprocess execution
- Includes a WASM plugin host for sandboxed extensions
- Is written in Rust for safety and performance

Think of TruShell as an approachable shell with cleaner syntax and an AST-based execution model.

---

## Highlights / Features

- Interactive REPL with history and familiar shell commands
- First-class variables declared with `let` and referenced with `$`
- Numeric units for readability (e.g. `1mb`, `500ms`)
- Pipes, redirects, and combination of stdout/stderr handling
- WASM-based plugin host with capability-style access
- Fallback to system command execution when a statement is not recognized

---

## Examples

Interactive use:

```bash
trushell> let name = "Alice"
trushell> let age = 30
trushell> let next_year = $age + 1
trushell> echo "Hello, $name! Next year you'll be $next_year."
```

Pipes and file operations:

```bash
trushell> cat server.log | grep "ERROR" > errors.txt
trushell> echo "Done" &> status.log
```

Code blocks and grouping:

```bash
trushell> let sum = { let a = 5; let b = 10; $a + $b }
```

---

## WASM plugin host

TruShell can load sandboxed WASM plugins. Plugins must include a JSON manifest (placed alongside the WASM module) that declares the plugin's name, version, API version, and the capabilities it requires.

Example manifest:

```json
{
  "name": "log-echo",
  "version": "0.1.0",
  "api_version": "1.0",
  "capabilities": ["logging"]
}
```

Supported capability examples:

- `logging` — allows the plugin to call a host logging function
- `environment-get` — allows the plugin to read environment variables via the host

Plugin examples live under `examples/plugins/` in the repository. A plugin that imports host functions it has not declared will fail to instantiate.

---

## Syntax overview

- Variables: `let x = 42` and reference as `$x`
- Strings: double-quoted literals `"text"`
- Numbers: integers and unit-suffixed numbers `1mb`, `500ms`
- Blocks: `{ ... }` return the value of the last expression inside
- Operators: arithmetic (`+ - * /`), comparisons (`== != > < >= <=`)

Operator precedence (lowest → highest): comparison, addition/subtraction, multiplication/division, primary (literals/identifiers/parentheses).

---

## Architecture

The interpreter follows a familiar pipeline:

1. Lexer: tokenizes input
2. Parser: builds an AST
3. Executor: runs the AST or falls back to system command execution
4. Output / side effects

Project layout (top-level):

```
TruShell/
├── src/
│   ├── main.rs         - REPL and command execution
│   └── parser.rs       - Lexer and parser
├── Cargo.toml          - Project manifest
├── Cargo.lock          - Dependency lock file
└── README.md           - This file
```

Design goals:
- Separation of concerns between lexing/parsing/execution
- Robust fallback to system commands when parsing fails
- Minimal dependencies for portability
- Extensible AST-based design to add language features safely

---

## Development

Requirements:

- Rust 1.70 or later
- Cargo (bundled with Rust)

Build & run:

```bash
# debug
cargo build
# release
cargo build --release
# run tests
cargo test
# enable backtraces while running
RUST_BACKTRACE=1 cargo run
```

Recommended workflow:

- Format: `cargo fmt`
- Lint: `cargo clippy`
- Add unit tests for new features and ensure CI passes

---

## Contributing

We welcome contributions. Suggested flow:

1. Fork the repository
2. Create a branch: `git checkout -b feature/your-feature`
3. Implement changes and add tests
4. Run `cargo fmt` and `cargo clippy`
5. Commit with a clear message and push your branch
6. Open a Pull Request describing the change and motivation

Please open an Issue to discuss larger design changes before implementing them.

---

## Roadmap

Planned work:

- Task Management (task create/list/complete)
- Time Tracking (time start/stop/log)
- Persistence via SQLite for tasks and history
- Configuration file: `~/.trushellrc`
- Command history and tab completion
- Shell integration helpers for `.bashrc` / `.zshrc`

---

## Support

Found a bug or have a question?

- Open an issue on GitHub
- Start a discussion if you want to brainstorm features

---

## License

TruShell is released under the terms found in LICENSE.md.

---

Maintainers: TruFoundation

Made with care — contributions and feedback welcome.
