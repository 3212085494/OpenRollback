//! OpenRollback 端到端演示二进制。
//!
//! 模拟 AI Agent 文件操作 → 快照 → 回滚的完整流程，供 `scripts/demo.sh` 调用。

use openrollback_lib::core::snapshot::SnapshotEngine;
use openrollback_lib::core::watcher::{FileAction, FileChangeEvent, FileWatcher};
use std::fs;
use std::thread;
use std::time::Duration;

fn main() {
    if let Err(err) = run_demo() {
        eprintln!("❌ Demo failed: {err:#}");
        std::process::exit(1);
    }
    println!("✅ Demo passed — snapshot create & rollback verified");
}

fn run_demo() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join("openrollback_demo");
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    fs::create_dir_all(&root)?;

    let test_dir = root.join("test_dir");
    fs::create_dir_all(&test_dir)?;

    // 1. 准备测试目录
    let a_path = test_dir.join("a.txt");
    let b_path = test_dir.join("b.txt");
    fs::write(&a_path, "original-a")?;
    fs::write(&b_path, "original-b")?;
    println!("📁 Prepared test_dir with a.txt, b.txt");

    // 2. 启动监控
    let base_dir = root.join(".openrollback");
    let engine = SnapshotEngine::new(&base_dir)?;
    engine.start_watching(&test_dir)?;
    println!("👁  Watching: {}", test_dir.display());

    thread::sleep(Duration::from_millis(100));

    let staging = base_dir.join("staging");

    // 3. 模拟 AI 操作（删除 a、修改 b、创建 c）
    let (a_hash, a_backup) = FileWatcher::backup_file_to_staging(&a_path, &staging)?;
    fs::remove_file(&a_path)?;

    let (b_hash, b_backup) = FileWatcher::backup_file_to_staging(&b_path, &staging)?;
    fs::write(&b_path, "修改")?;

    let c_path = test_dir.join("c.txt");
    fs::write(&c_path, "new-file")?;

    engine.enqueue_change(FileChangeEvent {
        path: a_path.clone(),
        action: FileAction::Delete,
        original_hash: a_hash,
        staging_backup: a_backup,
    })?;
    engine.enqueue_change(FileChangeEvent {
        path: b_path.clone(),
        action: FileAction::Modify,
        original_hash: b_hash,
        staging_backup: b_backup,
    })?;
    engine.enqueue_change(FileChangeEvent {
        path: c_path.clone(),
        action: FileAction::Create,
        original_hash: None,
        staging_backup: None,
    })?;

    println!("🤖 Simulated AI ops: rm a.txt, modify b.txt, touch c.txt");

    // 4. 创建快照
    let (snapshot_id, file_count) = engine.create_snapshot("测试快照".to_string())?;
    println!("📸 Snapshot created: id={snapshot_id}, files={file_count}");

    let changes = engine
        .database()
        .get_file_changes_for_snapshot(snapshot_id)?;
    anyhow::ensure!(changes.len() == 3, "expected 3 changes, got {}", changes.len());

    anyhow::ensure!(!a_path.exists(), "a.txt should be absent before rollback");
    anyhow::ensure!(
        fs::read_to_string(&b_path)? == "修改",
        "b.txt content mismatch before rollback"
    );
    anyhow::ensure!(c_path.exists(), "c.txt should exist before rollback");

    // 5. 回滚验证
    engine.rollback(snapshot_id)?;
    println!("⏪ Rolled back snapshot #{snapshot_id}");

    anyhow::ensure!(a_path.exists(), "a.txt should be restored");
    anyhow::ensure!(
        fs::read_to_string(&a_path)? == "original-a",
        "a.txt content not restored"
    );
    anyhow::ensure!(
        fs::read_to_string(&b_path)? == "original-b",
        "b.txt content not restored"
    );
    anyhow::ensure!(!c_path.exists(), "c.txt should be removed");

    engine.stop_watching()?;
    Ok(())
}
