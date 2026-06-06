//! Tauri 应用状态：通过 `Mutex` 包装核心引擎，满足 `Send + Sync`。

use crate::core::SnapshotEngine;
use std::sync::{Arc, Mutex};

/// 全局应用状态，由 Tauri `State` 管理。
pub struct AppState(pub Arc<Mutex<SnapshotEngine>>);

impl AppState {
    /// 创建并初始化应用状态。
    pub fn new() -> Result<Self, crate::error::OpenRollbackError> {
        let base_dir = SnapshotEngine::default_base_dir()?;
        let engine = SnapshotEngine::new(&base_dir)?;
        Ok(Self(Arc::new(Mutex::new(engine))))
    }
}
