# 06 · 交接（Gate 6）

- 对象：`/Users/wstreet7/project/tam`（tam 1.0.13，单 crate，CLI + ratatui TUI）
- 基线：HEAD `d395841288bbf86960a931924ff628c5b264d066`；本轮改动**全部未提交**，停在当前工作树
- 交接人：release（Gate 6）｜2026-11-08 CST｜状态：**Lead 已验收 Gate 0–5，PASS WITH CAVEATS**
- 目标：让接手者不读源码也能打开、运行、复核、采纳或回退本轮成果；路径/命令可直接复制；数字来自实拍，引用 04/05 处已标注

## 1. 交付物索引

### 1.1 接管文档（本目录，全部 `test -f` 已验证存在）

| 文档 | 行数 | 用途 |
|---|---:|---|
| [00-discovery.md](00-discovery.md) | 250 | Gate 0 现状勘察：模块/命令/基线、P0×3/P1×8/P2×17、真实数据画像 |
| [01-asbuilt.md](01-asbuilt.md) | 296 | Gate 1 现状契约：启动时序、模块依赖、CLI/持久化契约、R1–R20、D1–D11 |
| [02-gap.md](02-gap.md) | 185 | Gate 2 目标差距：Q1–Q9 决策、G1–G12、D 验收总表、E backlog |
| [03-plan.md](03-plan.md) | 221 | Gate 3 计划：S1–S8、逐步回滚、测试矩阵、diff 预算 |
| [04-implementation.md](04-implementation.md) | 170 | Gate 4 实现：G1–G12 改动/文件/验证、S9 B1–B5、门禁、残留风险 |
| [05-regression.md](05-regression.md) | 171 | Gate 5 独立回归：基线 vs 当前、196 条断言、Q-1/Q-2/Q-3、结论 |
| 06-handover.md | 本文 | Gate 6 交接：索引、运行、回退、遗留、路线 |

### 1.2 证据目录 [evidence/](evidence/)（64 个 = 62 既有 + 本文 2 个快照）

| 分组 | 代表文件 | 内容 |
|---|---|---|
| 基线四件套 | [build.log](evidence/build.log)、[test.log](evidence/test.log)、[fmt.log](evidence/fmt.log)、[clippy.log](evidence/clippy.log)、[versions.txt](evidence/versions.txt)、[baseline-smoke.log](evidence/baseline-smoke.log) | 改造前四件套与手工 smoke |
| 用户数据 | [iteration-00-sha256-before.txt](evidence/iteration-00-sha256-before.txt)、[iteration-final-sha256-after.txt](evidence/iteration-final-sha256-after.txt) | 真实 `~/.tam` 两文件 sha256 前后 |
| 分步与门禁 | `evidence/iteration-S1-*.log` … `iteration-S9-*.log`；[build](evidence/iteration-gate-build.log)、[test](evidence/iteration-gate-test.log)、[隔离 test](evidence/iteration-gate-test-temphome.log)、[clippy](evidence/iteration-gate-clippy.log)、[schema](evidence/iteration-gate-schema.log)、[smoke](evidence/iteration-gate-smoke.log) | 每步先红后绿、S9 复核、最终门禁 |
| 独立回归 | [regression-gates-summary.log](evidence/regression-gates-summary.log)、`regression-*.log`（23 个） | QA 独立复跑与 196 条断言 |
| 交接快照 | [final-changes.diff](evidence/final-changes.diff)、[final-git-status.log](evidence/final-git-status.log) | tracked 变更全量 diff（1324 行/48888 字节）与 git 实拍 |

### 1.3 代码 / 测试 / 文档变更文件（行数为当前实际 `wc -l`）

| 文件 | 行数 | 变更用途 |
|---|---:|---|
| [src/app.rs](../../src/app.rs) | 501 | TUI 去 unwrap、`get()` 边界、错误转 tip、selected 复位、config 条件同步、改名 trim |
| [src/cli.rs](../../src/cli.rs) | 146 | task id `i32`+`range(1..)`；`task list --status` 的 `StatusArg` |
| [src/commands/project.rs](../../src/commands/project.rs) / [src/commands/task.rs](../../src/commands/task.rs) | 113/50 | add restored 文案；edit/delete 先 DB 后 config + 恢复指令；trim/空名拒绝；id 与 status 透传 |
| [src/config.rs](../../src/config.rs) | 160 | 5 个旧测试改为 3 个纯 TOML/显式路径测试；抽 `default_for_dir` |
| [src/services/project.rs](../../src/services/project.rs) | 221 | updated 读第 3 列；错误类型匹配；删除计数守卫；同名 restore；规范化 helper |
| [src/services/task.rs](../../src/services/task.rs) | 200 | `normalize_task_title`；query 错误映射；`parse_db_datetime`；状态过滤 |
| [src/sql.rs](../../src/sql.rs) | 107 | **仅新增查询常量**，DDL 一字未改 |
| [src/utils.rs](../../src/utils.rs) | 35 | `print_error!` 改 `eprintln!` |
| [src/{commands.rs,main.rs,services.rs,ui.rs}](../../src/main.rs) | 3/68/2/17 | 仅 fmt（S8 机械格式化，零语义变化） |
| [tests/cli.rs](../../tests/cli.rs) | 731 | **新增（未跟踪）**：23 个集成用例 + TempHome/run_tam |
| [README.md](../../README.md) / [AGENTS.md](../../AGENTS.md) / [SKILL.md](../../skills/tam-usage/SKILL.md) | 124/83/119 | 版本 1.0.13；测试段一行；删除保护、restored、`--status` |

## 2. 变更记录

### 2.1 G1–G12（一句话变更 / 文件 / 验证）

| # | 变更 | 文件 | 验证 |
|---|---|---|---|
| G1 | `project list` 的 Updated 改读第 3 列，与 Created 独立 | services/project.rs | `project_list_updated_column`（sleep 1.1s，连续 5 次绿） |
| G2 | 改名改为「DB 成功后再写 config」；失败不动 current；save 失败给 `tam project use <new>` 指令、不回滚 DB | commands/project.rs、services/project.rs | `rename_failure_keeps_current_project` 等 3 例 + M1 |
| G3 | 有活跃任务的项目拒绝删除（报计数）；删当前项目在 DB 成功后清 config；不存在给 not found | commands/project.rs、services/project.rs、sql.rs、app.rs | `delete_*` 4 例 + smoke C 组 |
| G4 | 同名 add 命中软删行 → restore（输出 restored、刷新 updated、不自动 use） | services/project.rs、commands/project.rs、sql.rs | `add_restores_soft_deleted_project` 等 2 例 |
| G5 | 业务错误走 stderr；task id 改正整数（非数字/≤0 → exit 2）；not-found 精确文案、不泄漏 raw SQL | utils.rs、cli.rs、commands/task.rs、services/{task,project}.rs | `task_not_found_message` 等 4 例 + smoke E |
| G6 | TUI 服务错误转 tip 不退出；`get()` 边界；删除后复位；仅当对象==current 时同步 config | app.rs | 静态 grep（unwrap 13→0）+ QA pty M2–M7/M8 |
| G7 | 空名/空标题（add + S9 扩到 edit）trim 后拒绝，不写 DB | services/{project,task}.rs、commands/project.rs | `reject_blank_*`、`*_edit_blank_*` 4 例 |
| G8 | 时间解析统一 `parse_db_datetime`（NaiveDateTime→UTC），失败返 Err、不再伪造 now | services/task.rs | U4/U5 单测 + grep 零命中 |
| G9 | 新增 `task list --status <todo\|in-progress\|done>`；无参行为逐字节不变；非法值 exit 2 | cli.rs、commands/task.rs、services/task.rs、sql.rs | `task_list_*` 3 例 + 基线 diff 为空 |
| G10 | 5 个旧测试改纯 TOML/显式路径；新增 `tests/cli.rs` 每用例独立 HOME；移除硬编码路径 | config.rs、tests/cli.rs | 裸跑/隔离跑各 30 passed；sha256 不变 |
| G11 | 最后执行一次 `cargo fmt`（仅格式）；S9 后复检仍 0 | 8 个既有文件 | `cargo fmt --check` exit 0 |
| G12 | README 版本、SKILL 三条行为、AGENTS 测试段 | README/AGENTS/SKILL | grep 全部命中 |

### 2.2 S9 复核修复 B1–B5

| # | 问题 | 修复 | 验证 |
|---|---|---|---|
| B1 | `project edit a ""` 成功且写入空名 | service 用 `normalize_project_name`，命令入口 trim + 拒绝空名 | `project_edit_blank_name_rejected` |
| B2 | `task edit 1 "   "` 写入空白标题 | 新增 `normalize_task_title`，add/edit 共用 | `task_edit_blank_title_rejected` |
| B3 | 空白名三连不一致（use/edit/del） | add/query/edit/delete 全链路 trim，config 只存规范名 | `project_names_trimmed_consistently` |
| B4 | TUI 同名改名走 service 报 already exists | service edit 增加同名 no-op | 单测 + QA pty M8 |
| B5 | config 单测硬编码 `/tmp/...`（Windows 失败） | 改用 `std::env::temp_dir()` | 3 个 config 单测隔离 HOME 绿 |

### 2.3 Diff 与测试规模（实拍）

`git diff --stat`：**16 files changed, 527 insertions(+), 245 deletions(-)**；`src/` 合计 **+522 −240**（`git diff --numstat -- src`）；`tests/cli.rs` **731 行**（未跟踪新增）；`Cargo.toml`/`Cargo.lock` 未改。

测试 **30 个（7 单测 + 23 集成）**，`cargo test -- --list` 实点一致；独立回归另做 196 条断言（129 CLI + 27 对抗 + 31 TUI pty + 9 M1，05 §2/§5）。

### 2.4 基线（00）vs 当前门禁

| 门禁 | 00 基线 | 当前 | 证据 |
|---|---|---|---|
| `cargo build` | exit 0，4 warnings | exit 0，4 warnings（逐条同类） | [regression-build.log](evidence/regression-build.log) |
| `cargo test` 裸跑 | 5 passed，**污染真实 HOME** | 30 passed，裸跑安全 | [regression-test-bare.log](evidence/regression-test-bare.log) |
| 隔离 HOME 测试 | 4 passed / 1 failed | 30 passed | [regression-test-temphome.log](evidence/regression-test-temphome.log) |
| `cargo fmt --check` | exit 1，22 hunk / 8 文件 | **exit 0** | [regression-fmt.log](evidence/regression-fmt.log) |
| `cargo clippy` | 0 error / 17 warnings | 0 error / **15 warnings**（基线子集，无新类别） | [regression-clippy-count.log](evidence/regression-clippy-count.log) |
| 真实 `~/.tam` sha256 | 见基线文件 | **逐字节不变** | [regression-final-integrity.log](evidence/regression-final-integrity.log) |
| DB schema | 4 个 CREATE TABLE/TRIGGER | DDL 逐字节未改；`DELETE FROM` 0 命中 | [regression-schema.log](evidence/regression-schema.log) |
| 文档 | README 1.0.10 | 1.0.13；SKILL 三行为；AGENTS 测试段 | [regression-docs-hygiene.log](evidence/regression-docs-hygiene.log) |
| QA 独立断言 | — | **196 PASS / 0 FAIL** | [regression-gates-summary.log](evidence/regression-gates-summary.log) |

## 3. 运行手册（复制即用）

### 3.1 构建与运行

```bash
cd /Users/wstreet7/project/tam
cargo build                      # 或 cargo build --release
cargo run -- project list && cargo run -- task list --status done   # CLI 模式
cargo run -- -i                  # 交互 TUI（q 退出）
```

### 3.2 测试（当前裸跑已安全：每个集成用例自建临时 HOME）

```bash
cargo test                                   # 30 passed（7 单测 + 23 集成）
cargo test -- --test-threads=1               # 单线程复跑
cargo test --test cli task_not_found_message # 单用例
# 隔离 HOME 视角（正确写法：保留 RUSTUP_HOME/CARGO_HOME）
REAL_HOME=$HOME
HOME=$(mktemp -d) RUSTUP_HOME="${RUSTUP_HOME:-$REAL_HOME/.rustup}" CARGO_HOME="${CARGO_HOME:-$REAL_HOME/.cargo}" cargo test
```

> 不要直接 `HOME=$(mktemp -d) cargo test`——rustup 会因找不到默认工具链而失败（03 §11.7 实测）。只隔离 `HOME`、保留工具链变量。

### 3.3 格式与静态检查

```bash
cargo fmt --check; echo "fmt_exit=$?"         # 期望 0
cargo clippy 2>&1 | tail -3                   # 期望 0 error；15 warnings（基线 17）
cargo clippy 2>&1 | grep '^warning: ' | grep -v 'generated [0-9]* warning' | wc -l   # =15
```

### 3.4 数据位置、备份与只读查看

`~/.tam/tam.config.toml`（主题/current_project/db 与日志路径）、`~/.tam/tam.db3`（SQLite）、`~/.tam/logs/tam.log`（10 MB × 5 份）。

```bash
# 备份（先确保没有 tam 进程在写）
cp ~/.tam/tam.db3 ~/.tam/tam.db3.bak.$(date +%Y%m%d); cp ~/.tam/tam.config.toml ~/.tam/tam.config.toml.bak.$(date +%Y%m%d)
# 只读查看（mode=ro，绝不直接改库）
sqlite3 "file:$HOME/.tam/tam.db3?mode=ro" "select id,name,created,updated,is_deleted from project;"
sqlite3 "file:$HOME/.tam/tam.db3?mode=ro" "select id,title,status,project_id,is_deleted from task;"
shasum -a 256 ~/.tam/tam.config.toml ~/.tam/tam.db3
```

恢复：退出所有 tam 实例后把 `.bak` 文件拷回原名即可。

### 3.5 发布链（不要手工改版本/CHANGELOG 标题）

```bash
./release.sh    # cargo release patch --no-verify --no-publish --execute
```

`release.toml` 的 pre-release hook 调 **git-cliff** 重生成 `CHANGELOG.md`；tag 推送后由 **cargo-dist**（`dist-workspace.toml`，版本 `0.28.1-prerelease.2`）CI 跨平台构建并发布 crates.io / npm / Homebrew / GitHub Releases。CHANGELOG 由 conventional commits 生成，标题不要手改。

## 4. 采纳与回退

### 4.1 当前状态

- HEAD = `d395841`（`feat: add tam-usage skill for AI assistants`）；16 个 tracked 文件已修改、**全部未提交**；`tests/`、`docs/` 未跟踪。
- 先 review 再 stage；**不要** `git reset --hard` / `git stash` / 改写历史（本轮从未 commit，HEAD 即干净回退点）。

```bash
git status --short; git diff --stat
git diff -- src/                                                    # 生产代码逐行审阅
git apply --stat docs/forge-takeover/evidence/final-changes.diff    # 审计快照（tracked diff）
less tests/cli.rs                                                   # 新增测试，未跟踪
```

### 4.2 采纳（提交）

```bash
cd /Users/wstreet7/project/tam
git add tests/cli.rs docs/forge-takeover
git add src/ README.md AGENTS.md skills/tam-usage/SKILL.md           # 或 git add -p 逐文件审
git commit -m "feat: harden data consistency, error contract and test isolation (tam 1.0.13)"
```

`docs/forge-takeover/` 可按团队习惯单独提交（`docs: add forge takeover records`）。

### 4.3 回退代码（按文件，回 HEAD）

```bash
git checkout -- \
  AGENTS.md README.md skills/tam-usage/SKILL.md \
  src/app.rs src/cli.rs src/commands.rs src/commands/project.rs src/commands/task.rs \
  src/config.rs src/main.rs src/services.rs src/services/project.rs src/services/task.rs \
  src/sql.rs src/ui.rs src/utils.rs
rm -rf tests/                 # 新增测试整目录回退（未跟踪，git checkout 管不到）
# docs/forge-takeover 是本次交付物，保留；不要删除
```

- 整轮回退 = 上面两条；`Cargo.toml`/`Cargo.lock` 从未改动，无需处理。
- **`/tmp/forge-tam/{patches,snapshots}` 是临时目录**（重启可能清空），只能作短期分步参考，**不可作为长期回退依据**；长期审计用 [final-changes.diff](evidence/final-changes.diff)（tracked 快照，不含未跟踪的 `tests/cli.rs`）。
- 在干净 HEAD 上重放：`git apply docs/forge-takeover/evidence/final-changes.diff` 还原 tracked 变更，但 `tests/cli.rs` 必须另行从工作树拷贝——diff 中**没有**它。

## 5. 已知限制与遗留问题

| 项 | 级别 | 现状 / 影响 |
|---|---|---|
| **Q-1 TUI 窄终端提示截断** | **P2** | 80×24 下提示框只显示 `删除项目失败：project 'alpha' has 2 ac`；根因 `src/ui/components.rs` 的 `centered_rect(50,18)` 固定 50% 宽 + `Paragraph` 不换行，CJK 前缀按 2 列计宽。DB/config 行为正确（计数 `2` 仍可见） |
| Q-2 stale config 提示 | P3 | M1 后 `project current` 报 `project 'alpha' not found`；有恢复指令但未点明「配置中的项目已不存在」 |
| TUI 未进 CI | P2 | M2–M7 由 QA 用 `expect` pty 补证（80×24 与 120×40），但不在 `cargo test`，非 CI 可跑；无人眼视觉验收 |
| 无事务 / `busy_timeout` / WAL | P1 | `COUNT→UPDATE` 删除守卫、查含软删→restore 均非原子；`journal_mode=delete`；多进程并发可能 `SQLITE_BUSY` |
| config 兼容 / 多实例 | P1 | 旧配置缺字段或 TOML 损坏 → 直接 `exit(1)`，无 serde default / version / 备份自愈；配置整文件覆盖写（last-write-wins），无配置锁 |
| 无 CI 质量门禁 | P1 | `.github/` 无 `cargo test/fmt/clippy`（实测 grep 零命中）；仅 release.yml / publish-crates.yml |
| 文档其余漂移 | P2 | D2 许可表述、D5 架构列表、D7 `tam config` 描述、D8 `content` 列、D11 design.txt 仍未修（G12 仅三条行为） |
| 依赖重复 | P2 | `tui-input 0.12.1` 拉入 `ratatui 0.30.0-alpha.5`，与主依赖 `ratatui 0.29.0` 并存（Cargo.lock 实查），编译体积/类型分裂风险 |
| diff 超计划预算 | P2 | `src` +522/−240 = 762 行（计划 ≤420）；`tests/cli.rs` 731 行（计划 ≤600）；无功能面扩张（04 §9.1） |
| 历史脏数据 / 契约变化 | P2 | 旧版本可能已写入空白/带空格名，本轮只保证新写入规范化；非数字 id 由 exit 1 → 2、业务错误由 stdout → stderr（Q3 契约，依赖旧行为的脚本需适配） |

Q-1 复现（临时 HOME，真数据零接触；完整记录见 [regression-TUI-narrow-terminal.log](evidence/regression-TUI-narrow-terminal.log)）：

```bash
H=$(mktemp -d)
HOME=$H ./target/debug/tam project add alpha && HOME=$H ./target/debug/tam project use alpha && HOME=$H ./target/debug/tam task add t1 && HOME=$H ./target/debug/tam task add t2
HOME=$H expect -c 'set stty_init "rows 24 columns 80"; spawn ./target/debug/tam -i; ... 按 Down d Enter ...'
# 观察「提示」弹窗右侧被边框截断；120 列 pty 下同一文案完整
```

## 6. 后续路线（优先级）

> 来源：[02-gap §E](02-gap.md) backlog + [05-regression §8](05-regression.md) 残留风险；每条附验收提示。

**P1（下一轮建议先做）**

1. **事务 / WAL / busy_timeout + config 兼容**：删除守卫与 restore 包事务；`PRAGMA journal_mode=wal`、`busy_timeout`；config 缺字段用 serde default 合并、损坏时备份并重建（含 `version`）。验收：并发删除/新增不产生孤儿；旧版配置（删 `theme` 字段）可启动；损坏 TOML 有备份且自动恢复。
2. **CI 三 job**：`fmt --check`、`test`（隔离 HOME 的写法固化进 workflow）、`clippy`（0 error，warning ≤ 基线）。验收：PR 上三 job 全绿，故意引入格式/告警即红。

**P2（体验与工程质量）**

3. **TUI 窄终端换行（Q-1）**：`render_tips` 加 `Wrap { trim: false }` 或按 Unicode 宽度动态定宽。验收：80×24 pty 下完整显示 `project 'alpha' has 2 active task(s)`。
4. **TUI `TestBackend` 自动化**：覆盖 tip 文案、selected 复位、边界导航。验收：G6 至少部分进入 `cargo test`，无需 expect。
5. **依赖去重 `tui-input` 0.14**：消除 ratatui 0.29/0.30-alpha 双版本。验收：`cargo tree -d` 无重复 ratatui；build/test/fmt/clippy 不回归。
6. **文档漂移收口**：D2/D5/D7/D8/D11 一次性同步。验收：逐条 grep 断言。
7. **迭代功能**：i18n、鼠标、设置页、在线存储（00 §6.2 DB id 17/18/19/8），含 TUI 光标/弹窗主题接线。验收：先出产品决策（语言集、设置字段、存储协议），再按 G 编号拆分。

## 7. 验证摘要与复跑入口

| 检查 | 命令 | 期望 |
|---|---|---|
| 构建 | `cargo build` | exit 0，4 warnings |
| 测试（裸跑 / 单线程 / 隔离 HOME） | `cargo test`；`cargo test -- --test-threads=1`；§3.2 隔离命令 | 均 30 passed |
| 格式 / clippy | `cargo fmt --check; echo $?`；`cargo clippy 2>&1 \| grep -c '^warning: '` | 0；15 |
| schema / 软删 / 硬编码 | `git diff -U0 src/sql.rs \| grep -E 'CREATE (TABLE\|TRIGGER)'`；`grep -rn "DELETE FROM" src/`；`grep -rn "/Users/wstreet7" src tests` | 空；零命中 |
| 用户数据 | `shasum -a 256 ~/.tam/tam.config.toml ~/.tam/tam.db3` | 与[基线](evidence/iteration-00-sha256-before.txt)逐字节相同 |

证据索引：总表看 [regression-gates-summary.log](evidence/regression-gates-summary.log)；断言看 [regression-smoke-G1-G10.log](evidence/regression-smoke-G1-G10.log)（129）、[regression-smoke-extra.log](evidence/regression-smoke-extra.log)（27）、[regression-S5-M-tui.log](evidence/regression-S5-M-tui.log)（31）、[regression-M1.log](evidence/regression-M1.log)（9）；变更全量看 [final-changes.diff](evidence/final-changes.diff)；git 实拍看 [final-git-status.log](evidence/final-git-status.log)；分步红绿见 §1.2。

## 8. 用户数据安全声明

- 真实 `~/.tam` **全程未写入**：所有测试/演示/复核均注入临时 `HOME`；本轮只做只读 `sqlite3 mode=ro`、`stat`、`sha256`。
- 当前 `~/.tam/tam.config.toml` 的 `current_project = "pastoid"`（基线值，未被测试改写）；`tam.db3` 为 2026-06-25 的既有用户数据。
- sha256 与基线逐字节一致：config `ad333f65…06a23`、db3 `85d4d8ef…2a223`（QA 16:38 复核仍一致）；集成测试每用例独立 `TempHome`（23/23），`run_tam` 以 `env_clear` 注入 HOME/USERPROFILE/PATH/TZ，无 `set_var`、无 `/Users/wstreet7` 硬编码（05 §6.1）。

## 9. 结论

- **Lead 已验收 Gate 0–5；状态 PASS WITH CAVEATS。** 通过依据：G1–G12 与 S9 B1–B5 全部通过；build/fmt/clippy/测试/schema/隔离/文档/用户数据七类门禁达标；QA 独立 196 条断言全 PASS（05 §10）。
- 剩余风险与未验证项（不夸大）：① Q-1 为 P2 显示层缺陷（仅窄终端提示截断，行为正确）；② TUI 仅 pty 回放、无人眼验收，未测 Windows/其他终端；③ 无事务/WAL/config 兼容/多实例防护（P1 backlog）；④ 仓库仍无 CI 质量门禁；⑤ 发布链（`release.sh`、cargo-dist、npm/Homebrew）未验证；⑥ 文档 D2/D5/D7/D8/D11 漂移未清。
- 交接建议：按 §4.2 采纳并提交；下一轮优先 P1 两项（事务/WAL/config 兼容、CI 三 job），再按 §6 推进 P2。
