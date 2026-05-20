use runx_contracts::{
    ActForm, ClosureDisposition, HarnessReceipt, PostMergeObserverRuntimeDecision,
    PostMergeObserverRuntimeDedupePlan, PostMergeObserverSignalSource, Reference, ReferenceType,
};
use runx_runtime::post_merge_observer::{
    PostMergeObserverPublicationCommand, PostMergeObserverPublicationLedger,
    PostMergeObserverPublicationRuntimeDecision, PostMergeObserverRuntimeError,
    project_post_merge_observer_publication_commands,
};

const POST_MERGE_OBSERVER_FIXTURE: &str = include_str!(
    "../../../fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json"
);

#[test]
fn sealed_receipt_projects_publication_commands_and_dedupes_publication_key()
-> Result<(), Box<dyn std::error::Error>> {
    let receipt = post_merge_observer_receipt()?;
    let webhook = dedupe_plan(&receipt, PostMergeObserverSignalSource::Webhook);
    let scheduler = dedupe_plan(&receipt, PostMergeObserverSignalSource::Scheduler);
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let first = project_post_merge_observer_publication_commands(&webhook, &receipt, &mut ledger)?;
    let repeated =
        project_post_merge_observer_publication_commands(&scheduler, &receipt, &mut ledger)?;

    assert_eq!(
        first.decision,
        PostMergeObserverPublicationRuntimeDecision::Publish
    );
    assert_eq!(first.commands.len(), 3);
    assert!(matches!(
        &first.commands[0],
        PostMergeObserverPublicationCommand::SourceIssueComment { target, .. }
            if target.reference_type == ReferenceType::GithubIssue
    ));
    assert!(matches!(
        &first.commands[1],
        PostMergeObserverPublicationCommand::SourceThreadReply { target, .. }
            if target.reference_type == ReferenceType::SlackThread
    ));
    assert!(matches!(
        &first.commands[2],
        PostMergeObserverPublicationCommand::SourceIssueClose { target, .. }
            if target.reference_type == ReferenceType::GithubIssue
    ));
    assert_eq!(
        repeated.decision,
        PostMergeObserverPublicationRuntimeDecision::AlreadyPublished
    );
    assert!(repeated.commands.is_empty());
    assert_eq!(first.publication_key, repeated.publication_key);
    Ok(())
}

#[test]
fn already_published_dedupe_plan_emits_no_commands() -> Result<(), Box<dyn std::error::Error>> {
    let receipt = post_merge_observer_receipt()?;
    let mut dedupe = dedupe_plan(&receipt, PostMergeObserverSignalSource::Scheduler);
    dedupe.decision = PostMergeObserverRuntimeDecision::AlreadyPublished;
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let runtime = project_post_merge_observer_publication_commands(&dedupe, &receipt, &mut ledger)?;

    assert_eq!(
        runtime.decision,
        PostMergeObserverPublicationRuntimeDecision::AlreadyPublished
    );
    assert!(runtime.commands.is_empty());
    Ok(())
}

#[test]
fn missing_source_thread_metadata_fails_closed_before_commands()
-> Result<(), Box<dyn std::error::Error>> {
    let mut receipt = post_merge_observer_receipt()?;
    strip_slack_thread_metadata(&mut receipt);
    let dedupe = dedupe_plan(&receipt, PostMergeObserverSignalSource::Webhook);
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let error = project_post_merge_observer_publication_commands(&dedupe, &receipt, &mut ledger)
        .err()
        .ok_or("expected missing source-thread metadata error")?;

    assert!(matches!(
        error,
        PostMergeObserverRuntimeError::MissingSourceThreadMetadata
    ));
    assert!(!ledger.contains(&dedupe.publication_key));
    Ok(())
}

#[test]
fn public_command_text_redacts_local_paths_and_env_secrets()
-> Result<(), Box<dyn std::error::Error>> {
    let mut receipt = post_merge_observer_receipt()?;
    receipt.seal.summary =
        "Verified from /Users/kam/dev/runx/.env OPENAI_API_KEY=sk-live".to_owned();
    receipt.harness.seal = Some(receipt.seal.clone());
    let dedupe = dedupe_plan(&receipt, PostMergeObserverSignalSource::Webhook);
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let runtime = project_post_merge_observer_publication_commands(&dedupe, &receipt, &mut ledger)?;
    let bodies = runtime
        .commands
        .iter()
        .filter_map(|command| match command {
            PostMergeObserverPublicationCommand::SourceIssueComment { body, .. }
            | PostMergeObserverPublicationCommand::SourceThreadReply { body, .. } => Some(body),
            PostMergeObserverPublicationCommand::SourceIssueClose { .. } => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(bodies.len(), 2);
    for body in bodies {
        assert!(!body.contains("/Users/kam"));
        assert!(!body.contains("OPENAI_API_KEY"));
        assert!(!body.contains("sk-live"));
        assert!(body.contains("[redacted]"));
    }
    Ok(())
}

#[test]
fn closed_unmerged_projection_publishes_without_source_issue_close()
-> Result<(), Box<dyn std::error::Error>> {
    let receipt = closed_unmerged_receipt()?;
    let dedupe = dedupe_plan(&receipt, PostMergeObserverSignalSource::Webhook);
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let runtime = project_post_merge_observer_publication_commands(&dedupe, &receipt, &mut ledger)?;

    assert_eq!(
        runtime.decision,
        PostMergeObserverPublicationRuntimeDecision::Publish
    );
    assert_eq!(runtime.commands.len(), 2);
    assert!(matches!(
        &runtime.commands[0],
        PostMergeObserverPublicationCommand::SourceIssueComment { .. }
    ));
    assert!(matches!(
        &runtime.commands[1],
        PostMergeObserverPublicationCommand::SourceThreadReply { .. }
    ));
    assert!(runtime.commands.iter().all(|command| {
        !matches!(
            command,
            PostMergeObserverPublicationCommand::SourceIssueClose { .. }
        )
    }));
    let bodies = runtime
        .commands
        .iter()
        .filter_map(|command| match command {
            PostMergeObserverPublicationCommand::SourceIssueComment { body, .. }
            | PostMergeObserverPublicationCommand::SourceThreadReply { body, .. } => Some(body),
            PostMergeObserverPublicationCommand::SourceIssueClose { .. } => None,
        })
        .collect::<Vec<_>>();
    for body in bodies {
        assert!(body.contains("Verification: not_required"));
        assert!(body.contains("Proof: post_merge.provider_state"));
        assert!(!body.contains("shipped"));
    }
    Ok(())
}

fn post_merge_observer_receipt() -> Result<HarnessReceipt, serde_json::Error> {
    #[derive(serde::Deserialize)]
    struct Fixture {
        expected: HarnessReceipt,
    }

    serde_json::from_str::<Fixture>(POST_MERGE_OBSERVER_FIXTURE).map(|fixture| fixture.expected)
}

fn closed_unmerged_receipt() -> Result<HarnessReceipt, serde_json::Error> {
    let mut receipt = post_merge_observer_receipt()?;
    receipt.seal.reason_code = "closed_unmerged".to_owned();
    receipt.seal.summary =
        "Target PR was closed without merge; source issue remains unresolved.".to_owned();
    receipt.seal.disposition = ClosureDisposition::Closed;
    receipt.seal.criteria.retain(|criterion| {
        matches!(
            criterion.criterion_id.as_str(),
            "post_merge.provider_state"
                | "post_merge.human_gate"
                | "post_merge.source_thread_target_present"
        )
    });
    receipt
        .harness
        .acts
        .retain(|act| act.form == ActForm::Observation || act.form == ActForm::Reply);
    receipt.harness.idempotency.content_hash =
        "sha256:post-merge-closure-closed-unmerged-nitrosend".to_owned();
    receipt.harness.seal = Some(receipt.seal.clone());
    Ok(receipt)
}

fn dedupe_plan(
    receipt: &HarnessReceipt,
    signal_source: PostMergeObserverSignalSource,
) -> PostMergeObserverRuntimeDedupePlan {
    PostMergeObserverRuntimeDedupePlan {
        decision: PostMergeObserverRuntimeDecision::SealAndPublish,
        signal_source,
        lock_key: format!(
            "post-merge-observer:{}",
            receipt.harness.idempotency.content_hash
        ),
        receipt_id: receipt.id.clone(),
        receipt_ref: Reference {
            reference_type: ReferenceType::HarnessReceipt,
            uri: format!("runx:harness_receipt:{}", receipt.id),
            provider: None,
            locator: Some(receipt.seal.digest.clone()),
            label: Some("post-merge observer harness receipt".to_owned()),
            observed_at: None,
        },
        publication_key: format!(
            "post-merge-publication:{}:{}",
            receipt.harness.idempotency.intent_key, receipt.harness.idempotency.content_hash
        ),
        content_hash: receipt.harness.idempotency.content_hash.clone(),
    }
}

fn strip_slack_thread_metadata(receipt: &mut HarnessReceipt) {
    for criterion in &mut receipt.seal.criteria {
        for reference in &mut criterion.evidence_refs {
            if reference.reference_type == ReferenceType::SlackThread {
                reference.provider = None;
                reference.locator = None;
            }
        }
    }
    for act in &mut receipt.harness.acts {
        for reference in &mut act.surface_refs {
            if reference.reference_type == ReferenceType::SlackThread {
                reference.provider = None;
                reference.locator = None;
            }
        }
    }
    receipt.harness.seal = Some(receipt.seal.clone());
}
