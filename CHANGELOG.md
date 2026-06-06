# Changelog

本文件记录 OpenRollback 的版本变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)。

## [0.1.0] - 2026-06-06

### 新增

- 本地目录递归监控（`notify`），支持新建 / 修改 / 删除事件
- 增量快照：SQLite 元数据 + `~/.openrollback/backups/` 文件备份
- WATCH 时预缓存已有文件，支持「直接删除」后回滚
- 时间轴 UI：快照浏览、回滚确认、删除快照、操作日志
- 中文路径、空格路径、Emoji 路径支持
- MCP stdio 服务（5 个工具：`watch` / `stop_watch` / `commit` / `rollback` / `snapshots`）
- Windows x64 安装包（NSIS `.exe` + MSI）

### 修复与改进

- 回滚路径统一规范化（去除 `\\?\` 扩展前缀）
- 文件被占用时返回中文友好错误提示
- 空目录 / 未 WATCH 时拒绝创建快照并提示原因
- Release 构建下控制台无调试日志输出

### 已知限制

- v0.1.0 **仅提供 Windows x64** 预编译安装包
- 预缓存默认上限 10,000 个文件；超大目录建议监控子文件夹
- 未代码签名，Windows 可能提示「未知发布者」
- 快照为增量变更记录，非全量磁盘镜像

[0.1.0]: https://github.com/3212085494/OpenRollback/releases/tag/v0.1.0
