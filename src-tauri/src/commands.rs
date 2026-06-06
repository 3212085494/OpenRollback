//! Phase 3：Tauri 命令桥接层。
//!
//! 将前端 `invoke` 调用映射到核心 `SnapshotEngine` 操作。

use crate::app_state::AppState;
use crate::core::util::normalize_fs_path;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

/// 前端时间轴展示用的快照 DTO（含文件变更数）。
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotView {
    pub id: i64,
    pub timestamp: String,
    pub description: String,
    pub status: String,
    pub file_count: i64,
}

/// 应用健康检查。
#[tauri::command]
pub fn get_app_status() -> String {
    "OpenRollback backend ready".to_string()
}

/// 启动对指定目录的文件监控。
#[tauri::command]
pub async fn start_watching(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let engine = state.0.clone();
    let watch_path = PathBuf::from(path);
    tauri::async_runtime::spawn_blocking(move || {
        let engine = engine.lock().map_err(|_| "engine lock poisoned".to_string())?;
        engine
            .start_watching(&watch_path)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 停止文件监控。
#[tauri::command]
pub async fn stop_watching(state: State<'_, AppState>) -> Result<(), String> {
    let engine = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let engine = engine.lock().map_err(|_| "engine lock poisoned".to_string())?;
        engine.stop_watching().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 获取监控状态。
#[tauri::command]
pub async fn get_watching_status(state: State<'_, AppState>) -> Result<WatchingStatus, String> {
    let engine = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let engine = engine.lock().map_err(|_| "engine lock poisoned".to_string())?;
        Ok(WatchingStatus {
            is_watching: engine.is_watching(),
            paths: engine
                .watched_paths()
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            pending_changes: engine.pending_changes(),
            cached_files: engine.cached_files(),
            prefetch_total: engine
                .prefetch_info()
                .map(|p| p.total_candidates)
                .unwrap_or(0),
            prefetch_truncated: engine
                .prefetch_info()
                .is_some_and(|p| p.truncated),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 获取所有快照列表（含每条快照的文件变更数）。
#[tauri::command]
pub async fn get_snapshots(state: State<'_, AppState>) -> Result<Vec<SnapshotView>, String> {
    let engine = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let engine = engine.lock().map_err(|_| "engine lock poisoned".to_string())?;
        let snapshots = engine.list_snapshots().map_err(|e| e.to_string())?;
        let mut views = Vec::with_capacity(snapshots.len());
        for snap in snapshots {
            let file_count = engine
                .count_file_changes(snap.id)
                .map_err(|e| e.to_string())?;
            views.push(SnapshotView {
                id: snap.id,
                timestamp: snap.timestamp,
                description: snap.description,
                status: snap.status,
                file_count,
            });
        }
        Ok(views)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 创建快照结果。
#[derive(Debug, Clone, Serialize)]
pub struct CreateSnapshotResult {
    pub id: i64,
    pub file_count: i64,
}

/// 创建快照（提交当前变更队列）。
#[tauri::command]
pub async fn create_snapshot(
    state: State<'_, AppState>,
    description: String,
) -> Result<CreateSnapshotResult, String> {
    let engine = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let engine = engine.lock().map_err(|_| "engine lock poisoned".to_string())?;
        let (id, file_count) = engine
            .create_snapshot(description)
            .map_err(|e| e.to_string())?;
        Ok(CreateSnapshotResult { id, file_count })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 回滚到指定快照（撤销该快照的变更）。
#[tauri::command]
pub async fn trigger_rollback(
    state: State<'_, AppState>,
    snapshot_id: i64,
) -> Result<crate::core::snapshot::RollbackResult, String> {
    let engine = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let engine = engine.lock().map_err(|_| "engine lock poisoned".to_string())?;
        engine.rollback(snapshot_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 删除快照记录及备份（不修改磁盘上的实际文件）。
#[tauri::command]
pub async fn delete_snapshot(
    state: State<'_, AppState>,
    snapshot_id: i64,
) -> Result<(), String> {
    let engine = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let engine = engine.lock().map_err(|_| "engine lock poisoned".to_string())?;
        engine.delete_snapshot(snapshot_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 快照内单条文件变更（前端展示）。
#[derive(Debug, Clone, Serialize)]
pub struct FileChangeView {
    pub id: i64,
    pub path: String,
    pub action: String,
    pub original_hash: Option<String>,
    pub backup_path: Option<String>,
}

/// 系统快捷路径（桌面、文档等）。
#[derive(Debug, Clone, Serialize)]
pub struct QuickPath {
    pub label: String,
    pub path: String,
}

/// 获取快照包含的文件变更列表。
#[tauri::command]
pub async fn get_snapshot_files(
    state: State<'_, AppState>,
    snapshot_id: i64,
) -> Result<Vec<FileChangeView>, String> {
    let engine = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let engine = engine.lock().map_err(|_| "engine lock poisoned".to_string())?;
        let changes = engine
            .database()
            .get_file_changes_for_snapshot(snapshot_id)
            .map_err(|e| e.to_string())?;
        Ok(changes
            .into_iter()
            .map(|c| FileChangeView {
                id: c.id,
                path: c.path,
                action: c.action,
                original_hash: c.original_hash,
                backup_path: c.backup_path,
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 将监控路径规范为绝对路径（相对路径基于当前工作目录解析）。
#[tauri::command]
pub fn normalize_watch_path(path: String) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("监控路径不能为空".into());
    }

    let input = PathBuf::from(trimmed);
    let resolved = if input.is_absolute() {
        input
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(input)
    };

    let canonical = if resolved.exists() {
        resolved
            .canonicalize()
            .map_err(|e| format!("无法解析路径 {}: {e}", resolved.display()))?
    } else {
        resolved
    };

    Ok(normalize_fs_path(&canonical).to_string_lossy().into_owned())
}

/// 获取常用系统目录快捷路径。
#[tauri::command]
pub fn get_quick_paths() -> Vec<QuickPath> {
    let mut paths = Vec::new();

    if let Some(home) = dirs::home_dir() {
        paths.push(QuickPath {
            label: "用户目录".into(),
            path: home.to_string_lossy().into_owned(),
        });
    }
    if let Some(desktop) = dirs::desktop_dir() {
        paths.push(QuickPath {
            label: "桌面".into(),
            path: desktop.to_string_lossy().into_owned(),
        });
    }
    if let Some(documents) = dirs::document_dir() {
        paths.push(QuickPath {
            label: "文档".into(),
            path: documents.to_string_lossy().into_owned(),
        });
    }
    if let Some(downloads) = dirs::download_dir() {
        paths.push(QuickPath {
            label: "下载".into(),
            path: downloads.to_string_lossy().into_owned(),
        });
    }

    paths.retain(|p| Path::new(&p.path).exists());
    paths
}

/// 监控状态 DTO。
#[derive(serde::Serialize)]
pub struct WatchingStatus {
    pub is_watching: bool,
    pub paths: Vec<String>,
    pub pending_changes: usize,
    pub cached_files: usize,
    pub prefetch_total: usize,
    pub prefetch_truncated: bool,
}
