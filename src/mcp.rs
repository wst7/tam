//! MCP stdio server (JSON-RPC 2.0, newline delimited).
//!
//! stdout carries protocol messages ONLY; diagnostics go through the file
//! logger / stderr. Every response is flushed so interactive clients see it.

use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use crate::agent::{AgentError, Ctx};
use crate::cli::StatusArg;
use crate::commands::agent as ops;
use crate::model::TaskStatus;
use crate::services::task;

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "tam";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                write_message(
                    &mut out,
                    &jsonrpc_error(Value::Null, -32700, format!("Parse error: {}", error)),
                )?;
                continue;
            }
        };
        if let Some(response) = handle_request(&request) {
            write_message(&mut out, &response)?;
        }
    }
    Ok(())
}

fn write_message(out: &mut impl Write, message: &Value) -> anyhow::Result<()> {
    writeln!(out, "{}", message)?;
    out.flush()?;
    Ok(())
}

fn handle_request(request: &Value) -> Option<Value> {
    // Notifications (no id) never receive a response.
    let id = request.get("id").cloned()?;

    if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Some(jsonrpc_error(
            id,
            -32600,
            "Invalid Request: jsonrpc must be '2.0'",
        ));
    }
    let method = match request.get("method").and_then(Value::as_str) {
        Some(method) => method,
        None => return Some(jsonrpc_error(id, -32600, "Invalid Request: missing method")),
    };

    match method {
        "initialize" => Some(result_response(
            &id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
            }),
        )),
        "ping" => Some(result_response(&id, json!({}))),
        "tools/list" => Some(result_response(&id, json!({ "tools": tool_definitions() }))),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or(Value::Null);
            Some(handle_tools_call(&id, &params))
        }
        _ => Some(jsonrpc_error(
            id,
            -32601,
            format!("Method not found: {}", method),
        )),
    }
}

fn handle_tools_call(id: &Value, params: &Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return jsonrpc_error(id.clone(), -32602, "Invalid params: missing tool name");
    };
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    match call_tool(name, &arguments) {
        Ok(data) => {
            let text = serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string());
            result_response(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": false,
                }),
            )
        }
        Err(error) => {
            let text = serde_json::to_string(&error.to_json()).unwrap_or_else(|_| "{}".to_string());
            result_response(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": true,
                }),
            )
        }
    }
}

fn call_tool(name: &str, args: &Value) -> Result<Value, AgentError> {
    let project = string_arg(args, "project");
    let agent = string_arg(args, "agent");
    let ctx = Ctx::new(true, project.clone(), agent.clone());

    match name {
        "tam_status" => ops::op_status(&ctx).map(|outcome| outcome.data),
        "tam_project_list" => ops::op_project_list().map(|outcome| outcome.data),
        "tam_task_list" => {
            let filters = ops::ListFilters {
                status: status_arg(args)?,
                assigned: string_arg(args, "assigned"),
                priority: i32_arg(args, "priority")?,
                tags: tags_arg(args, "tag")?,
                search: string_arg(args, "search"),
                limit: usize_arg(args, "limit")?,
            };
            ops::op_task_list(&ctx, &filters).map(|outcome| outcome.data)
        }
        "tam_task_get" => {
            let id = required_i32(args, "id")?;
            ops::op_task_show(&ctx, id).map(|outcome| outcome.data)
        }
        "tam_task_create" => {
            let title = required_string(args, "title")?;
            ops::op_task_add(
                &ctx,
                title,
                string_arg(args, "description"),
                i32_arg(args, "priority")?,
                tags_arg(args, "tags")?,
                string_arg(args, "idempotency_key"),
            )
            .map(|outcome| outcome.data)
        }
        "tam_task_update" => {
            let id = required_i32(args, "id")?;
            let input = task::TaskUpdateInput {
                title: string_arg(args, "title"),
                description: string_arg(args, "description"),
                status: task_status_arg(args)?,
                priority: i32_arg(args, "priority")?,
                tags: tags_update_arg(args, "tags")?,
            };
            ops::op_task_update(&ctx, id, input).map(|outcome| outcome.data)
        }
        "tam_task_claim" => {
            let id = required_i32(args, "id")?;
            let agent = required_string(args, "agent")?;
            let ctx = Ctx::new(true, project, Some(agent));
            ops::op_task_claim(
                &ctx,
                id,
                i64_arg(args, "lease_seconds")?.unwrap_or(task::DEFAULT_LEASE_SECONDS),
                bool_arg(args, "force"),
            )
            .map(|outcome| outcome.data)
        }
        "tam_task_heartbeat" => {
            let id = required_i32(args, "id")?;
            let agent = required_string(args, "agent")?;
            let ctx = Ctx::new(true, project, Some(agent));
            ops::op_task_heartbeat(
                &ctx,
                id,
                i64_arg(args, "lease_seconds")?.unwrap_or(task::DEFAULT_LEASE_SECONDS),
            )
            .map(|outcome| outcome.data)
        }
        "tam_task_release" => {
            let id = required_i32(args, "id")?;
            ops::op_task_release(&ctx, id, bool_arg(args, "force")).map(|outcome| outcome.data)
        }
        "tam_task_complete" => {
            let id = required_i32(args, "id")?;
            ops::op_task_done(&ctx, id).map(|outcome| outcome.data)
        }
        "tam_task_next" => {
            let agent = required_string(args, "agent")?;
            let ctx = Ctx::new(true, project, Some(agent));
            ops::op_task_next(
                &ctx,
                bool_arg(args, "claim"),
                i64_arg(args, "lease_seconds")?.unwrap_or(task::DEFAULT_LEASE_SECONDS),
            )
            .map(|outcome| outcome.data)
        }
        "tam_task_events" => {
            let id = required_i32(args, "id")?;
            ops::op_task_events(&ctx, id).map(|outcome| outcome.data)
        }
        other => Err(AgentError::invalid(format!("unknown tool: {}", other))),
    }
}

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

fn string_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(|value| value.to_string())
}

fn required_string(args: &Value, key: &str) -> Result<String, AgentError> {
    string_arg(args, key)
        .ok_or_else(|| AgentError::invalid(format!("missing required argument: {}", key)))
}

fn required_i32(args: &Value, key: &str) -> Result<i32, AgentError> {
    let value = args
        .get(key)
        .ok_or_else(|| AgentError::invalid(format!("missing required argument: {}", key)))?;
    value
        .as_i64()
        .map(|value| value as i32)
        .ok_or_else(|| AgentError::invalid(format!("argument '{}' must be a number", key)))
}

fn i32_arg(args: &Value, key: &str) -> Result<Option<i32>, AgentError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(|value| Some(value as i32))
            .ok_or_else(|| AgentError::invalid(format!("argument '{}' must be a number", key))),
    }
}

fn i64_arg(args: &Value, key: &str) -> Result<Option<i64>, AgentError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| AgentError::invalid(format!("argument '{}' must be a number", key))),
    }
}

fn usize_arg(args: &Value, key: &str) -> Result<Option<usize>, AgentError> {
    match i64_arg(args, key)? {
        None => Ok(None),
        Some(value) if value >= 0 => Ok(Some(value as usize)),
        Some(_) => Err(AgentError::invalid(format!(
            "argument '{}' must be positive",
            key
        ))),
    }
}

fn bool_arg(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn status_arg(args: &Value) -> Result<Option<StatusArg>, AgentError> {
    let Some(value) = string_arg(args, "status") else {
        return Ok(None);
    };
    match value.as_str() {
        "todo" => Ok(Some(StatusArg::Todo)),
        "in-progress" => Ok(Some(StatusArg::InProgress)),
        "done" => Ok(Some(StatusArg::Done)),
        other => Err(AgentError::invalid(format!(
            "invalid status {:?}: expected todo|in-progress|done",
            other
        ))),
    }
}

fn task_status_arg(args: &Value) -> Result<Option<TaskStatus>, AgentError> {
    Ok(status_arg(args)?.map(TaskStatus::from))
}

/// `tags`/`tag` accept either an array of strings or a comma separated string.
fn tags_value(value: &Value, key: &str) -> Result<Vec<String>, AgentError> {
    match value {
        Value::String(raw) => Ok(raw
            .split(',')
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect()),
        Value::Array(values) => {
            let mut tags = Vec::new();
            for value in values {
                let tag = value.as_str().ok_or_else(|| {
                    AgentError::invalid(format!("'{}' must contain strings", key))
                })?;
                tags.push(tag.to_string());
            }
            Ok(tags)
        }
        _ => Err(AgentError::invalid(format!(
            "'{}' must be a string or an array of strings",
            key
        ))),
    }
}

/// Tool arguments where a missing/`null` field means "do not change", while an
/// explicit `""` or `[]` means "set to empty" (D4: MCP must be able to clear
/// tags just like the CLI).
fn tags_update_arg(args: &Value, key: &str) -> Result<Option<Vec<String>>, AgentError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => Ok(Some(tags_value(value, key)?)),
    }
}

fn tags_arg(args: &Value, key: &str) -> Result<Vec<String>, AgentError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(value) => tags_value(value, key),
    }
}

// ---------------------------------------------------------------------------
// Tool catalog (12 tools)
// ---------------------------------------------------------------------------

fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "tam_status",
            "Project status summary: counts, leases, unassigned tasks and next candidates.",
            json!({
                "type": "object",
                "properties": { "project": { "type": "string" } },
            }),
        ),
        tool(
            "tam_project_list",
            "List projects.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "tam_task_list",
            "List tasks with optional filters (default ordering created DESC).",
            json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "status": { "type": "string", "enum": ["todo", "in-progress", "done"] },
                    "assigned": { "type": "string" },
                    "priority": { "type": "integer", "minimum": 1, "maximum": 5 },
                    "tag": { "type": "string" },
                    "search": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 1000 }
                },
            }),
        ),
        tool(
            "tam_task_get",
            "Get one task with its event log.",
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"],
            }),
        ),
        tool(
            "tam_task_create",
            "Create a task (idempotent when idempotency_key is provided).",
            json!({
                "type": "object",
                "properties": {
                    "project": { "type": "string" },
                    "title": { "type": "string" },
                    "description": { "type": "string" },
                    "priority": { "type": "integer", "minimum": 1, "maximum": 5 },
                    "tags": { "type": ["string", "array"], "items": { "type": "string" } },
                    "idempotency_key": { "type": "string" }
                },
                "required": ["title"],
            }),
        ),
        tool(
            "tam_task_update",
            "Partially update a task.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "title": { "type": "string" },
                    "description": { "type": "string" },
                    "status": { "type": "string", "enum": ["todo", "in-progress", "done"] },
                    "priority": { "type": "integer", "minimum": 1, "maximum": 5 },
                    "tags": { "type": ["string", "array"], "items": { "type": "string" } }
                },
                "required": ["id"],
            }),
        ),
        tool(
            "tam_task_claim",
            "Atomically claim a task lease (conflict when another agent holds a valid lease).",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "agent": { "type": "string" },
                    "lease_seconds": { "type": "integer", "minimum": 5, "maximum": 86400 },
                    "force": { "type": "boolean" }
                },
                "required": ["id", "agent"],
            }),
        ),
        tool(
            "tam_task_heartbeat",
            "Renew the lease of a task you hold.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "agent": { "type": "string" },
                    "lease_seconds": { "type": "integer", "minimum": 5, "maximum": 86400 }
                },
                "required": ["id", "agent"],
            }),
        ),
        tool(
            "tam_task_release",
            "Release a task lease (in-progress tasks return to todo).",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "agent": { "type": "string" },
                    "force": { "type": "boolean" }
                },
                "required": ["id"],
            }),
        ),
        tool(
            "tam_task_complete",
            "Mark a task done and clear its lease.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "agent": { "type": "string" }
                },
                "required": ["id"],
            }),
        ),
        tool(
            "tam_task_next",
            "Return (and optionally claim) the highest priority claimable task.",
            json!({
                "type": "object",
                "properties": {
                    "agent": { "type": "string" },
                    "project": { "type": "string" },
                    "lease_seconds": { "type": "integer", "minimum": 5, "maximum": 86400 },
                    "claim": { "type": "boolean" }
                },
                "required": ["agent"],
            }),
        ),
        tool(
            "tam_task_events",
            "Return the event log of a task.",
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer" } },
                "required": ["id"],
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

// ---------------------------------------------------------------------------
// JSON-RPC envelope helpers
// ---------------------------------------------------------------------------

fn result_response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn jsonrpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message.into() },
    })
}
