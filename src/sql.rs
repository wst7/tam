pub const CREATE_PROJECT_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS project (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name CHAR(255) NOT NULL UNIQUE,
    created DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_deleted BOOLEAN DEFAULT 0
);
"#;

pub const CREATE_PROJECT_TABLE_TRIGGER_SQL: &str = r#"
CREATE TRIGGER IF NOT EXISTS project_update_timestamp
AFTER UPDATE ON project
FOR EACH ROW
BEGIN
    UPDATE project SET updated = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
"#;


pub const CREATE_TASK_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS task (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT DEFAULT '',
    created DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated DATETIME DEFAULT CURRENT_TIMESTAMP,
    status INTEGER NOT NULL DEFAULT '0' CHECK (status IN ('0', '1', '2')),
    project_id INTEGER NOT NULL,
    is_deleted BOOLEAN DEFAULT 0,
    FOREIGN KEY (project_id) REFERENCES project(id)
);
"#;

pub const CREATE_TASK_TABLE_TRIGGER_SQL: &str = r#"
CREATE TRIGGER IF NOT EXISTS task_update_timestamp
AFTER UPDATE ON task
FOR EACH ROW
BEGIN
    UPDATE task SET updated = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
"#;

pub const SELECT_PROJECTS_SQL: &str = r#"
SELECT id, name, created, updated FROM project WHERE is_deleted = 0;
"#;

pub const SELECT_PROJECT_SQL: &str = r#"
SELECT id, name FROM project WHERE name = ? AND is_deleted = 0;
"#;

pub const CREATE_PROJECT_SQL: &str = r#"
INSERT INTO project (name) VALUES (?);
"#;


pub const DELETE_PROJECT_SQL: &str = r#"
UPDATE project SET is_deleted = 1 WHERE name = ?;
"#;

pub const UPDATE_PROJECT_SQL: &str = r#"
UPDATE project SET name = ? WHERE name = ?;
"#;

pub const CREATE_TASK_SQL: &str = r#"
INSERT INTO task (title, project_id) VALUES (?, ?);
"#;


pub const SELECT_ALL_TASKS_SQL: &str = r#"
SELECT id, title, status, created, updated FROM task WHERE project_id = ? AND is_deleted = 0
ORDER BY created DESC;
"#;

pub const SELECT_TASK_SQL: &str = r#"
SELECT id, title, status, created, updated FROM task WHERE id = ? AND is_deleted = 0;
"#;

pub const UPDATE_TASK_STATUS_SQL: &str = r#"
UPDATE task SET status = ? WHERE id = ?;
"#;

pub const UPDATE_TASK_TITLE_SQL: &str = r#"
UPDATE task SET title = ? WHERE id = ?;
"#;

pub const DELETE_TASK_SQL: &str = r#"
UPDATE task SET is_deleted = 1 WHERE id = ?;
"#;