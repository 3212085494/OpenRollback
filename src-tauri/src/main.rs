//! OpenRollback 桌面应用入口。
//!
//! Phase 3：Tauri 命令在 `commands.rs` 中定义并通过 `lib.rs` 注册；
//! MCP stdio 服务由独立二进制 `openrollback-mcp` 提供。

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    openrollback_lib::run();
}
