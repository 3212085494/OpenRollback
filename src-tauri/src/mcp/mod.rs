//! MCP (Model Context Protocol) stdio 服务模块。
//!
//! 通过 JSON-RPC 2.0 over stdio 暴露 OpenRollback 工具，供 AI Agent 调用。

pub mod server;

pub use server::run_stdio_server;
