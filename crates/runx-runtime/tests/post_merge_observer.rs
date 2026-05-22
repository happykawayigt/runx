use std::cell::RefCell;

use runx_contracts::{
    ActForm, ClosureDisposition, CriterionStatus, HarnessReceipt, PostMergeObserverPlanError,
    PostMergeObserverRuntimeDecision, PostMergeObserverRuntimeDedupePlan,
    PostMergeObserverSignalSource, PostMergeProvider, PostMergePullRequestObservation,
    PostMergePullRequestState, PostMergeVerificationObservation, PostMergeVerificationStatus,
    Reference, ReferenceType,
};
use runx_runtime::post_merge_observer::{
    FixtureBackedGitHubPostMergeObserverAdapter, GithubPostMergePullRequestObserverAdapter,
    PostMergeObserverAdapter, PostMergeObserverAdapterError, PostMergeObserverHttpError,
    PostMergeObserverHttpMethod, PostMergeObserverHttpRequest, PostMergeObserverHttpResponse,
    PostMergeObserverHttpTransport, PostMergeObserverLivePublicationRequest,
    PostMergeObserverPublicationAdapter, PostMergeObserverPublicationCommand,
    PostMergeObserverPublicationLedger, PostMergeObserverPublicationRuntimeDecision,
    PostMergeObserverPullRequestObservationRequest, PostMergeObserverRuntimeError,
    PostMergeObserverSourcePublicationObservation, PostMergeObserverSourcePublicationRequest,
    PostMergeObserverVerificationObservationRequest, execute_post_merge_observer_with_adapter,
    execute_post_merge_observer_with_publication_adapter,
    project_post_merge_observer_publication_commands,
};

const POST_MERGE_OBSERVER_FIXTURE: &str = include_str!(
    "../../../fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json"
);
const GITHUB_PR_OBSERVATION_FIXTURE: &str = include_str!(
    "../../../fixtures/contracts/post-merge-observer/github-pr-merged-verified-observation.json"
);
const GITHUB_PR_API_RESPONSE: &str = r#"{
  "number": 188,
  "state": "closed",
  "merged": true,
  "merge_commit_sha": "9f14c0ffee1234567890abcdef1234567890abcd",
  "updated_at": "2026-05-20T04:55:10Z",
  "closed_at": "2026-05-20T04:55:00Z",
  "merged_at": "2026-05-20T04:55:00Z",
  "user": { "login": "human-reviewer" },
  "merged_by": { "login": "human-reviewer" },
  "base": {
    "repo": {
      "full_name": "runxhq/nitrosend"
    }
  }
}"#;
const NITROSEND_LIKE: &str =
    include_str!("../../../fixtures/operational-policy/nitrosend-like.json");
const OBSERVED_AT: &str = "2026-05-20T04:55:00Z";
const VERIFIED_AT: &str = "2026-05-20T04:55:30Z";

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
    let body = match &first.commands[0] {
        PostMergeObserverPublicationCommand::SourceIssueComment { body, .. } => body,
        _ => return Err("expected source issue comment command".into()),
    };
    assert!(body.contains("Source issue: github://runxhq/nitrosend/issues/77"));
    assert!(body.contains("Target PR: github://runxhq/nitrosend/pulls/188"));
    assert!(body.contains("Merge: 9f14c0ffee1234567890abcdef1234567890abcd"));
    assert!(body.contains("Verification summary: Nitrosend dogfood verification passed."));
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
        assert!(body.contains("Target PR: github://runxhq/nitrosend/pulls/188"));
        assert!(body.contains("Merge: not_available"));
        assert!(body.contains("Verification: not_required"));
        assert!(body.contains("Verification summary: not_required"));
        assert!(body.contains("Proof: post_merge.provider_state"));
        assert!(!body.contains("shipped"));
    }
    Ok(())
}

#[test]
fn failed_verification_projection_publishes_final_reply_without_source_issue_close()
-> Result<(), Box<dyn std::error::Error>> {
    let receipt = failed_verification_receipt()?;
    let dedupe = dedupe_plan(&receipt, PostMergeObserverSignalSource::Webhook);
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let runtime = project_post_merge_observer_publication_commands(&dedupe, &receipt, &mut ledger)?;

    assert_eq!(
        runtime.decision,
        PostMergeObserverPublicationRuntimeDecision::Publish
    );
    assert_eq!(runtime.commands.len(), 2);
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
    assert_eq!(bodies.len(), 2);
    for body in bodies {
        assert!(body.contains("Review gate: external_human"));
        assert!(body.contains("Closure: failed_verification"));
        assert!(body.contains("Target PR: github://runxhq/nitrosend/pulls/188"));
        assert!(body.contains("Merge: 9f14c0ffee1234567890abcdef1234567890abcd"));
        assert!(body.contains("Verification: post_merge.verification_failed"));
        assert!(body.contains("Verification summary: Nitrosend dogfood verification failed."));
        assert!(body.contains("Proof: post_merge.verification_failed"));
        assert!(body.contains("Next: review_failed_verification"));
        assert!(!body.contains("shipped"));
    }
    assert!(ledger.contains(&dedupe.publication_key));
    Ok(())
}

#[test]
fn live_adapter_projects_observed_closure_into_publication_commands_without_network()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: runx_contracts::OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let receipt = post_merge_observer_receipt()?;
    let mut adapter = FakePostMergeObserverAdapter { events: Vec::new() };
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let live = execute_post_merge_observer_with_adapter(
        &policy,
        &PostMergeObserverLivePublicationRequest {
            source_id: Some("bugs-fixes".to_owned()),
            source_issue_ref: fixture_source_issue_ref(),
            source_thread_ref: Some(fixture_source_thread_ref()),
            pull_request_ref: fixture_pull_request_ref(),
            signal_source: PostMergeObserverSignalSource::Webhook,
            signal_ref: Some(webhook_delivery_ref()),
        },
        &receipt,
        &mut adapter,
        &mut ledger,
    )?;

    assert_eq!(adapter.events, vec!["pull_request", "verification"]);
    assert_eq!(
        live.command.command_key,
        "post-merge-observer:github://runxhq/nitrosend/issues/77:github://runxhq/nitrosend/pulls/188"
    );
    assert_eq!(
        live.command.signal_ref.as_ref().map(|reference| {
            (
                reference.reference_type.clone(),
                reference.provider.as_deref(),
                reference.locator.as_deref(),
            )
        }),
        Some((
            ReferenceType::WebhookDelivery,
            Some("github"),
            Some("runxhq/nitrosend/delivery/evt_01HX")
        ))
    );
    assert_eq!(live.pull_request.provider, PostMergeProvider::Github);
    assert_eq!(live.pull_request.repo, "runxhq/nitrosend");
    assert_eq!(live.pull_request.number, 188);
    assert!(live.pull_request.merged);
    assert_eq!(
        live.verification.status,
        PostMergeVerificationStatus::Passed
    );
    assert_eq!(live.verification.evidence_refs.len(), 1);
    assert_eq!(
        live.verification.evidence_refs[0].reference_type,
        ReferenceType::Deployment
    );
    assert_eq!(live.closure_plan.reason_code, receipt.seal.reason_code);
    assert_eq!(
        live.publication.decision,
        PostMergeObserverPublicationRuntimeDecision::Publish
    );
    assert_eq!(live.publication.commands.len(), 3);
    assert_eq!(
        live.publication.receipt_ref.uri,
        format!("runx:harness_receipt:{}", receipt.id)
    );
    assert!(ledger.contains(&live.dedupe.publication_key));
    assert!(matches!(
        &live.publication.commands[0],
        PostMergeObserverPublicationCommand::SourceIssueComment { target, .. }
            if target.reference_type == ReferenceType::GithubIssue
                && target.provider.as_deref() == Some("github")
    ));
    assert!(matches!(
        &live.publication.commands[2],
        PostMergeObserverPublicationCommand::SourceIssueClose { target, .. }
            if target.reference_type == ReferenceType::GithubIssue
    ));
    Ok(())
}

#[test]
fn live_publication_adapter_requires_provider_readback_before_publication_dedupe()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: runx_contracts::OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let receipt = post_merge_observer_receipt()?;
    let mut adapter = FakePostMergeObserverAdapter { events: Vec::new() };
    let mut publisher = FakePostMergeObserverPublisher::default();
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let live = execute_post_merge_observer_with_publication_adapter(
        &policy,
        &live_publication_request(),
        &receipt,
        &mut adapter,
        &mut publisher,
        &mut ledger,
    )?;

    assert_eq!(adapter.events, vec!["pull_request", "verification"]);
    assert_eq!(publisher.events, vec!["source_publication"]);
    assert_eq!(
        live.publication.decision,
        PostMergeObserverPublicationRuntimeDecision::Publish
    );
    assert_eq!(live.publication.commands.len(), 3);
    let readback = live
        .source_publication
        .as_ref()
        .ok_or("expected source publication readback")?;
    assert_eq!(
        readback.request.receipt_ref.uri,
        format!("runx:harness_receipt:{}", receipt.id)
    );
    assert!(readback.request.close_source_issue);
    assert_eq!(readback.observation.published_refs.len(), 2);
    assert!(readback.observation.closed_ref.is_some());
    assert!(readback.proof_refs.iter().any(|reference| {
        reference.provider.as_deref() == Some("github")
            && reference.uri.contains("issuecomment-9001")
    }));
    assert!(readback.proof_refs.iter().any(|reference| {
        reference.reference_type == ReferenceType::SlackThread
            && reference.uri.contains("/reply/1716180950.000200")
    }));
    assert!(readback.proof_refs.iter().any(|reference| {
        reference.reference_type == ReferenceType::GithubIssue
            && reference.uri == "github://runxhq/nitrosend/issues/77"
    }));
    assert!(readback.proof_refs.iter().any(|reference| {
        reference.reference_type == ReferenceType::HarnessReceipt
            && reference.uri == format!("runx:harness_receipt:{}", receipt.id)
    }));
    assert!(ledger.contains(&live.dedupe.publication_key));
    Ok(())
}

#[test]
fn live_publication_readback_mismatch_fails_without_marking_publication_dedupe()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: runx_contracts::OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let receipt = post_merge_observer_receipt()?;
    let expected_publication_key = dedupe_plan(&receipt, PostMergeObserverSignalSource::Webhook)
        .publication_key
        .clone();
    let mut adapter = FakePostMergeObserverAdapter { events: Vec::new() };
    let mut publisher = FakePostMergeObserverPublisher {
        omit_close_readback: true,
        ..FakePostMergeObserverPublisher::default()
    };
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let error = execute_post_merge_observer_with_publication_adapter(
        &policy,
        &live_publication_request(),
        &receipt,
        &mut adapter,
        &mut publisher,
        &mut ledger,
    )
    .err()
    .ok_or("expected source publication readback error")?;

    assert!(matches!(
        error,
        PostMergeObserverRuntimeError::SourcePublicationMismatch(message)
            if message == "publication readback did not return a proof ref for every source command"
    ));
    assert_eq!(adapter.events, vec!["pull_request", "verification"]);
    assert_eq!(publisher.events, vec!["source_publication"]);
    assert!(!ledger.contains(&expected_publication_key));
    Ok(())
}

#[test]
fn live_adapter_command_validation_fails_before_provider_observation()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: runx_contracts::OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let receipt = post_merge_observer_receipt()?;
    let mut adapter = FakePostMergeObserverAdapter { events: Vec::new() };
    let mut ledger = PostMergeObserverPublicationLedger::new();
    let mut pull_request_ref = fixture_pull_request_ref();
    pull_request_ref.locator = None;

    let error = execute_post_merge_observer_with_adapter(
        &policy,
        &PostMergeObserverLivePublicationRequest {
            source_id: Some("bugs-fixes".to_owned()),
            source_issue_ref: fixture_source_issue_ref(),
            source_thread_ref: Some(fixture_source_thread_ref()),
            pull_request_ref,
            signal_source: PostMergeObserverSignalSource::Webhook,
            signal_ref: Some(webhook_delivery_ref()),
        },
        &receipt,
        &mut adapter,
        &mut ledger,
    )
    .err()
    .ok_or("expected live command validation error")?;

    assert!(matches!(
        error,
        PostMergeObserverRuntimeError::Projection(
            PostMergeObserverPlanError::MissingObserverCommandReferenceMetadata {
                field: "pull_request_ref"
            }
        )
    ));
    assert!(adapter.events.is_empty());
    Ok(())
}

#[test]
fn github_pr_http_observer_maps_pull_request_readback_without_network()
-> Result<(), Box<dyn std::error::Error>> {
    let transport = RecordingGithubPrTransport::with_status(200, GITHUB_PR_API_RESPONSE);
    let mut adapter = GithubPostMergePullRequestObserverAdapter::with_transport(
        "https://api.github.example/",
        &transport,
        Some("SECRET_GITHUB_TOKEN".to_owned()),
    )?;

    let observation =
        adapter.observe_pull_request(&PostMergeObserverPullRequestObservationRequest {
            source_id: Some("bugs-fixes".to_owned()),
            source_issue_ref: fixture_source_issue_ref(),
            source_thread_ref: Some(fixture_source_thread_ref()),
            pull_request_ref: fixture_pull_request_ref(),
        })?;

    assert_eq!(observation.provider, PostMergeProvider::Github);
    assert_eq!(observation.repo, "runxhq/nitrosend");
    assert_eq!(observation.number, 188);
    assert_eq!(observation.uri, "github://runxhq/nitrosend/pulls/188");
    assert_eq!(observation.state, PostMergePullRequestState::Closed);
    assert!(observation.merged);
    assert_eq!(
        observation.merge_sha.as_deref(),
        Some("9f14c0ffee1234567890abcdef1234567890abcd")
    );
    assert_eq!(observation.observed_at, OBSERVED_AT);
    assert_eq!(
        observation.actor.as_deref(),
        Some("github:user:human-reviewer")
    );

    let sent = transport.requests.borrow();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].method, PostMergeObserverHttpMethod::Get);
    assert_eq!(
        sent[0].url,
        "https://api.github.example/repos/runxhq/nitrosend/pulls/188"
    );
    assert!(sent[0].body.is_none());
    assert!(
        sent[0]
            .headers
            .iter()
            .any(|header| header.name == "authorization")
    );
    assert!(!format!("{:?}", sent[0]).contains("SECRET_GITHUB_TOKEN"));
    Ok(())
}

#[test]
fn github_pr_http_observer_mismatch_fails_without_publication_dedupe()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: runx_contracts::OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let receipt = post_merge_observer_receipt()?;
    let expected_publication_key = dedupe_plan(&receipt, PostMergeObserverSignalSource::Webhook)
        .publication_key
        .clone();
    let mismatched_response = GITHUB_PR_API_RESPONSE.replace("\"number\": 188", "\"number\": 189");
    let transport = RecordingGithubPrTransport::with_status(200, mismatched_response);
    let mut adapter = GithubPostMergePullRequestObserverAdapter::with_transport(
        "https://api.github.example",
        &transport,
        None,
    )?;
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let error = execute_post_merge_observer_with_adapter(
        &policy,
        &live_publication_request(),
        &receipt,
        &mut adapter,
        &mut ledger,
    )
    .err()
    .ok_or("expected github readback mismatch")?;

    assert!(matches!(
        error,
        PostMergeObserverRuntimeError::Adapter(PostMergeObserverAdapterError {
            operation: "observe_pull_request_github",
            message
        }) if message == "github pull request readback does not match requested pull request"
    ));
    assert_eq!(transport.requests.borrow().len(), 1);
    assert!(!ledger.contains(&expected_publication_key));
    Ok(())
}

#[test]
fn github_pr_http_observer_requires_verification_readback_before_publication_dedupe()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: runx_contracts::OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let receipt = post_merge_observer_receipt()?;
    let expected_publication_key = dedupe_plan(&receipt, PostMergeObserverSignalSource::Webhook)
        .publication_key
        .clone();
    let transport = RecordingGithubPrTransport::with_status(200, GITHUB_PR_API_RESPONSE);
    let mut adapter = GithubPostMergePullRequestObserverAdapter::with_transport(
        "https://api.github.example",
        &transport,
        None,
    )?;
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let error = execute_post_merge_observer_with_adapter(
        &policy,
        &live_publication_request(),
        &receipt,
        &mut adapter,
        &mut ledger,
    )
    .err()
    .ok_or("expected missing verification readback adapter error")?;

    assert!(matches!(
        error,
        PostMergeObserverRuntimeError::Adapter(PostMergeObserverAdapterError {
            operation: "observe_verification_github",
            message
        }) if message == "verification readback adapter is not configured"
    ));
    assert_eq!(transport.requests.borrow().len(), 1);
    assert!(!ledger.contains(&expected_publication_key));
    Ok(())
}

#[test]
fn fixture_backed_github_pr_adapter_observes_readback_without_network()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: runx_contracts::OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let receipt = post_merge_observer_receipt()?;
    let mut adapter =
        FixtureBackedGitHubPostMergeObserverAdapter::from_json_str(GITHUB_PR_OBSERVATION_FIXTURE)?;
    let mut ledger = PostMergeObserverPublicationLedger::new();

    let live = execute_post_merge_observer_with_adapter(
        &policy,
        &live_publication_request(),
        &receipt,
        &mut adapter,
        &mut ledger,
    )?;

    assert_eq!(live.pull_request.provider, PostMergeProvider::Github);
    assert_eq!(live.pull_request.repo, "runxhq/nitrosend");
    assert_eq!(live.pull_request.number, 188);
    assert_eq!(
        live.pull_request.merge_sha.as_deref(),
        Some("9f14c0ffee1234567890abcdef1234567890abcd")
    );
    assert_eq!(
        live.verification.status,
        PostMergeVerificationStatus::Passed
    );
    assert_eq!(live.closure_plan.reason_code, "merged_verified");
    assert_eq!(live.publication.commands.len(), 3);
    assert!(ledger.contains(&live.dedupe.publication_key));
    Ok(())
}

#[test]
fn fixture_backed_github_pr_adapter_rejects_mismatched_pull_request_readback()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: runx_contracts::OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let receipt = post_merge_observer_receipt()?;
    let expected_publication_key = dedupe_plan(&receipt, PostMergeObserverSignalSource::Webhook)
        .publication_key
        .clone();
    let mut adapter =
        FixtureBackedGitHubPostMergeObserverAdapter::from_json_str(GITHUB_PR_OBSERVATION_FIXTURE)?;
    let mut ledger = PostMergeObserverPublicationLedger::new();
    let mut request = live_publication_request();
    request.pull_request_ref.locator = Some("runxhq/nitrosend#189".to_owned());

    let error = execute_post_merge_observer_with_adapter(
        &policy,
        &request,
        &receipt,
        &mut adapter,
        &mut ledger,
    )
    .err()
    .ok_or("expected fixture readback mismatch")?;

    assert!(matches!(
        error,
        PostMergeObserverRuntimeError::Adapter(PostMergeObserverAdapterError {
            operation: "observe_pull_request_fixture",
            message
        }) if message == "pull request ref does not match fixture readback"
    ));
    assert!(!ledger.contains(&expected_publication_key));
    Ok(())
}

fn live_publication_request() -> PostMergeObserverLivePublicationRequest {
    PostMergeObserverLivePublicationRequest {
        source_id: Some("bugs-fixes".to_owned()),
        source_issue_ref: fixture_source_issue_ref(),
        source_thread_ref: Some(fixture_source_thread_ref()),
        pull_request_ref: fixture_pull_request_ref(),
        signal_source: PostMergeObserverSignalSource::Webhook,
        signal_ref: Some(webhook_delivery_ref()),
    }
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

fn failed_verification_receipt() -> Result<HarnessReceipt, serde_json::Error> {
    let mut receipt = post_merge_observer_receipt()?;
    receipt.seal.reason_code = "failed_verification".to_owned();
    receipt.seal.summary = "Merged PR was observed, but post-merge verification failed.".to_owned();
    receipt.seal.disposition = ClosureDisposition::Failed;
    receipt.seal.criteria.retain(|criterion| {
        matches!(
            criterion.criterion_id.as_str(),
            "post_merge.provider_state"
                | "post_merge.human_gate"
                | "post_merge.verification_passed"
                | "post_merge.source_thread_target_present"
        )
    });
    for criterion in &mut receipt.seal.criteria {
        if criterion.criterion_id == "post_merge.verification_passed" {
            criterion.criterion_id = "post_merge.verification_failed".to_owned();
            criterion.status = CriterionStatus::Failed;
            criterion.summary = Some("Nitrosend dogfood verification failed.".to_owned());
        }
    }
    receipt
        .harness
        .acts
        .retain(|act| act.form != ActForm::Revision);
    receipt.harness.idempotency.content_hash =
        "sha256:post-merge-closure-failed-verification-nitrosend".to_owned();
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
            proof_kind: None,
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

struct RecordingGithubPrTransport {
    requests: RefCell<Vec<PostMergeObserverHttpRequest>>,
    status: u16,
    body: String,
}

impl RecordingGithubPrTransport {
    fn with_status(status: u16, body: impl Into<String>) -> Self {
        Self {
            requests: RefCell::new(Vec::new()),
            status,
            body: body.into(),
        }
    }
}

impl PostMergeObserverHttpTransport for &RecordingGithubPrTransport {
    fn send(
        &self,
        request: PostMergeObserverHttpRequest,
    ) -> Result<PostMergeObserverHttpResponse, PostMergeObserverHttpError> {
        self.requests.borrow_mut().push(request);
        Ok(PostMergeObserverHttpResponse {
            status: self.status,
            body: self.body.clone(),
        })
    }
}

struct FakePostMergeObserverAdapter {
    events: Vec<&'static str>,
}

impl PostMergeObserverAdapter for FakePostMergeObserverAdapter {
    fn observe_pull_request(
        &mut self,
        request: &PostMergeObserverPullRequestObservationRequest,
    ) -> Result<PostMergePullRequestObservation, PostMergeObserverAdapterError> {
        self.events.push("pull_request");
        assert_eq!(request.source_id.as_deref(), Some("bugs-fixes"));
        assert_eq!(
            request.pull_request_ref.reference_type,
            ReferenceType::GithubPullRequest
        );
        assert_eq!(request.pull_request_ref.provider.as_deref(), Some("github"));
        Ok(PostMergePullRequestObservation {
            provider: PostMergeProvider::Github,
            repo: "runxhq/nitrosend".to_owned(),
            number: 188,
            uri: request.pull_request_ref.uri.clone(),
            state: PostMergePullRequestState::Closed,
            merged: true,
            merge_sha: Some("9f14c0ffee1234567890abcdef1234567890abcd".to_owned()),
            observed_at: OBSERVED_AT.to_owned(),
            closed_at: Some(OBSERVED_AT.to_owned()),
            actor: Some("github:user:human-reviewer".to_owned()),
        })
    }

    fn observe_verification(
        &mut self,
        request: &PostMergeObserverVerificationObservationRequest,
    ) -> Result<PostMergeVerificationObservation, PostMergeObserverAdapterError> {
        self.events.push("verification");
        assert!(request.pull_request.merged);
        assert_eq!(
            request.source_thread_ref.as_ref().map(|reference| {
                (
                    reference.reference_type.clone(),
                    reference.provider.as_deref(),
                    reference.locator.as_deref(),
                )
            }),
            Some((
                ReferenceType::SlackThread,
                Some("slack"),
                Some("T01NITRO/C02DOGFOOD/1716180900.000100")
            ))
        );
        Ok(PostMergeVerificationObservation {
            status: PostMergeVerificationStatus::Passed,
            summary: Some("Dogfood smoke check passed from sanitized metadata.".to_owned()),
            verification_ref: Some(Reference {
                reference_type: ReferenceType::Verification,
                uri: "runx:verification:nitrosend-dogfood-smoke".to_owned(),
                provider: None,
                locator: None,
                label: Some("Nitrosend dogfood smoke".to_owned()),
                observed_at: Some(VERIFIED_AT.to_owned()),
                proof_kind: None,
            }),
            evidence_refs: vec![Reference {
                reference_type: ReferenceType::Deployment,
                uri: "deploy://nitrosend/dogfood/2026-05-20T04-52Z".to_owned(),
                provider: Some("nitrosend".to_owned()),
                locator: None,
                label: Some("Nitrosend dogfood deploy".to_owned()),
                observed_at: Some(VERIFIED_AT.to_owned()),
                proof_kind: None,
            }],
            verified_at: Some(VERIFIED_AT.to_owned()),
        })
    }
}

#[derive(Default)]
struct FakePostMergeObserverPublisher {
    events: Vec<&'static str>,
    omit_issue_comment_readback: bool,
    omit_thread_reply_readback: bool,
    omit_close_readback: bool,
}

impl PostMergeObserverPublicationAdapter for FakePostMergeObserverPublisher {
    fn publish_source_update(
        &mut self,
        request: &PostMergeObserverSourcePublicationRequest,
    ) -> Result<PostMergeObserverSourcePublicationObservation, PostMergeObserverAdapterError> {
        self.events.push("source_publication");
        assert_post_merge_source_publication_request(request);
        let mut published_refs = Vec::new();
        if !self.omit_issue_comment_readback {
            published_refs.push(Reference {
                reference_type: ReferenceType::ExternalUrl,
                uri: "https://github.com/runxhq/nitrosend/issues/77#issuecomment-9001".to_owned(),
                provider: Some("github".to_owned()),
                locator: Some("runxhq/nitrosend#77-comment-9001".to_owned()),
                label: Some("post-merge source issue final comment".to_owned()),
                observed_at: Some(VERIFIED_AT.to_owned()),
                proof_kind: None,
            });
        }
        if !self.omit_thread_reply_readback {
            published_refs.push(Reference {
                reference_type: ReferenceType::SlackThread,
                uri: "slack://T01NITRO/C02DOGFOOD/p1716180900.000100/reply/1716180950.000200"
                    .to_owned(),
                provider: Some("slack".to_owned()),
                locator: Some("T01NITRO/C02DOGFOOD/1716180950.000200".to_owned()),
                label: Some("post-merge source thread final reply".to_owned()),
                observed_at: Some(VERIFIED_AT.to_owned()),
                proof_kind: None,
            });
        }

        Ok(PostMergeObserverSourcePublicationObservation {
            source_issue_ref: request.source_issue_ref.clone(),
            source_thread_ref: request.source_thread_ref.clone(),
            pull_request_ref: request.pull_request_ref.clone(),
            receipt_ref: request.receipt_ref.clone(),
            published_refs,
            closed_ref: (!self.omit_close_readback).then(|| request.source_issue_ref.clone()),
        })
    }
}

fn assert_post_merge_source_publication_request(
    request: &PostMergeObserverSourcePublicationRequest,
) {
    assert_eq!(
        request.source_issue_ref.reference_type,
        ReferenceType::GithubIssue
    );
    assert_eq!(
        request.source_thread_ref.reference_type,
        ReferenceType::SlackThread
    );
    assert_eq!(
        request.pull_request_ref.reference_type,
        ReferenceType::GithubPullRequest
    );
    assert_eq!(
        request.receipt_ref.reference_type,
        ReferenceType::HarnessReceipt
    );
    assert_eq!(request.reason_code, "merged_verified");
    assert!(request.close_source_issue);
    assert_eq!(request.commands.len(), 3);
    assert!(matches!(
        &request.commands[0],
        PostMergeObserverPublicationCommand::SourceIssueComment { target, body, .. }
            if target.uri == request.source_issue_ref.uri
                && body.contains(&request.pull_request_ref.uri)
                && body.contains(&request.receipt_ref.uri)
    ));
    assert!(matches!(
        &request.commands[1],
        PostMergeObserverPublicationCommand::SourceThreadReply { target, body, .. }
            if target.uri == request.source_thread_ref.uri
                && body.contains(&request.pull_request_ref.uri)
                && body.contains(&request.receipt_ref.uri)
    ));
    assert!(matches!(
        &request.commands[2],
        PostMergeObserverPublicationCommand::SourceIssueClose {
            target,
            reason_code,
            ..
        } if target.uri == request.source_issue_ref.uri && reason_code == "merged_verified"
    ));
}

fn fixture_source_issue_ref() -> Reference {
    Reference {
        reference_type: ReferenceType::GithubIssue,
        uri: "github://runxhq/nitrosend/issues/77".to_owned(),
        provider: Some("github".to_owned()),
        locator: Some("runxhq/nitrosend#77".to_owned()),
        label: Some("Nitrosend dogfood issue".to_owned()),
        observed_at: None,
        proof_kind: None,
    }
}

fn fixture_source_thread_ref() -> Reference {
    Reference {
        reference_type: ReferenceType::SlackThread,
        uri: "slack://T01NITRO/C02DOGFOOD/p1716180900.000100".to_owned(),
        provider: Some("slack".to_owned()),
        locator: Some("T01NITRO/C02DOGFOOD/1716180900.000100".to_owned()),
        label: Some("Nitrosend source thread".to_owned()),
        observed_at: None,
        proof_kind: None,
    }
}

fn fixture_pull_request_ref() -> Reference {
    Reference {
        reference_type: ReferenceType::GithubPullRequest,
        uri: "github://runxhq/nitrosend/pulls/188".to_owned(),
        provider: Some("github".to_owned()),
        locator: Some("runxhq/nitrosend#188".to_owned()),
        label: Some("human-merged PR".to_owned()),
        observed_at: Some(OBSERVED_AT.to_owned()),
        proof_kind: None,
    }
}

fn webhook_delivery_ref() -> Reference {
    Reference {
        reference_type: ReferenceType::WebhookDelivery,
        uri: "github://webhook-deliveries/evt_01HX".to_owned(),
        provider: Some("github".to_owned()),
        locator: Some("runxhq/nitrosend/delivery/evt_01HX".to_owned()),
        label: Some("GitHub pull_request webhook delivery".to_owned()),
        observed_at: Some(OBSERVED_AT.to_owned()),
        proof_kind: None,
    }
}
