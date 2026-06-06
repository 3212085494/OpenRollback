## OpenRollback v0.1.0 — 首发版 🎉

**AI Agent 的本地文件时光机**：监控目录变更、增量快照、时间轴一键回滚。

### 📥 下载（Windows x64）

| 文件 | 说明 |
|------|------|
| `OpenRollback_0.1.0_x64-setup.exe` | **推荐**：NSIS 安装程序 |
| `OpenRollback_0.1.0_x64_en-US.msi` | MSI 安装包（企业部署友好） |

### ✨ 核心功能

- 🔭 目录监控 — 捕获新建 / 修改 / 删除
- 📸 增量快照 — SQLite 记录 + 磁盘备份
- 🗑️ 删除可回滚 — WATCH 时预缓存已有文件
- ⏪ 时间轴 UI — 可视化浏览，一键回滚
- 🤖 MCP 集成 — Cursor / Claude 可直接调用
- 🌏 中文路径 — 支持空格与 Emoji 目录名

### 🚀 快速上手

1. 安装并打开 OpenRollback
2. 点击 **WATCH**，选择项目子文件夹（不要监控整个桌面）
3. 修改文件 → **创建快照**
4. 出问题 → 时间轴选择快照 → **回滚**

### 💻 系统要求

- Windows 10 / 11（64 位）
- Microsoft Edge WebView2（Win10 通常已自带）

### ⚠️ 已知限制

- 本版本**仅提供 Windows x64** 安装包
- 预缓存上限 10,000 文件；超大目录请监控子文件夹
- 安装包未代码签名，首次运行可能提示「未知发布者」
- 快照记录 WATCH 后的变更，不能替代 Git 或专业备份

### 🔗 链接

- 仓库：https://github.com/3212085494/OpenRollback
- 完整变更日志：[CHANGELOG.md](https://github.com/3212085494/OpenRollback/blob/main/CHANGELOG.md)

**Full Changelog**: https://github.com/3212085494/OpenRollback/releases/tag/v0.1.0
