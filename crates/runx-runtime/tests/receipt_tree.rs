use runx_contracts::{
    FanoutReceiptDecision, FanoutReceiptStrategy, FanoutReceiptSyncPoint, Receipt,
    JsonObject, ReceiptIssuer, ReceiptSignature,
};
use runx_core::state_machine::StepAdmissionWitness;
use runx_receipts::{
    ReceiptFindingCode, ReceiptTreeConfig, SignatureVerificationFailure, SignatureVerifier,
    canonical_receipt_body_digest,
};
use runx_runtime::receipt_tree::{
    validate_runtime_receipt_tree_with_policy, verify_runtime_receipt_tree_with_policy,
};
use runx_runtime::receipts::{RuntimeReceiptSignaturePolicy, graph_receipt, step_receipt};
use runx_runtime::{
    InvocationStatus, RuntimeReceiptResolver, SkillOutput, StepRun, validate_runtime_receipt_tree,
    verify_runtime_receipt_tree,
};

const CREATED_AT: &str = "2026-05-18T00:00:00Z";

#[test]
fn runtime_resolver_verifies_graph_receipt_with_children() -> Result<(), Box<dyn std::error::Error>>
{
    let (root, children) = graph_with_steps("tree_runtime_graph", &["plan", "apply"])?;
    let resolver = RuntimeReceiptResolver::new(children.clone());

    assert_eq!(resolver.receipts().len(), 2);
    assert!(children.iter().all(|child| {
        child.lineage.as_ref().and_then(|l| l.parent.as_ref()).map(|r| r.uri.as_str())
            == Some(format!("runx:receipt:{}", root.id).as_str())
    }));
    assert!(
        runx_receipts::validate_receipt_tree_with_resolver(
            &root,
            &resolver,
            ReceiptTreeConfig::default()
        )
        .is_ok()
    );
    assert!(validate_runtime_receipt_tree(&root, children, ReceiptTreeConfig::default()).is_ok());
    Ok(())
}

#[test]
fn runtime_tree_rejects_legacy_exact_id_child_ref() -> Result<(), Box<dyn std::error::Error>> {
    let (mut root, children) = graph_with_steps("tree_runtime_exact", &["child"])?;
    root.lineage.as_mut().unwrap().children[0].uri = children[0].id.clone();
    refresh_local_digest_and_signature(&mut root)?;

    let verification = verify_runtime_receipt_tree(&root, children, ReceiptTreeConfig::default());

    assert_finding(
        &verification,
        ReceiptFindingCode::ChildReceiptRefMalformed,
        "lineage.children[0]",
    );
    Ok(())
}

#[test]
fn runtime_resolver_reports_ambiguous_scoped_receipts() -> Result<(), Box<dyn std::error::Error>> {
    let (root, mut children) = graph_with_steps("tree_runtime_ambiguous", &["child"])?;
    children.push(children[0].clone());

    let verification = verify_runtime_receipt_tree(&root, children, ReceiptTreeConfig::default());

    assert_finding(
        &verification,
        ReceiptFindingCode::DuplicateChildReceipt,
        "runtime_receipts[1].id",
    );
    assert_finding(
        &verification,
        ReceiptFindingCode::ChildReceiptAmbiguous,
        "lineage.children[0]",
    );
    Ok(())
}

#[test]
fn runtime_tree_rejects_missing_child_receipt() -> Result<(), Box<dyn std::error::Error>> {
    let (root, _children) = graph_with_steps("tree_runtime_missing_child", &["child"])?;

    let verification = verify_runtime_receipt_tree(&root, Vec::new(), ReceiptTreeConfig::default());

    assert_finding(
        &verification,
        ReceiptFindingCode::ChildReceiptMissing,
        "lineage.children[0]",
    );
    Ok(())
}

#[test]
fn runtime_tree_rejects_extra_child_receipt() -> Result<(), Box<dyn std::error::Error>> {
    let (root, mut children) = graph_with_steps("tree_runtime_extra_child", &["child"])?;
    children.push(step_receipt(
        "tree_runtime_extra_child",
        "orphan",
        1,
        &skill_output(InvocationStatus::Success),
        CREATED_AT,
    )?);

    let verification = verify_runtime_receipt_tree(&root, children, ReceiptTreeConfig::default());

    assert_finding(
        &verification,
        ReceiptFindingCode::OrphanChildReceipt,
        "runtime_receipts[1].id",
    );
    Ok(())
}

#[test]
fn runtime_fanout_receipt_tree_uses_explicit_receipts() -> Result<(), Box<dyn std::error::Error>> {
    let steps = vec![
        step_run(
            "tree_runtime_fanout",
            "market",
            Some("advisors"),
            InvocationStatus::Success,
        )?,
        step_run(
            "tree_runtime_fanout",
            "risk",
            Some("advisors"),
            InvocationStatus::Failure,
        )?,
        step_run(
            "tree_runtime_fanout",
            "synthesize",
            None,
            InvocationStatus::Success,
        )?,
    ];
    let sync_point = fanout_sync_point(&steps[..2]);
    let mut steps = steps;
    let root = graph_receipt(
        "tree_runtime_fanout",
        &mut steps,
        vec![sync_point.clone()],
        CREATED_AT,
    )?;
    let children = child_receipts(&steps);

    assert_eq!(root.lineage.as_ref().unwrap().sync, vec![sync_point]);
    assert!(validate_runtime_receipt_tree(&root, children, ReceiptTreeConfig::default()).is_ok());
    Ok(())
}

#[test]
fn runtime_tree_rejects_structurally_valid_child_proof_tamper()
-> Result<(), Box<dyn std::error::Error>> {
    let (root, mut children) = graph_with_steps("tree_runtime_child_tamper", &["child"])?;
    children[0].acts[0].summary = "tampered child proof body".to_owned();

    assert!(runx_receipts::verify_receipt_tree(&root, &children).valid);
    let verification = verify_runtime_receipt_tree(&root, children, ReceiptTreeConfig::default());

    assert_finding(
        &verification,
        ReceiptFindingCode::SealDigestMismatch,
        "runtime_receipts[0].seal.digest",
    );
    assert_finding(
        &verification,
        ReceiptFindingCode::SignatureInvalid,
        "runtime_receipts[0].signature.value",
    );
    Ok(())
}

#[test]
fn runtime_tree_rejects_valid_alternate_child_with_same_id()
-> Result<(), Box<dyn std::error::Error>> {
    let (root, children) = graph_with_steps("tree_runtime_child_digest", &["child"])?;
    let mut alternate = children[0].clone();
    alternate.acts[0].summary = "valid alternate child body".to_owned();
    refresh_local_digest_and_signature(&mut alternate)?;

    let verification =
        verify_runtime_receipt_tree(&root, vec![alternate], ReceiptTreeConfig::default());

    assert_finding(
        &verification,
        ReceiptFindingCode::ChildReceiptDigestMismatch,
        "runtime_receipts[0].locator",
    );
    Ok(())
}

#[test]
fn runtime_tree_rejects_child_ref_without_digest_locator() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut root, children) = graph_with_steps("tree_runtime_missing_child_digest", &["child"])?;
    root.lineage.as_mut().unwrap().children[0].locator = None;
    refresh_local_digest_and_signature(&mut root)?;

    assert!(runx_receipts::verify_receipt_tree(&root, &children).valid);
    let verification = verify_runtime_receipt_tree(&root, children, ReceiptTreeConfig::default());

    assert_finding(
        &verification,
        ReceiptFindingCode::ChildReceiptDigestMismatch,
        "runtime_receipts[0].locator",
    );
    Ok(())
}

#[test]
fn runtime_tree_rejects_child_without_parent_link() -> Result<(), Box<dyn std::error::Error>> {
    let (root, mut children) = graph_with_steps("tree_runtime_missing_parent", &["child"])?;
    children[0].lineage.as_mut().unwrap().parent = None;
    refresh_local_digest_and_signature(&mut children[0])?;

    assert!(runx_receipts::verify_receipt_tree(&root, &children).valid);
    let verification = verify_runtime_receipt_tree(&root, children, ReceiptTreeConfig::default());

    assert_finding(
        &verification,
        ReceiptFindingCode::ChildReceiptParentMismatch,
        "lineage.children[0].lineage.parent",
    );
    Ok(())
}

#[test]
fn production_tree_policy_rejects_local_pseudo_signature_even_with_permissive_verifier()
-> Result<(), Box<dyn std::error::Error>> {
    let (root, children) = graph_with_steps("tree_runtime_prod_pseudo", &["child"])?;
    let verifier = PermissiveProductionVerifier;

    let verification = verify_runtime_receipt_tree_with_policy(
        &root,
        children,
        ReceiptTreeConfig::default(),
        RuntimeReceiptSignaturePolicy::production(&verifier),
    );

    assert_finding(
        &verification,
        ReceiptFindingCode::SignatureMalformed,
        "signature.value",
    );
    Ok(())
}

#[test]
fn production_tree_policy_accepts_supplied_non_pseudo_verifier()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut root, mut children) = graph_with_steps("tree_runtime_prod_real", &["plan", "apply"])?;
    resign_for_test_verifier(&mut root)?;
    for child in &mut children {
        resign_for_test_verifier(child)?;
    }
    let verifier = TestProductionVerifier;

    assert!(
        validate_runtime_receipt_tree_with_policy(
            &root,
            children,
            ReceiptTreeConfig::default(),
            RuntimeReceiptSignaturePolicy::production(&verifier),
        )
        .is_ok()
    );
    Ok(())
}

#[test]
fn production_tree_policy_without_verifier_fails_closed() -> Result<(), Box<dyn std::error::Error>>
{
    let (root, children) = graph_with_steps("tree_runtime_prod_missing", &["child"])?;

    let verification = verify_runtime_receipt_tree_with_policy(
        &root,
        children,
        ReceiptTreeConfig::default(),
        RuntimeReceiptSignaturePolicy::production_without_verifier(),
    );

    assert_finding(
        &verification,
        ReceiptFindingCode::SignatureVerifierMissing,
        "signature",
    );
    Ok(())
}

fn graph_with_steps(
    graph_name: &str,
    step_ids: &[&str],
) -> Result<(Receipt, Vec<Receipt>), Box<dyn std::error::Error>> {
    let steps = step_ids
        .iter()
        .map(|step_id| step_run(graph_name, step_id, None, InvocationStatus::Success))
        .collect::<Result<Vec<_>, _>>()?;
    let mut steps = steps;
    let root = graph_receipt(graph_name, &mut steps, Vec::new(), CREATED_AT)?;
    Ok((root, child_receipts(&steps)))
}

fn child_receipts(steps: &[StepRun]) -> Vec<Receipt> {
    steps.iter().map(|step| step.receipt.clone()).collect()
}

fn step_run(
    graph_name: &str,
    step_id: &str,
    fanout_group: Option<&str>,
    status: InvocationStatus,
) -> Result<StepRun, Box<dyn std::error::Error>> {
    let output = skill_output(status);
    let receipt = step_receipt(graph_name, step_id, 1, &output, CREATED_AT)?;
    let admission_witness = StepAdmissionWitness::local_runtime(step_id, &receipt.id);
    Ok(StepRun {
        step_id: step_id.to_owned(),
        attempt: 1,
        skill: step_id.to_owned(),
        runner: None,
        fanout_group: fanout_group.map(str::to_owned),
        output,
        outputs: JsonObject::new(),
        receipt,
        admission_witness,
    })
}

fn skill_output(status: InvocationStatus) -> SkillOutput {
    let (stdout, stderr, exit_code) = match status {
        InvocationStatus::Success => ("ok".to_owned(), String::new(), Some(0)),
        InvocationStatus::Failure => (String::new(), "failed".to_owned(), Some(1)),
    };
    SkillOutput {
        status,
        stdout,
        stderr,
        exit_code,
        duration_ms: 1,
        metadata: JsonObject::new(),
    }
}

fn fanout_sync_point(steps: &[StepRun]) -> FanoutReceiptSyncPoint {
    FanoutReceiptSyncPoint {
        group_id: "advisors".to_owned(),
        strategy: FanoutReceiptStrategy::Quorum,
        decision: FanoutReceiptDecision::Proceed,
        rule_fired: "quorum.min_success".to_owned(),
        reason: "1/2 branches succeeded".to_owned(),
        branch_count: 2,
        success_count: 1,
        failure_count: 1,
        required_successes: 1,
        branch_receipts: child_receipts(steps)
            .into_iter()
            .map(|receipt| receipt.id)
            .collect(),
        gate: None,
    }
}

fn resign_for_test_verifier(
    receipt: &mut Receipt,
) -> Result<(), Box<dyn std::error::Error>> {
    let digest = canonical_receipt_body_digest(receipt)?;
    receipt.signature.value = format!("ed25519-test:{digest}");
    Ok(())
}

fn refresh_local_digest_and_signature(
    receipt: &mut Receipt,
) -> Result<(), Box<dyn std::error::Error>> {
    let digest = canonical_receipt_body_digest(receipt)?;
    receipt.digest = digest.clone();
    receipt.signature.value = format!("sig:{digest}");
    Ok(())
}

struct PermissiveProductionVerifier;

impl SignatureVerifier for PermissiveProductionVerifier {
    fn verify(
        &self,
        _issuer: &ReceiptIssuer,
        _signature: &ReceiptSignature,
        _body_digest: &str,
    ) -> Result<(), SignatureVerificationFailure> {
        Ok(())
    }
}

struct TestProductionVerifier;

impl SignatureVerifier for TestProductionVerifier {
    fn verify(
        &self,
        _issuer: &ReceiptIssuer,
        signature: &ReceiptSignature,
        body_digest: &str,
    ) -> Result<(), SignatureVerificationFailure> {
        if signature.value == format!("ed25519-test:{body_digest}") {
            Ok(())
        } else {
            Err(SignatureVerificationFailure::SignatureMismatch)
        }
    }
}

fn assert_finding(
    verification: &runx_receipts::ReceiptVerification,
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
