#!/usr/bin/env python3
"""
OpenRollback 端到端自动化测试（Python 入口）

自动执行：准备桌面测试目录 → 快照 → 验证备份 → 破坏 → 回滚 → 验证恢复
核心逻辑在 Rust 二进制 openrollback-e2e-autotest 中（真实 WATCH + notify + 引擎 API）。
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO_MANIFEST = ROOT / "src-tauri" / "Cargo.toml"
BIN_NAME = "openrollback-e2e-autotest"


def run_step(title: str) -> None:
    print()
    print("=" * 60)
    print(title)
    print("=" * 60)


def main() -> int:
    run_step("OpenRollback E2E 自动化测试")
    print(f"项目根目录: {ROOT}")
    print(f"桌面测试目录: {Path.home() / 'Desktop' / 'openrollback_test'}")

    run_step("编译并运行 Rust E2E 测试二进制")
    cmd = [
        "cargo",
        "run",
        "--manifest-path",
        str(CARGO_MANIFEST),
        "--bin",
        BIN_NAME,
        "--quiet",
    ]
    print("执行命令:", " ".join(cmd))
    print()

    env = os.environ.copy()
    # 确保 cargo 在 PATH 中（Windows 常见安装路径）
    cargo_bin = Path.home() / ".cargo" / "bin"
    if cargo_bin.exists():
        env["PATH"] = str(cargo_bin) + os.pathsep + env.get("PATH", "")

    result = subprocess.run(
        cmd,
        cwd=str(ROOT),
        env=env,
        text=True,
    )

    if result.returncode != 0:
        print()
        print("❌ E2E 测试失败 (exit code", result.returncode, ")")
        return result.returncode

    print()
    print("✅ Python 脚本：全部步骤已由 Rust 后端验证通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
