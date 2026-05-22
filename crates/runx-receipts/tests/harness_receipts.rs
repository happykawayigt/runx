use std::collections::BTreeSet;

use serde::Deserialize;

use runx_contracts::{
    ClosureDisposition, Receipt, ReceiptCommitmentScope, ReceiptIssuer, ReceiptIssuerType,
    ReceiptSignature, Reference, ReferenceType,
};
use runx_receipts::{
    ReceiptFindingCode, ReceiptJournal, ReceiptProofContext, ReceiptProofStatusKind,
    ReceiptVerification, SignatureVerificationFailure, SignatureVerifier,
    canonical_receipt_body_digest, canonical_receipt_digest, canonical_receipt_json,
    receipt_proof_status, validate_receipt, validate_receipt_proof, validate_receipt_tree,
    verify_receipt, verify_receipt_proof, verify_receipt_tree, verify_receipt_with_journal,
};

const SUCCESS_RECEIPT: &str =
    include_str!("../../../fixtures/contracts/harness-spine/receipt-success.json");
const ABNORMAL_RECEIPT: &str =
    include_str!("../../../fixtures/contracts/harness-spine/receipt-abnormal.json");

#[derive(Debug, Deserialize)]
struct Fixture {
    expected: Receipt,
}

fn with_child(mut receipt: Receipt) -> Receipt {
    receipt
        .lineage
        .get_or_insert_with(Default::default)
        .children
        .push(Reference::runx(ReferenceType::Receipt, "hrn_rcpt_child_1"));
    receipt
}

#[test]
fn success_receipt_verifies_basic_invariants() -> Result<(), serde_json::Error> {
    let receipt = fixture(SUCCESS_RECEIPT)?;

    assert!(validate_receipt(&receipt).is_ok());
    assert!(matches!(
        canonical_receipt_json(&receipt),
        Ok(json) if json.starts_with(r#"{"acts":"#)
    ));
    assert!(matches!(
        canonical_receipt_digest(&receipt),
        Ok(digest) if digest.starts_with("sha256:")
    ));
    Ok(())
}

#[test]
fn abnormal_failed_receipt_verifies_basic_invariants() -> Result<(), serde_json::Error> {
    let receipt = fixture(ABNORMAL_RECEIPT)?;

    assert!(validate_receipt(&receipt).is_ok());
    assert_eq!(receipt.seal.disposition, ClosureDisposition::Failed);
    Ok(())
}

#[test]
fn seal_criterion_must_bind_to_act_criteria() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.seal.criteria[0].criterion_id = "missing_criterion".to_owned();

    let verification = verify_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::SealCriterionUnbound);
    Ok(())
}

#[test]
fn child_refs_must_be_receipt_refs() -> Result<(), serde_json::Error> {
    let mut receipt = with_child(fixture(SUCCESS_RECEIPT)?);
    receipt.lineage.as_mut().unwrap().children[0].reference_type = ReferenceType::Artifact;

    let verification = verify_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::ChildReceiptRefInvalid);
    Ok(())
}

#[test]
fn idempotency_requires_sha256_prefixes() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.idempotency.intent_key = "not-a-hash".to_owned();

    let verification = verify_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::HashCommitmentInvalid);
    Ok(())
}

#[test]
fn subject_commitment_requires_sha256_prefix() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.subject.commitments[0].value = "stdout".to_owned();
    assert_eq!(
        receipt.subject.commitments[0].scope,
        ReceiptCommitmentScope::Output
    );

    let verification = verify_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::HashCommitmentInvalid);
    Ok(())
}

#[test]
fn payment_authority_bound_survives_in_body() -> Result<(), serde_json::Error> {
    // Inspectability acceptance: the granted authority stays readable in the body.
    let receipt = fixture(SUCCESS_RECEIPT)?;
    let json = canonical_receipt_json(&receipt).expect("canonical json");
    assert!(json.contains("\"authority\""));
    assert!(json.contains("\"terms\""));
    assert!(json.contains("\"idempotency\""));
    Ok(())
}

#[test]
fn receipt_tree_requires_supplied_child_receipts() -> Result<(), serde_json::Error> {
    let receipt = with_child(fixture(SUCCESS_RECEIPT)?);

    let verification = verify_receipt_tree(&receipt, &[]);

    assert_finding(&verification, ReceiptFindingCode::ChildReceiptMissing);
    Ok(())
}

#[test]
fn receipt_tree_accepts_matching_child_receipts() -> Result<(), serde_json::Error> {
    let mut receipt = with_child(fixture(SUCCESS_RECEIPT)?);
    let mut child = fixture(ABNORMAL_RECEIPT)?;
    child.id = "hrn_rcpt_child_1".to_owned();

    assert!(validate_receipt_tree(&receipt, &[child]).is_ok());
    receipt.lineage.as_mut().unwrap().children.clear();
    assert!(validate_receipt_tree(&receipt, &[]).is_ok());
    Ok(())
}

#[test]
fn receipt_tree_rejects_child_receipt_cycles() -> Result<(), serde_json::Error> {
    let receipt = with_child(fixture(SUCCESS_RECEIPT)?);
    let mut child = fixture(ABNORMAL_RECEIPT)?;
    child.id = "hrn_rcpt_child_1".to_owned();
    child
        .lineage
        .get_or_insert_with(Default::default)
        .children
        .push(Reference::runx(ReferenceType::Receipt, "hrn_rcpt_child_1"));

    let verification = verify_receipt_tree(&receipt, &[child]);

    assert_finding(&verification, ReceiptFindingCode::ChildReceiptCycle);
    Ok(())
}

#[test]
fn receipt_tree_rejects_orphan_supplied_children() -> Result<(), serde_json::Error> {
    let receipt = fixture(SUCCESS_RECEIPT)?;
    let mut child = fixture(ABNORMAL_RECEIPT)?;
    child.id = "hrn_rcpt_orphan".to_owned();

    let verification = verify_receipt_tree(&receipt, &[child]);

    assert_finding(&verification, ReceiptFindingCode::OrphanChildReceipt);
    Ok(())
}

#[test]
fn receipt_tree_rejects_duplicate_child_receipt_ids() -> Result<(), serde_json::Error> {
    let receipt = with_child(fixture(SUCCESS_RECEIPT)?);
    let mut first = fixture(ABNORMAL_RECEIPT)?;
    first.id = "hrn_rcpt_child_1".to_owned();
    let second = first.clone();

    let verification = verify_receipt_tree(&receipt, &[first, second]);

    assert_finding(&verification, ReceiptFindingCode::DuplicateChildReceipt);
    Ok(())
}

#[test]
fn verifier_issuer_type_matches_schema() -> Result<(), serde_json::Error> {
    let json = r#"{"type":"verifier","kid":"key_1","public_key_sha256":"sha256:public"}"#;
    let issuer: ReceiptIssuer = serde_json::from_str(json)?;

    assert_eq!(issuer.issuer_type, ReceiptIssuerType::Verifier);
    Ok(())
}

#[test]
fn strict_proof_accepts_recomputed_digest_and_signature() -> Result<(), serde_json::Error> {
    let receipt = proof_receipt()?;
    let verifier = FixtureSignatureVerifier;
    let context = proof_context(&verifier);

    assert!(validate_receipt_proof(&receipt, &context).is_ok());
    Ok(())
}

#[test]
fn structural_validation_does_not_claim_strict_proof() -> Result<(), serde_json::Error> {
    let receipt = fixture(SUCCESS_RECEIPT)?;

    assert!(validate_receipt(&receipt).is_ok());
    assert_finding(
        &verify_receipt_proof(&receipt, &ReceiptProofContext::default()),
        ReceiptFindingCode::SignatureVerifierMissing,
    );
    Ok(())
}

#[test]
fn strict_proof_rejects_tampered_body_digest() -> Result<(), serde_json::Error> {
    let mut receipt = proof_receipt()?;
    receipt.acts[0].summary = "tampered".to_owned();
    let verifier = FixtureSignatureVerifier;
    let context = proof_context(&verifier);

    let verification = verify_receipt_proof(&receipt, &context);

    assert_finding(&verification, ReceiptFindingCode::SealDigestMismatch);
    assert_finding(&verification, ReceiptFindingCode::SignatureInvalid);
    Ok(())
}

#[test]
fn strict_proof_requires_signature_verifier() -> Result<(), serde_json::Error> {
    let receipt = proof_receipt()?;

    let verification = verify_receipt_proof(&receipt, &ReceiptProofContext::default());

    assert_finding(&verification, ReceiptFindingCode::SignatureVerifierMissing);
    Ok(())
}

#[test]
fn strict_proof_rejects_tampered_signature() -> Result<(), serde_json::Error> {
    let mut receipt = proof_receipt()?;
    receipt.signature.value = "sig:tampered".to_owned();
    let verifier = FixtureSignatureVerifier;
    let context = proof_context(&verifier);

    let verification = verify_receipt_proof(&receipt, &context);

    assert_finding(&verification, ReceiptFindingCode::SignatureInvalid);
    Ok(())
}

#[test]
fn strict_proof_reports_unsupported_issuer() -> Result<(), serde_json::Error> {
    let mut receipt = proof_receipt()?;
    receipt.issuer.issuer_type = ReceiptIssuerType::Verifier;
    refresh_proof_digest_and_signature(&mut receipt)?;
    let verifier = UnsupportedIssuerVerifier;
    let context = ReceiptProofContext {
        signature_verifier: Some(&verifier),
        authority_verified: true,
        external_attestations_verified: true,
        verified_redaction_refs: BTreeSet::new(),
        verified_hash_commitments: BTreeSet::new(),
    };

    let verification = verify_receipt_proof(&receipt, &context);

    assert_finding(&verification, ReceiptFindingCode::SignatureUnsupportedIssuer);
    Ok(())
}

#[test]
fn journal_verification_catches_tampered_selected_act_id() -> Result<(), serde_json::Error> {
    use runx_contracts::{
        Decision, DecisionChoice, DecisionInputs, DecisionJustification, Intent,
    };
    use runx_receipts::ReceiptJournalDecision;

    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    let journal = ReceiptJournal {
        receipt_id: receipt.id.clone(),
        decisions: vec![ReceiptJournalDecision {
            decision: Decision {
                decision_id: "dec_1".to_owned(),
                choice: DecisionChoice::Open,
                inputs: DecisionInputs::default(),
                proposed_intent: Intent {
                    purpose: "open".to_owned(),
                    legitimacy: "test".to_owned(),
                    success_criteria: Vec::new(),
                    constraints: Vec::new(),
                    derived_from: Vec::new(),
                },
                selected_act_id: Some("missing_act".to_owned()),
                selected_harness_ref: None,
                justification: DecisionJustification {
                    summary: "test".to_owned(),
                    evidence_refs: Vec::new(),
                },
                closure: None,
                artifact_refs: Vec::new(),
            },
        }],
    };
    let journal_digest = journal.digest();
    receipt.lineage.get_or_insert_with(Default::default).journal_ref = Some(Reference {
        locator: Some(journal_digest),
        ..Reference::runx(ReferenceType::Decision, "journal")
    });

    let verification = verify_receipt_with_journal(&receipt, &journal);
    assert_finding(&verification, ReceiptFindingCode::DecisionSelectedActMissing);
    Ok(())
}

#[test]
fn journal_verification_fails_closed_on_hash_mismatch() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    let journal = ReceiptJournal {
        receipt_id: receipt.id.clone(),
        decisions: Vec::new(),
    };
    receipt.lineage.get_or_insert_with(Default::default).journal_ref = Some(Reference {
        locator: Some("sha256:wrong".to_owned()),
        ..Reference::runx(ReferenceType::Decision, "journal")
    });

    let verification = verify_receipt_with_journal(&receipt, &journal);
    assert_finding(&verification, ReceiptFindingCode::JournalHashMismatch);
    Ok(())
}

#[test]
fn proof_status_projects_safe_public_summary() -> Result<(), serde_json::Error> {
    let receipt = proof_receipt()?;
    let verifier = FixtureSignatureVerifier;
    let verification = verify_receipt_proof(&receipt, &proof_context(&verifier));

    let status = receipt_proof_status(&receipt, &verification);

    assert_eq!(status.receipt_id, receipt.id);
    assert_eq!(status.status, ReceiptProofStatusKind::Verified);
    assert!(status.finding_summaries.is_empty());
    Ok(())
}

#[test]
fn proof_status_redacts_absolute_paths_from_findings() -> Result<(), serde_json::Error> {
    let receipt = proof_receipt()?;
    let verification = ReceiptVerification::from_findings(vec![runx_receipts::ReceiptFinding {
        code: ReceiptFindingCode::SignatureInvalid,
        path: "(/Users/kam/private/key)".to_owned(),
        message: "failed at /Users/kam/private/key and C:\\Users\\kam\\private\\key with VGhpcy1sb29rcy1saWtlLWEtc2VjcmV0LXZhbHVlPQ==".to_owned(),
    }]);

    let status = receipt_proof_status(&receipt, &verification);

    assert_eq!(status.status, ReceiptProofStatusKind::Failed);
    assert_eq!(status.finding_summaries[0].path, "[local-path]");
    assert_eq!(
        status.finding_summaries[0].message,
        "failed at [local-path] and [local-path] with [secret]"
    );
    Ok(())
}

fn fixture(json: &str) -> Result<Receipt, serde_json::Error> {
    serde_json::from_str::<Fixture>(json).map(|fixture| fixture.expected)
}

fn proof_receipt() -> Result<Receipt, serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    refresh_proof_digest_and_signature(&mut receipt)?;
    Ok(receipt)
}

fn refresh_proof_digest_and_signature(receipt: &mut Receipt) -> Result<(), serde_json::Error> {
    let digest = canonical_receipt_body_digest(receipt)
        .map_err(|error| serde_json::Error::io(std::io::Error::other(error.to_string())))?;
    receipt.digest = digest.clone();
    receipt.signature.value = format!("sig:{digest}");
    Ok(())
}

fn proof_context(verifier: &FixtureSignatureVerifier) -> ReceiptProofContext<'_> {
    ReceiptProofContext {
        signature_verifier: Some(verifier),
        authority_verified: true,
        external_attestations_verified: true,
        verified_redaction_refs: BTreeSet::new(),
        verified_hash_commitments: BTreeSet::new(),
    }
}

struct FixtureSignatureVerifier;

impl SignatureVerifier for FixtureSignatureVerifier {
    fn verify(
        &self,
        _issuer: &ReceiptIssuer,
        signature: &ReceiptSignature,
        body_digest: &str,
    ) -> Result<(), SignatureVerificationFailure> {
        if signature.value == format!("sig:{body_digest}") {
            Ok(())
        } else {
            Err(SignatureVerificationFailure::SignatureMismatch)
        }
    }
}

struct UnsupportedIssuerVerifier;

impl SignatureVerifier for UnsupportedIssuerVerifier {
    fn verify(
        &self,
        _issuer: &ReceiptIssuer,
        _signature: &ReceiptSignature,
        _body_digest: &str,
    ) -> Result<(), SignatureVerificationFailure> {
        Err(SignatureVerificationFailure::UnsupportedIssuer)
    }
}

fn assert_finding(verification: &ReceiptVerification, code: ReceiptFindingCode) {
    assert!(
        verification
            .findings
            .iter()
            .any(|finding| finding.code == code),
        "expected finding {code:?}; got {:?}",
        verification.findings
    );
}
