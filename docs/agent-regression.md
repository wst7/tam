# Agent-first 独立回归（Gate B）

> 验证者：qa（独立复核，不采信 builder 的 `agent-*.log` 结论）｜2026-11-08 17:26–17:39 CST｜仓库 `/Users/wstreet7/project/tam`
> 契约：`docs/agent-protocol.md` §7 十条（唯一验收标准）；实现说明：`docs/agent-implementation.md`。
> 基线：HEAD `d395841`（工作树含 Gate 3/4/5 + agent-first 未提交改动，18 modified + 新模块未跟踪）。
> 硬约束遵守：所有 tam 运行注入临时 HOME；迁移只 `cp` 真实库副本；真实 `~/.tam` 前后 sha256 逐字节一致；只写本文件 + `evidence/agent-regression-*.log`（16 个）；未改源码/tests/docs/README/AGENTS，未 git commit/reset/stash，未新增依赖。

## 1. 环境与版本

| 项 | 值 | 证据 |
|---|---|---|
| HEAD | `d395841288bbf86960a931924ff628c5b264d066` | `git rev-parse HEAD` |
| 工作树 | 18 files changed, +2321/−334；未跟踪 `src/agent.rs`(331)、`src/mcp.rs`(507)、`src/commands/agent.rs`(431)、`tests/agent.rs`(1129)、`skills/tam-agent/`、`docs/agent-*.md` | `git status` |
| 工具链 | rustc 1.94.1 / cargo 1.94.1（macOS） | 本次实测 |
| 产品版本 | tam 1.0.13；二进制 `target/debug/tam` | `-V` |
| 真实用户库 | `~/.tam/tam.db3` 20480B、schema v0（QA 未迁移）；config `current_project="pastoid"` | 只读 `PRAGMA user_version` |
| 用户数据基线 | `ad333f65…06a23`（config）、`85d4d8ef…2a223`（db3） | `evidence/iteration-00-sha256-before.txt` |

## 2. 方法

1. 先读契约、实现说明、README「Agent integration」、AGENTS 增量、SKILL、`src/{agent,mcp,db,cli,main,commands/agent,services/task}.rs`、`tests/agent.rs`，再按契约独立设计断言。（B1 时版本：protocol 231 行 / impl 161 行 / agent tests 1129 行；B2 修复版：235 / 188 / 1614。）
2. 自写 6 组独立脚本（Python，每脚本自行 `mktemp` HOME，子进程显式 `HOME/TZ/PATH`，绝不继承真实 HOME）：
   JSON CLI 391 断言 + 补充 13 + 幂等/status/limit 21；迁移 40；并发/租约 70；MCP 61；缺口对抗 63+12。
3. 每组前后 `shasum -a 256 ~/.tam/tam.config.toml ~/.tam/tam.db3`，并与 `iteration-00-sha256-before.txt` `diff -u`；最终 `USER_DATA_UNTOUCHED`。
4. 迁移测试：`cp ~/.tam/tam.db3` 到临时 HOME 的 `.tam/`，写入指向副本的 config，再运行 `tam task list --json` 触发迁移；真实库仅被 `cp` 读取。
5. 并发：`subprocess.Popen` 同时起两个进程（重复 5 轮），另测租约过期、heartbeat 续租、release、done、force、同 key 并发。
6. 所有结论附日志与退出码；未事后补日志，未伪造结果。

| 独立脚本（/tmp，临时） | 覆盖 | 断言数 |
|---|---|---|
| `qa_agent_json_cli.py` + 追加段 | envelope/exit/上下文/CRUD/过滤/生命周期/隔离 | 391 + 13 |
| `qa_supp.py` | 幂等、status 结构、默认 limit、env 归一化 | 21 |
| `qa_agent_migration.py` | 真实库副本 v0→v1 迁移 | 40 |
| `qa_agent_concurrency.py` | 并发 claim/租约/heartbeat/release/force/done/同 key | 70 |
| `qa_agent_mcp.py` | MCP stdio 12 工具与协议错误 | 61 |
| `qa_agent_gaps.py` + 追加段 | 6 类疑似缺口 + update-done 旁路 + next 并发 | 63 + 12 |

通用复现模板（所有场景同构，禁止使用真实 HOME）：

```bash
TAM=/Users/wstreet7/project/tam/target/debug/tam
H=$(mktemp -d)                       # 每个场景独立临时 HOME
HOME=$H TZ=UTC $TAM project add alpha --json
HOME=$H TZ=UTC $TAM task add t --project alpha --json
# 断言退出码 + 单行 stdout/stderr JSON；结束后 rm -rf "$H"
```

## 3. 基线（Gate 5）vs 当前门禁

| 门禁 | Gate 5 基线 | Gate B 当前 | 判定 | 证据 |
|---|---|---|---|---|
| `cargo build` | 0 / 4 warnings | **0 / 4 warnings（类别逐条相同）** | PASS | agent-regression-build.log |
| `cargo test` | 30 passed | **52 passed / 0 failed / 0 ignored**（13 单测 + 16 agent + 23 cli） | PASS | agent-regression-test-bare.log |
| 隔离 HOME / `--test-threads=1` | 30 passed | **52 passed ×2**（隔离并行/串行，真实哈希不变） | PASS | agent-regression-test-isolation.log |
| `cargo fmt --check` | 0 | **0** | PASS | agent-regression-fmt.log |
| `cargo clippy`（裸） | 0 error / 15 warnings | **0 error / 12 warnings**，类别为基线子集、无新类别 | PASS | agent-regression-clippy.log |
| Cargo.toml / Cargo.lock | 未改 | **未改（0 文件）** | PASS | agent-regression-clippy-cargo.log |
| schema | additive 已声明 | 基线 DDL 逐字节不变；+1 `task_events` 表、+6 `ALTER ADD COLUMN`、+4 索引；`DROP/RENAME/DELETE FROM` 0 | PASS | agent-regression-quality.log |
| 真实用户数据 | 基线哈希 | **逐字节一致**；mtime 未变（config 15:14、db/logs 06-25）；真实库仍 v0 | PASS | agent-regression-final-integrity.log |
| 人类 CLI / TUI | 正常 | 人类表格无 JSON 污染；TUI pty 启动/渲染/退出 0 | PASS | agent-regression-quality.log |

**B2（修复版）复验门禁**：build 0/4 warnings；`cargo test` **62 passed**（14 内联 + 25 agent + 23 cli，0 ignored）且默认/串行/隔离 HOME 四种跑法全绿；fmt 0；裸 clippy 0 error / 12 warnings（无新类别）；Cargo 未改；真实数据仍逐字节一致。详见 §13 与 `agent-regression-fix-*.log`。

## 4. §7 十条验收矩阵

| # | §7 要求 | 判定 | 独立证据（本次实跑） |
|---|---|---|---|
| 1 | 既有 30 测试绿；人类 CLI/TUI 不回归 | **PASS** | B1 52 tests 全绿（B2 复验 62 tests，见 §13；23 cli 用例名未变、原 7 内联仍在）；人类 `project/task/status` 输出仍为表格/文本，无 `tam.agent/v1`；TUI `tam -i` pty 下启动→渲染 “Projects views”→`q` 退出 0（agent-regression-quality.log） |
| 2 | 迁移：列/表/索引、行数不变、备份、幂等、真实库不被触碰 | **PASS** | 真实库副本：v0→v1、6 列、task_events、4 索引、`user_version=1`、project 5 / task 37 行不变、旧行默认 `priority=3/revision=1/tags=''`、pastoid 15 任务可读；`tam.db3.bak.v1` 20480B（v0/5/37/8 列）；二次运行幂等；空库不备份；真实库哈希不变（agent-regression-migration.log、-isolated-migration.log） |
| 3 | JSON envelope、流、exit 0/1/2/3/4 | **PASS** | 成功 stdout 单行 envelope（ok/schema/command/data/meta）、stderr 空；错误 stderr 单行 envelope、stdout 空；not_found=3、conflict=4、missing_project/missing_agent/invalid=1、clap=2 且无 envelope；`--json` 在子命令前/中/后均生效（agent-regression-json-cli.log，391+13 断言） |
| 4 | 原子 claim：并发恰一成功/一冲突；过期可被认领；heartbeat 续租；release 可再 claim | **PASS** | 5/5 轮并发：恰好 exit0×1 + exit4×1、DB 单 owner、revision=2、claimed 事件恰 1；租约 5s 过期后被 b 认领；heartbeat 续租后 b 冲突、过期后再认领；release 后 b 可 claim；force 抢占记录 `force=true/previous_agent`（agent-regression-concurrency.log，70 断言） |
| 5 | 幂等：同项目同 key 同 id + deduplicated；跨项目独立 | **PASS（有 caveat，见 §6 D1）** | 同项目第二次 `deduplicated:true` 且 id 相同；beta 项目同 key 独立新建；并发同 key 两进程均 exit0、同 id、DB 仅 1 行（json-cli 补充 21 断言、concurrency K1–K3） |
| 6 | 过滤/next：status/assigned/tag/priority/search/limit；next 排序与跳过 | **PASS** | priority/tag(AND、大小写)/search(title+description、大小写)/status/assigned(me/unassigned)/limit/组合全部正确；next：priority 升序→created 升序→id，跳过 Done 与有效租约，无候选 `{task:null}`；`next --claim` 无 agent → missing_agent（json-cli F、gaps X4） |
| 7 | 事件：顺序/actor/from/to/metadata | **PASS** | created→claimed→heartbeat→released→claimed→completed 顺序正确；actor 依次 human/a/a/a/b/human；from/to status 正确；claimed metadata 含 lease_seconds/force/previous_agent；update metadata.fields；show 返回 task+events（json-cli G22–G25、MCP T10） |
| 8 | 项目隔离：显式 `--project` 跨项目 id → not_found；未显式保持旧行为 | **PASS** | show/claim/update/done/heartbeat/release/events 七类跨项目均 exit3 not_found；无 `--project` 时全局 id 可正常读取 beta 任务（json-cli H） |
| 9 | MCP：initialize→tools/list→tools/call；stdout 无人类文本；未知方法错误码 | **PASS** | initialize(2024-11-05/capabilities.tools/serverInfo)、notifications 无响应、ping、tools/list 12 工具（name/description/inputSchema）、12 工具 tools/call 全链路、业务错误 isError=true、-32601/-32602/-32700/-32600、EOF exit0、stderr 空、每行均可解析 JSON（agent-regression-mcp.log，61 断言） |
| 10 | 工程门：build 0；test 全绿；fmt 0；clippy 无 error 且 ≤15；不新增依赖 | **PASS** | 见 §3；clippy 12 ≤ 15 且无新类别；Cargo.toml/lock 0 改动 |

## 5. JSON / MCP smoke 明细（独立断言）

- **JSON CLI（391 + 13 + 21 断言，0 FAIL）**：envelope/字段/RFC3339 Z；project list/current/add（含 restored、重复 add=conflict 4）；task add（description/priority/tag/idempotency）；list 过滤组合；show/events；update（title/description/status/priority/tags、清空 tags、空更新 invalid、priority 越界 invalid、空标题 invalid）；claim/heartbeat/release/done/next 生命周期与冲突；status 摘要；`--project`>`TAM_PROJECT`>config、`--agent`>`TAM_AGENT`、空 env 归一化；by-id 的 meta.project 观测（见 §6）。
- **补充（21 断言）**：同项目幂等同 id/deduplicated=true、跨项目独立；status data 结构（project/counts/leased/unassigned/next）；默认 limit=100/truncated=false；人类 `status` 文本且不含 JSON。
- **MCP（61 断言，0 FAIL）**：12 工具全链路（create→list→get→update→claim→heartbeat→events→status→next→release→claim→complete）；工具业务错误 isError；协议错误码；stdout 纯 JSON-RPC、stderr 空、EOF exit0。
- **构建器 smoke 复核**：builder `agent-json-smoke.log` 77 PASS、`agent-mcp-smoke.log` 39 PASS，均与日志自洽；QA 独立重跑规模更大（391 vs 77、61 vs 39），结论一致。

## 6. 并发 / 租约 / 幂等 / 迁移对抗结果

| 场景 | 实测 |
|---|---|
| 两进程并发 claim（5 轮） | 每轮恰 1 个 exit0 + 1 个 exit4 conflict；DB 单 owner；revision=created(1)+claim(1)=2；claimed 事件恰 1 |
| 租约过期（lease=5，sleep 5.6s） | 另一 agent claim 成功；`lease_expired` 由 false→true |
| heartbeat 续租 | holder 续租成功；3s 后再测另一 agent 仍 conflict；续租到期后可被认领 |
| release / force / done | release 清租约回 todo、可再 claim；force 抢占成功且事件记录 `force=true/previous_agent=agent-a`；done：他人有效租约 agent→conflict 4、human 允许、done 后再 done/claim→conflict 4 |
| 同 idempotency key 并发 create | 两进程均 exit0、同 task id、DB 仅 1 行 |
| 迁移（真实库副本） | 40 断言全过：v0→v1、6 列、task_events、4 索引、行数 5/37 不变、旧行默认值、备份 20480B(v0/5/37)、二次幂等、空库不备份、全程真实哈希不变 |
| 迁移测试在隔离 HOME 下 | 测试内 `std::env::var_os("HOME")` 找不到真实库 → 打印 “skipping” 并 **pass（未实际迁移）**；见 §7 质量 caveat |
| 并发 `next --claim`（3 轮） | 每轮 DB 恰 1 owner；抢输方返回 `{task:null}`(exit0) 或 conflict(exit4)（取决于竞态）；无双重认领 |

## 7. 疑似缺口判定（对抗验证，不修代码）

| 缺口 | 实测（B1 历史，2026-11-08 17:39） | 判定 / B2 复验（18:00） |
|---|---|---|
| D1 幂等 key 在任务软删后复用 | 带 key 创建 → 人类 `task del` 软删 → 同项目同 key 再 add：**exit 1 `internal`，message `failed to create task: A UNIQUE constraint failed`**（partial unique index 不筛 `is_deleted`） | **P2 实际缺陷 → ✅ 已修复**：schema v2（索引含 `is_deleted=0`），软删后同 key 新建成功且重试 dedup 同 id（§13 D1） |
| D2 `task update --status done` 绕过 done 租约冲突 | agent-a 持有效租约，agent-b `task update --status done` → **exit 0**，DB status=done、租约清空、事件为 `updated`（actor=agent-b）而非 `completed` | **P2 实际缺陷 → ✅ 已修复**：exit4 conflict、DB/revision 不变；holder/human 成功且写 `completed` 事件（§13 D2） |
| D3 `task update --status todo` 保留旧租约 | 认领后 human update --status todo → status=todo 但 `assigned_agent/lease_expires_at` 保留；`next` 返回 `{task:null}`（被当作 leased 跳过），直到租约过期或 `release` | **P2 语义缺口 → ✅ 已修复**：todo 清 assigned/lease，`next` 立即返回该任务（§13 D3） |
| D4 MCP 无法清空 tags | `tam_task_update tags:""` 仅字段 → isError invalid；`tags:""` + description → 成功但 tags 仍为 `["x"]`；CLI `--tags ""` 可清空 | **P2 功能缺口 → ✅ 已修复**：MCP `tags:[]` 与 `tags:""` 均清空；缺字段/null 不改（§13 D4） |
| D5 JSON 模式配置损坏不走 envelope | 临时 HOME 写坏 `tam.config.toml` → `project list --json` exit1，stderr 为人类文本 `Failed to load config: TOML parse error...`，无 JSON envelope | **P2 边界缺陷 → ✅ 已修复**：stderr 单行 `command:"startup"` envelope、exit1、stdout 空；human 模式仍人类文本（§13 D5） |
| `task del`/`task edit`/`task start`/`project use|edit|del --json` 不支持 | 全部 `invalid` exit1、stdout 空；契约 §4 未把这些列入 JSON 命令；实现文档仅列 project 三项 | **契约允许 / 文档部分声明 → B2 部分变化**：`task del --json` 已实现（软删+`deleted` 事件）；`project use/edit/del`、`task edit/start` 仍为 invalid（P3 文档项） |
| `project add --json` 的 ProjectView | 返回 `{id,name}`，无 created/updated 字段（非 null 而是缺字段）；`project list` 返回完整四字段 | **契约未明确 → ✅ 已修复**：add 返回完整 ProjectView（created/updated 非 null）（§13 P3） |
| by-id JSON 的 `meta.project` | show/update/events/claim 等未显式 `--project` 时 `meta.project=null`；显式 `--project` 时正确填充 | **契约未细化 → ✅ 已修复**：by-id 命令由任务所属项目填充 meta.project（§13 P3） |
| 未分配任务的 `lease_expired` | `assigned_agent=null, lease_expires_at=null → lease_expired=true` | **契约/文档已声明 → ✅ 已修正**：未分配恒 false；仅 holder+过期 true（§13 P3） |
| `--limit 0 / 1001` | 分别被 clamp 为 1 / 1000（不报错） | 契约未规定非法 limit 行为 → 观察项（不阻断，B2 未改） |
| 并发 `next --claim` 输家 | 返回 `{task:null}` 或 conflict，均不双写 | 契约未规定输家形态 → 可接受（B2 未改） |


关键缺口的可复现命令（**B1 修复前历史快照**；B2 修复后的实际行为见 §13）：

```bash
# D1 幂等 key 在软删后复用
H=$(mktemp -d); E="HOME=$H TZ=UTC"
$E $T project add p --json
ID=$($E $T task add t --project p --idempotency-key k --json | python3 -c 'import json,sys;print(json.load(sys.stdin)["data"]["task"]["id"])')
$E $T task del "$ID"                    # 人类软删（成功）
$E $T task add t --project p --idempotency-key k --json; echo "exit=$?"
# 实际 exit=1 internal "A UNIQUE constraint failed"

# D2 update --status done 绕过 done 租约冲突
H=$(mktemp -d); E="HOME=$H TZ=UTC"
$E $T project add p --json; $E $T task add t --project p --json
$E $T task claim 1 --project p --agent agent-a --lease 600 --json
$E $T task update 1 --project p --status done --agent agent-b --json; echo "exit=$?"
# 实际 exit=0，DB status=done/租约被清，事件为 updated(agent-b) 而非 completed

# D3 update --status todo 保留旧租约
$E $T task claim 1 --project p --agent agent-a --lease 600 --json
$E $T task update 1 --project p --status todo --json >/dev/null
$E $T task next --project p --json        # 实际 {"task":null}（todo 但被有效租约跳过）

# D4 MCP 清空 tags（JSON-RPC over stdio，tam_task_update）
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tam_task_update","arguments":{"id":2,"description":"d2","tags":""}}}' \
  | HOME=$H TAM_PROJECT=p $T mcp
# 实际 isError=false 但 task.tags 仍为 ["x"]；仅 tags="" 时 isError=true invalid

# D5 配置损坏时 JSON 模式无 envelope
H=$(mktemp -d); mkdir -p "$H/.tam"; echo 'broken toml' > "$H/.tam/tam.config.toml"
HOME=$H $T project list --json; echo "exit=$?"
# 实际 exit=1，stderr 为 "Failed to load config: TOML parse error ..."（非 JSON）
```

## 8. 测试质量审查（`tests/agent.rs`；B1 1129 行/16 用例 → B2 1614 行/25 用例）

- **正面**：B2 后 25 个 agent 集成用例 **25/25 各自 `TempHome`**（B1 为 16/16）；`command_for` 用 `env_clear` 注入 HOME/USERPROFILE/PATH/TZ；无 `#[ignore]`、`should_panic`、`set_var/remove_var`、`/Users/wstreet7` 硬编码；断言非“只判 exit0”：`assert_ok/assert_error` 共 80 处、原始 assert 168 处，校验 envelope 字段、TaskView 字段、revision、事件序列、DB 侧数据（并发/幂等/软删直接查 SQLite）；MCP 用独立读线程 + 10s 超时，避免死锁。B2 新增 9 个用例（D1–D5、L1、P3×3）均对 DB 字段/revision/事件逐项断言，未弱化原有断言。
- **caveat 1**：`migrates_copy_of_real_database` 以 `std::env::var_os("HOME")` 找真实库，隔离 HOME 下**静默 return 并计为 pass**（实测打印 `skipping: no real ~/.tam/tam.db3`）。因此 builder 的“隔离 HOME 52 passed”不能证明迁移在该跑法下被执行；QA 另行用真实库副本独立覆盖（40 断言）。
- **caveat 2**：并发 claim 测试只跑 1 轮（QA 扩到 5 轮）；lease 过期用 sleep 5.5s，CI 慢机可能抖动（本轮 QA 稳定）。
- **caveat 3**：迁移测试读取真实 `HOME` 路径（只读 `cp`），属环境依赖而非隔离失败；机器无真实库时覆盖静默缺失。
- **builder 声明复核**：52 tests（23+13+16）✓；新增内联 6（db 4 + task 2，7 old + 6 new = 13）✓；build 4 warnings ✓；clippy 0 error/12 warnings ✓；fmt 0 ✓；Cargo 未改 ✓；迁移 5/37、6 列、4 索引、备份 20480B、幂等 ✓；MCP 12 工具 ✓；README「Agent integration」/AGENTS 架构与测试段/SKILL/docs 均存在 ✓。**未发现 builder 数字夸大**；唯一“差异”是隔离 HOME 下迁移测试静默跳过（builder 未声明该限制）。

## 9. 未验证项（明确列出，不夸大）

- Windows / Linux 真机：仅 macOS；SQLite WAL、路径分隔符、`shasum` 命令差异未实测（实现已用 `std::env::temp_dir`/相对逻辑，但未跨平台跑）。
- TUI 仅 pty 自动冒烟（启动/渲染/退出）+ 静态检查；未人眼验收，未测 light 主题/鼠标/i18n（契约明确不做）。
- MCP 仅用自写 JSON-RPC 客户端验证，未接真实 Claude/Codex 客户端；未测并发多个 MCP 会话、超长行、半包/分片输入。
- 迁移失败路径（磁盘满、只读库、损坏库）未注入；备份恢复流程未演练（只验证备份文件存在且内容正确）。
- 未验证多机/多用户、HTTP（契约明确不做）、任务依赖图、SLA、TUI agent 视图。
- 长时间运行（租约默认 900s、心跳多轮）未做 soak；仅短租约快进。

## 10. 缺陷清单（不修代码）

| ID | 严重度 | 现象 / 复现 | 期望 / 实际 / 影响 |
|---|---|---|---|
| D1 | **P2** | `H=$(mktemp -d)`: `task add t --project p --idempotency-key k --json` → `task del <id>`（人类）→ 同 key 再 add | 期望：可重试创建新任务或明确 dedup；实际：exit1 `{"code":"internal","message":"... UNIQUE constraint failed"}`；影响：软删后该 key 永久不可用，agent 只收到 internal |
| D2 | **P2** | agent-a claim 任务（有效租约）→ agent-b `task update <id> --project p --status done --agent agent-b --json` | 期望：按 §4.2 done 规则 conflict 4；实际：exit0 完成并清租约，事件记为 `updated`（非 `completed`）；影响：绕过完成冲突守卫 + 审计语义不一致 |
| D3 | **P2** | claim 后 `task update <id> --status todo --json` → `task next --project p --json` | 期望：todo 时清租约（或文档声明需 release）；实际：todo 仍带有效租约，next 返回 null；影响：任务在租约到期（默认 900s）前对 next 不可见 |
| D4 | **P2** | MCP `tools/call tam_task_update {"id":2,"description":"d2","tags":""}`（任务 tags=`["x"]`） | 期望：与 CLI 一致清空 tags；实际：isError=false 但 tags 仍 `["x"]`；仅 tags="" 时 isError invalid；影响：MCP 客户端无法清空标签 |
| D5 | **P2** | 临时 HOME 写坏 config 后 `project list --json` | 期望：JSON envelope（internal, exit1）；实际：stderr 人类文本 `Failed to load config...`；影响：机器端解析失败（配置已坏，概率低） |
| O1 | P3 | `project add --json` 返回 `{id,name}`；by-id 命令 meta.project=null；`task edit/del/start --json` 不在实现文档清单 | 文档细化建议，非行为错误 |
| O2 | 观察 | `--limit 0/1001` clamp 为 1/1000；并发 `next --claim` 输家 null/conflict 二选一 | 契约未规定，行为自洽 |

**无 P0/P1。** 所有缺陷均为 P2 边界/一致性问题，不影响正常主流程、不损坏既有数据。

## 11. 结论

**B1 结论：PASS WITH CAVEATS（5 个 P2，已全部进入修复）。**
**B2 修复版复验结论：PASS（无 P0/P1/P2 残留；见 §13）。**

- §7 十条验收全部 PASS：修复版 62 tests 四种跑法全绿；JSON envelope/流/退出码、并发原子 claim、幂等、过滤/next、事件、项目隔离、MCP 全链路、迁移与工程门均通过。
- B1 独立断言 **671 条**、B2 复验新增 **773 条**（D1 48 + D2/D3/L1 89 + D4/D5/P3 113 + JSON 391 + MCP 62 + 并发 70），全部 0 FAIL。
- B1 Caveats ①（5 个 P2）已逐项修复并复验 PASS；② 迁移测试在隔离 HOME 静默跳过仍需真实库副本（QA 已独立补测 v0→v2 与 v1→v2）；③ 未做 Windows/Linux、真实 MCP 客户端、迁移失败注入、人眼 TUI（仍为非阻断未验证项）。
- 真实 `~/.tam` 全程逐字节未变（`USER_DATA_UNTOUCHED`），未迁移真实库；QA 未改任何源码/测试/文档，未 commit。


## 12. 证据索引（`docs/forge-takeover/evidence/`）

| 文件 | 内容 |
|---|---|
| agent-regression-sha-before.log / -final-integrity.log | 运行前后用户数据哈希、mtime、git 状态、无残留进程 |
| agent-regression-build.log / -fmt.log / -clippy.log / -clippy-cargo.log | build 0/4 warnings；fmt 0；clippy 0 err/12 warnings/无新类别；Cargo 未改 |
| agent-regression-test-bare.log / -test-isolation.log | 52 tests 裸跑/串行/隔离 HOME；真实哈希不变 |
| agent-regression-migration.log | 真实库副本迁移 40 断言（列/表/索引/行数/默认值/备份/幂等/空库） |
| agent-regression-isolated-migration.log | 隔离 HOME 下迁移测试静默 skip 证据；备份 20480B 复核 |
| agent-regression-json-cli.log | JSON envelope/exit/上下文/filters/lifecycle/隔离 391 断言 + 13 退出码 + 21 幂等/status/limit |
| agent-regression-concurrency.log | 5 轮并发 claim、租约过期、heartbeat、release/force/done、同 key 并发 70 断言 |
| agent-regression-mcp.log | MCP 12 工具全链路 + 协议错误 + stdout 纯度 61 断言 |
| agent-regression-gaps.log | 疑似缺口对抗（D1–D5、O1/O2）+ 并发 next --claim 证据 |
| agent-regression-quality.log | 测试清单/质量、schema additive、人类输出、TUI 冒烟、builder 声明复核 |
| agent-regression-gate-summary.log | 基线 vs 当前门禁数字与 builder 声明对照 |
| agent-regression-fix-sha-before.log / -quality.log | B2 前后用户数据哈希、人类 CLI/TUI、repo 状态、测试质量 |
| agent-regression-fix-build.log / -fmt.log / -clippy.log / -gate.log | B2 build 0/4、fmt 0、clippy 0 err/12 warnings、62 tests 计数与 Cargo 未改 |
| agent-regression-fix-test-bare.log / -test-isolation.log | 62 tests 裸跑/串行/隔离 HOME 四种跑法；真实哈希不变 |
| agent-regression-fix-d1-migration.log | v0→v2 / 构造 v1→v2 / 软删后 key 复用 48 断言 |
| agent-regression-fix-d2-d3-l1.log | D2/D3/L1 89 断言 |
| agent-regression-fix-d4-d5-p3.log | D4/D5/P3 113 断言 |
| agent-regression-fix-json-smoke.log / -mcp-smoke.log / -concurrency.log | B2 重跑 JSON 391、MCP 62、并发 70 断言 |

## 13. Gate B fixes re-verification（B2，2026-11-08 17:59–18:05）

> 对象：task-8 revision 9 修复版（schema v2 / D1–D5 / L1 / P3）。独立复跑，未引用 builder `agent-fix-*.log` 结论；所有运行注入临时 HOME，真实库只被 `cp`。

### 13.1 逐项复验

| 项 | 复验方法（独立） | 结果 | 证据 |
|---|---|---|---|
| **D1** schema v2 + 软删后 key 复用 | 真实库副本 v0→v2：`user_version=2`、6 新列、task_events、4 索引、idempotency 索引 SQL 含 `is_deleted = 0`、5/37 行不变、`.bak.v1` 为 v0 形状（`user_version=0`、8 列无 priority、5/37 行）；二次运行幂等；另独立构造 v1 库（旧索引）→ v2：索引被替换、**不新建备份**；行为：带 key 创建→`task del --json` 软删（`deleted` 事件/`is_deleted=1`/revision+1）→同 key 再 add 成功（新 id，dedup=false）→重试 dedup=true 同 id | **PASS**（48 断言，0 FAIL） | agent-regression-fix-d1-migration.log |
| **D2** update done 尊重租约 | agent-a claim → agent-b `update --status done`：exit4 conflict（details 含 agent-a）、DB 全字段/revision/事件数不变、无 completed；agent-a holder 成功：status=done、租约清空、revision+1、`completed` 事件 actor=agent-a、from/to=in-progress→done；human 成功且 actor=human | **PASS** | agent-regression-fix-d2-d3-l1.log |
| **D3** update todo 清租约 | holder `update --status todo`：assigned/lease 清空、status=todo、revision+1、`lease_expired=false`；随后 `next` 立即返回该任务（不再被跳过） | **PASS** | agent-regression-fix-d2-d3-l1.log |
| **L1** 租约完整性 | agent-b 对 agent-a 有效租约任务依次 `--title/--status todo/--priority/--tags`：均 exit4 conflict，DB title/status/assigned/lease/revision 与事件数逐次不变；holder `--title`/human `--title` 正常且保留租约；holder `--status todo` 清租约 | **PASS** | agent-regression-fix-d2-d3-l1.log |
| **D4** MCP tags 清空 | MCP `tam_task_update`：`tags:[]`、`tags:""`（含带 description）均清空为 `[]`；`tags:"Z, z"` 规范化为 `["z"]`；缺 tags 字段或 `tags:null` 不改；未知工具 isError、坏 JSON -32700、EOF exit0、stderr 空、stdout 纯 JSON-RPC | **PASS**（62 断言，0 FAIL） | agent-regression-fix-d4-d5-p3.log、-mcp-smoke.log |
| **D5** 坏配置 JSON envelope | 临时 HOME 写坏 config，`project list --json` / `task add --json` / `status --json` / `--json task list`：均 exit1、stdout 空、stderr 单行 envelope（ok=false、schema=tam.agent/v1、**command="startup"**、error.code=internal）；human `project list` 仍为人类文本 + exit1 | **PASS** | agent-regression-fix-d4-d5-p3.log |
| **P3** 文档/契约补齐 | `project add --json` 与 `project current --json` 的 created/updated 非 null（RFC3339 Z，restore 后同样非 null）；show/update/claim/heartbeat/release/done/events 未显式 `--project` 时 `meta.project` 均由任务所属项目填充；`task del --json` 软删 + `deleted` 事件（from=to=原状态、revision+1）、列表隐藏、再次 del/ show → not_found；未分配 `lease_expired=false`、有效租约 false、过期 true | **PASS**（113 断言，0 FAIL） | agent-regression-fix-d4-d5-p3.log |

### 13.2 B2 门禁与回归复跑

- `cargo build` exit 0 / **4 warnings**（unused Color、is_*/set_title dead、tip_* 未读，与 B1 相同）；`cargo test` **62 passed**（14 内联 + 25 agent + 23 cli）= 默认并行、`--test-threads=1`、隔离 HOME 并行、隔离 HOME 串行**四种跑法全绿**，0 ignored；`fmt --check` 0；裸 `clippy` 0 error / **12 warnings**（≤15，类别为基线子集，无新类别）；Cargo.toml/lock 0 改动。
- 重跑独立 smoke：JSON CLI **391 PASS**、MCP **62 PASS**、并发/租约 **70 PASS**（并发 claim 5/5 轮恰一成功一冲突、单 owner、revision=2；同 key 并发单行）。
- 人类 CLI：表格/文本输出无 `tam.agent/v1`/`"ok":` 污染；人类 `task del` 写 `deleted` 事件；TUI pty 启动/渲染/退出 0（无 JSON 泄漏）。人类 23 个既有测试全绿。
- 真实 `~/.tam`：sha256 与 `iteration-00-sha256-before.txt` 逐字节一致；真实库仍 `user_version=0`、current_project=pastoid、mtime 未变。
- 测试质量：25/25 agent 用例各自 `TempHome`；`#[ignore]` 0、`set_var` 0、硬编码真实 HOME 0；新增 9 个修复用例（D1–D5、L1、P3×3）断言非弱化（DB 字段/revision/事件逐项校验）。迁移测试在隔离 HOME 仍会静默 skip，但本轮 QA 已用真实库副本独立覆盖 v0→v2 与构造 v1→v2。
- builder 修复声明复核：62 tests（14+25+23）、12 warnings、schema v2/索引谓词、v0→v2 备份 v0 形状、D1–D5/L1/P3 行为均与独立复验一致，未发现夸大；builder 的 `agent-fix-*.log` 未作为本报告结论来源。

### 13.3 剩余风险（非阻断）

| 项 | 说明 | 建议 |
|---|---|---|
| P3 文档项（部分） | `project use/edit/del`、`task edit/start --json` 仍返回 invalid（契约未列入，实现文档未覆盖 task 三项）；`--limit 0/1001` 仍 clamp；并发 `next --claim` 输家 null/conflict 未规定 | 文档补一句即可，不阻断 |
| 迁移测试环境依赖 | 隔离 HOME 无真实库时迁移测试静默 pass，CI 若只跑隔离 HOME 会漏掉真实库迁移路径 | 改为显式 fixture 或 `#[ignore]` + 专门 job |
| 跨平台/真实客户端 | 未在 Windows/Linux、真实 Claude/Codex MCP 客户端、迁移失败注入（磁盘满/只读/损坏库）、长时 soak 下验证 | 保持为非阻断未验证项 |

### 13.4 B2 结论

**PASS**：D1–D5、L1、P3 全部修复并通过独立对抗复验；62 tests 四种跑法全绿；工程门无回归；真实用户数据逐字节未变。无 P0/P1/P2 残留（仅 P3 文档项与既有未验证项）。

