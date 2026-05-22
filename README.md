# TAM - Tasks Manager CLI Tool

[![Crate](https://img.shields.io/crates/v/tam.svg)](https://crates.io/crates/tam)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/wst7/tam/blob/main/LICENSE.md)
[![Release](https://github.com/wst7/tam/actions/workflows/release.yml/badge.svg)](https://github.com/wst7/tam/actions/workflows/release.yml)

A lightweight terminal-based project and task manager.

for the [task-tracker](https://roadmap.sh/projects/task-tracker) challenge from [roadmap.sh](https://roadmap.sh/).


<!-- ![Demo GIF](assets/demo.gif) -->

---

## Features
- 📁 Project Management: Add, edit, and delete projects
- ✅ Task Management: Add, edit, delete tasks and update their status (start, complete)
- 🖥 Dual Mode Support:
	* CLI Mode: Manage with command-line arguments
	* Interactive Mode: Use keyboard-driven terminal UI
- 🎨 Theme Support: Switch between light and dark themes in interactive mode

---

## Installation

### Shell
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/wst7/tam/releases/download/latest/tam-installer.sh | sh
```

### Powershell
```bash
powershell -ExecutionPolicy Bypass -c "irm https://github.com/wst7/tam/releases/download/latest/tam-installer.ps1 | iex"
```
### Homebrew
```bash
brew install wst7/tap/tam
```
### Npm
```bash
npm install -g @wst7/tam
```
### Crates.io
To install `tam` via [crates.io](https://crates.io/crates/tam):

```bash
cargo install tam
```

### GitHub
To install `tam` from the [GitHub releases](https://github.com/wst7/tam/releases)

---

## Usage

### Command mode
```bash
tam -h
```

```bash
A tasks manager cli tool
Version: 1.0.10

Usage: tam [OPTIONS] [COMMAND]

Commands:
  project  Manage projects, use `tam project --help` to see more options [aliases: p]
  task     Manage tasks, use `tam task --help` to see more options [aliases: t]
  config   Show tam configuration
  help     Print this message or the help of the given subcommand(s)

Options:
  -i, --interactive  Interactive mode
  -h, --help         Print help
  -V, --version      Print version

```

### Interactive mode
```bash
tam -i
```
---

## AI Assistant Skill

A usage skill is bundled in this repository to help AI assistants (Claude Code, Codex, OpenCode, Cursor, etc.) provide accurate `tam` command guidance.

### Install via `npx skills` (Recommended)

```bash
# Install from GitHub
npx skills add https://github.com/wst7/tam

```

### What's Included

The `tam-usage` skill covers:

- Installation methods (Shell, PowerShell, Homebrew, npm, cargo)
- Complete CLI command reference with aliases
- Project / Task management workflow
- Data model and soft-delete conventions
- Common pitfalls (e.g., forgetting `tam project use`)
- Troubleshooting tips

Once installed, your assistant will automatically load this skill when you ask questions about managing projects or tasks with `tam`.

---

## Contributing

Contributions are welcome! Please submit an issue or pull request on the [GitHub repository](https://github.com/wst7/tam).

---

## License

`tam` is licensed under the [MIT License](LICENSE).
