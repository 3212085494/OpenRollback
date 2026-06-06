//! 端到端：模拟用户 WATCH → 改文件 → 创建快照

#[cfg(test)]
mod e2e_tests {
    use crate::core::SnapshotEngine;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn watch_modify_snapshot_has_files() {
        let work = tempdir().unwrap();
        let base = work.path().join(".openrollback");
        let watch_dir = work.path().join("project");
        fs::create_dir_all(&watch_dir).unwrap();

        let engine = SnapshotEngine::new(&base).unwrap();
        engine.start_watching(&watch_dir).unwrap();

        assert!(engine.is_watching());
        assert_eq!(engine.pending_changes(), 0);

        fs::write(watch_dir.join("note.txt"), "first version").unwrap();
        thread::sleep(Duration::from_millis(400));

        let pending = engine.pending_changes();
        assert!(
            pending > 0,
            "expected pending changes after file write, got {pending}"
        );

        let (snapshot_id, persisted) = engine.create_snapshot("e2e test".into()).unwrap();
        let count = engine.count_file_changes(snapshot_id).unwrap();
        assert_eq!(count, persisted);
        assert!(
            count > 0,
            "snapshot #{snapshot_id} should contain file changes, got {count}"
        );
    }
}
