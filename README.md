# TAM - Tasks Manager CLI Tool

`tam` is a lightweight and efficient CLI tool for managing tasks. Use it to create, update, track, and list your tasks directly from the terminal.

---

## Features
- Add new tasks
- Update existing tasks
- Remove tasks
- Mark tasks as done or in progress
- List tasks by various statuses

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

Contributions are welcome! Please submit an issue or pull request on the [GitHub repository](https://github.com/your_username/tam).

---

## License

`tam` is licensed under the [MIT License](LICENSE).