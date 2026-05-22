// rust-style-allow: large-file because the launcher binary currently owns
// native command IO, delegation, and exit-code mapping in one audited cutover
// surface.
use std::env;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use runx_cli::launcher::{LauncherAction, help_text};
use runx_runtime::LocalOrchestrator;

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().skip(1).collect();

    match runx_cli::launcher::plan_launcher(args) {
        LauncherAction::Error(message) => {
            let _ignored = write_stderr_line(&format!("runx: {message}"));
            ExitCode::from(64)
        }
        LauncherAction::PrintHelp => write_stdout(&help_text()),
        LauncherAction::PrintVersion => {
            write_stdout_line(&format!("runx-cli {}", env!("CARGO_PKG_VERSION")))
        }
        LauncherAction::RunInit(plan) => runx_cli::scaffold::run_native_init(plan),
        LauncherAction::RunNew(plan) => runx_cli::scaffold::run_native_new(plan),
        LauncherAction::RunHistory(plan) => run_native_history(plan.args),
        LauncherAction::RunList(plan) => run_native_list(plan),
        LauncherAction::RunMcp(plan) => runx_cli::mcp::run_native_mcp(plan),
        LauncherAction::RunHarness(plan) => run_native_harness(plan.fixture_paths),
        LauncherAction::RunKernel(plan) => runx_cli::kernel::run_native_kernel(plan),
        LauncherAction::RunParser(plan) => runx_cli::parser::run_native_parser(plan),
        LauncherAction::RunConnect(plan) => run_native_connect(plan),
        LauncherAction::RunConfig(plan) => run_native_config(plan),
        LauncherAction::RunPolicy(plan) => runx_cli::policy::run_native_policy(plan),
        LauncherAction::RunRegistry(plan) => runx_cli::registry::run_native_registry(plan),
        LauncherAction::RunSkill(plan) => runx_cli::skill::run_native_skill(plan),
        LauncherAction::RunDoctor(plan) => runx_cli::doctor::run_native_doctor(plan),
        LauncherAction::RunDev(plan) => runx_cli::dev::run_native_dev(plan),
        LauncherAction::RunTool(plan) => runx_cli::tool::run_native_tool(plan),
    }
}

fn run_native_connect(plan: runx_cli::connect::ConnectPlan) -> ExitCode {
    let message = "runx connect is not available in the MIT OSS CLI; use the hosted/private CLI distribution for OAuth brokerage";
    if plan.json {
        let _ignored = write_stdout_line(
            &serde_json::json!({
                "status": "error",
                "error": message,
            })
            .to_string(),
        );
        return ExitCode::from(1);
    }
    let _ignored = write_stderr_line(&format!("runx: {message}"));
    ExitCode::from(1)
}

fn run_native_history(args: Vec<OsString>) -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            let _ignored = write_stderr_line(&format!("runx: failed to resolve cwd: {error}"));
            return ExitCode::from(1);
        }
    };
    match runx_cli::history::run_history_command(&args, &runx_cli::history::env_map(), &cwd) {
        Ok(output) => write_stdout(&output.output),
        Err(runx_cli::history::HistoryCliError::InvalidArgs(message)) => {
            let _ignored = write_stderr_line(&format!("runx: {message}"));
            ExitCode::from(64)
        }
        Err(error) => {
            let _ignored = write_stderr_line(&format!("runx: {error}"));
            ExitCode::from(1)
        }
    }
}

fn run_native_list(plan: runx_cli::launcher::ListPlan) -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            let _ignored = write_stderr_line(&format!("runx: failed to resolve cwd: {error}"));
            return ExitCode::from(1);
        }
    };
    match runx_cli::list::run_list_command(&plan, &cwd) {
        Ok(output) => write_stdout(&output),
        Err(error) => {
            let _ignored = write_stderr_line(&format!("runx: {error}"));
            ExitCode::from(1)
        }
    }
}

fn run_native_config(plan: runx_cli::config::ConfigPlan) -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            let _ignored = write_stderr_line(&format!("runx: failed to resolve cwd: {error}"));
            return ExitCode::from(1);
        }
    };
    match runx_cli::config::run_config_command(&plan, &runx_cli::history::env_map(), &cwd) {
        Ok(output) => write_stdout(&output),
        Err(runx_cli::config::ConfigCliError::InvalidArgs(message)) => {
            let _ignored = write_stderr_line(&format!("runx: {message}"));
            ExitCode::from(64)
        }
        Err(error) => {
            let _ignored = write_stderr_line(&format!("runx: {error}"));
            ExitCode::from(1)
        }
    }
}

fn run_native_harness(fixture_paths: Vec<OsString>) -> ExitCode {
    let mut outputs = Vec::new();
    for fixture_path in fixture_paths {
        let request = runx_runtime::HarnessRunRequest {
            fixture_path: PathBuf::from(fixture_path),
        };
        match LocalOrchestrator.run_harness(&request) {
            Ok(output) => outputs.push(output.output),
            Err(error) => {
                let _ignored = write_stderr_line(&format!(
                    "runx: native harness replay failed for {}: {error}",
                    request.fixture_path.display()
                ));
                return ExitCode::from(1);
            }
        }
    }

    let output = if outputs.len() == 1 {
        outputs.pop().unwrap_or(runx_contracts::JsonValue::Null)
    } else {
        runx_contracts::JsonValue::Array(outputs)
    };
    match serde_json::to_string_pretty(&output) {
        Ok(json) => write_stdout_line(&json),
        Err(error) => {
            let _ignored = write_stderr_line(&format!(
                "runx: failed to serialize harness receipt: {error}"
            ));
            ExitCode::from(1)
        }
    }
}

fn write_stdout(message: &str) -> ExitCode {
    let mut stdout = io::stdout().lock();
    if stdout.write_all(message.as_bytes()).is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn write_stdout_line(message: &str) -> ExitCode {
    let mut stdout = io::stdout().lock();
    if writeln!(stdout, "{message}").is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn write_stderr_line(message: &str) -> ExitCode {
    let mut stderr = io::stderr().lock();
    if writeln!(stderr, "{message}").is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
