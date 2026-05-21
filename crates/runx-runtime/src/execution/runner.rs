//! Native runtime engine for runx graphs.
//!
//! The public surface lives here: [`Runtime`], [`RuntimeOptions`], [`StepRun`],
//! [`GraphRun`], [`GraphCheckpoint`], and the feature-gated [`run_graph_file`]
//! helper. The internal state machine and the per-step execution helpers live
//! in private submodules.

use std::collections::BTreeMap;
use std::path::Path;

use runx_contracts::{ExecutionEvent, FanoutReceiptSyncPoint, HarnessReceipt, JsonObject};
use runx_core::state_machine::SequentialGraphState;
use runx_parser::ExecutionGraph;

use super::graph::load_graph;
use crate::RuntimeError;
use crate::adapter::{SkillAdapter, SkillOutput};
use crate::caller::{Caller, NoopCaller};
use crate::journal::ExecutionJournal;
use crate::receipts::graph_receipt;

mod authority;
mod execution;
mod inputs;
mod steps;
mod sync;

use execution::GraphExecution;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeOptions {
    pub created_at: String,
    pub env: BTreeMap<String, String>,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            created_at: "2026-05-18T00:00:00Z".to_owned(),
            env: safe_default_env(),
        }
    }
}

fn safe_default_env() -> BTreeMap<String, String> {
    let allowed = ["PATH", "SystemRoot", "PATHEXT"];
    allowed
        .into_iter()
        .filter_map(|key| std::env::var(key).ok().map(|value| (key.to_owned(), value)))
        .collect()
}

#[derive(Clone, Debug)]
pub struct StepRun {
    pub step_id: String,
    pub attempt: u32,
    pub skill: String,
    pub runner: Option<String>,
    pub fanout_group: Option<String>,
    pub output: SkillOutput,
    pub outputs: JsonObject,
    pub receipt: HarnessReceipt,
}

#[derive(Clone, Debug)]
pub struct GraphRun {
    pub graph: ExecutionGraph,
    pub state: SequentialGraphState,
    pub steps: Vec<StepRun>,
    pub sync_points: Vec<FanoutReceiptSyncPoint>,
    pub receipt: HarnessReceipt,
    pub journal: ExecutionJournal,
}

#[derive(Clone, Debug)]
pub struct GraphCheckpoint {
    pub graph_name: String,
    pub state: SequentialGraphState,
    pub steps: Vec<StepRun>,
    pub sync_points: Vec<FanoutReceiptSyncPoint>,
    pub journal: ExecutionJournal,
}

pub struct Runtime<A> {
    adapter: A,
    options: RuntimeOptions,
}

impl<A> Runtime<A>
where
    A: SkillAdapter,
{
    pub fn new(adapter: A, options: RuntimeOptions) -> Self {
        Self { adapter, options }
    }

    pub fn run_graph_file(&self, graph_path: &Path) -> Result<GraphRun, RuntimeError> {
        let mut caller = NoopCaller;
        self.run_graph_file_with_caller(graph_path, &mut caller)
    }

    pub fn run_graph_file_with_caller(
        &self,
        graph_path: &Path,
        caller: &mut dyn Caller,
    ) -> Result<GraphRun, RuntimeError> {
        let graph = load_graph(graph_path)?;
        let graph_dir = graph_path.parent().unwrap_or_else(|| Path::new("."));
        self.run_graph_with_caller(graph_dir, graph, caller)
    }

    pub fn run_graph_with_caller(
        &self,
        graph_dir: &Path,
        graph: ExecutionGraph,
        caller: &mut dyn Caller,
    ) -> Result<GraphRun, RuntimeError> {
        let mut execution = GraphExecution::new(&graph);
        execution.run(self, graph_dir, &graph, caller, None)?;
        let receipt = graph_receipt(
            &graph.name,
            &execution.runs,
            execution.sync_points.clone(),
            &self.options.created_at,
        )?;
        execution.record(
            caller,
            ExecutionEvent::Completed {
                message: format!("graph {} completed", graph.name),
                data: None,
            },
        )?;
        Ok(execution.finish(graph, receipt))
    }

    pub fn run_graph_file_until_steps(
        &self,
        graph_path: &Path,
        max_steps: usize,
    ) -> Result<GraphCheckpoint, RuntimeError> {
        let mut caller = NoopCaller;
        self.run_graph_file_until_steps_with_caller(graph_path, max_steps, &mut caller)
    }

    pub fn run_graph_file_until_steps_with_caller(
        &self,
        graph_path: &Path,
        max_steps: usize,
        caller: &mut dyn Caller,
    ) -> Result<GraphCheckpoint, RuntimeError> {
        let graph = load_graph(graph_path)?;
        let graph_dir = graph_path.parent().unwrap_or_else(|| Path::new("."));
        self.run_graph_until_steps_with_caller(graph_dir, &graph, max_steps, caller)
    }

    pub fn run_graph_until_steps_with_caller(
        &self,
        graph_dir: &Path,
        graph: &ExecutionGraph,
        max_steps: usize,
        caller: &mut dyn Caller,
    ) -> Result<GraphCheckpoint, RuntimeError> {
        let mut execution = GraphExecution::new(graph);
        execution.run(self, graph_dir, graph, caller, Some(max_steps))?;
        Ok(execution.checkpoint(graph.name.clone()))
    }

    pub fn resume_graph_file(
        &self,
        graph_path: &Path,
        checkpoint: GraphCheckpoint,
    ) -> Result<GraphRun, RuntimeError> {
        let mut caller = NoopCaller;
        self.resume_graph_file_with_caller(graph_path, checkpoint, &mut caller)
    }

    pub fn resume_graph_file_with_caller(
        &self,
        graph_path: &Path,
        checkpoint: GraphCheckpoint,
        caller: &mut dyn Caller,
    ) -> Result<GraphRun, RuntimeError> {
        let graph = load_graph(graph_path)?;
        let graph_dir = graph_path.parent().unwrap_or_else(|| Path::new("."));
        self.resume_graph_with_caller(graph_dir, graph, checkpoint, caller)
    }

    pub fn resume_graph_with_caller(
        &self,
        graph_dir: &Path,
        graph: ExecutionGraph,
        checkpoint: GraphCheckpoint,
        caller: &mut dyn Caller,
    ) -> Result<GraphRun, RuntimeError> {
        let mut execution = GraphExecution::from_checkpoint(&graph, checkpoint)?;
        execution.run(self, graph_dir, &graph, caller, None)?;
        let receipt = graph_receipt(
            &graph.name,
            &execution.runs,
            execution.sync_points.clone(),
            &self.options.created_at,
        )?;
        execution.record(
            caller,
            ExecutionEvent::Completed {
                message: format!("graph {} completed", graph.name),
                data: None,
            },
        )?;
        Ok(execution.finish(graph, receipt))
    }

    pub fn resume_graph_until_steps_with_caller(
        &self,
        graph_dir: &Path,
        graph: &ExecutionGraph,
        checkpoint: GraphCheckpoint,
        max_steps: usize,
        caller: &mut dyn Caller,
    ) -> Result<GraphCheckpoint, RuntimeError> {
        let mut execution = GraphExecution::from_checkpoint(graph, checkpoint)?;
        execution.run(self, graph_dir, graph, caller, Some(max_steps))?;
        Ok(execution.checkpoint(graph.name.clone()))
    }
}

#[cfg(feature = "cli-tool")]
pub fn run_graph_file(graph_path: impl AsRef<Path>) -> Result<GraphRun, RuntimeError> {
    let runtime = Runtime::new(
        crate::adapters::cli_tool::CliToolAdapter,
        RuntimeOptions::default(),
    );
    runtime.run_graph_file(graph_path.as_ref())
}
