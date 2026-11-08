# Agent-first 实现说明（Gate A + Gate B 修复）

- 契约：`docs/agent-protocol.md`（实现与验收的唯一契约；本文记录实现细节）。
- 基础：在 Gate 4 接管改动之上增量实现；既有 30 个测试（`tests/cli.rs` 23 + `src/` 内联 7）全部保留并通过。
- 约束：不新增 crate/dev-deps；不做破坏性 schema；不写真实 `~/.tam`；不 commit/reset/stash。
- 结论：迁移（v0→v2）、JSON CLI、原子 claim/lease、事件日志、幂等创建、MCP stdio server 全部落地；Gate B 复核的 D1–D5、P3 与租约完整性（L1）已修复。当前 **62** 个测试全绿（内联 14 + `tests/agent.rs` 25 + `tests/cli.rs` 23）。

## 1. 模块与职责

| 模块 | 类型 | 职责 |
|---|---|---|
| `src/agent.rs`（新） | 契约层 | `AgentError`（code/exit_code/details）、`Ctx`（json/project/agent + env 解析）、JSON envelope、`TaskView`/`EventView`/`ProjectView`、时间格式 |
| `src/mcp.rs`（新） | 服务端 | `tam mcp` 换行 JSON-RPC 2.0：initialize/ping/tools/list/tools/call、12 个工具、stdout 只输出协议消息 |
| `src/commands/agent.rs`（新） | 操作层 | JSON CLI 与 MCP 共用的 op 函数（project/task/status）、上下文解析、`dispatch_json` |
| `src/db.rs`（改） | 持久化 | 打开连接时 `foreign_keys=ON` / `busy_timeout=5000` / `journal_mode=WAL`；`apply_migrations`（v0→v1→v2）与备份 |
| `src/sql.rs`（改） | SQL | 新列/事件表/索引/查询常量；`task` 全量投影统一为 `JOIN project` |
| `src/model.rs`（改） | 模型 | `Task` 增加 project_id/project/content/priority/tags/assigned_agent/lease_expires_at/revision；新增 `TaskEvent`；`TaskStatus` 增加 `as_str` |
| `src/services/task.rs`（改） | 服务 | 人类旧接口保留 + agent 操作：create/filter/update/claim/heartbeat/release/complete/next/events/status/delete；事务化 claim |
| `src/services/project.rs`（改） | 服务 | 项目 CRUD；`query_project` 返回完整 ProjectView（含 created/updated） |
| `src/cli.rs`（改） | CLI | 全局 `--json/--project/--agent`（global=true，子命令前后皆可）；`status`/`mcp`；task 新子命令与过滤参数 |
| `src/main.rs`（改） | 入口 | 先 parse 再 init；JSON 模式先 config 预检（D5）；MCP 优先分发；人类模式保持原输出 |
| `src/commands/task.rs`（改） | CLI 编排 | 人类模式新命令输出（纯文本），JSON 模式走 `commands/agent.rs` |
| `tests/agent.rs`（新） | 测试 | JSON/迁移/并发/租约/幂等/隔离/MCP/QA 缺口回归；TempHome 注入 HOME |
| `docs/agent-implementation.md`（本文） | 文档 | 实现说明、示例、回退、偏差、风险 |
| `skills/tam-agent/SKILL.md`（新） | 文档 | agent 最短工作流与错误处理 |
| `README.md` / `AGENTS.md` | 文档 | “Agent integration” 一节 / 架构与测试段增量 |

## 2. 数据库迁移（v0 → v1 → v2）

打开库时的 pragma（`db::get_db`）：

```sql
PRAGMA foreign_keys=ON;
PRAGMA busy_timeout=5000;
PRAGMA journal_mode=WAL;
```

`init_db()` 先执行原有 `CREATE TABLE/TRIGGER IF NOT EXISTS`（基础 DDL 常量逐字节未改），再调用 `apply_migrations(conn, Some(db_path))`；目标 `PRAGMA user_version = 2`：

1. 读 `PRAGMA user_version`；`>= 2` 直接返回（可重入）。
2. **v0 → v1**（仅当 version < 1）：若库文件存在且 `project`/`task` 有行，先 `PRAGMA wal_checkpoint(TRUNCATE)` 再复制为 `<db>.bak.v1`（已存在不覆盖；空库不备份）；逐列检查 `PRAGMA table_info(task)` 后 `ALTER TABLE task ADD COLUMN`（priority/tags/assigned_agent/lease_expires_at/revision/idempotency_key）；创建 `task_events` + v1 索引（含旧版 idempotency 索引）。
3. **v1 → v2**（仅当 version < 2）：`DROP INDEX IF EXISTS idx_task_idempotency;` 后按 `is_deleted = 0` 谓词重建唯一索引；该步只替换索引，不新增备份。
4. `PRAGMA user_version=2`。

`content` 列在 v0 已存在（历史遗留未用），迁移直接复用为任务描述，不新增列。

索引（v2）：

```sql
CREATE UNIQUE INDEX IF NOT EXISTS idx_task_idempotency
  ON task(project_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL AND idempotency_key <> ''
    AND is_deleted = 0;   -- 软删任务的 key 可复用（D1）
```

**迁移验证（真实库副本）**：`cp ~/.tam/tam.db3` 到临时 HOME 后运行，实测 v0→v2 一步到位：`project=5 行 / task=37 行` 行数不变、6 列全部出现、`task_events` + 4 索引、idempotency 索引含 `is_deleted = 0`、`tam.db3.bak.v1`（20480B，v0 形状：user_version=0 且无 priority 列）生成、二次运行幂等；旧项目/任务可读；迁移后 D1 行为验证（key 创建→软删→同 key 新建成功，id 38→39）。真实 `~/.tam` 两文件 sha256 全程未变。

**回退**：删除新库并用 `tam.db3.bak.v1` 覆盖即可回到 v0（迁移只增列/加表/换索引，不改旧列语义）；代码侧回退见 §10。

## 3. JSON 契约（tam.agent/v1）

成功（stdout，单行）：

```json
{"ok":true,"schema":"tam.agent/v1","command":"task.claim",
 "data":{"task":{"id":12,"project_id":2,"project":"tam","title":"Fix login",
   "description":"…","status":"in-progress","priority":2,"tags":["bug"],
   "assigned_agent":"agent-a","lease_expires_at":"2026-11-08T10:15:00Z",
   "lease_expired":false,"revision":3,
   "created":"2026-11-08T09:00:00Z","updated":"2026-11-08T09:30:00Z"}},
 "meta":{"project":"tam","agent":"agent-a"}}
```

失败（stderr，单行）：

```json
{"ok":false,"schema":"tam.agent/v1","command":"task.claim",
 "error":{"code":"conflict","message":"task 12 is leased by agent-b until 2026-11-08T10:00:00Z",
   "details":{"task_id":12,"assigned_agent":"agent-b"}}}
```

启动失败（D5，坏配置）：`{"ok":false,"schema":"tam.agent/v1","command":"startup","error":{"code":"internal",...}}` 到 stderr，stdout 为空，exit 1。

| code | exit | 触发 |
|---|---|---|
| `invalid` | 1 | 校验失败（空标题/priority 越界/update 无字段/非法 lease） |
| `missing_project` | 1 | 无法解析项目（无 --project/TAM_PROJECT/config current） |
| `missing_agent` | 1 | claim/heartbeat/next --claim 未提供 agent |
| `internal` | 1 | DB/初始化/配置损坏等内部错误 |
| `not_found` | 3 | 任务/项目不存在，或显式 `--project` 跨项目访问 |
| `conflict` | 4 | 有效租约冲突、重复完成、未持有租约却 heartbeat/release |
| clap 用法错误 | 2 | 参数/枚举/非数字 id（不输出 envelope） |

上下文解析顺序：`--project` > `TAM_PROJECT` > config `current_project`；`--agent` > `TAM_AGENT`；显式 `--project` 时按 id 的操作必须属于该项目，否则 `not_found`（未显式时保持旧全局 id 行为）。by-id 命令的 `meta.project` 由任务所属项目填充；未分配任务的 `lease_expired=false`。

## 4. CLI 增量

| 命令 | 行为 |
|---|---|
| `tam status [--project P] --json` | `{project,counts:{todo,in_progress,done},leased,unassigned,next:[TaskView]}` |
| `tam project list/current/add … --json` | 读取/创建项目；ProjectView 含 `id,name,created,updated`，add 额外 `restored` |
| `tam task add <title> [--description] [--priority 1..5] [--tag TAG]... [--idempotency-key K] --json` | 创建；同项目同 key（活跃行）返回既有 task 且 `deduplicated:true`；软删后同 key 创建新任务 |
| `tam task list [--status] [--assigned me\|unassigned\|AGENT] [--priority] [--tag]... [--search] [--limit] --json` | 过滤在服务层组合；默认 `created DESC`、limit 100；超界 clamp 到 1..1000，`truncated` 表示截断 |
| `tam task show <id> --json` | `{task,events}` |
| `tam task update <id> [--title] [--description] [--status] [--priority] [--tags a,b] --json` | 局部更新；`status` 变 Done 时按 done 租约规则守卫并写 `completed`；变 Todo/Done 清租约，仅 InProgress 保留 |
| `tam task claim <id> [--lease 900] [--force] --json` | 原子认领：`in-progress` + 租约 + revision+1 + `claimed` 事件 |
| `tam task heartbeat <id> [--lease] --json` | 仅持有者可续租，否则 conflict |
| `tam task release <id> [--force] --json` | 清租约；`in-progress`→`todo`；非持有者需 `--force` |
| `tam task done <id> --json` | 完成并清租约；agent 被其他有效租约占→conflict，人类（无 --agent）允许 |
| `tam task del <id> --json` | 事务内软删（`is_deleted=1`、`revision+1`）+ `deleted` 事件；返回 `{task}` |
| `tam task next [--claim] [--lease] --json` | priority 升序 + created 升序，跳过 Done/有效租约；`{task:null}` 表示无候选 |
| `tam task events <id> --json` | `{events:[EventView]}` |

人类模式（无 `--json`）：既有命令输出与 TUI 完全不变；新命令输出纯文本。`--project/--agent` 在人类模式同样可用。`task edit/start --json` 与 `project use/edit/del --json` 不支持（返回 invalid）。

## 5. 认领 / 租约 / 幂等 / 事件

- **有效租约** = `assigned_agent IS NOT NULL AND lease_expires_at > now`（now = Unix 秒）。
- **claim**：`BEGIN IMMEDIATE` 事务内读取任务→校验范围→校验租约（未分配/自己/已过期可认领；他人有效租约且无 `--force` → conflict；Done → conflict）→ UPDATE（`status=1`、租约、`revision+1`）→ 写 `claimed` 事件（metadata 含 lease_seconds/force/previous_agent）→ commit。并发 claim 依赖 SQLite 写锁 + `busy_timeout=5000`：恰好一个成功、一个 conflict（测试实测 1/1）。
- **heartbeat**：仅 `assigned_agent == agent` 可续租，否则 conflict；`revision+1` + `heartbeat` 事件。
- **release**：无租约→conflict；非持有者且无 `--force`→conflict；清租约、`in-progress`→`todo`、`revision+1` + `released` 事件。
- **done**：Done 再完成→conflict；agent 调用且他人有效租约→conflict；清租约、`status=2`、`revision+1` + `completed` 事件；actor 缺省为 `human`。
- **update**：至少一个字段；空标题/priority 非法拒绝。租约完整性：agent（actor≠human）且任务存在他人有效租约 → 任何 update 都 conflict（在写入前返回，revision 不变）；人类调用不受限。`status` 变为 Done 时事件写 `completed`（D2）；变为 Todo 或 Done 时清 `assigned_agent/lease_expires_at`，仅 InProgress 保留租约（D3）；其余写 `updated` 事件（metadata.fields）。
- **delete**：`BEGIN IMMEDIATE` 事务内软删、`revision+1`、写 `deleted` 事件；返回删除后的 TaskView。
- **幂等**：`task(project_id, idempotency_key) WHERE key<>'' AND is_deleted = 0` 唯一索引；插入前查活跃行、冲突后查，返回既有 task 且 `deduplicated:true`。不同项目同 key 互不影响；软删后同 key 可重新创建（D1）。
- **事件**：`task_events(id,task_id,project_id,event,actor,from_status,to_status,metadata,created)`，`created`/`updated`/`claimed`/`heartbeat`/`released`/`completed`/`deleted` 按 id 升序可查；`TaskEvent` 的 status 以整数存储、JSON 序列化为 `todo|in-progress|done`。
- 人类旧命令（add/start/edit/done/delete）也写 `created`/`updated`/`completed`/`deleted` 事件（actor=`human`），不改变原有 stdout 文案。

## 6. MCP server（`tam mcp`）

- stdio、换行分隔 JSON-RPC 2.0；`protocolVersion=2024-11-05`；stdout 仅协议消息（日志走 log4rs 文件），每条响应 `flush`。
- `initialize` → `capabilities.tools`；`notifications/initialized` 无响应；`ping`；`tools/list`；`tools/call`；未知方法 `-32601`；解析错误 `-32700`；缺 tool name `-32602`。
- 12 个工具：`tam_status`、`tam_project_list`、`tam_task_list`、`tam_task_get`、`tam_task_create`、`tam_task_update`、`tam_task_claim`、`tam_task_heartbeat`、`tam_task_release`、`tam_task_complete`、`tam_task_next`、`tam_task_events`（输入 JSON Schema，输出 `content:[{type:"text",text:"<json>"}]`）。
- `tam_task_update` 的 `tags`：字段缺失/`null` = 不改；显式 `[]` 或 `""` = 清空（D4，与 CLI `--tags ""` 一致）。
- 业务错误 → `result.isError=true` + 错误 JSON；协议错误才用 JSON-RPC error。
- 接入：`{"mcpServers":{"tam":{"command":"tam","args":["mcp"]}}}`。

## 7. Gate B（QA §7）修复

| # | 根因 | 修复 | 测试 |
|---|---|---|---|
| D1 | v1 partial unique index 未排除 `is_deleted=1`；查询只找活跃行，软删后同 key 插入撞唯一约束 → internal | schema v2：`DROP INDEX ... ; CREATE UNIQUE INDEX ... AND is_deleted = 0`；v0 首次运行直达 v2；v1→v2 只换索引不再备份 | 单元：`migrates_v1_to_v2_index_without_new_backup` + 迁移后 key 复用断言；集成：`d1_idempotency_key_reusable_after_soft_delete`（同 key 并发/重复仍 dedup） |
| D2 | `update --status done` 绕过 done 的租约守卫，事件记为 `updated` | `update_task` 中 agent（actor≠human）且他人有效租约 → 目标 Done 也 `lease_conflict`；非 Done→Done 事件写 `completed` 并清租约 | `d2_update_done_respects_lease_and_emits_completed_event`（b conflict 且 DB 不变；holder 成功；事件 completed） |
| L1 | 租约守卫只在 Done 生效：agent-b 改 title/`--status todo` 会成功并清掉 agent-a 的租约（违反 claim 互斥） | `update_task` 守卫扩展为「agent + 他人有效租约 → 任何 update 都 conflict」，位于 assigned/lease 计算之前；人类 actor 仍允许 | `agent_b_cannot_update_task_leased_by_agent_a`（title 与 todo 两种 update 均 exit4；title/status/assigned/lease/revision 不变；holder 与 human 仍可更新） |
| D3 | `update --status todo` 保留旧租约，`next` 跳过该任务 | Todo/Done 清 `assigned_agent/lease_expires_at`；仅 InProgress 保留 | `d3_update_todo_clears_lease_and_next_returns_task` |
| D4 | MCP 参数层把显式 `tags:[]`/`""` 归为“无字段”，无法清空 | 区分缺失（None）与显式空（Some(vec![])）；normalize 后为空串 | `d4_mcp_update_can_clear_tags`（`[]`、`""` 均可清空；仅 tags:[] 是有效字段） |
| D5 | JSON 模式在 config Lazy 初始化时输出人类 stderr 并 exit1 | main 在 `logs::init_logging()` 前、`ctx.json` 时调用 `config::preflight_load()`（复用 `Config::load`，不初始化 Lazy）；失败输出 `command:"startup"` envelope、exit1、stdout 空 | `d5_broken_config_json_uses_error_envelope` |
| P3 | project add/current 缺 created/updated；by-id `meta.project=null`；`task del --json` 不支持；未分配任务 `lease_expired=true` | `SELECT_PROJECT_SQL` 补时间列；by-id op 用 task.project 填 meta；新增 scoped soft delete + JSON dispatch + `deleted` 事件；`lease_expired` 仅在有 holder 且过期时为 true | `p3_project_view_timestamps_are_populated`、`p3_by_id_meta_project_is_populated`、`p3_task_delete_json_writes_event`、`task_crud_views_and_events`（lease_expired=false） |

红/绿证据：`agent-fix-red.log`（4 个 db 单测 + 8 个集成用例失败）→ `agent-fix-*.log`（绿 + 门禁）。

## 8. 测试与证据

| 类型 | 数量 | 内容 |
|---|---|---|
| 既有 cli 集成 | 23 | 人类 CLI/G1–G10 回归（未改文件） |
| 内联单测 | 14 | config 3、task 5（含 tags/lease）、project 1、db 迁移 5（v0→v2/列/索引/备份/幂等/内存/空库/v1→v2） |
| agent 集成 | 25 | envelope/exit 码、env 与显式上下文、CRUD/事件、过滤/next、并发 claim、租约过期、heartbeat/release、done 冲突、幂等（含软删复用）、项目隔离、status、真实库副本迁移、MCP 握手/12 工具/错误/purity、人类输出不变、D1–D5 + P3 + L1 回归 |
| **合计** | **62** | 全部通过（裸跑 / 隔离 HOME / `--test-threads=1` 均全绿） |

工程门：`cargo build` exit 0 / 4 warnings；`cargo fmt --check` exit 0；`cargo clippy` 0 error / 12 warnings（基线 15，无新增类别）；`Cargo.toml`/`Cargo.lock` 未改；JSON smoke 88/88、MCP smoke 45/45。租约修复轮证据：`agent-fix-lease.log`、`agent-fix-lease-sha256.txt`。

证据：`docs/forge-takeover/evidence/agent-*.log` 与 `agent-fix-*.log`（红/绿/迁移/JSON/MCP/门禁），真实 `~/.tam` sha256 与 `iteration-00-sha256-before.txt` 一致，`current_project=pastoid` 不变。

## 9. 偏差与残留风险

| 项 | 说明 |
|---|---|
| `project use/edit/del --json`、`task edit/start --json` 不支持 | 契约只要求已列命令；其余带 `--json` 返回 `invalid`（exit 1）。 |
| MCP `tam_task_get` 返回 `{task,events}` | 与 CLI `task show` 共用实现；契约未细化 tool 输出结构。 |
| `next` 非 `--claim` 时允许无 agent | 契约 §4.3 仅要求 `next --claim` 必须有 agent。 |
| 备份为文件复制 | v0→v1 前 `wal_checkpoint(TRUNCATE)` 后复制主库（未引入 backup feature）；v1→v2 不新增备份（仅换索引）。 |
| 历史带空格/空白数据 | 迁移不清理旧数据；查询路径对 name/title 做 trim 校验，历史空名行仍保留。 |
| 人类可操作他人租约任务 | 契约明确人类（无 `--agent`）不受租约守卫限制；若未来要求人类也只读，需要新增策略开关。 |
| `--limit` 越界 | 按 1..1000 clamp（不报错），文档已写明。 |
| 无 WAL 之外的分布式锁 | 单文件 SQLite + `BEGIN IMMEDIATE` + busy_timeout；多机/多用户不做。 |
| TUI 无 agent 视图 | 按契约 §8 明确不做；TUI 行为未回归由既有测试覆盖。 |
| 时间戳 1 秒分辨率 | `created/updated` 由 SQLite `CURRENT_TIMESTAMP` 产生；测试不依赖同秒内先后（next 排序用 id 兜底）。 |

## 10. 回退

- 代码：工作树未 commit；按文件从备份恢复即可。关键新增文件可整体删除：`src/agent.rs`、`src/mcp.rs`、`src/commands/agent.rs`、`tests/agent.rs`、`skills/tam-agent/`、`docs/agent-implementation.md`；其余为增量修改。
- 数据库：保留 `<db>.bak.v1`（v0 形状）；回退代码后如旧版本打开 v2 库，旧版本会无视新增列（`SELECT` 显式列）仍可读写，但新列/事件不会被旧版本维护；完全回退用备份覆盖。
- 真实用户数据：所有测试/验证均在临时 HOME；迁移测试只 `cp` 真实库副本，未写真实 `~/.tam`。

## 11. 本轮明确未做

HTTP/REST/WS、远程多用户与鉴权、任务依赖图、SLA/提醒、TUI agent 视图、i18n/鼠标、物理删除、schema 列改名、分布式锁、CI job。
