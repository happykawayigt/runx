use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Eq, PartialEq)]
pub struct McpPlan {
    pub refs: Vec<PathBuf>,
    pub receipt_dir: Option<PathBuf>,
    pub runner: Option<String>,
}

pub fn parse_mcp_plan(args: &[OsString]) -> Result<McpPlan, String> {
    let subcommand = os_arg(args, 1)?;
    if subcommand != "serve" {
        return Err(format!("unknown mcp subcommand {subcommand}"));
    }
    let mut refs = Vec::new();
    let mut receipt_dir = None;
    let mut runner = None;
    let mut index = 2;
    while index < args.len() {
        let token = os_arg(args, index)?;
        if !token.starts_with("--") {
            refs.push(PathBuf::from(token));
            index += 1;
            continue;
        }
        let (flag, inline_value) = split_flag(token);
        match flag {
            "--receipt-dir" | "--receiptDir" => {
                let (value, next_index) = flag_value(args, index, flag, inline_value)?;
                receipt_dir = Some(PathBuf::from(value));
                index = next_index;
            }
            "--runner" => {
                let (value, next_index) = flag_value(args, index, flag, inline_value)?;
                runner = Some(value);
                index = next_index;
            }
            _ => return Err(format!("unknown mcp serve flag {flag}")),
        }
    }
    if refs.is_empty() {
        return Err("runx mcp serve requires at least one skill reference.".to_owned());
    }
    Ok(McpPlan {
        refs,
        receipt_dir,
        runner,
    })
}

pub fn run_native_mcp(plan: McpPlan) -> ExitCode {
    let options =
        match runx_runtime::adapters::mcp::McpServerOptions::from_skill_paths_with_execution(
            &plan.refs,
            "runx-cli",
            env!("CARGO_PKG_VERSION"),
            runx_runtime::adapters::mcp::McpServerExecutionOptions {
                runner: plan.runner,
                receipt_dir: plan.receipt_dir,
                env: std::env::vars().collect(),
            },
        ) {
            Ok(options) => options,
            Err(error) => {
                let _ignored = writeln!(std::io::stderr(), "runx: {error}");
                return ExitCode::from(1);
            }
        };
    match runx_runtime::adapters::mcp::serve_mcp_json_rpc(
        std::io::stdin().lock(),
        std::io::stdout().lock(),
        options,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ignored = writeln!(std::io::stderr(), "runx: {error}");
            ExitCode::from(1)
        }
    }
}

fn os_arg(args: &[OsString], index: usize) -> Result<&str, String> {
    args.get(index)
        .and_then(|arg| arg.to_str())
        .ok_or_else(|| "mcp arguments must be UTF-8".to_owned())
}

fn split_flag(token: &str) -> (&str, Option<&str>) {
    token
        .split_once('=')
        .map_or((token, None), |(flag, value)| (flag, Some(value)))
}

fn flag_value(
    args: &[OsString],
    index: usize,
    flag: &str,
    inline_value: Option<&str>,
) -> Result<(String, usize), String> {
    if let Some(value) = inline_value {
        return Ok((value.to_owned(), index + 1));
    }
    let value = os_arg(args, index + 1).map_err(|_| format!("{flag} requires a value"))?;
    if value.starts_with("--") {
        return Err(format!("{flag} requires a value"));
    }
    Ok((value.to_owned(), index + 2))
}
