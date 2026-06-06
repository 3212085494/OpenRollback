//! 桌面端到端自动化测试：WATCH → 删除文件 → 快照 → 回滚 → 验证恢复
//!
//! 由 `scripts/e2e_autotest.py` 调用。

use openrollback_lib::core::snapshot::SnapshotEngine;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

const TEST_DIR_NAME: &str = "openrollback_test";
const FILES: &[(&str, &str)] = &[
    ("test1.txt", "content of test1"),
    ("test2.js", "console.log('test2');"),
    ("test3.md", "# test3 markdown"),
];

fn step(n: u8, msg: &str) {
    println!("[Step {n}] {msg}");
}

fn ok(msg: &str) {
    println!("  ✅ {msg}");
}

fn fail(msg: impl Into<String>) -> anyhow::Error {
    anyhow::anyhow!(msg.into())
}

fn desktop_test_dir() -> anyhow::Result<PathBuf> {
    let desktop = dirs::desktop_dir().ok_or_else(|| fail("无法定位桌面目录"))?;
    Ok(desktop.join(TEST_DIR_NAME))
}

fn count_files_in_dir(dir: &Path) -> anyhow::Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let mut n = 0usize;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            n += 1;
        }
    }
    Ok(n)
}

fn count_backup_files(backup_dir: &Path) -> anyhow::Result<usize> {
    count_files_in_dir(backup_dir)
}

fn verify_test_files(dir: &Path, expect_present: bool) -> anyhow::Result<()> {
    for (name, expected) in FILES {
        let path = dir.join(name);
        if expect_present {
            anyhow::ensure!(path.exists(), "缺少文件: {}", path.display());
            let content = fs::read_to_string(&path)?;
            anyhow::ensure!(
                content == *expected,
                "文件内容不匹配 {}: got {:?}",
                name,
                content
            );
        } else {
            anyhow::ensure!(!path.exists(), "文件应已删除但仍存在: {}", path.display());
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("\n❌ E2E 测试失败: {e:#}");
        std::process::exit(1);
    }
    println!("\n🎉 E2E 自动化测试全部通过！");
}

fn run() -> anyhow::Result<()> {
    let test_dir = desktop_test_dir()?;
    let isolated_data = std::env::temp_dir().join("openrollback_e2e_autotest_data");

    // ── Step 1: 准备环境 ──
    step(1, "准备环境：创建桌面测试文件夹与 3 个文件");
    if test_dir.exists() {
        fs::remove_dir_all(&test_dir)?;
        ok(&format!("已清理旧目录: {}", test_dir.display()));
    }
    fs::create_dir_all(&test_dir)?;
    for (name, content) in FILES {
        fs::write(test_dir.join(name), content)?;
    }
    ok(&format!("目录: {}", test_dir.display()));
    for (name, _) in FILES {
        ok(&format!("已创建 {name}"));
    }
    let scan_count = count_files_in_dir(&test_dir)?;
    ok(&format!("目录含 {scan_count} 个文件"));
    anyhow::ensure!(scan_count == 3, "准备阶段应有 3 个文件，实际 {scan_count}");

    // ── Step 2: 启动监控 + 创建快照（捕获删除变更）──
    step(2, "启动监控并调用后端 create_snapshot 核心逻辑");
    if isolated_data.exists() {
        fs::remove_dir_all(&isolated_data)?;
    }
    let engine = SnapshotEngine::new(&isolated_data)?;
    engine.start_watching(&test_dir)?;
    let cached = engine.cached_files();
    anyhow::ensure!(cached >= 3, "预缓存应至少包含 3 个测试文件，实际 {cached}");
    ok(&format!("WATCH 已启动，预缓存 {cached} 个文件"));

    thread::sleep(Duration::from_millis(300));

    step(2, "模拟用户删除测试文件夹内全部文件（产生 pending 变更）");
    for (name, _) in FILES {
        fs::remove_file(test_dir.join(name))?;
        ok(&format!("已删除 {name}"));
    }
    thread::sleep(Duration::from_millis(500));

    let pending = engine.pending_changes();
    ok(&format!("待提交变更 {pending} 项"));
    anyhow::ensure!(pending >= 3, "删除后 pending 应 >= 3，实际 {pending}");

    let (snapshot_id, file_count) = engine.create_snapshot("E2E 自动化测试快照".into())?;
    ok(&format!(
        "create_snapshot 返回 id={snapshot_id}, file_count={file_count}"
    ));
    anyhow::ensure!(file_count >= 3, "快照应记录至少 3 个文件变更，实际 {file_count}");

    // ── Step 3: 验证备份目录 ──
    step(3, "验证备份目录是否包含 3 个备份文件");
    let backup_dir = isolated_data.join("backups").join(snapshot_id.to_string());
    let backup_count = count_backup_files(&backup_dir)?;
    anyhow::ensure!(
        backup_count >= 3,
        "备份目录应有至少 3 个文件，实际 {backup_count}"
    );
    ok(&format!("备份目录含 {backup_count} 个文件"));
    verify_test_files(&test_dir, false)?;
    ok("测试目录中 3 个文件均已删除");

    // ── Step 4: 已在 step 2 完成破坏 ──
    step(4, "模拟破坏（删除）— 已在 Step 2 完成");

    // ── Step 5: 回滚 ──
    step(5, "调用后端 rollback 核心函数");
    let result = engine.rollback(snapshot_id)?;
    ok(&format!(
        "rollback 完成: restored={}, removed={}, skipped={}",
        result.restored, result.removed, result.skipped
    ));
    for line in &result.details {
        println!("    · {line}");
    }
    anyhow::ensure!(result.restored >= 3, "应恢复至少 3 个文件");

    // ── Step 6: 验证回滚 ──
    step(6, "验证 openrollback_test 内 3 个文件是否已恢复");
    let after_count = count_files_in_dir(&test_dir)?;
    ok(&format!("回滚后目录含 {after_count} 个文件"));
    verify_test_files(&test_dir, true)?;
    ok("3 个文件内容与删除前完全一致");

    engine.stop_watching()?;

    // 可选清理（保留目录便于人工查看，仅清理隔离数据）
    let _ = fs::remove_dir_all(&isolated_data);
    ok("已清理临时引擎数据目录");

    Ok(())
}
