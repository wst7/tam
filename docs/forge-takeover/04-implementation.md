# 04 · 实现说明（Gate 4）

- 对象：`/Users/wstreet7/project/tam`（tam 1.0.13，单 crate，CLI + TUI）。
- 输入：[02-gap](02-gap.md)（G1–G12 / Q1–Q9）、[03-plan](03-plan.md)（S1–S8）、Lead Gate 4 复核意见（B1–B5）。
- 基线：HEAD `d395841`；真实用户数据 `~/.tam/tam.config.toml`、`~/.tam/tam.db3` 全程只读。
- 本文记录：每条 G 改了什么、为什么、回退文件、验证结果；以及执行顺序、门禁结果、复核修复、偏差与残留风险。
- **未执行 git commit/reset/stash/checkout 全仓；未改 schema DDL；未加依赖或 dev-deps；未新增生产侧 `TAM_HOME`。**
- 证据目录：[evidence/](evidence/)（全部命令 `2>&1 | tee` 实拍，无手写日志）。

## 1. 交付物清单

| 类别 | 文件 | 说明 |
|---|---|---|
| 生产代码 | `src/utils.rs` | `print_error!` 改 `eprintln!`（错误流） |
| 生产代码 | `src/cli.rs` | task id 改 `i32` + `range(1..)`；`task list --status` 的 `StatusArg`（ValueEnum） |
| 生产代码 | `src/commands/task.rs` | id 透传 `i32`；`list(status)` 透传 |
| 生产代码 | `src/commands/project.rs` | add restored 文案；edit/delete「先 DB 后 config」+ 恢复指令 + 统一成功输出；入口 trim 与空名拒绝（S9） |
| 生产代码 | `src/services/project.rs` | updated 读第 3 列；错误类型匹配；删除计数守卫；同名 restore；改名源/目标校验；`normalize_project_name`（trim+空名拒绝，S9）；edit 同名 no-op（S9） |
| 生产代码 | `src/services/task.rs` | `normalize_task_title`（add/edit 共用，S9）；`query_task` 错误映射；`parse_db_datetime`（不伪造 now）；`list(project_id, status)` |
| 生产代码 | `src/sql.rs` | 仅新增查询常量（含软删查询 / RESTORE / COUNT_ACTIVE / 状态过滤），DDL 一字未改 |
| 生产代码 | `src/app.rs` | TUI 去 unwrap、`get()` 边界、错误转 tip、删除后复位、仅对象==current 时同步 config；改名输入 trim（S9） |
| 生产代码 | `src/config.rs` | 5 个旧测试替换为 3 个纯 TOML/显式路径测试（S9 改为平台无关路径）；抽 `default_for_dir` / `default_for` |
| 生产代码（fmt） | `src/main.rs`、`src/commands.rs`、`src/services.rs`、`src/ui.rs` | S8 `cargo fmt` 机械格式化（零语义变化） |
| 新增测试 | `tests/cli.rs` | 731 行，**23 个集成用例**；std + `env!("CARGO_BIN_EXE_tam")`，无 dev-deps |
| 文档 | `README.md`、`skills/tam-usage/SKILL.md`、`AGENTS.md` | 1.0.13、SKILL 三条新行为、AGENTS 测试段一行 |
| 证据 | `docs/forge-takeover/evidence/iteration-*.log/txt` | 见 §7 |

`Cargo.toml` / `Cargo.lock` 未改动（无依赖/dev-deps 变化）。

## 2. 按 Gap 的实现与验证（G1–G12）

| Gap | 改了什么 | 为什么 | 回退文件 | 验证结果 |
|---|---|---|---|---|
| G1 | `services/project.rs::list` 的 `updated` 改读 `row.get(3)`（原来也读 2） | 列语义失效，Updated 恒等于 Created | `src/services/project.rs` | `project_list_updated_column` 绿（add→sleep 1.1s→edit，两列不同）；连续 5 次通过（`iteration-S3-verify.log`）；smoke S1 |
| G2 | `commands/project.rs::edit` 改为「DB 成功后再写 config」；仅 `current==old` 时把 current 设为 new；save 失败返回 Err，文案含 ``tam project use <new>``，不回滚 DB。`services/project.rs::edit` 先校验源活跃、目标活跃/软删行 | 原实现先写 config 后改库，UNIQUE 失败后 current 漂移；目标被软删行占用时报 raw SQL | `src/commands/project.rs`、`src/services/project.rs` | `rename_failure_keeps_current_project`、`rename_success_updates_current_project`、`rename_to_soft_deleted_name_errors` 绿；M1 实拍配置保存失败分支（见 §5）；smoke S2 |
| G3 | `services/project.rs::delete` 先取活跃项目→`COUNT_ACTIVE_TASKS_SQL`>0 拒绝并报数量→才软删；`commands/project.rs::delete` 先 DB 成功后清 config，成功统一输出 SUCCESS；`app.rs` 同语义 | 原 TODO 未实现，静默删除造成任务孤儿；删除当前项目时先清配置再删库 | `src/services/project.rs`、`src/commands/project.rs`、`src/sql.rs`、`src/app.rs` | `delete_refuses_project_with_active_tasks`（stderr 含 `2 active task(s)`）、`delete_current_project_clears_config`、`delete_missing_project_not_found`、`delete_allowed_when_only_soft_deleted_tasks` 绿；smoke S3/S8 |
| G4 | `services/project.rs::add` 先查含软删行：活跃同名→already exists；软删同名→`RESTORE_PROJECT_SQL`（触发器刷新 updated）并返回 `(Project, true)`；`commands/project.rs` 按 restored 输出 ``restored``；不自动 use | UNIQUE 跨软删行导致删错无法自愈 | `src/services/project.rs`、`src/commands/project.rs`、`src/sql.rs` | `add_restores_soft_deleted_project`（输出 restored、updated 刷新、current 不变）、`add_active_duplicate_already_exists` 绿；smoke S4 |
| G5 | `utils.rs`：`print_error!` `println!`→`eprintln!`；`cli.rs`：task id `i32` + `value_parser!(i32).range(1..)`；`services/{task,project}.rs`：`rusqlite::Error::QueryReturnedNoRows` 精确映射（`task <id> not found` / `project '<name>' not found`），禁止 `contains("no rows")` | 原实现泄漏 `Query returned no rows` 且错误走 stdout；项目侧字符串匹配写错 | `src/utils.rs`、`src/cli.rs`、`src/commands/task.rs`、`src/services/{task,project}.rs` | `task_not_found_message`、`project_not_found_message`、`non_numeric_task_id_usage_error`（abc/0/-1 → exit 2）、`errors_use_stderr` 绿；smoke S5（无 raw SQL 原文） |
| G6 | `app.rs`：所有 services 结果 `unwrap`→`match` + 中文 tip；`get_project` 用 `projects.get(selected)`；删除项目成功 `selected_project=None` + `tasks.clear()`；删除任务成功 `selected_task=None`；`query_*` 失败仅 tip；项目 add/edit/delete 仅当对象==`config.current_project` 时同步 config（失败 tip） | 可预期业务失败会 panic；索引越界；TUI 不同步 config | `src/app.rs` | 静态：`grep "unwrap()" src/app.rs` 由 13 处（7 处服务结果）降为 0；M1 实拍；M2–M7 未执行（见 §6） |
| G7 | `services/project.rs::add`、`services/task.rs::add` 对 trim 后空值 `bail!`；S9 扩展到 **edit**：`project edit` 空白目标、`task edit` 空白标题同样 exit 1 且 DB/config 不变；`project del` 空名拒绝 | 空名/空标题可入库，产生脏数据；edit 是复核发现的同族缺口 | `src/services/project.rs`、`src/services/task.rs`、`src/commands/project.rs` | `reject_blank_project_name`、`reject_blank_task_title`、`project_edit_blank_name_rejected`、`task_edit_blank_title_rejected` 绿；smoke S6 |
| G8 | `services/task.rs` 新增 `pub(crate) fn parse_db_datetime`（`NaiveDateTime` `%Y-%m-%d %H:%M:%S`→UTC，失败 Err）；`list` 与 `query_task` 共用；删除 `unwrap_or_else(|_| Utc::now())` | 原 `parse::<DateTime<Utc>>()` 必失败并静默伪造当前时刻 | `src/services/task.rs` | U4 `sqlite_timestamp_parsed_as_utc`、U5 `sqlite_timestamp_invalid_is_err` 绿；`grep` 伪造 fallback 零命中 |
| G9 | `cli.rs`：`TaskSubcommand::List { status: Option<StatusArg> }`（ValueEnum：todo/in-progress/done）；默认 `Self::List { status: None }`；`commands/task.rs` 透传；`services/task.rs::list(project_id, status)` 选 SQL；`sql.rs` 新增 `SELECT_TASKS_BY_STATUS_SQL`（同 `created DESC`） | 新功能（DB id=11）；无参行为必须不变 | `src/cli.rs`、`src/commands/task.rs`、`src/services/task.rs`、`src/sql.rs` | `task_list_status_filter`、`task_list_without_status_unchanged`、`invalid_status_value_usage_error`（exit 2）绿；无参输出与改造前 baseline `diff -u` 为空（`iteration-S6-verify.log`）；smoke S7 |
| G10 | `config.rs` 5 个旧测试（硬编码 `/Users/wstreet7`、经 `Lazy` 写真实 HOME）替换为 3 个纯 TOML/显式路径测试；S9 改为 `std::env::temp_dir()` 构造路径（Windows 可移植）；新增 `tests/cli.rs`（`TempHome` + `run_tam`，`env_clear` 注入 HOME/USERPROFILE/PATH/TZ，`CARGO_BIN_EXE_tam`） | 任何 `cargo test` 都会污染用户配置；测试不可移植 | `src/config.rs`、`tests/cli.rs` | 隔离 HOME 全绿；裸 `cargo test` 前后真实 `~/.tam` sha256 不变；`grep /Users/wstreet7 src tests` 零命中；`grep set_var tests` 零命中 |
| G11 | S8 最后执行一次 `cargo fmt`（仅格式化；import 顺序/空白，无逻辑变化）；S9 后再 `fmt --check` 仍为 0 | 基线 `fmt --check` exit 1（22 hunk/8 文件） | `src/{cli,commands,commands/project,main,services,sql,ui,utils}.rs` | `cargo fmt --check` exit 0；fmt 前后 `cargo build`/`cargo test` 不回归；pre-fmt 快照逐文件 diff 见 `iteration-S8-fmt.log` |
| G12 | README `Version: 1.0.13`；SKILL：删除受保护（含 `active task(s)` 计数）、同名 add `restored`、`task list --status <todo\|in-progress\|done>`；AGENTS 测试段一行（inline + tests/cli.rs + 隔离 HOME） | 文档与真实行为不一致 | `README.md`、`skills/tam-usage/SKILL.md`、`AGENTS.md` | grep 全绿（`iteration-S7-verify.log` / `iteration-gate-schema.log`）；SKILL 其余章节未改 |

## 3. 执行顺序（S1–S8 + 复核 S9，先红后绿）

| 步 | 内容 | 红证据 | 绿证据 |
|---|---|---|---|
| S1 | 隔离测试骨架：config 测试重写 + `tests/cli.rs`（TempHome/run_tam + I19） | `iteration-S1-red.log`（隔离 HOME 下 `test_config_dir` 失败，exit 101，临时 HOME 生成 `.tam/tam.config.toml`） | `iteration-S1-green.log`（3 单测 + 1 集成全绿；裸跑安全；sha256 不变） |
| S2 | G5+G7+G8 地基（错误流/id/校验/时间解析） | `iteration-S2-red.log`（6 集成失败：raw SQL、exit 码、空名入库）；`iteration-S2-red-unit.log`（U4/U5 编译失败：函数不存在） | `iteration-S2-verify.log`（5 单测 + 7 集成全绿） |
| S3 | G1+G2（updated 列、改名一致性） | `iteration-S3-red.log`（Updated==Created；失败改名后 current=beta；软删目标 raw UNIQUE） | `iteration-S3-verify.log`（11 集成全绿；G1 连续 5 次稳定） |
| S4 | G3+G4（删除守卫、同名恢复） | `iteration-S4-red.log`（有任务仍 exit 0；恢复报 already exists；非当前删除无 SUCCESS） | `iteration-S4-verify.log`（17 集成全绿） |
| S5 | G6（TUI 健壮性，无自动化用例） | `iteration-S5-tui-manual.log`（静态 13 处 unwrap / 直接下标） | 同文件（unwrap 0、无直接下标、无 `as i32`；M1 实拍；M2–M7 未执行） |
| S6 | G9（`task list --status`） | `iteration-S6-red.log`（`--status` unknown argument） | `iteration-S6-verify.log`（20 集成全绿；无参输出 diff 为空；非法值 exit 2） |
| S7 | G12（文档同步） | `iteration-S7-verify.log`（README 1.0.10 命中、SKILL 关键词缺失） | 同文件（1.0.13 命中、SKILL 三条、AGENTS 段命中） |
| S8 | G11（fmt）+ 全量门禁 | `iteration-S8-fmt.log`（fmt 前 `fmt --check` 为基线 exit 1） | `iteration-gate-*.log`、`iteration-final-sha256-after.txt` |
| S9 | Lead 复核 B1–B5（edit 空值、trim 一致性、TUI 同名、config 单测可移植） | `iteration-S9-repro.log`（B1/B2/B3 实拍复现）、`iteration-S9-red.log`（3 集成失败 + 2 单测编译失败） | `iteration-S9-verify.log`（23 集成 + 7 单测全绿；post-fix 复现对照）；`iteration-S9-fmt.log` |

每步应用前快照：`/tmp/forge-tam/patches/stepN.diff` + `snapshots/stepN-pre/`（含 `tests-cli.rs`）；S8 另存 `step8-fmt.diff` + `snapshots/pre-fmt/`；S9 另存 `step9.diff`。回滚方式见 §8。

## 4. 测试矩阵与结果

| 类型 | 文件 | 数量 | 结果 |
|---|---|---|---|
| 集成 | `tests/cli.rs` | **23**（覆盖 G1–G5、G7–G10、I19 与 S9 复核项） | 默认并行 23 passed；`--test-threads=1` 23 passed |
| 单元 | `src/config.rs` | 3（默认路径 / TOML 往返 / 显式路径反序列化，平台无关） | passed |
| 单元 | `src/services/task.rs` | 3（U4 真实 UTC；U5 非法字符串 Err；标题规范化） | passed |
| 单元 | `src/services/project.rs` | 1（项目名规范化 trim/空名） | passed |
| 合计 | — | **30** | 全绿（`iteration-gate-test.log`） |

集成用例清单：`binary_writes_only_under_injected_home`、`task_not_found_message`、`project_not_found_message`、`non_numeric_task_id_usage_error`、`errors_use_stderr`、`reject_blank_project_name`、`reject_blank_task_title`、`project_list_updated_column`、`rename_failure_keeps_current_project`、`rename_success_updates_current_project`、`rename_to_soft_deleted_name_errors`、`delete_refuses_project_with_active_tasks`、`delete_current_project_clears_config`、`delete_missing_project_not_found`、`delete_allowed_when_only_soft_deleted_tasks`、`add_restores_soft_deleted_project`、`add_active_duplicate_already_exists`、`task_list_status_filter`、`task_list_without_status_unchanged`、`invalid_status_value_usage_error`、`project_edit_blank_name_rejected`、`task_edit_blank_title_rejected`、`project_names_trimmed_consistently`。

## 5. Lead 复核修复（S9：B1–B5）

| # | 问题（复核发现） | 根因 | 改动 | 测试/验证 |
|---|---|---|---|---|
| B1 | `project add a; project use a; project edit a ""` → SUCCESS，DB 出现空名、config `current_project=""`、`project current` 打印空 | `services/project.rs::edit` 仅 trim 未拒绝空；`commands/project.rs::edit` 把 raw（空串）写 config | service edit 用 `normalize_project_name`（trim+空名 bail）；commands edit 入口 trim + 空名 bail，config 只写规范名 | `project_edit_blank_name_rejected`（exit 1 + stderr `cannot be empty` + current=alpha + list 仅 alpha + task add 仍可用）；smoke S9 |
| B2 | `task edit 1 "   "` → SUCCESS，标题变为空白 | `services/task.rs::edit` 未 trim/校验 | 新增 `normalize_task_title`，add/edit 共用（trim + 空名 bail） | `task_edit_blank_title_rejected`（exit 1 + stderr + list 标题不变）；smoke S9 |
| B3 | 空白不一致三连：add 存 alpha 但 `use " alpha "` not found；`edit alpha " beta "` 后 config 存 `" beta "`；`del " alpha "` 后 config 留悬空 current | `query_project` 未 trim；`commands::edit` 把 raw target 写 config；`commands::delete` 用 raw name 与 current 比较 | `normalize_project_name` 统一用于 add/query/edit/delete；commands use/edit/delete 入口 trim；config 只存规范名；`app.rs` 改名输入 trim | `project_names_trimmed_consistently`（add/use/edit/del 四处 trim + config 断言无空格 + 删除后 current 清空）；smoke S9 |
| B4 | CLI `project edit a a` 有 early return，但 TUI 直接走 service 会报 already exists | service 缺少同名 no-op | service edit 在 trim 后 `if name == new_name { return Ok(true); }`（CLI early return 保留） | 单测 `project_name_normalization_trims_and_rejects_blank` + 静态 `grep "if name == new_name" src/services/project.rs:166`；TUI 交互未伪造 |
| B5 | `test_default_config_paths` 硬编码 `/tmp/tam-unit-home/...`，Windows 分隔符失败 | 测试断言使用 Unix 绝对字符串 | 改为 `std::env::temp_dir().join("tam-unit-home")`，期望值用 `tam_dir.join(DB_FILE).display().to_string()`；不访问真实 HOME | 3 个 config 单测在隔离 HOME 下 passed（`iteration-S9-verify.log`） |

S9 代码文件：`src/services/project.rs`、`src/services/task.rs`、`src/commands/project.rs`、`src/config.rs`、`src/app.rs`、`tests/cli.rs`。未改 schema/依赖/文档（SKILL 无需新增章节，README 不变）。

## 6. 手工项

| 项 | 状态 | 记录 |
|---|---|---|
| M1 config 保存失败（edit 当前项目） | **已执行**（CLI 非交互）：临时 HOME 中 `chmod 400 .tam/tam.config.toml`，`project edit alpha beta` | exit 1；stderr 含 ``run `tam project use beta` ``；DB 已改名 beta；config 仍 `current_project = "alpha"`；`chmod 600` 后 `project use beta` 恢复成功（`iteration-S5-tui-manual.log`） |
| M2 重名改名 → tip、不退出、选中保持 | **未执行（仅代码审查）** | `app.rs::submit_project` Err→tip；服务端先校验后 UPDATE，DB/config 不变 |
| M3 删除有任务项目 → tip 含计数、项目仍在 | **未执行（仅代码审查）** | `delete_project` Err→tip；服务 `COUNT` 守卫 |
| M4 删除当前项目 → 退出后 current 未设置 | **未执行（仅代码审查）** | `delete_project` Ok→清 current（仅对象==current） |
| M5 删除最后项目后 ↓/e/d → 无 panic | **未执行（仅代码审查）** | `get_project` 用 `get()` + 复位 stale index；空列表导航 return |
| M6 改名/删除非当前项目 → config 不变 | **未执行（仅代码审查）** | config 写入条件 `current_project.as_deref() == Some(操作对象)` |
| M7 任务操作失败 → tip、不退出 | **未执行（仅代码审查）** | 全部 task 服务调用 match + 中文 tip；删除成功复位 selected_task |
| M8 fmt 逐 hunk 人工审查 | 已执行（脚本 + 人工） | `iteration-S8-fmt.log` 中 pre-fmt→working tree 的 `diff -ru` 仅格式/import 顺序/空行，无逻辑行 |

> 不伪造截图或交互记录：TUI 在本环境无法驱动，M2–M7 明确标注「未执行/仅代码审查」。

## 7. 最终门禁结果（S8 + S9 复核后复跑）

| 门禁 | 目标 | 实际 | 证据 |
|---|---|---|---|
| `cargo build` | exit 0，warnings ≤4 且不新增 | exit 0，**4 warnings**（与基线同类：unused Color、is_* / set_title dead、tip_* 未读） | `iteration-gate-build.log` |
| `cargo test`（裸，真实 HOME） | 全绿、不写 `~/.tam` | **7 + 23 = 30 passed**；前后 sha256 一致（USER DATA UNTOUCHED） | `iteration-gate-test.log` |
| 隔离 HOME `cargo test` | 全绿 | 7 + 23 passed，exit 0 | `iteration-gate-test-temphome.log` |
| 真实 `~/.tam` sha256 | 与 `iteration-00-sha256-before.txt` 一致 | `diff -u` 为空（两文件哈希逐字节相同） | `iteration-final-sha256-after.txt` |
| `cargo fmt --check` | exit 0 | exit 0（S9 后复核仍为 0） | `iteration-S8-fmt.log` / `iteration-S9-fmt.log` |
| `cargo clippy`（裸） | 0 error，≤17 warnings 且无新增类别 | **0 error / 15 warnings**（类别与基线一致；useless_conversion×3、single_match×2、dead/unused×4 等）；`--all-targets` 信息性记录 | `iteration-gate-clippy.log` |
| schema diff | DDL 常量逐字节不变 | `SCHEMA_UNTOUCHED`；提取 CREATE 块与 HEAD 比较 `DDL_IDENTICAL` | `iteration-gate-schema.log` |
| README/SKILL/AGENTS grep | 1.0.13 命中、1.0.10 零命中、SKILL 三行为、AGENTS 测试段 | 全部命中 | `iteration-gate-schema.log` |
| CLI smoke 8 场景（隔离 HOME） | 逐条退出码/流向/文案 | **47 项断言 PASS=47 FAIL=0**；另 S9 回归 10/10 PASS；smoke 前后真实 sha256 不变 | `iteration-gate-smoke.log` |
| 软删约定 | 无 `DELETE FROM` | `grep -rn "DELETE FROM" src/` 零命中 | `iteration-gate-schema.log` |
| 进程环境隔离 | tests 不改进程 env | `grep -rn set_var tests` 零命中 | `iteration-gate-schema.log` |

## 8. Diff 与回滚

`git diff --stat`（HEAD `d395841` → 工作树，S9 后）：

```
 16 files changed, 527 insertions(+), 245 deletions(-)
 src/app.rs                 | 125 ++++++++++++++++---------
 src/cli.rs                 |  55 ++++++++---
 src/commands.rs            |   2 +-
 src/commands/project.rs    | 100 +++++++++++-----------
 src/commands/task.rs       |  24 +++---
 src/config.rs              |  95 ++++++++----------
 src/main.rs                |   7 +-
 src/services.rs            |   2 +-
 src/services/project.rs    | 130 +++++++++++++++++++----
 src/services/task.rs       | 189 ++++++++++++++++++++++--------
 src/sql.rs                 |  24 +++-
 src/ui.rs                  |   4 +-
 src/utils.rs               |   5 +-
 AGENTS.md / README.md / SKILL.md  ±(1/1/3)
```

- 未跟踪新增：`tests/`（731 行）、`docs/`（接管文档与证据，属预期）。
- 生产侧 `src` 净变化：+522 / −240（`git diff --numstat -- src`）。
- 回滚单位（未使用 git commit/reset/checkout 全仓）：
  - 单步：`git checkout -- <本步文件>` 回 HEAD → `git apply /tmp/forge-tam/patches/stepN.diff` 重新套上此前各步；`tests/cli.rs` 从 `/tmp/forge-tam/snapshots/stepN-pre/tests-cli.rs` 恢复（S1 前不存在则 `rm -f tests/cli.rs`）。
  - 整轮：按 S9→S1 逆序回滚；fmt 单独从 `/tmp/forge-tam/snapshots/pre-fmt/` 拷回，确保「回滚 fmt 不回滚功能」。
  - 快照目录：`/tmp/forge-tam/{patches,snapshots}/`（step1–step9、pre-fmt）。

## 9. 偏差（需 Lead 知晓）

1. **Diff 预算超出计划 §9**：`src` 合计 762 行（+522/−240，计划 ≤420）；`tests/cli.rs` 731 行（≤600）。超预算集中在 `config.rs`（测试迁移）、`services/task.rs`（raw-row helper + 内联单测）、`services/project.rs`（规范化 helper + 单测）、`app.rs`（错误处理展开）、`cli.rs`（i32/StatusArg + fmt）。无功能面扩张。
2. **`src/app.rs` 在 S2 被提前修改**：task id 由 `String` 改为 `i32` 后 app 调用点必须同步，否则无法编译；仅改 `id.to_string()`→`id` 四处，G6 的完整改造仍在 S5。
3. **集成用例 23（计划 19）**：S3 增加 `rename_to_soft_deleted_name_errors`（G2 目标冲突分支）；S9 增加 3 个复核用例（B1/B2/B3）。
4. **S5 的 M2–M7 未执行**：本环境无法驱动交互式 TUI；以静态断言 + 代码审查记录，未伪造交互。
5. **clippy 15（基线 17）**：类别一致、无新增类别；减少来自 app 调用点调整与 S9 的输入 trim 改动。`useless_conversion` 由 4 降到 3（同类），未做计划外 clippy 清理。
6. **S9 代码改动后 `cargo fmt --check` 仍为 0**，未再执行全仓 `cargo fmt`（无需格式化）。
7. **删除/恢复不引入事务**：COUNT→UPDATE、查含软删→UPDATE 均按 Q6 延后（无 WAL/事务），见 §10。

## 10. 残留风险

| 风险 | 说明 | 缓解 / 后续 |
|---|---|---|
| 删除守卫非原子 | `COUNT_ACTIVE_TASKS` 与软删 `UPDATE` 之间无事务，极端并发下可插入新任务后删除项目（单用户/单实例场景概率极低） | Q6 延后：WAL/busy_timeout/事务入 backlog |
| 恢复依赖 name UNIQUE | restore 以 name 定位软删行；schema 不变（Q2 明确不改 UNIQUE） | 同名 add 语义即恢复；改名到软删名给明确错误 |
| config 保存失败后 DB/config 分歧 | 按 Q4 有意不回滚 DB，只报错 + 恢复指令（M1 实拍） | 用户执行 ``tam project use <new>`` 自愈；未做自动重试/备份（Q5 延后） |
| 历史脏数据 | 旧版本可能已写入空白名/带空格名；S9 只保证新写入规范化 | `query_project`/`use` 现在 trim 后查询，带空格 current 可被 `project use` 自愈；DB 内历史空名行不自动清理（不删用户数据） |
| 1s 时间戳分辨率 | G1/G4 用 `sleep 1.1s` 取得更新时间差，测试总时长约 +2.2s | 连续 5 次验证稳定（S3 verify） |
| 非数字/≤0 id 退出码变化 | 1 → 2（clap 用法错误），按 Q3 允许 | smoke S5 固定断言；SKILL/README 未列退出码细节 |
| 错误流变化 | `print_error!` 改 stderr，依赖 stdout 抓错误的脚本需适配 | Q3 契约；`errors_use_stderr` 覆盖 |
| TUI 未实机回归 | M2–M7 仅代码审查 | Lead 可在有终端环境按 §6 清单复测；静态断言已留档 |
| 无参 `task list` 输出 | 已用「改造前 baseline → 改造后」`diff -u` 证明为空 | `iteration-S6-verify.log` |
| 预算超限 | 见 §9.1 | 不改断言/不删测试；如需压缩可另开重构项 |
