//! Pure Rust harness receipt verification for runx.
//!
//! This crate owns the post-cutover receipt layer: a receipt is the sealed
//! proof of a harness node, with acts and decisions proven through that seal.

mod canonical;
mod tree;
mod verify;

pub use canonical::{
    canonical_receipt_body_digest, canonical_receipt_body_json, canonical_receipt_digest,
    canonical_receipt_json,
};
pub use runx_contracts::{
    RECEIPT_CANONICALIZATION, RECEIPT_SCHEMA, Receipt, ReceiptIssuer, ReceiptIssuerType,
    ReceiptSchema, ReceiptSignature, Seal, SignatureAlgorithm,
};
pub use tree::{
    ReceiptProofContextProvider, ReceiptResolveResult, ReceiptResolver, ReceiptTreeConfig,
    ResolvedReceipt, validate_receipt_tree, validate_receipt_tree_proof,
    validate_receipt_tree_proof_with_resolver, validate_receipt_tree_with_resolver,
    verify_receipt_tree, verify_receipt_tree_proof, verify_receipt_tree_proof_with_resolver,
    verify_receipt_tree_with_resolver,
};
pub use verify::{
    ReceiptError, ReceiptFinding, ReceiptFindingCode, ReceiptJournal, ReceiptJournalDecision,
    ReceiptProofContext, ReceiptProofFindingSummary, ReceiptProofStatus, ReceiptProofStatusKind,
    ReceiptVerification, SignatureVerificationFailure, SignatureVerifier,
    compute_verification_summary, receipt_proof_status, validate_receipt, validate_receipt_proof,
    validate_receipt_with_journal, verify_receipt, verify_receipt_proof,
    verify_receipt_with_journal,
};

#[cfg(test)]
mod tests {
    #[test]
    fn package_name_matches() {
        assert_eq!(env!("CARGO_PKG_NAME"), "runx-receipts");
    }
}
