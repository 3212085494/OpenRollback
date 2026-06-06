//! 文件系统监控模块（基于 `notify` crate）。
//!
//! 监听指定目录，捕获创建/修改/删除事件，写入内存队列。
//! 对修改/删除事件会维护内容缓存，以便回滚时恢复被删文件。

use crate::error::{OpenRollbackError, Result};
use notify::event::{AccessKind, AccessMode, CreateKind, ModifyKind, RemoveKind};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use super::util::{normalize_fs_path, sha256_hex};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 文件变更动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAction {
    Create,
    Modify,
    Delete,
}

impl FileAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Modify => "modify",
            Self::Delete => "delete",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "create" => Some(Self::Create),
            "modify" => Some(Self::Modify),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

/// 内存队列中的文件变更事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChangeEvent {
    pub path: PathBuf,
    pub action: FileAction,
    /// 变更前文件的 SHA-256 哈希（十六进制）。
    pub original_hash: Option<String>,
    /// 变更前文件内容的暂存路径。
    pub staging_backup: Option<PathBuf>,
}

/// 路径内容缓存，用于在删除事件发生时保留可恢复副本。
#[derive(Debug, Clone)]
struct CachedFile {
    hash: String,
    staging_backup: PathBuf,
}

/// 预缓存时跳过的单文件大小上限（50 MB）。
const PREFETCH_MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;
/// 预缓存的最大文件数量。
const PREFETCH_MAX_FILES: usize = 10_000;

/// 预缓存时跳过的重目录（减少桌面等大目录下的无效扫描）。
const PREFETCH_SKIP_DIR_NAMES: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "__pycache__",
    ".openrollback",
    "$RECYCLE.BIN",
    "System Volume Information",
];

/// 预缓存结果统计。
#[derive(Debug, Clone, Copy, Default)]
pub struct PrefetchResult {
    pub cached: usize,
    pub total_candidates: usize,
    pub truncated: bool,
}

/// 文件监控服务。
pub struct FileWatcher {
    /// 保持 watcher 存活，停止监控时 drop 即可。
    _watcher: RecommendedWatcher,
    queue: Arc<Mutex<VecDeque<FileChangeEvent>>>,
    content_cache: Arc<Mutex<HashMap<PathBuf, CachedFile>>>,
    staging_dir: PathBuf,
}

impl FileWatcher {
    /// 创建仅含内存队列的监控器（不启动 notify，供单元测试使用）。
    #[cfg(test)]
    pub fn new_in_memory(staging_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(staging_dir).map_err(|e| {
            OpenRollbackError::Io(format!("failed to create watcher staging dir: {e}"))
        })?;

        // 监控一个隔离空目录，避免测试期间产生真实文件事件。
        let isolated = staging_dir.join("_notify_isolated");
        std::fs::create_dir_all(&isolated).map_err(|e| {
            OpenRollbackError::Io(format!("failed to create isolated watch dir: {e}"))
        })?;

        Self::watch(&[isolated], staging_dir)
    }

    /// 启动对指定路径列表的递归监控。
    pub fn watch(paths: &[PathBuf], staging_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(staging_dir).map_err(|e| {
            OpenRollbackError::Io(format!("failed to create watcher staging dir: {e}"))
        })?;

        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let content_cache = Arc::new(Mutex::new(HashMap::new()));
        let staging_dir_buf = staging_dir
            .canonicalize()
            .unwrap_or_else(|_| staging_dir.to_path_buf());

        let queue_for_callback = Arc::clone(&queue);
        let cache_for_callback = Arc::clone(&content_cache);
        let staging_for_callback = staging_dir_buf.clone();

        let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                Self::handle_notify_event(
                    event,
                    &queue_for_callback,
                    &cache_for_callback,
                    &staging_for_callback,
                );
            }
        })
        .map_err(|e| OpenRollbackError::Watcher(e.to_string()))?;

        let mut watcher = watcher;
        for path in paths {
            watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|e| OpenRollbackError::Watcher(e.to_string()))?;
        }

        // 给 notify 后台线程留出初始化时间。
        std::thread::sleep(Duration::from_millis(50));

        Ok(Self {
            _watcher: watcher,
            queue,
            content_cache,
            staging_dir: staging_dir_buf,
        })
    }

    /// 手动入队一条变更（用于测试或非 notify 来源的事件）。
    pub fn push_event(&self, event: FileChangeEvent) {
        if let Ok(mut queue) = self.queue.lock() {
            queue.push_back(event);
        }
    }

    /// 取出并清空当前内存队列中的所有事件。
    pub fn drain_queue(&self) -> Vec<FileChangeEvent> {
        let mut queue = self
            .queue
            .lock()
            .map_err(|_| ())
            .unwrap_or_else(|_| panic!("file watcher queue poisoned"));
        let drained: Vec<_> = queue.drain(..).collect();
        drained
    }

    /// 当前队列中待处理事件数量。
    pub fn pending_count(&self) -> usize {
        self.queue
            .lock()
            .map(|q| q.len())
            .unwrap_or(0)
    }

    /// 内容缓存中的文件数量（用于删除回滚）。
    pub fn cached_file_count(&self) -> usize {
        self.content_cache
            .lock()
            .map(|c| c.len())
            .unwrap_or(0)
    }

    /// 递归预缓存监控目录内已有文件（按修改时间优先），使「直接删除」也可回滚。
    pub fn prefetch_existing_files(&self, root: &Path) -> Result<PrefetchResult> {
        let root = normalize_fs_path(root);
        if !root.is_dir() {
            return Ok(PrefetchResult::default());
        }

        let mut candidates: Vec<(PathBuf, SystemTime, u8)> = Vec::new();
        self.collect_prefetch_candidates(&root, &root, &mut candidates)?;

        crate::debug_log!(
            "[openrollback] prefetch scan: {} (candidates={})",
            root.display(),
            candidates.len()
        );

        // 最近修改的文件优先；监控根目录下的文件额外加权。
        candidates.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.0.cmp(&b.0))
        });

        let total_candidates = candidates.len();
        let mut cached = 0usize;

        for (path, _, _) in candidates.into_iter().take(PREFETCH_MAX_FILES) {
            if Self::cache_file(&self.content_cache, &path, &self.staging_dir).is_some() {
                cached += 1;
            }
        }

        Ok(PrefetchResult {
            cached,
            total_candidates,
            truncated: total_candidates > PREFETCH_MAX_FILES,
        })
    }

    fn should_skip_prefetch_dir(name: &str) -> bool {
        PREFETCH_SKIP_DIR_NAMES
            .iter()
            .any(|skip| name.eq_ignore_ascii_case(skip))
    }

    fn collect_prefetch_candidates(
        &self,
        root: &Path,
        dir: &Path,
        out: &mut Vec<(PathBuf, SystemTime, u8)>,
    ) -> Result<()> {
        let entries = fs::read_dir(dir).map_err(|e| {
            OpenRollbackError::Io(format!("failed to read dir {}: {e}", dir.display()))
        })?;

        let root_depth = if dir == root { 1u8 } else { 0u8 };

        for entry in entries.flatten() {
            let path = normalize_fs_path(&entry.path());
            if path.starts_with(&self.staging_dir) {
                continue;
            }

            let file_type = entry.file_type().map_err(|e| {
                OpenRollbackError::Io(format!("failed to read file type {}: {e}", path.display()))
            })?;

            if file_type.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if Self::should_skip_prefetch_dir(&name) {
                    continue;
                }
                self.collect_prefetch_candidates(root, &path, out)?;
            } else if file_type.is_file() {
                let meta = path.metadata().map_err(|e| {
                    OpenRollbackError::Io(format!("failed to read metadata {}: {e}", path.display()))
                })?;
                if meta.len() > PREFETCH_MAX_FILE_BYTES {
                    continue;
                }
                let mtime = meta.modified().unwrap_or(UNIX_EPOCH);
                // 根目录文件优先级更高（桌面上的 PNG 等）。
                let priority = if path.parent().map(|p| p == root).unwrap_or(false) {
                    2
                } else {
                    root_depth
                };
                out.push((path, mtime, priority));
            }
        }

        Ok(())
    }

    /// 判断路径是否应纳入快照队列。
    fn should_track_path(path: &Path, action: &FileAction, staging_dir: &Path) -> bool {
        if path.starts_with(staging_dir) {
            return false;
        }
        if path.exists() && path.is_dir() {
            return false;
        }
        match action {
            FileAction::Delete => true,
            FileAction::Create => !path.exists() || path.is_file(),
            FileAction::Modify => path.is_file(),
        }
    }

    fn cache_file(
        content_cache: &Arc<Mutex<HashMap<PathBuf, CachedFile>>>,
        path: &Path,
        staging_dir: &Path,
    ) -> Option<CachedFile> {
        let normalized = normalize_fs_path(path);
        let (hash, staging) = match Self::backup_file_to_staging(&normalized, staging_dir) {
            Ok((Some(hash), Some(staging))) => (hash, staging),
            _ => return None,
        };
        let cached = CachedFile {
            hash,
            staging_backup: staging,
        };
        if let Ok(mut cache) = content_cache.lock() {
            cache.insert(normalized, cached.clone());
        }
        Some(cached)
    }

    fn peek_cached_file(
        content_cache: &Arc<Mutex<HashMap<PathBuf, CachedFile>>>,
        path: &Path,
    ) -> Option<CachedFile> {
        let normalized = normalize_fs_path(path);
        content_cache.lock().ok().and_then(|cache| {
            cache
                .get(&normalized)
                .or_else(|| cache.get(path))
                .cloned()
        })
    }

    /// 处理 notify 事件并写入内存队列。
    fn handle_notify_event(
        event: Event,
        queue: &Arc<Mutex<VecDeque<FileChangeEvent>>>,
        content_cache: &Arc<Mutex<HashMap<PathBuf, CachedFile>>>,
        staging_dir: &Path,
    ) {
        for path in event.paths {
            // 文件关闭时预缓存内容，便于后续删除事件能回滚恢复。
            if matches!(
                event.kind,
                EventKind::Access(AccessKind::Close(
                    AccessMode::Read | AccessMode::Write | AccessMode::Other | AccessMode::Any
                ))
            ) {
                let normalized = normalize_fs_path(&path);
                if normalized.is_file() {
                    let _ = Self::cache_file(content_cache, &normalized, staging_dir);
                }
                continue;
            }

            let action = match event.kind {
                EventKind::Create(CreateKind::File | CreateKind::Any) => FileAction::Create,
                EventKind::Modify(
                    ModifyKind::Data(_)
                    | ModifyKind::Metadata(_)
                    | ModifyKind::Name(_)
                    | ModifyKind::Any,
                ) => FileAction::Modify,
                EventKind::Remove(RemoveKind::File | RemoveKind::Any) => FileAction::Delete,
                _ => continue,
            };

            if !Self::should_track_path(&path, &action, staging_dir) {
                continue;
            }

            let normalized = normalize_fs_path(&path);

            let change = match action {
                FileAction::Create => {
                    let _ = Self::cache_file(content_cache, &normalized, staging_dir);
                    FileChangeEvent {
                        path: normalized,
                        action,
                        original_hash: None,
                        staging_backup: None,
                    }
                }
                FileAction::Modify => {
                    let cached = Self::cache_file(content_cache, &normalized, staging_dir);
                    let (hash, staging) = cached
                        .map(|c| (Some(c.hash), Some(c.staging_backup)))
                        .unwrap_or((None, None));
                    FileChangeEvent {
                        path: normalized,
                        action,
                        original_hash: hash,
                        staging_backup: staging,
                    }
                }
                FileAction::Delete => {
                    let cached = Self::peek_cached_file(content_cache, &normalized);
                    let (hash, staging) = if let Some(c) = cached {
                        (Some(c.hash), Some(c.staging_backup))
                    } else if normalized.exists() {
                        Self::backup_file_to_staging(&normalized, staging_dir)
                            .unwrap_or((None, None))
                    } else {
                        (None, None)
                    };
                    FileChangeEvent {
                        path: normalized,
                        action,
                        original_hash: hash,
                        staging_backup: staging,
                    }
                }
            };

            if let Ok(mut q) = queue.lock() {
                q.push_back(change);
            }
        }
    }

    /// 将文件复制到暂存目录并计算 SHA-256。
    pub fn backup_file_to_staging(
        path: &Path,
        staging_dir: &Path,
    ) -> std::result::Result<(Option<String>, Option<PathBuf>), OpenRollbackError> {
        if !path.exists() || !path.is_file() {
            return Ok((None, None));
        }

        let content = std::fs::read(path)
            .map_err(|e| OpenRollbackError::Io(format!("failed to read {}: {e}", path.display())))?;
        let hash = sha256_hex(&content);

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let staging_path = staging_dir.join(format!("{hash}_{file_name}"));

        std::fs::write(&staging_path, &content).map_err(|e| {
            OpenRollbackError::Io(format!(
                "failed to write staging backup {}: {e}",
                staging_path.display()
            ))
        })?;

        Ok((Some(hash), Some(staging_path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn push_and_drain_queue() {
        let dir = tempdir().unwrap();
        let staging = dir.path().join("staging");
        let watcher = FileWatcher::watch(&[dir.path().to_path_buf()], &staging).unwrap();

        watcher.push_event(FileChangeEvent {
            path: dir.path().join("a.txt"),
            action: FileAction::Create,
            original_hash: None,
            staging_backup: None,
        });

        assert_eq!(watcher.pending_count(), 1);
        let events = watcher.drain_queue();
        assert_eq!(events.len(), 1);
        assert_eq!(watcher.pending_count(), 0);
    }

    #[test]
    fn modify_event_creates_staging_backup() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("data.txt");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "version-1").unwrap();
        drop(file);

        let staging = dir.path().join("staging");
        let watcher = FileWatcher::watch(&[dir.path().join(".")], &staging).unwrap();

        // 模拟修改：先备份再入队
        let (hash, staging_path) =
            FileWatcher::backup_file_to_staging(&file_path, &staging).unwrap();
        watcher.push_event(FileChangeEvent {
            path: file_path.clone(),
            action: FileAction::Modify,
            original_hash: hash,
            staging_backup: staging_path.clone(),
        });

        fs::write(&file_path, "version-2").unwrap();
        let events = watcher.drain_queue();
        let modify = events
            .iter()
            .find(|e| e.path == file_path && e.action == FileAction::Modify)
            .expect("should contain a modify event for data.txt");
        assert!(modify.staging_backup.as_ref().unwrap().exists());
    }
}
