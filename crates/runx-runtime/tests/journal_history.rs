use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use runx_contracts::HarnessReceipt;
use runx_runtime::journal::{
    HARNESS_RECEIPT_REF_PREFIX, HISTORY_PROJECTOR_ID, HistoryFilter, JOURNAL_PROJECTOR_ID,
    JournalProjectionError, PausedRunCheckpoint, exact_receipt_id, harness_receipt_ref,
    list_local_history, list_local_history_with_checkpoints, project_journal_for_receipt,
    project_receipt_journal,
};
use runx_runtime::{InvocationStatus, LocalReceiptStore, SkillOutput};
use serde_json::json;

const JOURNAL_ORACLE: &str = include_str!("../../../fixtures/journal/history-oracle.json");

#[test]
fn missing_history_store_projects_empty_safe_result() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let workspace = temp.path().join("workspace");
    let project_runx_dir = workspace.join(".runx");
    let store = LocalReceiptStore::new(project_runx_dir.join("receipts"));

    let history = list_local_history(
        &store,
        &workspace,
        &project_runx_dir,
        &HistoryFilter::default(),
    )?;

    assert_eq!(history.projector_id, HISTORY_PROJECTOR_ID);
    assert_eq!(history.store_label, ".runx/receipts");
    assert!(history.receipts.is_empty());
    assert_no_local_paths(&serde_json::to_string(&history)?);
    Ok(())
}

#[test]
fn history_lists_receipts_newest_first_with_safe_refs_and_filters()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let workspace = temp.path().join("workspace");
    let project_runx_dir = workspace.join(".runx");
    let store = LocalReceiptStore::new(project_runx_dir.join("receipts"));
    store.write_receipt(&receipt_with_metadata(
        InvocationStatus::Success,
        "hrn_rcpt_old",
        "2026-05-18T00:00:00Z",
        "Revision Skill",
        "local",
        "runner-a",
    )?)?;
    store.write_receipt(&receipt_with_metadata(
        InvocationStatus::Success,
        "hrn_rcpt_new",
        "2026-05-19T00:00:00Z",
        "Deploy Skill",
        "local",
        "runner-b",
    )?)?;

    let history = list_local_history(
        &store,
        &workspace,
        &project_runx_dir,
        &HistoryFilter {
            query: Some("artifact".to_owned()),
            source: Some("LOCAL".to_owned()),
            since: Some("2026-05-18T00:00:00Z".to_owned()),
            limit: Some(2),
            ..HistoryFilter::default()
        },
    )?;
    let oracle: serde_json::Value = serde_json::from_str(JOURNAL_ORACLE)?;
    let expected_order = oracle
        .get("history_order")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing history_order"))?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "history_order entry is not string",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(
        history
            .receipts
            .iter()
            .map(|receipt| receipt.receipt_ref.clone())
            .collect::<Vec<_>>(),
        expected_order
    );
    assert_eq!(history.receipts[0].name, "Deploy Skill");
    assert_eq!(history.receipts[0].source_type.as_deref(), Some("local"));
    assert_eq!(history.receipts[1].actors, vec!["runner-a"]);
    assert!(
        history.receipts[0]
            .artifact_types
            .contains(&"artifact".to_owned())
    );
    assert!(
        history.receipts[0]
            .receipt_ref
            .starts_with(HARNESS_RECEIPT_REF_PREFIX)
    );
    assert_no_local_paths(&serde_json::to_string(&history)?);
    Ok(())
}

#[test]
fn history_filter_matches_actor_status_skill_and_date() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let workspace = temp.path().join("workspace");
    let project_runx_dir = workspace.join(".runx");
    let store = LocalReceiptStore::new(project_runx_dir.join("receipts"));
    store.write_receipt(&receipt_with_metadata(
        InvocationStatus::Success,
        "hrn_rcpt_revision",
        "2026-05-18T00:01:00Z",
        "Revision Skill",
        "local",
        "runner-a",
    )?)?;
    store.write_receipt(&receipt_with_metadata(
        InvocationStatus::Failure,
        "hrn_rcpt_deploy",
        "2026-05-19T00:01:00Z",
        "Deploy Skill",
        "local",
        "runner-b",
    )?)?;

    let history = list_local_history(
        &store,
        &workspace,
        &project_runx_dir,
        &HistoryFilter {
            skill: Some("revision".to_owned()),
            status: Some("closed".to_owned()),
            actor: Some("runner-a".to_owned()),
            until: Some("2026-05-18T23:59:59Z".to_owned()),
            ..HistoryFilter::default()
        },
    )?;

    assert_eq!(history.receipts.len(), 1);
    assert_eq!(history.receipts[0].id, "hrn_rcpt_revision");
    Ok(())
}

#[test]
fn history_rejects_invalid_date_filters() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let workspace = temp.path().join("workspace");
    let project_runx_dir = workspace.join(".runx");
    let store = LocalReceiptStore::new(project_runx_dir.join("receipts"));
    let oracle: serde_json::Value = serde_json::from_str(JOURNAL_ORACLE)?;
    let invalid_date = oracle
        .get("invalid_date_filter")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing invalid_date_filter"))?;
    let field = invalid_date
        .get("field")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing invalid date field"))?;
    let value = invalid_date
        .get("value")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing invalid date value"))?;

    let result = list_local_history(
        &store,
        &workspace,
        &project_runx_dir,
        &HistoryFilter {
            since: Some(value.to_owned()),
            ..HistoryFilter::default()
        },
    );

    assert!(matches!(
        result,
        Err(JournalProjectionError::InvalidTimestamp { field: actual, .. }) if actual == field
    ));
    Ok(())
}

#[test]
fn history_merges_paused_ledgers_and_checkpoints() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let workspace = temp.path().join("workspace");
    let project_runx_dir = workspace.join(".runx");
    let store = LocalReceiptStore::new(project_runx_dir.join("receipts"));
    write_paused_ledger(
        store.root(),
        "gx_paused0000000000000000000000ab",
        "sourcey",
        "2026-04-28T01:00:00.000Z",
    )?;

    let history = list_local_history_with_checkpoints(
        &store,
        &workspace,
        &project_runx_dir,
        &HistoryFilter {
            status: Some("paused".to_owned()),
            since: Some("2026-04-28T00:00:00Z".to_owned()),
            until: Some("2026-04-28T02:00:00+01:00".to_owned()),
            ..HistoryFilter::default()
        },
        &[PausedRunCheckpoint {
            id: "rx_checkpoint00000000000000000001".to_owned(),
            name: "checkpoint-skill".to_owned(),
            kind: "runx.harness_receipt.v1".to_owned(),
            started_at: Some("2026-04-28T00:30:00Z".to_owned()),
            selected_runner: Some("agent-step".to_owned()),
            step_ids: vec!["plan".to_owned()],
            step_labels: vec!["plan work".to_owned()],
        }],
    )?;
    let oracle: serde_json::Value = serde_json::from_str(JOURNAL_ORACLE)?;
    let paused = oracle
        .get("paused_run")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing paused_run"))?;
    let expected_id = paused
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing paused id"))?;
    let expected_name = paused
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing paused name"))?;
    let expected_runner = paused
        .get("selected_runner")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing paused runner"))?;

    assert!(history.receipts.is_empty());
    assert_eq!(
        history
            .pending_runs
            .iter()
            .map(|run| run.id.as_str())
            .collect::<Vec<_>>(),
        vec![expected_id, "rx_checkpoint00000000000000000001",]
    );
    assert_eq!(history.pending_runs[0].name, expected_name);
    assert_eq!(
        history.pending_runs[0].selected_runner.as_deref(),
        Some(expected_runner)
    );
    assert_eq!(history.pending_runs[0].step_ids, vec!["discover"]);
    assert_eq!(history.pending_runs[1].step_labels, vec!["plan work"]);
    assert_no_local_paths(&serde_json::to_string(&history)?);
    Ok(())
}

#[test]
fn history_does_not_double_list_paused_ledger_with_terminal_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let workspace = temp.path().join("workspace");
    let project_runx_dir = workspace.join(".runx");
    let store = LocalReceiptStore::new(project_runx_dir.join("receipts"));
    write_paused_ledger(
        store.root(),
        "gx_paused_terminal",
        "sourcey",
        "2026-04-28T01:00:00.000Z",
    )?;
    store.write_receipt(&receipt_with_metadata(
        InvocationStatus::Success,
        "gx_paused_terminal",
        "2026-04-28T02:00:00Z",
        "Terminal Skill",
        "local",
        "runner-a",
    )?)?;

    let history = list_local_history(
        &store,
        &workspace,
        &project_runx_dir,
        &HistoryFilter::default(),
    )?;

    assert_eq!(history.receipts.len(), 1);
    assert!(history.pending_runs.is_empty());
    Ok(())
}

#[test]
fn malformed_history_store_remains_typed_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    fs::create_dir_all(temp.path())?;
    fs::write(temp.path().join("hrn_rcpt_bad.json"), "{")?;
    let store = LocalReceiptStore::new(temp.path());

    let result = list_local_history(
        &store,
        temp.path(),
        &temp.path().join(".runx"),
        &HistoryFilter::default(),
    );

    assert!(matches!(
        result,
        Err(JournalProjectionError::ReceiptStore(
            runx_runtime::ReceiptStoreError::MalformedJson { .. }
        ))
    ));
    Ok(())
}

#[test]
fn history_projection_fails_structurally_valid_stale_receipt_digest()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let workspace = temp.path().join("workspace");
    let project_runx_dir = workspace.join(".runx");
    let store = LocalReceiptStore::new(project_runx_dir.join("receipts"));
    let mut receipt = generated_runtime_receipt()?;
    receipt.seal.digest = "sha256:stale".to_owned();
    if let Some(seal) = receipt.harness.seal.as_mut() {
        seal.digest = "sha256:stale".to_owned();
    }
    assert!(runx_receipts::verify_harness_receipt(&receipt).valid);
    write_receipt_json(store.root(), &receipt)?;

    let result = list_local_history(
        &store,
        &workspace,
        &project_runx_dir,
        &HistoryFilter::default(),
    );

    assert!(matches!(
        result,
        Err(JournalProjectionError::ReceiptStore(
            runx_runtime::ReceiptStoreError::ReceiptProofInvalid { .. }
        ))
    ));
    Ok(())
}

#[test]
fn history_projection_fails_structurally_valid_tampered_receipt_signature()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let workspace = temp.path().join("workspace");
    let project_runx_dir = workspace.join(".runx");
    let store = LocalReceiptStore::new(project_runx_dir.join("receipts"));
    let mut receipt = generated_runtime_receipt()?;
    receipt.signature.value = "sig:sha256:tampered".to_owned();
    assert!(runx_receipts::verify_harness_receipt(&receipt).valid);
    write_receipt_json(store.root(), &receipt)?;

    let result = list_local_history(
        &store,
        &workspace,
        &project_runx_dir,
        &HistoryFilter::default(),
    );

    assert!(matches!(
        result,
        Err(JournalProjectionError::ReceiptStore(
            runx_runtime::ReceiptStoreError::ReceiptProofInvalid { .. }
        ))
    ));
    Ok(())
}

#[test]
fn runtime_generated_receipts_project_verified() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let workspace = temp.path().join("workspace");
    let project_runx_dir = workspace.join(".runx");
    let store = LocalReceiptStore::new(project_runx_dir.join("receipts"));
    let receipt = generated_runtime_receipt()?;
    store.write_receipt(&receipt)?;

    let history = list_local_history(
        &store,
        &workspace,
        &project_runx_dir,
        &HistoryFilter::default(),
    )?;
    let journal = project_receipt_journal(&receipt);

    assert_eq!(history.receipts[0].verification.status, "verified");
    assert_eq!(
        receipt_journal_verification_status(&journal),
        Some("verified")
    );
    Ok(())
}

#[test]
fn journal_projection_uses_exact_refs_and_reprojects_deterministically()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let store = LocalReceiptStore::new(temp.path().join("receipts"));
    let receipt = receipt_with_metadata(
        InvocationStatus::Success,
        "hrn_rcpt_123",
        "2026-05-18T00:00:00Z",
        "Journal Skill",
        "local",
        "runner-a",
    )?;
    store.write_receipt(&receipt)?;

    let direct = project_journal_for_receipt(&store, "hrn_rcpt_123")?;
    let typed = project_journal_for_receipt(&store, "runx:harness_receipt:hrn_rcpt_123")?;
    let reprojected = project_receipt_journal(&receipt);
    let oracle: serde_json::Value = serde_json::from_str(JOURNAL_ORACLE)?;
    let expected_ref = oracle
        .get("journal_source_ref")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing journal_source_ref"))?;
    let expected_projector = oracle
        .get("projector_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing projector_id"))?;

    assert_eq!(direct, typed);
    assert_eq!(direct, reprojected);
    assert_eq!(direct.projector_id, JOURNAL_PROJECTOR_ID);
    assert_eq!(direct.projector_id, expected_projector);
    assert_eq!(direct.receipt_ref, expected_ref);
    assert!(direct.rows.iter().all(|row| {
        row.source_refs
            .iter()
            .any(|source_ref| source_ref == expected_ref)
    }));
    assert!(
        direct
            .rows
            .iter()
            .all(|row| row.watermark == direct.watermark)
    );
    assert_no_local_paths(&serde_json::to_string(&direct)?);
    Ok(())
}

#[test]
fn journal_lookup_does_not_use_suffix_matching() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TestDir::new()?;
    let store = LocalReceiptStore::new(temp.path().join("receipts"));
    let receipt = receipt_with_metadata(
        InvocationStatus::Success,
        "hrn_rcpt_123",
        "2026-05-18T00:00:00Z",
        "Journal Skill",
        "local",
        "runner-a",
    )?;
    store.write_receipt(&receipt)?;

    let result = project_journal_for_receipt(&store, "123");

    assert!(matches!(
        result,
        Err(JournalProjectionError::ReceiptStore(
            runx_runtime::ReceiptStoreError::MissingReceipt { .. }
        ))
    ));
    assert_eq!(
        exact_receipt_id("runx:harness_receipt:hrn_rcpt_123"),
        "hrn_rcpt_123"
    );
    assert_eq!(
        harness_receipt_ref("hrn_rcpt_123"),
        "runx:harness_receipt:hrn_rcpt_123"
    );
    Ok(())
}

fn receipt_with_metadata(
    status: InvocationStatus,
    id: &str,
    created_at: &str,
    skill_name: &str,
    source_type: &str,
    actor: &str,
) -> Result<HarnessReceipt, Box<dyn std::error::Error>> {
    let mut receipt = generated_runtime_receipt_with(id, status, created_at)?;
    receipt.metadata = Some(json_object(json!({
        "skill_name": skill_name,
        "source_type": source_type,
        "runner": {
            "provider": actor
        }
    }))?);
    reseal_receipt(&mut receipt)?;
    Ok(receipt)
}

fn generated_runtime_receipt() -> Result<HarnessReceipt, Box<dyn std::error::Error>> {
    generated_runtime_receipt_with(
        "hrn_rcpt_journal-history_strict-proof",
        InvocationStatus::Success,
        "2026-05-18T00:00:00Z",
    )
}

fn generated_runtime_receipt_with(
    id: &str,
    status: InvocationStatus,
    created_at: &str,
) -> Result<HarnessReceipt, Box<dyn std::error::Error>> {
    let succeeded = status == InvocationStatus::Success;
    let output = SkillOutput {
        status: status.clone(),
        stdout: format!(
            r#"{{"artifact":{{"artifact_id":"artifact_{id}","artifact_type":"artifact"}}}}"#
        ),
        stderr: String::new(),
        exit_code: Some(if succeeded { 0 } else { 1 }),
        duration_ms: 10,
        metadata: BTreeMap::new(),
    };
    let mut receipt = runx_runtime::receipts::step_receipt(
        "journal-history",
        "strict-proof",
        1,
        &output,
        created_at,
    )?;
    receipt.id = id.to_owned();
    reseal_receipt(&mut receipt)?;
    Ok(receipt)
}

fn reseal_receipt(receipt: &mut HarnessReceipt) -> Result<(), Box<dyn std::error::Error>> {
    let digest = runx_receipts::canonical_receipt_body_digest(receipt)?;
    receipt.seal.digest = digest.clone();
    if let Some(seal) = receipt.harness.seal.as_mut() {
        seal.digest = digest.clone();
    }
    receipt.signature.value = format!("sig:{digest}");
    Ok(())
}

fn receipt_journal_verification_status(
    projection: &runx_runtime::journal::JournalProjection,
) -> Option<&str> {
    projection
        .rows
        .iter()
        .find(|row| row.event_kind == "harness_receipt_sealed")
        .and_then(|row| row.verification.as_ref())
        .map(|verification| verification.status.as_str())
}

fn json_object(value: serde_json::Value) -> Result<runx_contracts::JsonObject, io::Error> {
    serde_json::from_value(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

fn write_receipt_json(
    dir: &Path,
    receipt: &HarnessReceipt,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join(format!("{}.json", receipt.id)),
        serde_json::to_string(receipt)?,
    )?;
    Ok(())
}

fn write_paused_ledger(
    receipt_dir: &Path,
    run_id: &str,
    skill_name: &str,
    created_at: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ledger_dir = receipt_dir.join("ledgers");
    fs::create_dir_all(&ledger_dir)?;
    let producer = json!({
        "skill": skill_name,
        "runner": "graph"
    });
    let started = ledger_record(json!({
        "type": "run_event",
        "version": "1",
        "data": {
            "kind": "run_started",
            "status": "started",
            "step_id": null,
            "detail": {}
        },
        "meta": ledger_meta(run_id, serde_json::Value::Null, producer.clone(), created_at, "ax_start")
    }));
    let waiting = ledger_record(json!({
        "type": "run_event",
        "version": "1",
        "data": {
            "kind": "step_waiting_resolution",
            "status": "waiting",
            "step_id": "discover",
            "detail": {
                "request_ids": ["agent_step.test-step.output"],
                "resolution_kinds": ["agent_act"],
                "step_ids": ["discover"],
                "step_labels": ["inspect repo"],
                "inputs": {},
                "selected_runner": "agent-step"
            }
        },
        "meta": ledger_meta(run_id, "discover", producer, created_at, "ax_wait")
    }));
    fs::write(
        ledger_dir.join(format!("{run_id}.jsonl")),
        format!(
            "{}\n{}\n",
            serde_json::to_string(&started)?,
            serde_json::to_string(&waiting)?
        ),
    )?;
    Ok(())
}

fn ledger_record(entry: serde_json::Value) -> serde_json::Value {
    json!({ "entry": entry })
}

fn ledger_meta(
    run_id: &str,
    step_id: impl Into<serde_json::Value>,
    producer: serde_json::Value,
    created_at: &str,
    artifact_id: &str,
) -> serde_json::Value {
    json!({
        "artifact_id": artifact_id,
        "run_id": run_id,
        "step_id": step_id.into(),
        "producer": producer,
        "created_at": created_at,
        "hash": "sha256:test",
        "size_bytes": 2,
        "parent_artifact_id": null,
        "receipt_id": null,
        "redacted": false
    })
}

fn assert_no_local_paths(text: &str) {
    assert!(!text.contains("/Users/"));
    assert!(!text.contains("/private/"));
    assert!(!text.contains("runx-runtime-journal-history"));
}

struct TestDir {
    path: PathBuf,
}

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

impl TestDir {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let serial = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!(
            "runx-runtime-journal-history-{}-{serial}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.path);
    }
}
