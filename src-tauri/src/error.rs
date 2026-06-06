//! OpenRollback 统一错误类型定义。
//!
//! 领域错误使用 `thiserror` 派生，应用层通过 `anyhow::Result` 传播。

use thiserror::Error;

/// OpenRollback 核心领域错误。
#[derive(Debug, Error)]
pub enum OpenRollbackError {
    /// 数据库操作失败。
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// 文件 I/O 操作失败。
    #[error("io error: {0}")]
    Io(String),

    /// 文件系统监控错误。
    #[error("file watcher error: {0}")]
    Watcher(String),

    /// 快照或回滚操作失败。
    #[error("snapshot error: {0}")]
    Snapshot(String),
}

/// 领域层结果别名。
pub type Result<T> = std::result::Result<T, OpenRollbackError>;
