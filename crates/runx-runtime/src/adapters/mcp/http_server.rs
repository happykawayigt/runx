//! Expose the governed MCP server over streamable HTTP/SSE.
//!
//! rmcp's [`StreamableHttpService`] is a `tower` service that carries the same
//! governed [`RmcpProofServer`] the stdio path uses; this drives it over a TCP
//! listener with hyper. Governance (admission, sandbox, receipt sealing) lives in
//! the server, not the transport, so the HTTP surface seals exactly like the stdio
//! surface. Each session gets a fresh governed server from the same options.
use std::sync::Arc;

use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use hyper_util::service::TowerToHyperService;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService};
use tokio::net::TcpListener;

use super::server::RmcpProofServer;
use super::types::{McpServerError, McpServerOptions};

/// Blocking entry point for the CLI: build a runtime and serve until exit.
/// Mirrors the stdio server's `block_on_rmcp_server`.
pub fn serve_mcp_http_server_blocking(
    listen_addr: &str,
    options: McpServerOptions,
) -> Result<(), McpServerError> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_io()
        .enable_time()
        .build()
        .map_err(|error| {
            McpServerError::new(format!(
                "MCP HTTP server runtime initialization failed: {error}"
            ))
        })?
        .block_on(serve_mcp_http_server(listen_addr, options))
}

/// Serve the governed MCP server over streamable HTTP at `listen_addr` until the
/// process exits (the accept loop only returns on a listener error).
pub async fn serve_mcp_http_server(
    listen_addr: &str,
    options: McpServerOptions,
) -> Result<(), McpServerError> {
    let listener = TcpListener::bind(listen_addr).await.map_err(|error| {
        McpServerError::new(format!("MCP HTTP bind {listen_addr} failed: {error}"))
    })?;
    serve_mcp_http_listener(listener, options).await
}

/// Drive the streamable-HTTP service over an already-bound listener. Split from
/// [`serve_mcp_http_server`] so tests can bind an ephemeral port and read it back.
pub(crate) async fn serve_mcp_http_listener(
    listener: TcpListener,
    options: McpServerOptions,
) -> Result<(), McpServerError> {
    let service = StreamableHttpService::new(
        move || Ok::<_, std::io::Error>(RmcpProofServer::from_options(options.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    loop {
        let (stream, _peer) = listener
            .accept()
            .await
            .map_err(|error| McpServerError::new(format!("MCP HTTP accept failed: {error}")))?;
        let io = TokioIo::new(stream);
        let hyper_service = TowerToHyperService::new(service.clone());
        tokio::spawn(async move {
            // Per-connection errors (client disconnects, malformed frames) are
            // isolated to the connection task; they must not stop the server.
            let _ = http1::Builder::new()
                .serve_connection(io, hyper_service)
                .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn io_err(context: &str, error: impl std::fmt::Display) -> McpServerError {
        McpServerError::new(format!("{context}: {error}"))
    }

    fn test_options() -> McpServerOptions {
        McpServerOptions {
            package_name: "http-server-test".to_owned(),
            package_version: "0.0.0".to_owned(),
            tools: Vec::new(),
        }
    }

    #[test]
    fn serves_an_mcp_initialize_over_http() -> Result<(), McpServerError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| io_err("building the test runtime", error))?;
        runtime.block_on(async {
            // Bind first, then hand the listener to the server, so the client can
            // connect without racing the accept loop.
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .map_err(|error| io_err("binding the test listener", error))?;
            let addr = listener
                .local_addr()
                .map_err(|error| io_err("reading the bound addr", error))?;
            tokio::spawn(serve_mcp_http_listener(listener, test_options()));

            let mut stream = tokio::net::TcpStream::connect(addr)
                .await
                .map_err(|error| io_err("connecting to the server", error))?;
            let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#;
            let request = format!(
                "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(request.as_bytes())
                .await
                .map_err(|error| io_err("writing the request", error))?;

            let mut buffer = vec![0_u8; 4096];
            let read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer))
                .await
                .map_err(|error| io_err("response timed out", error))?
                .map_err(|error| io_err("reading the response", error))?;
            let response = String::from_utf8_lossy(&buffer[..read]);
            let head = &response[..response.len().min(600)];
            assert!(
                response.starts_with("HTTP/1.1 200"),
                "the governed MCP server must answer the http initialize with 200; got: {head}"
            );
            let lower = response.to_ascii_lowercase();
            assert!(
                lower.contains("protocolversion")
                    || lower.contains("serverinfo")
                    || lower.contains("mcp-session-id")
                    || lower.contains("\"result\""),
                "the response must carry an mcp initialize result; got: {head}"
            );
            Ok(())
        })
    }
}
