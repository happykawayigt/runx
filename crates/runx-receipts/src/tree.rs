// rust-style-allow: large-file -- tree traversal, resolver outcomes, and the
// adversarial unit matrix stay together until the resolver contract stabilizes.
use std::collections::{BTreeMap, BTreeSet};

use runx_contracts::{Receipt, Reference, ReferenceType};

use crate::{
    ReceiptFinding, ReceiptFindingCode, ReceiptProofContext, ReceiptVerification,
    validate_receipt, verify_receipt, verify_receipt_proof,
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
    fn proof_context<'a>(&'a self, receipt: &Receipt) -> ReceiptProofContext<'a>;
}

#[derive(Clone, Debug)]
pub struct ResolvedReceipt<'a> {
    pub path: String,
    pub receipt: &'a Receipt,
}

#[derive(Clone, Debug)]
pub enum ReceiptResolveResult<'a> {
    Found(ResolvedReceipt<'a>),
    Missing,
    Malformed,
    Ambiguous,
    ResolverError,
}

pub fn validate_receipt_tree(
    root: &Receipt,
    children: &[Receipt],
) -> Result<(), ReceiptVerification> {
    let resolver = SliceReceiptResolver { children };
    validate_receipt_tree_with_resolver(root, &resolver, ReceiptTreeConfig::default())
}

#[must_use]
pub fn verify_receipt_tree(
    root: &Receipt,
    children: &[Receipt],
) -> ReceiptVerification {
    let resolver = SliceReceiptResolver { children };
    verify_receipt_tree_with_resolver(root, &resolver, ReceiptTreeConfig::default())
}

pub fn validate_receipt_tree_proof(
    root: &Receipt,
    children: &[Receipt],
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
    root: &Receipt,
    children: &[Receipt],
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
    root: &Receipt,
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
    root: &Receipt,
    resolver: &impl ReceiptResolver,
    config: ReceiptTreeConfig,
) -> ReceiptVerification {
    let mut findings = verify_receipt(root).findings;
    let supplied = resolver.supplied_receipts();
    findings.extend(duplicate_child_findings(&supplied));
    findings.extend(child_receipt_findings(&supplied));
    verify_tree_relationships(root, resolver, config, &supplied, findings)
}

pub fn validate_receipt_tree_proof_with_resolver(
    root: &Receipt,
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
    root: &Receipt,
    resolver: &impl ReceiptResolver,
    config: ReceiptTreeConfig,
    proof_contexts: &impl ReceiptProofContextProvider,
) -> ReceiptVerification {
    let root_context = proof_contexts.proof_context(root);
    let mut findings = verify_receipt_proof(root, &root_context).findings;
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
    root: &Receipt,
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
    findings.extend(traversal.subtree_findings("", root, 0));
    findings.extend(orphan_child_findings(supplied, &traversal.reached));
    ReceiptVerification::from_findings(findings)
}

fn verify_tree_relationships_with_proof<R: ReceiptResolver>(
    root: &Receipt,
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
    findings.extend(traversal.subtree_findings("", root, 0));
    findings.extend(orphan_child_findings(supplied, &traversal.reached));
    ReceiptVerification::from_findings(findings)
}

struct SliceReceiptResolver<'a> {
    children: &'a [Receipt],
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
            validate_receipt(child.receipt)
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
            verify_receipt_proof(child.receipt, &context)
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
        receipt: &Receipt,
    ) -> Vec<ReceiptFinding>;
}

struct StructuralChildProofPolicy;

impl ChildProofPolicy for StructuralChildProofPolicy {
    fn findings(
        &mut self,
        _path: &str,
        _reference: &Reference,
        _receipt: &Receipt,
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
        receipt: &Receipt,
    ) -> Vec<ReceiptFinding> {
        let mut findings = child_digest_link_findings(path, reference, receipt);
        if !self.verified_receipts.insert(receipt_address(receipt)) {
            return findings;
        }
        let context = self.proof_contexts.proof_context(receipt);
        findings.extend(
            verify_receipt_proof(receipt, &context)
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
        receipt: &Receipt,
        depth: usize,
    ) -> Vec<ReceiptFinding> {
        if !self.visiting.insert(receipt.id.clone()) {
            return vec![ReceiptFinding {
                code: ReceiptFindingCode::ChildReceiptCycle,
                path: join(path, "id"),
                message: "child harness receipt refs must not form cycles".to_owned(),
            }];
        }

        let mut findings = Vec::new();
        let empty: Vec<Reference> = Vec::new();
        let child_refs = receipt
            .lineage
            .as_ref()
            .map_or(&empty, |lineage| &lineage.children);
        if child_refs.len() > self.config.max_breadth {
            findings.push(ReceiptFinding {
                code: ReceiptFindingCode::ChildReceiptBreadthLimit,
                path: join(path, "lineage.children"),
                message: "child receipt refs exceed configured breadth limit".to_owned(),
            });
        }

        let child_findings = child_refs
            .iter()
            .take(self.config.max_breadth)
            .enumerate()
            .flat_map(|(index, reference)| {
                self.child_ref_findings(
                    &join(path, &format!("lineage.children[{index}]")),
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
        parent: &Receipt,
        reference: &Reference,
        depth: usize,
    ) -> Vec<ReceiptFinding> {
        if reference.reference_type != ReferenceType::Receipt {
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
            ReceiptResolveResult::ResolverError => return vec![resolver_error(path)],
        };
        let child = resolved.receipt;
        if self.visiting.contains(child.id.as_str()) {
            return vec![ReceiptFinding {
                code: ReceiptFindingCode::ChildReceiptCycle,
                path: path.to_owned(),
                message: "child harness receipt refs must not point to an ancestor".to_owned(),
            }];
        }
        let child_path = resolved.path.clone();
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

fn receipt_address(receipt: &Receipt) -> usize {
    receipt as *const Receipt as usize
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
        message: "child harness receipt ref must be a typed runx harness receipt URI".to_owned(),
    }
}

fn ambiguous_child(path: &str) -> ReceiptFinding {
    ReceiptFinding {
        code: ReceiptFindingCode::ChildReceiptAmbiguous,
        path: path.to_owned(),
        message: "child harness receipt ref resolved to multiple supplied receipts".to_owned(),
    }
}

fn resolver_error(path: &str) -> ReceiptFinding {
    ReceiptFinding {
        code: ReceiptFindingCode::ChildReceiptResolverError,
        path: path.to_owned(),
        message: "child harness receipt ref resolver failed before proof verification".to_owned(),
    }
}

fn parent_link_findings(
    path: &str,
    parent: &Receipt,
    child: &Receipt,
    config: ReceiptTreeConfig,
) -> Vec<ReceiptFinding> {
    let parent_uri = format!("runx:receipt:{}", parent.id);
    let child_parent = child
        .lineage
        .as_ref()
        .and_then(|lineage| lineage.parent.as_ref());
    match child_parent {
        Some(parent_ref) if parent_ref.uri == parent_uri => Vec::new(),
        Some(_) => vec![ReceiptFinding {
            code: ReceiptFindingCode::ChildReceiptParentMismatch,
            path: format!("{path}.lineage.parent"),
            message: "child lineage parent ref must match the parent receipt".to_owned(),
        }],
        None if config.require_parent_links => vec![ReceiptFinding {
            code: ReceiptFindingCode::ChildReceiptParentMismatch,
            path: format!("{path}.lineage.parent"),
            message: "strict tree verification requires child lineage parent refs".to_owned(),
        }],
        None => Vec::new(),
    }
}

fn child_digest_link_findings(
    path: &str,
    reference: &Reference,
    child: &Receipt,
) -> Vec<ReceiptFinding> {
    if reference.locator.as_deref() == Some(child.digest.as_str()) {
        return Vec::new();
    }
    vec![ReceiptFinding {
        code: ReceiptFindingCode::ChildReceiptDigestMismatch,
        path: format!("{path}.locator"),
        message: "strict tree proof requires child receipt refs to carry the exact child receipt digest".to_owned(),
    }]
}

fn child_finding(path: &str, finding: ReceiptFinding) -> ReceiptFinding {
    ReceiptFinding {
        path: format!("{path}.{}", finding.path),
        ..finding
    }
}

fn join(path: &str, segment: &str) -> String {
    if path.is_empty() {
        segment.to_owned()
    } else {
        format!("{path}.{segment}")
    }
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
    use runx_contracts::{Receipt, ReceiptIssuer, Reference, ReferenceType};

    const SUCCESS_RECEIPT: &str =
        include_str!("../../../fixtures/contracts/harness-spine/receipt-success.json");
    const ABNORMAL_RECEIPT: &str =
        include_str!("../../../fixtures/contracts/harness-spine/receipt-abnormal.json");

    #[derive(Debug, Deserialize)]
    struct Fixture {
        expected: Receipt,
    }

    fn child_refs_mut(receipt: &mut Receipt) -> &mut Vec<Reference> {
        &mut receipt.lineage.get_or_insert_with(Default::default).children
    }

    #[test]
    fn slice_adapter_accepts_only_typed_receipt_uri() -> Result<(), serde_json::Error> {
        let mut root = fixture(SUCCESS_RECEIPT)?;
        let child = child("hrn_rcpt_child_1")?;

        child_refs_mut(&mut root)[0].uri = "hrn_rcpt_child_1".to_owned();
        let verification = verify_receipt_tree(&root, std::slice::from_ref(&child));
        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptRefMalformed,
            "lineage.children[0]",
        );

        child_refs_mut(&mut root)[0].uri =
            "runx:receipt:hrn_rcpt_child_1".to_owned();
        assert!(verify_receipt_tree(&root, &[child]).valid);
        Ok(())
    }

    #[test]
    fn malformed_and_wrong_namespace_refs_are_stable_findings() -> Result<(), serde_json::Error> {
        let mut root = fixture(SUCCESS_RECEIPT)?;
        let child = child("hrn_rcpt_child_1")?;

        child_refs_mut(&mut root)[0].uri = "runx:graph_receipt:hrn_rcpt_child_1".to_owned();
        let verification = verify_receipt_tree(&root, std::slice::from_ref(&child));
        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptRefMalformed,
            "lineage.children[0]",
        );

        child_refs_mut(&mut root)[0].uri = ":hrn_rcpt_child_1".to_owned();
        let verification = verify_receipt_tree(&root, &[child]);
        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptRefMalformed,
            "lineage.children[0]",
        );
        Ok(())
    }

    #[test]
    fn suffix_only_refs_are_malformed_not_aliases() -> Result<(), serde_json::Error> {
        let mut root = fixture(SUCCESS_RECEIPT)?;
        child_refs_mut(&mut root)[0].uri = "child_1".to_owned();
        let child = child("hrn_rcpt_child_1")?;

        let verification = verify_receipt_tree(&root, &[child]);

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptRefMalformed,
            "lineage.children[0]",
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
            "lineage.children[0]",
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
            "lineage.children[0]",
        );
        Ok(())
    }

    #[test]
    fn resolver_error_result_is_a_stable_finding() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;

        let verification = verify_receipt_tree_with_resolver(
            &root,
            &ResolverErrorResolver,
            ReceiptTreeConfig::default(),
        );

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptResolverError,
            "lineage.children[0]",
        );
        Ok(())
    }

    #[test]
    fn strict_mode_rejects_mismatched_parent_link() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;
        let mut child = child("hrn_rcpt_child_1")?;
        child.lineage.get_or_insert_with(Default::default).parent =
            Some(reference(ReferenceType::Receipt, "other"));

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
            "lineage.children[0].lineage.parent",
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
            "lineage.children[0].lineage.parent",
        );
        Ok(())
    }

    #[test]
    fn depth_limit_blocks_hostile_nested_tree() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;
        let mut child_receipt = child("hrn_rcpt_child_1")?;
        child_refs_mut(&mut child_receipt)
            .push(reference(ReferenceType::Receipt, "grandchild"));
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
            "children[0].lineage.children[0]",
        );
        Ok(())
    }

    #[test]
    fn breadth_limit_blocks_hostile_fanout() -> Result<(), serde_json::Error> {
        let mut root = fixture(SUCCESS_RECEIPT)?;
        child_refs_mut(&mut root)
            .push(reference(ReferenceType::Receipt, "second"));
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
            "lineage.children",
        );
        Ok(())
    }

    #[test]
    fn positive_nested_tree_verifies() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;
        let mut child_receipt = child("hrn_rcpt_child_1")?;
        child_refs_mut(&mut child_receipt)
            .push(reference(ReferenceType::Receipt, "grandchild"));
        let grandchild = child("grandchild")?;

        assert!(verify_receipt_tree(&root, &[child_receipt, grandchild]).valid);
        Ok(())
    }

    #[test]
    fn positive_fanout_tree_verifies() -> Result<(), serde_json::Error> {
        let mut root = fixture(SUCCESS_RECEIPT)?;
        child_refs_mut(&mut root)
            .push(reference(ReferenceType::Receipt, "second"));
        let first = child("hrn_rcpt_child_1")?;
        let second = child("second")?;

        assert!(verify_receipt_tree(&root, &[first, second]).valid);
        Ok(())
    }

    #[test]
    fn strict_parent_links_can_verify_cleanly() -> Result<(), serde_json::Error> {
        let root = fixture(SUCCESS_RECEIPT)?;
        let mut child = child("hrn_rcpt_child_1")?;
        child.lineage.get_or_insert_with(Default::default).parent =
            Some(Reference::runx(ReferenceType::Receipt, &root.id));

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
            "lineage.children[0]",
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
    fn strict_tree_proof_rejects_legacy_exact_id_child_ref() -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let child = proof_child("hrn_rcpt_child_1")?;
        link_child_digest(&mut root, 0, &child)?;
        child_refs_mut(&mut root)[0].uri = child.id.clone();
        refresh_proof_digest_and_signature(&mut root)?;
        let proof_contexts = FixtureProofContexts::default();

        let verification = verify_receipt_tree_proof(&root, &[child], &proof_contexts);

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptRefMalformed,
            "lineage.children[0]",
        );
        Ok(())
    }

    #[test]
    fn strict_tree_proof_rejects_structurally_valid_child_proof_mismatch()
    -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let mut child = proof_child("hrn_rcpt_child_1")?;
        link_child_digest(&mut root, 0, &child)?;
        child.acts[0].summary = "tampered child proof body".to_owned();
        let proof_contexts = FixtureProofContexts::default();

        assert!(verify_receipt_tree(&root, std::slice::from_ref(&child)).valid);
        let verification = verify_receipt_tree_proof(&root, &[child], &proof_contexts);

        assert_finding(
            &verification,
            ReceiptFindingCode::SealDigestMismatch,
            "children[0].digest",
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
        alternate.acts[0].summary = "valid alternate child body".to_owned();
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
        child.acts[0].summary = "hidden tampered child".to_owned();
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
            "hidden_child.digest",
        );
        assert_finding(
            &verification,
            ReceiptFindingCode::SignatureInvalid,
            "hidden_child.signature.value",
        );
        Ok(())
    }

    #[test]
    fn strict_tree_proof_rejects_resolver_error() -> Result<(), serde_json::Error> {
        let root = proof_root()?;
        let proof_contexts = FixtureProofContexts::default();

        let verification = verify_receipt_tree_proof_with_resolver(
            &root,
            &ResolverErrorResolver,
            ReceiptTreeConfig::default(),
            &proof_contexts,
        );

        assert_finding(
            &verification,
            ReceiptFindingCode::ChildReceiptResolverError,
            "lineage.children[0]",
        );
        Ok(())
    }

    #[test]
    fn strict_tree_proof_rejects_custom_resolver_duplicate_id_child_after_reached()
    -> Result<(), serde_json::Error> {
        let mut root = proof_root()?;
        let first = proof_child("shared_child")?;
        let mut second = proof_child("shared_child")?;
        *child_refs_mut(&mut root) = vec![
            reference(ReferenceType::Receipt, "first"),
            reference(ReferenceType::Receipt, "second"),
        ];
        child_refs_mut(&mut root)[0].locator = Some(first.digest.clone());
        child_refs_mut(&mut root)[1].locator = Some(second.digest.clone());
        refresh_proof_digest_and_signature(&mut root)?;
        second.acts[0].summary = "hidden duplicate-id tamper".to_owned();
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
            "hidden_second.digest",
        );
        assert_finding(
            &verification,
            ReceiptFindingCode::SignatureInvalid,
            "hidden_second.signature.value",
        );
        Ok(())
    }

    fn fixture(json: &str) -> Result<Receipt, serde_json::Error> {
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

    fn child(id: &str) -> Result<Receipt, serde_json::Error> {
        let mut receipt = fixture(ABNORMAL_RECEIPT)?;
        receipt.id = id.to_owned();
        child_refs_mut(&mut receipt).clear();
        Ok(receipt)
    }

    fn proof_root() -> Result<Receipt, serde_json::Error> {
        let mut receipt = fixture(SUCCESS_RECEIPT)?;
        refresh_proof_digest_and_signature(&mut receipt)?;
        Ok(receipt)
    }

    fn proof_child(id: &str) -> Result<Receipt, serde_json::Error> {
        let mut receipt = fixture(SUCCESS_RECEIPT)?;
        receipt.id = id.to_owned();
        child_refs_mut(&mut receipt).clear();
        refresh_proof_digest_and_signature(&mut receipt)?;
        Ok(receipt)
    }

    fn link_child_digest(
        root: &mut Receipt,
        index: usize,
        child: &Receipt,
    ) -> Result<(), serde_json::Error> {
        child_refs_mut(root)[index].locator = Some(child.digest.clone());
        refresh_proof_digest_and_signature(root)
    }

    fn refresh_proof_digest_and_signature(
        receipt: &mut Receipt,
    ) -> Result<(), serde_json::Error> {
        let digest = canonical_receipt_body_digest(receipt)
            .map_err(|error| serde_json::Error::io(std::io::Error::other(error.to_string())))?;
        receipt.digest = digest.clone();
        receipt.signature.value = format!("sig:{digest}");
        Ok(())
    }

    fn reference(reference_type: ReferenceType, id: &str) -> Reference {
        Reference::runx(reference_type, id)
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
        fn proof_context<'a>(&'a self, _receipt: &Receipt) -> ReceiptProofContext<'a> {
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

    struct ResolverErrorResolver;

    impl ReceiptResolver for ResolverErrorResolver {
        fn resolve_child<'a>(&'a self, _reference: &Reference) -> ReceiptResolveResult<'a> {
            ReceiptResolveResult::ResolverError
        }

        fn supplied_receipts<'a>(&'a self) -> Vec<ResolvedReceipt<'a>> {
            Vec::new()
        }
    }

    struct HiddenChildResolver<'a> {
        child: &'a Receipt,
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
        first: &'a Receipt,
        second: &'a Receipt,
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
