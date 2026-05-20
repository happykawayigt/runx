use runx_contracts::{
    JsonValue, OperationalPolicy, OperationalPolicyAction, TargetRepoRunnerDedupeLookupObservation,
    TargetRepoRunnerDedupeResult, TargetRepoRunnerExistingPullRequest, TargetRepoRunnerPlanRequest,
    TargetRepoRunnerProvider, TargetRepoRunnerProviderPullRequest,
    TargetRepoRunnerPullRequestDisposition, TargetRepoRunnerReadinessObservation,
    TargetRepoRunnerSourceContext, plan_target_repo_runner, plan_target_repo_runner_execution,
};
use runx_runtime::target_runner::{
    TargetRepoRunnerFixtureExecutionInput, TargetRepoRunnerRuntimeError,
    execute_target_repo_runner_execution_fixture, execute_target_repo_runner_fixture,
};

const NITROSEND_LIKE: &str =
    include_str!("../../../fixtures/operational-policy/nitrosend-like.json");

#[test]
fn runtime_fails_closed_before_dedupe_or_pr_when_checkout_not_scafld_ready()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let plan = plan_target_repo_runner(&policy, &nitrosend_request("nitrosend/api"))?;

    let error = execute_target_repo_runner_fixture(TargetRepoRunnerFixtureExecutionInput {
        plan,
        readiness: readiness(false),
        dedupe: empty_dedupe_observation("nitrosend/api", "unobserved"),
        created_pull_request: Some(created_pull_request()),
    })
    .err();

    assert!(matches!(
        error,
        Some(TargetRepoRunnerRuntimeError::Plan(
            runx_contracts::TargetRepoRunnerPlanError::NotScafldReady { .. }
        ))
    ));
    Ok(())
}

#[test]
fn runtime_rechecks_execution_readiness_before_mutation_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let plan = plan_target_repo_runner(&policy, &nitrosend_request("nitrosend/api"))?;
    let execution_plan = plan_target_repo_runner_execution(&plan, &readiness(true))?;

    let error = execute_target_repo_runner_execution_fixture(
        &plan,
        &execution_plan,
        &readiness(false),
        &empty_dedupe_observation("nitrosend/api", &execution_plan.provider_lookup.key),
        Some(&created_pull_request()),
    )
    .err();

    assert!(matches!(
        error,
        Some(TargetRepoRunnerRuntimeError::ReadinessMismatch(_))
    ));
    Ok(())
}

#[test]
fn runtime_creates_pull_request_when_provider_dedupe_has_no_match()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let plan = plan_target_repo_runner(&policy, &nitrosend_request("nitrosend/api"))?;
    let execution_plan = plan_target_repo_runner_execution(&plan, &readiness(true))?;
    let created = created_pull_request();

    let execution = execute_target_repo_runner_execution_fixture(
        &plan,
        &execution_plan,
        &readiness(true),
        &empty_dedupe_observation("nitrosend/api", &execution_plan.provider_lookup.key),
        Some(&created),
    )?;

    assert_eq!(
        execution.disposition,
        TargetRepoRunnerPullRequestDisposition::Create
    );
    assert_eq!(
        execution.dedupe_execution.result,
        TargetRepoRunnerDedupeResult::LookupRequired
    );
    assert_eq!(execution.pull_request.url, created.url);
    assert_eq!(
        nested_string(
            &execution.pull_request_receipt.metadata,
            &["dedupe", "result"]
        ),
        Some("created")
    );
    assert_eq!(
        nested_string(
            &execution.pull_request_receipt.metadata,
            &["source", "thread_uri"]
        ),
        Some("slack://nitrosend/C0APFMY0V8Q/1778834840.485629")
    );
    assert_eq!(
        execution.source_publication_receipt.pull_request_ref.uri,
        "https://github.com/nitrosend/api/pull/145"
    );
    assert_public_only(&execution)?;
    Ok(())
}

#[test]
fn runtime_reuses_provider_pull_request_when_dedupe_refs_match()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let plan = plan_target_repo_runner(&policy, &nitrosend_request("nitrosend/api"))?;
    let execution_plan = plan_target_repo_runner_execution(&plan, &readiness(true))?;
    let observed = TargetRepoRunnerProviderPullRequest {
        url: "https://github.com/nitrosend/api/pull/144".to_owned(),
        number: Some(144),
        branch: Some("runx/source-482".to_owned()),
        open: true,
        markers: execution_plan.provider_lookup.query.markers.clone(),
        refs: execution_plan.provider_lookup.query.required_refs.clone(),
    };

    let execution = execute_target_repo_runner_execution_fixture(
        &plan,
        &execution_plan,
        &readiness(true),
        &TargetRepoRunnerDedupeLookupObservation {
            provider: TargetRepoRunnerProvider::Github,
            target_repo: "nitrosend/api".to_owned(),
            key: execution_plan.provider_lookup.key.clone(),
            pull_requests: vec![observed],
        },
        None,
    )?;

    assert_eq!(
        execution.disposition,
        TargetRepoRunnerPullRequestDisposition::Reuse
    );
    assert_eq!(
        execution.dedupe_execution.result,
        TargetRepoRunnerDedupeResult::Reused
    );
    assert_eq!(execution.pull_request.number, Some(144));
    assert_eq!(
        execution
            .deduped_plan
            .dedupe
            .existing_pull_request
            .as_ref()
            .map(|pull_request| pull_request.url.as_str()),
        Some("https://github.com/nitrosend/api/pull/144")
    );
    assert_eq!(
        nested_string(
            &execution.pull_request_receipt.metadata,
            &["dedupe", "result"]
        ),
        Some("reused")
    );
    assert_eq!(
        execution.source_publication_receipt.source_thread_ref.uri,
        "slack://nitrosend/C0APFMY0V8Q/1778834840.485629"
    );
    assert_public_only(&execution)?;
    Ok(())
}

#[test]
fn runtime_requires_created_pull_request_for_create_decision()
-> Result<(), Box<dyn std::error::Error>> {
    let policy: OperationalPolicy = serde_json::from_str(NITROSEND_LIKE)?;
    let plan = plan_target_repo_runner(&policy, &nitrosend_request("nitrosend/api"))?;
    let execution_plan = plan_target_repo_runner_execution(&plan, &readiness(true))?;

    let error = execute_target_repo_runner_execution_fixture(
        &plan,
        &execution_plan,
        &readiness(true),
        &empty_dedupe_observation("nitrosend/api", &execution_plan.provider_lookup.key),
        None,
    )
    .err();

    assert!(matches!(
        error,
        Some(TargetRepoRunnerRuntimeError::CreatedPullRequestRequired { .. })
    ));
    Ok(())
}

fn nitrosend_request(target_repo: &str) -> TargetRepoRunnerPlanRequest {
    TargetRepoRunnerPlanRequest {
        source_id: Some("bugs-fixes".to_owned()),
        target_repo: target_repo.to_owned(),
        action: OperationalPolicyAction::IssueToPr,
        runner_id: None,
        source: TargetRepoRunnerSourceContext {
            provider: runx_contracts::OperationalPolicySourceProvider::Slack,
            locator: "slack://nitrosend/C0APFMY0V8Q".to_owned(),
            thread_locator: Some("slack://nitrosend/C0APFMY0V8Q/1778834840.485629".to_owned()),
            thread_ts: Some("1778834840.485629".to_owned()),
            issue_url: Some("https://github.com/nitrosend/nitrosend/issues/482".to_owned()),
        },
        signal_fingerprint: Some("sha256:nitrosend-source-482".to_owned()),
        existing_pull_request: None,
    }
}

fn readiness(scafld_ready: bool) -> TargetRepoRunnerReadinessObservation {
    TargetRepoRunnerReadinessObservation {
        target_repo: "nitrosend/api".to_owned(),
        runner_id: "aster-production".to_owned(),
        scafld_ready,
    }
}

fn empty_dedupe_observation(
    target_repo: &str,
    key: &str,
) -> TargetRepoRunnerDedupeLookupObservation {
    TargetRepoRunnerDedupeLookupObservation {
        provider: TargetRepoRunnerProvider::Github,
        target_repo: target_repo.to_owned(),
        key: key.to_owned(),
        pull_requests: Vec::new(),
    }
}

fn created_pull_request() -> TargetRepoRunnerExistingPullRequest {
    TargetRepoRunnerExistingPullRequest {
        url: "https://github.com/nitrosend/api/pull/145".to_owned(),
        number: Some(145),
        branch: Some("runx/source-482-new".to_owned()),
    }
}

fn nested_string<'a>(
    object: &'a std::collections::BTreeMap<String, JsonValue>,
    path: &[&str],
) -> Option<&'a str> {
    let mut value = object.get(*path.first()?)?;
    for segment in &path[1..] {
        let JsonValue::Object(object) = value else {
            return None;
        };
        value = object.get(*segment)?;
    }
    match value {
        JsonValue::String(value) => Some(value.as_str()),
        _ => None,
    }
}

fn assert_public_only<T: serde::Serialize>(value: &T) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(value)?;
    assert!(!json.contains("/Users/"));
    assert!(!json.contains("/tmp/"));
    assert!(!json.contains("RUNX_"));
    assert!(!json.contains("SECRET"));
    Ok(())
}
