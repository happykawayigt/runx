#![cfg(feature = "cli-tool")]

use std::path::Path;

use runx_core::state_machine::GraphStatus;
use runx_receipts::validate_receipt_tree;
use runx_runtime::adapters::cli_tool::CliToolAdapter;
use runx_runtime::{Runtime, RuntimeOptions, run_graph_file};

#[test]
fn hello_graph_runs_to_receipt_tree() -> Result<(), Box<dyn std::error::Error>> {
    let run = run_graph_file(Path::new("../../examples/hello-graph/graph.yaml"))?;

    assert_eq!(run.graph.name, "hello-graph");
    assert_eq!(run.state.status, GraphStatus::Succeeded);
    assert_eq!(
        run.steps
            .iter()
            .map(|step| step.step_id.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    assert_eq!(run.steps[0].output.stdout, "hello from graph\n");
    assert!(run.steps[1].output.stdout.starts_with("hello from graph"));

    let children = run
        .steps
        .iter()
        .map(|step| step.receipt.clone())
        .collect::<Vec<_>>();
    assert!(validate_receipt_tree(&run.receipt, &children).is_ok());
    Ok(())
}

#[test]
fn hello_graph_resumes_from_checkpoint() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new(CliToolAdapter, RuntimeOptions::default());
    let graph_path = Path::new("../../examples/hello-graph/graph.yaml");

    let checkpoint = runtime.run_graph_file_until_steps(graph_path, 1)?;
    assert_eq!(checkpoint.steps.len(), 1);
    assert_eq!(checkpoint.steps[0].step_id, "first");

    let run = runtime.resume_graph_file(graph_path, checkpoint)?;
    assert_eq!(run.state.status, GraphStatus::Succeeded);
    assert_eq!(
        run.steps
            .iter()
            .map(|step| step.step_id.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    Ok(())
}
