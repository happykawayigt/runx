use std::collections::{BTreeMap, BTreeSet};

use runx_contracts::JsonValue;

use super::super::types::{
    FanoutBranchResult, FanoutConflictGate, FanoutGate, FanoutGroupPolicy, FanoutSyncDecision,
    GraphStepStatus,
};
use super::values::{resolve_structured_field, stable_value};
use super::{Counts, DecisionDetails, is_resolved, sync_decision};

pub(super) fn conflict_decision(
    policy: &FanoutGroupPolicy,
    results: &[FanoutBranchResult],
    resolved_gate_keys: Option<&BTreeSet<String>>,
    counts: Counts,
) -> Option<FanoutSyncDecision> {
    for gate in policy.conflict_gates.as_deref().unwrap_or(&[]) {
        let candidates = conflict_candidates(results, gate);
        let values = conflict_values(candidates.iter().copied(), gate);
        let distinct: BTreeSet<String> = candidates
            .iter()
            .map(|result| {
                resolve_structured_field(result.outputs.as_ref(), &gate.field)
                    .map_or_else(|| "undefined".to_owned(), stable_value)
            })
            .collect();
        if distinct.len() > 1 {
            let decision = sync_decision(
                policy,
                gate.action.clone().into(),
                "conflict",
                counts,
                DecisionDetails::new(
                    format!("conflict.{}", gate.field),
                    format!(
                        "fanout branches disagreed on structured field {}",
                        gate.field
                    ),
                )
                .with_gate(FanoutGate::Conflict {
                    field: gate.field.clone(),
                    values: Some(values),
                    action: gate.action.clone(),
                }),
            );
            if !is_resolved(&decision, resolved_gate_keys) {
                return Some(decision);
            }
        }
    }
    None
}

fn conflict_candidates<'a>(
    results: &'a [FanoutBranchResult],
    gate: &FanoutConflictGate,
) -> Vec<&'a FanoutBranchResult> {
    results
        .iter()
        .filter(|result| {
            result.status == GraphStepStatus::Succeeded
                && (gate.steps.is_empty() || gate.steps.contains(&result.step_id))
        })
        .collect()
}

fn conflict_values<'a>(
    results: impl Iterator<Item = &'a FanoutBranchResult>,
    gate: &FanoutConflictGate,
) -> BTreeMap<String, JsonValue> {
    results
        .filter_map(|result| {
            resolve_structured_field(result.outputs.as_ref(), &gate.field)
                .cloned()
                .map(|value| (result.step_id.clone(), value))
        })
        .collect()
}
