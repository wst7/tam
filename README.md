# TAM - Tasks Manager CLI Tool

`tam` is a lightweight and efficient CLI tool for managing tasks. Use it to create, update, track, and list your tasks directly from the terminal.

---

## Features
- [x] Add new tasks
- [x] Update existing tasks
- [x] Remove tasks
- [x] Mark tasks as done or in progress
- [x] List tasks by various statuses
- [ ] Support multiple task processing

---

## Installation

To install `tam` via [crates.io](https://crates.io):

```bash
cargo install tam
```

---

## Usage

```bash
tam <COMMAND> [OPTIONS]
```

### Commands

| Command   | Description          |
|-----------|----------------------|
| `add`     | Add a new task       |
| `update`  | Update an existing task |
| `remove`  | Remove a task        |
| `done`    | Mark a task as done  |
| `start`   | Start a task         |
| `list`    | List tasks           |
| `help`    | Show help information |

### Options
| Option           | Description            |
|------------------|------------------------|
| `-h, --help`     | Print help information |
| `-V, --version`  | Print version          |

---

## Examples

### Add a Task
```bash
tam add "Finish Rust project"
```

### Update a Task
```bash
tam update 1 "Complete CLI tool documentation"
```

### Mark a Task as Done
```bash
tam done 1
```

### List All Tasks
```bash
tam list
```

---

## Contributing

Contributions are welcome! Please submit an issue or pull request on the [GitHub repository](https://github.com/wst7/tam).

---

## License

`tam` is licensed under the [MIT License](LICENSE).