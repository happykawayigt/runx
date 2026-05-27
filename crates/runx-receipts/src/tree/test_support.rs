use std::collections::BTreeSet;

use runx_contracts::{Receipt, ReceiptIssuer, Reference, ReferenceType};
use serde::Deserialize;

use super::{ReceiptProofContextProvider, ReceiptResolveResult, ReceiptResolver, ResolvedReceipt};
use crate::{
    ReceiptFindingCode, ReceiptProofContext, ReceiptSignature, ReceiptVerification,
    SignatureVerificationFailure, SignatureVerifier, canonical_receipt_body_digest,
};

pub(super) const SUCCESS_RECEIPT: &str =
    include_str!("../../../../fixtures/contracts/harness-spine/receipt-success.json");
const ABNORMAL_RECEIPT: &str =
    include_str!("../../../../fixtures/contracts/harness-spine/receipt-abnormal.json");

#[derive(Debug, Deserialize)]
struct Fixture {
    expected: Receipt,
}

pub(super) fn child_refs_mut(receipt: &mut Receipt) -> &mut Vec<Reference> {
    &mut receipt
        .lineage
        .get_or_insert_with(Default::default)
        .children
}

pub(super) fn fixture(json: &str) -> Result<Receipt, serde_json::Error> {
    let mut receipt = serde_json::from_str::<Fixture>(json).map(|fixture| fixture.expected)?;
    // The flat success fixture carries no children; the tree tests need one
    // typed child ref to mutate, so seed a single receipt ref.
    if receipt
        .lineage
        .as_ref()
        .is_none_or(|lineage| lineage.children.is_empty())
    {
        child_refs_mut(&mut receipt)
            .push(Reference::runx(ReferenceType::Receipt, "hrn_rcpt_child_1"));
    }
    Ok(receipt)
}

pub(super) fn child(id: &str) -> Result<Receipt, serde_json::Error> {
    let mut receipt = fixture(ABNORMAL_RECEIPT)?;
    receipt.id = id.into();
    child_refs_mut(&mut receipt).clear();
    Ok(receipt)
}

pub(super) fn proof_root() -> Result<Receipt, serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    refresh_proof_digest_and_signature(&mut receipt)?;
    Ok(receipt)
}

pub(super) fn proof_child(id: &str) -> Result<Receipt, serde_json::Error> {
    let mut receipt = fixture(SUCCESS_RECEIPT)?;
    receipt.id = id.into();
    child_refs_mut(&mut receipt).clear();
    refresh_proof_digest_and_signature(&mut receipt)?;
    Ok(receipt)
}

pub(super) fn link_child_digest(
    root: &mut Receipt,
    index: usize,
    child: &Receipt,
) -> Result<(), serde_json::Error> {
    child_refs_mut(root)[index].locator = Some(child.digest.clone());
    refresh_proof_digest_and_signature(root)
}

pub(super) fn refresh_proof_digest_and_signature(
    receipt: &mut Receipt,
) -> Result<(), serde_json::Error> {
    let digest = canonical_receipt_body_digest(receipt)
        .map_err(|error| serde_json::Error::io(std::io::Error::other(error.to_string())))?;
    receipt.digest = digest.clone().into();
    receipt.signature.value = format!("sig:{digest}").into();
    Ok(())
}

pub(super) fn reference(reference_type: ReferenceType, id: &str) -> Reference {
    Reference::runx(reference_type, id)
}

pub(super) fn assert_finding(
    verification: &ReceiptVerification,
    code: ReceiptFindingCode,
    path: &str,
) {
    assert!(
        verification
            .findings
            .iter()
            .any(|finding| finding.code == code && finding.path == path),
        "expected finding {code:?} at {path}; got {:?}",
        verification.findings
    );
}

#[derive(Default)]
pub(super) struct FixtureProofContexts {
    verifier: FixtureSignatureVerifier,
}

impl ReceiptProofContextProvider for FixtureProofContexts {
    fn proof_context<'a>(&'a self, _receipt: &Receipt) -> ReceiptProofContext<'a> {
        ReceiptProofContext {
            signature_verifier: Some(&self.verifier),
            authority_verified: true,
            external_attestations_verified: true,
            verified_redaction_refs: BTreeSet::new(),
            verified_hash_commitments: BTreeSet::new(),
        }
    }
}

#[derive(Default)]
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

pub(super) struct AmbiguousResolver;

impl ReceiptResolver for AmbiguousResolver {
    fn resolve_child<'a>(&'a self, _reference: &Reference) -> ReceiptResolveResult<'a> {
        ReceiptResolveResult::Ambiguous
    }

    fn supplied_receipts<'a>(&'a self) -> Vec<ResolvedReceipt<'a>> {
        Vec::new()
    }
}

pub(super) struct ResolverErrorResolver;

impl ReceiptResolver for ResolverErrorResolver {
    fn resolve_child<'a>(&'a self, _reference: &Reference) -> ReceiptResolveResult<'a> {
        ReceiptResolveResult::ResolverError
    }

    fn supplied_receipts<'a>(&'a self) -> Vec<ResolvedReceipt<'a>> {
        Vec::new()
    }
}

pub(super) struct HiddenChildResolver<'a> {
    pub(super) child: &'a Receipt,
}

impl ReceiptResolver for HiddenChildResolver<'_> {
    fn resolve_child<'a>(&'a self, _reference: &Reference) -> ReceiptResolveResult<'a> {
        ReceiptResolveResult::Found(ResolvedReceipt {
            path: "hidden_child".to_owned(),
            receipt: self.child,
        })
    }

    fn supplied_receipts<'a>(&'a self) -> Vec<ResolvedReceipt<'a>> {
        Vec::new()
    }
}

pub(super) struct DuplicateIdResolver<'a> {
    pub(super) first: &'a Receipt,
    pub(super) second: &'a Receipt,
}

impl ReceiptResolver for DuplicateIdResolver<'_> {
    fn resolve_child<'a>(&'a self, reference: &Reference) -> ReceiptResolveResult<'a> {
        if reference.uri.ends_with(":first") {
            return ReceiptResolveResult::Found(ResolvedReceipt {
                path: "hidden_first".to_owned(),
                receipt: self.first,
            });
        }
        ReceiptResolveResult::Found(ResolvedReceipt {
            path: "hidden_second".to_owned(),
            receipt: self.second,
        })
    }

    fn supplied_receipts<'a>(&'a self) -> Vec<ResolvedReceipt<'a>> {
        Vec::new()
    }
}
