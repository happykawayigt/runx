use runx_contracts::JsonValue;
use runx_core::kernel_eval::{KernelEvalOutput, evaluate_kernel_document_str};

#[test]
fn evaluates_policy_fixture_document() -> Result<(), Box<dyn std::error::Error>> {
    let output = evaluate_kernel_document_str(include_str!(
        "../../../fixtures/kernel/policy/retry-admission-denies-mutating-without-key.json"
    ))?;

    let KernelEvalOutput::Output { value } = output;
    assert_eq!(
        value,
        json_value(
            r#"{
            "status": "deny",
            "reasons": ["step 'deploy' declares mutating retry without an idempotency key"]
        }"#
        )?
    );
    Ok(())
}

#[test]
fn evaluates_state_machine_fixture_document() -> Result<(), Box<dyn std::error::Error>> {
    let output = evaluate_kernel_document_str(include_str!(
        "../../../fixtures/kernel/state-machine/sequential-plan-first-step.json"
    ))?;

    let KernelEvalOutput::Output { value } = output;
    assert_eq!(
        value,
        json_value(
            r#"{
            "type": "run_step",
            "stepId": "first",
            "attempt": 1,
            "contextFrom": []
        }"#
        )?
    );
    Ok(())
}

#[test]
fn evaluates_raw_input_document() -> Result<(), Box<dyn std::error::Error>> {
    let output = evaluate_kernel_document_str(
        r#"{"kind":"state-machine.createSingleStepState","stepId":"only"}"#,
    )?;

    let KernelEvalOutput::Output { value } = output;
    assert_eq!(
        value,
        json_value(
            r#"{
            "stepId": "only",
            "status": "pending"
        }"#
        )?
    );
    Ok(())
}

fn json_value(source: &str) -> Result<JsonValue, serde_json::Error> {
    serde_json::from_str(source)
}
