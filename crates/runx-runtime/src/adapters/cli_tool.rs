// rust-style-allow: large-file - the cli-tool adapter keeps process spawning,
// sandbox wrapping, output draining, redaction, and timeout cleanup in one
// auditable subprocess boundary.
use std::fs;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use runx_contracts::JsonValue;
use wait_timeout::ChildExt;

use crate::RuntimeError;
use crate::adapter::{InvocationStatus, SkillAdapter, SkillInvocation, SkillOutput};
use crate::credentials::CredentialDelivery;
use crate::sandbox::{SandboxPlan, prepare_process_sandbox};

const OUTPUT_LIMIT_BYTES: usize = 1024 * 1024;
const FORCE_KILL_GRACE: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct CliToolAdapter;

impl SkillAdapter for CliToolAdapter {
    fn adapter_type(&self) -> &'static str {
        "cli-tool"
    }

    fn invoke(&self, request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
        let started = Instant::now();
        let credential_delivery = request.credential_delivery.clone();
        credential_delivery.reject_process_env_boundary("cli-tool")?;
        let sandbox = prepare_process_sandbox(
            &request.source,
            &request.skill_directory,
            &request.inputs,
            &request.env,
        )?;
        let mut child = match spawn_cli_tool_process(&sandbox) {
            Ok(child) => child,
            Err(error) => {
                cleanup_sandbox(&sandbox);
                return Err(error);
            }
        };
        let stdout = capture_pipe(child.stdout.take(), "opening cli-tool stdout pipe")?;
        let stderr = capture_pipe(child.stderr.take(), "opening cli-tool stderr pipe")?;
        write_stdin(&mut child, &request)?;
        let (status, timed_out) = wait_for_exit(&mut child, request.source.timeout_seconds)?;
        let stdout =
            collect_redacted_output(stdout, &credential_delivery, "collecting cli-tool stdout")?;
        let stderr =
            collect_redacted_output(stderr, &credential_delivery, "collecting cli-tool stderr")?;
        let cleanup_errors = cleanup_sandbox(&sandbox);
        let mut output = cli_tool_output(started, status, timed_out, stdout, stderr, sandbox);
        if !cleanup_errors.is_empty() {
            output.metadata.insert(
                "cleanup_errors".to_owned(),
                JsonValue::Array(cleanup_errors.into_iter().map(JsonValue::String).collect()),
            );
        }
        Ok(output)
    }
}

fn spawn_cli_tool_process(sandbox: &SandboxPlan) -> Result<Child, RuntimeError> {
    let mut command = Command::new(&sandbox.command);
    command
        .args(&sandbox.args)
        .current_dir(&sandbox.cwd)
        .env_clear()
        .envs(&sandbox.env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(&mut command);
    command
        .spawn()
        .map_err(|source| RuntimeError::io("spawning cli-tool process", source))
}

fn capture_pipe<R>(
    pipe: Option<R>,
    context: &'static str,
) -> Result<JoinHandle<std::io::Result<CapturedOutput>>, RuntimeError>
where
    R: Read + Send + 'static,
{
    pipe.map(capture_stream).ok_or_else(|| io_error(context))
}

fn collect_redacted_output(
    handle: JoinHandle<std::io::Result<CapturedOutput>>,
    credential_delivery: &CredentialDelivery,
    context: &'static str,
) -> Result<CapturedText, RuntimeError> {
    let output = join_capture(handle, context)?;
    if output.truncated {
        return Ok(CapturedText {
            text: String::new(),
            truncated: true,
        });
    }
    Ok(CapturedText {
        text: credential_delivery.redact_bytes_to_string(output.bytes, OUTPUT_LIMIT_BYTES),
        truncated: false,
    })
}

fn cli_tool_output(
    started: Instant,
    status: ExitStatus,
    timed_out: bool,
    stdout: CapturedText,
    stderr: CapturedText,
    sandbox: SandboxPlan,
) -> SkillOutput {
    let output_truncated = stdout.truncated || stderr.truncated;
    let success = status.success() && !timed_out && !output_truncated;
    let (stdout, stderr) = if output_truncated {
        (
            String::new(),
            format!(
                "runx cli-tool output exceeded {OUTPUT_LIMIT_BYTES} byte capture limit; stdout/stderr omitted"
            ),
        )
    } else {
        (stdout.text, stderr.text)
    };
    SkillOutput {
        status: if success {
            InvocationStatus::Success
        } else {
            InvocationStatus::Failure
        },
        stdout,
        stderr,
        exit_code: status.code(),
        duration_ms: duration_ms(started),
        metadata: sandbox.metadata.clone(),
    }
}

fn write_stdin(
    child: &mut std::process::Child,
    request: &SkillInvocation,
) -> Result<(), RuntimeError> {
    let Some(mut stdin) = child.stdin.take() else {
        return Ok(());
    };
    if request.source.input_mode == Some(runx_parser::InputMode::Stdin) {
        let bytes = serde_json::to_vec(&request.inputs)
            .map_err(|source| RuntimeError::json("serializing stdin inputs", source))?;
        stdin
            .write_all(&bytes)
            .map_err(|source| RuntimeError::io("writing cli-tool stdin", source))?;
    }
    Ok(())
}

fn capture_stream<R>(mut reader: R) -> JoinHandle<std::io::Result<CapturedOutput>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut captured = Vec::new();
        let mut truncated = false;
        let mut buffer = [0_u8; 8192];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                return Ok(CapturedOutput {
                    bytes: captured,
                    truncated,
                });
            }
            let remaining = OUTPUT_LIMIT_BYTES.saturating_sub(captured.len());
            if remaining > 0 {
                captured.extend_from_slice(&buffer[..count.min(remaining)]);
            }
            if count > remaining {
                truncated = true;
            }
        }
    })
}

fn join_capture(
    handle: JoinHandle<std::io::Result<CapturedOutput>>,
    context: &'static str,
) -> Result<CapturedOutput, RuntimeError> {
    match handle.join() {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(source)) => Err(RuntimeError::io(context, source)),
        Err(_) => Err(RuntimeError::io(
            context,
            std::io::Error::other("output reader thread failed"),
        )),
    }
}

struct CapturedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

struct CapturedText {
    text: String,
    truncated: bool,
}

fn wait_for_exit(
    child: &mut std::process::Child,
    timeout_seconds: Option<u64>,
) -> Result<(ExitStatus, bool), RuntimeError> {
    let Some(timeout_seconds) = timeout_seconds else {
        let status = child
            .wait()
            .map_err(|source| RuntimeError::io("waiting for cli-tool process", source))?;
        return Ok((status, false));
    };

    match child
        .wait_timeout(Duration::from_secs(timeout_seconds))
        .map_err(|source| RuntimeError::io("waiting for cli-tool process with timeout", source))?
    {
        Some(status) => Ok((status, false)),
        None => {
            kill_timed_out_process(child, KillSignal::Terminate)?;
            thread::sleep(FORCE_KILL_GRACE);
            kill_timed_out_process(child, KillSignal::Force)?;
            let status = child.wait().map_err(|source| {
                RuntimeError::io("waiting for timed out cli-tool process", source)
            })?;
            Ok((status, true))
        }
    }
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

enum KillSignal {
    Terminate,
    Force,
}

impl KillSignal {
    #[cfg(unix)]
    fn kill_arg(&self) -> &'static str {
        match self {
            Self::Terminate => "-TERM",
            Self::Force => "-KILL",
        }
    }
}

#[cfg(unix)]
fn kill_timed_out_process(
    child: &mut std::process::Child,
    signal: KillSignal,
) -> Result<(), RuntimeError> {
    let process_group = format!("-{}", child.id());
    let status = Command::new("/bin/kill")
        .arg(signal.kill_arg())
        .arg(&process_group)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if status.is_ok_and(|status| status.success()) {
        return Ok(());
    }
    if child
        .try_wait()
        .map_err(|source| RuntimeError::io("polling timed out cli-tool process", source))?
        .is_some()
    {
        return Ok(());
    }
    kill_direct_child_if_running(child)
}

#[cfg(not(unix))]
fn kill_timed_out_process(
    child: &mut std::process::Child,
    _signal: KillSignal,
) -> Result<(), RuntimeError> {
    kill_direct_child_if_running(child)
}

fn kill_direct_child_if_running(child: &mut std::process::Child) -> Result<(), RuntimeError> {
    if child
        .try_wait()
        .map_err(|source| RuntimeError::io("polling timed out cli-tool process", source))?
        .is_some()
    {
        return Ok(());
    }
    child
        .kill()
        .map_err(|source| RuntimeError::io("killing timed out cli-tool process", source))
}

fn duration_ms(started: Instant) -> u64 {
    let millis = started.elapsed().as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

fn io_error(context: &'static str) -> RuntimeError {
    RuntimeError::io(context, std::io::Error::other(context))
}

fn cleanup_sandbox(sandbox: &SandboxPlan) -> Vec<String> {
    let mut errors = Vec::new();
    for path in &sandbox.cleanup_paths {
        if let Err(error) = fs::remove_dir_all(path) {
            errors.push(format!("{}: {error}", path.display()));
        }
    }
    errors
}

pub fn output_object(output: &SkillOutput) -> runx_contracts::JsonObject {
    let mut object = runx_contracts::JsonObject::new();
    if let Ok(parsed) = serde_json::from_str::<JsonValue>(&output.stdout) {
        object.insert("skill_claim".to_owned(), parsed);
    }
    object.insert(
        "stdout".to_owned(),
        JsonValue::String(output.stdout.clone()),
    );
    object.insert(
        "stderr".to_owned(),
        JsonValue::String(output.stderr.clone()),
    );
    object.insert(
        "status".to_owned(),
        JsonValue::String(if output.succeeded() {
            "success".to_owned()
        } else {
            "failure".to_owned()
        }),
    );
    object
}
