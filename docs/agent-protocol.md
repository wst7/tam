# tam Agent Protocol v1（设计契约）

> 目标：把 `tam` 从“人类 CLI + TUI”扩展为 **agent-first** 的任务管理系统，同时保持现有人类 CLI/TUI 向后兼容。
> 核心判断：agent 不需要更漂亮的表格，需要的是 **稳定的机器可读契约、显式上下文、原子认领、幂等写入、可审计状态**，以及一个 **零配置接入面**（MCP stdio + JSON CLI）。
> 状态：设计已定，待实现（Gate A）。本文是实现与验收的唯一契约；实现细节可偏离但行为不得偏离。

## 0. 设计原则

1. **人类模式不回归**：现有 `tam project/task/config` 的人类输出、别名、退出码尽量保持；TUI 继续可用。
2. **机器模式显式开启**：只有 `--json` 才输出 JSON；失败 JSON 走 stderr，成功走 stdout。
3. **上下文显式优先**：`--project`/`--agent` > `TAM_PROJECT`/`TAM_AGENT` > 配置文件 `current_project`。不再让全局 current_project 成为多 agent 的隐式竞争点。
4. **认领是原子的**：同一任务同一时刻只能有一个有效租约；并发 claim 必须一个成功、一个冲突，而不是双写。
5. **写入可重试**：创建支持 idempotency key；状态更新/认领返回 revision，便于 agent 判断是否发生并发修改。
6. **迁移可回滚**：只做 additive migration（加列/加表/加索引），首次迁移前自动备份；不物理删除历史数据。
7. **接入零依赖**：MCP server 走 stdio，不需要端口、token、守护进程；JSON CLI 可在任何 shell/agent 里用。

## 1. 兼容性与范围

| 项 | 决定 |
|---|---|
| 既有命令 | 全部保留；现有测试必须继续通过 |
| TUI | 保留；本轮不新增 agent 视图 |
| JSON | 新增全局 `--json`；未指定时人类输出不变 |
| 数据库 | 同一 SQLite 文件；additive migration；`PRAGMA user_version=1` |
| 新增依赖 | **不新增 crate**：`serde_json`、`chrono`、`rusqlite` 已存在 |
| 网络 | 不提供 HTTP；MCP stdio 为本轮唯一服务端接入 |
| 多用户/鉴权 | 不做；`agent` 是调用方自报身份，仅用于租约/审计，不是安全边界 |
| schema 破坏性变更 | 不做；不删列、不改既有列语义 |

## 2. 数据模型增量

既有 `task` 表新增列（全部带默认值，旧行自动获得默认值）：

| 列 | 类型 | 默认 | 语义 |
|---|---|---|---|
| `content` | TEXT | `''` | **复用**为任务描述/上下文（原字段一直未用） |
| `priority` | INTEGER | `3` | 1..5，**1 最高**；非法值拒绝 |
| `tags` | TEXT | `''` | 规范化为小写、去空格、去重、逗号分隔，如 `bug,backend` |
| `assigned_agent` | TEXT NULL | NULL | 当前租约持有者 |
| `lease_expires_at` | INTEGER NULL | NULL | Unix 秒；NULL 表示无租约 |
| `revision` | INTEGER | `1` | 每次有效写入 +1，供并发判断 |
| `idempotency_key` | TEXT NULL | NULL | 项目内唯一；用于创建重试 |

新表：

```sql
CREATE TABLE IF NOT EXISTS task_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    event TEXT NOT NULL,           -- created/updated/claimed/heartbeat/released/completed/reopened
    actor TEXT NOT NULL DEFAULT '',-- agent id 或 'human'
    from_status INTEGER,
    to_status INTEGER,
    metadata TEXT NOT NULL DEFAULT '{}',
    created DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

索引：

```sql
CREATE INDEX IF NOT EXISTS idx_task_project_status ON task(project_id, status, is_deleted);
CREATE INDEX IF NOT EXISTS idx_task_assignee ON task(assigned_agent);
CREATE UNIQUE INDEX IF NOT EXISTS idx_task_idempotency
  ON task(project_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL AND idempotency_key <> ''
    AND is_deleted = 0;   -- schema v2: keys are reusable after a soft delete
```

### 2.1 迁移规则

- 打开库时设置：`PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000; PRAGMA journal_mode=WAL;`
- `PRAGMA user_version` 目标为 **2**；迁移分两步、逐版本执行且可重入：
  1. **v0 → v1**：若 `tam.db3` 存在且非空，先 `wal_checkpoint(TRUNCATE)` 再复制为 `tam.db3.bak.v1`（已存在不覆盖）；逐列检查 `PRAGMA table_info(task)`，缺失才 `ALTER TABLE task ADD COLUMN ...`；创建 `task_events` 与 v1 索引（含旧版 idempotency 索引）。
  2. **v1 → v2**：`DROP INDEX IF EXISTS idx_task_idempotency;` 后按 §2 的 `is_deleted = 0` 谓词重建唯一索引。该步只替换索引、不新增备份。
  3. 设置 `PRAGMA user_version=2`。
- 真实 v0 库首次运行直接迁移到 v2；备份 `tam.db3.bak.v1` 是 **v0→v1 之前** 的快照（S1 阶段）；v1 库升级到 v2 不强制再备份。
- 迁移不得删除/改写既有行；旧任务默认 `priority=3`、`revision=1`、无租约。
- 迁移失败必须报错并保留原库/备份，不得静默继续。

## 3. JSON 契约（`tam.agent/v1`）

### 3.1 成功（stdout）

```json
{
  "ok": true,
  "schema": "tam.agent/v1",
  "command": "task.claim",
  "data": { "...": "command-specific" },
  "meta": { "project": "tam", "agent": "agent-a" }
}
```

### 3.2 失败（stderr）

```json
{
  "ok": false,
  "schema": "tam.agent/v1",
  "command": "task.claim",
  "error": {
    "code": "conflict",
    "message": "task 12 is leased by agent-b until 2026-11-08T10:00:00Z",
    "details": { "task_id": 12, "assigned_agent": "agent-b" }
  }
}
```

退出码：`0` 成功；`1` 校验/内部/缺上下文；`2` clap 用法错误；`3` not found；`4` conflict（租约/重复/版本冲突）。错误 code：`invalid`、`not_found`、`conflict`、`missing_project`、`missing_agent`、`internal`。

### 3.3 Task 视图（所有返回 task 的命令共用）

```json
{
  "id": 12,
  "project_id": 2,
  "project": "tam",
  "title": "Fix login",
  "description": "…",
  "status": "todo",              // todo | in-progress | done
  "priority": 2,
  "tags": ["bug", "backend"],
  "assigned_agent": "agent-a",
  "lease_expires_at": "2026-11-08T10:15:00Z",
  "lease_expired": false,        // 仅 assigned_agent 存在且 expires<=now 时为 true；未分配恒为 false
  "revision": 3,
  "created": "2026-11-08T09:00:00Z",
  "updated": "2026-11-08T09:30:00Z"
}
```

## 4. CLI 契约

全局参数（可放在子命令前后）：`--json`、`--project <name>`、`--agent <id>`。环境变量：`TAM_PROJECT`、`TAM_AGENT`。

### 4.1 读取

| 命令 | JSON data |
|---|---|
| `tam project list --json` | `[{id,name,created,updated}]` |
| `tam project current --json` | `{project:ProjectView}` 或 `{project:null}` |
| `tam project add <name> --json` | `{project:ProjectView,restored:bool}`；ProjectView 含 `id,name,created,updated` |
| `tam status [--project P] --json` | `{project,counts:{todo,in_progress,done},leased,unassigned,next:[TaskView]}` |
| `tam task list [filters] --json` | `{tasks:[TaskView],count,limit,truncated}` |
| `tam task show <id> --json` | `{task:TaskView,events:[EventView]}` |
| `tam task events <id> --json` | `{events:[EventView]}` |
| `tam task del <id> --json` | `{task:TaskView}`；软删（`is_deleted=1`、`revision+1`）并写 `deleted` 事件 |

`task list` filters：`--status todo\|in-progress\|done`、`--assigned <agent>\|me\|unassigned`、`--priority 1..5`、`--tag <tag>`（可重复）、`--search <text>`（title/description 子串，大小写不敏感）、`--limit N`（默认 100）。`--limit` 超出 1..1000 时按边界 clamp（不报错），`truncated` 表示是否被截断。默认排序保持 `created DESC`（与现有人类列表一致）。

### 4.2 写入

| 命令 | 行为 |
|---|---|
| `tam task add <title> [--description TEXT] [--priority 1..5] [--tag TAG]... [--idempotency-key KEY] --json` | 创建；同项目同 key 已存在时返回既有 task 且 `deduplicated:true` |
| `tam task update <id> [--title T] [--description D] [--status S] [--priority N] [--tags a,b] --json` | 局部更新；无字段则报 invalid；**agent 调用时，只要任务被其他 agent 的有效租约持有，任何字段的 update 都 conflict（租约完整性）**；`status` 变为 Done 时写 `completed` 事件；变为 Todo/Done 时清租约，仅 InProgress 保留租约 |
| `tam task claim <id> [--lease 900] [--force] --json` | 原子认领；成功置 `in-progress` + 租约；被他人有效租约占 → exit 4 |
| `tam task heartbeat <id> [--lease 900] --json` | 仅租约持有者可续租；否则 conflict |
| `tam task release <id> [--force] --json` | 释放租约；`in-progress` 回到 `todo`；非持有者需 `--force` |
| `tam task done <id> --json` | 完成并清租约；agent 调用时若被其他 agent 有效租约占 → conflict；人类调用（无 --agent）允许 |
| `tam task next [--claim] [--lease 900] --json` | 选优先级最高（priority 升序）+ 最早创建的可认领任务；`--claim` 时原子认领；无候选返回 `{task:null}` |
| `tam task del <id> --json` | 软删除任务并写 `deleted` 事件；返回 `{task:TaskView}`（含 `is_deleted` 前的字段） |

规则：
- `--lease` 默认 900 秒，范围 5..86400。
- 有效租约 = `lease_expires_at > now`。
- claim 允许：未分配 / 自己的旧租约 / 已过期租约；`--force` 可抢占他人有效租约（事件 metadata 记录 `force=true`）。
- 所有状态写入递增 `revision` 并写 `task_events`；`updated` 由既有触发器刷新。
- 空标题/空项目名继续拒绝；priority 非法拒绝；tags 为空串等价于清空。
- 租约守卫：agent（有 `--agent`）对他人有效租约的任务执行 `update`（含仅改 title/description）或 `done` 均 conflict；人类调用（无 `--agent`）不受此限制。
- 错误消息走 stderr；成功 stdout 仅一条 JSON。

### 4.3 上下文解析与多项目规则（agent 必读）

1. `--project`，否则 `TAM_PROJECT`，否则 config `current_project`。
2. `--agent`，否则 `TAM_AGENT`。claim/heartbeat/release/next --claim 必须有 agent；done/update 可选（缺省视为 human，actor=`human`）。
3. **多项目环境必须显式指定项目**。`current_project` 是一份全局可变配置，多个 agent 进程共享；不要假设它属于当前 agent。
4. 项目作用域规则：

| 命令类别 | 是否必须解析 project | 显式指定时 |
|---|---|---|
| `task list/add`、`status`、`task next`；MCP `tam_task_list/create/status/next` | **是**；否则 `missing_project` exit 1 | 使用该项目的任务 |
| `task show/update/claim/heartbeat/release/done/del/events`；MCP by-id 工具 | 否（task id 全局） | 校验任务属于该项目，跨项目 → `not_found` exit 3 |
| `project list/add` | 否 | `list` 返回全部活跃项目，`add` 创建新项目 |
| `tam project use` | — | 人类便利命令；agent 不应依赖（会改全局 current） |

5. 推荐：
   - 单 agent 固定项目：进程级 `TAM_PROJECT=<name>`（或 MCP 启动 env），不要改全局 `current_project`。
   - 单 agent 多项目：每次项目级调用都带 `--project <name>`；by-id 操作也建议带，以启用跨项目保护。
   - MCP：每次 `tools/call` 都传 `project`，或启动服务器时注入 `TAM_PROJECT`。
   - 不知道有哪些项目：`tam project list --json`。

```bash
TAM_PROJECT=tam tam task next --agent agent-a --claim --json
tam task list --project tam --status todo --json
tam task claim 12 --project tam --agent agent-a --json
```

```json
{"name":"tam_task_next","arguments":{"project":"tam","agent":"agent-a","claim":true}}
```

6. 未显式指定 project 时，by-id 操作保持旧全局 id 行为（兼容人类 CLI）；这是**兼容性例外**，多项目 agent 不应依赖。

## 5. MCP server（`tam mcp`）

- 传输：stdio，**换行分隔 JSON-RPC 2.0**；stdout 只输出协议消息（日志写文件）。
- 协议版本：`2024-11-05`；`initialize` 返回 `capabilities.tools`；处理 `notifications/initialized`、`ping`、`tools/list`、`tools/call`；未知方法返回 `-32601`。
- 工具（输入 JSON Schema，输出 `content:[{type:"text",text:"<json>"}]`）：

| 工具 | 参数 |
|---|---|
| `tam_status` | `project?` |
| `tam_project_list` | — |
| `tam_task_list` | `project?, status?, assigned?, priority?, tag?, search?, limit?` |
| `tam_task_get` | `id` |
| `tam_task_create` | `project?, title, description?, priority?, tags?, idempotency_key?` |
| `tam_task_update` | `id, title?, description?, status?, priority?, tags?` |
| `tam_task_claim` | `id, agent, lease_seconds?, force?` |
| `tam_task_heartbeat` | `id, agent, lease_seconds?` |
| `tam_task_release` | `id, agent?, force?` |
| `tam_task_complete` | `id, agent?` |
| `tam_task_next` | `agent, project?, lease_seconds?, claim?` |
| `tam_task_events` | `id` |

- 工具错误：`result.isError=true`，text 为 JSON 错误对象；不要把业务错误升级成 JSON-RPC 错误（除协议解析/未知方法）。
- 接入示例（Claude/Codex 等 MCP 客户端）：

```json
{ "mcpServers": { "tam": { "command": "tam", "args": ["mcp"] } } }
```

## 6. 接入文档与 skill

- `llms.txt`（仓库根）：**给 LLM 的单文件使用说明**，自包含项目上下文/JSON/租约/MCP/常见错误；README 说明如何把它交给 agent（粘贴、@ 引用、skill、URL、MCP）。
- `docs/agent-protocol.md`：本文件（协议/示例/迁移/限制）。
- `skills/tam-agent/SKILL.md`：给 agent 的最短工作流（status → next/claim → update/done → release），含 JSON 示例与错误处理。
- README 增加 “Agent integration” 与 “Give this guide to an agent” 两节（JSON + MCP 配置 + llms.txt 交付方式）。
- AGENTS.md 架构段补 `agent.rs`/`mcp.rs` 与测试入口。

## 7. 验收标准（实现必须全部满足）

1. 既有 30 个测试全绿；人类 CLI 输出与 TUI 不回归。
2. 迁移：旧库副本迁移后列/表/索引存在（`user_version=2`，idempotency 索引排除 `is_deleted=1`），数据行数不变；v0→v1 前生成 `tam.db3.bak.v1`；重复运行不报错；真实 `~/.tam` 全程不被测试触碰。
3. JSON：所有新增命令成功/失败 envelope 符合 §3；成功 stdout、错误 stderr；exit 0/1/2/3/4 符合 §3.2；配置损坏时 `--json` 的启动失败也输出 `command:"startup"` envelope（D5）。
4. 原子 claim：两个进程并发 claim 同一任务，恰好一个 exit 0、一个 exit 4；租约过期后可被另一 agent 认领；heartbeat 续租；release 后可再次 claim。
5. 幂等：同项目同 idempotency_key 两次 add 返回同一 task id，第二次 `deduplicated:true`；不同项目同 key 互不影响；任务软删后同 key 可再次创建新任务（不再撞 UNIQUE）。
6. 过滤/next：status/assigned/tag/priority/search/limit 组合正确；next 按 priority 升序 + created 升序，跳过 Done/有效租约。
7. 事件：created/claimed/heartbeat/released/completed/updated 按序可查；actor/from_status/to_status 正确。
8. 项目隔离：显式 `--project` 时跨项目 id 操作返回 not_found；不显式时保持旧行为。
9. MCP：initialize → tools/list → tools/call（create/list/claim/done）全链路通过；stdout 无人类文本；未知方法错误码正确。
10. 工程门：`cargo build` 0；`cargo test` 全绿；`cargo fmt --check` 0；`cargo clippy` 无 error 且 warnings ≤ 基线（15）；不新增依赖。

## 8. 明确不做（本轮）

- HTTP/REST/WebSocket server、远程多用户、鉴权。
- 任务依赖图（blocked_by）、SLA/到期提醒、通知推送。
- TUI 的 agent 视图、i18n、鼠标。
- 物理删除、schema 列改名、破坏性迁移。
- 分布式锁（SQLite 单文件 + WAL 足够本轮）。
