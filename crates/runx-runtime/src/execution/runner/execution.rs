// rust-style-allow: large-file because graph execution keeps step planning,
// fanout synchronization, and checkpoint emission together while Rust remains
// the parity implementation for the existing execution contract.
use std::collections::BTreeMap;
use std::path::Path;

use runx_contracts::{ExecutionEvent, FanoutReceiptSyncPoint, JsonValue};
use runx_core::state_machine::{
    FanoutBranchResult, FanoutGroupPolicy, FanoutSyncDecision, FanoutSyncOutcome, GraphStepStatus,
    SequentialGraphEvent, SequentialGraphPlan, SequentialGraphState, SequentialGraphStepDefinition,
    create_sequential_graph_state, evaluate_fanout_sync, plan_sequential_graph_transition,
    transition_sequential_graph,
};
use runx_parser::{ExecutionGraph, GraphStep};

use super::super::fanout::fanout_policies;
use super::super::graph::find_step;
use super::steps::{output_error, run_step, runtime_error_step_run};
use super::sync::{
    completed_event, failed_event, fanout_sync_point, latest_fanout_receipt_ids, started_event,
};
use super::{GraphCheckpoint, GraphRun, Runtime, StepRun};
use crate::RuntimeError;
use crate::adapter::SkillAdapter;
use crate::host::Host;
use crate::journal::ExecutionJournal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StepFailureMode {
    Propagate,
    RecordAndContinue,
}

pub(super) struct GraphExecution {
    definitions: Vec<SequentialGraphStepDefinition>,
    step_indexes: BTreeMap<String, usize>,
    state: SequentialGraphState,
    pub(super) runs: Vec<StepRun>,
    pub(super) sync_points: Vec<FanoutReceiptSyncPoint>,
    journal: ExecutionJournal,
}

pub(super) struct FanoutRunPlan {
    group_id: String,
    step_ids: Vec<String>,
    attempts: BTreeMap<String, u32>,
}

pub(super) struct StepExecutionPlan<'a> {
    step_id: &'a str,
    attempt: u32,
    failure_mode: StepFailureMode,
}

impl GraphExecution {
    pub(super) fn new(graph: &ExecutionGraph) -> Self {
        let definitions = super::super::graph::step_definitions(graph);
        Self {
            state: create_sequential_graph_state(graph.name.clone(), &definitions),
            step_indexes: step_indexes(graph),
            definitions,
            runs: Vec::new(),
            sync_points: Vec::new(),
            journal: ExecutionJournal::default(),
        }
    }

    pub(super) fn from_checkpoint(
        graph: &ExecutionGraph,
        checkpoint: GraphCheckpoint,
    ) -> Result<Self, RuntimeError> {
        if checkpoint.graph_name != graph.name {
            return Err(RuntimeError::CheckpointGraphMismatch {
                checkpoint_graph: checkpoint.graph_name,
                graph: graph.name.clone(),
            });
        }
        Ok(Self {
            definitions: super::super::graph::step_definitions(graph),
            step_indexes: step_indexes(graph),
            state: checkpoint.state,
            runs: checkpoint.steps,
            sync_points: checkpoint.sync_points,
            journal: checkpoint.journal,
        })
    }

    pub(super) fn run<A>(
        &mut self,
        runtime: &Runtime<A>,
        graph_dir: &Path,
        graph: &ExecutionGraph,
        host: &mut dyn Host,
        max_new_steps: Option<usize>,
    ) -> Result<(), RuntimeError>
    where
        A: SkillAdapter,
    {
        let fanout_policies = fanout_policies(graph);
        let initial_step_count = self.runs.len();
        loop {
            if reached_step_limit(initial_step_count, self.runs.len(), max_new_steps) {
                return Ok(());
            }
            let plan = plan_sequential_graph_transition(
                &self.state,
                &self.definitions,
                &fanout_policies,
                None,
            );
            if self.apply_plan(runtime, graph_dir, graph, host, &fanout_policies, plan)? {
                break;
            }
        }
        Ok(())
    }

    pub(super) fn apply_plan<A>(
        &mut self,
        runtime: &Runtime<A>,
        graph_dir: &Path,
        graph: &ExecutionGraph,
        host: &mut dyn Host,
        fanout_policies: &BTreeMap<String, FanoutGroupPolicy>,
        plan: SequentialGraphPlan,
    ) -> Result<bool, RuntimeError>
    where
        A: SkillAdapter,
    {
        match plan {
            SequentialGraphPlan::RunStep {
                step_id, attempt, ..
            } => self.apply_step_plan(runtime, graph_dir, graph, host, &step_id, attempt),
            SequentialGraphPlan::RunFanout {
                group_id,
                step_ids,
                attempts,
                ..
            } => {
                self.run_fanout_plan(
                    runtime,
                    graph_dir,
                    graph,
                    host,
                    fanout_policies,
                    FanoutRunPlan {
                        group_id,
                        step_ids,
                        attempts,
                    },
                )?;
                Ok(false)
            }
            SequentialGraphPlan::Complete => Ok(self.complete_graph()),
            SequentialGraphPlan::Blocked {
                step_id,
                reason,
                sync_decision,
            } => self.block_graph(graph, step_id, reason, sync_decision),
            SequentialGraphPlan::Failed {
                step_id,
                reason,
                sync_decision,
            } => self.fail_graph(graph, step_id, reason, sync_decision),
            SequentialGraphPlan::Paused {
                step_id,
                reason,
                sync_decision,
            } => self.pause_for_sync(graph, step_id, reason, sync_decision),
            SequentialGraphPlan::Escalated {
                step_id,
                reason,
                sync_decision,
            } => self.escalate_for_sync(graph, step_id, reason, sync_decision),
        }
    }

    pub(super) fn apply_step_plan<A>(
        &mut self,
        runtime: &Runtime<A>,
        graph_dir: &Path,
        graph: &ExecutionGraph,
        host: &mut dyn Host,
        step_id: &str,
        attempt: u32,
    ) -> Result<bool, RuntimeError>
    where
        A: SkillAdapter,
    {
        self.run_one_step(runtime, graph_dir, graph, step_id, attempt, host)?;
        Ok(false)
    }

    pub(super) fn complete_graph(&mut self) -> bool {
        self.state = transition_sequential_graph(&self.state, &SequentialGraphEvent::Complete);
        true
    }

    pub(super) fn run_fanout_plan<A>(
        &mut self,
        runtime: &Runtime<A>,
        graph_dir: &Path,
        graph: &ExecutionGraph,
        host: &mut dyn Host,
        fanout_policies: &BTreeMap<String, FanoutGroupPolicy>,
        plan: FanoutRunPlan,
    ) -> Result<(), RuntimeError>
    where
        A: SkillAdapter,
    {
        for step_id in plan.step_ids {
            let attempt = plan.attempts.get(&step_id).copied().unwrap_or(1);
            self.run_one_step_with_mode(
                runtime,
                graph_dir,
                graph,
                host,
                StepExecutionPlan {
                    step_id: &step_id,
                    attempt,
                    failure_mode: StepFailureMode::RecordAndContinue,
                },
            )?;
        }
        self.record_proceeding_fanout_sync_point(graph, fanout_policies, &plan.group_id)
    }

    pub(super) fn block_graph(
        &mut self,
        graph: &ExecutionGraph,
        step_id: String,
        reason: String,
        sync_decision: Option<FanoutSyncDecision>,
    ) -> Result<bool, RuntimeError> {
        if let Some(sync_decision) = sync_decision {
            self.push_sync_point(graph, &sync_decision)?;
        }
        Err(RuntimeError::GraphBlocked { step_id, reason })
    }

    pub(super) fn fail_graph(
        &mut self,
        graph: &ExecutionGraph,
        step_id: String,
        reason: String,
        sync_decision: Option<FanoutSyncDecision>,
    ) -> Result<bool, RuntimeError> {
        if let Some(sync_decision) = sync_decision {
            self.push_sync_point(graph, &sync_decision)?;
        }
        self.state = transition_sequential_graph(
            &self.state,
            &SequentialGraphEvent::FailGraph {
                error: reason.clone(),
            },
        );
        Err(RuntimeError::GraphPlanningFailed { step_id, reason })
    }

    pub(super) fn pause_graph(
        &mut self,
        step_id: String,
        reason: String,
        sync_decision: runx_core::state_machine::FanoutSyncDecision,
    ) -> Result<bool, RuntimeError> {
        self.state = transition_sequential_graph(
            &self.state,
            &SequentialGraphEvent::PauseGraph {
                reason: reason.clone(),
            },
        );
        Err(RuntimeError::GraphPaused {
            step_id,
            reason,
            sync_decision: Box::new(sync_decision),
        })
    }

    pub(super) fn pause_for_sync(
        &mut self,
        graph: &ExecutionGraph,
        step_id: String,
        reason: String,
        sync_decision: FanoutSyncDecision,
    ) -> Result<bool, RuntimeError> {
        self.push_sync_point(graph, &sync_decision)?;
        self.pause_graph(step_id, reason, sync_decision)
    }

    pub(super) fn escalate_graph(
        &mut self,
        step_id: String,
        reason: String,
        sync_decision: runx_core::state_machine::FanoutSyncDecision,
    ) -> Result<bool, RuntimeError> {
        self.state = transition_sequential_graph(
            &self.state,
            &SequentialGraphEvent::EscalateGraph {
                reason: reason.clone(),
            },
        );
        Err(RuntimeError::GraphEscalated {
            step_id,
            reason,
            sync_decision: Box::new(sync_decision),
        })
    }

    pub(super) fn escalate_for_sync(
        &mut self,
        graph: &ExecutionGraph,
        step_id: String,
        reason: String,
        sync_decision: FanoutSyncDecision,
    ) -> Result<bool, RuntimeError> {
        self.push_sync_point(graph, &sync_decision)?;
        self.escalate_graph(step_id, reason, sync_decision)
    }

    pub(super) fn run_one_step<A>(
        &mut self,
        runtime: &Runtime<A>,
        graph_dir: &Path,
        graph: &ExecutionGraph,
        step_id: &str,
        attempt: u32,
        host: &mut dyn Host,
    ) -> Result<(), RuntimeError>
    where
        A: SkillAdapter,
    {
        self.run_one_step_with_mode(
            runtime,
            graph_dir,
            graph,
            host,
            StepExecutionPlan {
                step_id,
                attempt,
                failure_mode: StepFailureMode::Propagate,
            },
        )
    }

    pub(super) fn run_one_step_with_mode<A>(
        &mut self,
        runtime: &Runtime<A>,
        graph_dir: &Path,
        graph: &ExecutionGraph,
        host: &mut dyn Host,
        plan: StepExecutionPlan<'_>,
    ) -> Result<(), RuntimeError>
    where
        A: SkillAdapter,
    {
        let step = self.find_step(graph, plan.step_id)?;
        enforce_transition_gates(graph, step, &self.runs)?;
        self.record(host, started_event(plan.step_id))?;
        self.start_step(runtime, plan.step_id);
        let run = match run_step(
            runtime,
            graph_dir,
            &graph.name,
            step,
            plan.attempt,
            &self.runs,
            host,
        ) {
            Ok(run) => run,
            Err(error) if plan.failure_mode == StepFailureMode::RecordAndContinue => {
                runtime_error_step_run(runtime, &graph.name, step, plan.attempt, error)?
            }
            Err(error) => return Err(error),
        };
        if run.output.succeeded() {
            self.succeed_step(runtime, plan.step_id, &run);
            self.runs.push(run);
            self.record(host, completed_event(plan.step_id))
        } else {
            self.fail_step(runtime, plan.step_id, &run);
            host.log(format!("step {} failed", plan.step_id))?;
            self.record(host, failed_event(plan.step_id))?;
            if plan.failure_mode == StepFailureMode::RecordAndContinue {
                self.runs.push(run);
                Ok(())
            } else {
                Err(RuntimeError::SkillFailed {
                    skill_name: plan.step_id.to_owned(),
                    message: run.output.stderr.clone(),
                })
            }
        }
    }

    pub(super) fn start_step<A>(&mut self, runtime: &Runtime<A>, step_id: &str) {
        self.state = transition_sequential_graph(
            &self.state,
            &SequentialGraphEvent::StartStep {
                step_id: step_id.to_owned(),
                at: runtime.options.created_at.clone(),
            },
        );
    }

    pub(super) fn succeed_step<A>(&mut self, runtime: &Runtime<A>, step_id: &str, run: &StepRun) {
        self.state = transition_sequential_graph(
            &self.state,
            &SequentialGraphEvent::StepSucceeded {
                step_id: step_id.to_owned(),
                at: runtime.options.created_at.clone(),
                receipt_id: run.receipt.id.to_string(),
                admission_witness: Box::new(run.admission_witness.clone()),
                outputs: Some(run.outputs.clone()),
            },
        );
    }

    pub(super) fn fail_step<A>(&mut self, runtime: &Runtime<A>, step_id: &str, run: &StepRun) {
        self.state = transition_sequential_graph(
            &self.state,
            &SequentialGraphEvent::StepFailed {
                step_id: step_id.to_owned(),
                at: runtime.options.created_at.clone(),
                error: output_error(run),
            },
        );
    }

    pub(super) fn record(
        &mut self,
        host: &mut dyn Host,
        event: ExecutionEvent,
    ) -> Result<(), RuntimeError> {
        self.journal.push(event.clone());
        host.report(event)
    }

    pub(super) fn finish(
        self,
        graph: ExecutionGraph,
        receipt: runx_contracts::Receipt,
    ) -> GraphRun {
        GraphRun {
            graph,
            state: self.state,
            steps: self.runs,
            sync_points: self.sync_points,
            receipt,
            journal: self.journal,
        }
    }

    pub(super) fn checkpoint(self, graph_name: String) -> GraphCheckpoint {
        GraphCheckpoint {
            graph_name,
            state: self.state,
            steps: self.runs,
            sync_points: self.sync_points,
            journal: self.journal,
        }
    }

    pub(super) fn record_proceeding_fanout_sync_point(
        &mut self,
        graph: &ExecutionGraph,
        fanout_policies: &BTreeMap<String, FanoutGroupPolicy>,
        group_id: &str,
    ) -> Result<(), RuntimeError> {
        let follow_up =
            plan_sequential_graph_transition(&self.state, &self.definitions, fanout_policies, None);
        if matches!(
            follow_up,
            SequentialGraphPlan::RunFanout {
                group_id: ref next_group_id,
                ..
            } if next_group_id == group_id
        ) {
            return Ok(());
        }

        let Some(policy) = fanout_policies.get(group_id) else {
            return Ok(());
        };
        let decision = evaluate_fanout_sync(
            policy,
            &self.branch_results(graph, group_id, fanout_policy_requires_outputs(policy)),
            None,
        );
        if decision.decision == FanoutSyncOutcome::Proceed {
            self.push_sync_point(graph, &decision)?;
        }
        Ok(())
    }

    pub(super) fn push_sync_point(
        &mut self,
        graph: &ExecutionGraph,
        decision: &FanoutSyncDecision,
    ) -> Result<(), RuntimeError> {
        let sync_point = fanout_sync_point(
            decision,
            &latest_fanout_receipt_ids(&self.runs, graph, &decision.group_id),
        );
        let already_recorded = self.sync_points.iter().any(|existing| {
            existing.group_id == sync_point.group_id
                && existing.rule_fired == sync_point.rule_fired
                && existing.decision == sync_point.decision
        });
        if !already_recorded {
            self.sync_points.push(sync_point);
        }
        Ok(())
    }

    pub(super) fn branch_results(
        &self,
        graph: &ExecutionGraph,
        group_id: &str,
        include_outputs: bool,
    ) -> Vec<FanoutBranchResult> {
        graph
            .steps
            .iter()
            .filter(|step| step.fanout_group.as_deref() == Some(group_id))
            .map(|step| {
                let state = self
                    .state
                    .steps
                    .iter()
                    .find(|candidate| candidate.step_id == step.id);
                FanoutBranchResult {
                    step_id: step.id.clone(),
                    status: state.map_or(GraphStepStatus::Failed, |state| state.status.clone()),
                    outputs: if include_outputs {
                        state.and_then(|state| state.outputs.clone())
                    } else {
                        None
                    },
                }
            })
            .collect()
    }

    fn find_step<'a>(
        &self,
        graph: &'a ExecutionGraph,
        step_id: &str,
    ) -> Result<&'a GraphStep, RuntimeError> {
        if let Some(step) = self
            .step_indexes
            .get(step_id)
            .and_then(|index| graph.steps.get(*index))
            .filter(|step| step.id == step_id)
        {
            return Ok(step);
        }
        find_step(graph, step_id)
    }
}

fn step_indexes(graph: &ExecutionGraph) -> BTreeMap<String, usize> {
    graph
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| (step.id.clone(), index))
        .collect()
}

fn fanout_policy_requires_outputs(policy: &FanoutGroupPolicy) -> bool {
    policy
        .threshold_gates
        .as_ref()
        .is_some_and(|gates| !gates.is_empty())
        || policy
            .conflict_gates
            .as_ref()
            .is_some_and(|gates| !gates.is_empty())
}

pub(super) fn reached_step_limit(
    initial: usize,
    current: usize,
    max_new_steps: Option<usize>,
) -> bool {
    max_new_steps.is_some_and(|max| current.saturating_sub(initial) >= max)
}

pub(super) fn enforce_transition_gates(
    graph: &ExecutionGraph,
    step: &GraphStep,
    runs: &[StepRun],
) -> Result<(), RuntimeError> {
    let Some(policy) = &graph.policy else {
        return Ok(());
    };
    for gate in policy.transitions.iter().filter(|gate| gate.to == step.id) {
        let Some(value) = transition_field_value(&gate.field, runs) else {
            return Err(RuntimeError::GraphBlocked {
                step_id: step.id.clone(),
                reason: format!("transition gate '{}' is unresolved", gate.field),
            });
        };
        if let Some(expected) = &gate.equals
            && value != expected
        {
            return Err(RuntimeError::GraphBlocked {
                step_id: step.id.clone(),
                reason: format!(
                    "transition gate '{}' expected {}",
                    gate.field,
                    display_json(expected)
                ),
            });
        }
        if let Some(disallowed) = &gate.not_equals
            && value == disallowed
        {
            return Err(RuntimeError::GraphBlocked {
                step_id: step.id.clone(),
                reason: format!(
                    "transition gate '{}' must not equal {}",
                    gate.field,
                    display_json(disallowed)
                ),
            });
        }
        if gate.equals.is_none() && gate.not_equals.is_none() {
            return Err(RuntimeError::GraphBlocked {
                step_id: step.id.clone(),
                reason: format!("transition gate '{}' has no comparison", gate.field),
            });
        }
    }
    Ok(())
}

pub(super) fn transition_field_value<'a>(
    field: &str,
    runs: &'a [StepRun],
) -> Option<&'a JsonValue> {
    let mut segments = field.split('.');
    let step_id = segments.next()?;
    let run = runs.iter().rev().find(|run| run.step_id == step_id)?;
    let first = segments.next()?;
    if run.outputs.contains_key("skill_claim") && first != "status" {
        return None;
    }
    let mut value = run.outputs.get(first)?;
    for segment in segments {
        let JsonValue::Object(object) = value else {
            return None;
        };
        value = object.get(segment)?;
    }
    Some(value)
}

pub(super) fn display_json(value: &JsonValue) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<unprintable>".to_owned())
}
