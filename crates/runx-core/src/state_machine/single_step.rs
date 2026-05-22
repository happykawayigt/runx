use super::types::{SingleStepEvent, SingleStepState, StepStatus};

#[must_use]
pub fn create_single_step_state(step_id: impl Into<String>) -> SingleStepState {
    SingleStepState {
        step_id: step_id.into(),
        status: StepStatus::Pending,
        started_at: None,
        completed_at: None,
        error: None,
    }
}

#[must_use]
pub fn transition_single_step(state: &SingleStepState, event: &SingleStepEvent) -> SingleStepState {
    match event {
        SingleStepEvent::Admit if state.status == StepStatus::Pending => {
            let mut next = state.clone();
            next.status = StepStatus::Admitted;
            next
        }
        SingleStepEvent::Start { at } if state.status == StepStatus::Admitted => {
            let mut next = state.clone();
            next.status = StepStatus::Running;
            next.started_at = Some(at.clone());
            next
        }
        SingleStepEvent::Succeed {
            at,
            admission_witness,
        } if state.status == StepStatus::Running
            && admission_witness.step_id == state.step_id
            && !admission_witness.receipt_id.is_empty() =>
        {
            let mut next = state.clone();
            next.status = StepStatus::Succeeded;
            next.completed_at = Some(at.clone());
            next
        }
        SingleStepEvent::Fail { at, error } if state.status == StepStatus::Running => {
            let mut next = state.clone();
            next.status = StepStatus::Failed;
            next.completed_at = Some(at.clone());
            next.error = Some(error.clone());
            next
        }
        _ => state.clone(),
    }
}
