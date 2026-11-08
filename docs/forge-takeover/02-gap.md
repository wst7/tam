# 02 · 目标与差距（Gate 2）

- 输入：[00-discovery](00-discovery.md)（P0×3 / P1×8 / P2×17）、[01-asbuilt](01-asbuilt.md)（R1–R20 / D1–D11）。
- 基线：commit `d395841`；证据 `docs/forge-takeover/evidence/`（build 0、test 0 但污染真实 HOME、fmt 1、clippy 0）。
- 本文只定义「本轮做什么、怎么验收、怎么回滚」；不含代码与实现细节（Gate 3 产出）。决策 Q1–Q9 由 Lead 拍板，不再回问用户。

## A. 目标与非目标

### A.1 本轮目标

| # | 目标 | 对应 |
|---|---|---|
| T1 | 数据安全与一致性：改名/删除/恢复不再造成 `current_project` 漂移或孤儿任务 | 00 P0-1/P0-2/P1-5；01 R1/R2/R10 |
| T2 | 可读、可脚本判定的错误契约：not-found 友好文案、错误走 stderr | 00 P1-3/P1-4；01 R9/R16 |
| T3 | 一个真实新功能：`task list --status` 过滤 | DB id=11；01 §5.2 |
| T4 | 质量网：测试隔离 + 集成回归覆盖 T1–T3，真实用户数据零接触 | 00 P0-3/P1-6；01 R6 |
| T5 | 质量基线归零：`fmt --check` 通过、clippy 不新增 warning/无 error、build 0 | 00 P2-1；01 R7 |
| T6 | 文档与行为对齐：README 版本、SKILL 三条新行为 | 00 P2-3；01 D1/D6 |

### A.2 非目标（坚决不动）

| 不动项 | 说明 |
|---|---|
| DB schema | 不改 `CREATE TABLE/TRIGGER`、不改 `name UNIQUE`、不迁移 schema、不加 `user_version`（01 R13 延后） |
| 软删数据 | 不物理删除任何 project/task 行；不清理既有软删数据（01 §4.2） |
| 模块结构 | 不重排 `src/` 模块、不引入新层、不改 `commands → services → db` 方向（01 §2） |
| CLI 契约 | 不改既有命令名/别名/参数位置；`task list` 仅新增可选 `--status`；不新增顶层命令 |
| 软删约定 | 删除仍走 `is_deleted=1`，禁止 `DELETE FROM`（AGENTS.md:73-74、SKILL.md:14） |
| 无关重构 | 不批量清理 clippy 17 条、不动 theme/UI 外观、不做依赖去重（除 backlog 记录） |
| 生产侧数据根目录 | 不新增 `TAM_HOME` 等公开契约；CLI 仍只认 `HOME`（见 B 补充取舍） |
| 未列功能 | i18n、鼠标、设置页、在线存储、TUI 光标配色、`content` 字段（均进 E） |

## B. 决策记录（Lead 已拍板）

| # | 决策 | 明确不做 |
|---|---|---|
| Q1 | 项目存在活跃任务时**拒绝删除**：stderr `project 'x' has N active task(s)`，exit 1；仅零活跃任务才软删 | 不做 `--force`、不做级联软删（任务保持原状） |
| Q2 | 保留 DB 层 `name UNIQUE`（跨软删行）；`project add <已删名>` 执行 **restore**（`is_deleted=1→0`，触发器刷新 `updated`），输出 restored；活跃同名仍报 already exists；`project edit` 目标被软删行占用 → 明确错误（提示先 `tam project add` 恢复或换名） | 不改 UNIQUE、不迁移 schema、不自动恢复 |
| Q3 | 业务错误改走 **stderr**（`print_error!` 由 `println!` 改 `eprintln!`），exit 1；clap 用法错误保持 exit 2；not-found 文案 `task 999 not found` / `project 'x' not found`；禁止泄漏 `Query returned no rows` | 不新增 exit 3 |
| Q4 | `config` 仍是 `current_project` 唯一事实源；**DB 先成功、再写 config**；改名/删除当前项目仅成功后同步；config 保存失败 → stderr 报错 + 恢复指令（不静默、不回滚 DB）；CLI 与 TUI 同一语义（TUI 用 tip 呈现） | 不做 id 化当前项目迁移 |
| Q5 | 配置兼容（serde default/version/备份）**本轮不做** | 进 E backlog（01 R13） |
| Q6 | 多实例（WAL/busy_timeout/锁）**本轮不做** | 进 E backlog（01 R12） |
| Q7 | 范围仅本清单 + `tam task list --status <todo\|in-progress\|done>`（无 `--status` 保持现行为；非法值 clap exit 2） | i18n/鼠标/在线存储/设置页/TUI 光标配色本轮不做 |
| Q8 | `task.content` 不启用、不删除，仅记 backlog | — |
| Q9 | README 帮助示例版本改 1.0.13；SKILL 同步「删除受保护/同名可恢复/new --status」；TUI 失败用 tip 弹窗保留状态、不退出 | 不改 SKILL 其余章节 |

**补充取舍（有意为之，非遗漏）**

1. **测试隔离不引入生产侧 `TAM_HOME`**：00 §5.3 第 1 条降级为「进程级 HOME 注入 + config 既有路径覆盖能力」即可——测试为每个用例创建临时目录并作为子进程 `HOME`；config 单元测试只做纯 TOML 往返/显式路径，不读真实 HOME。代价：同一进程内不能并行切换数据根目录，测试需进程级隔离；收益：不新增公开契约、不扩生产代码面。
2. **TUI 同步口径补充**：TUI 只在其操作的**项目等于 `config.current_project`** 时同步配置；操作其它项目不改 config（避免「在 TUI 里管别的项目却偷换当前项目」）。
3. **restore 不自动 `use`**：恢复后 `current_project` 保持不变；需要切过去仍执行 `tam project use`。
4. **非数字 task id 归 clap 用法错误**（exit 2），数字但不存在归业务错误（exit 1）；原实现两者都是 raw DB 错误 exit 1，属有意的错误分类变化（Q3 允许）。

## C. 本轮 Gap 表

每行 = 一个可独立提交/回滚的缺口。验收列 Given-When-Then 均可用命令行或固定手工步骤判定；「验证」给出命令名（Gate 3 落地测试命名）或手工步骤。

| ID | 用户故事 | 现状（00/01 + file:line） | 目标行为 | Given / When / Then | 验证方式 | 回滚单元 |
|---|---|---|---|---|---|---|
| G1 | 作为用户，我希望项目列表的 Updated 反映最近变更 | 00 P1-1；01 R3；`services/project.rs:39-47` 的 updated 也解析 `row.get(2)` | updated 读第 3 列，与 created 独立解析；时间格式 `%Y-%m-%d %H:%M:%S` 不变 | Given 库中项目 created=07:00:00、updated=08:00:00；When `tam project list`；Then 两列分别显示 07:00:00 / 08:00:00 | `cargo test --test cli project_list_updated_column`（fixture/1s sleep 二选一）；手工改 `sqlite3` updated 后 list | `src/services/project.rs` |
| G2 | 作为用户，改名失败不应让当前项目被换掉 | 00 P0-1；01 R1；`commands/project.rs:76-94`（save :81 先于 DB edit :94）；`services/project.rs:109-123` | DB 成功后才写 config；失败 config 不变；config 保存失败 stderr + `tam project use <new>` 恢复指令、不回滚 DB；源/目标为非活跃行时按 Q2/Q3 拒绝 | Given current=alpha、beta 已存在；When `project edit alpha beta`；Then exit 1、stderr 含 UNIQUE、`project current`=alpha、list 仍 alpha+beta。Given current=alpha；When edit alpha gamma；Then exit 0 且 current=gamma | `cargo test --test cli rename_*`（失败/成功/current 保存失败三例）；手工两条命令 | `src/commands/project.rs`、`src/services/project.rs` |
| G3 | 作为用户，删除有任务的项目应被阻止；删当前项目后配置应同步 | 00 P0-2/P1-5；01 R2/R10；`commands/project.rs:49-50` TODO、`:52-63` 先清配置；`sql.rs:57-59` 无任务校验；TUI `app.rs:356-360` 不清 config | 活跃任务>0 → stderr `project 'x' has N active task(s)` exit 1，DB/config 不变；=0 才软删；删除当前项目在 DB 成功后清 config（失败按 Q4）；项目不存在/已删 → `project 'x' not found` exit 1；成功统一 SUCCESS 输出；TUI 同语义、refuse 用 tip 显示计数 | Given beta 有 2 活跃任务；When `project del beta`；Then exit 1、stderr 含 `2 active task(s)`、beta 仍 list、task 行未变。Given beta 仅软删任务；When del beta；Then exit 0、`is_deleted=1`。Given current=alpha；When del alpha；Then exit 0 且 `project current` 报未设置。Given TUI current=alpha 删除 alpha；Then 退出后 config 无 current | `cargo test --test cli delete_guard_*`；手工 4 场景 + `sqlite3` 行数对比 | `src/commands/project.rs`、`src/services/project.rs`、`src/sql.rs`（仅新增计数 SQL）、`src/app.rs` |
| G4 | 作为用户，删错的项目能通过相同名字加回来恢复 | 00 P1-2；01 R4；`services/project.rs:8-22` 仅 INSERT、UNIQUE 冲突直接 bail；`sql.rs:4` UNIQUE 跨软删行 | add 命中软删行 → `is_deleted=0`（触发器刷新 updated），输出含 restored；活跃同名仍 already exists exit 1；不自动 use；不改 UNIQUE/schema | Given beta 已软删；When `project add beta`；Then exit 0、输出含 restored、list 出现 beta 且 updated 刷新、current 不变。Given beta 活跃；When add beta；Then exit 1 already exists。When add 已删的当前项目 alpha；Then restored 且 current 仍 alpha | `cargo test --test cli revive_soft_deleted_project`；手工 3 条 | `src/services/project.rs`、`src/sql.rs`（新增恢复 SQL） |
| G5 | 作为脚本使用者，错误应可读、走 stderr、不被 SQL 文案污染 | 00 P1-3；01 R9/R16；`services/task.rs:75-97` 无映射；`services/project.rs:74-80` 字符串映射失效；`utils.rs:18` 用 println；`cli.rs:85-97` id 为 String | id 改为正整数解析（非数字/≤0 → clap exit 2）；不存在 → stderr `task 999 not found` exit 1；项目不存在 → `project 'x' not found`；用错误类型匹配而非字符串；`print_error!` 改 eprintln；成功输出仍 stdout | Given 无 id=999；When `task done 999`；Then exit 1、stderr 精确含 `task 999 not found`、stdout 为空。When `task done abc`；Then exit 2。When `project use nope`；Then exit 1 stderr `project 'nope' not found`；Then 全仓 stderr 无 `Query returned no rows` | `cargo test --test cli not_found_messages`；`cmd 2>e 1>o` 检查流向；`grep -rn "Query returned no rows" src/` | `src/services/{task,project}.rs`、`src/utils.rs`、`src/cli.rs`、`src/commands/task.rs` |
| G6 | 作为 TUI 用户，可预期的失败不应崩溃或丢状态 | 00 P1-4；01 R5；`app.rs:324,336,349,358,395,417,422` unwrap；`:428` 索引越界；rename/delete 不同步 config | 服务错误 → tip 弹窗保留状态不退出；选中索引越界安全（get 替代下标）；改名/删除当前项目同步 config（仅当对象==current）；删除后 selected 复位 | Given alpha、beta 并存；When TUI 把 alpha 改名为 beta；Then tip 报错、不退出、选中不变、DB/config 不变。Given 仅剩 1 项目；When 删除后按 ↓/e/d；Then 不 panic、tip 提示无项目。Given current=alpha；When TUI 删除 alpha；Then 退出后 config 无 current | 静态：`grep -n "unwrap()" src/app.rs` 中无服务结果 unwrap；手工清单见 D.4；`cargo run -- -i` | `src/app.rs` |
| G7 | 作为用户，空名字不应能建出项目/任务 | 00 P1-7；`cli.rs:47,82`；`commands/project.rs:7-12`、`commands/task.rs:7-11` 无校验 | trim 后为空 → stderr 可读错误 exit 1，不写 DB；其余命名规则不变 | Given 空库；When `project add ""`；Then exit 1、stderr 提示名称不能为空、`project list` 仍空。When `task add "   "`；Then exit 1 且任务行数不变 | `cargo test --test cli reject_blank_names`；`sqlite3` 计数 | `src/commands/project.rs`、`src/commands/task.rs` |
| G8 | 作为用户，任务时间戳应来自库中真实值而非被伪造成当前时刻 | 00 P2-8；01 R8；`services/task.rs:88-93` 解析必失败 → `unwrap_or_else(\|_\| Utc::now())` | 与 list 一致的 `NaiveDateTime` 解析（UTC）；失败必须返回错误，不得伪造 | Given 文本 `2026-11-08 07:14:54`；When 经 query_task 读取（start/done/edit/del 路径）；Then 得 `2026-11-08T07:14:54Z`，非当前时刻 | 单元测试解析函数；`grep -rn "unwrap_or_else(|_| Utc::now())" src/` 零命中 | `src/services/task.rs` |
| G9 | 作为用户，我只想看某一状态的任务 | DB id=11；01 §5.2；`cli.rs:86-88` task list 无参数；`sql.rs:70-73` 无 status 过滤 | 新增 `--status <todo\|in-progress\|done>`；无 `--status` 行为逐字节不变；非法值 clap exit 2；沿用 created DESC 与现有空列表文案；状态展示文案不变 | Given 当前项目 todo/in-progress/done 各 1；When `task list --status done`；Then 表内仅 Done 行。When `task list`；Then 与改造前一致（3 行）。When `--status wip`；Then exit 2 | `cargo test --test cli task_list_status_filter`；无参数输出与基线 `diff` | `src/cli.rs`、`src/commands/task.rs`、`src/services/task.rs`、`src/sql.rs` |
| G10 | 作为维护者，测试不应碰我的真实数据，且能回归本轮行为 | 00 P0-3/P1-6；01 R6；`config.rs:106-150`（硬编码 :112、save :135-141） | 重写 config 5 测试为纯 TOML/临时目录（不触 Lazy/真实 HOME）；新增 `tests/cli.rs`，每用例独立 HOME + `CARGO_BIN_EXE_tam`，覆盖 G1–G9；移除 `/Users/wstreet7` 断言；`cargo test` 前后真实 `~/.tam` 两文件 sha256 不变 | Given 真实 HOME 有用户数据；When `cargo test`；Then 全绿且 sha256 前后一致；Given 任一用例；Then 不创建/修改 `~/.tam` | D.2 命令；`grep -rn "/Users/wstreet7" src tests` 零命中 | `src/config.rs`、`tests/cli.rs`（新增）、`Cargo.toml`（如需 dev-deps） |
| G11 | 作为维护者，格式基线归零后才能看清功能 diff | 00 P2-1；01 R7；`fmt.exit=1`，22 hunk/8 文件 | `cargo fmt --check` exit 0；仅格式化、不改语义；最后做 | Given 功能项全部落地；When `cargo fmt --check`；Then exit 0 且 `git diff` 无逻辑变化 | `cargo fmt --check; echo $?`；`cargo build` 仍 0 | 8 文件：cli.rs、commands.rs、commands/project.rs、main.rs、services.rs、sql.rs、ui.rs、utils.rs |
| G12 | 作为用户，文档应描述真实行为 | 00 P2-3；01 D1；`README.md:66` Version 1.0.10；`SKILL.md:44,53,87` | README 示例改 1.0.13；SKILL 写明：删除受保护（含计数）、同名 add 恢复软删项目、`task list --status` 及取值；不扩写其它章节 | Given 文档更新后；When `grep "Version: 1.0.13" README.md`；Then 命中且 1.0.10 不出现；SKILL 三处行为描述存在且与 `tam -h` 一致 | `grep`；`tam -h` 与 README 示例对照 | `README.md`、`skills/tam-usage/SKILL.md` |

### C.2 实施顺序与依赖（Gate 3 输入）

1. **G10 骨架先行**（隔离 harness + 空 `tests/cli.rs`），后续每项补「红→绿」用例；否则无法证明修复。
2. **G5 先于 G2/G3/G4**：统一 not-found 错误类型/出口，供项目与任务路径复用。
3. **G1 → G2 → G3 → G4**（数据读取 → 改名 → 删除 → 恢复，均动 `services/project.rs`，按此顺序可逐步回归）。
4. **G7 → G8 → G9**（独立小项，无相互依赖）。
5. **G6 在 G3/G4 语义稳定后**，避免 TUI 重复造 config 同步逻辑。
6. **G12 随 G9 行为定稿**；**G11 最后**（避免格式化 diff 混入功能提交，保持可回滚）。

## D. 验收总表（基线 → 目标）

### D.1 质量门

| 维度 | 基线 | 目标 | 判定命令 |
|---|---|---|---|
| build | exit 0、4 warnings | exit 0、warnings ≤4 且不新增 | `cargo build 2>&1 \| tail -3` |
| test | 5 passed，但写真实 HOME | 隔离 HOME 下全绿（含 `tests/cli.rs` 全部用例） | `cargo test`（测试已隔离）；另跑 `HOME=$(mktemp -d) cargo test` |
| 真实数据安全 | 无保证（曾污染） | `~/.tam/tam.config.toml`、`~/.tam/tam.db3` 前后 sha256 不变 | D.2 命令 |
| fmt | exit 1、22 hunk/8 文件 | exit 0 | `cargo fmt --check; echo $?` |
| clippy | 0 error、17 warnings | 0 error、warnings ≤17 且不新增 | `cargo clippy --all-targets 2>&1 \| grep -c '^warning'` |
| schema/软删约定 | — | `CREATE TABLE/TRIGGER` 常量逐字节不变；无物理删行 | `git diff -U0 src/sql.rs \| grep -E 'CREATE (TABLE\|TRIGGER)'` 应为空；集成用例断言删除后行数不变 |
| 错误流向 | 业务错误在 stdout | 业务错误在 stderr、退出码 1；clap 用法错 2 | D.3 场景逐条 `1>/tmp/o 2>/tmp/e` 断言 |
| 文档 | README 1.0.10 | README 1.0.13；SKILL 三条行为 | `grep` 断言 |

### D.2 测试隔离与数据安全命令

```bash
before=$(shasum -a 256 ~/.tam/tam.config.toml ~/.tam/tam.db3)
cargo test                      # 测试内部为每个用例注入临时 HOME
after=$(shasum -a 256 ~/.tam/tam.config.toml ~/.tam/tam.db3)
[ "$before" = "$after" ] && echo "USER DATA UNTOUCHED"
grep -rn "/Users/wstreet7" src tests && echo "HARDCODED PATH FOUND" || echo "OK"
```

### D.3 CLI smoke 场景（每条断言 退出码/流/关键文案）

| # | 场景 | 期望 |
|---|---|---|
| S1 | `project add p1` → `project use p1` → `project list` | exit 0；list 的 Created/Updated 独立（G1） |
| S2 | `project add p2` → `project use p1` → `project edit p1 p2` | exit 1；stderr UNIQUE；`project current`=p1（G2） |
| S3 | p1 有 2 个活跃 task → `project del p1` | exit 1；stderr `project 'p1' has 2 active task(s)`；任务未动（G3） |
| S4 | p2 无活跃 task → `project del p2` → `project add p2` | exit 0；add 输出含 restored；list 出现 p2（G4） |
| S5 | `task done 999` / `task done abc` / `project use nope` | 1 且 stderr `task 999 not found` / 2（clap）/ 1 且 stderr `project 'nope' not found`；无 SQL 原文（G5） |
| S6 | `project add ""` / `task add "   "` | exit 1，stderr 可读错误，库无新行（G7） |
| S7 | 建 3 个不同状态任务 → `task list --status done` / `task list` / `--status wip` | 1 行 Done；无参数 3 行且与基线一致；exit 2（G8/G9） |
| S8 | 删当前项目后 `project current` | exit 1 且文案 `No current project set...`（G3/Q4 联动） |

### D.4 TUI 静态检查 + 人工清单

- 静态：`grep -n "unwrap()" src/app.rs` 无「服务调用结果」上的 unwrap；`get_project/get_task` 使用 `get()` 边界安全；rename/delete 路径存在 config 同步调用。
- 人工（`cargo run -- -i`，记录终端截图/文字）：
  1. 重名改名 → tip 报错、TUI 不退出、选中保持；
  2. 删除有任务的项目 → tip 含任务数、项目仍在；
  3. 删除当前项目 → 退出后 `tam project current` 未设置；
  4. 删除最后一个项目后按 ↓/e/d → 无 panic、tip 提示无项目；
  5. 对非当前项目改名/删除 → config 的 current_project 不变；
  6. 任务屏增/改/删/start/done 失败（如已 Done 再 done）→ tip 展示、不退出。

## E. Backlog（本轮不做，按优先级）

| 项 | 来源 | 优先级 | 备注 |
|---|---|---|---|
| 配置兼容：serde default/version/备份/损坏自愈 | 00 P1-8；01 R13；Q5 | P1 | 旧配置缺字段当前直接 exit(1) |
| 多实例：WAL、busy_timeout、配置锁 | 00 P2-6；01 R12；Q6 | P1 | 并发写风险，需先定单/多实例契约 |
| 删除/改名对已软删行的 SQL 守卫（R11 剩余面） | 01 R11 | P1 | 本轮 G2/G3 覆盖来源/目标非活跃行；其余 SQL 守卫入 backlog |
| 删除输出/幂等语义统一（R10 剩余面） | 01 R10 | P1 | 本轮 G3 统一成功/失败输出，其余（重复删除提示）入 backlog |
| log4rs 初始化 expect → 友好错误 | 01 R14 | P1 | 路径异常时 panic 101 |
| `-i` + 子命令静默忽略 | 01 R15 | P2 | 需定「报错」还是「忽略」 |
| `project list` ORDER BY | 00 P2-16；01 R17 | P2 | 顺带可在 G1 做，但非验收要求 |
| 死代码/死依赖：`content`、`serde_json`、`TaskStatus::is_*`、`Theme.tip_*` | 00 Q8/P2-10；01 R18 | P2 | Q8 明确不启用 content |
| 依赖去重：`tui-input` 0.12.1 → 0.14.0 | Lead 指定 | P2 | 需评估 API 变更 |
| CI 质量门禁（fmt/test/clippy job） | 01 R7 剩余面；Lead 指定 | P2 | 本轮只修 fmt，不加 job |
| i18n / 鼠标 / 设置页 / 在线存储 / TUI 光标配色 | 00 §6.2（DB id 17/18/19/8/5） | P2 | Q7 明确延后 |
| 主题弹窗接线（tip_bg/tip_fg） | 00 P2-10 | P2 | 与 TUI 光标配色同批 |
| TUI 每帧查库/渲染内改状态 | 00 P2-9 | P2 | 性能/架构债 |
| 文档其余漂移（D2/D5/D7/D8/D11） | 01 §8 | P2 | 本轮只改 D1 + SKILL 三条 |
| 发布链：dist prerelease、homebrew 重复 | 01 R19 | P2 | 发布可重复性 |
| UTC 时间展示不提示时区 | 01 R20 | P2 | 需先定展示契约 |

## F. 与 01-asbuilt 的逐条对齐

| 01 风险 | 本轮处置 | 说明 |
|---|---|---|
| R1 改名先 config 后 DB | ✅ 覆盖 G2 | 顺序反转 + 失败不回写 |
| R2 删除不校验任务/FK | ✅ 覆盖 G3 | 拒绝删除；本轮不启用 FK（G3 用计数查询） |
| R3 updated 读错列 | ✅ 覆盖 G1 | 读第 3 列 |
| R4 软删 + UNIQUE 跨行 | ✅ 覆盖 G4（+G2 目标冲突错误） | 保留 UNIQUE，用 restore 解决重建 |
| R5 TUI unwrap/索引 | ✅ 覆盖 G6 | 改 tip + 边界安全 + config 同步 |
| R6 测试硬编码 HOME/污染 | ✅ 覆盖 G10 | 纯 TOML 单测 + 独立 HOME 集成测试 |
| R7 无 CI 门禁/fmt 失败 | ◐ 部分覆盖 G11 | fmt 修绿；CI job 延后（E） |
| R8 query_task 时间解析 | ✅ 覆盖 G8 | 失败返回错误，不伪造 |
| R9 无效 task id raw 错误 | ✅ 覆盖 G5 | 分类 + 友好文案 + stderr |
| R10 删除输出/幂等不一致 | ◐ 部分覆盖 G3 | 成功/失败输出统一；重复删除提示入 backlog |
| R11 软删行可被 del/edit 命中 | ✅ 覆盖 G2+G3 | edit 源/目标、del 均按 not-found 处理 |
| R12 无事务/并发 | ⏸ 延后 | E（Q6） |
| R13 schema/config 兼容 | ⏸ 延后 | E（Q5） |
| R14 log4rs expect | ⏸ 延后 | E |
| R15 `-i` 忽略子命令 | ⏸ 延后 | E |
| R16 错误走 stdout | ✅ 覆盖 G5 | `print_error!` → eprintln |
| R17 ORDER BY | ⏸ 延后 | E（G1 可顺带） |
| R18 死代码/死依赖 | ⏸ 延后 | E（Q8） |
| R19 发布链 | ⏸ 延后 | E |
| R20 UTC 展示 | ⏸ 延后 | E |

覆盖统计：**R 全覆盖 10 条**（R1–R6、R8、R9、R11、R16），**部分覆盖 2 条**（R7、R10），**延后 8 条**（R12–R15、R17–R20）；另覆盖 00 P1-7（G7）、00 P2-3/D1（G12）、DB id=11 新功能（G9）。

**与 00/01 的偏差与口径补充**（需 Lead 知晓）

1. **新增 `tests/` 与 AGENTS.md:15「There is no tests/ directory」冲突**：G10 落地后该句过期，建议 Gate 3 顺带改一行（属文档同步，不扩范围）；若不允许，则记入 backlog。
2. **非数字 id 归类变化**：原实现返回 raw DB 错误 exit 1，G5 改为 clap 用法错误 exit 2（依据 Q3「clap 用法错误保持 exit 2」）。
3. **Q4 的 TUI 口径细化**：只同步「被操作项目 == current_project」的情形，避免 TUI 管理非当前项目时改配置（见 B 补充 2）。
4. **Q2 的恢复口径细化**：restore 不自动切换当前项目（见 B 补充 3）。
5. **P1-6 覆盖方式**：本轮以 `tests/cli.rs` 集成测试覆盖行为，不为每个 service 单独建单测（它与 00 建议的「核心服务覆盖」等价但更省成本）。
