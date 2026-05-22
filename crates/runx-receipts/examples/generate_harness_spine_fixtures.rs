//! Regenerates the flat `runx.receipt.v1` harness-spine fixtures and the
//! canonical-json oracle. Run with:
//!   cargo run --manifest-path crates/Cargo.toml -p runx-receipts \
//!     --example generate_harness_spine_fixtures
use std::fs;
use std::path::Path;

use runx_contracts::{
    ActForm, AuthorityAttenuation, ClosureDisposition, CriterionStatus, HashAlgorithm, Lineage,
    RECEIPT_CANONICALIZATION, Receipt, ReceiptAct, ReceiptAuthority, ReceiptCommitment,
    ReceiptCommitmentScope, ReceiptCriterion, ReceiptEnforcement, ReceiptIdempotency, ReceiptIssuer,
    ReceiptIssuerType, ReceiptSchema, ReceiptSignature, ReceiptSubjectKind, Reference, ReferenceType,
    Seal, SignatureAlgorithm, Subject,
};
use runx_receipts::{canonical_receipt_body_digest, canonical_receipt_digest, canonical_receipt_json};
use serde_json::{Value, json};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let spine = root.join("fixtures/contracts/harness-spine");
    let canonical = root.join("fixtures/contracts/canonical-json");

    let success = sealed(success_receipt());
    let abnormal = sealed(abnormal_receipt());
    let post_merge = sealed(post_merge_receipt());

    write_fixture(
        &spine.join("receipt-success.json"),
        "receipt_success",
        "Sealed runx.receipt.v1 for a successful skill run.",
        "receipt",
        &success,
    );
    write_fixture(
        &spine.join("receipt-abnormal.json"),
        "receipt_abnormal",
        "Sealed runx.receipt.v1 for a failed skill run.",
        "receipt",
        &abnormal,
    );
    write_fixture(
        &spine.join("post-merge-observer-merged-verified.json"),
        "post_merge_observer_merged_verified",
        "Sealed post-merge observer runx.receipt.v1.",
        "receipt",
        &post_merge,
    );

    let oracle = json!({
        "schema": "runx.canonical_json_oracle.v1",
        "canonicalization": RECEIPT_CANONICALIZATION,
        "cases": [
            oracle_case("receipt-success", "harness-spine/receipt-success.json", &success),
            oracle_case("receipt-abnormal", "harness-spine/receipt-abnormal.json", &abnormal),
            oracle_case(
                "post-merge-observer-merged-verified",
                "harness-spine/post-merge-observer-merged-verified.json",
                &post_merge,
            ),
        ],
    });
    fs::write(
        canonical.join("runx-receipt-c14n-v1.oracles.json"),
        format!("{}\n", serde_json::to_string_pretty(&oracle).unwrap()),
    )
    .unwrap();

    // Remove the retired old-shape oracle.

    println!("regenerated harness-spine fixtures + receipt c14n oracle");
}

fn write_fixture(path: &Path, name: &str, description: &str, kind: &str, receipt: &Receipt) {
    let wrapper = json!({
        "fixture_kind": kind,
        "name": name,
        "description": description,
        "scope": "harness-spine",
        "expected": serde_json::to_value(receipt).unwrap(),
    });
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(&wrapper).unwrap()),
    )
    .unwrap();
}

fn oracle_case(name: &str, fixture: &str, receipt: &Receipt) -> Value {
    json!({
        "name": name,
        "fixture": fixture,
        "full_canonical_json": canonical_receipt_json(receipt).unwrap(),
        "full_sha256": canonical_receipt_digest(receipt).unwrap(),
        "body_canonical_json": runx_receipts::canonical_receipt_body_json(receipt).unwrap(),
        "body_sha256": canonical_receipt_body_digest(receipt).unwrap(),
    })
}

fn sealed(mut receipt: Receipt) -> Receipt {
    let digest = canonical_receipt_body_digest(&receipt).unwrap();
    receipt.digest = digest.clone();
    receipt.signature.value = format!("sig:{digest}");
    receipt
}

fn base(id: &str, kind: ReceiptSubjectKind, subject_id: &str) -> Receipt {
    Receipt {
        schema: ReceiptSchema::V1,
        id: id.to_owned(),
        created_at: "2026-05-22T00:00:00Z".to_owned(),
        canonicalization: RECEIPT_CANONICALIZATION.to_owned(),
        issuer: ReceiptIssuer {
            issuer_type: ReceiptIssuerType::Local,
            kid: "fixture-key".to_owned(),
            public_key_sha256: format!("sha256:{}", "0".repeat(64)),
        },
        signature: ReceiptSignature {
            alg: SignatureAlgorithm::Ed25519,
            value: "sig:pending".to_owned(),
        },
        digest: "sha256:pending".to_owned(),
        idempotency: ReceiptIdempotency {
            intent_key: format!("sha256:{}", "1".repeat(64)),
            trigger_fingerprint: format!("sha256:{}", "2".repeat(64)),
            content_hash: format!("sha256:{}", "3".repeat(64)),
        },
        subject: Subject {
            kind,
            reference: Reference::runx(ReferenceType::Harness, subject_id),
            commitments: vec![ReceiptCommitment {
                scope: ReceiptCommitmentScope::Output,
                algorithm: HashAlgorithm::Sha256,
                value: format!("sha256:{}", "4".repeat(64)),
                canonicalization: "runx.stable-json.v1".to_owned(),
            }],
        },
        authority: ReceiptAuthority {
            actor_ref: Reference::runx(ReferenceType::Principal, "local_runtime"),
            authority_proof_refs: Vec::new(),
            grant_refs: Vec::new(),
            scope_refs: Vec::new(),
            terms: Vec::new(),
            attenuation: AuthorityAttenuation {
                parent_authority_ref: None,
                subset_proof: None,
            },
            mandate_ref: None,
            enforcement: ReceiptEnforcement {
                profile_hash: format!("sha256:{}", "5".repeat(64)),
                redaction_refs: Vec::new(),
                setup_refs: Vec::new(),
                teardown_refs: Vec::new(),
            },
        },
        acts: Vec::new(),
        seal: Seal {
            disposition: ClosureDisposition::Closed,
            reason_code: "process_closed".to_owned(),
            summary: "closed".to_owned(),
            closed_at: "2026-05-22T00:00:00Z".to_owned(),
            criteria: Vec::new(),
        },
        lineage: Some(Lineage::default()),
        metadata: None,
    }
}

fn success_receipt() -> Receipt {
    let mut receipt = base("hrn_rcpt_echo_success", ReceiptSubjectKind::Skill, "echo_success");
    receipt.acts = vec![ReceiptAct {
        id: "act_echo".to_owned(),
        form: ActForm::Observation,
        summary: "Executed graph step echo".to_owned(),
        criteria: vec![ReceiptCriterion {
            criterion_id: "process_exit".to_owned(),
            status: CriterionStatus::Verified,
            evidence_refs: Vec::new(),
            verification_refs: Vec::new(),
            summary: Some("cli-tool exited successfully".to_owned()),
        }],
        by: None,
        artifact_refs: Vec::new(),
        detail_ref: None,
    }];
    receipt.seal.summary = "cli-tool exited successfully".to_owned();
    receipt.seal.criteria = vec![ReceiptCriterion {
        criterion_id: "process_exit".to_owned(),
        status: CriterionStatus::Verified,
        evidence_refs: Vec::new(),
        verification_refs: Vec::new(),
        summary: Some("cli-tool exited successfully".to_owned()),
    }];
    receipt
}

fn abnormal_receipt() -> Receipt {
    let mut receipt = base("hrn_rcpt_echo_abnormal", ReceiptSubjectKind::Skill, "echo_abnormal");
    receipt.acts = vec![ReceiptAct {
        id: "act_echo".to_owned(),
        form: ActForm::Observation,
        summary: "Executed graph step echo".to_owned(),
        criteria: vec![ReceiptCriterion {
            criterion_id: "process_exit".to_owned(),
            status: CriterionStatus::Failed,
            evidence_refs: Vec::new(),
            verification_refs: Vec::new(),
            summary: Some("cli-tool failed".to_owned()),
        }],
        by: None,
        artifact_refs: Vec::new(),
        detail_ref: None,
    }];
    receipt.seal.disposition = ClosureDisposition::Failed;
    receipt.seal.reason_code = "process_failed".to_owned();
    receipt.seal.summary = "cli-tool failed".to_owned();
    receipt.seal.criteria = vec![ReceiptCriterion {
        criterion_id: "process_exit".to_owned(),
        status: CriterionStatus::Failed,
        evidence_refs: Vec::new(),
        verification_refs: Vec::new(),
        summary: Some("cli-tool failed".to_owned()),
    }];
    receipt
}

fn post_merge_receipt() -> Receipt {
    let mut receipt = base(
        "hrn_rcpt_post_merge_nitrosend_77_188",
        ReceiptSubjectKind::Skill,
        "post_merge_observer",
    );
    receipt.idempotency.intent_key =
        "post-merge:github://runxhq/nitrosend/issues/77:github://runxhq/nitrosend/pulls/188"
            .to_owned();
    let issue_ref = Reference {
        provider: Some("github".to_owned()),
        locator: Some("runxhq/nitrosend#77".to_owned()),
        ..Reference::with_uri(ReferenceType::GithubIssue, "github://runxhq/nitrosend/issues/77")
    };
    let pr_ref = Reference {
        provider: Some("github".to_owned()),
        locator: Some("runxhq/nitrosend!188".to_owned()),
        ..Reference::with_uri(
            ReferenceType::GithubPullRequest,
            "github://runxhq/nitrosend/pulls/188",
        )
    };
    let slack_ref = Reference {
        provider: Some("slack".to_owned()),
        locator: Some("team/channel/1700000000.0001".to_owned()),
        ..Reference::with_uri(
            ReferenceType::SlackThread,
            "slack://team/channel/1700000000.0001",
        )
    };
    let verification_ref = Reference::runx(ReferenceType::Verification, "ver_post_merge_verified");
    let acts = [
        ("act_observe", ActForm::Observation),
        ("act_verify", ActForm::Verification),
        ("act_reply", ActForm::Reply),
        ("act_revise", ActForm::Revision),
    ];
    receipt.acts = acts
        .iter()
        .map(|(id, form)| ReceiptAct {
            id: (*id).to_owned(),
            form: form.clone(),
            summary: format!("post-merge {id}"),
            criteria: Vec::new(),
            by: None,
            artifact_refs: vec![issue_ref.clone(), pr_ref.clone(), slack_ref.clone()],
            detail_ref: None,
        })
        .collect();
    receipt.seal.reason_code = "merged_verified".to_owned();
    receipt.seal.summary = "Target PR shipped and verified.".to_owned();
    receipt.seal.criteria = vec![
        criterion(
            "post_merge.provider_state",
            Vec::new(),
            vec![pr_ref.clone()],
            None,
        ),
        criterion("post_merge.human_gate", Vec::new(), Vec::new(), None),
        criterion(
            "post_merge.verification_passed",
            vec![verification_ref.clone()],
            vec![pr_ref.clone()],
            Some("Nitrosend dogfood verification passed."),
        ),
        criterion(
            "post_merge.source_thread_target_present",
            vec![verification_ref],
            vec![slack_ref, issue_ref],
            None,
        ),
        criterion(
            "post_merge.close_policy_authorized",
            Vec::new(),
            Vec::new(),
            None,
        ),
    ];
    use runx_contracts::{JsonObject, JsonValue};
    let mut pr = JsonObject::new();
    pr.insert(
        "merge_sha".to_owned(),
        JsonValue::String("9f14c0ffee1234567890abcdef1234567890abcd".to_owned()),
    );
    let mut observer = JsonObject::new();
    observer.insert("pr".to_owned(), JsonValue::Object(pr));
    let mut metadata = JsonObject::new();
    metadata.insert("observer_contract".to_owned(), JsonValue::Object(observer));
    receipt.metadata = Some(metadata);
    receipt
}

fn criterion(
    id: &str,
    verification_refs: Vec<Reference>,
    evidence_refs: Vec<Reference>,
    summary: Option<&str>,
) -> ReceiptCriterion {
    ReceiptCriterion {
        criterion_id: id.to_owned(),
        status: CriterionStatus::Verified,
        evidence_refs,
        verification_refs,
        summary: summary.map(str::to_owned),
    }
}
