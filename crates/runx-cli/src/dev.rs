use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use runx_runtime::dev::DevLane;
use runx_runtime::{DevLoopOptions, DevReportStatus, render_dev_result, run_dev_once};

use crate::launcher::DevPlan;

pub fn run_native_dev(plan: DevPlan) -> ExitCode {
    let current_dir = match env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            let _ignored = write_stderr(&format!("runx: failed to resolve cwd: {error}\n"));
            return ExitCode::from(1);
        }
    };
    let root = resolve_root(&current_dir);

    let mut options = DevLoopOptions::new(&root);
    options.unit_path = plan
        .root
        .as_ref()
        .map(|path| resolve_unit_path(&root, path));
    if let Some(lane) = &plan.lane {
        options.lane = DevLane::from(lane.as_str());
    }
    let report = match run_dev_once(&options) {
        Ok(report) => report,
        Err(error) => {
            let _ignored = write_stderr(&format!("runx: dev failed: {error:?}\n"));
            return ExitCode::from(1);
        }
    };

    let exit_code = match report.status {
        DevReportStatus::Success => 0,
        DevReportStatus::Skipped => 0,
        DevReportStatus::NeedsApproval => 0,
        DevReportStatus::Failure => 1,
    };

    let stdout = if plan.json {
        match serde_json::to_string(&report) {
            Ok(text) => format!("{text}\n"),
            Err(error) => {
                let _ignored =
                    write_stderr(&format!("runx: failed to serialize dev report: {error}\n"));
                return ExitCode::from(1);
            }
        }
    } else {
        render_dev_result(&report)
    };

    let _ignored = write_stdout(&stdout);
    ExitCode::from(exit_code)
}

fn resolve_root(current_dir: &Path) -> PathBuf {
    env::var("RUNX_PROJECT_DIR")
        .or_else(|_| env::var("RUNX_CWD"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| current_dir.to_path_buf())
}

fn resolve_unit_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn write_stdout(value: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(value.as_bytes())
}

fn write_stderr(value: &str) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    handle.write_all(value.as_bytes())
}
