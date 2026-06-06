//! OpenRollback 核心引擎模块。
//!
//! 包含数据库、文件监控、快照与回滚三大子系统。

pub mod db;
pub mod snapshot;
pub mod util;
pub mod watcher;

#[cfg(test)]
mod e2e_watch_test;
#[cfg(test)]
mod watcher_integration_test;

pub use snapshot::SnapshotEngine;
pub use watcher::FileWatcher;
