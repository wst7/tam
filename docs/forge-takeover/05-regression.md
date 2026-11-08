# 05 · 回归验证（Gate 5）

> 验证者：qa（独立复核，不采信 04/builder 的日志结论）｜2026-11-08 16:26–16:41 CST｜仓库 `/Users/wstreet7/project/tam`
> 基线：HEAD `d395841288bbf86960a931924ff628c5b264d066`（工作树含 Gate 3/4 未提交改动，16 个 tracked 文件修改 + tests/、docs/ 未跟踪）
> 范围：只读源码 + 独立复跑门禁 + 自写 CLI/TUI/M1 测试；**只新增本文件与 `evidence/regression-*.log`**，未改源码/测试/00–04，未 git commit/reset/stash。
> 唯一交付：本文件 + [evidence/](evidence/) 下 23 个 `regression-*` 证据文件。

## 1. 环境与版本

| 项 | 值 | 证据 |
|---|---|---|
| HEAD | `d395841`（= 00 基线 commit） | `git rev-parse HEAD` |
| 工作树 | 16 files changed, +527/−245；`src/` +522/−240；未跟踪 `tests/`(731 行)、`docs/` | `git status --porcelain`、`git diff --shortstat` |
| 工具链 | rustc 1.94.1 (e408947bf 2026-03-25)；cargo 1.94.1 (29ea6fb6a) | 本次实测 |
| 产品版本 | tam 1.0.13（`target/debug/tam -V`，隔离 HOME） | [regression-docs-hygiene.log](evidence/regression-docs-hygiene.log) |
| 平台 | macOS（workspace `/Users/wstreet7/project/tam`），真实 `~/.tam` current_project=pastoid | 00 §5.1 |
| 用户数据基线 | `ad333f65…06a23`（config）、`85d4d8ef…2a223`（db3） | `evidence/iteration-00-sha256-before.txt` |

## 2. 方法

1. **独立阅读**：00/01/02/03/04 全文、`tests/cli.rs` 731 行、`git diff src/` 1267 行，先建立“应然”再验证“实然”。
2. **每个 tam 调用都注入临时 HOME**：`HOME=$(mktemp -d)`（另有 `env_clear`+HOME/USERPROFILE/PATH/TZ 的集成测试进程）。真实 `~/.tam` 仅在每组前后做 `shasum -a 256` 对比，从未被二进制使用。
3. **裸 `cargo test`**：先记录真实 sha256/mtime，跑完再对比（证明“裸跑已安全”），另跑 `RUSTUP_HOME`/`CARGO_HOME` 指向真实工具链、仅 `HOME` 隔离的视角。
4. **自写断言脚本**（不引用 builder 的 smoke 结论，仅用其保存的“改造前基线输出”做 G9 字节对比）：
   - 核心 CLI smoke：A–H 场景 **129 条断言**（[regression-smoke-G1-G10.log](evidence/regression-smoke-G1-G10.log)）；
   - 额外对抗场景 **27 条**（[regression-smoke-extra.log](evidence/regression-smoke-extra.log)）；
   - M1 配置保存失败 **9 条**（[regression-M1.log](evidence/regression-M1.log)）；
   - TUI M2–M7 + M8 用 `expect` 驱动真实 pty **31 条**断言，并用自写最小 VT 模拟器重建帧文本（避免 ANSI 日志拼接误判）（[regression-S5-M-tui.log](evidence/regression-S5-M-tui.log)）。
5. 所有命令 `2>&1 | tee` 实拍，退出码写入日志；未事后补日志、未伪造截图。

## 3. 基线（00）vs 当前总表

| 门禁 | 00 基线 | 当前（S9 后） | 判定 | 证据 |
|---|---|---|---|---|
| `cargo build` | exit 0，4 warnings | exit 0，**4 warnings，类别逐条相同** | PASS | [regression-build.log](evidence/regression-build.log) |
| `cargo test` 裸跑 | 5 passed，**污染真实 HOME** | **7 单测 + 23 集成 = 30 passed / 0 failed / 0 ignored**；前后 sha256 不变 | PASS | [regression-test-bare.log](evidence/regression-test-bare.log) |
| `cargo test` 隔离 HOME | 4 passed/1 failed（硬编码 HOME） | 30 passed；真 HOME 哈希不变 | PASS | [regression-test-temphome.log](evidence/regression-test-temphome.log) |
| `--test-threads=1` | — | 裸跑 & 隔离各 30 passed | PASS | [regression-test-single-thread.log](evidence/regression-test-single-thread.log) |
| `cargo fmt --check` | exit 1，22 hunk / 8 文件 | **exit 0** | PASS | [regression-fmt.log](evidence/regression-fmt.log) |
| `cargo clippy`（裸） | 0 error / 17 warnings | **0 error / 15 warnings**，类别为基线子集（少 1 个 useless_conversion、1 个无用 cast），无新类别 | PASS | [regression-clippy.log](evidence/regression-clippy.log)、[regression-clippy-count.log](evidence/regression-clippy-count.log) |
| `clippy --all-targets` | 信息性 | test profile 16 warnings，仅信息 | INFO | [regression-clippy-all-targets.log](evidence/regression-clippy-all-targets.log) |
| 真实用户数据 | 基线哈希 | **逐字节一致，USER_DATA_UNTOUCHED**；mtime 未变（config 15:14:42、db/logs 06-25） | PASS | [regression-final-integrity.log](evidence/regression-final-integrity.log) |
| schema | 4 个 CREATE TABLE/TRIGGER 常量 | `git diff -U0 src/sql.rs \| grep CREATE` 为空；DDL 常量逐字节相同（4=4）；`DELETE FROM` 0 命中；Cargo.toml/lock 未改 | PASS | [regression-schema.log](evidence/regression-schema.log) |
| 文档/卫生 | README 1.0.10、fmt 失败 | README 1.0.13（1.0.10 零命中）；SKILL 三条行为；AGENTS 测试段；`/Users/wstreet7` 0；`set_var` 0；`Query returned no rows` 0；`unwrap_or_else(\|_\| Utc::now())` 0 | PASS | [regression-docs-hygiene.log](evidence/regression-docs-hygiene.log) |
| 04 声明复核 | — | 全部数字吻合（见 §6.2）；无夸大 | PASS | [regression-04claims-crosscheck.log](evidence/regression-04claims-crosscheck.log) |

## 4. 验收矩阵：G1–G12 + S9 B1–B5

| 项 | 判定 | 独立证据（本次实跑） | 命令摘要 |
|---|---|---|---|
| G1 Updated 列独立 | PASS | smoke A：add→sleep1.1s→edit 后 `Created=08:30:07`、`Updated=08:30:08` | `project add alpha; sleep 1.1; project edit alpha alpha-renamed; project list` |
| G2 改名失败不动 current/DB/list；成功同步 | PASS | smoke B1–B12：失败 exit1/stderr already exists/current=alpha/list 字节相同；成功后 current=gamma 且旧名消失 | `edit alpha beta`、`edit alpha gamma` + `project current/list` |
| G3 删除守卫/当前项目清理/不存在 | PASS | smoke C1–C26：2 任务拒绝（计 2）、1 个拒绝（计 1）、全软删后允许；删当前→current exit1；不存在→not found；非当前也 SUCCESS；DB `is_deleted=1` 且任务行保留 | `project del` × 多场景 + `sqlite3` 行数 |
| G4 同名恢复 / 活跃重名 | PASS | smoke D：restored、Updated 刷新、current 不变、活跃重名 exit1 already exists | `add→del→sleep→add` |
| G5 错误契约/流向/退出码 | PASS | smoke E：`done 999`→1/stderr `task 999 not found`；`abc/0/-1`→2；`use nope`→1；错误 stdout 空、成功 stderr 空；stderr 无 raw SQL | `cmd 1>o 2>e` + 断言 |
| G6 TUI 不 panic/边界/配置同步 | PASS | 静态：`unwrap()` 0、无 `self.projects[`/`self.tasks[`、config 写入只在 `current_project.as_deref()==Some(...)` 下、错误全转 tip；动态：M2–M7 + M8（见 §5） | `grep -n` + expect pty |
| G7 空名拒绝 | PASS | smoke F：`project add ""/"   "`、`task add "   "` 均 exit1、stderr cannot be empty、DB 行数 0 | 临时 HOME + `sqlite3 count(*)` |
| G8 时间戳不伪造 | PASS | 单测 U4/U5 绿；`grep 'unwrap_or_else(\|_\| Utc::now())'` 0；`parse_db_datetime` 非法输入 Err | `cargo test` + grep |
| G9 `task list --status` | PASS | smoke G：三状态各 1；done/in-progress/todo 过滤正确；**无参输出与改造前基线字节相同（cmp）**；`wip`/缺值 exit2 | 与 `evidence/iteration-S6-baseline.log` 场景同构对比 |
| G10 测试隔离/真实数据 | PASS | 裸跑 & 隔离跑 30 passed，前后 sha256 一致；`binary_writes_only_under_injected_home` 绿；临时 HOME 内生成 `.tam/{config,db3,logs}` | §3 两行 + smoke A8–A11/H1–H2 |
| G11 fmt 归零 | PASS | `cargo fmt --check` exit 0；build/test 不回归 | §3 |
| G12 文档对齐 | PASS | README 1.0.13；SKILL 含 restored/active task(s)/--status；AGENTS 含 tests/cli.rs + 临时 HOME | grep 断言 |
| S9 B1 `project edit alpha ""` | PASS | smoke B13–B17 + DB 级：exit1、stderr cannot be empty、DB 仍 `1:alpha`、config current=alpha、list 不变 | 临时 HOME + `sqlite3` |
| S9 B2 `task edit 1 "   "` | PASS | smoke B19–B21 + DB：exit1、DB title 仍 `original title` | 同上 |
| S9 B3 trim 一致性 | PASS | smoke B22–B31：add/use/edit/del 四入口 trim，config 存 `current_project = "beta"` 无空格，删除后 current exit1 | 临时 HOME + 读 config |
| S9 B4 同名 no-op（含 TUI 路径） | PASS | CLI `edit beta beta` exit0 INFO no changes；TUI 同名校验 M8：exit0、无失败 tip、alpha 仍活跃；service `if name == new_name { return Ok(true) }` | [regression-S5-M-tui.log](evidence/regression-S5-M-tui.log)、[regression-smoke-extra.log](evidence/regression-smoke-extra.log) |
| S9 B5 config 单测可移植 | PASS | 3 个 config 单测隔离 HOME 全绿；路径由 `std::env::temp_dir()` 构造，无绝对 `/tmp/...` 断言 | `cargo test` |

## 5. CLI smoke 与对抗场景结果

- **核心 smoke（自写，129/129 PASS，0 FAIL）**：A G1+G10；B/B2/B3 G2+S9 空白与 trim；C/C2 G3 删除守卫与 current；D G4 恢复；E G5 错误/流/退出码；F G7 空名；G G9 过滤与无参基线字节对比；H 真实数据哈希。见 [regression-smoke-G1-G10.log](evidence/regression-smoke-G1-G10.log)。
- **额外对抗（27/27 PASS）**：改名到软删名（提示 `tam project add alpha` 恢复且 current 不变）、非当前项目改名不动 config、无当前项目时 `task add/list` 明确报错、`use ""`/`del ""` 拒绝、同名 edit no-op、软删任务不可再操作、空过滤列表文案、`-h`/`-V`/未知命令退出码。见 [regression-smoke-extra.log](evidence/regression-smoke-extra.log)。
- **TUI（31/31 PASS，真实 pty 驱动）**：
  - M2 重名改名 → tip `重命名项目失败：project 'beta' already exists`，不退出、DB/config 不变；
  - M3 删除有 2 任务项目 → tip `删除项目失败：project 'alpha' has 2 active task(s)`，项目与任务保留；
  - M4 删除当前项目 → 退出后 DB 软删、config 清空、`project current` exit1；
  - M5 删最后一个项目后 ↓/e/d → tip `请先选择项目`，无 panic；
  - M6 删除非当前项目 → 仅 beta 软删，config current 仍 alpha；
  - M7 Done 任务再 `c` → tip `任务完成失败：Task 'task-done' is already done.`，不退出；
  - M8（B4）TUI 同名改名 → no-op、无失败 tip。
  证据（含 VT 重建的提示原文）：[regression-S5-M-tui.log](evidence/regression-S5-M-tui.log)。
- **M1 配置保存失败（独立重放，9/9 PASS）**：临时 HOME 中 `project add alpha → use alpha → chmod 400 .tam/tam.config.toml`（目录/logs 可写）→ `edit alpha beta`：exit1、stdout 空、stderr 含 `run \`tam project use beta\`` 恢复指令、DB 已改名 beta、config 仍 `current_project = "alpha"`、`project current` exit1；`chmod 600` 后 `use beta` exit0 且 config 修复。见 [regression-M1.log](evidence/regression-M1.log)。

## 6. 测试质量与 04 声明复核

### 6.1 测试质量（读 `tests/cli.rs` 731 行 + 实跑）

- **正面的**：23 个集成用例**每个都自建 `TempHome`**（逐一核对 23/23）；`run_tam` 用 `env_clear` 后注入 HOME/USERPROFILE/PATH/TZ；无 `set_var`、无 `#[ignore]`、无 `should_panic`、无 `/Users/wstreet7`、无共享全局状态；断言不只判 exit 0（例如 G2 比较整张 list 字节、G3 查任务仍在、G4 断言 restored+updated 刷新+current 不变、G5 断言 stdout 空/stderr 精确文案、G10 断言临时 HOME 内文件存在且 config 指向该 HOME）；7 个单测为纯函数/TOML 往返，不触 Lazy/真实 HOME。
- **覆盖缺口**（都不影响本轮验收，但应记入后续）：
  1. G6 TUI 无 `cargo test` 自动化（本次用 expect pty 补证，仍非 CI 可跑）；
  2. M1 config 保存失败、以及“删除当前项目时 config 保存失败”分支无自动化；
  3. 多实例/并发（Q6）、config 损坏/缺字段兼容（Q5）、DB 损坏路径未覆盖（本轮明确延后）；
  4. 任务状态机仅覆盖 start/done 正常路径，未覆盖 Done→start 回退等既有 P2-11 语义；
  5. 真实 HOME“未被写”是构造性保证（注入 HOME）+ 外部哈希，测试内部无独立断言。

### 6.2 04 声明逐项复核

| 04 声明 | 复核结果 |
|---|---|
| 测试 30（7 单测 + 23 集成） | ✅ 实测 30，`--list` 名称与 §4 清单**逐条一致** |
| clippy 0 error / 15 warnings、未新增类别 | ✅ 实测 15；类别为基线子集（无新类别） |
| diff stat 16 files +527/−245；src +522/−240；tests/cli.rs 731 行 | ✅ 逐项吻合 |
| 真实 sha256 与 iteration-00 一致 | ✅ 逐字节；本次 16:38 复核仍一致 |
| CLI smoke 47 + S9 10 | ✅ builder 日志 `PASS=47 FAIL=0` / `PASS=10 FAIL=0`；本次另做 129+27 条独立断言 |
| docs 00–03 未修改（250/296/185/221 行） | ✅ 行数吻合；mtime（15:29–15:50）早于 04（16:23）与本次证据（16:26 后）；docs/ 未跟踪、无 git 基线，故为间接证据 |
| M1 实拍 | ✅ 独立重放结果与 04 §6 描述一致（含恢复指令文案） |
| M2–M7 “未执行（仅代码审查）” | ⚠ 保守而非夸大：本次已在 pty 中真实执行 M2–M7+M8 并全过；04 可据此升级（未要求改 04） |
| app.rs `unwrap()` 13→0 | ✅ `git show HEAD:src/app.rs` 13 处 → 当前 0 |
| Cargo.toml/lock 未改、无 dev-deps | ✅ `git status` 零改动 |

结论：**未发现 04 的夸大或数字不符**；唯一“差异”是 04 对 TUI 手工项的保守标注（本次补做成功）。

## 7. 手工/TUI 未验证项（明确列表，不夸大）

- TUI 仅在 **pty + expect 按键回放**下验证（80×24 与 120×40 两种尺寸），**未做人眼视觉验收**；未测 theme=light、中文输入法、鼠标、真实 `q` 之外的退出路径。
- 未验证 Windows/macOS 真实终端差异（本次仅 macOS pty）；TUI M2–M8 未纳入 `cargo test`。
- 未验证多实例/并发（WAL、busy_timeout、配置 last-write-wins）、config 损坏/旧版缺字段启动路径、DB 损坏/权限异常路径（Q5/Q6 明确延后）。
- 未验证发布链（`release.sh`、git-cliff、cargo-dist、npm/Homebrew 安装器）；仓库仍无 CI 的 fmt/test/clippy 门禁（02 §E 延后）。
- README/SKILL 既有文档漂移 D2/D5/D7/D8/D11 仍未修（G12 仅覆盖三条行为），属 backlog。

## 8. 残留风险与后续建议

| 风险 | 现状 | 建议 |
|---|---|---|
| 删除守卫非原子（COUNT→UPDATE 无事务） | 04 §10 已列；单实例概率极低 | Q6：WAL/事务/busy_timeout 一并落地 |
| config 保存失败后 DB/config 分歧 | 按 Q4 有意不回滚，M1 已实拍；恢复指令有效 | 可加 `tam config doctor`/自动检测提示（backlog） |
| TUI 长错误提示在窄终端被裁剪 | 本次新发现（P2，见 §9） | `render_tips` 按 Unicode 显示宽度定宽或启用换行/滚动 |
| TUI 无自动化 | 本轮用 expect 补证，成本高且非 CI | 评估 `ratatui` TestBackend 单测（G6 可部分自动化） |
| 无 CI 质量门禁 | 02 §E 延后 | 至少加 fmt/test（隔离 HOME）/clippy 三个 job |
| 文档剩余漂移 | README 许可/徽章、SKILL 依赖描述等 | 单独文档同步提交 |

## 9. 缺陷清单

**无阻断缺陷（无 P0/P1）。** 仅 1 项非阻断显示问题 + 2 项 backlog 观察：

| ID | 严重度 | 现象 / 复现 | 期望 / 实际 / 影响 |
|---|---|---|---|
| Q-1 | **P2**（显示层） | 80×24 终端：项目有 2 个任务时 TUI 按 `d`→`Enter`，提示框只显示 `删除项目失败：project 'alpha' has 2 ac`，尾部被裁剪 | 期望完整一行（120 列下完整）：实际被 popup 边框截断；根因 `src/ui/components.rs:36` `centered_rect(50,18)` + 不换行 `Paragraph`，CJK 前缀按 2 列计宽；影响：窄终端下错误细节/计数文案被截断（计数数字仍可见），DB/config 行为正确。建议加 `Wrap { trim: false }` 或按 `unicode-width` 计算弹窗宽度。证据 [regression-TUI-narrow-terminal.log](evidence/regression-TUI-narrow-terminal.log) |
| Q-2 | P3（UX 观察） | M1 后 `project current` 输出 `project 'alpha' not found` | 该错误来自 stale config；已按 Q4 提供恢复指令，但 `current` 的提示可更明确（“配置中的当前项目已不存在，请 tam project use …”）。非本轮验收项。 |
| Q-3 | P3（文档） | SKILL 仍写“tam task 命令依赖当前项目”（D6），README 许可表述（D2）等 | 属 backlog；不影响 G12 的三条行为验收。 |

未发现数据损坏、未发现真实 `~/.tam` 被读写（所有运行均注入临时 HOME，最终哈希与基线逐字节一致）。

## 10. 结论

**PASS WITH CAVEATS。**

- **PASS 的理由**：G1–G12 与 S9 B1–B5 全部通过；build/fmt/clippy/测试/文档/schema/隔离/用户数据所有门禁达标；本次独立断言 129（CLI）+ 27（对抗）+ 31（TUI）+ 9（M1）= **196 条全部 PASS，0 FAIL**；04 声明数字全部复核一致、无夸大。
- **Caveats**：① 发现 1 个 P2 显示层缺陷（窄终端 TUI 提示裁剪，非本轮回归且不阻断）；② TUI 仅为 pty 回放、非人眼验收；③ 04 §10 的既有残留风险（无事务/多实例/config 兼容/无 CI）仍成立；④ 本轮未做发布链与跨平台验证。
- 未修改任何源码/测试/00–04，未 git commit；真实用户数据逐字节未变（`USER_DATA_UNTOUCHED`）。

## 11. 证据索引（`docs/forge-takeover/evidence/`）

| 文件 | 内容 |
|---|---|
| regression-sha-before.log、regression-final-integrity.log | 用户数据前后哈希、mtime、仓库状态、无残留进程 |
| regression-build.log | `cargo build`（exit 0，4 warnings，逐条列出） |
| regression-test-bare.log / regression-test-temphome.log / regression-test-single-thread.log | 30 tests 三种跑法 + 哈希 |
| regression-fmt.log / regression-clippy.log / regression-clippy-count.log / regression-clippy-all-targets.log | fmt 0；clippy 0 error/15 warnings/无新类别；`--all-targets` 仅信息 |
| regression-gates-summary.log | 基线 vs 当前数字总表 |
| regression-schema.log | DDL 字节对比、`DELETE FROM` 0、sqlite_master 实查、Cargo 未改 |
| regression-docs-hygiene.log | README/SKILL/AGENTS、硬编码路径、set_var、raw SQL、00–03 行数 |
| regression-smoke-G1-G10.log | 自写核心 CLI smoke（129 断言） |
| regression-smoke-extra.log | 额外对抗场景（27 断言） |
| regression-smoke-S9-db.log | S9 空白 edit 的 DB 级不变断言 |
| regression-M1.log | 独立复现 config 保存失败（9 断言） |
| regression-static-tui.log | TUI 静态检查（unwrap/索引/config 同步/错误转 tip） |
| regression-S5-M-tui.log | expect pty 驱动 M2–M8（31 断言）+ VT 重建提示原文 |
| regression-TUI-narrow-terminal.log | 80×24 终端提示裁剪复现（Q-1） |
| regression-test-quality.log | 用例清单、每例独立 HOME、无 ignored/set_var/硬编码 |
| regression-04claims.log / regression-04claims-crosscheck.log | 04 数字/名称/类别逐项复核 |
