mod ids;
pub mod init;
pub mod new;
pub mod templates;

pub use init::{
    InitAction, InitGeneratedValues, RunxInitOptions, RunxInitResult, RunxInstallState,
    RunxProjectState, ensure_runx_install_state, ensure_runx_project_state, runx_init,
};
pub use new::{RunxNewOptions, RunxNewResult, sanitize_runx_package_name, scaffold_runx_package};

use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ScaffoldError {
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Json {
        action: &'static str,
        path: PathBuf,
        source: serde_json::Error,
    },
    InvalidState {
        path: PathBuf,
        message: String,
    },
    NonEmptyTarget {
        path: PathBuf,
    },
}

impl ScaffoldError {
    pub(crate) fn io(action: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            action,
            path: path.into(),
            source,
        }
    }

    pub(crate) fn json(
        action: &'static str,
        path: impl Into<PathBuf>,
        source: serde_json::Error,
    ) -> Self {
        Self::Json {
            action,
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for ScaffoldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                action,
                path,
                source,
            } => write!(formatter, "{action} {}: {source}", path.display()),
            Self::Json {
                action,
                path,
                source,
            } => write!(formatter, "{action} {}: {source}", path.display()),
            Self::InvalidState { path, message } => {
                write!(
                    formatter,
                    "{} is not a valid Runx state: {message}",
                    path.display()
                )
            }
            Self::NonEmptyTarget { path } => {
                write!(
                    formatter,
                    "Refusing to scaffold into non-empty directory: {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ScaffoldError {}
