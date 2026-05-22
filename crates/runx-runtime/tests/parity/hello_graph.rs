use std::path::Path;

use runx_core::state_machine::GraphStatus;
use runx_runtime::run_graph_file;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedSummary {
    graph_name: String,
    state: String,
    step_ids: Vec<String>,
    stdout: Vec<String>,
    created_at: String,
    graph_seal_digest: String,
    child_seal_digests: Vec<String>,
    sandbox_profile: String,
    graph_receipt_id: String,
    child_receipt_ids: Vec<String>,
}

#[test]
fn hello_graph_matches_post_cutover_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let expected: ExpectedSummary = serde_json::from_str(include_str!(
        "../../../../fixtures/runtime/hello-graph/summary.json"
    ))?;
    let run = run_graph_file(Path::new("../../examples/hello-graph/graph.yaml"))?;

    assert_eq!(run.graph.name, expected.graph_name);
    assert_eq!(status_name(&run.state.status), expected.state);
    assert_eq!(
        run.steps
            .iter()
            .map(|step| step.step_id.clone())
            .collect::<Vec<_>>(),
        expected.step_ids
    );
    assert_eq!(
        run.steps
            .iter()
            .map(|step| step.output.stdout.clone())
            .collect::<Vec<_>>(),
        expected.stdout
    );
    assert_eq!(run.receipt.created_at, expected.created_at);
    assert_eq!(run.receipt.digest, expected.graph_seal_digest);
    assert_eq!(
        run.receipt.harness.enforcement.sandbox.profile,
        expected.sandbox_profile
    );
    for step in &run.steps {
        assert_eq!(step.receipt.created_at, expected.created_at);
        assert_eq!(
            step.receipt.harness.enforcement.sandbox.profile,
            expected.sandbox_profile
        );
    }
    assert_eq!(
        run.steps
            .iter()
            .map(|step| step.receipt.digest.clone())
            .collect::<Vec<_>>(),
        expected.child_seal_digests
    );
    assert_eq!(run.receipt.id, expected.graph_receipt_id);
    assert_eq!(
        run.steps
            .iter()
            .map(|step| step.receipt.id.clone())
            .collect::<Vec<_>>(),
        expected.child_receipt_ids
    );
    Ok(())
}

fn status_name(status: &GraphStatus) -> &'static str {
    match status {
        GraphStatus::Pending => "pending",
        GraphStatus::Running => "running",
        GraphStatus::Succeeded => "succeeded",
        GraphStatus::Failed => "failed",
        GraphStatus::Paused => "paused",
        GraphStatus::Escalated => "escalated",
    }
}
