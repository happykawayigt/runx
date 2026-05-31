use std::path::{Path, PathBuf};

use runx_contracts::{ClosureDisposition, JsonObject, JsonValue, ReceiptSchema};
use runx_receipts::canonical_receipt_json;
use runx_runtime::effects::{
    EffectSettlementEvidence, EffectSettlementRequest, EffectSupervisor, EffectSupervisorError,
    RuntimeEffectRegistry,
};
use runx_runtime::payment::supervisor::{
    PAYMENT_RAIL_SUPERVISOR_VERIFIER_ID, PaymentSupervisorError,
    PaymentSupervisorSettlementEvidence, PaymentSupervisorSettlementRequest,
    payment_supervisor_evidence_to_effect_record,
};
use runx_runtime::{
    HarnessExpectedStatus, HarnessFixtureError, HarnessFixtureKind, InvocationStatus,
    RuntimeOptions, SkillAdapter, SkillInvocation, SkillOutput, load_harness_fixture,
    parse_harness_fixture, run_harness_fixture_with_adapter,
};

const FIXTURE_CREATED_AT: &str = "2026-05-18T00:00:00Z";

#[test]
fn loads_active_harness_fixtures_without_retired_receipt_fields() -> Result<(), HarnessFixtureError>
{
    for (path, expected_status, expected_disposition) in [
        (
            "fixtures/harness/echo-skill.yaml",
            HarnessExpectedStatus::Sealed,
            ClosureDisposition::Closed,
        ),
        (
            "fixtures/harness/sequential-graph.yaml",
            HarnessExpectedStatus::Sealed,
            ClosureDisposition::Closed,
        ),
        (
            "fixtures/harness/payment-approval-graph.yaml",
            HarnessExpectedStatus::Sealed,
            ClosureDisposition::Closed,
        ),
        (
            "fixtures/harness/payment-approval-denied.yaml",
            HarnessExpectedStatus::PolicyDenied,
            ClosureDisposition::Blocked,
        ),
        (
            "fixtures/harness/x402-pay-approval.yaml",
            HarnessExpectedStatus::Sealed,
            ClosureDisposition::Closed,
        ),
        (
            "fixtures/harness/x402-pay-approval-denied.yaml",
            HarnessExpectedStatus::PolicyDenied,
            ClosureDisposition::Blocked,
        ),
        (
            "fixtures/harness/x402-pay-paid-echo.yaml",
            HarnessExpectedStatus::Sealed,
            ClosureDisposition::Closed,
        ),
        (
            "fixtures/harness/x402-pay-idempotency-replay.yaml",
            HarnessExpectedStatus::Sealed,
            ClosureDisposition::Closed,
        ),
        (
            "fixtures/harness/x402-pay-idempotency-capability-reuse.yaml",
            HarnessExpectedStatus::PolicyDenied,
            ClosureDisposition::Blocked,
        ),
        (
            "fixtures/harness/x402-pay-idempotency-crash-recovery.yaml",
            HarnessExpectedStatus::Escalated,
            ClosureDisposition::Deferred,
        ),
        (
            "fixtures/harness/stripe-spt-payment.yaml",
            HarnessExpectedStatus::Sealed,
            ClosureDisposition::Closed,
        ),
    ] {
        let fixture = load_harness_fixture(fixture_path(path))?;
        assert_eq!(fixture.expect.status, Some(expected_status));
        let receipt = fixture
            .expect
            .receipt
            .ok_or(HarnessFixtureError::Required {
                field: "expect.receipt".to_owned(),
            })?;
        assert_eq!(receipt.schema, ReceiptSchema::V1);
        // A suspended (deferred) run carries the "deferred" state; every
        // terminal seal carries "sealed".
        let expected_state = if expected_disposition == ClosureDisposition::Deferred {
            "deferred"
        } else {
            "sealed"
        };
        assert_eq!(receipt.state.as_deref(), Some(expected_state));
        assert_eq!(receipt.disposition, Some(expected_disposition));
    }
    Ok(())
}

#[test]
#[cfg(feature = "cli-tool")]
fn replays_x402_idempotency_sequence_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    for (path, expected_status, expected_disposition, expected_steps) in [
        (
            "fixtures/harness/x402-pay-idempotency-replay.yaml",
            HarnessExpectedStatus::Sealed,
            ClosureDisposition::Closed,
            5,
        ),
        (
            "fixtures/harness/x402-pay-idempotency-capability-reuse.yaml",
            HarnessExpectedStatus::PolicyDenied,
            ClosureDisposition::Blocked,
            5,
        ),
        (
            "fixtures/harness/x402-pay-idempotency-crash-recovery.yaml",
            HarnessExpectedStatus::Escalated,
            ClosureDisposition::Deferred,
            0,
        ),
    ] {
        let output = run_fixture_with_test_adapter(path)?;

        assert_eq!(output.status, expected_status, "{path}");
        assert_eq!(
            output.receipt.seal.disposition, expected_disposition,
            "{path}"
        );
        assert_eq!(output.step_receipts.len(), expected_steps, "{path}");
    }
    Ok(())
}

#[test]
fn parses_harness_skill_fixture_contract() -> Result<(), HarnessFixtureError> {
    let fixture = load_harness_fixture(fixture_path("fixtures/harness/echo-skill.yaml"))?;

    assert_eq!(fixture.name, "echo-skill");
    assert_eq!(fixture.kind, HarnessFixtureKind::Skill);
    assert_eq!(fixture.target, "../skills/echo");
    let receipt = fixture
        .expect
        .receipt
        .ok_or(HarnessFixtureError::Required {
            field: "expect.receipt".to_owned(),
        })?;
    assert_eq!(receipt.harness_id.as_deref(), Some("hrn_echo-skill_echo"));
    assert_eq!(receipt.reason_code.as_deref(), Some("process_closed"));
    assert_eq!(receipt.act_ids, vec!["act_echo"]);
    Ok(())
}

#[test]
fn parses_harness_graph_fixture_contract() -> Result<(), HarnessFixtureError> {
    let fixture = load_harness_fixture(fixture_path("fixtures/harness/sequential-graph.yaml"))?;

    assert_eq!(fixture.name, "sequential-graph");
    assert_eq!(fixture.kind, HarnessFixtureKind::Graph);
    assert_eq!(fixture.target, "../graphs/sequential/graph.yaml");
    assert_eq!(fixture.expect.steps, vec!["first", "second"]);
    let receipt = fixture
        .expect
        .receipt
        .ok_or(HarnessFixtureError::Required {
            field: "expect.receipt".to_owned(),
        })?;
    assert_eq!(
        receipt.harness_id.as_deref(),
        Some("hrn_sequential-echo_graph")
    );
    assert_eq!(
        receipt.child_receipt_refs,
        vec![
            "runx:receipt:sha256:3e9617d1d7d0494106096a195a0369ffdfee9e24a54bea74967019339733c569",
            "runx:receipt:sha256:da09438dd433579faf33fc206a4b1183bfafc8ad7b5c03859fb453a6badd4603"
        ]
    );
    Ok(())
}

#[test]
fn rejects_retired_receipt_kind_field_with_stable_path() {
    for field in [
        "kind".to_owned(),
        retired_execution_receipt_field("skill"),
        retired_execution_receipt_field("graph"),
    ] {
        let result = parse_harness_fixture(&format!(
            r#"
name: old
kind: skill
target: ../skills/echo
expect:
  receipt:
    {field}: value
"#,
        ));

        assert!(matches!(
            result,
            Err(HarnessFixtureError::RetiredReceiptField { field_path })
                if field_path == format!("expect.receipt.{field}")
        ));
    }
}

#[test]
fn retired_receipt_expectations_are_rejected() {
    for field in [
        retired_execution_receipt_field("skill"),
        retired_execution_receipt_field("graph"),
        "skill_name".to_owned(),
        "source_type".to_owned(),
        "graph_name".to_owned(),
        "owner".to_owned(),
    ] {
        let result = parse_harness_fixture(&format!(
            r#"
name: old
kind: skill
target: ../skills/echo
expect:
  receipt:
    {field}: value
"#,
        ));

        assert!(matches!(
            result,
            Err(HarnessFixtureError::RetiredReceiptField { field_path })
                if field_path == format!("expect.receipt.{field}")
        ));
    }
}

#[test]
fn rejects_unsupported_fixture_mode_with_stable_path() {
    let result = parse_harness_fixture(
        r#"
name: old
kind: mcp
target: ../skills/echo
expect:
  status: sealed
"#,
    );

    assert!(matches!(
        result,
        Err(HarnessFixtureError::UnsupportedFixtureMode { mode, field_path })
            if mode == "mcp" && field_path == "kind"
    ));
}

#[test]
fn replays_active_harness_skill_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_fixture_with_test_adapter("fixtures/harness/echo-skill.yaml")?;

    assert_eq!(output.status, HarnessExpectedStatus::Sealed);
    assert_eq!(output.receipt.subject.reference.uri, "hrn_echo-skill_echo");
    assert_eq!(output.receipt.seal.disposition, ClosureDisposition::Closed);
    let skill_output = output.skill_output.ok_or(HarnessFixtureError::Required {
        field: "skill_output".to_owned(),
    })?;
    assert_eq!(skill_output.stdout, "hello from harness");
    Ok(())
}

#[test]
fn replays_active_harness_graph_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let output = run_fixture_with_test_adapter("fixtures/harness/sequential-graph.yaml")?;

    assert_eq!(output.status, HarnessExpectedStatus::Sealed);
    assert_eq!(
        output.receipt.subject.reference.uri,
        "hrn_sequential-echo_graph"
    );
    assert_eq!(output.receipt.seal.disposition, ClosureDisposition::Closed);
    assert_eq!(output.step_receipts.len(), 2);
    assert_eq!(output.step_receipts[0].acts[0].id, "act_first");
    assert_eq!(output.step_receipts[1].acts[0].id, "act_second");
    Ok(())
}

#[test]
fn replays_payment_approval_graph_fixture_with_rail_proof() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_fixture_with_test_adapter("fixtures/harness/payment-approval-graph.yaml")?;

    assert_eq!(output.status, HarnessExpectedStatus::Sealed);
    assert_eq!(
        output.receipt.subject.reference.uri,
        "hrn_x402-pay-approval_graph"
    );
    assert_eq!(output.receipt.seal.disposition, ClosureDisposition::Closed);
    assert_eq!(output.step_receipts.len(), 2);
    assert_eq!(
        output.step_receipts[0].subject.reference.uri,
        "hrn_x402-pay-approval_approve-spend"
    );
    assert_eq!(
        output.step_receipts[1].subject.reference.uri,
        "hrn_x402-pay-approval_fulfill"
    );
    let fulfill_act =
        output.step_receipts[1]
            .acts
            .first()
            .ok_or(HarnessFixtureError::Required {
                field: "fulfill act".to_owned(),
            })?;
    assert_eq!(
        fulfill_act
            .criterion_bindings
            .iter()
            .flat_map(|criterion| criterion.verification_refs.iter())
            .map(|reference| reference.uri.as_str())
            .collect::<Vec<_>>(),
        vec!["receipt-proof:mock:x402-pay-approval-001"]
    );
    Ok(())
}

#[test]
fn replays_payment_denied_graph_fixture_as_policy_denied() -> Result<(), Box<dyn std::error::Error>>
{
    let output = run_fixture_with_test_adapter("fixtures/harness/payment-approval-denied.yaml")?;

    assert_eq!(output.status, HarnessExpectedStatus::PolicyDenied);
    assert_eq!(
        output.receipt.subject.reference.uri,
        "hrn_x402-pay-approval_graph"
    );
    assert_eq!(output.receipt.seal.disposition, ClosureDisposition::Blocked);
    assert_eq!(output.receipt.seal.reason_code, "graph_blocked");
    assert_eq!(output.step_receipts.len(), 1);
    assert_eq!(
        output.step_receipts[0].subject.reference.uri,
        "hrn_x402-pay-approval_approve-spend"
    );
    Ok(())
}

#[test]
fn replay_receipts_match_checked_in_canonical_oracles() -> Result<(), Box<dyn std::error::Error>> {
    let echo = run_fixture_with_test_adapter("fixtures/harness/echo-skill.yaml")?;
    assert_oracle(
        "fixtures/harness/oracle/echo-skill.receipt.json",
        &canonical_receipt_json(&echo.receipt)?,
    )?;

    let graph = run_fixture_with_test_adapter("fixtures/harness/sequential-graph.yaml")?;
    assert_oracle(
        "fixtures/harness/oracle/sequential-graph.receipt.json",
        &canonical_receipt_json(&graph.receipt)?,
    )?;
    assert_oracle(
        "fixtures/harness/oracle/sequential-graph.first.json",
        &canonical_receipt_json(&graph.step_receipts[0])?,
    )?;
    assert_oracle(
        "fixtures/harness/oracle/sequential-graph.second.json",
        &canonical_receipt_json(&graph.step_receipts[1])?,
    )?;

    let payment = run_fixture_with_test_adapter("fixtures/harness/payment-approval-graph.yaml")?;
    assert_oracle(
        "fixtures/harness/oracle/payment-approval-graph.receipt.json",
        &canonical_receipt_json(&payment.receipt)?,
    )?;
    assert_oracle(
        "fixtures/harness/oracle/payment-approval-graph.approve-spend.json",
        &canonical_receipt_json(&payment.step_receipts[0])?,
    )?;
    assert_oracle(
        "fixtures/harness/oracle/payment-approval-graph.fulfill.json",
        &canonical_receipt_json(&payment.step_receipts[1])?,
    )?;
    Ok(())
}

#[test]
#[cfg(not(feature = "cli-tool"))]
fn default_harness_runner_reports_disabled_cli_tool_feature() {
    let result =
        runx_runtime::run_harness_fixture(fixture_path("fixtures/harness/echo-skill.yaml"));

    assert!(matches!(
        result,
        Err(runx_runtime::HarnessReplayError::CliToolFeatureDisabled)
    ));
}

fn run_fixture_with_test_adapter(
    relative_path: &str,
) -> Result<runx_runtime::HarnessReplayOutput, runx_runtime::HarnessReplayError> {
    run_harness_fixture_with_adapter(
        fixture_path(relative_path),
        TestAdapter,
        fixture_runtime_options(),
    )
}

fn fixture_runtime_options() -> RuntimeOptions {
    RuntimeOptions {
        created_at: FIXTURE_CREATED_AT.to_owned(),
        effects: RuntimeEffectRegistry::with_payment_effect(FixtureEffectSupervisor),
        ..RuntimeOptions::local_development()
    }
}

#[derive(Clone, Debug)]
struct FixtureEffectSupervisor;

impl EffectSupervisor for FixtureEffectSupervisor {
    fn settlement_evidence(
        &self,
        request: EffectSettlementRequest<'_>,
    ) -> Result<EffectSettlementEvidence, EffectSupervisorError> {
        let request = request.payment_rail()?;
        match request.proof_ref {
            "receipt-proof:mock:x402-pay-approval-001" => Ok(EffectSettlementEvidence::generic(
                payment_supervisor_evidence_to_effect_record(payment_supervisor_evidence(
                    request,
                    "merchant-123",
                    "payment:x402-pay-approval-001",
                )),
            )),
            "receipt-proof:mock:paid-echo-001" => Ok(EffectSettlementEvidence::generic(
                payment_supervisor_evidence_to_effect_record(payment_supervisor_evidence(
                    request,
                    "merchant:paid-echo",
                    "payment:paid-echo-001",
                )),
            )),
            _ => Err(PaymentSupervisorError::InvalidSupervisorEvidence {
                message: format!(
                    "fixture supervisor has no settlement for {}",
                    request.proof_ref
                ),
            }
            .into()),
        }
    }
}

fn payment_supervisor_evidence(
    request: PaymentSupervisorSettlementRequest<'_>,
    expected_counterparty: &str,
    expected_idempotency_key: &str,
) -> PaymentSupervisorSettlementEvidence {
    PaymentSupervisorSettlementEvidence {
        verifier_id: PAYMENT_RAIL_SUPERVISOR_VERIFIER_ID.to_owned(),
        proof_ref: request.proof_ref.to_owned(),
        rail: request.rail.to_owned(),
        counterparty: expected_counterparty.to_owned(),
        amount_minor: request.amount_minor,
        currency: request.currency.to_owned(),
        idempotency_key: expected_idempotency_key.to_owned(),
        settlement_status: Some("fulfilled".to_owned()),
        provider_event_ref: Some(format!("fixture:event:{}", request.proof_ref)),
    }
}

struct TestAdapter;

impl SkillAdapter for TestAdapter {
    fn adapter_type(&self) -> &'static str {
        "cli-tool"
    }

    fn invoke(&self, request: SkillInvocation) -> Result<SkillOutput, runx_runtime::RuntimeError> {
        // The skill emits only the rail-packet claim. The trusted runtime
        // supervisor supplies matching settlement evidence, so test and live
        // execution share the same fail-closed boundary.
        let stdout = if request.skill_name == "pay-fulfill-rail" {
            r#"{"payment_rail_packet":{"data":{"rail_result":{"status":"fulfilled","rail":"mock","amount_minor":125,"currency":"USD"},"rail_proof":{"proof_ref":"receipt-proof:mock:x402-pay-approval-001","idempotency_key":"payment:x402-pay-approval-001"},"credential_envelope":{"form":"paid_tool_credential","credential_ref":"credential:mock:x402-pay-approval-001"}}}}"#.to_owned()
        } else {
            request
                .inputs
                .get("message")
                .and_then(|value| match value {
                    JsonValue::String(value) => Some(value.as_str()),
                    _ => None,
                })
                .unwrap_or_default()
                .to_owned()
        };
        Ok(SkillOutput {
            status: InvocationStatus::Success,
            stdout,
            stderr: String::new(),
            exit_code: Some(0),
            duration_ms: 0,
            metadata: JsonObject::default(),
        })
    }
}

fn fixture_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative_path)
}

fn assert_oracle(relative_path: &str, actual: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = fixture_path(relative_path);
    if std::env::var("RUNX_REGEN_FIXTURES").is_ok() {
        std::fs::write(&path, format!("{actual}\n"))?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(path)?;
    assert_eq!(expected, format!("{actual}\n"), "{relative_path}");
    Ok(())
}

fn retired_execution_receipt_field(prefix: &str) -> String {
    format!("{prefix}_{}", "execution")
}
