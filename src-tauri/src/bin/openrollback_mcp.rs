//! OpenRollback MCP Server 独立二进制入口。
//!
//! 通过 stdio 与 AI Agent（Cursor / Claude 等）通信。
//! 配置示例见项目根目录 `mcp-config.example.json`。

use openrollback_lib::mcp::run_stdio_server;

fn main() {
    if let Err(err) = run_stdio_server() {
        eprintln!("[openrollback-mcp] fatal: {err:#}");
        std::process::exit(1);
    }
}
