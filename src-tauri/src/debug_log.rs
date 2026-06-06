//! 仅 Debug 构建输出的诊断日志；Release 构建保持控制台干净。

#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)*);
        }
    };
}
