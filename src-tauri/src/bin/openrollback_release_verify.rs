//! 发布前验证：中文/空格路径冒烟 + 破坏性（Sad Path）场景。
//!
//! 由 `scripts/release_verify.ps1` 调用。

use openrollback_lib::core::snapshot::SnapshotEngine;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

const CHINESE_DIR: &str = "我的测试项目 🚀";
const TEST_FILE: &str = "测试文档.txt";
const TEST_CONTENT: &str = "OpenRollback 中文路径冒烟测试内容";

fn section(title: &str) {
    println!("\n━━ {title} ━━");
}

fn ok(msg: &str) {
    println!("  ✅ {msg}");
}

fn run() -> anyhow::Result<()> {
    section("5. 中文/空格/Emoji 路径冒烟测试");
    smoke_chinese_space_path()?;

    section("6a. Sad Path — 空文件夹创建快照");
    sad_empty_folder_snapshot()?;

    section("6b. Sad Path — 未启动监控时创建快照");
    sad_snapshot_without_watch()?;

    section("6c. Sad Path — 无权限/受保护系统路径");
    sad_protected_system_path()?;

    println!("\n🎉 发布验证（引擎层）全部通过");
    Ok(())
}

fn isolated_engine(label: &str) -> anyhow::Result<(tempfile::TempDir, SnapshotEngine)> {
    let temp = tempfile::tempdir()?;
    let base = temp.path().join(format!("openrollback_{label}"));
    let engine = SnapshotEngine::new(&base)?;
    Ok((temp, engine))
}

fn parent_dir() -> anyhow::Result<PathBuf> {
    dirs::desktop_dir()
        .or_else(dirs::document_dir)
        .ok_or_else(|| anyhow::anyhow!("无法定位桌面或文档目录"))
}

fn smoke_chinese_space_path() -> anyhow::Result<()> {
    let parent = parent_dir()?;
    let test_dir = parent.join(CHINESE_DIR);

    if test_dir.exists() {
        fs::remove_dir_all(&test_dir)?;
    }
    fs::create_dir_all(&test_dir)?;

    let file_path = test_dir.join(TEST_FILE);
    fs::write(&file_path, TEST_CONTENT)?;
    ok(&format!("已创建目录: {}", test_dir.display()));
    ok(&format!("已创建文件: {TEST_FILE}"));

    let (_guard, engine) = isolated_engine("smoke")?;
    engine.start_watching(&test_dir)?;
    ok("WATCH 已启动");

    thread::sleep(Duration::from_millis(400));
    fs::remove_file(&file_path)?;
    ok("已删除测试文件");
    thread::sleep(Duration::from_millis(500));

    anyhow::ensure!(
        engine.pending_changes() > 0,
        "删除后应有 pending 变更"
    );

    let (snapshot_id, file_count) = engine.create_snapshot("中文路径冒烟快照".into())?;
    ok(&format!("快照 #{snapshot_id}，{file_count} 个变更"));

    anyhow::ensure!(!file_path.exists(), "快照后文件应已删除");

    let result = engine.rollback(snapshot_id)?;
    anyhow::ensure!(result.restored >= 1, "应恢复至少 1 个文件");

    let restored = fs::read_to_string(&file_path)?;
    anyhow::ensure!(
        restored == TEST_CONTENT,
        "回滚后内容不匹配: got {restored:?}"
    );
    ok("回滚后文件内容与删除前一致，无乱码");

    engine.stop_watching()?;
    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

fn sad_empty_folder_snapshot() -> anyhow::Result<()> {
    let work = tempfile::tempdir()?;
    let empty = work.path().join("empty_watch");
    fs::create_dir_all(&empty)?;

    let (_guard, engine) = isolated_engine("sad_empty")?;
    engine.start_watching(&empty)?;
    ok("已对空文件夹启动 WATCH");

    let err = engine
        .create_snapshot("不应成功的快照".into())
        .expect_err("空文件夹无变更时不应创建快照");

    let msg = err.to_string();
    anyhow::ensure!(
        msg.contains("暂无待提交") || msg.contains("snapshot error"),
        "错误信息应友好且可读，实际: {msg}"
    );
    ok(&format!("正确拒绝: {}", truncate(&msg, 80)));
    Ok(())
}

fn sad_snapshot_without_watch() -> anyhow::Result<()> {
    let (_guard, engine) = isolated_engine("sad_nowatch")?;

    let err = engine
        .create_snapshot("未监控".into())
        .expect_err("未 WATCH 时不应创建快照");

    let msg = err.to_string();
    anyhow::ensure!(
        msg.contains("监控未启动") || msg.contains("watcher") || msg.contains("snapshot error"),
        "应提示先启动监控，实际: {msg}"
    );
    ok(&format!("正确拒绝: {}", truncate(&msg, 80)));
    Ok(())
}

fn sad_protected_system_path() -> anyhow::Result<()> {
    let candidates: &[&str] = &[
        r"C:\System Volume Information",
        r"C:\Windows\System32\config",
    ];

    let (_guard, engine) = isolated_engine("sad_perm")?;
    let mut tested = false;

    for path_str in candidates {
        let path = PathBuf::from(path_str);
        if !path.exists() {
            continue;
        }
        tested = true;
        let result = engine.start_watching(&path);
        match result {
            Err(e) => {
                let msg = e.to_string();
                anyhow::ensure!(!msg.is_empty(), "错误信息不应为空");
                ok(&format!(
                    "受保护路径 {} 被优雅拒绝: {}",
                    path.display(),
                    truncate(&msg, 72)
                ));
                return Ok(());
            }
            Ok(()) => {
                // 极少数环境可能允许监控；尝试无变更快照应仍被拒绝
                let snap_err = engine.create_snapshot("系统目录快照".into());
                engine.stop_watching().ok();
                if let Err(e) = snap_err {
                    ok(&format!(
                        "可监控但无变更时被拒绝: {}",
                        truncate(&e.to_string(), 72)
                    ));
                    return Ok(());
                }
            }
        }
    }

    if !tested {
        ok("跳过：当前环境无可用受保护系统路径候选");
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("\n❌ 发布验证失败: {e:#}");
        std::process::exit(1);
    }
}
