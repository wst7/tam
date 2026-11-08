# @wst7/dsh-tam — DSH plugin for tam

A DeepSeek Harness **bundle** that exposes `tam` as 14 native model-facing tools. It wraps
the tam JSON CLI (`tam ... --json`, schema `tam.agent/v1`) and returns canonical task
data — the agent does not shell out or parse human tables.

The plugin does **not** embed the Rust binary. It runs a `tam` with the agent JSON
contract. When the plugin is installed from npm it pulls `@wst7/tam` as an optional
dependency (once tam 1.0.14+ is published), and cargo-dist's launcher downloads the
matching platform binary on first use. If that package is unavailable, the plugin falls
back to a local/PATH `tam`.

## Requirements

- DSH 0.2.x, Node.js >= 22.19 (the plugin uses ESM + `AbortSignal`).
- `tam` with agent JSON support: auto-installed via optional `@wst7/tam` (tam 1.0.14+),
  already on `PATH`, or pointed to by `command` (below). Verify:
  ```bash
  tam --json project list
  # must print one JSON envelope {"ok":true,"schema":"tam.agent/v1",...}
  ```
  The npm dependency is optional: if it is not yet published/resolvable, the plugin
  still installs and falls back to `PATH`/local build.
- In multi-project setups, pass `project` on project-scoped tool calls (same rule as the
  CLI/MCP contract).

## Install (users)

### From npm (recommended)

```bash
dsh plugin --profile desktop add @wst7/dsh-tam
# pin a version
dsh plugin --profile desktop add @wst7/dsh-tam@0.1.0
```

Restart DeepSeek Harness after installing (a running app does not hot-add a new bundle).
The tools then appear in the model tool list.

Once `@wst7/tam@>=1.0.14` is on npm, this also installs the CLI as an optional
dependency; the first tool call may download the platform binary. Offline/enterprise
users can set `command` to a preinstalled binary instead.

Upgrade / remove:

```bash
dsh plugin --profile desktop add @wst7/dsh-tam@latest
dsh plugin --profile desktop remove @wst7/dsh-tam
```

### From a release tarball

```bash
curl -L -o /tmp/wst7-dsh-tam-0.1.0.tgz \
  https://github.com/wst7/tam/releases/download/dsh-plugin-v0.1.0/wst7-dsh-tam-0.1.0.tgz
dsh plugin --profile desktop add /tmp/wst7-dsh-tam-0.1.0.tgz
```

`dsh plugin add <url>` also accepts a tarball URL directly.

### From source (this checkout)

```bash
cargo build                     # plugin auto-detects ./target/debug/tam
dsh plugin --profile desktop add ./dsh-plugin
```

### Verify

The Electron `desktop` profile is managed by the app, so `dsh --dump-config` refuses for
it; check the profile manifest instead:

```bash
grep -n '@wst7/dsh-tam' ~/.dsh/profiles/desktop/package.json
```

For a non-Electron profile (e.g. `headless`), `dsh --profile <name> --dump-config` shows
the `# == @wst7/dsh-tam` layer.

### Configure

Resolution order: `config.command` → `TAM_COMMAND` → bundled `@wst7/tam` (when
installed) → checkout `target/debug|release/tam` → `tam` on `PATH`. Default agent `dsh`,
no default project. Override in the profile's `cordis.patch.yml`:

```yaml
- id: tam-tools
  name: @wst7/dsh-tam
  config:
    command: "/absolute/path/to/tam"
    project: "tam"       # optional default project
    agent: "dsh"         # optional default agent identity
```

`config` replaces the bundle's `{}`, so include every key you need. For multiple
projects, leave `project` unset and pass `project` per tool call. `command` can also be
set through the `TAM_COMMAND` environment variable.

## Visual panel

After restarting DSH, the left sidebar has a **任务** (Tasks) entry at the bottom. It opens a
right-side panel that reads and writes the same task database:

- project selector, create project, refresh;
- task list for the selected project;
- create task, edit title / description / status / priority / tags, delete task;
- manual edits go through the same lease and revision rules as the CLI/tools (a task
  leased by another agent can reject an edit with `conflict`).

The panel is a small same-origin API (`/plugins/dsh-tam/api`) plus a prebuilt browser
bundle (`client.js`); it has no build step. The API and the model tools share one
`TamClient`, so both use the configured `command`, project, and agent.


## Tools

| Tool | Purpose |
|---|---|
| `tam_status` | counts, leases, unassigned work, next candidates |
| `tam_project_list` / `tam_project_add` | discover / create projects |
| `tam_task_list` | status/assigned/priority/tags/search/limit |
| `tam_task_show` | task + audit events |
| `tam_task_create` | title/description/priority/tags/idempotency key |
| `tam_task_update` | title/description/status/priority/tags (`tags: []` clears) |
| `tam_task_claim` / `tam_task_heartbeat` / `tam_task_release` | lease lifecycle |
| `tam_task_complete` / `tam_task_next` / `tam_task_events` / `tam_task_delete` | finish, next, audit, soft delete |

Business errors from tam are surfaced as DSH tool errors with tam's code/exit code
(`not_found`, `conflict`, etc.); the model never sees raw SQL errors.

## Development

```bash
cargo build
node --test dsh-plugin/test/*.test.mjs
```

The tests create an isolated temporary `HOME`, seed a database, and exercise the tool
specs against the real binary; the user's `~/.tam` is never touched.

`lib/core.js` is plain Node (runner + tool specs + argument validation); `index.js` only
registers raw `ToolDefinition`s on Cordis's injected `tools` service. There is no
`@deepseek-ai/dsh-tools` import, so a linked bundle resolves without DSH peer packages.

## Publishing

See [PUBLISHING.md](PUBLISHING.md) for maintainer release steps (npm / tarball / CI),
versioning, and the tam-binary prerequisite.
