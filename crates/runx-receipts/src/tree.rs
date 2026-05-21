// rust-style-allow: large-file -- tree traversal, resolver outcomes, and the
// adversarial unit matrix stay together until the resolver contract stabilizes.
use std::collections::{BTreeMap, BTreeSet};

use runx_contracts::{HarnessReceipt, Reference, ReferenceType};

use crate::{
    ReceiptFinding, ReceiptFindingCode, ReceiptProofContext, ReceiptVerification,
    validate_harness_receipt, verify_harness_receipt, verify_harness_receipt_proof,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiptTreeConfig {
    pub max_depth: usize,
    pub max_breadth: usize,
    pub require_parent_links: bool,
}

impl Default for ReceiptTreeConfig {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_breadth: 1024,
            require_parent_links: false,
        }
    }
}

pub trait ReceiptResolver {
    fn resolve_child<'a>(&'a self, reference: &Reference) -> ReceiptResolveResult<'a>;
    fn supplied_receipts<'a>(&'a self) -> Vec<ResolvedReceipt<'a>>;
}

pub trait ReceiptProofContextProvider {
    fn proof_context<'a>(&'a self, receipt: &HarnessReceipt) -> ReceiptProofContext<'a>;
}

#[derive(Clone, Debug)]
pub struct ResolvedReceipt<'a> {
    pub path: String,
    pub receipt: &'a HarnessReceipt,
}

#[derive(Clone, Debug)]
pub enum ReceiptResolveResult<'a> {
    Found(ResolvedReceipt<'a>),
    Missing,
    Malformed,
    Ambiguous,
}

pub fn validate_receipt_tree(
    root: &HarnessReceipt,
    children: &[HarnessReceipt],
) -> Result<(), ReceiptVerification> {
    let resolver = SliceReceiptResolver { children };
    validate_receipt_tree_with_resolver(root, &resolver, ReceiptTreeConfig::default())
}

#[must_use]
pub fn verify_receipt_tree(
    root: &HarnessReceipt,
    children: &[HarnessReceipt],
) -> ReceiptVerification {
    let resolver = SliceReceiptResolver { children };
    verify_receipt_tree_with_resolver(root, &resolver, ReceiptTreeConfig::default())
}

pub fn validate_receipt_tree_proof(
    root: &HarnessReceipt,
    children: &[HarnessReceipt],
    proof_contexts: &impl ReceiptProofContextProvider,
) -> Result<(), ReceiptVerification> {
    let resolver = SliceReceiptResolver { children };
    validate_receipt_tree_proof_with_resolver(
        root,
        &resolver,
        ReceiptTreeConfig::default(),
        proof_contexts,
    )
}

#[must_use]
pub fn verify_receipt_tree_proof(
    root: &HarnessReceipt,
    children: &[HarnessReceipt],
    proof_contexts: &impl ReceiptProofContextProvider,
) -> ReceiptVerification {
    let resolver = SliceReceiptResolver { children };
    verify_receipt_tree_proof_with_resolver(
        root,
        &resolver,
        ReceiptTreeConfig::default(),
        proof_contexts,
    )
}

pub fn validate_receipt_tree_with_resolver(
    root: &HarnessReceipt,
    resolver: &impl ReceiptResolver,
    config: ReceiptTreeConfig,
) -> Result<(), ReceiptVerification> {
    let verification = verify_receipt_tree_with_resolver(root, resolver, config);
    if verification.valid {
        Ok(())
    } else {
        Err(verification)
    }
}

#[must_use]
pub fn verify_receipt_tree_with_resolver(
    root: &HarnessReceipt,
    resolver: &impl ReceiptResolver,
    config: ReceiptTreeConfig,
) -> ReceiptVerification {
    let mut findings = verify_harness_receipt(root).findings;
    let supplied = resolver.supplied_receipts();
    findings.extend(duplicate_child_findings(&supplied));
    findings.extend(child_receipt_findings(&supplied));
    verify_tree_relationships(root, resolver, config, &supplied, findings)
}

pub fn validate_receipt_tree_proof_with_resolver(
    root: &HarnessReceipt,
    resolver: &impl ReceiptResolver,
    config: ReceiptTreeConfig,
    proof_contexts: &impl ReceiptProofContextProvider,
) -> Result<(), ReceiptVerification> {
    let verification =
        verify_receipt_tree_proof_with_resolver(root, resolver, config, proof_contexts);
    if verification.valid {
        Ok(())
    } else {
        Err(verification)
    }
}

#[must_use]
pub fn verify_receipt_tree_proof_with_resolver(
    root: &HarnessReceipt,
    resolver: &impl ReceiptResolver,
    config: ReceiptTreeConfig,
    proof_contexts: &impl ReceiptProofContextProvider,
) -> ReceiptVerification {
    let root_context = proof_contexts.proof_context(root);
    let mut findings = verify_harness_receipt_proof(root, &root_context).findings;
    let supplied = resolver.supplied_receipts();
    findings.extend(duplicate_child_findings(&supplied));
    findings.extend(child_receipt_proof_findings(&supplied, proof_contexts));
    verify_tree_relationships_with_proof(
        root,
        resolver,
        config,
        &supplied,
        findings,
        proof_contexts,
    )
}

fn verify_tree_relationships<R: ReceiptResolver>(
    root: &HarnessReceipt,
    resolver: &R,
    config: ReceiptTreeConfig,
    supplied: &[ResolvedReceipt<'_>],
    mut findings: Vec<ReceiptFinding>,
) -> ReceiptVerification {
    let mut traversal = TreeTraversal {
        resolver,
        config,
        proof_policy: StructuralChildProofPolicy,
        visiting: BTreeSet::new(),
        reached: BTreeSet::new(),
    };
    findings.extend(traversal.subtree_findings("harness", root, 0));
    findings.extend(orphan_child_findings(supplied, &traversal.reached));
    ReceiptVerification::from_findings(findings)
}

fn verify_tree_relationships_with_proof<R: ReceiptResolver>(
    root: &HarnessReceipt,
    resolver: &R,
    config: ReceiptTreeConfig,
    supplied: &[ResolvedReceipt<'_>],
    mut findings: Vec<ReceiptFinding>,
    proof_contexts: &impl ReceiptProofContextProvider,
) -> ReceiptVerification {
    let mut traversal = TreeTraversal {
        resolver,
        config,
        proof_policy: StrictChildProofPolicy::new(supplied, proof_contexts),
        visiting: BTreeSet::new(),
        reached: BTreeSet::new(),
    };
    findings.extend(traversal.subtree_findings("harness", root, 0));
    findings.extend(orphan_child_findings(supplied, &traversal.reached));
    ReceiptVerification::from_findings(findings)
}

struct SliceReceiptResolver<'a> {
    children: &'a [HarnessReceipt],
}

impl ReceiptResolver for SliceReceiptResolver<'_> {
    fn resolve_child<'a>(&'a self, reference: &Reference) -> ReceiptResolveResult<'a> {
        let Some(receipt_id) = referenced_receipt_id(reference) else {
            return ReceiptResolveResult::Malformed;
        };
        let mut matches = self
            .children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.id == receipt_id);
        let Some((index, receipt)) = matches.next() else {
            return ReceiptResolveResult::Missing;
        };
        if matches.next().is_some() {
            return ReceiptResolveResult::Ambiguous;
        }
        ReceiptResolveResult::Found(ResolvedReceipt {
            path: format!("children[{index}]"),
            receipt,
        })
    }

    fn supplied_receipts<'a>(&'a self) -> Vec<ResolvedReceipt<'a>> {
        self.children
            .iter()
            .enumerate()
            .map(|(index, receipt)| ResolvedReceipt {
                path: format!("children[{index}]"),
                receipt,
            })
            .collect()
    }
}

fn duplicate_child_findings(children: &[ResolvedReceipt<'_>]) -> Vec<ReceiptFinding> {
    let mut seen = BTreeMap::new();
    children
        .iter()
        .filter_map(|child| {
            if seen
                .insert(child.receipt.id.as_str(), child.path.as_str())
                .is_some()
            {
                Some(ReceiptFinding {
                    code: ReceiptFindingCode::DuplicateChildReceipt,
                    path: format!("{}.id", child.path),
                    message: "child receipt ids must be unique".to_owned(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn child_receipt_findings(children: &[ResolvedReceipt<'_>]) -> Vec<ReceiptFinding> {
    children
        .iter()
        .flat_map(|child| {
            validate_harness_receipt(child.receipt)
                .err()
                .map_or_else(Vec::new, |verification| {
                    verification
                        .findings
                        .into_iter()
                        .map(|finding| child_finding(&child.path, finding))
                        .collect()
                })
        })
        .collect()
}

fn child_receipt_proof_findings(
    children: &[ResolvedReceipt<'_>],
    proof_contexts: &impl ReceiptProofContextProvider,
) -> Vec<ReceiptFinding> {
    children
        .iter()
        .flat_map(|child| {
            let context = proof_contexts.proof_context(child.receipt);
            verify_harness_receipt_proof(child.receipt, &context)
                .findings
                .into_iter()
                .map(|finding| child_finding(&child.path, finding))
                .collect::<Vec<_>>()
        })
        .collect()
}

trait ChildProofPolicy {
    fn findings(
        &mut self,
        path: &str,
        reference: &Reference,
        receipt: &HarnessReceipt,
    ) -> Vec<ReceiptFinding>;
}

struct StructuralChildProofPolicy;

impl ChildProofPolicy for StructuralChildProofPolicy {
    fn findings(
        &mut self,
        _path: &str,
        _reference: &Reference,
        _receipt: &HarnessReceipt,
    ) -> Vec<ReceiptFinding> {
        Vec::new()
    }
}

struct StrictChildProofPolicy<'a, P: ReceiptProofContextProvider> {
    proof_contexts: &'a P,
    verified_receipts: BTreeSet<usize>,
}

impl<'a, P: ReceiptProofContextProvider> StrictChildProofPolicy<'a, P> {
    fn new(supplied: &[ResolvedReceipt<'_>], proof_contexts: &'a P) -> Self {
        Self {
            proof_contexts,
            verified_receipts: supplied
                .iter()
                .map(|child| receipt_address(child.receipt))
                .collect(),
        }
    }
}

impl<P: ReceiptProofContextProvider> ChildProofPolicy for StrictChildProofPolicy<'_, P> {
    fn findings(
        &mut self,
        path: &str,
        reference: &Reference,
        receipt: &HarnessReceipt,
    ) -> Vec<ReceiptFinding> {
        let mut findings = child_digest_link_findings(path, reference, receipt);
        if !self.verified_receipts.insert(receipt_address(receipt)) {
            return findings;
        }
        let context = self.proof_contexts.proof_context(receipt);
        findings.extend(
            verify_harness_receipt_proof(receipt, &context)
                .findings
                .into_iter()
                .map(|finding| child_finding(path, finding)),
        );
        findings
    }
}

struct TreeTraversal<'a, R: ReceiptResolver, P: ChildProofPolicy> {
    resolver: &'a R,
    config: ReceiptTreeConfig,
    proof_policy: P,
    visiting: BTreeSet<String>,
    reached: BTreeSet<String>,
}

impl<R: ReceiptResolver, P: ChildProofPolicy> TreeTraversal<'_, R, P> {
    fn subtree_findings(
        &mut self,
        path: &str,
        receipt: &HarnessReceipt,
        depth: usize,
    ) -> Vec<ReceiptFinding> {
        if !self.visiting.insert(receipt.id.clone()) {
            return vec![ReceiptFinding {
                code: ReceiptFindingCode::ChildReceiptCycle,
                path: format!("{path}.id"),
                message: "child harness receipt refs must not form cycles".to_owned(),
            }];
        }

        let mut findings = Vec::new();
        let child_refs = &receipt.harness.child_harness_receipt_refs;
        if child_refs.len() > self.config.max_breadth {
            findings.push(ReceiptFinding {
                code: ReceiptFindingCode::ChildReceiptBreadthLimit,
                path: format!("{path}.child_harness_receipt_refs"),
                message: "child harness receipt refs exceed configured breadth limit".to_owned(),
            });
        }

        let child_findings = receipt
            .harness
            .child_harness_receipt_refs
            .iter()
            .take(self.config.max_breadth)
            .enumerate()
            .flat_map(|(index, reference)| {
                self.child_ref_findings(
                    &format!("{path}.child_harness_receipt_refs[{index}]"),
                    receipt,
                    reference,
                    depth,
                )
            })
            .collect::<Vec<_>>();
        findings.extend(child_findings);
        self.visiting.remove(receipt.id.as_str());
        findings
    }

    fn child_ref_findings(
        &mut self,
        path: &str,
        parent: &HarnessReceipt,
        reference: &Reference,
        depth: usize,
    ) -> Vec<ReceiptFinding> {
        if reference.reference_type != ReferenceType::HarnessReceipt {
            return vec![malformed_child_ref(path)];
        };
        let next_depth = depth.saturating_add(1);
        if next_depth > self.config.max_depth {
            return vec![ReceiptFinding {
                code: ReceiptFindingCode::ChildReceiptDepthLimit,
                path: path.to_owned(),
                message: "child harness receipt refs exceed configured depth limit".to_owned(),
            }];
        };
        let resolved = match self.resolver.resolve_child(reference) {
            ReceiptResolveResult::Found(resolved) => resolved,
            ReceiptResolveResult::Missing => return vec![missing_child(path)],
            ReceiptResolveResult::Malformed => return vec![malformed_child_ref(path)],
            ReceiptResolveResult::Ambiguous => return vec![ambiguous_child(path)],
        };
        let child = resolved.receipt;
        if self.visiting.contains(child.id.as_str()) {
            return vec![ReceiptFinding {
                code: ReceiptFindingCode::ChildReceiptCycle,
                path: path.to_owned(),
                message: "child harness receipt refs must not point to an ancestor".to_owned(),
            }];
        }
        let child_path = format!("{}.harness", resolved.path);
        let mut findings = self.proof_policy.findings(&resolved.path, reference, child);
        if self.reached.contains(child.id.as_str()) {
            return findings;
        }
        findings.extend(parent_link_findings(path, parent, child, self.config));
        findings.extend(self.subtree_findings(&child_path, child, next_depth));
        self.reached.insert(child.id.clone());
        findings
    }
}

fn receipt_address(receipt: &HarnessReceipt) -> usize {
    receipt as *const HarnessReceipt as usize
}

fn orphan_child_findings(
    children: &[ResolvedReceipt<'_>],
    reached: &BTreeSet<String>,
) -> Vec<ReceiptFinding> {
    children
        .iter()
        .filter(|child| !reached.contains(child.receipt.id.as_str()))
        .map(|child| ReceiptFinding {
            code: ReceiptFindingCode::OrphanChildReceipt,
            path: format!("{}.id", child.path),
            message: "supplied child receipts must be reachable from the root receipt".to_owned(),
        })
        .collect()
}

fn missing_child(path: &str) -> ReceiptFinding {
    ReceiptFinding {
        code: ReceiptFindingCode::ChildReceiptMissing,
        path: path.to_owned(),
        message: "child harness receipt ref must resolve to a supplied child receipt".to_owned(),
    }
}

fn malformed_child_ref(path: &str) -> ReceiptFinding {
    ReceiptFinding {
        code: ReceiptFindingCode::ChildReceiptRefMalformed,
        path: path.to_owned(),
        message: "child harness receipt ref must be a typed runx harness receipt URI or exact id"
            .to_owned(),
    }
}

fn ambiguous_child(path: &str) -> ReceiptFinding {
    ReceiptFinding {
        code: ReceiptFindingCode::ChildReceiptAmbiguous,
        path: path.to_owned(),
        message: "child harness receipt ref resolved to multiple supplied receipts".to_owned(),
    }
}

fn parent_link_findings(
    path: &str,
    parent: &HarnessReceipt,
    child: &HarnessReceipt,
    config: ReceiptTreeConfig,
) -> Vec<ReceiptFinding> {
    match &child.harness.parent_harness_ref {
        Some(parent_ref) if parent_ref == &parent.harness.harness_ref => Vec::new(),
        Some(_) => vec![ReceiptFinding {
            code: ReceiptFindingCode::ChildReceiptParentMismatch,
            path: format!("{path}.parent_harness_ref"),
            message: "child harness parent ref must match the parent harness ref".to_owned(),
        }],
        None if config.require_parent_links => vec![ReceiptFinding {
            code: ReceiptFindingCode::ChildReceiptParentMismatch,
            path: format!("{path}.parent_harness_ref"),
            message: "strict tree verification requires child harness parent refs".to_owned(),
        }],
        None => Vec::new(),
    }
}

fn child_digest_link_findings(
    path: &str,
    reference: &Reference,
    child: &HarnessReceipt,
) -> Vec<ReceiptFinding> {
    if reference.locator.as_deref() == Some(child.seal.digest.as_str()) {
        return Vec::new();
    }
    vec![ReceiptFinding {
        code: ReceiptFindingCode::ChildReceiptDigestMismatch,
        path: format!("{path}.locator"),
        message: "strict tree proof requires child harness receipt refs to carry the exact child receipt digest".to_owned(),
    }]
}

fn child_finding(path: &str, finding: ReceiptFinding) -> ReceiptFinding {
    ReceiptFinding {
        path: format!("{path}.{}", finding.path),
        ..finding
    }
}

fn referenced_receipt_id(reference: &Reference) -> Option<&str> {
    if reference.reference_type != ReferenceType::HarnessReceipt {
        return None;
    }
    reference
        .uri
        .strip_prefix("runx:harness_receipt:")
        .filter(|id| !id.is_empty())
        .or_else(|| {
            reference
                .uri
                .as_str()
                .split_once(':')
                .is_none()
                .then_some(reference.uri.as_str())
        })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::{
        ReceiptProofContextProvider, ReceiptResolveResult, ReceiptResolver, ReceiptTreeConfig,
        ResolvedReceipt, validate_receipt_tree_proof, validate_receipt_tree_with_resolver,
        verify_receipt_tree, verify_receipt_tree_proof, verify_receipt_tree_proof_with_resolver,
        verify_receipt_tree_with_resolver,
    };
    use crate::{
        ReceiptFindingCode, ReceiptProofContext, ReceiptSignature, ReceiptVerification,
        SignatureVerificationFailure, SignatureVerifier, canonical_receipt_body_digest,
    };
    use runx_contracts::{HarnessReceipt, ReceiptIssuer, Reference, ReferenceType};

    const SUCCESS_RECEIPT: &str =
        include_str!("../../../fixtures/contracts/harness-spine/harness-receipt-success.json");
    const ABNORMAL_RECEIPT: &str =
        include_str!("../../../fixtures/contracts/harness-spine/harness-receipt-abnormal.json");

    #[derive(Debug, Deserialize)]
    struct Fixture {
        expected: HarnessReceipt,
    }

    #[test]
    fn slice_adapter_accepts_exact_id_and_typed_uri() -> Result<(), serde_json::Error> {
        let mut root = fixture(SUCCESS_RECEIPT)?;
        let child = child("hrn_rcpt_child_1")?;

        root.harness.child_harness_receipt_refs[0].uri = "hrn_rcpt_child_1".to_owned();
        assert!(verify_receipt_tree(&root, std::slice::from_ref(&child)).valid);

        root.harness.child_harness_receipt_refs[0].uri =
            "runx:harness_receipt:hrn_rcpt_child_1".to_owned();
        assert!(verify_receipt_tree(&root, &[child]).valid);
        Ok(())
    }

    #[test]
    fn malformed_and_wrong_namespace_refs_are_stable_findings() -> Result<(), serde_json::Error> {
        let mut root = fixture(SUCCESS_RECEIPT)?;
        let child = child("hrn_rcpt_child_1")?;

        root.harness.child_harness_receipt_refs[0].uri = "runx:receipt:hrn_rcpt_child_1".to_owned();
        let verification = verify_receipt_tree(&root, std::slice::from_ref(&child));
        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptRefMalformed,
            "harness.child_harness_receipt_refs[0]",
        );

        root.harness.child_harness_receipt_refs[0].uri = ":hrn_rcpt_child_1".to_owned();
        let verification = verify_receipt_tree(&root, &[child]);
        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptRefMalformed,
            "harness.child_harness_receipt_refs[0]",
        );
        Ok(())
    }

    #[test]
    fn suffix_only_refs_do_not_resolve_by_suffix() -> Result<(), serde_json::Error> {
        let mut root = fixture(SUCCESS_RECEIPT)?;
        root.harness.child_harness_receipt_refs[0].uri = "child_1".to_owned();
        let child = child("hrn_rcpt_child_1")?;

        let verification = verify_receipt_tree(&root, &[child]);

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptMissing,
            "harness.child_harness_receipt_refs[0]",
        );
        Ok(())
    }

    #[test]
    fn duplicate_ids_make_slice_resolution_ambiguous() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;
        let first = child("hrn_rcpt_child_1")?;
        let second = child("hrn_rcpt_child_1")?;

        let verification = verify_receipt_tree(&root, &[first, second]);

        assert_finding(
            &verification,
            ReceiptFindingCode::DuplicateChildReceipt,
            "children[1].id",
        );
        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptAmbiguous,
            "harness.child_harness_receipt_refs[0]",
        );
        Ok(())
    }

    #[test]
    fn resolver_ambiguous_result_is_a_stable_finding() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;

        let verification = verify_receipt_tree_with_resolver(
            &root,
            &AmbiguousResolver,
            ReceiptTreeConfig::default(),
        );

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptAmbiguous,
            "harness.child_harness_receipt_refs[0]",
        );
        Ok(())
    }

    #[test]
    fn strict_mode_rejects_mismatched_parent_link() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;
        let mut child = child("hrn_rcpt_child_1")?;
        child.harness.parent_harness_ref = Some(reference(ReferenceType::Harness, "other"));

        let verification = verify_receipt_tree_with_resolver(
            &root,
            &super::SliceReceiptResolver {
                children: std::slice::from_ref(&child),
            },
            ReceiptTreeConfig {
                require_parent_links: true,
                ..ReceiptTreeConfig::default()
            },
        );

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptParentMismatch,
            "harness.child_harness_receipt_refs[0].parent_harness_ref",
        );
        Ok(())
    }

    #[test]
    fn strict_mode_requires_present_parent_link() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;
        let child = child("hrn_rcpt_child_1")?;

        let verification = verify_receipt_tree_with_resolver(
            &root,
            &super::SliceReceiptResolver {
                children: std::slice::from_ref(&child),
            },
            ReceiptTreeConfig {
                require_parent_links: true,
                ..ReceiptTreeConfig::default()
            },
        );

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptParentMismatch,
            "harness.child_harness_receipt_refs[0].parent_harness_ref",
        );
        Ok(())
    }

    #[test]
    fn depth_limit_blocks_hostile_nested_tree() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;
        let mut child_receipt = child("hrn_rcpt_child_1")?;
        child_receipt
            .harness
            .child_harness_receipt_refs
            .push(reference(ReferenceType::HarnessReceipt, "grandchild"));
        let grandchild = child("grandchild")?;

        let verification = verify_receipt_tree_with_resolver(
            &root,
            &super::SliceReceiptResolver {
                children: &[child_receipt, grandchild],
            },
            ReceiptTreeConfig {
                max_depth: 1,
                ..ReceiptTreeConfig::default()
            },
        );

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptDepthLimit,
            "children[0].harness.child_harness_receipt_refs[0]",
        );
        Ok(())
    }

    #[test]
    fn breadth_limit_blocks_hostile_fanout() -> Result<(), serde_json::Error> {
        let mut root = fixture(SUCCESS_RECEIPT)?;
        root.harness
            .child_harness_receipt_refs
            .push(reference(ReferenceType::HarnessReceipt, "second"));
        let first = child("hrn_rcpt_child_1")?;
        let second = child("second")?;

        let verification = verify_receipt_tree_with_resolver(
            &root,
            &super::SliceReceiptResolver {
                children: &[first, second],
            },
            ReceiptTreeConfig {
                max_breadth: 1,
                ..ReceiptTreeConfig::default()
            },
        );

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptBreadthLimit,
            "harness.child_harness_receipt_refs",
        );
        Ok(())
    }

    #[test]
    fn positive_nested_tree_verifies() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;
        let mut child_receipt = child("hrn_rcpt_child_1")?;
        child_receipt
            .harness
            .child_harness_receipt_refs
            .push(reference(ReferenceType::HarnessReceipt, "grandchild"));
        let grandchild = child("grandchild")?;

        assert!(verify_receipt_tree(&root, &[child_receipt, grandchild]).valid);
        Ok(())
    }

    #[test]
    fn positive_fanout_tree_verifies() -> Result<(), serde_json::Error> {
        let mut root = fixture(SUCCESS_RECEIPT)?;
        root.harness
            .child_harness_receipt_refs
            .push(reference(ReferenceType::HarnessReceipt, "second"));
        let first = child("hrn_rcpt_child_1")?;
        let second = child("second")?;

        assert!(verify_receipt_tree(&root, &[first, second]).valid);
        Ok(())
    }

    #[test]
    fn strict_parent_links_can_verify_cleanly() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;
        let mut child = child("hrn_rcpt_child_1")?;
        child.harness.parent_harness_ref = Some(root.harness.harness_ref.clone());

        assert!(
            validate_receipt_tree_with_resolver(
                &root,
                &super::SliceReceiptResolver {
                    children: std::slice::from_ref(&child),
                },
                ReceiptTreeConfig {
                    require_parent_links: true,
                    ..ReceiptTreeConfig::default()
                },
            )
            .is_ok()
        );
        Ok(())
    }

    #[test]
    fn strict_tree_proof_accepts_root_and_child() -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let child = proof_child("hrn_rcpt_child_1")?;
        link_child_digest(&mut root, 0, &child)?;
        let proof_contexts = FixtureProofContexts::default();

        assert!(validate_receipt_tree_proof(&root, &[child], &proof_contexts).is_ok());
        Ok(())
    }

    #[test]
    fn strict_tree_proof_rejects_missing_child() -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let child = proof_child("hrn_rcpt_child_1")?;
        link_child_digest(&mut root, 0, &child)?;
        let proof_contexts = FixtureProofContexts::default();

        let verification = verify_receipt_tree_proof(&root, &[], &proof_contexts);

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptMissing,
            "harness.child_harness_receipt_refs[0]",
        );
        Ok(())
    }

    #[test]
    fn strict_tree_proof_rejects_extra_child() -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let child = proof_child("hrn_rcpt_child_1")?;
        link_child_digest(&mut root, 0, &child)?;
        let extra = proof_child("hrn_rcpt_extra")?;
        let proof_contexts = FixtureProofContexts::default();

        let verification = verify_receipt_tree_proof(&root, &[child, extra], &proof_contexts);

        assert_finding(
            &verification,
            ReceiptFindingCode::OrphanChildReceipt,
            "children[1].id",
        );
        Ok(())
    }

    #[test]
    fn strict_tree_proof_rejects_structurally_valid_root_proof_mismatch()
    -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let child = proof_child("hrn_rcpt_child_1")?;
        link_child_digest(&mut root, 0, &child)?;
        root.harness.child_harness_receipt_refs[0].uri = child.id.clone();
        let proof_contexts = FixtureProofContexts::default();

        assert!(verify_receipt_tree(&root, std::slice::from_ref(&child)).valid);
        let verification = verify_receipt_tree_proof(&root, &[child], &proof_contexts);

        assert_finding(
            &verification,
            ReceiptFindingCode::SealDigestMismatch,
            "seal.digest",
        );
        assert_finding(
            &verification,
            ReceiptFindingCode::SignatureInvalid,
            "signature.value",
        );
        Ok(())
    }

    #[test]
    fn strict_tree_proof_rejects_structurally_valid_child_proof_mismatch()
    -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let mut child = proof_child("hrn_rcpt_child_1")?;
        link_child_digest(&mut root, 0, &child)?;
        child.harness.acts[0].summary = "tampered child proof body".to_owned();
        let proof_contexts = FixtureProofContexts::default();

        assert!(verify_receipt_tree(&root, std::slice::from_ref(&child)).valid);
        let verification = verify_receipt_tree_proof(&root, &[child], &proof_contexts);

        assert_finding(
            &verification,
            ReceiptFindingCode::SealDigestMismatch,
            "children[0].seal.digest",
        );
        assert_finding(
            &verification,
            ReceiptFindingCode::SignatureInvalid,
            "children[0].signature.value",
        );
        Ok(())
    }

    #[test]
    fn strict_tree_proof_rejects_valid_alternate_child_with_same_id()
    -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let original = proof_child("hrn_rcpt_child_1")?;
        link_child_digest(&mut root, 0, &original)?;
        let mut alternate = proof_child("hrn_rcpt_child_1")?;
        alternate.harness.acts[0].summary = "valid alternate child body".to_owned();
        refresh_proof_digest_and_signature(&mut alternate)?;
        let proof_contexts = FixtureProofContexts::default();

        let verification = verify_receipt_tree_proof(&root, &[alternate], &proof_contexts);

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptDigestMismatch,
            "children[0].locator",
        );
        Ok(())
    }

    #[test]
    fn strict_tree_proof_rejects_custom_resolver_child_not_in_supplied_receipts()
    -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let mut child = proof_child("hrn_rcpt_child_1")?;
        link_child_digest(&mut root, 0, &child)?;
        child.harness.acts[0].summary = "hidden tampered child".to_owned();
        let resolver = HiddenChildResolver { child: &child };
        let proof_contexts = FixtureProofContexts::default();

        assert!(
            verify_receipt_tree_with_resolver(&root, &resolver, ReceiptTreeConfig::default()).valid
        );
        let verification = verify_receipt_tree_proof_with_resolver(
            &root,
            &resolver,
            ReceiptTreeConfig::default(),
            &proof_contexts,
        );

        assert_finding(
            &verification,
            ReceiptFindingCode::SealDigestMismatch,
            "hidden_child.seal.digest",
        );
        assert_finding(
            &verification,
            ReceiptFindingCode::SignatureInvalid,
            "hidden_child.signature.value",
        );
        Ok(())
    }

    #[test]
    fn strict_tree_proof_rejects_custom_resolver_duplicate_id_child_after_reached()
    -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let first = proof_child("shared_child")?;
        let mut second = proof_child("shared_child")?;
        root.harness.child_harness_receipt_refs = vec![
            reference(ReferenceType::HarnessReceipt, "first"),
            reference(ReferenceType::HarnessReceipt, "second"),
        ];
        root.harness.child_harness_receipt_refs[0].locator = Some(first.seal.digest.clone());
        root.harness.child_harness_receipt_refs[1].locator = Some(second.seal.digest.clone());
        refresh_proof_digest_and_signature(&mut root)?;
        second.harness.acts[0].summary = "hidden duplicate-id tamper".to_owned();
        let resolver = DuplicateIdResolver {
            first: &first,
            second: &second,
        };
        let proof_contexts = FixtureProofContexts::default();

        let verification = verify_receipt_tree_proof_with_resolver(
            &root,
            &resolver,
            ReceiptTreeConfig::default(),
            &proof_contexts,
        );

        assert_finding(
            &verification,
            ReceiptFindingCode::SealDigestMismatch,
            "hidden_second.seal.digest",
        );
        assert_finding(
            &verification,
            ReceiptFindingCode::SignatureInvalid,
            "hidden_second.signature.value",
        );
        Ok(())
    }

    fn fixture(json: &str) -> Result<HarnessReceipt, serde_json::Error> {
        serde_json::from_str::<Fixture>(json).map(|fixture| fixture.expected)
    }

    fn child(id: &str) -> Result<HarnessReceipt, serde_json::Error> {
        let mut receipt = fixture(ABNORMAL_RECEIPT)?;
        receipt.id = id.to_owned();
        Ok(receipt)
    }

    fn proof_root() -> Result<HarnessReceipt, serde_json::Error> {
        let mut receipt = fixture(SUCCESS_RECEIPT)?;
        refresh_proof_digest_and_signature(&mut receipt)?;
        Ok(receipt)
    }

    fn proof_child(id: &str) -> Result<HarnessReceipt, serde_json::Error> {
        let mut receipt = fixture(SUCCESS_RECEIPT)?;
        receipt.id = id.to_owned();
        receipt.harness.child_harness_receipt_refs.clear();
        refresh_proof_digest_and_signature(&mut receipt)?;
        Ok(receipt)
    }

    fn link_child_digest(
        root: &mut HarnessReceipt,
        index: usize,
        child: &HarnessReceipt,
    ) -> Result<(), serde_json::Error> {
        root.harness.child_harness_receipt_refs[index].locator = Some(child.seal.digest.clone());
        refresh_proof_digest_and_signature(root)
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

    fn reference(reference_type: ReferenceType, id: &str) -> Reference {
        Reference {
            uri: format!("runx:{}:{id}", reference_type_name(&reference_type)),
            reference_type,
            provider: None,
            locator: None,
            label: None,
            observed_at: None,
            proof_kind: None,
        }
    }

    fn reference_type_name(reference_type: &ReferenceType) -> &'static str {
        match reference_type {
            ReferenceType::HarnessReceipt => "harness_receipt",
            ReferenceType::Harness => "harness",
            _ => "reference",
        }
    }

    fn assert_finding(verification: &ReceiptVerification, code: ReceiptFindingCode, path: &str) {
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
    struct FixtureProofContexts {
        verifier: FixtureSignatureVerifier,
    }

    impl ReceiptProofContextProvider for FixtureProofContexts {
        fn proof_context<'a>(&'a self, _receipt: &HarnessReceipt) -> ReceiptProofContext<'a> {
            ReceiptProofContext {
                signature_verifier: Some(&self.verifier),
                authority_verified: true,
                external_attestations_verified: true,
                verified_redaction_refs: std::collections::BTreeSet::new(),
                verified_hash_commitments: std::collections::BTreeSet::new(),
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

    struct AmbiguousResolver;

    impl ReceiptResolver for AmbiguousResolver {
        fn resolve_child<'a>(&'a self, _reference: &Reference) -> ReceiptResolveResult<'a> {
            ReceiptResolveResult::Ambiguous
        }

        fn supplied_receipts<'a>(&'a self) -> Vec<ResolvedReceipt<'a>> {
            Vec::new()
        }
    }

    struct HiddenChildResolver<'a> {
        child: &'a HarnessReceipt,
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

    struct DuplicateIdResolver<'a> {
        first: &'a HarnessReceipt,
        second: &'a HarnessReceipt,
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
}
