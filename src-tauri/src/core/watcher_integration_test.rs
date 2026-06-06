//! Windows 上 notify 真实文件事件探测（手动运行：cargo test notify_captures_real_fs_events -- --nocapture）

#[cfg(test)]
mod real_notify_tests {
    use crate::core::watcher::FileWatcher;
    use std::fs;
    use std::io::Write;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn notify_captures_real_fs_events() {
        let dir = tempdir().unwrap();
        let watch_path = dir.path().to_path_buf();
        let staging = dir.path().join("staging");

        let watcher = FileWatcher::watch(&[watch_path.clone()], &staging).unwrap();

        // 新建文件
        let file_a = watch_path.join("test_a.txt");
        let mut f = fs::File::create(&file_a).unwrap();
        writeln!(f, "hello").unwrap();
        drop(f);

        thread::sleep(Duration::from_millis(300));

        // 修改已有文件
        fs::write(&file_a, "hello-modified").unwrap();
        thread::sleep(Duration::from_millis(300));

        // 再建一个
        fs::write(watch_path.join("test_b.txt"), "world").unwrap();
        thread::sleep(Duration::from_millis(300));

        let count = watcher.pending_count();
        let events = watcher.drain_queue();

        assert!(
            count > 0,
            "notify should capture at least one file event on this platform (pending={count}, drained={})",
            events.len()
        );
    }
}
