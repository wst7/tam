# TAM - Tasks Manager CLI Tool

![Crate](https://img.shields.io/crates/v/tam)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

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

### Crates.io
To install `tam` via [crates.io](https://crates.io):

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

## Contributing

Contributions are welcome! Please submit an issue or pull request on the [GitHub repository](https://github.com/wst7/tam).

---

## License

`tam` is licensed under the [MIT License](LICENSE).
