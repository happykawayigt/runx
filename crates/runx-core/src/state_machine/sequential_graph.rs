// rust-style-allow: large-file because graph planning parity keeps linear and
// fanout planning helpers together until policy parity lands.
use std::collections::{BTreeMap, BTreeSet};

use super::fanout::evaluate_fanout_sync;
use super::types::{
    FanoutBranchFailurePolicy, FanoutBranchResult, FanoutGroupPolicy, FanoutSyncDecision,
    FanoutSyncOutcome, FanoutSyncStrategy, GraphStatus, GraphStepStatus, SequentialGraphEvent,
    SequentialGraphPlan, SequentialGraphState, SequentialGraphStepDefinition,
    SequentialGraphStepState,
};

#[must_use]
pub fn create_sequential_graph_state(
    graph_id: impl Into<String>,
    steps: &[SequentialGraphStepDefinition],
) -> SequentialGraphState {
    SequentialGraphState {
        graph_id: graph_id.into(),
        status: GraphStatus::Pending,
        steps: steps
            .iter()
            .map(|step| SequentialGraphStepState {
                step_id: step.id.clone(),
                status: GraphStepStatus::Pending,
                attempts: 0,
                started_at: None,
                completed_at: None,
                receipt_id: None,
                outputs: None,
                error: None,
            })
            .collect(),
    }
}

#[must_use]
pub fn plan_sequential_graph_transition(
    state: &SequentialGraphState,
    steps: &[SequentialGraphStepDefinition],
    fanout_policies: &BTreeMap<String, FanoutGroupPolicy>,
    resolved_fanout_gate_keys: Option<&BTreeSet<String>>,
) -> SequentialGraphPlan {
    if let Some(running_step) = state
        .steps
        .iter()
        .find(|step| step.status == GraphStepStatus::Running)
    {
        return SequentialGraphPlan::Blocked {
            step_id: running_step.step_id.clone(),
            reason: "step is already running".to_owned(),
            sync_decision: None,
        };
    }

    let mut index = 0;
    while index < steps.len() {
        let step_definition = &steps[index];
        if let Some(group_id) = fanout_group_id(step_definition) {
            let group_steps = collect_contiguous_fanout_group(steps, index, group_id);
            match plan_fanout_group(
                state,
                &group_steps,
                fanout_policies.get(group_id),
                resolved_fanout_gate_keys,
            ) {
                FanoutGroupPlan::Proceed => {
                    index += group_steps.len();
                    continue;
                }
                FanoutGroupPlan::Plan(plan) => return *plan,
            }
        }

        if let Some(plan) = plan_step(state, step_definition) {
            return plan;
        }
        index += 1;
    }

    SequentialGraphPlan::Complete
}

#[must_use]
pub fn transition_sequential_graph(
    state: &SequentialGraphState,
    event: &SequentialGraphEvent,
) -> SequentialGraphState {
    match event {
        SequentialGraphEvent::StartStep { step_id, at } => update_step(
            state,
            step_id,
            |step| start_step(step, at),
            GraphStatus::Running,
        ),
        SequentialGraphEvent::StepSucceeded {
            step_id,
            at,
            receipt_id,
            admission_witness,
            outputs,
        } => update_step(
            state,
            step_id,
            |step| succeed_step(step, at, receipt_id, admission_witness, outputs.clone()),
            state.status.clone(),
        ),
        SequentialGraphEvent::StepFailed { step_id, at, error } => update_step(
            state,
            step_id,
            |step| fail_step(step, at, error),
            state.status.clone(),
        ),
        SequentialGraphEvent::Complete if is_graph_complete(state) => SequentialGraphState {
            status: GraphStatus::Succeeded,
            ..state.clone()
        },
        SequentialGraphEvent::Complete => state.clone(),
        SequentialGraphEvent::PauseGraph { .. } => SequentialGraphState {
            status: GraphStatus::Paused,
            ..state.clone()
        },
        SequentialGraphEvent::EscalateGraph { .. } => SequentialGraphState {
            status: GraphStatus::Escalated,
            ..state.clone()
        },
        SequentialGraphEvent::FailGraph { .. } => SequentialGraphState {
            status: GraphStatus::Failed,
            ..state.clone()
        },
    }
}

enum FanoutGroupPlan {
    Proceed,
    Plan(Box<SequentialGraphPlan>),
}

enum FanoutCandidatePlan {
    Plan(Box<SequentialGraphPlan>),
    ProceedToSync,
}

enum NonProceedFanoutDecision {
    Halt(FanoutSyncDecision),
    Pause(FanoutSyncDecision),
    Escalate(FanoutSyncDecision),
}

fn plan_step(
    state: &SequentialGraphState,
    step_definition: &SequentialGraphStepDefinition,
) -> Option<SequentialGraphPlan> {
    let Some(step_state) = find_step_state(state, &step_definition.id) else {
        return Some(SequentialGraphPlan::Failed {
            step_id: step_definition.id.clone(),
            reason: "step state is missing".to_owned(),
            sync_decision: None,
        });
    };

    if step_state.status == GraphStepStatus::Succeeded {
        return None;
    }
    if retry_budget_exhausted(step_state, step_definition) {
        return Some(SequentialGraphPlan::Failed {
            step_id: step_definition.id.clone(),
            reason: "step failed and retry budget is exhausted".to_owned(),
            sync_decision: None,
        });
    }
    if let Some(missing_context) = missing_context(state, step_definition) {
        return Some(SequentialGraphPlan::Blocked {
            step_id: step_definition.id.clone(),
            reason: format!("waiting for context from {missing_context}"),
            sync_decision: None,
        });
    }

    Some(SequentialGraphPlan::RunStep {
        step_id: step_definition.id.clone(),
        attempt: step_state.attempts + 1,
        context_from: step_definition.context_from.clone().unwrap_or_default(),
    })
}

fn plan_fanout_group(
    state: &SequentialGraphState,
    group_steps: &[SequentialGraphStepDefinition],
    policy: Option<&FanoutGroupPolicy>,
    resolved_fanout_gate_keys: Option<&BTreeSet<String>>,
) -> FanoutGroupPlan {
    let Some(first_step) = group_steps.first() else {
        return FanoutGroupPlan::Plan(Box::new(SequentialGraphPlan::Failed {
            step_id: "unknown".to_owned(),
            reason: "fanout group is empty".to_owned(),
            sync_decision: None,
        }));
    };
    let Some(group_id) = fanout_group_id(first_step) else {
        return FanoutGroupPlan::Plan(Box::new(SequentialGraphPlan::Failed {
            step_id: first_step.id.clone(),
            reason: "fanout group is empty".to_owned(),
            sync_decision: None,
        }));
    };

    match plan_fanout_candidates(state, group_steps, group_id) {
        FanoutCandidatePlan::Plan(plan) => return FanoutGroupPlan::Plan(plan),
        FanoutCandidatePlan::ProceedToSync => {}
    }

    let fanout_policy = policy
        .cloned()
        .unwrap_or_else(|| default_fanout_policy(group_id));
    let decision = evaluate_fanout_sync(
        &fanout_policy,
        &fanout_results(state, group_steps),
        resolved_fanout_gate_keys,
    );
    let Some(non_proceed_decision) = non_proceed_fanout_decision(decision) else {
        return FanoutGroupPlan::Proceed;
    };

    FanoutGroupPlan::Plan(Box::new(sync_decision_plan(
        first_step,
        non_proceed_decision,
    )))
}

fn plan_fanout_candidates(
    state: &SequentialGraphState,
    group_steps: &[SequentialGraphStepDefinition],
    group_id: &str,
) -> FanoutCandidatePlan {
    let mut step_ids = Vec::new();
    let mut attempts = BTreeMap::new();
    let mut context_from = BTreeMap::new();

    for step_definition in group_steps {
        let Some(step_state) = find_step_state(state, &step_definition.id) else {
            return FanoutCandidatePlan::Plan(Box::new(SequentialGraphPlan::Failed {
                step_id: step_definition.id.clone(),
                reason: "step state is missing".to_owned(),
                sync_decision: None,
            }));
        };
        if step_state.status == GraphStepStatus::Succeeded
            || retry_budget_exhausted(step_state, step_definition)
        {
            continue;
        }
        if let Some(missing_context) = missing_context(state, step_definition) {
            return FanoutCandidatePlan::Plan(Box::new(SequentialGraphPlan::Blocked {
                step_id: step_definition.id.clone(),
                reason: format!("waiting for context from {missing_context}"),
                sync_decision: None,
            }));
        }
        step_ids.push(step_definition.id.clone());
        attempts.insert(step_definition.id.clone(), step_state.attempts + 1);
        context_from.insert(
            step_definition.id.clone(),
            step_definition.context_from.clone().unwrap_or_default(),
        );
    }

    if step_ids.is_empty() {
        FanoutCandidatePlan::ProceedToSync
    } else {
        FanoutCandidatePlan::Plan(Box::new(SequentialGraphPlan::RunFanout {
            group_id: group_id.to_owned(),
            step_ids,
            attempts,
            context_from,
        }))
    }
}

fn sync_decision_plan(
    first_step: &SequentialGraphStepDefinition,
    decision: NonProceedFanoutDecision,
) -> SequentialGraphPlan {
    match decision {
        NonProceedFanoutDecision::Halt(decision) => SequentialGraphPlan::Failed {
            step_id: first_step.id.clone(),
            reason: decision.reason.clone(),
            sync_decision: Some(decision),
        },
        NonProceedFanoutDecision::Pause(decision) => SequentialGraphPlan::Paused {
            step_id: first_step.id.clone(),
            reason: decision.reason.clone(),
            sync_decision: decision,
        },
        NonProceedFanoutDecision::Escalate(decision) => SequentialGraphPlan::Escalated {
            step_id: first_step.id.clone(),
            reason: decision.reason.clone(),
            sync_decision: decision,
        },
    }
}

fn non_proceed_fanout_decision(decision: FanoutSyncDecision) -> Option<NonProceedFanoutDecision> {
    match decision.decision {
        FanoutSyncOutcome::Proceed => None,
        FanoutSyncOutcome::Halt => Some(NonProceedFanoutDecision::Halt(decision)),
        FanoutSyncOutcome::Pause => Some(NonProceedFanoutDecision::Pause(decision)),
        FanoutSyncOutcome::Escalate => Some(NonProceedFanoutDecision::Escalate(decision)),
    }
}

fn collect_contiguous_fanout_group(
    steps: &[SequentialGraphStepDefinition],
    start_index: usize,
    group_id: &str,
) -> Vec<SequentialGraphStepDefinition> {
    steps
        .iter()
        .skip(start_index)
        .take_while(|step| fanout_group_id(step) == Some(group_id))
        .cloned()
        .collect()
}

fn fanout_group_id(step: &SequentialGraphStepDefinition) -> Option<&str> {
    step.fanout_group
        .as_deref()
        .filter(|group_id| !group_id.is_empty())
}

fn fanout_results(
    state: &SequentialGraphState,
    group_steps: &[SequentialGraphStepDefinition],
) -> Vec<FanoutBranchResult> {
    group_steps
        .iter()
        .map(|step| {
            let step_state = find_step_state(state, &step.id);
            FanoutBranchResult {
                step_id: step.id.clone(),
                status: step_state.map_or(GraphStepStatus::Failed, |state| state.status.clone()),
                outputs: step_state.and_then(|state| state.outputs.clone()),
            }
        })
        .collect()
}

fn start_step(step: &SequentialGraphStepState, at: &str) -> SequentialGraphStepState {
    if matches!(
        step.status,
        GraphStepStatus::Running | GraphStepStatus::Succeeded
    ) {
        return step.clone();
    }
    SequentialGraphStepState {
        status: GraphStepStatus::Running,
        attempts: step.attempts + 1,
        started_at: Some(at.to_owned()),
        completed_at: None,
        outputs: None,
        error: None,
        ..step.clone()
    }
}

fn succeed_step(
    step: &SequentialGraphStepState,
    at: &str,
    receipt_id: &str,
    admission_witness: &super::types::StepAdmissionWitness,
    outputs: Option<runx_contracts::JsonObject>,
) -> SequentialGraphStepState {
    if step.status != GraphStepStatus::Running {
        return step.clone();
    }
    if !admission_witness.matches_step_receipt(&step.step_id, receipt_id) {
        return step.clone();
    }
    SequentialGraphStepState {
        status: GraphStepStatus::Succeeded,
        completed_at: Some(at.to_owned()),
        receipt_id: Some(receipt_id.to_owned()),
        outputs,
        error: None,
        ..step.clone()
    }
}

fn fail_step(step: &SequentialGraphStepState, at: &str, error: &str) -> SequentialGraphStepState {
    if step.status != GraphStepStatus::Running {
        return step.clone();
    }
    SequentialGraphStepState {
        status: GraphStepStatus::Failed,
        completed_at: Some(at.to_owned()),
        outputs: None,
        error: Some(error.to_owned()),
        ..step.clone()
    }
}

fn update_step(
    state: &SequentialGraphState,
    step_id: &str,
    update: impl Fn(&SequentialGraphStepState) -> SequentialGraphStepState,
    next_status: GraphStatus,
) -> SequentialGraphState {
    SequentialGraphState {
        graph_id: state.graph_id.clone(),
        status: next_status,
        steps: state
            .steps
            .iter()
            .map(|step| {
                if step.step_id == step_id {
                    update(step)
                } else {
                    step.clone()
                }
            })
            .collect(),
    }
}

fn default_fanout_policy(group_id: &str) -> FanoutGroupPolicy {
    FanoutGroupPolicy {
        group_id: group_id.to_owned(),
        strategy: FanoutSyncStrategy::All,
        min_success: None,
        on_branch_failure: FanoutBranchFailurePolicy::Halt,
        threshold_gates: Some(Vec::new()),
        conflict_gates: Some(Vec::new()),
    }
}

fn retry_budget_exhausted(
    step_state: &SequentialGraphStepState,
    step_definition: &SequentialGraphStepDefinition,
) -> bool {
    step_state.status == GraphStepStatus::Failed
        && step_state.attempts
            >= step_definition
                .retry
                .as_ref()
                .map_or(1, |retry| retry.max_attempts)
}

fn missing_context(
    state: &SequentialGraphState,
    step_definition: &SequentialGraphStepDefinition,
) -> Option<String> {
    step_definition
        .context_from
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .find(|step_id| {
            find_step_state(state, step_id)
                .is_none_or(|step| step.status != GraphStepStatus::Succeeded)
        })
        .cloned()
}

fn is_graph_complete(state: &SequentialGraphState) -> bool {
    state.steps.iter().all(|step| {
        !matches!(
            step.status,
            GraphStepStatus::Pending | GraphStepStatus::Running
        )
    })
}

fn find_step_state<'a>(
    state: &'a SequentialGraphState,
    step_id: &str,
) -> Option<&'a SequentialGraphStepState> {
    state.steps.iter().find(|step| step.step_id == step_id)
}
