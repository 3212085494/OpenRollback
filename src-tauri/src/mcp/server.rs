//! 简易 MCP stdio 服务器实现（JSON-RPC 2.0）。
//!
//! 支持 `initialize`、`tools/list`、`tools/call` 方法。

use crate::core::SnapshotEngine;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Mutex;

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "openrollback";
const SERVER_VERSION: &str = "0.1.0";

/// 启动 MCP stdio 服务（阻塞主线程）。
pub fn run_stdio_server() -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = SnapshotEngine::default_base_dir()?;
    let engine = SnapshotEngine::new(&base_dir)?;
    let service = McpService {
        engine: Mutex::new(engine),
        initialized: Mutex::new(false),
    };

    crate::debug_log!("[openrollback-mcp] listening on stdio");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(line) {
            Ok(req) => service.handle_request(req),
            Err(err) => Some(JsonRpcResponse::error(
                None,
                -32700,
                format!("Parse error: {err}"),
            )),
        };

        if let Some(resp) = response {
            let out = serde_json::to_string(&resp)?;
            writeln!(stdout, "{out}")?;
            stdout.flush()?;
        }
    }

    Ok(())
}

/// MCP 服务内部状态。
struct McpService {
    engine: Mutex<SnapshotEngine>,
    initialized: Mutex<bool>,
}

impl McpService {
    fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        if req.id.is_none() && req.method.as_deref() == Some("notifications/initialized") {
            return None;
        }

        let method = req.method.as_deref().unwrap_or("");
        let id = req.id;

        match method {
            "initialize" => Some(self.handle_initialize(id)),
            "tools/list" => self.guard_initialized(id.clone(), |s| s.handle_tools_list(id)),
            "tools/call" => {
                let params = req.params.unwrap_or(json!({}));
                self.guard_initialized(id.clone(), |s| s.handle_tools_call(id, params))
            }
            "ping" => Some(JsonRpcResponse::success(id, json!({}))),
            "" => Some(JsonRpcResponse::error(id, -32600, "Invalid Request".into())),
            other => Some(JsonRpcResponse::error(
                id,
                -32601,
                format!("Method not found: {other}"),
            )),
        }
    }

    fn guard_initialized<F>(&self, id: Option<Value>, f: F) -> Option<JsonRpcResponse>
    where
        F: FnOnce(&Self) -> JsonRpcResponse,
    {
        let ready = self.initialized.lock().map(|g| *g).unwrap_or(false);
        if !ready {
            return Some(JsonRpcResponse::error(
                id,
                -32002,
                "Server not initialized; call initialize first".into(),
            ));
        }
        Some(f(self))
    }

    fn handle_initialize(&self, id: Option<Value>) -> JsonRpcResponse {
        if let Ok(mut init) = self.initialized.lock() {
            *init = true;
        }
        JsonRpcResponse::success(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            }),
        )
    }

    fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            json!({
                "tools": [
                    tool_def(
                        "openrollback_watch",
                        "Start watching a directory for file changes",
                        json!({
                            "type": "object",
                            "properties": {
                                "directory_path": {
                                    "type": "string",
                                    "description": "Path to watch recursively"
                                }
                            },
                            "required": ["directory_path"]
                        }),
                    ),
                    tool_def(
                        "openrollback_stop_watch",
                        "Stop the active file watcher",
                        json!({ "type": "object", "properties": {} }),
                    ),
                    tool_def(
                        "openrollback_commit",
                        "Create a snapshot from pending file changes",
                        json!({
                            "type": "object",
                            "properties": {
                                "description": {
                                    "type": "string",
                                    "description": "Snapshot description"
                                }
                            },
                            "required": ["description"]
                        }),
                    ),
                    tool_def(
                        "openrollback_rollback",
                        "Rollback file system to undo a snapshot",
                        json!({
                            "type": "object",
                            "properties": {
                                "snapshot_id": {
                                    "type": "integer",
                                    "description": "Snapshot ID"
                                }
                            },
                            "required": ["snapshot_id"]
                        }),
                    ),
                    tool_def(
                        "openrollback_snapshots",
                        "List all snapshots",
                        json!({ "type": "object", "properties": {} }),
                    ),
                    tool_def(
                        "openrollback_delete_snapshot",
                        "Delete a snapshot record and its backups without changing live files",
                        json!({
                            "type": "object",
                            "properties": {
                                "snapshot_id": {
                                    "type": "integer",
                                    "description": "Snapshot ID to delete"
                                }
                            },
                            "required": ["snapshot_id"]
                        }),
                    ),
                ]
            }),
        )
    }

    fn handle_tools_call(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let result = match name {
            "openrollback_watch" => self.tool_watch(&args),
            "openrollback_stop_watch" => self.tool_stop_watch(),
            "openrollback_commit" => self.tool_commit(&args),
            "openrollback_rollback" => self.tool_rollback(&args),
            "openrollback_snapshots" => self.tool_snapshots(),
            "openrollback_delete_snapshot" => self.tool_delete_snapshot(&args),
            other => Err(format!("Unknown tool: {other}")),
        };

        match result {
            Ok(text) => JsonRpcResponse::success(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": false
                }),
            ),
            Err(err) => JsonRpcResponse::success(
                id,
                json!({
                    "content": [{ "type": "text", "text": err }],
                    "isError": true
                }),
            ),
        }
    }

    fn with_engine<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&SnapshotEngine) -> Result<T, crate::error::OpenRollbackError>,
    {
        let engine = self
            .engine
            .lock()
            .map_err(|_| "engine lock poisoned".to_string())?;
        f(&engine).map_err(|e| e.to_string())
    }

    fn tool_watch(&self, args: &Value) -> Result<String, String> {
        let path = args
            .get("directory_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing directory_path".to_string())?;
        self.with_engine(|engine| engine.start_watching(PathBuf::from(path).as_path()))?;
        Ok(format!("Watching: {path}"))
    }

    fn tool_stop_watch(&self) -> Result<String, String> {
        self.with_engine(|engine| engine.stop_watching())?;
        Ok("Watcher stopped".to_string())
    }

    fn tool_commit(&self, args: &Value) -> Result<String, String> {
        let desc = args
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("MCP commit")
            .to_string();
        let (snapshot_id, file_count) = self.with_engine(|engine| engine.create_snapshot(desc))?;
        Ok(format!(
            "Snapshot created: id={snapshot_id}, files={file_count}"
        ))
    }

    fn tool_rollback(&self, args: &Value) -> Result<String, String> {
        let snapshot_id = args
            .get("snapshot_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing snapshot_id".to_string())?;
        let result = self.with_engine(|engine| engine.rollback(snapshot_id))?;
        Ok(format!(
            "Rolled back snapshot {snapshot_id}: restored={}, removed={}, skipped={}",
            result.restored, result.removed, result.skipped
        ))
    }

    fn tool_snapshots(&self) -> Result<String, String> {
        let snapshots = self.with_engine(|engine| engine.list_snapshots())?;
        serde_json::to_string_pretty(&snapshots).map_err(|e| e.to_string())
    }

    fn tool_delete_snapshot(&self, args: &Value) -> Result<String, String> {
        let snapshot_id = args
            .get("snapshot_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing snapshot_id".to_string())?;
        self.with_engine(|engine| engine.delete_snapshot(snapshot_id))?;
        Ok(format!("Deleted snapshot {snapshot_id}"))
    }
}

fn tool_def(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

/// 入站 JSON-RPC 请求。
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    id: Option<Value>,
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
}

/// 出站 JSON-RPC 响应。
#[derive(Debug, serde::Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, serde::Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl JsonRpcResponse {
    fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Option<Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_response_shape() {
        let base_dir = tempfile::tempdir().unwrap().path().join(".openrollback");
        let engine = SnapshotEngine::new(&base_dir).unwrap();
        let service = McpService {
            engine: Mutex::new(engine),
            initialized: Mutex::new(true),
        };
        let resp = service.handle_tools_list(Some(json!(1)));
        let result = resp.result.unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 6);
        assert_eq!(tools[0]["name"], "openrollback_watch");
    }
}
