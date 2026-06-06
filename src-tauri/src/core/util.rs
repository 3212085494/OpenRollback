//! 核心引擎通用工具函数。

use crate::error::OpenRollbackError;
use sha2::{Digest, Sha256};
use std::io;
use std::path::{Path, PathBuf};

/// 计算字节内容的 SHA-256 十六进制摘要。
pub fn sha256_hex(content: &[u8]) -> String {
    let digest = Sha256::digest(content);
    digest.iter().map(|byte| format!("{:02x}", byte)).collect()
}

/// 规范化文件系统路径（统一为无 `\\?\` 前缀的绝对路径）。
pub fn normalize_fs_path(path: &Path) -> PathBuf {
    let stripped = strip_extended_prefix(path);
    let resolved = if stripped.exists() {
        stripped.canonicalize().unwrap_or(stripped)
    } else {
        stripped
    };
    strip_extended_prefix(&resolved)
}

fn strip_extended_prefix(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

/// 将 std::io::Error 转为用户可读的中文 I/O 错误（含文件占用场景）。
pub fn io_err(op: &str, path: &Path, err: io::Error) -> OpenRollbackError {
    let hint = match err.kind() {
        io::ErrorKind::PermissionDenied => {
            "权限不足或文件正被其他程序占用，请关闭相关程序后重试"
        }
        io::ErrorKind::NotFound => "路径不存在",
        io::ErrorKind::AlreadyExists => "目标已存在",
        io::ErrorKind::AddrInUse | io::ErrorKind::AddrNotAvailable => "资源被占用",
        _ if err.raw_os_error() == Some(32) => {
            // Windows ERROR_SHARING_VIOLATION
            "文件正被其他程序占用，请关闭后重试"
        }
        _ if err.raw_os_error() == Some(33) => {
            // Windows ERROR_LOCK_VIOLATION
            "文件被锁定，请关闭占用程序后重试"
        }
        _ => "磁盘读写失败",
    };
    OpenRollbackError::Io(format!(
        "{op}「{}」: {hint} ({err})",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_err_maps_permission_denied_to_chinese_hint() {
        let err = io_err(
            "恢复文件",
            Path::new("test.txt"),
            io::Error::new(io::ErrorKind::PermissionDenied, "access denied"),
        );
        let msg = err.to_string();
        assert!(msg.contains("权限不足") || msg.contains("占用"), "got: {msg}");
    }

    #[test]
    fn io_err_maps_windows_sharing_violation() {
        let err = io_err(
            "恢复文件",
            Path::new("test.txt"),
            io::Error::from_raw_os_error(32),
        );
        let msg = err.to_string();
        assert!(msg.contains("占用"), "got: {msg}");
    }
}
