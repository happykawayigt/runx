use std::collections::BTreeSet;

use runx_contracts::JsonValue;

use super::super::types::{
    FanoutBranchResult, FanoutGate, FanoutGroupPolicy, FanoutSyncDecision, FanoutSyncOutcome,
    FanoutThresholdGate, GraphStepStatus,
};
use super::values::{json_value_as_f64, resolve_structured_field};
use super::{Counts, DecisionDetails, is_resolved, sync_decision};

pub(super) fn threshold_decision(
    policy: &FanoutGroupPolicy,
    results: &[FanoutBranchResult],
    resolved_gate_keys: Option<&BTreeSet<String>>,
    counts: Counts,
) -> Option<FanoutSyncDecision> {
    for gate in policy.threshold_gates.as_deref().unwrap_or(&[]) {
        let Some(above) = gate.above.as_f64() else {
            continue;
        };
        let Some(result) = results
            .iter()
            .find(|candidate| candidate.step_id == gate.step)
        else {
            continue;
        };
        if result.status != GraphStepStatus::Succeeded {
            continue;
        }
        let Some(value) = resolve_structured_field(result.outputs.as_ref(), &gate.field) else {
            return Some(threshold_missing_decision(policy, gate, counts));
        };
        let Some(number) = json_value_as_f64(value) else {
            return Some(threshold_non_numeric_decision(
                policy,
                gate,
                value.clone(),
                counts,
            ));
        };
        if number > above {
            let decision = threshold_above_decision(policy, gate, value.clone(), number, counts);
            if !is_resolved(&decision, resolved_gate_keys) {
                return Some(decision);
            }
        }
    }
    None
}

fn threshold_missing_decision(
    policy: &FanoutGroupPolicy,
    gate: &FanoutThresholdGate,
    counts: Counts,
) -> FanoutSyncDecision {
    sync_decision(
        policy,
        FanoutSyncOutcome::Halt,
        "threshold",
        counts,
        DecisionDetails::new(
            format!("threshold.{}.{}.missing", gate.step, gate.field),
            format!(
                "threshold field {}.{} was not produced",
                gate.step, gate.field
            ),
        )
        .with_gate(FanoutGate::Threshold {
            step_id: Some(gate.step.clone()),
            field: gate.field.clone(),
            value: None,
            compared_to: None,
            action: gate.action.clone(),
        }),
    )
}

fn threshold_non_numeric_decision(
    policy: &FanoutGroupPolicy,
    gate: &FanoutThresholdGate,
    value: JsonValue,
    counts: Counts,
) -> FanoutSyncDecision {
    sync_decision(
        policy,
        FanoutSyncOutcome::Halt,
        "threshold",
        counts,
        DecisionDetails::new(
            format!("threshold.{}.{}.non_numeric", gate.step, gate.field),
            format!(
                "threshold field {}.{} must be numeric",
                gate.step, gate.field
            ),
        )
        .with_gate(FanoutGate::Threshold {
            step_id: Some(gate.step.clone()),
            field: gate.field.clone(),
            value: Some(value),
            compared_to: None,
            action: gate.action.clone(),
        }),
    )
}

fn threshold_above_decision(
    policy: &FanoutGroupPolicy,
    gate: &FanoutThresholdGate,
    value: JsonValue,
    number: f64,
    counts: Counts,
) -> FanoutSyncDecision {
    sync_decision(
        policy,
        gate.action.clone().into(),
        "threshold",
        counts,
        DecisionDetails::new(
            format!("threshold.{}.{}.above", gate.step, gate.field),
            format!(
                "{}.{}={number} exceeded {}",
                gate.step, gate.field, gate.above
            ),
        )
        .with_gate(FanoutGate::Threshold {
            step_id: Some(gate.step.clone()),
            field: gate.field.clone(),
            value: Some(value),
            compared_to: Some(gate.above.clone()),
            action: gate.action.clone(),
        }),
    )
}
