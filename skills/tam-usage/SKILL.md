---
name: tam-usage
description: Use when the user wants to manage projects or tasks with the tam CLI tool, install tam, or troubleshoot tam commands. Also trigger when the user mentions project/task tracking, todo lists, lightweight terminal project managers, or the tam tool specifically.
---

# TAM Usage Guide

`tam` is a lightweight terminal project and task manager. All data is stored in an SQLite database under `~/.tam/`.

## Design Rationale

- **Project / Task hierarchy**: You must create a project first, then add tasks under it. Tasks cannot exist independently.
- **Current project mechanism**: `tam task` commands depend on a "current project" (set via `tam project use`). This avoids repeating the project name on every task operation.
- **Soft deletes**: Both `project` and `task` tables have an `is_deleted` field. Deletion updates `is_deleted = 1`, preserving historical data. Never run `DELETE FROM` directly.

## Installation

```bash
# Shell one-liner
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/wst7/tam/releases/download/latest/tam-installer.sh | sh

# PowerShell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/wst7/tam/releases/download/latest/tam-installer.ps1 | iex"

# Homebrew
brew install wst7/tap/tam

# npm
npm install -g @wst7/tam

# crates.io
cargo install tam
```

## CLI Command Reference

### Project Management (do this first)

| Command | Description | Alias |
|---------|-------------|-------|
| `tam project add <name>` | Create a project | `tam p a` |
| `tam project list` | List all projects | `tam p l` |
| `tam project use <name>` | **Set current project** (required before task commands) | `tam p u` |
| `tam project del <name>` | Delete a project (soft delete) | `tam p d` |
| `tam project current` | Show the current project | `tam p c` |
| `tam project edit <name> <new>` | Rename a project | `tam p e` |

### Task Management (requires a current project)

| Command | Description | Alias |
|---------|-------------|-------|
| `tam task add <title>` | Add a task | `tam t a` |
| `tam task list` | List tasks in the current project | `tam t l` |
| `tam task edit <id> <title>` | Edit a task title | `tam t e` |
| `tam task del <id>` | Delete a task (soft delete) | `tam t d` |
| `tam task start <id>` | Mark a task as "in progress" | — |
| `tam task done <id>` | Mark a task as "done" | — |

### Other

```bash
tam config   # Show current config (~/.tam/tam.config.toml)
tam -h       # Show help
```

## Data Model

```
project: id, name, created, updated, is_deleted
task:    id, title, content, status(0/1/2), project_id, created, updated, is_deleted
```

- `status`: 0 = Todo, 1 = In Progress, 2 = Done
- Both tables have `is_deleted BOOLEAN DEFAULT 0`. Deletion updates it to 1.
- Both tables have `updated` timestamp triggers that auto-refresh on UPDATE.

## Runtime Files

| File | Purpose |
|------|---------|
| `~/.tam/tam.config.toml` | Config: theme, current project, DB/log paths |
| `~/.tam/tam.db3` | SQLite database |
| `~/.tam/logs/tam.log` | Rolling logs (10 MB trigger, 5 backups) |

## Common Pitfalls

1. **Forgetting `project use`**: Almost all `tam task` commands require a current project to be set first, or they will error with `No current project set`.
2. **Hard deletes**: Do not manually run `DELETE FROM` against the SQLite database. tam uses soft deletes; direct deletion breaks data consistency.
3. **Corrupted config causes immediate exit**: If `~/.tam/tam.config.toml` is malformed, tam calls `exit(1)` immediately. Deleting the file lets tam regenerate a default config on next run.

## Complete Workflow Example

```bash
# 1. Install and verify version
tam -V

# 2. Create projects
tam project add website-redesign
tam project add backend-api

# 3. Switch to the project you want to work on
tam project use website-redesign

# 4. Add tasks
tam task add "Design homepage mockup"
tam task add "Write copy for landing page"
tam task add "Set up CI pipeline"

# 5. List tasks (shows the current project name)
tam task list

# 6. Start and complete a task
tam task start 1    # start task 1
tam task done 1     # mark task 1 as done

# 7. Switch projects and keep working
tam project use backend-api
tam task add "Implement auth middleware"
```
