//! MCP (Model Context Protocol) adapter.
//!
//! - `types`: shared data types and the `McpTransport` trait.
//! - `adapter`: the `McpAdapter` `SkillAdapter` entry point.
//! - `transport`: stdio process and fixture client transports.
//! - `framing`, `jsonrpc`: protocol-level helpers shared by client and server.
//! - `server`: `serve_mcp_json_rpc` and host-result projections.
//! - `server_skill`: server-side skill and graph execution.
//! - `templates`: argument templating and tool-result stringification.
//! - `sandbox_metadata`: receipt-side sandbox metadata builders.

mod adapter;
mod framing;
mod jsonrpc;
mod sandbox_metadata;
mod server;
mod server_skill;
mod templates;
mod transport;
mod types;

pub use adapter::McpAdapter;
pub use server::{mcp_tool_result_from_host_result, serve_mcp_json_rpc};
pub use templates::{map_mcp_arguments, stringify_mcp_tool_result};
pub use transport::{FixtureMcpTransport, ProcessMcpTransport};
pub use types::{
    McpContent, McpHostRunResult, McpListToolsRequest, McpServerError, McpServerExecutionOptions,
    McpServerOptions, McpServerSkillExecution, McpServerTool, McpServerToolBehavior,
    McpToolCallRequest, McpToolDescriptor, McpToolResult, McpTransport, McpTransportError,
};
