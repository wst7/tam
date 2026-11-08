---
name: tam-agent
description: Use when an AI agent needs to read or mutate tam tasks through the machine interface: JSON CLI (`tam ... --json`, schema tam.agent/v1) or the MCP stdio server (`tam mcp`). Covers project selection, task filtering, atomic claims/leases, idempotent creation, events, and error/exit-code handling. Not for human-only TUI usage.
---

# TAM Agent Guide

`tam` exposes a stable machine contract for agents. Human CLI/TUI output is unchanged; use `--json` (or MCP) for everything below.

Single-file brief for pasting into an agent context: [`llms.txt`](../../llms.txt) at the repository root.

## 1. Pick the interface

| Interface | When |
|---|---|
| JSON CLI | shell / one-shot commands: `tam task list --project tam --json` |
| MCP stdio | MCP clients (Claude Code, Codex, …): `{"mcpServers":{"tam":{"command":"tam","args":["mcp"]}}}` |

All JSON responses share the envelope `{ok,schema:"tam.agent/v1",command,data,meta}` on stdout. Failures go to **stderr** with `{ok:false,...,error:{code,message,details}}`.

## 2. Context — read this first, especially with multiple projects

**A project is not always required, but if more than one project exists you must be explicit. Never assume the `current_project` is yours.**

Resolution order: `--project` > `TAM_PROJECT` > config `current_project`. Same for `--agent` > `TAM_AGENT`.

| Command type | Project required? | Multi-project rule |
|---|---|---|
| `task list/add`, `status`, `task next` (and MCP `tam_task_list/create/status/next`) | **Yes** — must resolve a project | Pass `--project NAME` (CLI) or `"project":"NAME"` (MCP) every time, or set `TAM_PROJECT` for the process |
| `task show/update/claim/heartbeat/release/done/del/events` (and MCP by-id tools) | No — task id is global | **Also pass `--project`/`project`**: it enables cross-project protection (`not_found`, exit 3) |
| `project list`, `project add` | No | Use `project list --json` to discover names |
| `tam project use` | — | Human convenience only. Do **not** rely on it for agents: it mutates one global config shared by every process |

No `--project`, no `TAM_PROJECT`, and no config current project → `missing_project` (exit 1). A project-scoped command never silently picks a random project.

Recommended patterns:

```bash
# One agent per project: process-scoped, no global mutation
TAM_PROJECT=tam tam task next --agent agent-a --claim --json

# One agent across projects: be explicit on every project-scoped call
tam task list --project tam --status todo --json
tam task claim 12 --project tam --agent agent-a --json
tam task done 12 --project tam --agent agent-a --json

# Discover names
tam project list --json
```

MCP: put `"project"` in every tool call

```json
{"name":"tam_task_next","arguments":{"project":"tam","agent":"agent-a","claim":true}}
```

or launch the server with `"env":{"TAM_PROJECT":"tam"}` for a single-project agent.

An explicit `--project`/`project` scopes by-id operations: a task from another project is `not_found` (exit 3). Without it, by-id commands keep the legacy global-id behavior. Without `--agent` you are a human caller (allowed to complete leased tasks).

Exit codes: `0` ok, `1` invalid/missing context/internal, `2` CLI usage, `3` not found, `4` conflict.

## 3. Minimal work loop

```bash
# 1. Where am I?
tam status --project tam --json
# {counts:{todo,in_progress,done},leased,unassigned,next:[TaskView]}

# 2. Get work (atomic claim optional)
tam task next --project tam --agent agent-a --claim --lease 900 --json
# data.task is null when nothing is claimable

# 3. Read details + history
tam task show 12 --json          # {task,events}

# 4. Work, then record progress
tam task update 12 --description "reproduced; fix in progress" --json
tam task heartbeat 12 --agent agent-a --lease 900 --json   # long jobs

# 5. Finish (or hand back)
tam task done 12 --agent agent-a --json
tam task release 12 --agent agent-a --json                 # returns it to todo

# 6. Abandon a task entirely (soft delete + `deleted` event)
tam task del 12 --json
```

Never claim by writing status directly: `claim` is the atomic lease operation and records an event.

## 4. Common operations

```bash
tam project add tam --json                     # {project,restored}
tam project list --json
tam task add "Fix login" --project tam --priority 2 --tag bug --tag backend --json
tam task add "Fix login" --project tam --idempotency-key retry-1 --json
# second call with retry-1 returns the same task id with deduplicated:true

tam task list --project tam --status todo --assigned me --priority 1 --json
tam task list --project tam --tag bug --search login --limit 20 --json
tam task events 12 --json

tam task claim 12 --agent agent-a --lease 900 --json          # exit 4 if leased by another agent
tam task claim 12 --agent agent-a --force --json              # preempt (recorded in event metadata)
tam task heartbeat 12 --agent agent-a --json                  # holder only
tam task release 12 --agent agent-a --json                    # non-holder needs --force
tam task done 12 --agent agent-a --json
tam task del 12 --json                                       # soft delete + deleted event
```

Lease guard: while another agent holds a valid lease, **any** `task update` from an agent conflicts (exit 4) — including title-only edits — so only the holder (or a human caller without `--agent`) may modify it. `--status todo`/`done` clears the lease, so the task becomes claimable again immediately.

Filters: `--status todo|in-progress|done`, `--assigned me|unassigned|<agent>`, `--priority 1..5`, `--tag` (repeatable, all must match), `--search` (title/description, case-insensitive), `--limit` (default 100, max 1000). Default order `created DESC`; `next` orders priority asc, then created asc.

## 5. TaskView

```json
{"id":12,"project_id":2,"project":"tam","title":"Fix login","description":"…",
 "status":"todo","priority":2,"tags":["bug","backend"],
 "assigned_agent":null,"lease_expires_at":null,"lease_expired":false,
 "revision":1,"created":"2026-11-08T09:00:00Z","updated":"2026-11-08T09:00:00Z"}
```

`revision` increases on every effective write; compare it to detect concurrent changes. `lease_expired` is only `true` when the task has an `assigned_agent` and its `lease_expires_at` is in the past; unassigned tasks always report `false`.

## 6. Error handling

```json
{"ok":false,"schema":"tam.agent/v1","command":"task.claim",
 "error":{"code":"conflict","message":"task 12 is leased by agent-b until 2026-11-08T10:00:00Z",
  "details":{"task_id":12,"assigned_agent":"agent-b"}}}
```

| code | exit | action |
|---|---|---|
| `invalid` | 1 | fix the request (blank title, priority out of 1..5, empty update, bad lease) |
| `missing_project` | 1 | pass `--project` or `TAM_PROJECT` |
| `missing_agent` | 1 | pass `--agent`/`TAM_AGENT` (claim/heartbeat/next --claim) |
| `not_found` | 3 | task/project gone or outside the explicit project |
| `conflict` | 4 | lease held elsewhere, already done, not the lease holder — re-read, wait for expiry, or use `--force` deliberately |
| `internal` | 1 | database/runtime problem; retry later |

## 7. MCP notes

- stdout is JSON-RPC only; logs go to `~/.tam/logs/tam.log`.
- `initialize` → `tools/list` → `tools/call`; 12 tools mirror the CLI operations.
- `tam_task_update` tags semantics: omit `tags` to keep them; pass `[]` or `""` to clear them (same as CLI `--tags ""`).
- Tool business failures use `result.isError=true` with the same error JSON; protocol errors use JSON-RPC codes (`-32700`, `-32601`, `-32602`).
- Long tasks: `tam_task_heartbeat` before `lease_seconds` elapses; otherwise another agent may claim after expiry.
