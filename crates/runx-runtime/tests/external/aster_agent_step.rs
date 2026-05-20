use std::path::{Path, PathBuf};

use runx_contracts::{ClosureDisposition, JsonObject, JsonValue};
use runx_receipts::validate_harness_receipt;
use runx_runtime::{
    HarnessExpectedStatus, HarnessReplayOutput, InvocationStatus, RuntimeOptions, SkillAdapter,
    SkillInvocation, SkillOutput, load_harness_fixture, run_harness_fixture_with_adapter,
};

#[test]
fn aster_agent_step_replays_current_rust_bridge_terminal_report()
-> Result<(), Box<dyn std::error::Error>> {
    let output = run_case()?;

    assert_eq!(output.status, HarnessExpectedStatus::Sealed);
    assert_eq!(output.receipt.seal.disposition, ClosureDisposition::Closed);
    validate_harness_receipt(&output.receipt)
        .map_err(|verification| format!("{:?}", verification.findings))?;

    let payload = skill_payload(&output)?;
    assert_no_retired_bridge_fields(&payload, "report")?;
    assert_eq!(string_field_value(&payload, "schema")?, "runx.skill_run.v1");
    assert_eq!(string_field_value(&payload, "status")?, "sealed");
    assert_eq!(
        string_field_value(&payload, "receipt_id")?,
        "hrn_rcpt_aster_issue_triage_14"
    );
    assert_eq!(
        string_field_value(&payload, "run_id")?,
        "run_aster_issue_triage_14"
    );

    let receipt = object_field_value(&payload, "receipt")?;
    assert_eq!(string_field(receipt, "schema")?, "runx.harness_receipt.v1");
    assert_eq!(
        string_field(receipt, "id")?,
        "hrn_rcpt_aster_issue_triage_14"
    );
    let harness = object_field(receipt, "harness")?;
    assert_eq!(string_field(harness, "state")?, "sealed");
    let seal = object_field(receipt, "seal")?;
    assert_eq!(string_field(seal, "disposition")?, "closed");

    let skill_output = output
        .skill_output
        .as_ref()
        .ok_or("agent-step fixture did not produce skill output")?;
    assert_eq!(
        string_field(&skill_output.metadata, "agent_request_id")?,
        "agent_step.aster-rust-bridge.output"
    );

    Ok(())
}

#[test]
fn aster_external_fixture_records_grounded_bridge_sources() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = load_harness_fixture(case_path())?;

    assert_eq!(
        fixture.metadata.get("external_project"),
        Some(&JsonValue::String("aster".to_owned()))
    );
    assert_eq!(
        fixture.metadata.get("bridge_contract"),
        Some(&JsonValue::String("runx.skill_run.v1".to_owned()))
    );
    assert_eq!(
        fixture.metadata.get("receipt_contract"),
        Some(&JsonValue::String("runx.harness_receipt.v1".to_owned()))
    );

    let inputs = JsonValue::Object(fixture.inputs);
    let bridge = object_field_value(&inputs, "bridge")?;
    let accepted_command = array_field(bridge, "accepted_command")?;
    assert_eq!(
        accepted_command.first(),
        Some(&JsonValue::String("skill".to_owned()))
    );
    assert!(
        accepted_command
            .iter()
            .any(|value| value == &JsonValue::String("--runner".to_owned()))
    );

    Ok(())
}

fn run_case() -> Result<HarnessReplayOutput, runx_runtime::HarnessReplayError> {
    run_harness_fixture_with_adapter(case_path(), NoopAdapter, RuntimeOptions::default())
}

fn skill_payload(output: &HarnessReplayOutput) -> Result<JsonValue, Box<dyn std::error::Error>> {
    let skill_output = output
        .skill_output
        .as_ref()
        .ok_or("agent-step fixture did not produce skill output")?;
    Ok(serde_json::from_str(&skill_output.stdout)?)
}

fn object_field_value<'a>(
    value: &'a JsonValue,
    field: &str,
) -> Result<&'a JsonObject, Box<dyn std::error::Error>> {
    let JsonValue::Object(object) = value else {
        return Err("value is not an object".into());
    };
    object_field(object, field)
}

fn object_field<'a>(
    object: &'a JsonObject,
    field: &str,
) -> Result<&'a JsonObject, Box<dyn std::error::Error>> {
    match object.get(field) {
        Some(JsonValue::Object(value)) => Ok(value),
        Some(_) => Err(format!("{field} is not an object").into()),
        None => Err(format!("{field} is missing").into()),
    }
}

fn array_field<'a>(
    object: &'a JsonObject,
    field: &str,
) -> Result<&'a Vec<JsonValue>, Box<dyn std::error::Error>> {
    match object.get(field) {
        Some(JsonValue::Array(value)) => Ok(value),
        Some(_) => Err(format!("{field} is not an array").into()),
        None => Err(format!("{field} is missing").into()),
    }
}

fn string_field_value<'a>(
    value: &'a JsonValue,
    field: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    let JsonValue::Object(object) = value else {
        return Err("value is not an object".into());
    };
    string_field(object, field)
}

fn string_field<'a>(
    object: &'a JsonObject,
    field: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    match object.get(field) {
        Some(JsonValue::String(value)) => Ok(value),
        Some(_) => Err(format!("{field} is not a string").into()),
        None => Err(format!("{field} is missing").into()),
    }
}

fn assert_no_retired_bridge_fields(
    value: &JsonValue,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match value {
        JsonValue::Object(object) => {
            for (key, child) in object {
                if is_retired_bridge_field(key) {
                    return Err(format!("retired bridge field {path}.{key}").into());
                }
                assert_no_retired_bridge_fields(child, &format!("{path}.{key}"))?;
            }
        }
        JsonValue::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                assert_no_retired_bridge_fields(child, &format!("{path}.{index}"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_retired_bridge_field(field: &str) -> bool {
    matches!(
        field,
        "runId"
            | "receiptId"
            | "outcome"
            | "effect"
            | "issue_to_pr_outcome"
            | "verification_report"
            | "verificationReport"
            | "target_effect"
            | "targetEffect"
    )
}

fn case_path() -> PathBuf {
    repo_root().join("fixtures/external/aster/agent-step/rust-bridge-sealed-skill.yaml")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct NoopAdapter;

impl SkillAdapter for NoopAdapter {
    fn adapter_type(&self) -> &'static str {
        "noop"
    }

    fn invoke(&self, _request: SkillInvocation) -> Result<SkillOutput, runx_runtime::RuntimeError> {
        Ok(SkillOutput {
            status: InvocationStatus::Success,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
            duration_ms: 0,
            metadata: Default::default(),
        })
    }
}
