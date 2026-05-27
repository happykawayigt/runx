use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::error::{RunxError, RunxResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandPlan {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub stdin: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
}

impl CommandPlan {
    /// Build a plan from a command vector (program + leading args) plus extra args.
    ///
    /// # Examples
    ///
    /// ```
    /// use runx_sdk::command::CommandPlan;
    /// use runx_sdk::error::RunxResult;
    ///
    /// # fn main() -> RunxResult<()> {
    /// let plan = CommandPlan::new(&["runx".into()], &["skill".into(), "echo".into()])?;
    /// assert_eq!(plan.program, "runx");
    /// assert_eq!(plan.args, ["skill", "echo"]);
    /// assert_eq!(plan.argv(), ["runx", "skill", "echo"]);
    ///
    /// // An empty command is rejected.
    /// assert!(CommandPlan::new(&[], &[]).is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(command: &[String], args: &[String]) -> RunxResult<Self> {
        let (program, command_args) = command.split_first().ok_or(RunxError::EmptyCommand)?;
        let mut combined_args = command_args.to_vec();
        combined_args.extend_from_slice(args);
        Ok(Self {
            program: program.clone(),
            args: combined_args,
            cwd: None,
            env: BTreeMap::new(),
            stdin: None,
        })
    }

    pub fn with_cwd(mut self, cwd: Option<PathBuf>) -> Self {
        self.cwd = cwd;
        self
    }

    pub fn with_env(mut self, env: BTreeMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn with_stdin(mut self, stdin: Option<String>) -> Self {
        self.stdin = stdin;
        self
    }

    pub fn argv(&self) -> Vec<String> {
        let mut argv = Vec::with_capacity(self.args.len() + 1);
        argv.push(self.program.clone());
        argv.extend(self.args.clone());
        argv
    }
}

pub fn run_command(plan: &CommandPlan) -> RunxResult<CommandOutput> {
    let mut command = Command::new(&plan.program);
    command.args(&plan.args);
    if let Some(cwd) = &plan.cwd {
        command.current_dir(cwd);
    }
    if !plan.env.is_empty() {
        command.envs(&plan.env);
    }
    if plan.stdin.is_some() {
        command.stdin(Stdio::piped());
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = command.spawn()?;
    if let Some(stdin) = &plan.stdin {
        write_stdin(&mut child, stdin)?;
    }
    let output = child.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        return Err(RunxError::CommandStatus {
            args: plan.argv(),
            status: output.status.code(),
            stderr,
        });
    }
    Ok(CommandOutput { stdout, stderr })
}

fn write_stdin(child: &mut std::process::Child, input: &str) -> RunxResult<()> {
    let mut stdin = child.stdin.take().ok_or(RunxError::MissingStdin)?;
    use std::io::Write as _;
    stdin.write_all(input.as_bytes())?;
    Ok(())
}
