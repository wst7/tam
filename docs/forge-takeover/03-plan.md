# 03 · 改造计划（Gate 3）

- 输入：[00-discovery](00-discovery.md)、[01-asbuilt](01-asbuilt.md)、[02-gap](02-gap.md)（G1–G12、Q1–Q9、D 验收、F 对齐）。
- 基线：HEAD `d395841`；证据 `docs/forge-takeover/evidence/`（build 0/4 warnings、test 5 passed 但污染真实 HOME、fmt 1/22 hunk/8 文件、clippy 17 warnings/0 error）。
- 约束：只做 G1–G12；不改 schema DDL、不加依赖/dev-deps、不做 i18n/CI job/无关重构；**全程不 git commit、不改写历史、不写真实 `~/.tam`**。计划是执行步骤，不含代码。
- 步骤数：**9（Step 0 + S1–S8）**；自动化用例 **24**（集成 19 + 单测 5）；手工项 8；脚本门 3。

## 1. Step 0 基线冻结（只读 + 快照，不跑裸 cargo test）

目标：记录「当前基线 → 目标基线」，建立可对照的哈希/日志/补丁目录；用户数据只读。

```bash
cd /Users/wstreet7/project/tam
mkdir -p /tmp/forge-tam/{baseline,patches,snapshots,logs}
# 0.1 仓库状态与 HEAD（docs/ 为未跟踪接管文档，属预期）
{ echo "HEAD=$(git rev-parse HEAD)"; git status --porcelain=v1; git diff --stat; } \
  | tee /tmp/forge-tam/baseline/git.txt
# 0.2 真实用户数据指纹（只读；配置 + 库两个文件，日志不动）
shasum -a 256 ~/.tam/tam.config.toml ~/.tam/tam.db3 | tee /tmp/forge-tam/baseline/userdata-sha256.txt
# 0.3 基线质量（不跑裸 cargo test：会写真实 HOME；用隔离 HOME 取红基线）
REAL_HOME=$HOME; HOME=$(mktemp -d) RUSTUP_HOME="${RUSTUP_HOME:-$REAL_HOME/.rustup}" CARGO_HOME="${CARGO_HOME:-$REAL_HOME/.cargo}" cargo test 2>&1 | tee /tmp/forge-tam/baseline/test-temphome.log; echo "exit=${PIPESTATUS[0]}"
cargo clippy 2>&1 | tee /tmp/forge-tam/baseline/clippy.log; grep '^warning: ' /tmp/forge-tam/baseline/clippy.log | grep -vc 'generated [0-9]* warning'
cargo fmt --check >/dev/null 2>&1; echo "fmt_exit=$?"
```

| 基线项 | 当前值（S0 记录） | 目标值（S8 验收） |
|---|---|---|
| HEAD / 工作树 | `d395841`；docs/ 未跟踪 | 同 HEAD；功能 diff 仅本计划文件清单 |
| build | exit 0，4 warnings | exit 0，warnings ≤4 且不新增 |
| test（隔离 HOME） | 4 passed / 1 failed（`test_config_dir` 硬编码 `/Users/wstreet7`，00 §5.2 已复现）；裸跑会改写真实配置 | 隔离 HOME 全绿；裸跑安全且前后真实 sha256 不变 |
| fmt | exit 1（22 hunk / 8 文件） | exit 0 |
| clippy（裸） | 0 error / 17 warnings | 0 error，≤17 warnings 且无新增 |
| 用户数据 sha256 | 见 `userdata-sha256.txt` | 与 S0 完全一致 |

## 2. 步骤总览（顺序固定，不可重排）

| 步 | Gap | 一句话 | 新增自动化 |
|---|---|---|---|
| S1 | G10 | 隔离测试骨架先红：config 5 测试改纯 TOML/显式路径，新增 `tests/cli.rs`（std + `CARGO_BIN_EXE_tam` + 每用例独立 HOME/USERPROFILE，无 dev-deps） | 1 集成 + 3 单测 |
| S2 | G5+G7+G8 | 错误/id/校验地基：id 改正整数、not-found 友好文案走 stderr、trim 空名拒绝、时间解析 helper 不伪造 | 6 集成 + 2 单测 |
| S3 | G1+G2 | `updated` 读第 3 列；改名 DB 成功后才写 config（失败保持、保存失败给恢复指令） | 3 集成 |
| S4 | G3+G4 | 删除前计数活跃任务并拒绝；删除当前项目成功后清 config；同名 add 恢复软删项目 | 6 集成 |
| S5 | G6 | TUI 去 unwrap/索引安全/错误转 tip/删除后复位/仅当对象==current 时同步 config | 0（手工 6） |
| S6 | G9 | `task list --status <todo\|in-progress\|done>`；无参行为逐字节不变；非法值 exit 2 | 3 集成 |
| S7 | G12 | README 1.0.13、SKILL 三条新行为、AGENTS.md 测试段一行 | 0（脚本 grep） |
| S8 | G11 | `cargo fmt`（仅格式，最后做）+ 全量门禁 | 0（命令 + 人工审 hunk） |

## 3. 步骤明细（每步独立可回滚）

| 步 | 目标 | 涉及文件 | 行为变化 | 红测（先失败）→ 绿测 | 验证命令 | 回滚（应用前快照） | 影响面 / 风险 |
|---|---|---|---|---|---|---|---|
| S1 | G10：测试隔离骨架先红 | `src/config.rs`（仅测试段 + 抽 `default_for(home)` 私有 helper）、`tests/cli.rs`（新增）；`Cargo.toml` 不动 | 生产行为不变；单测不再触碰 Lazy/真实 HOME | 红：`HOME=$(mktemp -d)`（保留 RUSTUP_HOME/CARGO_HOME 指向真实工具链）跑 cargo test → `test_config_dir` failed；裸跑前后真实 sha256 变化（历史证据见 00 §5.2，S1 不重复裸跑）。绿：同上全绿 + sha256 不变 + `grep -rn "/Users/wstreet7" src tests` 零命中 | `REAL_HOME=$HOME; HOME=$(mktemp -d) RUSTUP_HOME="${RUSTUP_HOME:-$REAL_HOME/.rustup}" CARGO_HOME="${CARGO_HOME:-$REAL_HOME/.cargo}" cargo test; echo exit=$?`；D.2 哈希对照 | `patches/step1.diff` + `snapshots/step1-pre`（含 tests/cli.rs 时恢复；S1 前不存在则删除） | 并行安全依赖每用例独立 HOME；先落 1 个隔离 guard 用例（I19），其余用例随各步追加；单测不调用 `get_config`/`Config::load`/`Lazy`，避免依赖真实 HOME |
| S2 | G5+G7+G8：错误/id/校验地基 | `src/utils.rs`、`src/cli.rs`、`src/commands/{task,project}.rs`、`src/services/{task,project}.rs` | 业务错误走 stderr（成功仍 stdout）；id 用 `i32` + `value_parser!(i32).range(1..)`（非数字/≤0→exit 2；不存在→exit 1 `task <id> not found`）；项目 not-found 用 rusqlite 错误类型匹配（修掉字符串 `no rows returned`）；service 层 trim 后拒绝空名/空标题；时间 `NaiveDateTime` helper 失败返 Err | 红：`task done 999` stderr 空且 stdout 含 `Query returned no rows`；`task done abc` exit 1；`project add ""` exit 0 入库。绿：I10–I15 + U4/U5 | `cargo test --test cli not_found; cargo test --test cli reject_blank`；`tam task done abc; echo $?` | `patches/step2.diff` | 非数字 id 由 exit 1 变 clap exit 2（Q3 允许，脚本面变化）；`print_error!` 改 eprintln 影响所有错误流断言 |
| S3 | G1+G2：updated 列与改名一致性 | `src/services/project.rs`、`src/commands/project.rs` | `project list` 的 updated 读 `row.get(3)`；`project edit` 先 DB 后 config；源非活跃/目标为软删行 → 明确错误；config 保存失败 → stderr + `tam project use <new>` 恢复指令，不回滚 DB（M1 触发方式：仅 `chmod 400` config 文件，`.tam` 目录与 `logs/` 保持可写，避免 log4rs `expect` 先 panic） | 红：I1 现状 Updated==Created；I2 现状失败后 current=beta。绿：I1 两列独立；I2 current 仍 alpha 且 list 不变；I3 成功后 current=gamma；M1 手工 | `cargo test --test cli project_list_updated_column rename_` | `patches/step3.diff` | 1s 时间分辨率：I1 用 `add → sleep 1.1s → edit` 取得差值，连续 5 次验证稳定性 |
| S4 | G3+G4：删除保护与同名恢复 | `src/sql.rs`（仅新增查询常量）、`src/services/project.rs`、`src/commands/project.rs` | 删除：先取活跃项目（不存在/已删→not found），`COUNT(*) WHERE project_id=? AND is_deleted=0` >0 → stderr `project 'x' has N active task(s)` exit 1；=0 才软删；当前项目仅在 DB 成功后清 config；成功统一 SUCCESS。add：同名活跃→already exists；同名软删→`is_deleted=0` 恢复并输出 restored（不自动 use） | 红：I4 现状 exit 0 且项目被软删；I8 现状 exit 1。绿：I4–I9 全绿 | `cargo test --test cli delete_; cargo test --test cli add_` | `patches/step4.diff` + 从 `snapshots/step4-pre/tests-cli.rs` 恢复本步追加的删除/恢复用例 | COUNT 与 UPDATE 非原子（无事务，Q6 延后）；恢复依赖 name UNIQUE 不变 |
| S5 | G6：TUI 健壮性 | `src/app.rs`（不改 ui/） | 服务错误→tip 不退出；`get_project/get_task` 用 `get()`；删除后 `selected_*` 复位；`query_*` 错误不 panic；仅当被操作项目 == `config.current_project` 时同步 config（失败转 tip） | 红：静态 `grep -n "unwrap()" src/app.rs` 命中服务结果 7 处；手工 M2 现状 panic。绿：同 grep 仅剩非服务调用；M2–M7 全过 | `grep -n "unwrap()" src/app.rs`；`cargo run -- -i` 按 M2–M7 操作 | `patches/step5.diff` | TUI 无法自动化；需人工记录 + 静态断言，不阻塞 Gate 自动化 |
| S6 | G9：`task list --status` | `src/cli.rs`、`src/commands/task.rs`、`src/services/task.rs`、`src/sql.rs` | `TaskSubcommand::List { status: Option<StatusArg> }`（ValueEnum `todo\|in-progress\|done`）；无 `--status` 走原 SQL/文案；非法值 exit 2；沿用 `created DESC` | 红：I16 现状 `--status` 触发 clap unknown arg exit 2（与非法值同码但语义不同）；I17 需先固化基线输出。绿：I16 仅 1 行 Done、I17 与基线 diff 为空、I18 exit 2 | `cargo test --test cli task_list_`；`tam task list --status wip; echo $?` | `patches/step6.diff` | CLI help 增加一行；脚本无参调用零影响 |
| S7 | G12：文档与行为对齐 | `README.md`、`skills/tam-usage/SKILL.md`、`AGENTS.md`（一行） | 文档 only：README 示例 1.0.13；SKILL 写删除保护（含计数）、同名恢复、`--status` 取值；AGENTS 测试段改为「测试在 src inline + tests/cli.rs，须用隔离 HOME」 | 红：`grep "Version: 1.0.10" README.md` 命中；绿：零命中且 `grep "Version: 1.0.13"` 命中；SKILL 三处行为、AGENTS 测试段命中 | `grep -n "1.0.13\|1.0.10" README.md; grep -n "restored\|active task\|--status" skills/tam-usage/SKILL.md` | `patches/step7.diff` | 不改 SKILL 其余章节；AGENTS 仅一行，防止与 G10 冲突 |
| S8 | G11：fmt + 全量门禁 | 8 个既有文件（cli、commands.rs、commands/project、main、services.rs、sql、ui、utils）+ 新增文件 | 仅格式化，零语义变化（import 顺序可能变化，逐 hunk 人工确认） | 红：`cargo fmt --check` exit 1。绿：exit 0；`cargo build` 0；全量 test 不变；clippy ≤17 | `cargo fmt; cargo fmt --check; echo $?`；`git diff step8` 人工审查 | `patches/step8-fmt.diff` + `snapshots/pre-fmt/` | 必须最后做；独立 patch/快照，避免回滚 fmt 时连带功能 |


**S1 补充**：`tests/cli.rs` 结构 = `TempHome`（唯一临时目录 + `Drop` 清理）+ `run_tam(&[&str]) -> Output`（`env_clear` 后注入 `HOME`/`USERPROFILE`/`PATH`/`TZ`）+ 断言助手（exit/stdout/stderr 子串）。用例内不修改进程环境、不依赖执行顺序、不共享目录；新增文件不进 `git diff`，回滚时从每步 `snapshots/stepN-pre/tests-cli.rs` 恢复（S1 前不存在则删除）。

## 4. 回滚机制（可操作，与「不 commit」兼容）

```bash
# 每步「应用前」：把当前工作树 diff 快照为该步的恢复点（= stepN.diff）
git -C /Users/wstreet7/project/tam diff > /tmp/forge-tam/patches/stepN.diff
rsync -a --exclude .git --exclude target --exclude docs \
  /Users/wstreet7/project/tam/ /tmp/forge-tam/snapshots/stepN-pre/
# tests/cli.rs 未跟踪、不在 git diff 中：必须单独进快照（S1 前不存在则跳过）
[ -f tests/cli.rs ] && cp tests/cli.rs /tmp/forge-tam/snapshots/stepN-pre/tests-cli.rs || true
# 每步「验证通过后」：再存一份累计快照，供审计本步增量
git -C /Users/wstreet7/project/tam diff > /tmp/forge-tam/patches/stepN.after.diff
```

- **单步回滚（默认路径，等价于反向应用本步 patch）**：`git checkout -- <本步文件列表>` 回到 HEAD → `git apply /tmp/forge-tam/patches/stepN.diff` 重新套上此前各步改动 → 恢复未跟踪文件。因为 `stepN.diff` 就是「本步开始前的精确工作树」，该操作与「反向 apply 本步增量 patch」等价。
- **未跟踪的 `tests/cli.rs` 是特例**：它不在 `git diff` 中，且 S2–S6 会持续追加用例。每步应用前必须 `cp tests/cli.rs /tmp/forge-tam/snapshots/stepN-pre/tests-cli.rs`（S1 前不存在则跳过）；回滚时从该快照拷回；S1 回滚且快照不存在则 `rm -f tests/cli.rs`。禁止只按 `git diff` 回滚该文件。
- **审计本步增量**：`diff -u /tmp/forge-tam/patches/stepN.diff /tmp/forge-tam/patches/stepN.after.diff`（diff 的 diff，仅供审查，不用于回滚）。
- **首次创建（仅 S1）**：S1 前不存在 `tests/cli.rs`，S1 回滚 = `rm -f tests/cli.rs`；S2 起一律从 `snapshots/stepN-pre/tests-cli.rs` 恢复，不删除文件；`snapshots/step1-pre/` 留档。
- **S8 fmt 独立**：格式化前单独 `git diff > /tmp/forge-tam/patches/step8-fmt.diff` 并 `rsync` 8 个文件到 `snapshots/pre-fmt/`；回滚 fmt = 从 `snapshots/pre-fmt/` 拷回（功能步骤 patch 未动），确保「回滚 fmt 不会回滚功能」。
- **整轮回滚**：按 S8→S1 逆序执行单步回滚；最终 `git checkout -- . && rm -rf tests/`（保留 docs/ 接管文档）。禁止 `git stash`/`git reset --hard`/`git commit` 等会改写历史或暂存区的操作。

## 5. 关键实现口径（计划引用，不写代码）

| 项 | 口径 |
|---|---|
| id | `i32` + `value_parser!(i32).range(1..)`；非数字/≤0 → clap exit 2；数字不存在 → `task <id> not found` exit 1 |
| not-found | 用 `rusqlite::Error::QueryReturnedNoRows` 等错误类型匹配，禁止 `contains("no rows")`；项目侧现有字符串判断必须替换 |
| 错误流 | `print_error!` 改 `eprintln!`；成功输出仍 stdout；clap 用法错保持 exit 2 |
| 项目删除 | 先取活跃项目（不存在/已删→not found）→ COUNT 活跃任务 >0 拒绝并报数量 → 才软删；config 仅在 DB 成功后同步 |
| 项目 add | 活跃同名→already exists；软删同名→restore（`is_deleted=0`，触发器刷新 updated）并输出 restored；不自动 use |
| 项目 edit | DB 成功后才写 config；目标为软删行→明确错误；config 保存失败→报错 + 恢复指令，不回滚 DB |
| TUI | 仅被操作项目 == `current_project` 时同步 config；服务错误→tip；`get_project` 用 `get()`；删除后 selected 复位；`query_*` 错误不 panic |
| 空值校验 | service 层 `trim()` 后为空拒绝（同时覆盖 CLI 与 TUI） |
| 状态过滤 | clap `ValueEnum`（`todo\|in-progress\|done`）；无 `--status` 行为不变；非法值 exit 2 |
| 时间解析 | 抽公共 helper：`NaiveDateTime` `%Y-%m-%d %H:%M:%S` → UTC；失败返回 Err，禁止 `unwrap_or_else(|_| Utc::now())` |
| schema | 不改 `CREATE TABLE/TRIGGER` 常量；只新增查询常量（COUNT/RESTORE/状态过滤/软删目标查询） |

## 6. 测试矩阵（G1–G12 → 用例/手工项）

| Gap | 类型 | 用例名 / 手工项 | 红（改造前）→ 绿（改造后）断言 |
|---|---|---|---|
| G1 | 自动化 | `project_list_updated_column` | 现状 Updated==Created → 两列独立且 updated>created |
| G2 | 自动化 | `rename_failure_keeps_current_project`、`rename_success_updates_current_project`；**手工 M1** | 现状失败后 current 漂移 → 失败保持 alpha、成功 current=gamma；M1 config 保存失败提示恢复指令 |
| G3 | 自动化 | `delete_refuses_project_with_active_tasks`、`delete_current_project_clears_config`、`delete_missing_project_not_found`、`delete_allowed_when_only_soft_deleted_tasks` | 现状 exit 0 删库 → exit 1 + `N active task(s)`；当前项目删除后 current 清空；不存在→not found |
| G4 | 自动化 | `add_restores_soft_deleted_project`、`add_active_duplicate_already_exists` | 现状 already exists → exit 0 + restored；活跃重名仍报 already exists |
| G5 | 自动化 | `task_not_found_message`、`project_not_found_message`、`non_numeric_task_id_usage_error`、`errors_use_stderr` | 现状 raw SQL/exit 码错 → 精确文案、stderr、exit 1/2 |
| G6 | 手工 | M2–M7（TUI 六场景）+ 静态 `grep "unwrap()" src/app.rs` | 现状 panic → tip 保留状态、无 panic、config 同步仅限当前项目 |
| G7 | 自动化 | `reject_blank_project_name`、`reject_blank_task_title` | 现状 exit 0 入库 → exit 1 且行数不变 |
| G8 | 自动化 | U4 `sqlite_timestamp_parsed_as_utc`、U5 `sqlite_timestamp_invalid_is_err` | 现状静默 now() → 真实 UTC；非法字符串 Err；grep fallback 零命中 |
| G9 | 自动化 | `task_list_status_filter`、`task_list_without_status_unchanged`、`invalid_status_value_usage_error` | 无参输出与基线 diff 为空；过滤 1 行；非法 exit 2 |
| G10 | 自动化+脚本 | U1–U3（config 纯 TOML）、I19 `binary_writes_only_under_injected_home`、D.2 哈希/grep | 现状临时 HOME 失败 + 裸跑污染 → 全绿 + sha256 不变 |
| G11 | 命令+人工 | `cargo fmt --check`；M8 逐 hunk 审查 | exit 1 → exit 0 且 build/test 不回归 |
| G12 | 脚本 | README/SKILL/AGENTS grep | 1.0.10 命中 → 1.0.13 + SKILL 三行为 + AGENTS 测试段命中 |

**必须自动化**：G1–G5、G7–G10（19 集成 + 5 单测 = 24）。**仅手工**：G6 TUI（M2–M7）、G2 的 config 保存失败分支（M1）、G11 格式人工审查（M8）。**脚本门**：G11 fmt、G12 grep、schema diff。
F 对齐：R1→S3（+S4 删除侧）、R2→S4、R3→S3、R4→S4（+S3 目标冲突错误）、R5→S5、R6→S1、R7→S8（部分，CI job 入 E）、R8→S2、R9→S2、R10→S4（部分，重复删除提示入 E）、R11→S3+S4、R16→S2；R12–R15、R17–R20 延后（02 §E）。
M1 手工：临时 HOME 中**只读配置文件本身**（`chmod 400 <home>/.tam/tam.config.toml`；目录与 logs/ 保持可写，否则 log4rs 会先 panic）→ `project edit a b` → 断言 stderr 含 `tam project use b` 恢复指令、DB 已改名、config 未更新 → `chmod 600` 复原。M2–M7 见 02 §D.4 六条，逐条记入 `iteration-S5-tui-manual.log`。

## 7. 验收命令（基线 → 目标；复制即用）

```bash
cd /Users/wstreet7/project/tam
# 7.1 build
cargo build 2>&1 | tee /tmp/forge-tam/logs/build.log; echo "build_exit=${PIPESTATUS[0]}"
# 7.2 test：S1 后裸跑也应安全；另跑隔离 HOME
cargo test 2>&1 | tee docs/forge-takeover/evidence/iteration-gate-test.log; echo "test_exit=${PIPESTATUS[0]}"
REAL_HOME=$HOME; HOME=$(mktemp -d) RUSTUP_HOME="${RUSTUP_HOME:-$REAL_HOME/.rustup}" CARGO_HOME="${CARGO_HOME:-$REAL_HOME/.cargo}" cargo test 2>&1 | tee docs/forge-takeover/evidence/iteration-gate-test-temphome.log
# 7.3 真实用户数据前后 sha256（每组验证都跑）
before=$(shasum -a 256 ~/.tam/tam.config.toml ~/.tam/tam.db3)
cargo test >/dev/null 2>&1
after=$(shasum -a 256 ~/.tam/tam.config.toml ~/.tam/tam.db3)
[ "$before" = "$after" ] && echo USER_DATA_UNTOUCHED || echo "USER_DATA_CHANGED"
# 7.4 fmt
cargo fmt --check; echo "fmt_exit=$?"
# 7.5 clippy：裸命令作门禁（基线 17，目标 ≤17 且无新增）；--all-targets 仅信息性
cargo clippy 2>&1 | tee /tmp/forge-tam/logs/clippy.log
# 基线 17（排除 clippy 末尾 generated 汇总行）；目标 ≤17 且无新增
grep '^warning: ' /tmp/forge-tam/logs/clippy.log | grep -v 'generated [0-9]* warning' | wc -l
cargo clippy --all-targets 2>&1 | tail -3
# 7.6 schema/软删约定：DDL 常量逐字节不变
git diff -U0 src/sql.rs | grep -E 'CREATE (TABLE|TRIGGER)' && echo SCHEMA_CHANGED || echo SCHEMA_UNTOUCHED
# 7.7 文档与硬编码
grep -n "Version: 1.0.13" README.md; grep -n "Version: 1.0.10" README.md || echo README_OK
grep -n "restored\|active task(s)\|--status" skills/tam-usage/SKILL.md
grep -rn "/Users/wstreet7" src tests || echo NO_HARDCODED_HOME
```

> 注意：直接 `HOME=$(mktemp -d) cargo test` 会因 rustup 找不到默认工具链而失败（本次 Step 0 实测 exit 1：`rustup could not choose a version of cargo`）。必须保留 `RUSTUP_HOME`/`CARGO_HOME` 指向真实工具链，只把 `HOME` 指到临时目录。

**CLI smoke 8 场景（S8 收尾执行，逐条断言 退出码/流/关键文案；1>/tmp/o 2>/tmp/e）**

| # | 命令序列（隔离 HOME） | 期望 |
|---|---|---|
| S1 | `project add p1` → `use p1` → `project list` | exit 0；Created/Updated 独立（G1） |
| S2 | `project add p2` → `use p1` → `project edit p1 p2` | exit 1；stderr UNIQUE；`project current`=p1（G2） |
| S3 | p1 下 2 活跃 task → `project del p1` | exit 1；stderr `project 'p1' has 2 active task(s)`；任务未动（G3） |
| S4 | p2 无活跃任务 → `project del p2` → `project add p2` | exit 0；输出含 restored；list 出现 p2（G4） |
| S5 | `task done 999` / `task done abc` / `project use nope` | 1/stderr `task 999 not found`；2（clap）；1/stderr `project 'nope' not found`；无 SQL 原文（G5） |
| S6 | `project add ""` / `task add "   "` | exit 1，stderr 可读，库无新行（G7） |
| S7 | 三状态各 1 → `task list --status done` / `task list` / `--status wip` | 1 行 Done；3 行且与基线一致；exit 2（G8/G9） |
| S8 | 删当前项目后 `project current` | exit 1 + `No current project set...`（G3/Q4） |

## 8. 证据留存（供 05-regression 引用）

统一放 `docs/forge-takeover/evidence/`，命名 `iteration-<step>-<kind>.log`，`2>&1 | tee` 合并输出并保留退出码：

| 时机 | 文件 | 内容 |
|---|---|---|
| S0 | `iteration-00-baseline.log`、`iteration-00-sha256-before.txt` | git 状态/HEAD、基线质量、临时 HOME 测试红证据、用户数据哈希 |
| S1 | `iteration-S1-red.log`、`iteration-S1-green.log` | 隔离失败红证据 → 隔离全绿 |
| S2–S7 | `iteration-S2-verify.log` … `iteration-S7-verify.log` | 每步红/绿用例输出 + 该步验证命令输出 |
| S5 | `iteration-S5-tui-manual.log` | M2–M7 人工操作与 tip 文案记录 |
| S8 | `iteration-S8-fmt.log`、`iteration-gate-{build,test,test-temphome,clippy,schema,smoke}.log`、`iteration-final-sha256-after.txt` | 格式与最终门禁 |
| 收尾 | `iteration-final-report.md`（可选） | 与 02 D.1 基线→目标对照表 |

evidence/ 目录未跟踪、不入库；每步验证必须先 tee 再判定，禁止事后补日志。

## 9. Diff 预算与文件清单

| 文件 | 预计净改 | 内容 | 上限 |
|---|---|---|---|
| `tests/cli.rs`（新增） | ~520 行 | TempHome/run_tam/断言助手 + 19 集成用例 | ≤600 |
| `src/config.rs` | +15/−45 | 5 个旧测试替换为 3 个纯 TOML/显式路径测试；抽 `default_for(home)` 私有 helper | ≤60 |
| `src/utils.rs` | ±1 | `print_error!` println→eprintln | ≤5 |
| `src/cli.rs` | +25 | id `value_parser`（4 处）、`--status` ValueEnum | ≤40 |
| `src/commands/project.rs` | +45/−25 | edit 顺序、delete 流程、空名校验、输出统一 | ≤80 |
| `src/commands/task.rs` | +20/−10 | 空标题校验、id 透传、status 透传 | ≤35 |
| `src/services/project.rs` | +70/−20 | updated 列、错误类型匹配、COUNT、restore、时间 helper | ≤100 |
| `src/services/task.rs` | +45/−15 | query_task 错误映射、时间 helper、list 状态参数 | ≤70 |
| `src/sql.rs` | +18 | 仅新增查询常量（COUNT/RESTORE/状态过滤/含软删查询） | ≤25 |
| `src/app.rs` | +85/−25 | unwrap→tip、get() 边界、selected 复位、config 同步 | ≤120 |
| `README.md`/`SKILL.md`/`AGENTS.md` | ≤10 行 | 版本、三条行为、测试段一行 | ≤12 |
| **合计** | 生产 ≤420、测试 ≤600 | — | 超 20% 停并回报 |

**明确不做**：schema DDL/触发器改动、不新增生产侧 `TAM_HOME`（测试只用进程级 HOME 注入）、依赖或 dev-deps、i18n/鼠标/设置页、CI job、WAL/busy_timeout、config serde default、`content` 字段、主题弹窗、发布链、无关 clippy 清理、`tam config` Debug→TOML 重构。

## 10. 风险表

| 风险 | 触发场景 | 缓解 | 检出方式 |
|---|---|---|---|
| 集成测试并行 / HOME 注入失败 | 多用例共享目录或进程环境被改 | 每用例独立 `TempHome`；`Command` 显式注入 HOME/USERPROFILE；不改进程 env | 默认并行与 `--test-threads=1` 双跑结果一致；`grep -rn "set_var" tests` 为零 |
| log4rs 写入临时 HOME | 每次 tam 调用都会建 `logs/` | 每用例独立临时目录，`Drop` 清理；不检查/不共享日志 | 测试后 `/tmp` 下残留为 0；真实 `~/.tam/logs` mtime 不变 |
| 时间戳 1s 分辨率 | G1 依赖 add→edit 的 updated 差值 | S3 用 `sleep 1.1s`、连续 5 次复跑；仍不稳则改手工 fixture 并在日志记录 | 连续 5 次 G1 全绿 |
| clap id 类型变更影响 help/脚本 | 非数字 id 从 exit 1 变 exit 2；help 文本变化 | 提前在 S7/SKILL 说明；S5 smoke 固定断言；无参/正常路径不变 | `tam task --help` diff 仅预期行；S5 场景通过 |
| TUI 无自动化 | G6 只能人工 | 静态 grep 断言 + M2–M7 人工清单；不阻塞自动化门禁 | `iteration-S5-tui-manual.log` 签字；grep 结果留档 |
| fmt 噪声/误回滚 | S8 与功能改动混在一起 | S8 最后单独快照/patch；只拷回格式文件 | `cargo fmt --check` 前后；`git apply -R` 只影响格式 |
| 真实 HOME 污染复发 | 任何裸 `cargo test`/二进制命令 | S1 前禁裸跑；S1 后每次验证先记录 sha256 再执行；发现变更立即停 | D.2 哈希对照 + `USER_DATA_CHANGED` |
| 文档与代码不同步 | S7 先于最终实现定稿 | S7 放在 S6 后、逐条 grep 对照 `tam -h` | G12 grep；人工通读一遍 |

## 11. 与 02-gap 的偏差与口径补充（需 Lead 知晓）

1. **clippy 门禁**：02 §D.1 写 `cargo clippy --all-targets | grep -c` 计数；本计划按 Lead 口径改为**裸 `cargo clippy`** 与基线 17 比较作门禁，`--all-targets` 仅信息性输出。
2. **G10 的 `Cargo.toml` 改动取消**：02 §G10 验证列写「Cargo.toml（如需 dev-deps）」；本轮不加任何 dev-deps，`tests/cli.rs` 只用 std + `CARGO_BIN_EXE_tam`，`Cargo.toml` 不动。
3. **顺序调整**：02 §C.2 把 G7/G8 排在 G3/G4 之后；本计划按 Lead 顺序把 G5+G7+G8 合并为 S2（错误/校验地基），G1+G2 为 S3、G3+G4 为 S4，减少 `services/{project,task}.rs` 反复改动。
4. **G5 错误分类变化**：非数字/≤0 id 由现状 raw DB exit 1 改为 clap exit 2（02 §B 补充 4 已记录），S2 落地并在 smoke S5 断言。
5. **G12 增加 AGENTS.md 一行**：02 §偏差 1 建议项，Lead 已明确纳入 S7；只改测试段一行，不扩范围。
6. **G1 fixture 选择**：02 给「fixture / 1s sleep 二选一」，本计划选 sleep（无 dev-deps、不依赖 sqlite3 CLI），风险与缓解见 §10。
7. **隔离 HOME 跑 cargo test 需保留工具链环境**：Step 0 实测直接 `HOME=$(mktemp -d) cargo test` 会被 rustup 拒绝（找不到默认 toolchain）；命令已改为 `HOME=tmp RUSTUP_HOME=真实 CARGO_HOME=真实 cargo test`。集成测试子进程仍只注入临时 HOME/USERPROFILE，不依赖该命令。
8. **哈希命令平台差异**：计划用 macOS `shasum -a 256`；非 macOS 环境替换为 `sha256sum`，其余命令不变。
