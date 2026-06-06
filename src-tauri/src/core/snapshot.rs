//! 快照与回滚引擎模块。
//!
//! - `create_snapshot`：将监控队列中的变更持久化到 DB，并备份文件到 `~/.openrollback/backups/`
//! - `rollback`：根据 DB 记录逆向恢复文件状态

use super::db::{Database, SnapshotRecord};
use super::watcher::{FileAction, FileChangeEvent, FileWatcher, PrefetchResult};
use serde::Serialize;

/// 回滚执行结果（返回给前端展示）。
#[derive(Debug, Clone, Serialize)]
pub struct RollbackResult {
    pub restored: usize,
    pub removed: usize,
    pub skipped: usize,
    pub details: Vec<String>,
}
use crate::error::{OpenRollbackError, Result};
use chrono::Utc;
use super::util::{io_err, normalize_fs_path, sha256_hex};
use crate::debug_log;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 同一文件的重复 notify 事件合并为一条（优先保留有备份的记录）。
fn coalesce_events(events: Vec<FileChangeEvent>) -> Vec<FileChangeEvent> {
    let mut by_path: HashMap<PathBuf, FileChangeEvent> = HashMap::new();
    for ev in events {
        let key = normalize_fs_path(&ev.path);
        by_path
            .entry(key)
            .and_modify(|existing| {
                if event_quality(&ev) > event_quality(existing) {
                    *existing = ev.clone();
                }
            })
            .or_insert(ev);
    }
    let mut out: Vec<_> = by_path.into_values().collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn event_quality(ev: &FileChangeEvent) -> u8 {
    let mut q = match ev.action {
        FileAction::Create => 1,
        FileAction::Modify => 2,
        FileAction::Delete => 3,
    };
    if ev.staging_backup.is_some() {
        q += 4;
    }
    if ev.original_hash.is_some() {
        q += 2;
    }
    q
}

/// 快照引擎，协调数据库、文件监控与备份目录。
pub struct SnapshotEngine {
    db: Database,
    watcher: Mutex<Option<FileWatcher>>,
    backups_dir: PathBuf,
    staging_dir: PathBuf,
    watched_paths: Mutex<Vec<PathBuf>>,
    prefetch_info: Mutex<Option<PrefetchResult>>,
}

impl SnapshotEngine {
    /// 创建快照引擎（不自动启动文件监控，由 `start_watching` 控制）。
    pub fn new(base_dir: &Path) -> Result<Self> {
        let backups_dir = base_dir.join("backups");
        let staging_dir = base_dir.join("staging");
        fs::create_dir_all(&backups_dir).map_err(|e| {
            OpenRollbackError::Io(format!("failed to create backups directory: {e}"))
        })?;
        fs::create_dir_all(&staging_dir).map_err(|e| {
            OpenRollbackError::Io(format!("failed to create staging directory: {e}"))
        })?;

        let db = Database::init(base_dir)?;
        Ok(Self {
            db,
            watcher: Mutex::new(None),
            backups_dir,
            staging_dir,
            watched_paths: Mutex::new(Vec::new()),
            prefetch_info: Mutex::new(None),
        })
    }

    /// 使用已有监控器创建引擎（单元测试专用）。
    #[cfg(test)]
    pub fn new_with_watcher(base_dir: &Path, watcher: FileWatcher) -> Result<Self> {
        let engine = Self::new(base_dir)?;
        *engine
            .watcher
            .lock()
            .map_err(|_| OpenRollbackError::Watcher("watcher lock poisoned".into()))? =
            Some(watcher);
        Ok(engine)
    }

    /// 返回默认的 OpenRollback 数据目录（`~/.openrollback`）。
    pub fn default_base_dir() -> Result<PathBuf> {
        dirs::home_dir()
            .map(|h| h.join(".openrollback"))
            .ok_or_else(|| OpenRollbackError::Snapshot("cannot resolve home directory".into()))
    }

    /// 启动对指定目录的递归监控。
    pub fn start_watching(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(OpenRollbackError::Watcher(format!(
                "watch path does not exist: {}",
                path.display()
            )));
        }

        let path_buf = path
            .canonicalize()
            .map_err(|e| OpenRollbackError::Watcher(format!("cannot resolve watch path: {e}")))?;

        // 重新 WATCH 时替换旧监控器，避免路径列表与真实监控状态不一致。
        *self
            .watcher
            .lock()
            .map_err(|_| OpenRollbackError::Watcher("watcher lock poisoned".into()))? = None;

        let watcher = FileWatcher::watch(std::slice::from_ref(&path_buf), &self.staging_dir)?;

        let prefetch = watcher.prefetch_existing_files(&path_buf)?;

        *self
            .watcher
            .lock()
            .map_err(|_| OpenRollbackError::Watcher("watcher lock poisoned".into()))? =
            Some(watcher);

        *self
            .watched_paths
            .lock()
            .map_err(|_| OpenRollbackError::Watcher("paths lock poisoned".into()))? =
            vec![path_buf];

        *self
            .prefetch_info
            .lock()
            .map_err(|_| OpenRollbackError::Watcher("prefetch lock poisoned".into()))? =
            Some(prefetch);

        debug_log!(
            "[openrollback] prefetched {}/{} files (truncated={})",
            prefetch.cached,
            prefetch.total_candidates,
            prefetch.truncated
        );
        Ok(())
    }

    /// 预缓存统计。
    pub fn prefetch_info(&self) -> Option<PrefetchResult> {
        self.prefetch_info.lock().ok().and_then(|g| *g)
    }

    /// 上次启动监控时预缓存的文件数。
    pub fn cached_files(&self) -> usize {
        self.watcher
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|w| w.cached_file_count()))
            .unwrap_or(0)
    }

    /// 停止文件监控。
    pub fn stop_watching(&self) -> Result<()> {
        *self
            .watcher
            .lock()
            .map_err(|_| OpenRollbackError::Watcher("watcher lock poisoned".into()))? = None;
        *self
            .watched_paths
            .lock()
            .map_err(|_| OpenRollbackError::Watcher("paths lock poisoned".into()))? = Vec::new();
        *self
            .prefetch_info
            .lock()
            .map_err(|_| OpenRollbackError::Watcher("prefetch lock poisoned".into()))? = None;
        Ok(())
    }

    /// 是否正在监控。
    pub fn is_watching(&self) -> bool {
        self.watcher
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    /// 当前监控路径列表。
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths
            .lock()
            .map(|p| p.clone())
            .unwrap_or_default()
    }

    /// 待处理变更数量。
    pub fn pending_changes(&self) -> usize {
        self.watcher
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|w| w.pending_count()))
            .unwrap_or(0)
    }

    /// 列出所有快照。
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotRecord>> {
        self.db.list_snapshots()
    }

    /// 统计快照下的文件变更数。
    pub fn count_file_changes(&self, snapshot_id: i64) -> Result<i64> {
        self.db.count_file_changes(snapshot_id)
    }

    /// 创建快照：持久化队列中的变更事件，并将相关文件备份到磁盘。
    pub fn create_snapshot(&self, desc: String) -> Result<(i64, i64)> {
        if !self.is_watching() {
            return Err(OpenRollbackError::Snapshot(
                "文件监控未启动，请先点击 WATCH 后再创建快照".into(),
            ));
        }

        let raw_events = self
            .watcher
            .lock()
            .map_err(|_| OpenRollbackError::Watcher("watcher lock poisoned".into()))?
            .as_ref()
            .map(|w| w.drain_queue())
            .unwrap_or_default();
        let events = coalesce_events(raw_events);

        debug_log!(
            "[openrollback] create_snapshot: paths={:?}, pending={}",
            self.watched_paths(),
            events.len()
        );

        if events.is_empty() {
            return Err(OpenRollbackError::Snapshot(
                "监控目录中暂无待提交的文件变更，请先在目录内新建、修改或删除文件".into(),
            ));
        }

        let timestamp = Utc::now().to_rfc3339();
        let snapshot_id = self.db.insert_snapshot(&timestamp, &desc, "active")?;

        let snapshot_backup_dir = self.backups_dir.join(snapshot_id.to_string());
        fs::create_dir_all(&snapshot_backup_dir).map_err(|e| {
            OpenRollbackError::Io(format!(
                "failed to create snapshot backup dir {}: {e}",
                snapshot_backup_dir.display()
            ))
        })?;

        let mut file_count = 0_i64;
        let mut skipped = 0_i64;
        for event in events {
            match self.persist_file_change(snapshot_id, &snapshot_backup_dir, event) {
                Ok(()) => file_count += 1,
                Err(e) => {
                    debug_log!("[openrollback] skipped unrecoverable change: {e}");
                    skipped += 1;
                }
            }
        }

        if file_count == 0 {
            let _ = self.db.delete_snapshot(snapshot_id);
            let _ = fs::remove_dir_all(&snapshot_backup_dir);
            let hint = if self.prefetch_info().is_some_and(|p| p.truncated) {
                "监控目录文件过多，部分文件未预缓存；建议监控更小的子文件夹"
            } else {
                "请确认在点击 WATCH 之后再删除文件"
            };
            return Err(OpenRollbackError::Snapshot(format!(
                "所有 {skipped} 个变更均无法备份，无法创建可回滚快照（{hint}）"
            )));
        }

        Ok((snapshot_id, file_count))
    }

    /// 删除快照记录及备份文件（不修改磁盘上的实际文件）。
    pub fn delete_snapshot(&self, snapshot_id: i64) -> Result<()> {
        self.db
            .get_snapshot(snapshot_id)?
            .ok_or_else(|| OpenRollbackError::Snapshot(format!("snapshot {snapshot_id} not found")))?;

        self.db.delete_snapshot(snapshot_id)?;

        let backup_dir = self.backups_dir.join(snapshot_id.to_string());
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir).map_err(|e| {
                OpenRollbackError::Io(format!(
                    "failed to remove backup dir {}: {e}",
                    backup_dir.display()
                ))
            })?;
        }

        Ok(())
    }

    /// 回滚到指定快照之前的状态（撤销该快照记录的所有变更）。
    pub fn rollback(&self, snapshot_id: i64) -> Result<RollbackResult> {
        let snapshot = self
            .db
            .get_snapshot(snapshot_id)?
            .ok_or_else(|| OpenRollbackError::Snapshot(format!("snapshot {snapshot_id} not found")))?;

        if snapshot.status == "rolled_back" {
            return Err(OpenRollbackError::Snapshot(format!(
                "snapshot {snapshot_id} already rolled back"
            )));
        }

        let changes = self.db.get_file_changes_for_snapshot(snapshot_id)?;
        if changes.is_empty() {
            return Err(OpenRollbackError::Snapshot(format!(
                "快照 #{snapshot_id} 无文件变更记录，无法回滚"
            )));
        }

        let mut restored = 0usize;
        let mut removed = 0usize;
        let mut skipped = 0usize;
        let mut details = Vec::new();

        for change in changes.iter().rev() {
            let path = normalize_fs_path(Path::new(&change.path));
            let path_display = path.display().to_string();
            let action = FileAction::parse(&change.action).ok_or_else(|| {
                OpenRollbackError::Snapshot(format!("unknown action: {}", change.action))
            })?;

            match action {
                FileAction::Create => {
                    if path.exists() {
                        fs::remove_file(&path).map_err(|e| io_err("撤销新建文件", &path, e))?;
                        removed += 1;
                        details.push(format!("已撤销新建: {path_display}"));
                    } else {
                        skipped += 1;
                        details.push(format!("新建文件已不存在（跳过）: {path_display}"));
                    }
                }
                FileAction::Modify | FileAction::Delete => {
                    let Some(backup) = change.backup_path.as_ref() else {
                        skipped += 1;
                        details.push(format!("无法恢复（无备份）: {path_display}"));
                        continue;
                    };
                    let backup_path = normalize_fs_path(Path::new(backup));
                    if !backup_path.exists() {
                        skipped += 1;
                        details.push(format!(
                            "无法恢复（备份文件丢失）: {path_display}"
                        ));
                        continue;
                    }
                    Self::restore_file_from_backup(&path, &backup_path)?;
                    if !path.exists() {
                        return Err(OpenRollbackError::Snapshot(format!(
                            "恢复后文件仍不存在: {path_display}"
                        )));
                    }
                    restored += 1;
                    let verb = if action == FileAction::Delete {
                        "已恢复删除"
                    } else {
                        "已恢复修改"
                    };
                    details.push(format!("{verb}: {path_display}"));
                }
            }
        }

        let need_restore = changes.iter().any(|c| {
            matches!(c.action.as_str(), "modify" | "delete")
        });

        if need_restore && restored == 0 {
            return Err(OpenRollbackError::Snapshot(format!(
                "未能恢复任何文件。{}",
                details.join("；")
            )));
        }

        if restored + removed == 0 {
            return Err(OpenRollbackError::Snapshot(format!(
                "回滚未产生任何效果。{}",
                details.join("；")
            )));
        }

        self.db
            .update_snapshot_status(snapshot_id, "rolled_back")?;
        Ok(RollbackResult {
            restored,
            removed,
            skipped,
            details,
        })
    }

    /// 手动入队监控事件（测试 / 演示用）。
    pub fn enqueue_change(&self, event: FileChangeEvent) -> Result<()> {
        let guard = self
            .watcher
            .lock()
            .map_err(|_| OpenRollbackError::Watcher("watcher lock poisoned".into()))?;
        let watcher = guard
            .as_ref()
            .ok_or_else(|| OpenRollbackError::Watcher("watcher not started".into()))?;
        watcher.push_event(event);
        Ok(())
    }

    #[cfg(test)]
    fn push_watch_event(&self, event: FileChangeEvent) {
        self.enqueue_change(event).ok();
    }

    /// 获取数据库引用。
    pub fn database(&self) -> &Database {
        &self.db
    }

    fn persist_file_change(
        &self,
        snapshot_id: i64,
        snapshot_backup_dir: &Path,
        event: FileChangeEvent,
    ) -> Result<()> {
        let normalized_path = normalize_fs_path(&event.path);
        let path_str = normalized_path.to_string_lossy().into_owned();
        let backup_path = match event.staging_backup {
            Some(staging) => {
                let file_name = staging
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "backup.bin".to_string());
                let dest = snapshot_backup_dir.join(&file_name);
                if staging != dest {
                    if dest.exists() {
                        fs::remove_file(&dest).ok();
                    }
                    fs::rename(&staging, &dest).or_else(|_| {
                        fs::copy(&staging, &dest).map(|_| ()).map_err(|e| {
                            OpenRollbackError::Io(format!(
                                "failed to copy backup {} -> {}: {e}",
                                staging.display(),
                                dest.display()
                            ))
                        })
                    })?;
                    let _ = fs::remove_file(&staging);
                }
                Some(dest)
            }
            None => {
                if event.action == FileAction::Create && normalized_path.exists() {
                    Some(self.backup_existing_file(snapshot_backup_dir, &normalized_path)?)
                } else {
                    None
                }
            }
        };

        if matches!(event.action, FileAction::Modify | FileAction::Delete) && backup_path.is_none()
        {
            return Err(OpenRollbackError::Snapshot(format!(
                "无法为 {} 创建可回滚快照：缺少文件备份（删除前请先打开或修改该文件）",
                normalized_path.display()
            )));
        }

        let backup_path_str = backup_path.as_ref().map(|p| p.to_string_lossy().into_owned());

        self.db.insert_file_change(
            snapshot_id,
            &path_str,
            event.action.as_str(),
            event.original_hash.as_deref(),
            backup_path_str.as_deref(),
        )?;

        Ok(())
    }

    fn backup_existing_file(&self, snapshot_backup_dir: &Path, path: &Path) -> Result<PathBuf> {
        let content = fs::read(path).map_err(|e| {
            OpenRollbackError::Io(format!("failed to read {}: {e}", path.display()))
        })?;
        let hash = sha256_hex(&content);
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let dest = snapshot_backup_dir.join(format!("{hash}_{file_name}"));
        fs::write(&dest, content).map_err(|e| {
            OpenRollbackError::Io(format!("failed to write backup {}: {e}", dest.display()))
        })?;
        Ok(dest)
    }

    fn restore_file_from_backup(target: &Path, backup: &Path) -> Result<()> {
        if !backup.exists() {
            return Err(OpenRollbackError::Snapshot(format!(
                "backup file not found: {}",
                backup.display()
            )));
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| io_err("创建目标目录", parent, e))?;
        }

        if target.exists() {
            fs::remove_file(target).map_err(|e| io_err("替换已有文件", target, e))?;
        }

        fs::copy(backup, target).map_err(|e| io_err("恢复文件", target, e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::watcher::{FileAction, FileChangeEvent, FileWatcher};
    use std::io::Write;
    use tempfile::tempdir;

    fn setup_engine() -> (tempfile::TempDir, SnapshotEngine) {
        let temp = tempdir().unwrap();
        let base_dir = temp.path().join(".openrollback");
        let staging = base_dir.join("staging");
        let watcher = FileWatcher::new_in_memory(&staging).unwrap();
        let engine = SnapshotEngine::new_with_watcher(&base_dir, watcher).unwrap();
        (temp, engine)
    }

    #[test]
    fn create_snapshot_persists_changes_and_backups() {
        let work_dir = tempdir().unwrap();
        let file_path = work_dir.path().join("doc.txt");

        let (_temp, engine) = setup_engine();

        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "hello").unwrap();
        drop(file);

        let staging = _temp.path().join(".openrollback/staging");
        let (hash, staging_path) =
            FileWatcher::backup_file_to_staging(&file_path, &staging).unwrap();

        engine.push_watch_event(FileChangeEvent {
            path: file_path.clone(),
            action: FileAction::Modify,
            original_hash: hash,
            staging_backup: staging_path,
        });

        let (snapshot_id, _) = engine
            .create_snapshot("initial change".to_string())
            .unwrap();
        assert_eq!(snapshot_id, 1);

        let changes = engine
            .database()
            .get_file_changes_for_snapshot(snapshot_id)
            .unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].action, "modify");
        assert!(changes[0]
            .backup_path
            .as_ref()
            .unwrap()
            .contains(&snapshot_id.to_string()));
    }

    #[test]
    fn rollback_restores_modified_file() {
        let work_dir = tempdir().unwrap();
        let file_path = work_dir.path().join("data.txt");

        let (_temp, engine) = setup_engine();

        fs::write(&file_path, "version-1").unwrap();

        let staging_dir = _temp.path().join(".openrollback/staging");
        let (hash, staging_path) =
            FileWatcher::backup_file_to_staging(&file_path, &staging_dir).unwrap();

        engine.push_watch_event(FileChangeEvent {
            path: file_path.clone(),
            action: FileAction::Modify,
            original_hash: hash,
            staging_backup: staging_path,
        });

        let (snapshot_id, _) = engine.create_snapshot("before edit".to_string()).unwrap();

        fs::write(&file_path, "version-2").unwrap();
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "version-2");

        engine.rollback(snapshot_id).unwrap();
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "version-1");

        let snapshot = engine.database().get_snapshot(snapshot_id).unwrap().unwrap();
        assert_eq!(snapshot.status, "rolled_back");
    }

    #[test]
    fn rollback_restores_deleted_file_when_backup_exists() {
        let work_dir = tempdir().unwrap();
        let file_path = work_dir.path().join("photo.png");

        let (_temp, engine) = setup_engine();

        fs::write(&file_path, "image-bytes").unwrap();

        let staging = _temp.path().join(".openrollback/staging");
        let (hash, staging_path) =
            FileWatcher::backup_file_to_staging(&file_path, &staging).unwrap();

        engine.push_watch_event(FileChangeEvent {
            path: file_path.clone(),
            action: FileAction::Modify,
            original_hash: hash,
            staging_backup: staging_path,
        });

        let snapshot_id = engine.create_snapshot("deleted file".to_string()).unwrap().0;
        fs::remove_file(&file_path).unwrap();
        assert!(!file_path.exists());

        engine.rollback(snapshot_id).unwrap();
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "image-bytes");
    }

    #[test]
    fn rollback_removes_created_file() {
        let work_dir = tempdir().unwrap();
        let file_path = work_dir.path().join("new.txt");

        let (_temp, engine) = setup_engine();

        fs::write(&file_path, "brand new").unwrap();

        engine.push_watch_event(FileChangeEvent {
            path: file_path.clone(),
            action: FileAction::Create,
            original_hash: None,
            staging_backup: None,
        });

        let (snapshot_id, _) = engine.create_snapshot("file created".to_string()).unwrap();
        assert!(file_path.exists());

        engine.rollback(snapshot_id).unwrap();
        assert!(!file_path.exists());
    }

    #[test]
    fn delete_snapshot_removes_db_and_backup_dir() {
        let work_dir = tempdir().unwrap();
        let file_path = work_dir.path().join("doc.txt");

        let (_temp, engine) = setup_engine();

        fs::write(&file_path, "hello").unwrap();

        let staging = _temp.path().join(".openrollback/staging");
        let (hash, staging_path) =
            FileWatcher::backup_file_to_staging(&file_path, &staging).unwrap();

        engine.push_watch_event(FileChangeEvent {
            path: file_path.clone(),
            action: FileAction::Modify,
            original_hash: hash,
            staging_backup: staging_path,
        });

        let (snapshot_id, _) = engine.create_snapshot("delete me".to_string()).unwrap();
        let backup_dir = _temp
            .path()
            .join(".openrollback/backups")
            .join(snapshot_id.to_string());
        assert!(backup_dir.exists());

        engine.delete_snapshot(snapshot_id).unwrap();

        assert!(engine.database().get_snapshot(snapshot_id).unwrap().is_none());
        assert!(!backup_dir.exists());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "hello");
    }

    #[test]
    fn delete_rollback_restores_chinese_filename() {
        let work_dir = tempdir().unwrap();
        let sub = work_dir.path().join("win831");
        fs::create_dir_all(&sub).unwrap();
        let file_path = sub.join("新建文本文档.txt");
        fs::write(&file_path, "content-123").unwrap();

        let base_dir = work_dir.path().join(".openrollback");
        let engine = SnapshotEngine::new(&base_dir).unwrap();
        engine.start_watching(work_dir.path()).unwrap();

        fs::remove_file(&file_path).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(400));

        let (snapshot_id, _) = engine.create_snapshot("delete chinese".into()).unwrap();
        assert!(!file_path.exists());

        let result = engine.rollback(snapshot_id).unwrap();
        assert_eq!(result.restored, 1);
        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "content-123");
    }

    #[test]
    fn prefetch_enables_delete_rollback() {
        let work_dir = tempdir().unwrap();
        let file_path = work_dir.path().join("screenshot.png");
        fs::write(&file_path, "png-bytes").unwrap();

        let base_dir = work_dir.path().join(".openrollback");
        let engine = SnapshotEngine::new(&base_dir).unwrap();
        engine.start_watching(work_dir.path()).unwrap();
        assert!(
            engine.cached_files() >= 1,
            "prefetch should cache existing files"
        );

        fs::remove_file(&file_path).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(400));

        let (snapshot_id, file_count) = engine.create_snapshot("delete png".into()).unwrap();
        assert!(file_count >= 1, "delete event should be captured");
        assert!(!file_path.exists());

        engine.rollback(snapshot_id).unwrap();
        assert!(file_path.exists());
        assert_eq!(fs::read(&file_path).unwrap(), b"png-bytes");
    }

    #[test]
    fn create_snapshot_rejects_empty_folder_without_changes() {
        let work_dir = tempdir().unwrap();
        let base_dir = work_dir.path().join(".openrollback");
        let engine = SnapshotEngine::new(&base_dir).unwrap();
        engine.start_watching(work_dir.path()).unwrap();

        let err = engine.create_snapshot("empty folder".into()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("暂无待提交"),
            "expected empty-queue error, got: {msg}"
        );
    }

    #[test]
    #[cfg(windows)]
    fn rollback_error_when_target_file_locked() {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;

        let work_dir = tempdir().unwrap();
        let file_path = work_dir.path().join("locked.txt");

        let (_temp, engine) = setup_engine();

        fs::write(&file_path, "version-1").unwrap();

        let staging_dir = _temp.path().join(".openrollback/staging");
        let (hash, staging_path) =
            FileWatcher::backup_file_to_staging(&file_path, &staging_dir).unwrap();

        engine.push_watch_event(FileChangeEvent {
            path: file_path.clone(),
            action: FileAction::Modify,
            original_hash: hash,
            staging_backup: staging_path,
        });

        let (snapshot_id, _) = engine.create_snapshot("before lock".to_string()).unwrap();
        fs::write(&file_path, "version-2").unwrap();

        const FILE_SHARE_NONE: u32 = 0;
        let _lock = OpenOptions::new()
            .write(true)
            .share_mode(FILE_SHARE_NONE)
            .open(&file_path)
            .expect("open file for exclusive lock");

        let err = engine.rollback(snapshot_id).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("占用") || msg.contains("权限") || msg.contains("Permission"),
            "expected file-in-use hint, got: {msg}"
        );
    }

    #[test]
    fn start_and_stop_watching() {
        let temp = tempdir().unwrap();
        let base_dir = temp.path().join(".openrollback");
        let engine = SnapshotEngine::new(&base_dir).unwrap();

        assert!(!engine.is_watching());
        engine.start_watching(temp.path()).unwrap();
        assert!(engine.is_watching());
        engine.stop_watching().unwrap();
        assert!(!engine.is_watching());
    }
}
