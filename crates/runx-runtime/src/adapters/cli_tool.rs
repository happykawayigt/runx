use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use runx_contracts::JsonValue;

use crate::RuntimeError;
use crate::adapter::{InvocationStatus, SkillAdapter, SkillInvocation, SkillOutput};
use crate::credentials::CredentialDelivery;
use crate::sandbox::{SandboxPlan, prepare_process_sandbox};

const OUTPUT_LIMIT_BYTES: usize = 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
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
        let sandbox = prepare_process_sandbox(
            &request.source,
            &request.skill_directory,
            &request.inputs,
            &request.env,
        )?;
        let mut child = spawn_cli_tool_process(&sandbox, &credential_delivery)?;
        let stdout = capture_pipe(child.stdout.take(), "opening cli-tool stdout pipe")?;
        let stderr = capture_pipe(child.stderr.take(), "opening cli-tool stderr pipe")?;
        write_stdin(&mut child, &request)?;
        let timed_out = wait_for_exit(&mut child, request.source.timeout_seconds)?;
        let status = child
            .wait()
            .map_err(|source| RuntimeError::io("waiting for cli-tool process", source))?;
        let stdout =
            collect_redacted_output(stdout, &credential_delivery, "collecting cli-tool stdout")?;
        let stderr =
            collect_redacted_output(stderr, &credential_delivery, "collecting cli-tool stderr")?;
        Ok(cli_tool_output(
            started, status, timed_out, stdout, stderr, sandbox,
        ))
    }
}

fn spawn_cli_tool_process(
    sandbox: &SandboxPlan,
    credential_delivery: &CredentialDelivery,
) -> Result<Child, RuntimeError> {
    let mut command = Command::new(&sandbox.command);
    command
        .args(&sandbox.args)
        .current_dir(&sandbox.cwd)
        .env_clear()
        .envs(&sandbox.env)
        .envs(credential_delivery.secret_env().iter())
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
        metadata: sandbox.metadata,
    }
}

fn write_stdin(
    child: &mut std::process::Child,
    request: &SkillInvocation,
) -> Result<(), RuntimeError> {
    let Some(mut stdin) = child.stdin.take() else {
        return Ok(());
    };
    if request.source.input_mode.as_deref() == Some("stdin") {
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
) -> Result<bool, RuntimeError> {
    let timeout = timeout_seconds.map(Duration::from_secs);
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|source| RuntimeError::io("polling cli-tool process", source))?
            .is_some()
        {
            return Ok(false);
        }
        if timeout.is_some_and(|timeout| started.elapsed() >= timeout) {
            kill_timed_out_process(child, KillSignal::Terminate)?;
            thread::sleep(FORCE_KILL_GRACE);
            kill_timed_out_process(child, KillSignal::Force)?;
            return Ok(true);
        }
        thread::sleep(POLL_INTERVAL);
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

pub fn output_object(output: &SkillOutput) -> runx_contracts::JsonObject {
    let mut object = runx_contracts::JsonObject::new();
    if let Ok(JsonValue::Object(parsed)) = serde_json::from_str::<JsonValue>(&output.stdout) {
        object.extend(parsed);
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
