use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread::JoinHandle;
use std::time::Duration;

use runx_cli::connect::{ConnectAction, ConnectAuthorityKind, ConnectPlan, parse_connect_plan};

#[test]
fn parses_connect_list() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        parse_connect_plan(&args(["connect", "list", "--json"]))?,
        ConnectPlan {
            action: ConnectAction::List,
            provider: None,
            grant_id: None,
            scopes: Vec::new(),
            scope_family: None,
            authority_kind: None,
            target_repo: None,
            target_locator: None,
            json: true,
        }
    );
    Ok(())
}

#[test]
fn parses_connect_revoke() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        parse_connect_plan(&args(["connect", "revoke", "grant_github_1"]))?,
        ConnectPlan {
            action: ConnectAction::Revoke,
            provider: None,
            grant_id: Some("grant_github_1".to_owned()),
            scopes: Vec::new(),
            scope_family: None,
            authority_kind: None,
            target_repo: None,
            target_locator: None,
            json: false,
        }
    );
    Ok(())
}

#[test]
fn parses_connect_preprovision_aliases_and_scope_splitting()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        parse_connect_plan(&args([
            "connect",
            "github",
            "--scope",
            "repo:read, checks:read",
            "--scope",
            "issues:write",
            "--scope_family",
            "github_repo",
            "--authorityKind",
            "constructive",
            "--target-repo",
            "runxhq/aster",
            "--targetLocator=github:repo:runxhq/aster",
            "--json",
        ]))?,
        ConnectPlan {
            action: ConnectAction::Preprovision,
            provider: Some("github".to_owned()),
            grant_id: None,
            scopes: vec![
                "repo:read".to_owned(),
                "checks:read".to_owned(),
                "issues:write".to_owned(),
            ],
            scope_family: Some("github_repo".to_owned()),
            authority_kind: Some(ConnectAuthorityKind::Constructive),
            target_repo: Some("runxhq/aster".to_owned()),
            target_locator: Some("github:repo:runxhq/aster".to_owned()),
            json: true,
        }
    );
    Ok(())
}

#[test]
fn rejects_invalid_connect_shapes() {
    assert_eq!(
        parse_connect_plan(&args(["connect", "list", "--scope", "repo:read"])),
        Err("runx connect list does not accept provider or scope flags".to_owned())
    );
    assert_eq!(
        parse_connect_plan(&args(["connect", "revoke"])),
        Err("runx connect revoke requires exactly one grant id".to_owned())
    );
    assert_eq!(
        parse_connect_plan(&args(["connect", "github", "--authority-kind", "owner"])),
        Err("invalid connect authority kind owner".to_owned())
    );
}

#[test]
fn connect_missing_service_config_fails_before_network() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_runx"))
        .args(["connect", "list"])
        .env_remove("RUNX_CONNECT_BASE_URL")
        .env_remove("RUNX_CONNECT_ACCESS_TOKEN")
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8(output.stdout)?, "");
    assert!(String::from_utf8(output.stderr)?.contains("RUNX_CONNECT_BASE_URL"));
    Ok(())
}

#[test]
fn connect_list_empty_human_output() -> Result<(), Box<dyn std::error::Error>> {
    let server = serve_once(r#"{"grants":[]}"#)?;
    let output = run_connect_command(&server.base_url, ["connect", "list"])?;
    let request = join_request(server.handle)?;

    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)?.contains("No connections yet."));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    assert!(request.starts_with("GET /v1/grants "));
    assert_header_eq(
        &request,
        "authorization",
        "Bearer SECRET_CONNECT_ACCESS_TOKEN_DO_NOT_LEAK",
    );
    Ok(())
}

#[test]
fn connect_revoke_json_wrapper() -> Result<(), Box<dyn std::error::Error>> {
    let server = serve_once(
        &serde_json::json!({
            "status": "revoked",
            "grant": grant_json("grant_fixture_active")
        })
        .to_string(),
    )?;
    let output = run_connect_command(
        &server.base_url,
        ["connect", "revoke", "grant_fixture_active", "--json"],
    )?;
    let request = join_request(server.handle)?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains(r#""status": "success""#));
    assert!(stdout.contains(r#""connect""#));
    assert!(stdout.contains(r#""status": "revoked""#));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    assert!(request.starts_with("DELETE /v1/grants/grant_fixture_active "));
    Ok(())
}

#[test]
fn connect_revoke_human_output_omits_empty_optional_rows() -> Result<(), Box<dyn std::error::Error>>
{
    let server = serve_once(
        &serde_json::json!({
            "status": "revoked",
            "grant": {
                "grant_id": "grant_fixture_active",
                "provider": "github",
                "scopes": ["repo:read"],
                "status": "revoked"
            }
        })
        .to_string(),
    )?;
    let output = run_connect_command(
        &server.base_url,
        ["connect", "revoke", "grant_fixture_active"],
    )?;
    let _request = join_request(server.handle)?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("✓  connection revoked  revoked"));
    assert!(stdout.contains("provider  github"));
    assert!(stdout.contains("grant     grant_fixture_active"));
    assert!(stdout.contains("scopes    repo:read"));
    assert!(stdout.contains("next      runx connect github"));
    assert!(!stdout.contains("family"));
    assert!(!stdout.contains("authority"));
    assert!(!stdout.contains("locator"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

#[test]
fn connect_preprovision_json_wrapper_and_body() -> Result<(), Box<dyn std::error::Error>> {
    let server = serve_once(
        &serde_json::json!({
            "status": "created",
            "grant": grant_json("grant_created")
        })
        .to_string(),
    )?;
    let output = run_connect_command(
        &server.base_url,
        [
            "connect",
            "github",
            "--scope",
            "repo:read",
            "--scope-family",
            "github_repo",
            "--authority-kind",
            "read_only",
            "--target-repo",
            "runxhq/aster",
            "--json",
        ],
    )?;
    let request = join_request(server.handle)?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains(r#""status": "success""#));
    assert!(stdout.contains(r#""grant_id": "grant_created""#));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    assert!(request.starts_with("POST /v1/connect/sessions "));
    assert!(request.contains(r#""provider":"github""#));
    assert!(request.contains(r#""scope_family":"github_repo""#));
    Ok(())
}

#[test]
fn connect_preprovision_human_output_includes_ready_status()
-> Result<(), Box<dyn std::error::Error>> {
    let server = serve_once(
        &serde_json::json!({
            "status": "unchanged",
            "grant": grant_json("grant_existing")
        })
        .to_string(),
    )?;
    let output = run_connect_command(
        &server.base_url,
        ["connect", "github", "--scope", "repo:read"],
    )?;
    let _request = join_request(server.handle)?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("✓  connection ready  unchanged"));
    assert!(stdout.contains("provider   github"));
    assert!(stdout.contains("grant      grant_existing"));
    assert!(stdout.contains("scopes     repo:read"));
    assert!(stdout.contains("authority  read_only"));
    assert!(stdout.contains("next       runx connect list"));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}

fn args<const N: usize>(values: [&str; N]) -> Vec<OsString> {
    values.into_iter().map(OsString::from).collect()
}

struct TestServer {
    base_url: String,
    handle: JoinHandle<Result<String, String>>,
}

fn serve_once(response_body: &str) -> Result<TestServer, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let response_body = response_body.to_owned();
    let handle = std::thread::spawn(move || {
        let (mut stream, _address) = listener.accept().map_err(|error| error.to_string())?;
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .map_err(|error| error.to_string())?;
        let request = read_http_request(&mut stream)?;
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream
            .write_all(response.as_bytes())
            .map_err(|error| error.to_string())?;
        Ok(request)
    });
    Ok(TestServer {
        base_url: format!("http://{address}"),
        handle,
    })
}

fn read_http_request(stream: &mut std::net::TcpStream) -> Result<String, String> {
    let mut bytes = Vec::new();
    loop {
        let mut buffer = [0_u8; 1024];
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                bytes.extend_from_slice(&buffer[..count]);
                if request_is_complete(&bytes) {
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

fn request_is_complete(bytes: &[u8]) -> bool {
    let request = String::from_utf8_lossy(bytes);
    let Some(header_end) = request.find("\r\n\r\n") else {
        return false;
    };
    let content_length = request
        .lines()
        .find_map(|line| {
            line.split_once(':').and_then(|(name, value)| {
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim())
            })
        })
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    bytes.len() >= header_end + 4 + content_length
}

fn assert_header_eq(request: &str, expected_name: &str, expected_value: &str) {
    let found = request.lines().any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case(expected_name) && value.trim() == expected_value
        })
    });
    assert!(
        found,
        "missing expected header {expected_name}: {expected_value}"
    );
}

fn join_request(
    handle: JoinHandle<Result<String, String>>,
) -> Result<String, Box<dyn std::error::Error>> {
    match handle.join() {
        Ok(Ok(request)) => Ok(request),
        Ok(Err(error)) => Err(error.into()),
        Err(_panic) => Err("connect mock server panicked".into()),
    }
}

fn run_connect_command<const N: usize>(
    base_url: &str,
    args: [&str; N],
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_runx"))
        .args(args)
        .env("RUNX_CONNECT_BASE_URL", base_url)
        .env(
            "RUNX_CONNECT_ACCESS_TOKEN",
            "SECRET_CONNECT_ACCESS_TOKEN_DO_NOT_LEAK",
        )
        .output()?)
}

fn grant_json(grant_id: &str) -> serde_json::Value {
    serde_json::json!({
        "grant_id": grant_id,
        "principal_id": "principal_1",
        "provider": "github",
        "scopes": ["repo:read"],
        "scope_family": "github_repo",
        "authority_kind": "read_only",
        "target_repo": "runxhq/aster",
        "target_locator": "github:repo:runxhq/aster",
        "connection_id": "conn_1",
        "status": "active",
        "created_at": "2026-05-19T00:00:00Z"
    })
}
