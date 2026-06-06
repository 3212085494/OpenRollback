#!/usr/bin/env bash
# OpenRollback 端到端演示脚本
# 通过 openrollback-demo 二进制模拟完整快照/回滚流程
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "✨ OpenRollback: AI 的时间机器 ✨"
echo "=================================="
echo ""

echo "▶ Running end-to-end demo (watch → AI ops → snapshot → rollback)..."
cargo run --manifest-path src-tauri/Cargo.toml --bin openrollback-demo --quiet

echo ""
echo "▶ Running unit tests..."
cargo test --manifest-path src-tauri/Cargo.toml --quiet

echo ""
echo "✅ All demo checks passed"
