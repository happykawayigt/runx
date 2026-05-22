use std::path::Path;

use serde::Deserialize;

use runx_contracts::{
    OperationalPolicy, OperationalPolicyAction, OperationalPolicySourceProvider, Receipt,
    TargetRepoRunnerPlanRequest, TargetRepoRunnerSourceContext, plan_target_repo_runner,
    plan_target_repo_runner_dedupe_lookup,
};

const FIXTURE_JSON: &str =
    include_str!("../../../fixtures/external/nitrosend/issue-intake/api-source-thread.json");
const POLICY_JSON: &str = include_str!("../../../fixtures/operational-policy/nitrosend-like.json");
const POST_MERGE_JSON: &str = include_str!(
    "../../../fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json"
);

#[derive(Debug, Deserialize)]
struct ExternalNitrosendFixture {
    schema: String,
    fixture_id: String,
    source: ExternalSource,
    signal: ExternalSignal,
    target: ExternalTarget,
    runtime_fixtures: Vec<String>,
    expected_target_plan: ExpectedTargetPlan,
    expected_dedupe_lookup: ExpectedDedupeLookup,
    post_merge_fixture: String,
}

#[derive(Debug, Deserialize)]
struct ExternalSource {
    source_id: String,
    provider: OperationalPolicySourceProvider,
    locator: String,
    thread_locator: String,
    thread_ts: String,
    issue_url: String,
}

#[derive(Debug, Deserialize)]
struct ExternalSignal {
    fingerprint: String,
}

#[derive(Debug, Deserialize)]
struct ExternalTarget {
    repo: String,
    action: String,
    runner_id: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedTargetPlan {
    policy_id: String,
    source_id: String,
    target_repo: String,
    owner_route_id: String,
    source_thread_required: bool,
    mutate_target_repo: bool,
    require_human_merge_gate: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedDedupeLookup {
    marker_prefix: String,
    required_source_thread_ref: String,
}

#[derive(Debug, Deserialize)]
struct HarnessFixture {
    expected: serde_json::Value,
}

#[test]
fn nitrosend_external_fixture_derives_target_plan_and_dedupe_lookup()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture: ExternalNitrosendFixture = serde_json::from_str(FIXTURE_JSON)?;
    let policy: OperationalPolicy = serde_json::from_str(POLICY_JSON)?;

    assert_eq!(fixture.schema, "runx.external_dogfood_fixture.v1");
    assert_eq!(fixture.fixture_id, "nitrosend-api-source-thread");
    assert_eq!(fixture.target.action, "issue-to-pr");

    let plan = plan_target_repo_runner(
        &policy,
        &TargetRepoRunnerPlanRequest {
            source_id: Some(fixture.source.source_id.clone()),
            target_repo: fixture.target.repo.clone(),
            action: OperationalPolicyAction::IssueToPr,
            runner_id: Some(fixture.target.runner_id.clone()),
            source: TargetRepoRunnerSourceContext {
                provider: fixture.source.provider,
                locator: fixture.source.locator.clone(),
                thread_locator: Some(fixture.source.thread_locator.clone()),
                thread_ts: Some(fixture.source.thread_ts.clone()),
                issue_url: Some(fixture.source.issue_url.clone()),
            },
            signal_fingerprint: Some(fixture.signal.fingerprint.clone()),
            existing_pull_request: None,
        },
    )?;

    assert_eq!(plan.policy_id, fixture.expected_target_plan.policy_id);
    assert_eq!(
        plan.source.source_id,
        fixture.expected_target_plan.source_id
    );
    assert_eq!(plan.target.repo, fixture.expected_target_plan.target_repo);
    assert_eq!(
        plan.owner.route_id,
        fixture.expected_target_plan.owner_route_id
    );
    assert_eq!(
        plan.source_thread.required,
        fixture.expected_target_plan.source_thread_required
    );
    assert_eq!(
        plan.mutate_target_repo,
        fixture.expected_target_plan.mutate_target_repo
    );
    assert_eq!(
        plan.require_human_merge_gate,
        fixture.expected_target_plan.require_human_merge_gate
    );

    let lookup = plan_target_repo_runner_dedupe_lookup(&plan);
    assert_eq!(
        lookup.source_thread_ref.uri,
        fixture.expected_dedupe_lookup.required_source_thread_ref
    );
    assert!(
        lookup
            .query
            .markers
            .iter()
            .any(|marker| { marker.starts_with(&fixture.expected_dedupe_lookup.marker_prefix) })
    );

    let serialized = serde_json::to_string(&lookup)?;
    assert!(!serialized.contains("/Users/"));
    assert!(!serialized.contains("/tmp/"));
    Ok(())
}

#[test]
fn nitrosend_external_fixture_cites_existing_runtime_and_post_merge_fixtures()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture: ExternalNitrosendFixture = serde_json::from_str(FIXTURE_JSON)?;
    let root = repo_root()?;

    for runtime_fixture in &fixture.runtime_fixtures {
        assert!(
            root.join(runtime_fixture).exists(),
            "runtime fixture does not exist: {runtime_fixture}"
        );
    }

    assert_eq!(
        fixture.post_merge_fixture,
        "fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json"
    );
    let harness_fixture: HarnessFixture = serde_json::from_str(POST_MERGE_JSON)?;
    let receipt: Receipt = serde_json::from_value(harness_fixture.expected)?;
    assert_eq!(receipt.seal.reason_code, "merged_verified");
    Ok(())
}

fn repo_root() -> Result<&'static Path, Box<dyn std::error::Error>> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "runx-contracts crate is under crates/".into())
}
