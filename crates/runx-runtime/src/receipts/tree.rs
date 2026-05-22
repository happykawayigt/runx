use runx_contracts::{Receipt, Reference, ReferenceType};
use runx_receipts::{
    ReceiptFindingCode, ReceiptResolveResult, ReceiptResolver, ReceiptTreeConfig,
    ReceiptVerification, ResolvedReceipt, verify_receipt_tree_proof_with_resolver,
};

use super::seal::{RuntimeReceiptProofContextProvider, RuntimeReceiptSignaturePolicy};

#[derive(Clone, Debug, Default)]
pub struct RuntimeReceiptResolver {
    receipts: Vec<Receipt>,
}

impl RuntimeReceiptResolver {
    #[must_use]
    pub fn new(receipts: impl IntoIterator<Item = Receipt>) -> Self {
        Self {
            receipts: receipts.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn receipts(&self) -> &[Receipt] {
        &self.receipts
    }
}

impl ReceiptResolver for RuntimeReceiptResolver {
    fn resolve_child<'a>(&'a self, reference: &Reference) -> ReceiptResolveResult<'a> {
        let Some(receipt_id) = referenced_receipt_id(reference) else {
            return ReceiptResolveResult::Malformed;
        };
        let mut matches = self
            .receipts
            .iter()
            .enumerate()
            .filter(|(_, receipt)| receipt.id == receipt_id);
        let Some((index, receipt)) = matches.next() else {
            return ReceiptResolveResult::Missing;
        };
        if matches.next().is_some() {
            return ReceiptResolveResult::Ambiguous;
        }
        ReceiptResolveResult::Found(ResolvedReceipt {
            path: runtime_receipt_path(index),
            receipt,
        })
    }

    fn supplied_receipts<'a>(&'a self) -> Vec<ResolvedReceipt<'a>> {
        self.receipts
            .iter()
            .enumerate()
            .map(|(index, receipt)| ResolvedReceipt {
                path: runtime_receipt_path(index),
                receipt,
            })
            .collect()
    }
}

pub fn validate_runtime_receipt_tree(
    root: &Receipt,
    receipts: impl IntoIterator<Item = Receipt>,
    config: ReceiptTreeConfig,
) -> Result<(), ReceiptVerification> {
    let verification = verify_runtime_receipt_tree(root, receipts, config);
    if verification.valid {
        Ok(())
    } else {
        Err(verification)
    }
}

pub fn validate_runtime_receipt_tree_with_policy(
    root: &Receipt,
    receipts: impl IntoIterator<Item = Receipt>,
    config: ReceiptTreeConfig,
    signature_policy: RuntimeReceiptSignaturePolicy<'_>,
) -> Result<(), ReceiptVerification> {
    let verification =
        verify_runtime_receipt_tree_with_policy(root, receipts, config, signature_policy);
    if verification.valid {
        Ok(())
    } else {
        Err(verification)
    }
}

#[must_use]
pub fn verify_runtime_receipt_tree(
    root: &Receipt,
    receipts: impl IntoIterator<Item = Receipt>,
    config: ReceiptTreeConfig,
) -> ReceiptVerification {
    verify_runtime_receipt_tree_with_policy(
        root,
        receipts,
        config,
        RuntimeReceiptSignaturePolicy::local_development(),
    )
}

#[must_use]
pub fn verify_runtime_receipt_tree_with_policy(
    root: &Receipt,
    receipts: impl IntoIterator<Item = Receipt>,
    config: ReceiptTreeConfig,
    signature_policy: RuntimeReceiptSignaturePolicy<'_>,
) -> ReceiptVerification {
    let resolver = RuntimeReceiptResolver::new(receipts);
    let proof_contexts = RuntimeReceiptProofContextProvider::new(signature_policy);
    let verification = verify_receipt_tree_proof_with_resolver(
        root,
        &resolver,
        runtime_receipt_tree_config(config),
        &proof_contexts,
    );
    // The decision -> act-id integrity property is journal-dependent and
    // reported as `unverified` by plain proof verification; the runtime confirms
    // it through the in-hand journal, so it is not a blocking tree finding.
    let blocking: Vec<_> = verification
        .findings
        .into_iter()
        .filter(|finding| {
            !matches!(finding.code, ReceiptFindingCode::DecisionIntegrityUnverified)
        })
        .collect();
    ReceiptVerification::from_findings(blocking)
}

fn runtime_receipt_path(index: usize) -> String {
    format!("runtime_receipts[{index}]")
}

fn runtime_receipt_tree_config(mut config: ReceiptTreeConfig) -> ReceiptTreeConfig {
    config.require_parent_links = true;
    config
}

fn referenced_receipt_id(reference: &Reference) -> Option<&str> {
    if reference.reference_type != ReferenceType::Receipt {
        return None;
    }
    reference
        .uri
        .strip_prefix("runx:receipt:")
        .filter(|id| !id.is_empty())
}
