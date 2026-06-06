//! SQLite 数据库管理模块。
//!
//! 负责 `openrollback.db` 的初始化、迁移，以及 snapshots / file_changes 表的 CRUD。

use crate::error::{OpenRollbackError, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 快照记录（API / 前端序列化）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id: i64,
    pub timestamp: String,
    pub description: String,
    pub status: String,
}

/// 文件变更记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChangeRecord {
    pub id: i64,
    pub snapshot_id: i64,
    pub path: String,
    pub action: String,
    pub original_hash: Option<String>,
    pub backup_path: Option<String>,
}

/// SQLite 数据库管理器。
pub struct Database {
    conn: Connection,
    db_path: PathBuf,
}

impl Database {
    /// 在指定数据目录下初始化 `openrollback.db` 并创建表结构。
    pub fn init(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir).map_err(|e| {
            OpenRollbackError::Io(format!("failed to create data directory: {e}"))
        })?;

        let db_path = data_dir.join("openrollback.db");
        let conn = Connection::open(&db_path)?;
        let db = Self { conn, db_path };
        db.migrate()?;
        Ok(db)
    }

    /// 返回数据库文件路径。
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// 执行 schema 迁移（幂等）。
    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                description TEXT NOT NULL,
                status TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS file_changes (
                id INTEGER PRIMARY KEY,
                snapshot_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                action TEXT NOT NULL,
                original_hash TEXT,
                backup_path TEXT,
                FOREIGN KEY (snapshot_id) REFERENCES snapshots(id)
            );
            ",
        )?;
        Ok(())
    }

    /// 插入一条快照记录，返回自增 ID。
    pub fn insert_snapshot(
        &self,
        timestamp: &str,
        description: &str,
        status: &str,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO snapshots (timestamp, description, status) VALUES (?1, ?2, ?3)",
            params![timestamp, description, status],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// 插入一条文件变更记录，返回自增 ID。
    pub fn insert_file_change(
        &self,
        snapshot_id: i64,
        path: &str,
        action: &str,
        original_hash: Option<&str>,
        backup_path: Option<&str>,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO file_changes (snapshot_id, path, action, original_hash, backup_path)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![snapshot_id, path, action, original_hash, backup_path],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// 按 ID 查询快照。
    pub fn get_snapshot(&self, snapshot_id: i64) -> Result<Option<SnapshotRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, description, status FROM snapshots WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![snapshot_id])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(SnapshotRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
            }));
        }
        Ok(None)
    }

    /// 查询指定快照下的所有文件变更，按 ID 升序。
    pub fn get_file_changes_for_snapshot(
        &self,
        snapshot_id: i64,
    ) -> Result<Vec<FileChangeRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, snapshot_id, path, action, original_hash, backup_path
             FROM file_changes
             WHERE snapshot_id = ?1
             ORDER BY id ASC",
        )?;

        let rows = stmt.query_map(params![snapshot_id], |row| {
            Ok(FileChangeRecord {
                id: row.get(0)?,
                snapshot_id: row.get(1)?,
                path: row.get(2)?,
                action: row.get(3)?,
                original_hash: row.get(4)?,
                backup_path: row.get(5)?,
            })
        })?;

        let mut changes = Vec::new();
        for row in rows {
            changes.push(row?);
        }
        Ok(changes)
    }

    /// 统计指定快照下的文件变更数量。
    pub fn count_file_changes(&self, snapshot_id: i64) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM file_changes WHERE snapshot_id = ?1",
            params![snapshot_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// 列出所有快照，按 ID 降序（最新在前）。
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, description, status
             FROM snapshots
             ORDER BY id DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SnapshotRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
            })
        })?;

        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(row?);
        }
        Ok(snapshots)
    }

    /// 更新快照状态（如回滚后标记为 `rolled_back`）。
    pub fn update_snapshot_status(&self, snapshot_id: i64, status: &str) -> Result<()> {
        let updated = self.conn.execute(
            "UPDATE snapshots SET status = ?1 WHERE id = ?2",
            params![status, snapshot_id],
        )?;
        if updated == 0 {
            return Err(OpenRollbackError::Snapshot(format!(
                "snapshot {snapshot_id} not found"
            )));
        }
        Ok(())
    }

    /// 删除快照及其所有文件变更记录（不触碰磁盘上的实际文件）。
    pub fn delete_snapshot(&self, snapshot_id: i64) -> Result<()> {
        if self.get_snapshot(snapshot_id)?.is_none() {
            return Err(OpenRollbackError::Snapshot(format!(
                "snapshot {snapshot_id} not found"
            )));
        }

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(OpenRollbackError::from)?;

        tx.execute(
            "DELETE FROM file_changes WHERE snapshot_id = ?1",
            params![snapshot_id],
        )?;
        let deleted = tx.execute("DELETE FROM snapshots WHERE id = ?1", params![snapshot_id])?;
        if deleted == 0 {
            return Err(OpenRollbackError::Snapshot(format!(
                "snapshot {snapshot_id} not found"
            )));
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_creates_database_and_tables() {
        let dir = tempdir().unwrap();
        let db = Database::init(dir.path()).unwrap();

        assert!(db.path().exists());
        assert_eq!(db.path().file_name().unwrap(), "openrollback.db");

        let snapshot_id = db
            .insert_snapshot("2026-06-06T00:00:00Z", "test snapshot", "active")
            .unwrap();
        assert_eq!(snapshot_id, 1);

        let change_id = db
            .insert_file_change(
                snapshot_id,
                "/tmp/example.txt",
                "modify",
                Some("abc123"),
                Some("/backups/1/example.txt"),
            )
            .unwrap();
        assert_eq!(change_id, 1);

        let snapshot = db.get_snapshot(snapshot_id).unwrap().unwrap();
        assert_eq!(snapshot.description, "test snapshot");

        let changes = db.get_file_changes_for_snapshot(snapshot_id).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "/tmp/example.txt");
    }

    #[test]
    fn delete_snapshot_removes_record_and_file_changes() {
        let dir = tempdir().unwrap();
        let db = Database::init(dir.path()).unwrap();

        let snapshot_id = db
            .insert_snapshot("2026-06-06T00:00:00Z", "to delete", "active")
            .unwrap();
        db.insert_file_change(snapshot_id, "/tmp/a.txt", "modify", None, None)
            .unwrap();

        db.delete_snapshot(snapshot_id).unwrap();

        assert!(db.get_snapshot(snapshot_id).unwrap().is_none());
        assert_eq!(
            db.get_file_changes_for_snapshot(snapshot_id).unwrap().len(),
            0
        );
    }
}
