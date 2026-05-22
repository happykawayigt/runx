use serde::Deserialize;

use runx_contracts::{Act, ActForm, GovernedActRef, Receipt, ReferenceType, Signal};

const FIXTURES: &[&str] = &[
    include_str!("../../../fixtures/contracts/harness-spine/act-ref.json"),
    include_str!("../../../fixtures/contracts/harness-spine/receipt-abnormal.json"),
    include_str!("../../../fixtures/contracts/harness-spine/receipt-success.json"),
    include_str!(
        "../../../fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json"
    ),
    include_str!("../../../fixtures/contracts/harness-spine/signal-fingerprint-links.json"),
    include_str!("../../../fixtures/contracts/harness-spine/verification-act.json"),
];

const POST_MERGE_OBSERVER_FIXTURE: &str = include_str!(
    "../../../fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json"
);

#[derive(Debug, Deserialize)]
struct Fixture {
    fixture_kind: FixtureKind,
    expected: serde_json::Value,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FixtureKind {
    Act,
    GovernedActRef,
    Receipt,
    Signal,
}

#[test]
fn harness_spine_fixtures_roundtrip() -> Result<(), serde_json::Error> {
    for fixture_json in FIXTURES {
        let fixture: Fixture = serde_json::from_str(fixture_json)?;
        assert_roundtrip(fixture)?;
    }
    Ok(())
}

#[test]
fn receipt_rejects_unknown_fields() -> Result<(), serde_json::Error> {
    let mut fixture: Fixture = serde_json::from_str(include_str!(
        "../../../fixtures/contracts/harness-spine/receipt-success.json"
    ))?;
    fixture.expected["unexpected"] = serde_json::json!(true);

    let result = serde_json::from_value::<Receipt>(fixture.expected);

    assert!(result.is_err());
    Ok(())
}

#[test]
fn governed_act_ref_requires_receipt_context() {
    let value = serde_json::json!({
        "act_ref": {
            "act_id": "act_revision_1"
        }
    });

    let result = serde_json::from_value::<GovernedActRef>(value);

    assert!(result.is_err());
}

#[test]
fn provider_workflow_act_form_is_rejected() {
    let value = serde_json::json!({
        "act_id": "act_legacy",
        "form": "pull_request",
        "intent": {
            "purpose": "Do legacy work",
            "legitimacy": "Not admitted"
        },
        "summary": "Provider workflow names are not act forms",
        "closure": {
            "disposition": "blocked",
            "reason_code": "provider_workflow_form",
            "summary": "Provider workflow names are not act forms",
            "closed_at": "2026-05-18T00:00:00Z"
        },
        "source_refs": [],
        "target_refs": [],
        "surface_refs": [],
        "artifact_refs": [],
        "verification_refs": [],
        "harness_refs": [],
        "performed_at": "2026-05-18T00:00:00Z"
    });

    let result = serde_json::from_value::<Act>(value);

    assert!(result.is_err());
}

#[test]
fn post_merge_observer_fixture_binds_seal_criteria_to_acts()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture: Fixture = serde_json::from_str(POST_MERGE_OBSERVER_FIXTURE)?;
    let receipt: Receipt = serde_json::from_value(fixture.expected)?;

    assert_eq!(receipt.seal.reason_code, "merged_verified");
    assert_eq!(
        receipt.idempotency.intent_key,
        "post-merge:github://runxhq/nitrosend/issues/77:github://runxhq/nitrosend/pulls/188"
    );

    let act_forms = receipt
        .acts
        .iter()
        .map(|act| act.form.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        act_forms,
        vec![
            ActForm::Observation,
            ActForm::Verification,
            ActForm::Reply,
            ActForm::Revision,
        ]
    );

    let criteria = receipt
        .seal
        .criteria
        .iter()
        .map(|criterion| criterion.criterion_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        criteria,
        vec![
            "post_merge.provider_state",
            "post_merge.human_gate",
            "post_merge.verification_passed",
            "post_merge.source_thread_target_present",
            "post_merge.close_policy_authorized",
        ]
    );

    let Some(thread_criterion) = receipt
        .seal
        .criteria
        .iter()
        .find(|criterion| criterion.criterion_id == "post_merge.source_thread_target_present")
    else {
        return Err("source-thread criterion exists".into());
    };
    assert!(!thread_criterion.verification_refs.is_empty());
    assert!(thread_criterion.evidence_refs.iter().any(|reference| {
        reference.reference_type == ReferenceType::SlackThread
            && reference
                .locator
                .as_deref()
                .is_some_and(|locator| locator.matches('/').count() >= 2)
    }));

    let retired_tokens = [
        ["harness", "_receipt"].concat(),
        ["runx.harness", "_receipt.v1"].concat(),
        ["verification", "_", "summary"].concat(),
    ];
    for retired_token in &retired_tokens {
        assert!(
            !POST_MERGE_OBSERVER_FIXTURE.contains(retired_token),
            "fixture contains retired token {retired_token}"
        );
    }

    Ok(())
}

fn assert_roundtrip(fixture: Fixture) -> Result<(), serde_json::Error> {
    match fixture.fixture_kind {
        FixtureKind::Act => roundtrip::<Act>(fixture.expected),
        FixtureKind::GovernedActRef => roundtrip::<GovernedActRef>(fixture.expected),
        FixtureKind::Receipt => roundtrip::<Receipt>(fixture.expected),
        FixtureKind::Signal => roundtrip::<Signal>(fixture.expected),
    }
}

fn roundtrip<T>(expected: serde_json::Value) -> Result<(), serde_json::Error>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let parsed: T = serde_json::from_value(expected.clone())?;
    let actual = serde_json::to_value(parsed)?;
    assert_eq!(actual, expected);
    Ok(())
}
