use std::collections::BTreeSet;

use serde::Deserialize;

use runx_contracts::{
    ActForm, HarnessReceipt, HarnessState, HashAlgorithm, HashCommitment, ReceiptIssuer,
    ReceiptIssuerType, ReceiptSignature, Reference, ReferenceType,
};
use runx_receipts::{
    ReceiptFindingCode, ReceiptProofContext, ReceiptProofStatusKind, ReceiptVerification,
    SignatureVerificationFailure, SignatureVerifier, canonical_receipt_body_digest,
    canonical_receipt_digest, canonical_receipt_json, receipt_proof_status, validate_harness,
    validate_harness_receipt, validate_harness_receipt_proof, validate_receipt_tree,
    verify_harness_receipt, verify_harness_receipt_proof, verify_receipt_tree,
};

const SUCCESS_RECEIPT: &str =
    include_str!("../../../fixtures/contracts/harness-spine/harness-receipt-success.json");
const ABNORMAL_RECEIPT: &str =
    include_str!("../../../fixtures/contracts/harness-spine/harness-receipt-abnormal.json");

#[derive(Debug, Deserialize)]
struct Fixture {
    expected: HarnessReceipt,
}

#[test]
fn harness_spine_success_receipt_verifies_basic_invariants() -> Result<(), serde_json::Error> {
    let receipt = fixture(SUCCESS_RECEIPT)?;

    assert!(validate_harness_receipt(&receipt).is_ok());
    assert!(matches!(
        canonical_receipt_json(&receipt),
        Ok(json) if json.starts_with(r#"{"created_at":"#)
    ));
    assert!(matches!(
        canonical_receipt_digest(&receipt),
        Ok(digest) if digest.starts_with("sha256:")
    ));
    Ok(())
}

#[test]
fn harness_spine_abnormal_failed_receipt_verifies_basic_invariants() -> Result<(), serde_json::Error>
{
    let receipt = fixture(ABNORMAL_RECEIPT)?;

    assert!(validate_harness_receipt(&receipt).is_ok());
    assert_eq!(receipt.harness.state, HarnessState::Failed);
    assert!(receipt.harness.acts.is_empty());
    Ok(())
}

#[test]
fn receipt_rejects_when_top_level_seal_differs_from_harness_seal() -> Result<(), serde_json::Error>
{
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.seal.reason_code = "different".to_owned();

    let verification = verify_harness_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::ReceiptSealMismatch);
    Ok(())
}

#[test]
fn terminal_harness_state_requires_seal() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.harness.seal = None;

    let verification = verify_harness_receipt(&receipt);

    assert_finding(
        &verification,
        ReceiptFindingCode::TerminalHarnessMissingSeal,
    );
    assert_finding(&verification, ReceiptFindingCode::ReceiptSealMismatch);
    Ok(())
}

#[test]
fn nonterminal_harness_state_rejects_seal() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.harness.state = HarnessState::Running;

    let result = validate_harness(&receipt.harness);

    assert!(result.is_err());
    if let Err(verification) = result {
        assert_finding(&verification, ReceiptFindingCode::NonTerminalHarnessHasSeal);
    }
    Ok(())
}

#[test]
fn revision_act_requires_revision_details() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.harness.acts[0].revision = None;

    let verification = verify_harness_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::ActFormDetailsInvalid);
    Ok(())
}

#[test]
fn verification_act_requires_verification_details() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    let act = &mut receipt.harness.acts[0];
    act.form = ActForm::Verification;
    act.revision = None;

    let verification = verify_harness_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::ActFormDetailsInvalid);
    Ok(())
}

#[test]
fn decision_selected_act_id_must_exist() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.harness.decisions[0].selected_act_id = Some("missing".to_owned());

    let verification = verify_harness_receipt(&receipt);

    assert_finding(
        &verification,
        ReceiptFindingCode::DecisionSelectedActMissing,
    );
    Ok(())
}

#[test]
fn seal_criterion_act_id_must_exist() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    if let Some(seal) = receipt.harness.seal.as_mut() {
        seal.criteria[0].act_id = Some("missing".to_owned());
        receipt.seal = seal.clone();
    }

    let verification = verify_harness_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::SealCriterionActMissing);
    Ok(())
}

#[test]
fn seal_criterion_must_bind_to_act_criteria() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    if let Some(seal) = receipt.harness.seal.as_mut() {
        seal.criteria[0].criterion_id = "missing_criterion".to_owned();
        receipt.seal = seal.clone();
    }

    let verification = verify_harness_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::SealCriterionUnbound);
    Ok(())
}

#[test]
fn child_harness_refs_must_be_harness_receipt_refs() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.harness.child_harness_receipt_refs[0].reference_type = ReferenceType::Receipt;

    let verification = verify_harness_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::ChildReceiptRefInvalid);
    Ok(())
}

#[test]
fn receipt_tree_requires_supplied_child_receipts() -> Result<(), serde_json::Error> {
    let receipt = fixture(SUCCESS_RECEIPT)?;

    let verification = verify_receipt_tree(&receipt, &[]);

    assert_finding(&verification, ReceiptFindingCode::ChildReceiptMissing);
    Ok(())
}

#[test]
fn receipt_tree_accepts_matching_child_receipts() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    let mut child = fixture(ABNORMAL_RECEIPT)?;
    child.id = "hrn_rcpt_child_1".to_owned();

    assert!(validate_receipt_tree(&receipt, &[child]).is_ok());
    receipt.harness.child_harness_receipt_refs.clear();
    assert!(validate_receipt_tree(&receipt, &[]).is_ok());
    Ok(())
}

#[test]
fn receipt_tree_checks_nested_child_receipt_refs() -> Result<(), serde_json::Error> {
    let receipt = fixture(SUCCESS_RECEIPT)?;
    let mut child = fixture(ABNORMAL_RECEIPT)?;
    child.id = "hrn_rcpt_child_1".to_owned();
    child.harness.child_harness_receipt_refs.push(Reference {
        reference_type: ReferenceType::HarnessReceipt,
        uri: "runx:harness_receipt:missing_grandchild".to_owned(),
        provider: None,
        locator: None,
        label: None,
        observed_at: None,
        proof_kind: None,
    });

    let verification = verify_receipt_tree(&receipt, &[child]);

    assert_finding(&verification, ReceiptFindingCode::ChildReceiptMissing);
    assert!(
        verification
            .findings
            .iter()
            .any(|finding| { finding.path == "children[0].harness.child_harness_receipt_refs[0]" })
    );
    Ok(())
}

#[test]
fn receipt_tree_rejects_child_receipt_cycles() -> Result<(), serde_json::Error> {
    let receipt = fixture(SUCCESS_RECEIPT)?;
    let mut child = fixture(ABNORMAL_RECEIPT)?;
    child.id = "hrn_rcpt_child_1".to_owned();
    child.harness.child_harness_receipt_refs.push(Reference {
        reference_type: ReferenceType::HarnessReceipt,
        uri: "runx:harness_receipt:hrn_rcpt_child_1".to_owned(),
        provider: None,
        locator: None,
        label: None,
        observed_at: None,
        proof_kind: None,
    });

    let verification = verify_receipt_tree(&receipt, &[child]);

    assert_finding(&verification, ReceiptFindingCode::ChildReceiptCycle);
    Ok(())
}

#[test]
fn receipt_tree_rejects_orphan_supplied_children() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.harness.child_harness_receipt_refs.clear();
    let mut child = fixture(ABNORMAL_RECEIPT)?;
    child.id = "hrn_rcpt_orphan".to_owned();

    let verification = verify_receipt_tree(&receipt, &[child]);

    assert_finding(&verification, ReceiptFindingCode::OrphanChildReceipt);
    Ok(())
}

#[test]
fn receipt_tree_rejects_wrong_namespace_child_refs() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.harness.child_harness_receipt_refs[0].uri = "runx:receipt:hrn_rcpt_child_1".to_owned();
    let mut child = fixture(ABNORMAL_RECEIPT)?;
    child.id = "hrn_rcpt_child_1".to_owned();

    let verification = verify_receipt_tree(&receipt, &[child]);

    assert_finding(&verification, ReceiptFindingCode::ChildReceiptRefMalformed);
    assert_finding(&verification, ReceiptFindingCode::OrphanChildReceipt);
    Ok(())
}

#[test]
fn receipt_tree_rejects_duplicate_child_receipt_ids() -> Result<(), serde_json::Error> {
    let receipt = fixture(SUCCESS_RECEIPT)?;
    let mut first = fixture(ABNORMAL_RECEIPT)?;
    first.id = "hrn_rcpt_child_1".to_owned();
    let second = first.clone();

    let verification = verify_receipt_tree(&receipt, &[first, second]);

    assert_finding(&verification, ReceiptFindingCode::DuplicateChildReceipt);
    Ok(())
}

#[test]
fn hash_commitments_require_sha256_prefix() -> Result<(), serde_json::Error> {
    let mut receipt = fixture(ABNORMAL_RECEIPT)?;
    if let Some(seal) = receipt.harness.seal.as_mut() {
        seal.hash_commitments[0].value = "stderr".to_owned();
        receipt.seal = seal.clone();
    }

    let verification = verify_harness_receipt(&receipt);

    assert_finding(&verification, ReceiptFindingCode::HashCommitmentInvalid);
    Ok(())
}

#[test]
fn verifier_issuer_type_matches_schema() -> Result<(), serde_json::Error> {
    let json = r#"{"type":"verifier","kid":"key_1","public_key_sha256":"sha256:public"}"#;
    let issuer: runx_contracts::ReceiptIssuer = serde_json::from_str(json)?;

    assert_eq!(issuer.issuer_type, ReceiptIssuerType::Verifier);
    Ok(())
}

#[test]
fn strict_proof_accepts_recomputed_digest_and_signature() -> Result<(), serde_json::Error> {
    let receipt = proof_receipt()?;
    let verifier = FixtureSignatureVerifier;
    let context = proof_context(&verifier);

    assert!(validate_harness_receipt_proof(&receipt, &context).is_ok());
    Ok(())
}

#[test]
fn structural_validation_does_not_claim_strict_proof() -> Result<(), serde_json::Error> {
    let receipt = fixture(SUCCESS_RECEIPT)?;

    assert!(validate_harness_receipt(&receipt).is_ok());
    assert_finding(
        &verify_harness_receipt_proof(&receipt, &ReceiptProofContext::default()),
        ReceiptFindingCode::SealDigestMismatch,
    );
    Ok(())
}

#[test]
fn strict_proof_rejects_tampered_body_digest() -> Result<(), serde_json::Error> {
    let mut receipt = proof_receipt()?;
    receipt.harness.acts[0].summary = "tampered".to_owned();
    let verifier = FixtureSignatureVerifier;
    let context = proof_context(&verifier);

    let verification = verify_harness_receipt_proof(&receipt, &context);

    assert_finding(&verification, ReceiptFindingCode::SealDigestMismatch);
    assert_finding(&verification, ReceiptFindingCode::SignatureInvalid);
    Ok(())
}

#[test]
fn strict_proof_requires_signature_verifier() -> Result<(), serde_json::Error> {
    let receipt = proof_receipt()?;

    let verification = verify_harness_receipt_proof(&receipt, &ReceiptProofContext::default());

    assert_finding(&verification, ReceiptFindingCode::SignatureVerifierMissing);
    assert_finding(
        &verification,
        ReceiptFindingCode::VerificationSummaryInvalid,
    );
    Ok(())
}

#[test]
fn strict_proof_rejects_root_authority_attenuation_claim() -> Result<(), serde_json::Error> {
    let mut receipt = proof_receipt()?;
    receipt.harness.authority.attenuation.parent_authority_ref = None;
    receipt.harness.authority.attenuation.subset_proof = None;
    receipt.harness.authority.authority_proof_refs.clear();
    receipt.harness.authority.grant_refs.clear();
    receipt.harness.authority.scope_refs.clear();
    if let Some(summary) = receipt.seal.verification_summary.as_mut() {
        summary.authority_attenuation_valid = true;
    }
    if let Some(seal) = receipt.harness.seal.as_mut() {
        if let Some(summary) = seal.verification_summary.as_mut() {
            summary.authority_attenuation_valid = true;
        }
    }
    refresh_proof_digest_and_signature(&mut receipt)?;
    let verifier = FixtureSignatureVerifier;

    let verification = verify_harness_receipt_proof(&receipt, &proof_context(&verifier));

    assert_finding(
        &verification,
        ReceiptFindingCode::VerificationSummaryInvalid,
    );
    Ok(())
}

#[test]
fn strict_proof_rejects_tampered_signature() -> Result<(), serde_json::Error> {
    let mut receipt = proof_receipt()?;
    receipt.signature.value = "sig:tampered".to_owned();
    let verifier = FixtureSignatureVerifier;
    let context = proof_context(&verifier);

    let verification = verify_harness_receipt_proof(&receipt, &context);

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

    let verification = verify_harness_receipt_proof(&receipt, &context);

    assert_finding(
        &verification,
        ReceiptFindingCode::SignatureUnsupportedIssuer,
    );
    Ok(())
}

#[test]
fn strict_proof_rejects_missing_authority_result() -> Result<(), serde_json::Error> {
    let receipt = proof_receipt()?;
    let verifier = FixtureSignatureVerifier;
    let mut context = proof_context(&verifier);
    context.authority_verified = false;

    let verification = verify_harness_receipt_proof(&receipt, &context);

    assert_finding(&verification, ReceiptFindingCode::AuthorityProofMissing);
    Ok(())
}

#[test]
fn strict_proof_rejects_unverified_redaction_refs() -> Result<(), serde_json::Error> {
    let mut receipt = proof_receipt()?;
    receipt.harness.enforcement.redaction_refs.push(Reference {
        reference_type: ReferenceType::RedactionPolicy,
        uri: "runx:redaction:redact_1".to_owned(),
        provider: None,
        locator: None,
        label: None,
        observed_at: None,
        proof_kind: None,
    });
    refresh_proof_digest_and_signature(&mut receipt)?;
    let verifier = FixtureSignatureVerifier;
    let context = proof_context(&verifier);

    let verification = verify_harness_receipt_proof(&receipt, &context);

    assert_finding(&verification, ReceiptFindingCode::RedactionProofMissing);
    Ok(())
}

#[test]
fn strict_proof_accepts_verified_redaction_refs() -> Result<(), serde_json::Error> {
    let mut receipt = proof_receipt()?;
    receipt.seal.redaction_refs.push(Reference {
        reference_type: ReferenceType::RedactionPolicy,
        uri: "runx:redaction:redact_1".to_owned(),
        provider: None,
        locator: None,
        label: None,
        observed_at: None,
        proof_kind: None,
    });
    if let Some(seal) = receipt.harness.seal.as_mut() {
        seal.redaction_refs = receipt.seal.redaction_refs.clone();
    }
    refresh_proof_digest_and_signature(&mut receipt)?;
    let verifier = FixtureSignatureVerifier;
    let mut context = proof_context(&verifier);
    context
        .verified_redaction_refs
        .insert("runx:redaction:redact_1".to_owned());

    assert!(validate_harness_receipt_proof(&receipt, &context).is_ok());
    Ok(())
}

#[test]
fn strict_proof_rejects_unverified_hash_commitments() -> Result<(), serde_json::Error> {
    let mut receipt = proof_receipt()?;
    let commitment = HashCommitment {
        algorithm: HashAlgorithm::Sha256,
        value: "sha256:stdout".to_owned(),
        canonicalization: "runx.hash.v1".to_owned(),
    };
    receipt.harness.enforcement.stdout_hash = Some(commitment);
    refresh_proof_digest_and_signature(&mut receipt)?;
    let verifier = FixtureSignatureVerifier;
    let context = proof_context(&verifier);

    let verification = verify_harness_receipt_proof(&receipt, &context);

    assert_finding(
        &verification,
        ReceiptFindingCode::HashCommitmentProofMissing,
    );
    Ok(())
}

#[test]
fn strict_proof_rejects_missing_external_attestation() -> Result<(), serde_json::Error> {
    let receipt = proof_receipt()?;
    let verifier = FixtureSignatureVerifier;
    let mut context = proof_context(&verifier);
    context.external_attestations_verified = false;

    let verification = verify_harness_receipt_proof(&receipt, &context);

    assert_finding(
        &verification,
        ReceiptFindingCode::ExternalAttestationMissing,
    );
    Ok(())
}

#[test]
fn proof_status_projects_safe_public_summary() -> Result<(), serde_json::Error> {
    let receipt = proof_receipt()?;
    let verifier = FixtureSignatureVerifier;
    let verification = verify_harness_receipt_proof(&receipt, &proof_context(&verifier));

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

fn fixture(json: &str) -> Result<HarnessReceipt, serde_json::Error> {
    serde_json::from_str::<Fixture>(json).map(|fixture| fixture.expected)
}

fn proof_receipt() -> Result<HarnessReceipt, serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    refresh_proof_digest_and_signature(&mut receipt)?;
    Ok(receipt)
}

fn refresh_proof_digest_and_signature(
    receipt: &mut HarnessReceipt,
) -> Result<(), serde_json::Error> {
    let digest = canonical_receipt_body_digest(receipt)
        .map_err(|error| serde_json::Error::io(std::io::Error::other(error.to_string())))?;
    receipt.seal.digest = digest.clone();
    if let Some(seal) = receipt.harness.seal.as_mut() {
        seal.digest = digest.clone();
    }
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
