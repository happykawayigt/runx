use std::collections::BTreeMap;
use std::path::PathBuf;

use runx_contracts::{ContextEntry, JsonObject};
use runx_parser::SkillSource;
use serde::{Deserialize, Serialize};

use crate::RuntimeError;
use crate::credentials::CredentialDelivery;

/// Metadata key under which a skill's non-secret credential-delivery
/// observations are recorded on [`SkillOutput::metadata`].
pub const CREDENTIAL_DELIVERY_OBSERVATIONS_METADATA: &str = "credential_delivery_observations";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvocationStatus {
    Success,
    Failure,
}

#[derive(Clone, Debug)]
pub struct SkillInvocation {
    pub skill_name: String,
    pub source: SkillSource,
    pub inputs: JsonObject,
    pub resolved_inputs: JsonObject,
    pub current_context: Vec<ContextEntry>,
    pub skill_directory: PathBuf,
    pub env: BTreeMap<String, String>,
    pub credential_delivery: CredentialDelivery,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkillOutput {
    pub status: InvocationStatus,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub metadata: JsonObject,
}

impl SkillOutput {
    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.status == InvocationStatus::Success
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FanoutExecutionMode {
    Serial,
    IsolatedParallel,
}

pub trait SkillAdapter {
    fn adapter_type(&self) -> &'static str;
    fn invoke(&self, request: SkillInvocation) -> Result<SkillOutput, RuntimeError>;

    fn fanout_execution_mode(&self, source: &SkillSource) -> FanoutExecutionMode {
        let _ = source;
        FanoutExecutionMode::Serial
    }

    fn clone_for_fanout(&self) -> Option<Box<dyn SkillAdapter + Send + Sync>> {
        None
    }
}

impl<A> SkillAdapter for Box<A>
where
    A: SkillAdapter + ?Sized,
{
    fn adapter_type(&self) -> &'static str {
        self.as_ref().adapter_type()
    }

    fn invoke(&self, request: SkillInvocation) -> Result<SkillOutput, RuntimeError> {
        self.as_ref().invoke(request)
    }

    fn fanout_execution_mode(&self, source: &SkillSource) -> FanoutExecutionMode {
        self.as_ref().fanout_execution_mode(source)
    }

    fn clone_for_fanout(&self) -> Option<Box<dyn SkillAdapter + Send + Sync>> {
        self.as_ref().clone_for_fanout()
    }
}
