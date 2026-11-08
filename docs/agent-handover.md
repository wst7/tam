# tam Agent-First 重设计 · 交接（Gate C）

- 对象：`/Users/wstreet7/project/tam`（tam 1.0.13，单 crate，人类 CLI + TUI）
- 基线：HEAD `d395841288bbf86960a931924ff628c5b264d066`；agent-first 全部改动**未提交**
- 阶段：Gate A 设计/实现 → Gate B 独立回归（B1 发现 5 个 P2，B2 修复复验 PASS）→ Gate C 交接
- 状态：契约 §7 十条验收 **PASS**；**62 tests**；QA 独立 **773 条断言 0 FAIL**；真实 `~/.tam` 零写入
- 读法：先读[协议契约](agent-protocol.md)（唯一验收标准），再看本文的 quickstart / 迁移 / 回退 / 风险

## 1. 设计摘要：为什么 agent-first

agent 需要的不是更漂亮的表格，而是**稳定的机器契约**：单行 JSON、结构化错误、显式上下文、原子认领、可重试写入、可审计状态。

| 决策 | 做法 | 解决的问题 |
|---|---|---|
| 机器模式显式开启 | 仅 `--json` 输出 JSON；成功 stdout、失败 stderr | 人类输出零回归，脚本可稳定解析 |
| 两种接入面 | JSON CLI（任意 shell）+ `tam mcp`（stdio MCP） | 零配置：无端口、无 token、无守护进程 |
| 显式上下文 | `--project` > `TAM_PROJECT` > config；`--agent` > `TAM_AGENT` | 多 agent 不再隐式争抢全局 `current_project` |
| 原子认领 | `task claim`：`BEGIN IMMEDIATE` + 租约（默认 900s） | 同一任务同一时刻只有一个 owner，不会双写 |
| 可重试写入 | `--idempotency-key` + `revision` | 网络/进程重试返回同一任务；可检测并发修改 |
| 可审计 | 所有写入追加 `task_events`（actor/from/to/metadata） | 事后可回答「谁在何时改了什么」 |
| 迁移可回滚 | additive migration + 首次迁移前自动备份 + WAL | 旧库可升级，也可用 `.bak.v1` 回到 v0 |
| 零新依赖 | 复用 `serde_json`/`chrono`/`rusqlite` | `Cargo.toml`/`Cargo.lock` 未改 |

人类 CLI 命令、别名、表格输出与 TUI 全部保留（TUI 本轮不新增 agent 视图）。agent 身份是调用方自报，**不是安全边界**（无多用户/鉴权）。

## 2. 交付物索引（全部相对本目录；已 `test -f` 验证）

| 文档 | 行数 | 用途 |
|---|---:|---|
| [agent-protocol.md](agent-protocol.md) | 262 | 设计契约：JSON envelope、退出码、CLI/MCP、迁移、§7 十条验收 |
| [agent-implementation.md](agent-implementation.md) | 188 | 实现说明：模块、迁移、租约/幂等/事件、Gate B 修复、偏差 |
| [agent-regression.md](agent-regression.md) | 252 | 独立回归：B1 缺陷与 B2 复验、773 断言、证据索引、剩余风险 |
| 本文（agent-handover.md） | 243 | Gate C 交接：运行、迁移/回退、验收、路线 |

| 代码 / 测试 / 文档 | 行数 | 变更用途 |
|---|---:|---|
| [src/agent.rs](../src/agent.rs) | 333 | **新**：`AgentError`（code/exit）、`Ctx`、envelope、Task/Event/Project View |
| [src/mcp.rs](../src/mcp.rs) | 516 | **新**：`tam mcp` stdio JSON-RPC 2.0、12 工具、stdout 保纯 |
| [src/commands/agent.rs](../src/commands/agent.rs) | 441 | **新**：JSON CLI 与 MCP 共用 op、上下文解析、`dispatch_json` |
| [src/db.rs](../src/db.rs) | 439 | pragma（WAL/busy_timeout/foreign_keys）+ `apply_migrations` v0→v2 + 备份 |
| [src/services/task.rs](../src/services/task.rs) | 1000 | 租约/claim/heartbeat/release/done/update/next/events/过滤/幂等 |
| [src/sql.rs](../src/sql.rs) | 236 | 新列/事件表/4 索引/查询常量；基础 DDL 常量未改 |
| [src/model.rs](../src/model.rs) | 211 | `Task` 新字段、`TaskEvent`、`TaskStatus::as_str` |
| [src/cli.rs](../src/cli.rs) | 295 | 全局 `--json/--project/--agent`；`status`/`mcp`；新子命令与过滤参数 |
| [src/main.rs](../src/main.rs) | 137 | 先 parse 后 init；JSON config 预检（D5）；MCP 优先分发 |
| [src/commands/task.rs](../src/commands/task.rs) | 278 | 人类模式新命令文本输出；JSON 走 `commands/agent.rs` |
| [tests/agent.rs](../tests/agent.rs) | 1614 | **新（未跟踪）**：25 个 JSON/MCP/租约/迁移/幂等/隔离用例 |
| [skills/tam-agent/SKILL.md](../skills/tam-agent/SKILL.md) | 150 | **新**：agent 最短工作流、错误处理、MCP 注意点 |
| [llms.txt](../llms.txt) | 221 | **新**：给 LLM 的单文件使用说明（README 说明如何交付） |
| [dsh-plugin/README.md](../dsh-plugin/README.md) | 148 | **新**：DSH bundle `@wst7/dsh-tam`（14 个原生工具、左侧可视化面板、自动/回退安装、配置/测试） |
| [dsh-plugin/PUBLISHING.md](../dsh-plugin/PUBLISHING.md) | 162 | **新**：npm/tarball/CI 发版、tam 先行发布与版本兼容说明 |
| [README.md](../README.md) / [AGENTS.md](../AGENTS.md) | 274 / 87 | “Agent integration” / “Give this guide to an agent” / DSH plugin / 架构与测试段 |

既有 [tests/cli.rs](../tests/cli.rs)（731 行，23 用例）未改；人类代码路径仅增量。文档之外，本目录 `forge-takeover/evidence/agent-*.log` 为全部实拍证据；DSH 面板截图见 `dsh-tam-panel.png`，左侧入口见 `dsh-tam-entry.png`。

## 3. 分层变更

### 3.1 数据层：v0 → v1 → v2 迁移

打开连接即设 `PRAGMA foreign_keys=ON; busy_timeout=5000; journal_mode=WAL;`。`init_db()` 先跑原 DDL，再 `apply_migrations`：

1. **v0→v1**：库文件存在且有行时，先 `wal_checkpoint(TRUNCATE)`，复制为 `<db>.bak.v1`（已存在不覆盖，空库不备份）；逐列 `PRAGMA table_info(task)` 后 `ALTER TABLE task ADD COLUMN` 补 **6 列**（priority/tags/assigned_agent/lease_expires_at/revision/idempotency_key；`content` 复用为描述）；建 `task_events` 表与 v1 索引。
2. **v1→v2**：`DROP INDEX idx_task_idempotency` 后按 `is_deleted = 0` 谓词重建唯一索引（软删任务的 key 可复用，D1）；**只换索引、不新增备份**。
3. `PRAGMA user_version = 2`；可重入（`>=2` 直接返回），迁移不删/不改既有行，旧行默认 `priority=3`、`revision=1`、无租约。

4 个索引：`idx_task_events_task`、`idx_task_project_status`、`idx_task_assignee`、`idx_task_idempotency`（v2 谓词含 `is_deleted=0`）。基础 `CREATE TABLE/TRIGGER` 常量逐字节未改，属 additive。

### 3.2 JSON 契约（`tam.agent/v1`）

成功：`{"ok":true,"schema":"tam.agent/v1","command":"task.claim","data":{…},"meta":{"project":"tam","agent":"agent-a"}}` → stdout 单行。
失败：`{"ok":false,…,"error":{"code","message","details"}}` → stderr 单行，stdout 为空。启动失败（坏配置）也输出 `command:"startup"` envelope（D5）。

| code | exit | 触发 |
|---|---:|---|
| `invalid` | 1 | 校验失败（空标题、priority 越界、空 update、非法 lease） |
| `missing_project` / `missing_agent` | 1 | 无法解析项目 / 认领类命令缺 agent |
| `internal` | 1 | DB、初始化、配置损坏等 |
| clap 用法错误 | 2 | 参数/枚举/非数字 id（不输出 envelope） |
| `not_found` | 3 | 任务/项目不存在，或显式 `--project` 跨项目访问 |
| `conflict` | 4 | 有效租约冲突、重复完成、非持有者 heartbeat/release |

上下文：`--project` > `TAM_PROJECT` > config `current_project`；`--agent` > `TAM_AGENT`。显式 `--project` 时 by-id 操作必须属于该项目，否则 `not_found`；未显式时保持旧全局 id 行为。

### 3.3 CLI 增量与过滤

新增：`tam status`、`tam task show/update/claim/heartbeat/release/next/events`、`tam mcp`；`project list/current/add`、`task add/list/del` 支持 `--json`。
`task list` 过滤：`--status todo|in-progress|done`、`--assigned me|unassigned|<agent>`、`--priority 1..5`、`--tag`（可重复，AND）、`--search`（title/description，大小写不敏感）、`--limit`（默认 100，越界 clamp 到 1..1000，`truncated` 标记截断）；默认 `created DESC`。

### 3.4 租约、幂等、事件（核心语义）

- **有效租约** = `assigned_agent` 非空且 `lease_expires_at > now`（Unix 秒）。
- **claim**：事务内校验（未分配/自己/已过期可认领，他人有效租约或 Done → `conflict` 4，`--force` 可抢占并记录 `force=true/previous_agent`）→ 置 `in-progress` + 租约 + `revision+1` + `claimed` 事件；并发由 SQLite 写锁 + `busy_timeout` 保证恰好一个成功。
- **租约完整性（L1）**：agent 对他人有效租约任务执行**任何** `update`（含仅改 title）或 `done` 都 `conflict`，且不改变 revision/事件；人类（无 `--agent`）按契约不受限。`status` 变 Todo/Done 清租约，仅 InProgress 保留；非 Done→Done 写 `completed`（D2）。
- **heartbeat/release**：仅持有者可续租；release 清租约并把 `in-progress` 回 `todo`，非持有者需 `--force`。
- **幂等**：`(project_id, idempotency_key)` 唯一（仅活跃行）；重复创建返回同一 task 且 `deduplicated:true`；不同项目互不影响；软删后同 key 可新建（v2 索引）。
- **事件**：`created/updated/claimed/heartbeat/released/completed/deleted`，含 actor、from/to status、metadata（lease_seconds/force/previous_agent/fields），按 id 升序可查；人类旧命令也写事件（actor=`human`），stdout 文案不变。

### 3.5 MCP 与人类兼容

`tam mcp`：stdio、换行分隔 JSON-RPC 2.0、协议 `2024-11-05`；支持 `initialize`/`notifications/initialized`/`ping`/`tools/list`/`tools/call`，未知方法 `-32601`；**12 个工具** = status、project_list、task_list/get/create/update/claim/heartbeat/release/complete/next/events。业务错误走 `result.isError=true`，不污染 JSON-RPC 错误；stdout 只输出协议消息。
人类模式：无 `--json` 时输出与 TUI 完全不变；`-i`、别名、退出码保持。`--json` 可放在子命令前/中/后。

## 4. Quickstart

JSON CLI 最小工作流（临时 HOME 或真实数据均可；`--project` 可省略则取 config）：

```bash
# 1) 看板：计数 + 可认领候选
tam status --project tam --json

# 2) 原子认领下一个任务（无候选时 task=null）
tam task next --project tam --agent agent-a --claim --lease 900 --json

# 3) 读详情 + 事件历史
tam task show 12 --json

# 4) 记录进展 / 长任务续租
tam task update 12 --description "reproduced; fix in progress" --json
tam task heartbeat 12 --agent agent-a --lease 900 --json

# 5) 完成（或 release 交还）；6) 放弃则软删
tam task done 12 --agent agent-a --json
tam task release 12 --agent agent-a --json
tam task del 12 --json
```

MCP 配置片段（Claude/Codex 等客户端）：

```json
{ "mcpServers": { "tam": { "command": "tam", "args": ["mcp"] } } }
```

**多项目必读**：项目解析顺序是 `--project` > `TAM_PROJECT` > 全局 `current_project`。多 project / 多 agent 时不要依赖全局 current；项目级命令（`task list/add`、`status`、`task next`）必须显式带 `--project`（CLI）或 `"project"`（MCP），by-id 命令也建议带以启用跨项目 `not_found` 保护。单项目 agent 可用进程级 `TAM_PROJECT=tam` 或 MCP 启动 env，避免改全局配置。详见 [agent-protocol.md](agent-protocol.md) §4.3。

最小 agent 循环：`status` → `next --claim` → `show` → `update`/`heartbeat` → `done`（或 `release`）；**不要**直接写 status 绕过 claim（会丢原子性且没有租约事件）。完整 playbook 见 [skills/tam-agent/SKILL.md](../skills/tam-agent/SKILL.md)。

## 5. 迁移与回退

- **首次迁移**：用新版 tam 打开 v0 库会先备份 `~/.tam/tam.db3.bak.v1`（**v0 形状**：`user_version=0`、8 列无 priority、行数一致），再迁移到 `user_version=2`。备份已存在则不覆盖；空库不备份；重复运行幂等。
- **只读检查**：

```bash
sqlite3 "file:$HOME/.tam/tam.db3?mode=ro" "PRAGMA user_version;"
sqlite3 "file:$HOME/.tam/tam.db3?mode=ro" "PRAGMA table_info(task);"
sqlite3 "file:$HOME/.tam/tam.db3?mode=ro" "select name from sqlite_master where type='index' and name like 'idx_%';"
sqlite3 "file:$HOME/.tam/tam.db3?mode=ro" "select count(*) from task_events;"
```

- **备份 / 恢复**：先停所有 tam 实例；`cp ~/.tam/tam.db3 ~/.tam/tam.db3.bak.$(date +%Y%m%d)`；完全回退 v0 用 `.bak.v1` 覆盖主库（连同 `-wal`/`-shm` 一并清理）。
- **v1→v2 不重新备份**（只替换索引）；若需要 v1 形态快照，升级前自行 `cp`。
- **代码回退**：改动未 commit，HEAD `d395841` 即干净点。整体回退见 §9；新增文件可整体删除（`src/agent.rs`、`src/mcp.rs`、`src/commands/agent.rs`、`tests/agent.rs`、`skills/tam-agent/`、`docs/agent-*.md`），其余按文件 `git checkout -- <file>`。
- **`/tmp` 旧快照不可依赖**：本轮分步快照在 `/tmp/forge-takeover*` 或 `/tmp/forge-tam` 下，重启可能清空；长期审计用 [agent-final-changes.diff](forge-takeover/evidence/agent-final-changes.diff)（tracked 快照，**不含**未跟踪的新模块与 `tests/agent.rs`）。
- 旧版本打开 v2 库时能读写旧列（`SELECT` 显式列），但不会维护新列/事件——完全回退仍建议用备份。

## 6. 验证摘要（实拍）

| 门禁 | 结果 | 证据 |
|---|---|---|
| `cargo build` | exit 0，**4 warnings**（与基线同类） | [agent-regression-fix-build.log](forge-takeover/evidence/agent-regression-fix-build.log) |
| `cargo test` | **62 passed** = 14 内联 + 25 agent + 23 cli；默认/`--test-threads=1`/隔离 HOME 四种跑法全绿，0 ignored | [agent-regression-fix-test-bare.log](forge-takeover/evidence/agent-regression-fix-test-bare.log) |
| `cargo fmt --check` | exit 0 | [agent-regression-fix-fmt.log](forge-takeover/evidence/agent-regression-fix-fmt.log) |
| `cargo clippy` | 0 error / **12 warnings**（基线 15，无新类别） | [agent-regression-fix-clippy.log](forge-takeover/evidence/agent-regression-fix-clippy.log) |
| schema | additive：基础 DDL 未改，+`task_events`、+6 列、+4 索引；`DROP/RENAME/DELETE FROM` 0 | [agent-regression-quality.log](forge-takeover/evidence/agent-regression-quality.log) |
| 依赖 | `Cargo.toml`/`Cargo.lock` 0 改动 | [agent-regression-clippy-cargo.log](forge-takeover/evidence/agent-regression-clippy-cargo.log) |
| 真实用户数据 | sha256 与 `iteration-00` 逐字节一致；真实库仍 `user_version=0` | [agent-regression-fix-final-integrity.log](forge-takeover/evidence/agent-regression-fix-final-integrity.log) |
| QA 独立断言 | **773 条 0 FAIL**（JSON 391 + MCP 62 + 并发 70 + D1 48 + D2/D3/L1 89 + D4/D5/P3 113） | [agent-regression.md](agent-regression.md) §11/§13 |

## 7. 遗留问题与风险（不夸大）

| 项 | 级别 | 说明 |
|---|---|---|
| `--json` 覆盖不全 | P3 | `project use/edit/del` 与 `task edit/start` 带 `--json` 返回 `invalid`（契约未列入；`task del --json` 已实现） |
| 人类可操作他人租约任务 | 设计取舍 | 契约明确人类（无 `--agent`）不受租约守卫限制；若需只读需新增策略开关 |
| v1→v2 不重备份 | P3 | 只替换索引；v1 形态快照需升级前自行备份 |
| 单文件 SQLite 并发边界 | P2 风险 | 依赖 `BEGIN IMMEDIATE` + WAL + `busy_timeout`；多机/多用户/分布式锁不做 |
| 迁移测试环境依赖 | P3 | 隔离 HOME 无真实库时迁移用例静默 skip（CI 只跑隔离 HOME 会漏真实库迁移路径）；QA 已用真实库副本独立补测 |
| 跨平台未验证 | P2 未验证 | 仅 macOS；Windows/Linux 的 WAL、路径、终端未实测 |
| 真实 MCP 客户端 | P2 未验证 | 仅自写 JSON-RPC 客户端；未接 Claude/Codex，未测多会话/超长行/半包 |
| 迁移失败注入 | P2 未验证 | 磁盘满/只读库/损坏库未演练；备份恢复流程未实跑 |
| 长时 soak | P2 未验证 | 未做 900s 默认租约的多轮心跳/长期运行 |
| TUI 无 agent 视图 | 契约外 | 按 §8 明确不做；TUI 仅 pty 冒烟 |
| `--limit` clamp | 观察 | `0/1001` 被 clamp 为 `1/1000`，不报错（文档已写明） |
| `next --claim` 输家形态 | 观察 | 并发输家返回 `{task:null}`(exit 0) 或 `conflict`(exit 4)，契约未规定，两者均无双写 |

## 8. 后续路线（优先级）

**P1**

1. **事务/WAL 深化 + config 兼容**：把剩余多步写入纳入事务；旧配置缺字段用 serde default 合并、损坏时备份自愈。验收：迁移失败注入（磁盘满/只读）有明确错误且不破坏原库；旧配置可启动。
2. **CI 质量门**：`fmt --check`、`test`（隔离 HOME + 真实库副本迁移专用 job）、`clippy`（≤基线）三 job。验收：PR 全绿；迁移测试不再静默 skip。

**P2**

3. **TUI TestBackend 自动化 + agent 视图**：先用 TestBackend 覆盖 TUI 关键路径，再评估只读 agent 状态面板（租约/agent）。验收：进入 `cargo test`，无需 pty。
4. **依赖去重**：`tui-input 0.12.1` 拉入的 `ratatui 0.30.0-alpha.5` 与主依赖 0.29 并存。验收：`cargo tree -d` 无重复 ratatui。
5. **HTTP/远程鉴权（可选）**：若多机协作成为需求，再引入 MCP over HTTP + token；本轮明确不做。
6. **依赖图/SLA/通知**：`blocked_by`、到期提醒、webhook。验收：先定 schema 与事件语义，再实现。
7. **迁移失败演练**：磁盘满、只读库、半迁移恢复。验收：可自动判定「迁移失败 = 原库/备份完好」。

## 9. 采纳与提交

当前 HEAD `d395841`，**全部改动未提交**；18 个 tracked 文件已改，以下为未跟踪新增：`src/agent.rs`、`src/mcp.rs`、`src/commands/agent.rs`、`tests/`（`cli.rs`+`agent.rs`）、`skills/tam-agent/`、`docs/`。

```bash
cd /Users/wstreet7/project/tam
git status --short
git diff --stat
git diff -- src/                                   # 审阅生产代码
git add tests/agent.rs tests/cli.rs docs skills/tam-agent \
        src/agent.rs src/mcp.rs src/commands/agent.rs \
        src README.md AGENTS.md skills/tam-usage/SKILL.md
git commit -m "feat: agent-first machine contract (JSON CLI, MCP, atomic claims, schema v2)"
```

- 建议 `git add -p` 分批审；**不要** `git reset --hard` / `git stash` / 改写历史（HEAD 即回退点）。
- 文档可单独提交（`docs: add agent protocol, implementation and regression`）。
- 回退：`git checkout -- <18 个 tracked 文件>` + `rm -rf tests/agent.rs src/agent.rs src/mcp.rs src/commands/agent.rs skills/tam-agent docs/agent-*.md`（若整轮放弃）；数据库用 `.bak.v1`。

## 10. 用户数据安全声明

- 真实 `~/.tam` **全程未写入**：所有测试/验证注入临时 `HOME`，迁移测试只 `cp` 真实库副本。
- 真实库仍为 **v0**（`PRAGMA user_version=0`，无 `task_events`）；`current_project = "pastoid"` 未变。
- sha256 与基线逐字节一致：config `ad333f65…06a23`、db3 `85d4d8ef…2a223`（基线见 [iteration-00-sha256-before.txt](forge-takeover/evidence/iteration-00-sha256-before.txt)）。
- 25/25 agent 集成用例各自 `TempHome`，`env_clear` 注入 HOME/USERPROFILE/PATH/TZ；无 `#[ignore]`、`set_var`、硬编码真实 HOME。

## 11. 证据索引与复跑

```bash
# 工程门（全部安全：测试自建临时 HOME）
cargo build
cargo test
cargo test -- --test-threads=1
cargo fmt --check; echo "fmt_exit=$?"
cargo clippy 2>&1 | grep -c '^warning: '        # 12
# 隔离 HOME 视角（保留工具链变量）
REAL_HOME=$HOME
HOME=$(mktemp -d) RUSTUP_HOME="${RUSTUP_HOME:-$REAL_HOME/.rustup}" CARGO_HOME="${CARGO_HOME:-$REAL_HOME/.cargo}" cargo test
# 用户数据（只读）
shasum -a 256 ~/.tam/tam.config.toml ~/.tam/tam.db3
sqlite3 "file:$HOME/.tam/tam.db3?mode=ro" "PRAGMA user_version;"   # 真实库仍为 0
```

证据（`docs/forge-takeover/evidence/`）：构建器红/绿 `agent-{build,test,fmt,clippy,migration,json-smoke,mcp-smoke,schema,sha256}*`；Gate B 修复 `agent-fix-*`；QA 独立 `agent-regression-*`；本轮交接快照 [agent-final-changes.diff](forge-takeover/evidence/agent-final-changes.diff)（tracked 变更，3564 行/126484 字节，不含未跟踪新文件）与 [agent-final-git-status.log](forge-takeover/evidence/agent-final-git-status.log)（HEAD/分支/status/diff stat/行数/测试计数/哈希/链接检查实拍）。
