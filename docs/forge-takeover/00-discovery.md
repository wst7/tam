# Gate 0 现状勘察 —— tam 1.0.13（brownfield 接管）

> 勘察者：analyst ｜ 2026-11-08 15:15–15:25 CST ｜ 仓库 /Users/wstreet7/project/tam（HEAD d395841）
> 范围：只读源码 + 只读/隔离复现；未改源码、未改 `/Users/wstreet7` 下用户数据。基线证据见 `docs/forge-takeover/evidence/`；文中「本次复现」指隔离 `HOME=/tmp/forge-tam/...` 运行 `target/debug/tam`（附录 B）。

## 1. 项目概览

### 1.1 基本信息

| 项 | 值 | 来源 |
|---|---|---|
| 包名/版本 | tam 1.0.13 | Cargo.toml:2-3 |
| edition / license | 2021 / MIT OR Apache-2.0 | Cargo.toml:4,9 |
| 形态 | 单 crate 二进制（无 [lib]） | Cargo.toml |
| 源码规模 | 21 个 .rs，2047 行 | `find src -name '*.rs' \| xargs wc -l` |
| 直接依赖 | 16 个（clap/ratatui/crossterm/rusqlite(bundled)/log4rs/tabled/…） | Cargo.toml:19-36 |
| 测试布局 | 仅 config.rs 5 个 inline 测试；无 tests/ | test.log:57-62；`ls tests` 不存在 |
| 工具链 | rustc 1.94.1 / cargo 1.94.1 | versions.txt |

### 1.2 目录结构

```
tam/  ├─ src/（全部代码，21 文件/2047 行）  ├─ docs/forge-takeover/（本次接管文档+evidence/，新增未提交）
     ├─ skills/tam-usage/SKILL.md（119 行）  ├─ .github/workflows/（release.yml 由 cargo-dist 生成、publish-crates.yml）
     ├─ assets/ wix/（图标与 MSI 资源）  ├─ Cargo.toml rustfmt.toml dist-workspace.toml release.{sh,toml}
     └─ README.md AGENTS.md CHANGELOG.md design.txt
```

### 1.3 模块职责与行数

| 模块 | 行数 | 职责 / 关键符号 |
|---|---:|---|
| main.rs | 65 | 启动编排：日志→建库→clap→交互/命令/帮助（main） |
| cli.rs | 111 | clap 命令/别名定义与分发（Cli, Commands, *Subcommand） |
| commands/project.rs | 96 | 项目命令编排，含 current_project 配置维护 |
| commands/task.rs | 48 | 任务命令编排，依赖当前项目 |
| commands/config.rs | 8 | 打印配置（config） |
| services/project.rs | 123 | 项目 SQL 执行与行映射（add/list/query_project/delete/edit） |
| services/task.rs | 149 | 任务 SQL 与状态流转（add/list/query_task/start/done/edit/delete） |
| db.rs / sql.rs | 35 / 89 | 连接创建、建表初始化（get_db/init_db）；15 个 SQL 常量（软删、updated 触发器） |
| model.rs | 157 | Project/Task/TaskStatus、表格渲染、SQL 映射与 impl |
| config.rs | 151 | 配置加载/保存（once_cell Lazy）+ 5 个 inline 测试 |
| logs.rs | 46 | log4rs 滚动日志（10MB × 5，init_logging） |
| app.rs | 450 | TUI 状态机与事件处理（App, handle_event） |
| ui/{project_screen,task_screen,components}.rs | 137/165/98 | 项目屏、三列状态任务屏、提示/确认弹窗与居中布局 |
| theme.rs / utils.rs | 64 / 36 | 明暗主题色板；print_success/info/error 宏 |

分层：入口层 main.rs（先副作用后解析参数，main.rs:22-30）→ 命令层 commands/*（编排+配置读写）→ 服务层 services/*（SQL/行映射/弱校验）→ 数据层 db.rs+sql.rs（每次新建 Connection，db.rs:11-21；SQL 全为字符串常量）。TUI 走 app.rs → ui::draw → services，绕过 commands 层。

## 2. 关键入口与运行方式

### 2.1 启动时序（main.rs）

| 顺序 | 位置 | 行为 |
|---|---|---|
| 1 | main.rs:22 | `logs::init_logging()`；配置加载失败即 exit(1)（config.rs:15-24） |
| 2 | main.rs:23-29 | `init_db()` 建 project/task 表与 updated 触发器（db.rs:23-30） |
| 3 | main.rs:30-38 | `Cli::parse()` → `-i` 进 interactive_mode；有子命令进 command_mode；都没有则打印 help |

**副作用**：第 1、2 步早于参数解析，故 `tam --help` / `tam -V` 也会创建 `~/.tam/`、配置、db3、logs（本次复现：隔离 HOME 跑 `--help` 后生成 4 个文件，exit 0）。运行路径：CLI（main.rs:51-64，失败 exit 1）、TUI（`tam -i`，main.rs:41-49，q 退出；与子命令同给时交互优先）、帮助（无参数，main.rs:35-38，exit 0）。

### 2.2 CLI 命令面与默认子命令

| 组 | 命令（别名） | 默认 / 位置 |
|---|---|---|
| 项目 | add(a) / list(l) / use(u) / del(d) / current(c) / edit(e) | 无子命令 = list（cli.rs:67）；定义 cli.rs:44-63 |
| 任务 | add(a) / list(l) / edit(e) / del(d) / start / done | 无子命令 = list（cli.rs:102）；定义 cli.rs:78-98 |
| 其他 | config；全局 -i/--interactive、-h、-V | cli.rs:16-41；config → commands/config.rs:3 |

TUI 按键（app.rs）：项目屏 ↑↓ 导航、Enter 进任务屏、a 新增、e 重命名、d 删除确认、t 切主题、q 退出（app.rs:114-145）；任务屏 ↑↓ 导航、←→ 切状态列、a/e/d/s/c/t、Esc 返回、q 退出（app.rs:181-214）；弹窗 Enter 保存·Esc 取消（app.rs:82-111,150-179）。

### 2.3 配置与数据文件

| 文件 | 默认路径 | 来源 |
|---|---|---|
| 配置 | ~/.tam/tam.config.toml | config.rs:8-9,76-83 |
| 数据库 | ~/.tam/tam.db3 | config.rs:13,48 |
| 日志 | ~/.tam/logs/tam.log（10MB 触发，保留 5 份） | config.rs:11-12；logs.rs:18-25 |

配置字段 db_file / current_project / logfile_path / logfile_pattern / theme（config.rs:30-37），经 `Lazy` 加载，任何失败 `exit(1)`（config.rs:15-24）。

## 3. 构建/测试/lint/运行基线（2026-11-08，证据 docs/forge-takeover/evidence/）

| # | 命令 | 退出码 | 摘要 | 证据 |
|---|---|---:|---|---|
| B1 | cargo build | 0 | 24.35s；4 warnings（unused import Color；dead code：is_done/is_todo/is_in_progress、set_title；tip_bg/tip_fg 未读） | build.log, build.exit |
| B2 | cargo test | 0 | 5 passed / 0 failed（全在 config）；5 warnings（test 配置多一条 db.rs:34 unused import）。**该次运行使用真实 HOME，改写了用户配置，见 §5** | test.log, test.exit |
| B3 | cargo fmt --check | **1** | 22 个 diff hunk / **8 个文件**（cli 2、commands/project 9、commands.rs 1、main 3、services.rs 1、sql 4、ui 1、utils 1） | fmt.log, fmt.exit |
| B4 | cargo clippy | 0 | 17 warnings / 0 errors；官方提示 13 条可 `--fix` | clippy.log, clippy.exit |
| B5 | rustc / cargo | — | 1.94.1 / 1.94.1 | versions.txt |

**与「已知事实」的差异**：口头基线记 fmt 为「6 处 diff（services.rs、sql.rs×3、ui.rs、utils.rs）」，实测 `fmt.log` 为 **22 处 / 8 文件**，额外涉及 cli.rs、main.rs、commands.rs、commands/project.rs。方向不变（fmt 失败），但修复面被低估。

**fmt 失败结论与建议**：属格式卫生问题非行为缺陷（rustfmt.toml 仅 3 条约定，无争议）；CI 仅有 release.yml/publish-crates.yml，**无 fmt/clippy/test 门禁**，当前不阻断发布，但违反 AGENTS.md:24-25。建议 Gate 3 做「仅格式化、单独提交」的机械修复（零行为变更，便于后续 diff 归因），再评估把 `fmt --check` 纳入 CI；不建议长期豁免——接管期功能改动会持续混入无关格式 diff。

clippy 17 warnings（13 条可自动修）：useless conversion×4（app.rs:313,323,336,347）、match→if×2（app.rs:83,151）、useless format!×2（config.rs:77,80）、direct ToString（model.rs:124）、无用 cast（app.rs:422）、unneeded return（db.rs:20）、borrowed expr（main.rs:37）、returning let binding（main.rs:64）、dead code/unused import×4（ui/project_screen.rs:3；model.rs:70,135；theme.rs:14）。建议与 fmt 修复分开提交。

## 4. 已知缺陷清单

分级：P0 = 破坏用户数据或核心流程不可用；P1 = 功能明显错误/易崩溃/质量门缺失；P2 = 体验、一致性与工程质量。证据栏「smoke」= `evidence/baseline-smoke.log` 行号；「本次复现」= 附录 B；TUI panic 类为代码路径证据（本次未做交互式触发，已标注）。

### 4.1 P0

| ID | 现象 | 证据 | 根因（file:line） | 影响 | 建议阶段 |
|---|---|---|---|---|---|
| P0-1 | 重命名项目先写配置、后改库，失败不回滚：`project edit alpha beta` 先把 current_project 改 beta 再报 UNIQUE 失败，随后 `task add` 写入 beta | smoke:32-47；本次复现 `project edit alpha ""` → 先报 "Current project updated to ''"，随后 UNIQUE 失败，配置残留空串 | commands/project.rs:76-94（save 在 :81，DB edit 在 :94），无事务/回滚 | current_project 与库不一致，任务写进错误项目；用户只见「改名失败」不知配置已变 | Gate 2 定一致性契约；Gate 3 修 |
| P0-2 | 删除项目不校验关联任务（TODO 未实现），静默成功，任务成孤儿且无恢复路径 | smoke:48-61；本次复现：beta 有 1 个活跃任务，`project del beta`（非当前项目）exit=0，库中 project 3 is_deleted=1 而 task 2 project_id=3 仍活跃 | commands/project.rs:49-50 的 TODO；sql.rs:57-59 仅置 is_deleted；db.rs 未开 `PRAGMA foreign_keys`（本次实测 0） | 逻辑孤儿任务无法在列表/TUI 出现，只能靠已知 id 盲操作；数据不可回收 | Gate 2 决策删除语义；Gate 3 实现 |
| P0-3 | 测试直接读写真实 `~/.tam` 且断言硬编码 `/Users/wstreet7`：任何 `cargo test` 都改写用户 current_project | test.log:57-62；基线事故：用户配置被改成 myproject；本次隔离复现：测试二进制在 `HOME=/tmp/...` 下写 config 且 `test_config_dir` 断言失败（left /tmp/... right /Users/wstreet7/.tam） | config.rs:106-114（硬编码 :112）、config.rs:134-141（test_edit_config 保存 current_project="myproject"）；测试并行无隔离 | 用户数据被测试污染；测试在其他机器/CI 必失败，等于没有可移植测试 | Gate 2 验收硬门槛（先隔离再谈新增测试） |

### 4.2 P1

| ID | 现象 | 证据 | 根因（file:line） | 影响 | 建议阶段 |
|---|---|---|---|---|---|
| P1-1 | 项目列表 Updated 恒等于 Created | smoke:11-17；**真实数据实证**：拷贝用户库到 /tmp 后 `project list` 显示 pastoid Updated=2026-05-20，库内实为 2026-05-24（本次复现） | services/project.rs:39-47：updated 也解析 `row.get(2)`（应为 3） | 列语义失效，看不到最近变更 | Gate 3 快修 |
| P1-2 | 软删项目名仍占 UNIQUE：删除后重建同名报 already exists；无 restore/undelete | smoke:57-59；sql.rs:1-9 UNIQUE(name) 不区分 is_deleted | sql.rs:4 + sql.rs:57-59；CLI 无恢复入口（cli.rs:44-63） | 删错无法自愈，只能手改 SQLite（违反软删约定） | Gate 2 定唯一性/恢复模型；Gate 3 实现 |
| P1-3 | 无效 id / 不存在项目泄漏原始 DB 错误 `ERROR Query returned no rows`；项目侧映射串写错导致失效 | smoke:29-31；本次复现 `task done abc`、`task done 999`、`project use nope` 均输出 "Query returned no rows" 混合文本 | services/task.rs:75-97（无错误映射）；services/project.rs:74-80（匹配 "no rows returned" 与实际错误不符） | 用户无法区分不存在/已删除/DB 故障；脚本无法稳定判定 | Gate 3（随错误契约） |
| P1-4 | TUI 多处 `unwrap()` 会 panic：重名编辑、增删改任务/项目、每帧查库；另有选中下标越界 | 代码路径：app.rs:324,336,349,358,395,417,422；app.rs:428 `self.projects[selected]`（删除后 selected 不复位） | app.rs 各 submit/delete/query 函数；delete_project/delete_task 未复位 selected（app.rs:356-360,392-398） | 一次可预期业务失败（如重名）即终止 TUI，且无错误提示 | Gate 3 |
| P1-5 | TUI 删除当前项目不清 `current_project`（CLI 有清理），留下悬挂指针 | app.rs:356-360 直接调 service；对比 commands/project.rs:52-63 有清理 | 同一「配置-数据一致性」缺口 | 退出 TUI 后 `task add/list` 报原始错误；重启不自愈 | Gate 2 定契约；Gate 3 修 |
| P1-6 | 测试覆盖近乎为零：仅 config 5 个测试；services/sql/db/commands/app/ui 无测试；无 tests/ | test.log:57-62；db.rs:32-35 测试模块为空；AGENTS.md:15 | 测试策略缺失；Cargo.toml 排除 tests/**/* | 任何重构无回归网；基线正确性只能靠手工 smoke | Gate 2 定覆盖口径；Gate 3 补 |
| P1-7 | 输入校验缺失：空项目名、空任务标题可入库；`project add ""` 产生空白行；`edit alpha ""` 先改配置再失败 | 本次复现：`project add ""` SUCCESS 且列表出现空行；`task add ""` SUCCESS | cli.rs:47,82 无校验；services 层无校验；DB 无 CHECK | 脏数据进入用户库，列表/TUI 出现空行；与 P0-1 叠加放大 | Gate 3（策略可 Gate 2 定） |
| P1-8 | 配置无向后兼容：缺字段（旧配置）或 TOML 损坏 → 直接 exit(1)，无默认合并、无备份 | 本次复现：删掉 theme 字段后 `project list` 输出 "missing field `theme`" 并 exit 1；损坏 TOML 同样 exit 1 | config.rs:15-24（Lazy 内 exit）、config.rs:63-75（严格反序列化）、Config 无 serde default | 升级或手改配置即让 CLI 完全不可用，需删配置重建（丢 current_project） | Gate 2 定迁移策略 |

### 4.3 P2

| ID | 现象 / 根因 | 证据 |
|---|---|---|
| P2-1 | fmt 失败：22 hunk / 8 文件 | fmt.log；§3 |
| P2-2 | clippy 17 warnings（13 可自动修），含 direct ToString（model.rs:124）、无用 cast（app.rs:422） | clippy.log；§3 |
| P2-3 | README 帮助示例写 Version 1.0.10，实际 1.0.13 | README.md:66 vs Cargo.toml:3；本次 `tam -V`=1.0.13 |
| P2-4 | 错误走 stdout：print_error! 用 println!，成功/失败无法按流区分 | utils.rs:15-24（:18） |
| P2-5 | 任何调用（含 `--help`/`-V`）先创建 ~/.tam/{config,db3,logs} | main.rs:22-31；本次复现 |
| P2-6 | 并发无防护：journal_mode=delete、无 busy_timeout、无 foreign_keys（实测 0）；配置整文件覆盖写，多实例 last-write-wins | db.rs:11-21；本次实测 pragma；config.rs:84-90 |
| P2-7 | `tam config` 打印 Rust Debug 而非 TOML；design.txt 约定 `config [key [value]]` 未实现 | commands/config.rs:3-8；design.txt:15 |
| P2-8 | 任务时间解析用 `parse::<DateTime<Utc>>()` 解析 SQLite "YYYY-MM-DD HH:MM:SS" 必失败并静默回退 Utc::now()；与项目服务两套风格 | services/task.rs:88-94 vs services/project.rs:30-47 |
| P2-9 | TUI 每帧查库（render 内调 query_projects/query_tasks），render 内还改状态（tip_message） | ui/project_screen.rs:29；ui/task_screen.rs:33,36 |
| P2-10 | 主题半接线：tip_bg/tip_fg 从未使用；弹窗硬编码 Color::Black/LightYellow，亮色主题下与色板不一致 | theme.rs:14-15；ui/components.rs:49,52,60,81,84,95 |
| P2-11 | 状态流转无约束：done 后可 start 回退，Todo 可直接 done（仅防重复同态） | services/task.rs:98-127 |
| P2-12 | 软删过滤不彻底：UPDATE/DELETE 项目只按 name、不按 is_deleted=0，对已删项目仍返回成功 | sql.rs:57-63；services/project.rs:84-98,109-123 |
| P2-13 | 宏隐式依赖 colored trait：`.green()` 能编译只因 print_info! 内部 `use colored::Colorize`；换直接写法即编译失败 | commands/project.rs:41；utils.rs:29-31 |
| P2-14 | 日志初始化失败即 panic（expect），无级别/格式配置入口 | logs.rs:23,33,43,45 |
| P2-15 | init_db 逐条执行无事务、无 schema_version 迁移机制，中断可留半套 schema | db.rs:23-30 |
| P2-16 | 项目列表无 ORDER BY，顺序依赖实现细节；TUI 选中索引与列表顺序强耦合 | sql.rs:44-46；app.rs:426-432 |
| P2-17 | 文档漂移：AGENTS.md 称 inline 测试「e.g. src/db.rs」但 db.rs 测试模块为空；README 特性含「delete projects」而行为有缺陷 | AGENTS.md:15；db.rs:32-35；README.md:17 |

## 5. 用户数据安全与测试隔离

### 5.1 真实数据画像（只读，2026-11-08 15:22）

| 项 | 值 | 证据 |
|---|---|---|
| 项目 / 任务 | 项目 5 行（4 活跃 runx/tam/pastoid/panda-zen + 1 软删 usenom）；任务 37 行（31 活跃 + 6 软删） | `sqlite3 "file:$HOME/.tam/tam.db3?mode=ro"` |
| 各项目活跃任务 | runx 1、tam 13、pastoid 15、panda-zen 2、usenom 0 | 同上 |
| 活跃孤儿任务 | 当前 0（usenom 下唯一任务也已软删）；但结构缺口真实存在，P0-2 复现可直接制造 | 同上 |
| updated≠created 的项目 | 3 个（runx/usenom/pastoid），故 P1-1 在真实数据上可见 | 同上 |

「tam」项目下 13 个活跃任务即用户 backlog，见 §6.2。

### 5.2 基线测试污染事件与恢复

- 机制：`test_edit_config` 以 `Config::default()` 为底、把 current_project 写成 "myproject" 并 `save()`（config.rs:134-141）；默认 HOME 即真实 `~/.tam/tam.config.toml`；`test_load*` 同时读同一文件，并行下互相竞态。
- 已发生事实：基线 `cargo test`（test.exit=0）后，用户 current_project 由 pastoid 变为 myproject；Lead 已恢复为 pastoid。
- 本次复核（15:22）：`~/.tam/tam.config.toml` current_project="pastoid"，mtime 2026-11-08 15:14；`~/.tam/tam.db3` mtime 2026-06-25 09:46（未被触碰）；`~/.tam/logs/tam.log` 最后写入 2026-06-25（测试不初始化日志，符合预期）。
- 本次勘察对真实数据的操作：只读 `sqlite3 mode=ro`、`stat`、`cat`；仅对 /tmp 的库拷贝跑过 `project list`（拷贝前后用户 db3 mtime 不变）；隔离复现：既有测试二进制 + `HOME=/tmp/...` → 4 passed / 1 failed（`test_config_dir` 硬编码断言失败），且 config 被写为 current_project="myproject"。即**隔离后测试仍失败**，证明「硬编码路径断言」与「污染用户配置」是同一处设计缺陷。

### 5.3 对 Gate 2 的硬性要求

1. 数据根目录必须可注入（环境变量或参数），默认仍为 `~/.tam` 以保持兼容。
2. 测试必须断言真实 HOME 不被写入，或完全在临时目录运行；移除 `config.rs:112` 这类绝对路径断言。
3. 现有 5 个测试在隔离环境下必须全绿，否则不进入功能迭代。
4. 演示/回归脚本同样遵守隔离；**禁止裸跑 `cargo test`**（已发生一次污染）。

## 6. 未决问题与 Gate 2 需求输入

### 6.1 待决策问题（需 Lead / 用户拍板）

| # | 问题 | 关联 |
|---|---|---|
| Q1 | 删除项目语义：有任务时拒绝 / 级联软删任务 / 归档只读？是否要 `--force` 与确认交互？ | P0-2 |
| Q2 | 项目名唯一性：仅活跃行唯一（允许重建同名）还是全局唯一？是否需要 restore/undelete？墓碑如何命名？ | P1-2 |
| Q3 | 错误契约：错误走 stderr？区分「不存在/已删除/DB 故障」？退出码约定（1 通用 / 2 用法 / 3 未找到）？ | P1-3 |
| Q4 | current_project 一致性契约：以配置还是以库为准？改名/删除的事务边界？CLI 与 TUI 是否统一？ | P0-1, P1-5 |
| Q5 | 配置兼容：是否引入 config version + 默认值合并 + 备份/自愈？最低兼容版本？ | P1-8 |
| Q6 | 多实例：支持同时开多个 tam（WAL/busy_timeout/配置锁），还是明确单实例并在锁冲突时提示？ | P2-6 |
| Q7 | 范围：§6.2 中 7 项未完成 backlog（i18n/鼠标/过滤/设置页等）哪些进本轮？ | §6.2 |
| Q8 | `task.content`（sql.rs:25）从未写入/展示，是移除还是本轮启用（备注/详情）？ | sql.rs:21-33 |
| Q9 | README/SKILL 是否随代码更新纳入验收（版本号、行为描述）？TUI 错误呈现：弹窗保留状态还是退出交回 CLI？ | P2-3, P1-4 |

### 6.2 用户真实 backlog 对照（project=tam 的 13 个活跃任务，只读 DB）

| DB id | 任务 | 代码现状（证据） | 判定 |
|---|---|---|---|
| 3 | dialog light theme color setting | 弹窗硬编码颜色（ui/components.rs:49,52,…），未接 theme | 未完成 |
| 4 | project add task error while current project not set | CLI/TUI 均有明确报错（commands/task.rs:16；app.rs:339） | 已完成 |
| 5 | light theme cursor color setting | 仅设置光标位置（ui/*_screen.rs popup 末行），无光标配色 | 未完成 |
| 7 | help view key use <> wrap | project_screen.rs:74、task_screen.rs:97 已包 `<>` | 已完成 |
| 8 | projects and tasks store online(maybe settings store url) | 仅本地 SQLite（db.rs:19；config.rs:13） | 未完成（大项） |
| 10 | save theme when theme changed | app.rs:412-413 `set_theme` 持久化 | 已完成 |
| 11 | task list add filter params | task list 无参数（cli.rs:86-88） | 未完成 |
| 13 | bug: use up/down key switch tasks no response | next/prev_task 基于状态过滤列表（app.rs:265-297），1.0.12 已修 | 已完成 |
| 14 | publish to npm and brew | CHANGELOG 1.0.13 + dist-workspace installers | 已完成 |
| 15 | task list command show current project | commands/task.rs:23 打印当前项目 | 已完成 |
| 17 | i18n | 中文文案硬编码（app.rs / ui/*），无 i18n 依赖 | 未完成 |
| 18 | add mouse event | 仅处理 Key 事件（app.rs:74-217），无 Mouse | 未完成 |
| 19 | add settings view or dialog | 无设置屏/弹窗，仅 CLI `config` 打印 | 未完成 |

结论：13 项中 6 项已完成、7 项未完成；其中 4 项（i18n/鼠标/过滤/设置页）属「迭代新增功能」候选，应作为 Gate 2 范围输入，而非 Gate 0 缺陷。

### 6.3 验收口径素材（候选，供 Gate 2 提炼）

| # | 可验证标准 | 对应 |
|---|---|---|
| A1 | `cargo fmt --check` exit 0（8 文件机械格式化后） | P2-1 |
| A2 | 隔离 HOME 下 `cargo test` 全绿；真实 `~/.tam` 前后内容/哈希不变；硬编码 `/Users/wstreet7` 断言移除或参数化 | P0-3 |
| A3 | `project edit` 失败（重名/空名/不存在）时 current_project 不变，有回归测试 | P0-1 |
| A4 | 删除含任务项目默认拒绝并报告任务数、非 0 退出（或按 Q1 的显式 force 路径） | P0-2 |
| A5 | 非法 task id / 不存在项目：无 "Query returned no rows"，stderr + 约定退出码 | P1-3 |
| A6 | TUI 关键失败路径（重名、空输入、删除后导航）不 panic | P1-4 |
| A7 | CLI 与 TUI 删除当前项目后 `project current` 一致（无悬挂指针） | P1-5 |
| A8 | 空项目名/空任务标题被拒绝（退出码 + 可读错误） | P1-7 |
| A9 | README 帮助示例版本与 Cargo.toml 一致（可脚本断言） | P2-3 |

### 6.4 建议的范围切分

- **Gate 2（需求/架构决策，不写业务代码）**：Q1–Q9；测试隔离与错误/退出码契约；数据模型（唯一性、恢复、删除语义、schema_version）；确认 §6.2 中 7 项未完成 backlog 是否入本轮。
- **Gate 3（实现）**：P0-1/P0-2/P0-3 数据安全问题优先；随后 P1-1/P1-3/P1-4/P1-5/P1-7；fmt/clippy/文档作为机械提交并行推进。

## 附录 A 证据索引（相对 docs/forge-takeover/evidence/）

| 文件 | 内容 |
|---|---|
| build.log / build.exit | cargo build；4 warnings；exit 0 |
| test.log / test.exit | cargo test；5 passed；exit 0 |
| fmt.log / fmt.exit | cargo fmt --check；22 hunk / 8 文件；exit 1 |
| clippy.log / clippy.exit | cargo clippy；17 warnings / 0 errors；exit 0 |
| versions.txt / baseline-smoke.log | rustc/cargo 1.94.1；隔离 HOME 手工 smoke（列表时间、edit 污染、删除孤儿、无效 id） |

## 附录 B 本次勘察命令（全部只读/隔离；未改源码与用户数据）

```bash
# 只读用户 SQLite（mode=ro）
sqlite3 "file:$HOME/.tam/tam.db3?mode=ro" "select id,name,created,updated,is_deleted from project;"
stat -f '%Sm %N' "$HOME/.tam/tam.config.toml" "$HOME/.tam/tam.db3"
# 隔离 HOME 复现（真实 ~/.tam 不受影响）
HOME=/tmp/forge-tam/agent-analyst /Users/wstreet7/project/tam/target/debug/tam --help
HOME=/tmp/forge-tam/agent-analyst-test /Users/wstreet7/project/tam/target/debug/deps/tam-1ff1ff064e11f82a --test-threads=1
# 用户库拷贝上复现 P1-1（用户库本身只读）
cp ~/.tam/tam.db3 /tmp/forge-tam/db-copy.db3 && HOME=/tmp/forge-tam/probe-realdata /Users/wstreet7/project/tam/target/debug/tam project list
```

> 未重跑基线四件套（日志已在 `docs/forge-takeover/evidence/`）；本次补充均为只读或 `/tmp` 隔离运行。
