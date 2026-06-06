//! 诊断用户数据库：`cargo run --bin inspect-db`

use openrollback_lib::core::SnapshotEngine;
use std::path::Path;

fn main() {
    let base = SnapshotEngine::default_base_dir().expect("home dir");
    let engine = SnapshotEngine::new(&base).expect("engine");
    let db = engine.database();

    println!("DB: {}", db.path().display());
    println!("Backups: {}", base.join("backups").display());

    let snapshots = engine.list_snapshots().expect("list");
    for snap in snapshots.iter().take(10) {
        let changes = db
            .get_file_changes_for_snapshot(snap.id)
            .unwrap_or_default();
        println!(
            "\n#{} [{}] {} — {} changes",
            snap.id, snap.status, snap.description, changes.len()
        );
        for c in changes.iter().take(20) {
            let backup_ok = c
                .backup_path
                .as_ref()
                .map(|p| Path::new(p).exists())
                .unwrap_or(false);
            println!(
                "  {:?} {} backup={} exists={}",
                c.action,
                c.path,
                c.backup_path.as_deref().unwrap_or("(none)"),
                backup_ok
            );
        }
        for c in &changes {
            if matches!(c.action.as_str(), "modify" | "delete") && c.backup_path.is_none() {
                println!("  !! rollback would fail: missing backup for {}", c.path);
            }
            if let Some(bp) = &c.backup_path {
                if !Path::new(bp).exists() {
                    println!("  !! rollback would fail: backup missing on disk: {bp}");
                }
            }
        }
    }
}
