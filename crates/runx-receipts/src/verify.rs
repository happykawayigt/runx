use std::collections::BTreeSet;

use runx_contracts::{
    AuthorityAttenuation, Receipt, ReceiptAct, ReceiptCommitment, ReceiptCriterion, Reference,
    ReferenceType, Seal,
};

mod finding;
mod journal;
mod projection;
mod proof;

pub use finding::{ReceiptError, ReceiptFinding, ReceiptFindingCode, ReceiptVerification};
pub use journal::{
    ReceiptJournal, ReceiptJournalDecision, validate_receipt_with_journal,
    verify_receipt_with_journal,
};
pub use projection::{
    ReceiptProofFindingSummary, ReceiptProofStatus, ReceiptProofStatusKind, receipt_proof_status,
};
pub use proof::{
    ReceiptProofContext, SignatureVerificationFailure, SignatureVerifier,
    compute_verification_summary, validate_receipt_proof, verify_receipt_proof,
};

pub fn validate_receipt(receipt: &Receipt) -> Result<(), ReceiptVerification> {
    let verification = verify_receipt(receipt);
    if verification.valid {
        Ok(())
    } else {
        Err(verification)
    }
}

#[must_use]
pub fn verify_receipt(receipt: &Receipt) -> ReceiptVerification {
    let mut verifier = Verifier::default();
    verifier.check_envelope(receipt);
    verifier.check_receipt(receipt);
    verifier.finish()
}

#[derive(Default)]
struct Verifier {
    findings: Vec<ReceiptFinding>,
}

impl Verifier {
    fn finish(self) -> ReceiptVerification {
        ReceiptVerification::from_findings(self.findings)
    }

    fn check_envelope(&mut self, receipt: &Receipt) {
        self.check_non_empty("id", &receipt.id);
        self.check_non_empty("created_at", &receipt.created_at);
        self.check_non_empty("canonicalization", &receipt.canonicalization);
        self.check_non_empty("issuer.kid", &receipt.issuer.kid);
        self.check_sha256_prefix("issuer.public_key_sha256", &receipt.issuer.public_key_sha256);
        self.check_non_empty("signature.value", &receipt.signature.value);
    }

    fn check_receipt(&mut self, receipt: &Receipt) {
        self.check_authority_attenuation("authority", &receipt.authority.attenuation);
        self.check_hash_prefixes(receipt);
        if let Some(lineage) = &receipt.lineage {
            self.check_child_receipt_refs("lineage.children", &lineage.children);
        }
        self.check_acts(&receipt.acts);
        self.check_seal_criteria(receipt, &receipt.seal);
    }

    fn check_authority_attenuation(&mut self, path: &str, attenuation: &AuthorityAttenuation) {
        match (&attenuation.parent_authority_ref, &attenuation.subset_proof) {
            (Some(parent), Some(proof)) if proof.parent_authority_ref == *parent => {}
            (Some(_), Some(_)) => self.push(
                ReceiptFindingCode::AuthorityAttenuationInvalid,
                format!("{path}.attenuation.subset_proof.parent_authority_ref"),
                "subset proof must cite the same parent authority ref",
            ),
            (Some(_), None) => self.push(
                ReceiptFindingCode::AuthorityAttenuationInvalid,
                format!("{path}.attenuation.subset_proof"),
                "parent authority refs require a subset proof",
            ),
            (None, Some(_)) => self.push(
                ReceiptFindingCode::AuthorityAttenuationInvalid,
                format!("{path}.attenuation.subset_proof"),
                "root authority must not carry a subset proof",
            ),
            (None, None) => {}
        }
    }

    fn check_hash_prefixes(&mut self, receipt: &Receipt) {
        self.check_sha256_prefix(
            "authority.enforcement.profile_hash",
            &receipt.authority.enforcement.profile_hash,
        );
        self.check_sha256_prefix("idempotency.intent_key", &receipt.idempotency.intent_key);
        self.check_sha256_prefix(
            "idempotency.trigger_fingerprint",
            &receipt.idempotency.trigger_fingerprint,
        );
        self.check_sha256_prefix("idempotency.content_hash", &receipt.idempotency.content_hash);
        self.check_sha256_prefix("digest", &receipt.digest);
        for (index, commitment) in receipt.subject.commitments.iter().enumerate() {
            self.check_commitment(&format!("subject.commitments[{index}]"), commitment);
        }
    }

    fn check_child_receipt_refs(&mut self, path: &str, refs: &[Reference]) {
        for (index, reference) in refs.iter().enumerate() {
            if reference.reference_type != ReferenceType::Receipt {
                self.push(
                    ReceiptFindingCode::ChildReceiptRefInvalid,
                    format!("{path}[{index}].type"),
                    "child receipt refs must use type receipt",
                );
            }
        }
    }

    fn check_acts(&mut self, acts: &[ReceiptAct]) {
        for (index, act) in acts.iter().enumerate() {
            if act.id.is_empty() {
                self.push(
                    ReceiptFindingCode::ActFormDetailsInvalid,
                    format!("acts[{index}].id"),
                    "acts must carry a non-empty id",
                );
            }
        }
    }

    fn check_seal_criteria(&mut self, receipt: &Receipt, seal: &Seal) {
        let act_criteria = act_criterion_ids(&receipt.acts);
        for (index, criterion) in seal.criteria.iter().enumerate() {
            let criterion_path = format!("seal.criteria[{index}]");
            // A rolled-up seal criterion must be backed by a per-act criterion
            // binding of the same id (the seal rolls up act criteria).
            if !receipt.acts.is_empty() && !act_criteria.contains(&criterion.criterion_id) {
                self.push(
                    ReceiptFindingCode::SealCriterionUnbound,
                    format!("{criterion_path}.criterion_id"),
                    "seal criterion must roll up an act criterion binding",
                );
            }
        }
    }

    fn check_commitment(&mut self, path: &str, commitment: &ReceiptCommitment) {
        self.check_sha256_prefix(&format!("{path}.value"), &commitment.value);
    }

    fn check_sha256_prefix(&mut self, path: &str, value: &str) {
        if !value.starts_with("sha256:") {
            self.push(
                ReceiptFindingCode::HashCommitmentInvalid,
                path,
                "hash values must use the sha256: prefix",
            );
        }
    }

    fn check_non_empty(&mut self, path: &str, value: &str) {
        if value.is_empty() {
            self.push(
                ReceiptFindingCode::EmptyEnvelopeField,
                path,
                "receipt envelope fields must not be empty",
            );
        }
    }

    fn push(
        &mut self,
        code: ReceiptFindingCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.findings.push(ReceiptFinding {
            code,
            path: path.into(),
            message: message.into(),
        });
    }
}

fn act_criterion_ids(acts: &[ReceiptAct]) -> BTreeSet<String> {
    acts.iter()
        .flat_map(|act| act.criteria.iter())
        .map(|criterion: &ReceiptCriterion| criterion.criterion_id.clone())
        .collect()
}

fn act_ids(acts: &[ReceiptAct]) -> BTreeSet<String> {
    acts.iter().map(|act| act.id.clone()).collect()
}
