//! OpenRollback — AI Agent 系统级安全网与时间机器。
//!
//! Tauri 后端入口，协调文件监控、快照引擎、MCP 协议与 SQLite 元数据存储。

pub mod app_state;
pub mod commands;
pub mod core;
pub mod debug_log;
pub mod error;
pub mod mcp;

use anyhow::Context;
use app_state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(err) = run_app() {
        eprintln!("OpenRollback failed to start: {err:#}");
        std::process::exit(1);
    }
}

/// 应用主逻辑，使用 `anyhow` 统一错误传播。
fn run_app() -> anyhow::Result<()> {
    let state = AppState::new().context("failed to initialize app state")?;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_app_status,
            commands::start_watching,
            commands::stop_watching,
            commands::get_watching_status,
            commands::get_snapshots,
            commands::create_snapshot,
            commands::trigger_rollback,
            commands::delete_snapshot,
            commands::get_snapshot_files,
            commands::get_quick_paths,
            commands::normalize_watch_path,
        ])
        .run(tauri::generate_context!())
        .context("error while running tauri application")?;

    Ok(())
}
