# 01 · 现状契约（As-Built）

- 项目：`tam` 1.0.13（`Cargo.toml:2`），单 crate、单二进制，21 个 `.rs` 文件共 2047 行；CLI + ratatui TUI 双模式。
- 基线：commit `d395841`；证据见 `docs/forge-takeover/evidence/`（`build.exit=0`、`test.exit=0`、`fmt.exit=1`、`clippy.exit=0`、`baseline-smoke.log`）。
- 方法：只读源码与只读命令；未运行 `cargo test`、未重跑 build、未触碰 `~/.tam` 用户数据。所有契约以源码事实为准，行号即证据。
- 边界：本文只描述现状与风险，不含改造方案（Gate 3 输出）。

## 1. 系统上下文与运行时

### 1.1 进程启动顺序（CLI 与 TUI 共用前缀）
| # | 动作 | 位置 | 失败行为 |
|---|------|------|----------|
| 1 | `main()` | `src/main.rs:21` | — |
| 2 | `logs::init_logging()`（最先调用） | `src/main.rs:22` | — |
| 3 | 间接触发 `config::get_config()` → `Lazy<Config>` 首次求值 | `src/logs.rs:16` → `src/config.rs:15-24` | 加载失败 `eprintln` + `exit(1)`（`config.rs:18-21`） |
| 4 | `Config::load()`：确保 `~/.tam` 存在；配置缺失则写默认值 | `src/config.rs:63-83` | `config_dir` 失败：默认值路径 `exit(1)`（`config.rs:43-46`），加载路径由 `config.rs:18-21` 兜底 |
| 5 | log4rs 初始化：10 MB 触发、固定窗口 5 份 | `src/logs.rs:18-45` | `expect` → panic，退出码 101（`logs.rs:23,33,43,45`） |
| 6 | `init_db()`：打开 SQLite 并执行 4 条 `CREATE ... IF NOT EXISTS` | `src/main.rs:23-29`、`src/db.rs:23-30` | `eprintln` + `exit(1)`（`main.rs:25-28`） |
| 7 | `Cli::parse()`（clap） | `src/main.rs:30` | `--help/-V` → 退出 0；参数错误 → clap 退出 2 |
| 8 | 分派：`--interactive` 优先，其次子命令，否则打印 help | `src/main.rs:31-38` | 无 command 且无 `-i`：`Cli::parse_from(["tam","--help"])` → 退出 0 |
| 9a | 命令模式 → `execute_command` | `src/main.rs:51-64` | 任意 `Err` → `print_error!` + `exit(1)`（`main.rs:52-55`） |
| 9b | 交互模式 → `App::new().run()` | `src/main.rs:41-49`、`src/app.rs:40-70` | `run` 返回 Err → `eprintln` + `exit(1)`（`main.rs:44-48`） |

注意：`logs` 模块在 `main.rs` 中最后声明（`main.rs:19`）却最先调用；`logs::init_logging` 内部先读 config（`logs.rs:16`），因此 **config 加载实际发生在日志初始化之前**，`~/.tam` 目录由 config 层创建，而非日志层。

### 1.2 数据 / 配置 / 日志三类文件
| 类别 | 默认路径 | 路径来源 | 创建时机 | 失败/覆盖行为 |
|------|----------|----------|----------|----------------|
| 配置 | `~/.tam/tam.config.toml` | `config.rs:8-9,65` | 首次运行或缺失时由默认值写入（`config.rs:66-70`） | TOML 非法或缺必填字段 → `exit(1)`（`config.rs:18-21,72-73`） |
| 数据库 | `~/.tam/tam.db3` | `config.rs:13,48`；可被配置覆盖 | `Connection::open` 自动创建（`db.rs:19`） | 打开/DDL 失败 → `exit(1)`（`main.rs:23-29`） |
| 日志 | `~/.tam/logs/tam.log` | `config.rs:11,49` | log4rs 首次写入（`logs.rs:28-33`） | 路径不可建 → panic（`logs.rs:23,33`） |
| 日志轮转 | `~/.tam/logs/tam.{}.log` | `config.rs:12,50` | 达到 10 MB 时滚动，保留 5 份（`logs.rs:18-23`） | 同上 |
| 目录 | `~/.tam/` | `config.rs:8,76-83` | config 层 `create_dir_all` | 无 HOME/USERPROFILE → `exit(1)`（`config.rs:77,43-46`） |

`db_file`、`logfile_path`、`logfile_pattern` 均可通过配置文件重定位，因此“所有状态都在 `~/.tam`”只是默认值，不是强制契约。`theme` 取值 `dark|light`，非法值回退 `dark`（`theme.rs:56-63`；`config.rs:57`）。

## 2. 模块表与依赖方向
| 模块/文件 | 行数 | 职责 | 公开接口 | 出边依赖 |
|-----------|------|------|----------|----------|
| `main.rs` | 65 | 启动编排、模式分派、错误出口 | `main`, `interactive_mode`, `command_mode`, `execute_command` | cli, commands, app, db, logs, config(隐式) |
| `cli.rs` | 111 | clap 命令树与默认子命令分派 | `Cli`, `Commands`, `ProjectSubcommand`, `TaskSubcommand`, `execute_command` | commands |
| `commands.rs` | 3 | 子模块聚合 | — | commands/{project,task,config} |
| `commands/project.rs` | 96 | project 命令实现、`current_project` 读写、用户提示 | `add/list/use_project/delete/get_current_project/edit` | services::project, config, model, 宏 |
| `commands/task.rs` | 48 | task 命令实现与提示 | `add/list/delete/start/done/edit` | services::{task,project}, model, 宏 |
| `commands/config.rs` | 8 | 打印 Config 调试格式 | `config` | config |
| `config.rs` | 151 | 常量、TOML 配置、`Lazy` 单例、保存 | `TAM_DIR`, `CONFIG_FILE`, `LOG_FILE_PATH`, `LOG_FILE_PATTERN`, `DB_FILE`, `Config{db_file,current_project,logfile_path,logfile_pattern,theme}`, `get_config`, `save`, `set_theme` | env_home, toml, once_cell |
| `db.rs` | 35 | 连接创建、目录创建、建表 | `get_db`, `init_db` | config, sql, rusqlite |
| `sql.rs` | 88 | 全部 SQL 常量 | 15 个 SQL 常量（`sql.rs:1-89`） | 无 |
| `services.rs` | 1 | 子模块聚合 | — | services/{project,task} |
| `services/project.rs` | 123 | 项目 CRUD、当前项目解析 | `add/list/query_project/delete/get_current_project/edit` | db, sql, model, config(`:101`) |
| `services/task.rs` | 149 | 任务 CRUD、状态流转 | `add/list/start/done/edit/delete`（`query_task` 私有） | db, sql, model, services::project(`:6,20`) |
| `model.rs` | 157 | `Project`/`Task`/`TaskStatus` 及表格打印 | `new/print/set_title`, `TaskStatus::{is_*,next,prev}`, `ToSql/FromSql/ToString` | chrono, rusqlite, tabled, serde |
| `theme.rs` | 64 | 明暗主题 | `Theme::dark/light`, `From<&str>` | ratatui |
| `logs.rs` | 46 | 滚动日志初始化 | `init_logging` | log4rs, config |
| `app.rs` | 450 | TUI 状态机、键处理、直接调 services | `App::new/run`, `query_projects/query_tasks/get_project/get_task/get_tasks_with_status` | services, config, theme, ui, model |
| `ui.rs` | 17 | 帧分派 | `draw` | app, ui/screens |
| `ui/project_screen.rs` | 137 | 项目列表/帮助/弹窗渲染 | `draw` | app, components |
| `ui/task_screen.rs` | 165 | 三列看板/帮助/弹窗渲染 | `draw` | app, components, model |
| `ui/components.rs` | 98 | 居中弹窗、提示、确认框 | `centered_rect/render_tips/render_confirm` | app |
| `utils.rs` | 36 | `print_success!/print_error!/print_info!` 宏 | 3 个 `#[macro_export]` 宏 | colored |

**依赖方向结论**：整体为 `main → cli → commands → services → db/sql` 单向；`app/ui` 与 `commands` 是 services 的两条并列调用方；不存在 services→commands/app、db→services 的反向依赖，**“禁止反向依赖”基本成立**。三个已存在但方向特殊的边：
1. `services/project.rs:100-107` 直接读 `config`，打破“services 只碰 DB”的分层（与任务书已知事实一致）；
2. `services/task.rs:6,20` 服务间调用（task→project）用于取当前项目，`get_current_project` 的 config 语义因此下沉到 services；
3. `commands/project.rs:41` 使用 `colored::Colorize`，该 trait 仅靠 `print_info!` 宏展开块内的 `use`（`utils.rs:29`）进入作用域，属隐式依赖。

## 3. CLI 契约

### 3.1 命令树
| 层级 | 命令/子命令 | 别名 | 参数 | 默认行为 |
|------|-------------|------|------|----------|
| 顶层 | `project` | `p`（`cli.rs:28`） | 可选子命令 | 无子命令 → `List`（`cli.rs:67`） |
| 顶层 | `task` | `t`（`cli.rs:34`） | 可选子命令 | 无子命令 → `List`（`cli.rs:102`） |
| 顶层 | `config` | — | 无 | 打印配置调试文本（`cli.rs:40`） |
| 顶层 | `help` | — | — | clap 自动生成 |
| project | `add <name>` | `a`（`cli.rs:46`） | 1 位置参数 | — |
| project | `list` | `l`（`cli.rs:49`） | — | — |
| project | `use <name>` | `u`（`cli.rs:52`） | 1 | 设置 `current_project` |
| project | `del <name>` | `d`（`cli.rs:55`） | 1 | 软删除 |
| project | `current` | `c`（`cli.rs:58`） | — | 打印当前项目 |
| project | `edit <name> <new_name>` | `e`（`cli.rs:61`） | 2 | 改名 |
| task | `add <title>` | `a`（`cli.rs:81`） | 1 | 挂到当前项目 |
| task | `edit <id> <title>` | `e`（`cli.rs:84`） | 2 | 改标题 |
| task | `list` | `l`（`cli.rs:87`） | — | 当前项目任务 |
| task | `del <id>` | `d`（`cli.rs:90`） | 1 | 软删除 |
| task | `start <id>` | **无**（`cli.rs:93`） | 1 | status=1 |
| task | `done <id>` | **无**（`cli.rs:96`） | 1 | status=2 |

全局 flag 仅 `-i, --interactive`（`cli.rs:20-22`）。`--version/-V` 输出 `tam 1.0.13`（`cli.rs:9`）；`about` 为 “A tasks manager cli tool\nVersion: 1.0.13”，版本号经 `colored` 着色（`cli.rs:10-14`），是否输出 ANSI 取决于 `colored` 的 TTY 检测。**共 13 个叶子命令、2 个组别名、10 个子命令别名**。

### 3.2 默认行为矩阵与退出码
| 输入 | 行为 | 退出码 | 位置 |
|------|------|--------|------|
| `tam` | 打印 help 后退出 | 0 | `main.rs:35-38` |
| `tam -h` / `tam help` | clap help | 0 | clap |
| `tam -V` | 打印版本 | 0 | `cli.rs:9` |
| `tam -i` | TUI | 0（正常 q 退出） | `main.rs:31-32` |
| `tam -i project list` | **TUI，子命令被静默忽略** | 0 | `main.rs:31-34` |
| `tam project` / `tam task` | 等价 list | 0 | `cli.rs:67,102` |
| `tam config` | 打印 `config {:#?}` | 0 | `commands/config.rs:6` |
| 参数非法/未知子命令 | clap 报错到 stderr | 2 | clap 默认 |
| 业务失败（anyhow） | `ERROR ...` 到 **stdout** | 1 | `main.rs:52-55`、`utils.rs:15-24` |
| config 加载/DB 初始化失败 | 错误到 stderr | 1 | `config.rs:18-21`、`main.rs:25-28` |
| config 保存失败（use/del/edit 当前项目） | stderr 提示 | 1（直接 `exit`） | `commands/project.rs:33,60,87` |
| TUI panic / log4rs 初始化失败 | panic | 101 | `app.rs:324,336,349,358,395,417,422`、`logs.rs:23,33,43,45` |

### 3.3 输出约定
| 输出 | 形态 | 位置 |
|------|------|------|
| 成功 | `SUCCESS`（绿底加粗）+ 文本，`println!` 到 stdout | `utils.rs:3-12` |
| 提示 | `INFO`（蓝底）到 stdout | `utils.rs:26-36` |
| 错误 | `ERROR`（红底）**到 stdout**（非 stderr） | `utils.rs:14-24`、`main.rs:53` |
| 项目表 | 列 `Project | Created | Updated`，圆角表格，时间 `%Y-%m-%d %H:%M:%S` | `model.rs:26-46` |
| 任务表 | 列 `ID | Title | Status`，状态文案 `⏳ Todo`/`🚧 In Progress`/`✅ Done` | `model.rs:140-156`、`model.rs:124-132` |
| 空列表 | `No projects found. ...` / `INFO No tasks found. ...` | `commands/project.rs:16-18`、`commands/task.rs:20-22` |
| `config` | Rust `Debug` 结构体，不是 TOML 文件内容 | `commands/config.rs:6` |

`main` 忽略所有命令返回的 `Ok(bool)`（`main.rs:58-64`），失败只通过 `Err` 传播；因此 `services` 返回的 `bool` 无契约意义。

## 4. 持久化契约

### 4.1 DDL 与约束
| 对象 | 事实 | 位置 |
|------|------|------|
| `project` | `id INTEGER PK AUTOINCREMENT`、`name CHAR(255) NOT NULL UNIQUE`、`created/updated DATETIME DEFAULT CURRENT_TIMESTAMP`、`is_deleted BOOLEAN DEFAULT 0` | `sql.rs:2-8` |
| `task` | `id` PK、`title TEXT NOT NULL`、`content TEXT DEFAULT ''`、`created/updated`、`status INTEGER NOT NULL DEFAULT '0' CHECK (status IN ('0','1','2'))`、`project_id INTEGER NOT NULL`、`is_deleted`、`FOREIGN KEY (project_id) REFERENCES project(id)` | `sql.rs:22-32` |
| 触发器 | `project_update_timestamp`、`task_update_timestamp`：`AFTER UPDATE ... FOR EACH ROW` 执行 `UPDATE ... SET updated = CURRENT_TIMESTAMP` | `sql.rs:11-18,35-42` |
| 索引 | 无任何显式 `CREATE INDEX`；仅 PK 与 `project.name UNIQUE` 的隐式索引 | `sql.rs:1-89` |
| 初始化 | 每次启动执行 4 条 `CREATE ... IF NOT EXISTS`；无 `user_version`、无迁移 | `db.rs:23-30` |

触发器副效应：任何 `UPDATE`（含软删除、改名）都会把 `updated` 刷新为当前 UTC；SQLite 默认 `recursive_triggers=OFF`，不会自递归。

### 4.2 软删除语义（逐条 SQL）
| SQL 常量 | 匹配条件 | 软删除守卫 | 后果 |
|----------|----------|------------|------|
| `SELECT_PROJECTS_SQL` | `is_deleted = 0` | — | 列表只见活跃项目（`sql.rs:44-46`） |
| `SELECT_PROJECT_SQL` | `name = ? AND is_deleted = 0` | — | 已删名不可 `use`/被解析（`sql.rs:48-50`） |
| `DELETE_PROJECT_SQL` | 仅 `name = ?` | **无** | 已删项目可再次“删除成功”；不校验任务（`sql.rs:57-59`） |
| `UPDATE_PROJECT_SQL` | 仅 `name = ?` | **无** | 可改已软删行的名字，继续占用 UNIQUE（`sql.rs:61-63`） |
| `SELECT_ALL_TASKS_SQL` | `project_id = ? AND is_deleted = 0`，`ORDER BY created DESC` | — | 列表语义（`sql.rs:70-73`） |
| `SELECT_TASK_SQL` | `id = ? AND is_deleted = 0` | — | 已删任务不可再操作（`sql.rs:75-77`） |
| `UPDATE_TASK_STATUS_SQL` / `TITLE` | 仅 `id = ?` | 前置 `query_task` 已过滤 | 见 `sql.rs:79-85`、`services/task.rs:98-138` |
| `DELETE_TASK_SQL` | 仅 `id = ?` | 前置 `query_task` 已过滤 | 见 `sql.rs:87-89`、`services/task.rs:140-149` |

由此产生的确定行为：① `project.name` 的 UNIQUE 跨软删行生效 → 删除后无法重建同名项目（smoke 中 `re-add beta` 报 `project beta already exists`）；② 删除项目不级联任务，活跃任务保留且 CLI 无法再访问该 `project_id`；③ 删除已删项目 `rows=1`，返回成功但无任何输出；④ 改名可命中软删行。

### 4.3 外键与时间戳

- 外键：DDL 声明了 `FOREIGN KEY`（`sql.rs:31`），但 `src/` 内 `grep -ni "pragma|foreign_keys"` **零命中**，`get_db`（`db.rs:11-21`）未执行 `PRAGMA foreign_keys=ON`；SQLite 默认关闭 → 约束**实际不生效**，仅作装饰。smoke 证据：项目全删后 `project_id=1/2` 的任务仍存在（`baseline-smoke.log` 末尾 `db rows after deletes`）。
- 时区：SQLite `CURRENT_TIMESTAMP` 为 **UTC**，存储文本 `YYYY-MM-DD HH:MM:SS`；展示不做本地换算（`model.rs:34-39`）。smoke 中本地 15:14 对应显示 07:14:54，即 UTC。
- 解析路径：

| 读取路径 | 列索引 | 解析方式 | 结果 |
|----------|--------|----------|------|
| `services/project.rs:23-57` list | created=2, updated=**2** | `NaiveDateTime::parse_from_str("%Y-%m-%d %H:%M:%S")` | 两者同值；**updated 列读错**（`services/project.rs:39-40`） |
| `services/project.rs:59-82` query | 不读时间（SQL 只选 id,name） | — | `created/updated = None` |
| `services/task.rs:36-73` list | created=3, updated=4 | `NaiveDateTime::parse_from_str` | 正确 |
| `services/task.rs:75-97` query | created=3, updated=4 | `str::parse::<DateTime<Utc>>()` | 字符串无时区偏移，chrono 要求 RFC3339 偏移 → **必然 Err**，`unwrap_or_else(Utc::now())` 静默伪造当前时间（`services/task.rs:88-93`） |

### 4.4 连接、事务与并发

每次调用 `get_db()` 新建一条 `rusqlite::Connection`（`db.rs:11-21`），无连接池；`src/` 内 `grep -ni "transaction"` 零命中 → 所有多步操作（任务先查后改、项目改名“先写配置后写库”）**无事务**。未设置 `busy_timeout`，多进程/多实例同时写入时默认立即返回 `SQLITE_BUSY`。数据库无 schema 版本，`CREATE IF NOT EXISTS` 无法为既有库补列/改约束。

## 5. 数据流

### 5.1 CLI：project
| 命令 | 调用链 | config 写入 | 失败分支 |
|------|--------|-------------|----------|
| add | `cli.rs:68` → `commands/project.rs:7` → `services/project.rs:8` → `sql.rs:52` INSERT → 打印成功+提示（`commands/project.rs:9-10`） | 无 | UNIQUE → `project X already exists`（`services/project.rs:14-15`）；其他 DB 错误 → bail |
| list | `cli.rs:69` → `commands/project.rs:14` → `services/project.rs:23` → `sql.rs:44` → `model.rs:26` | 无 | DB 错误 → `Err` |
| use | `cli.rs:70` → `commands/project.rs:23` → `services/project.rs:59`（`sql.rs:48`）→ 赋值 `current_project` → `config.rs:84-90` 写文件 | **写**（`:26-27`） | 项目不存在 → anyhow；保存失败 → stderr + `exit(1)`（`:31-34`） |
| del | `cli.rs:71` → `commands/project.rs:45-67`：若 `current==name` 先清空并保存 → `services/project.rs:84` → `sql.rs:57` | 条件写（`:52-63`） | 不存在 → `project X not found`（`:89-90`）；**非当前项目删除无成功输出**；无任务关联校验（`TODO` 位于 `:49`） |
| current | `cli.rs:72` → `commands/project.rs:39` → `services/project.rs:100-107` 读 config → 若 Some 再 `query_project` → INFO 绿色文案（`:41`） | 无 | 未设置 → 固定文案 `No current project set. Use ...`（`services/project.rs:105`） |
| edit | `cli.rs:73` → `commands/project.rs:70-96`：同名提前返回（`:71-74`）；若当前项目**先写 config**（`:79-90`）→ `services/project.rs:109` → `sql.rs:61` | 条件写（**先于 DB**） | DB 失败 → 配置已漂移（smoke：`current project updated to 'beta'` 后 rename 失败）；UNIQUE → `Failed to update project ...`（`:118-120`） |

### 5.2 CLI：task
| 命令 | 调用链 | config 读取 | 失败分支 |
|------|--------|-------------|----------|
| add | `cli.rs:103` → `commands/task.rs:7` → `services/task.rs:13`；`p_id=None` → `services/project.rs:100` 取当前项目（`:17-28`）→ `sql.rs:65` INSERT | **读**（间接） | 无当前项目 → 固定文案（`services/task.rs:21-25`）；DB 错误 → `Err` |
| list | `commands/task.rs:13` → `services/project.rs:100` → `services/task.rs:36` → `sql.rs:70` → `model.rs:140`；先打印当前项目名（`commands/task.rs:23`） | **读** | 未设置当前项目 → `commands/task.rs:16` / `services/project.rs:105` |
| edit | `commands/task.rs:44` → `services/task.rs:129-138`：`query_task`（`:75-97`）+ `sql.rs:83` | **不读** | 无效 id → raw `Query returned no rows`（smoke 已复现） |
| del | `commands/task.rs:29` → `services/task.rs:140-149` → `sql.rs:87` | **不读** | 同上 |
| start | `commands/task.rs:34` → `services/task.rs:98-113`：已是 InProgress → 拒绝（`:101-106`）；否则 `sql.rs:79` | **不读** | 任务不存在 → raw 错误；无状态机事务 |
| done | `commands/task.rs:39` → `services/task.rs:115-127`：已是 Done → 拒绝（`:118-120`）；否则 `sql.rs:79` | **不读** | 同上 |

状态机事实：`start` 允许 Done→InProgress，`done` 允许 Todo→Done（只做“已是目标态”短路）；`content` 列无任何写入/读取（仅 DDL `sql.rs:25`）。

### 5.3 TUI：动作与调用链

启动：`App::new` 读 `theme`（`app.rs:55`）；`run` 每帧 `draw` 后 `event::read`（`app.rs:59-77`）；Project 屏每帧 `query_projects`（`ui/project_screen.rs:29` → `app.rs:416-418`），Task 屏每帧 `get_project` + `query_tasks`（`ui/task_screen.rs:31-36` → `app.rs:419-425`）。

| 屏 | 按键 | 动作 | 调用链 / 写入 | 失败分支 |
|----|------|------|----------------|----------|
| Project | `q` | 退出 | `app.rs:115-117` | — |
| Project | `Enter` | 进入任务屏 | `app.rs:118-123`（需已选项目） | 未选则无动作 |
| Project | `↑/↓` | 选择循环 | `app.rs:124-125,233-263` | 空列表直接返回 |
| Project | `a` | 新增弹窗 | `app.rs:126` → `submit_project` `app.rs:310-320` → `services::project::add`（`:313`） | `Err` 转 `tip_message` |
| Project | `e` | 改名弹窗（预填旧名） | `app.rs:127-134` → `submit_project` `:321-327` → `services::project::edit`（`:323`）`.unwrap()` | **panic**；**不同步 config.current_project** |
| Project | `d` | 确认弹窗 | `app.rs:135-142`，确认 `:90-99` → `delete_project` `:356-360` → `services::project::delete`（`:358`）`.unwrap()` | **panic**；**不清 config**；选中索引可能越界 |
| Project | `t` | 切换主题并持久化 | `app.rs:143` → `change_theme` `:400-414` → `config.set_theme`（`:413`，`config.rs:91-99`） | 保存失败仅 `log::info` |
| Task | `q` | 退出 | `app.rs:182-184` | — |
| Task | `↑/↓` | 当前状态列内循环 | `app.rs:185-186,265-297` | 空列返回 |
| Task | `←/→` | 切换状态列（Todo→InProgress→Done→Todo） | `app.rs:187-188,299-308` | — |
| Task | `a` | 新增任务 | `app.rs:189,330-341` → `services::task::add(project.id)`（`:336`）`.unwrap()` | **panic**；**目标项目取 UI 选中项，不读 config** |
| Task | `e` | 编辑标题 | `app.rs:190-197` → `submit_task` `:342-353` → `services::task::edit`（`:345`）`.unwrap()` | **panic** |
| Task | `s` | 开始 | `app.rs:198` → `start_task` `:362-375` → `services::task::start`（`:365`） | 错误 → 中文 tip「任务开始失败」 |
| Task | `c` | 完成 | `app.rs:199` → `complete_task` `:377-390` → `services::task::done`（`:380`） | 错误 → tip「任务完成失败」 |
| Task | `d` | 删除确认 | `app.rs:200-207,158-167` → `delete_task` `:392-398` → `services::task::delete`（`:395`）`.unwrap()` | **panic** |
| Task | `Esc` | 返回项目屏 | `app.rs:208-211` | — |
| Task | `t` | 切换主题 | `app.rs:212-213` | 同 Project |

TUI 关键契约事实：**全仓 TUI 无任何 `current_project` 赋值**（`app.rs` 仅 `:55,412` 读 config），因此 TUI 选中的项目与 CLI 的“当前项目”是两套互不同步的概念；TUI 的增删改都绕开 `commands` 层，直接调用 `services`（`:313,323,336,345,358,365,380,395,417,422`），导致 CLI 在 `commands` 层实现的 config 同步逻辑在 TUI 侧完全缺失。

### 5.4 config 写入点汇总
| 写入点 | 触发 | 失败行为 |
|--------|------|----------|
| `config.rs:66-70`（默认值落盘） | 首次运行/配置缺失 | `exit(1)`（外层 Lazy，`config.rs:18-21`） |
| `commands/project.rs:26-27`（use） | 切换当前项目 | stderr + `exit(1)`（`:31-34`） |
| `commands/project.rs:52-62`（del 当前项目） | 删除当前项目 | stderr + `exit(1)`（`:58-61`） |
| `commands/project.rs:75-90`（edit 当前项目） | 改名当前项目 | stderr + `exit(1)`（`:85-88`）；**先于 DB 写入** |
| `app.rs:412-413`（theme） | TUI 按 `t` | 仅记录日志，不中断 |

读取点：`config::get_config`（`config.rs:26-28`，返回 **Clone**，进程内不刷新 Lazy）、`logs.rs:16`、`db.rs:12`、`services/project.rs:101`、`app.rs:55,412`、`commands/config.rs:4`。

## 6. 外部集成与发布链
| 依赖 | 版本（`Cargo.toml`） | 实际用途 | 证据 |
|------|---------------------|----------|------|
| `rusqlite` | 0.37 `bundled` | 内嵌 SQLite，无系统库依赖 | `Cargo.toml:36`、`db.rs:19` |
| `toml` | 0.9 | 配置读写（无 serde default，字段必填） | `config.rs:73,87` |
| `log4rs`/`log` | 1.3 / 0.4 | 滚动文件日志（10 MB × 5） | `logs.rs:18-45` |
| `clap` | 4.5 derive | 命令树/别名/help/version；自身错误退出码 2 | `cli.rs:1-111` |
| `ratatui`/`crossterm`/`tui-input` | 0.29/0.29/0.12 | TUI 渲染、事件、行输入 | `app.rs`, `ui/*` |
| `tabled` | 0.20 | 圆角表格 | `model.rs:7,26-46,140-156` |
| `colored` | 3.0 | 输出标签与 help 版本着色 | `utils.rs`, `cli.rs:2` |
| `chrono` | 0.4 `serde` | 时间戳解析/UTC 类型 | `model.rs:1`, `services/*` |
| `serde`/`serde_json` | 1.0.219 / 1.0.140 | serde 用于 config、model derive；**`serde_json` 在 `src/` 无任何引用（死依赖）** | `Cargo.toml:27-28`；`src/` 内 grep 零命中 |
| `once_cell` | 1.21 | config 全局 `Lazy` | `config.rs:3,15` |
| `env_home` | 0.1 | Unix 读 `HOME`、Windows 读 `USERPROFILE`（crate 文档事实） | `config.rs:77` |
| `anyhow` | 1.0 | 命令层错误类型 | `main.rs:52` |

| 发布环节 | 工具/位置 | 事实 |
|----------|-----------|------|
| 版本 | `Cargo.toml:2` = 1.0.13；CHANGELOG 1.0.13 日期 2025-08-04 | 当前版本已发布 |
| 版本 bump | `release.sh:5`：`cargo release patch --no-verify --no-publish --execute` | `cargo-release` 未在 `Cargo.toml` 声明为依赖，需全局安装 |
| CHANGELOG | `release.toml:1-8` pre-release hook 调 `git cliff`；`cliff.toml` | `git-cliff` 亦需全局安装 |
| 跨平台构建 | `dist-workspace.toml:7` cargo-dist `0.28.1-prerelease.2`；7 个 target；installers = shell/powershell/npm/homebrew/msi | 使用 prerelease 版工具 |
| GitHub workflow | `.github/workflows/release.yml`：tag push（`**[0-9]+.[0-9]+.[0-9]+*`）→ plan/build/host/publish/announce | 自动生成，勿手改 |
| PR 门禁 | `pr-run-mode = "plan"`（`dist-workspace.toml:29`） | PR 仅跑 `dist plan`，**不编译、不测试** |
| crates.io | `.github/workflows/publish-crates.yml:26`：`cargo publish -p tam`，由 dist 的 `custom-publish-crates`（`release.yml:350-361`）调用 | 仅发布阶段 |
| npm / Homebrew / MSI | `dist-workspace.toml:11,27,35-38`（tap `wst7/homebrew-tap`、npm scope `@wst7`） | `publish-jobs` 中 `homebrew` 重复出现（`:27`），配置异味 |
| CI 质量门禁 | `.github/` 全目录无 `cargo test` / `cargo fmt` / `cargo clippy`（grep 零命中） | **无任何质量门禁** |

## 7. 技术债与风险清单
| # | 严重度 | 风险 | 影响 | 证据 |
|---|--------|------|------|------|
| R1 | P0 | 改名“先写 config，后写 DB”，无事务 | DB 失败后 `current_project` 指向错误项目，后续 `task add` 写错归档（smoke 已复现） | `commands/project.rs:75-94`、`services/project.rs:109-122` |
| R2 | P0 | 删除项目不校验/不级联任务，FK 未启用 | 任务成为不可见孤儿，数据一致性破坏 | `commands/project.rs:48-65`（TODO `:49`）、`sql.rs:31,57-59`、`db.rs:11-21` |
| R3 | P0 | `project list` 的 `updated` 读取 `created` 列 | 输出错误数据，任何时间维度判断失效 | `services/project.rs:39-40` |
| R4 | P0 | 软删除 + `name UNIQUE` 跨行生效 | 删除后同名项目永不可重建；改名目标可为已删名 | `sql.rs:4,57-63`、smoke `re-add beta` |
| R5 | P0 | TUI 多处 `unwrap()` + `selected_project` 索引越界 | 交互中直接 panic 退出（101），且可能发生在 draw 阶段 | `app.rs:324,336,349,358,395,417,422`、索引 `app.rs:426-432` 被 `ui/task_screen.rs:31` 调用 |
| R6 | P0 | 测试硬编码真实 HOME 并写真实配置，且被当作测试套件 | `cargo test` 污染用户 `~/.tam`（已发生一次）；换机/改 HOME 即失败 | `config.rs:112,121,135-150`；`test.exit=0` 掩盖副作用 |
| R7 | P0 | 无 CI 质量门禁；`cargo fmt --check` 当前失败 | 合入不可验证，格式/回归靠运气 | `.github/**` grep 零命中；`fmt.exit=1`，8 文件 22 处差异 |
| R8 | P1 | `query_task` 时间解析必然失败并静默回退 `Utc::now()` | 任务时间戳被伪造，未来任何时间展示/排序会错 | `services/task.rs:88-93`；chrono 要求 RFC3339 偏移 |
| R9 | P1 | 无效 task id 直接暴露 `Query returned no rows` | 用户错误提示不友好、不可定位 | `services/task.rs:78-95`；smoke `invalid task id` |
| R10 | P1 | 删除非当前项目无成功输出；删除已删项目静默成功 | CLI 反馈不一致、幂等语义含糊 | `commands/project.rs:45-66`、`sql.rs:57` |
| R11 | P1 | 软删除行仍可被 `del/edit` 的 SQL 命中 | `edit` 可改名隐藏行，继续占用名字 | `sql.rs:57-63` |
| R12 | P1 | 无事务、无 `busy_timeout`、每操作新连接 | 并发写 `SQLITE_BUSY`；先查后改存在竞态 | `db.rs:11-21`；grep transaction/pragma 零命中 |
| R13 | P1 | 无 schema 版本/迁移；config 无 serde default | 旧库缺列无法自动升级；旧配置缺字段直接 `exit(1)` | `db.rs:23-30`、`config.rs:30-37,72-75` |
| R14 | P1 | log4rs 初始化全用 `expect` | 日志路径异常时启动即 panic（101），非友好错误 | `logs.rs:23,33,43,45` |
| R15 | P1 | `-i` 与子命令同时给出时静默忽略子命令 | 脚本误用无提示 | `main.rs:31-34` |
| R16 | P1 | 业务错误打印到 stdout（`ERROR`），部分配置错误走 stderr | 管道/脚本按 stderr 判错会漏 | `utils.rs:15-24` vs `commands/project.rs:32,59,86` |
| R17 | P2 | `project list` 无 `ORDER BY`；`task` 按 `created DESC` 但秒级精度 | 项目顺序依赖 rowid，不稳定 | `sql.rs:44-46,70-73` |
| R18 | P2 | 死代码/死依赖：`content` 列、`serde_json`、`TaskStatus::is_*`、`set_title`、`Theme.tip_*` | 误导维护者，clippy 17 条 warning 不阻断 | `sql.rs:25`；`Cargo.toml:28`；clippy 证据 |
| R19 | P2 | 发布链使用 cargo-dist prerelease；`publish-jobs` 重复 homebrew | 发布可重复性/可维护性风险 | `dist-workspace.toml:7,27` |
| R20 | P2 | 时间统一 UTC 且不提示时区 | 本地用户误读时间 | `model.rs:34-39`；smoke 07:14:54 vs 本地 15:14 |

## 8. 与文档（AGENTS/README/SKILL/design）不一致对照
| # | 文档声明 | 代码事实 | 位置 |
|---|----------|----------|------|
| D1 | README Usage 示例 `Version: 1.0.10` | 实际 1.0.13 | `README.md:66` vs `Cargo.toml:2` |
| D2 | README 徽章/License 段只写 MIT | `Cargo.toml` 声明 `MIT OR Apache-2.0`；仓库仅有 `LICENSE.md`(MIT)，无 Apache 文件 | `README.md:4,122`、`Cargo.toml:11`、`AGENTS.md:77` |
| D3 | AGENTS：测试内联于 `src/`（举例 `config.rs`、`db.rs`） | `db.rs` 仅有空测试模块；真正的副作用测试在 `config.rs`，会写真实配置 | `db.rs:32-35`、`config.rs:134-150` |
| D4 | AGENTS：`cargo test` 作为标准测试入口 | 该命令当前会写入用户真实 `~/.tam/tam.config.toml`（未隔离 HOME） | `config.rs:135-141,143-150` |
| D5 | AGENTS：Architecture 列表 | 漏列 `model.rs`、`utils.rs`、`ui/components.rs` 等实际模块 | `AGENTS.md` vs `src/` |
| D6 | SKILL：`tam task` 命令“几乎都依赖当前项目” | 仅 `task add/list` 依赖；`edit/del/start/done` 凭全局 id 工作，不读 config | `SKILL.md:13,87` vs `services/task.rs:75-149` |
| D7 | SKILL：`tam config` 展示 `~/.tam/tam.config.toml` | 实际打印 `Config` 的 Rust Debug 结构，非文件内容 | `SKILL.md:62` vs `commands/config.rs:4-7` |
| D8 | SKILL：数据模型含 `content` 列（隐含可用） | 全代码无读写，仅 DDL 存在 | `SKILL.md:70` vs `sql.rs:25` |
| D9 | SKILL：软删除/触发器/滚动日志描述 | 与代码一致（复核通过） | `SKILL.md:14,74-75,79-83` vs `sql.rs:11-42`、`logs.rs:18-23` |
| D10 | AGENTS/README：发布需 `release.sh` + git-cliff | 一致，但工具未声明为依赖，`--no-verify` 跳过发布前校验，本地无测试环节 | `release.sh:5`、`release.toml:1-8` |
| D11 | `design.txt`：`tam config [key [value]]`；task 字段 `id,name,project-id` | CLI `config` 无参数；task 实际为 `title/content/status/created/updated/project_id/is_deleted` | `design.txt:8-11,15` vs `cli.rs:40`、`sql.rs:22-32` |

## 附：证据与复核限制

- 只读证据：`docs/forge-takeover/evidence/{build,test,fmt,clippy}.{log,exit}`、`baseline-smoke.log`、`versions.txt`；smoke 中 `updated` 与 `created` 同值即 R3 的运行时证据。
- 未复跑 `cargo build/test`，未执行任何会触达 `~/.tam` 的二进制命令；`tam -h` 等输出语义由 clap 派生属性与 `main.rs` 代码推定。
- 对任务书已知事实的复核差异：TUI 直接调用 services 的行号为 `313,323,336,345,358,365,380,395,417,422`（任务书写作 `346/381/415`）；`tests/` 目录确实不存在，但 `Cargo.toml` 的 `exclude` 只影响打包，不影响 `cargo test` 的副作用；`--interactive` 与子命令同时出现时是“TUI 优先、命令丢弃”，而非报错。
