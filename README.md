# TAM - Tasks Manager CLI Tool

![Crate](https://img.shields.io/crates/v/tam)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

`tam` is a lightweight and efficient CLI tool for managing tasks. Use it to create, update, track, and list your tasks directly from the terminal. 

for the [task-tracker](https://roadmap.sh/projects/task-tracker) challenge from [roadmap.sh](https://roadmap.sh/).


![Demo GIF](assets/demo.gif)

---

## Features
- [x] Add new tasks
- [x] Update existing tasks
- [x] Remove tasks
- [x] Mark tasks as done or in progress
- [x] List tasks by various statuses
- [x] Support multiple task processing
- [x] Interactive cli mode

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
tam <COMMAND> [OPTIONS]
```

#### Commands

| Command   | Description          |
|-----------|----------------------|
| `add`     | Add a new task       |
| `update`  | Update an existing task |
| `remove`  | Remove a task        |
| `done`    | Mark a task as done  |
| `start`   | Start a task         |
| `list`    | List tasks           |
| `help`    | Show help information |

#### Options
| Option           | Description            |
|------------------|------------------------|
| `-h, --help`     | Print help information |
| `-V, --version`  | Print version          |
| `-i, --interactive`  | Interactive mode          |

### Interactive mode
```bash
tam -i
```

---

## Examples

### Add a Task
```bash
tam add "Finish Rust project"
```

### Update a Task
```bash
tam edit task_id new_title

tam update 1 "Complete CLI tool documentation"
# or
tam edit 1  "Complete CLI tool documentation"
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