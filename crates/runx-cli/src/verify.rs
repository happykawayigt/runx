// rust-style-allow: large-file - verify owns legacy receipt-tree checks plus the new single-receipt machine verdict.
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use runx_contracts::{Receipt, Reference, ReferenceType};
use runx_receipts::{
    ReceiptProofContext, ReceiptVerifySignatureMode, ReceiptVerifyVerdict,
    SignatureVerificationFailure, SignatureVerifier, verify_receipt_document_verdict,
};
use runx_runtime::{
    Ed25519ReceiptVerifier, ReceiptPathInputs, ReceiptTreeConfig, RuntimeReceiptConfig,
    RuntimeReceiptSignaturePolicy, resolve_receipt_path, verify_runtime_receipt_tree_with_policy,
};
use serde::Serialize;

use crate::history::{
    RUNX_RECEIPT_VERIFY_ED25519_PUBLIC_KEY_BASE64_ENV, RUNX_RECEIPT_VERIFY_KID_ENV,
};

const RECEIPT_REFERENCE_PREFIX: &str = "runx:receipt:";
const SINGLE_RECEIPT_MAX_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug)]
pub enum VerifyCliError {
    InvalidArgs(String),
    InvalidReceiptVerifier(String),
    Store(String),
    Serialize(serde_json::Error),
}

impl fmt::Display for VerifyCliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgs(message) | Self::InvalidReceiptVerifier(message) => {
                formatter.write_str(message)
            }
            Self::Store(message) => formatter.write_str(message),
            Self::Serialize(error) => write!(formatter, "failed to serialize report: {error}"),
        }
    }
}

impl std::error::Error for VerifyCliError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyCliResult {
    pub output: String,
    pub failed: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ParsedVerifyArgs {
    receipt_id: Option<String>,
    receipt_dir: Option<PathBuf>,
    receipt: Option<ReceiptInput>,
    json: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ReceiptInput {
    Path(PathBuf),
    Stdin,
}

#[derive(Clone, Debug, Serialize)]
struct VerifyReport {
    receipt_dir: String,
    signature_mode: &'static str,
    trees: Vec<TreeReport>,
    unreadable_files: Vec<FileIssue>,
    valid: bool,
}

#[derive(Clone, Debug, Serialize)]
struct TreeReport {
    root_receipt_id: String,
    receipt_count: usize,
    parent_missing: Option<String>,
    valid: bool,
    findings: Vec<FindingReport>,
}

#[derive(Clone, Debug, Serialize)]
struct FindingReport {
    code: String,
    path: String,
    message: String,
}

#[derive(Clone, Debug, Serialize)]
struct FileIssue {
    file: String,
    message: String,
}

pub fn run_verify_command(
    args: &[OsString],
    env: &BTreeMap<String, String>,
    cwd: &Path,
) -> Result<VerifyCliResult, VerifyCliError> {
    run_verify_command_with_stdin(args, env, cwd, io::empty())
}

// rust-style-allow: long-function - argument dispatch keeps tree and single-receipt modes mutually exclusive in one parser.
pub fn run_verify_command_with_stdin<R: Read>(
    args: &[OsString],
    env: &BTreeMap<String, String>,
    cwd: &Path,
    stdin: R,
) -> Result<VerifyCliResult, VerifyCliError> {
    let parsed = parse_verify_args(args)?;
    if let Some(input) = parsed.receipt.as_ref() {
        return run_single_receipt_verify(input, parsed.json, env, cwd, stdin);
    }
    let receipt_config = RuntimeReceiptConfig::default();
    let resolved = resolve_receipt_path(ReceiptPathInputs {
        explicit_dir: parsed.receipt_dir.as_deref(),
        runtime_config: Some(&receipt_config),
        env,
        cwd,
    });
    let verifier = production_verifier(env)?;
    let signature_mode = if verifier.is_some() {
        "production"
    } else {
        "local-development"
    };

    let (receipts, unreadable_files) = load_receipts(&resolved.path)?;
    let trees = group_trees(&receipts);

    let selected: Vec<&ReceiptTree> = match parsed.receipt_id.as_deref() {
        Some(receipt_id) => {
            let tree = trees
                .iter()
                .find(|tree| tree.member_ids.contains(receipt_id))
                .ok_or_else(|| {
                    VerifyCliError::InvalidArgs(format!(
                        "receipt {receipt_id} was not found in {}",
                        resolved.path.display()
                    ))
                })?;
            vec![tree]
        }
        None => trees.iter().collect(),
    };

    let mut tree_reports = Vec::new();
    for tree in selected {
        let policy = match verifier.as_ref() {
            Some(verifier) => RuntimeReceiptSignaturePolicy::production(verifier),
            None => RuntimeReceiptSignaturePolicy::local_development(),
        };
        // Supplied receipts are the candidate children; the root itself is the
        // traversal anchor and must not be offered as its own child.
        let members: Vec<Receipt> = tree
            .member_ids
            .iter()
            .filter(|id| id.as_str() != tree.root.id.as_str())
            .filter_map(|id| receipts.iter().find(|receipt| receipt.id.as_str() == id))
            .cloned()
            .collect();
        let verification = verify_runtime_receipt_tree_with_policy(
            &tree.root,
            members,
            ReceiptTreeConfig::default(),
            policy,
        );
        let valid = verification.valid && tree.parent_missing.is_none();
        tree_reports.push(TreeReport {
            root_receipt_id: tree.root.id.to_string(),
            receipt_count: tree.member_ids.len(),
            parent_missing: tree.parent_missing.clone(),
            valid,
            findings: verification
                .findings
                .into_iter()
                .map(|finding| FindingReport {
                    code: format!("{:?}", finding.code),
                    path: finding.path,
                    message: finding.message,
                })
                .collect(),
        });
    }

    let valid = tree_reports.iter().all(|tree| tree.valid) && unreadable_files.is_empty();
    let report = VerifyReport {
        receipt_dir: resolved.path.display().to_string(),
        signature_mode,
        trees: tree_reports,
        unreadable_files,
        valid,
    };

    let output = if parsed.json {
        format!(
            "{}\n",
            serde_json::to_string_pretty(&report).map_err(VerifyCliError::Serialize)?
        )
    } else {
        render_report(&report)
    };
    Ok(VerifyCliResult {
        output,
        failed: !report.valid,
    })
}

fn parse_verify_args(args: &[OsString]) -> Result<ParsedVerifyArgs, VerifyCliError> {
    let mut parsed = ParsedVerifyArgs::default();
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        let Some(text) = arg.to_str() else {
            return Err(invalid_args("arguments must be valid UTF-8"));
        };
        match text {
            "--json" => parsed.json = true,
            "--receipt-dir" => {
                let value = iter
                    .next()
                    .ok_or_else(|| invalid_args("--receipt-dir requires a directory"))?;
                parsed.receipt_dir = Some(PathBuf::from(value));
            }
            "--receipt" => {
                let value = iter
                    .next()
                    .ok_or_else(|| invalid_args("--receipt requires a path or -"))?;
                parsed.receipt = Some(parse_receipt_input(value)?);
            }
            other if other.starts_with("--receipt=") => {
                let value = other.trim_start_matches("--receipt=");
                parsed.receipt = Some(parse_receipt_input_text(value)?);
            }
            other if other.starts_with("--") => {
                return Err(invalid_args(format!("unknown verify flag {other}")));
            }
            other => {
                if parsed.receipt_id.is_some() {
                    return Err(invalid_args("verify accepts at most one receipt id"));
                }
                parsed.receipt_id = Some(other.to_owned());
            }
        }
    }
    if parsed.receipt.is_some() && (parsed.receipt_id.is_some() || parsed.receipt_dir.is_some()) {
        return Err(invalid_args(
            "--receipt cannot be combined with a receipt id or --receipt-dir",
        ));
    }
    Ok(parsed)
}

fn parse_receipt_input(value: &OsString) -> Result<ReceiptInput, VerifyCliError> {
    let Some(text) = value.to_str() else {
        return Err(invalid_args("--receipt path must be valid UTF-8"));
    };
    parse_receipt_input_text(text)
}

fn parse_receipt_input_text(value: &str) -> Result<ReceiptInput, VerifyCliError> {
    if value.is_empty() {
        return Err(invalid_args("--receipt requires a path or -"));
    }
    Ok(if value == "-" {
        ReceiptInput::Stdin
    } else {
        ReceiptInput::Path(PathBuf::from(value))
    })
}

fn run_single_receipt_verify<R: Read>(
    input: &ReceiptInput,
    json: bool,
    env: &BTreeMap<String, String>,
    cwd: &Path,
    stdin: R,
) -> Result<VerifyCliResult, VerifyCliError> {
    let document = read_single_receipt_input(input, cwd, stdin)?;
    let verifier = production_verifier(env)?;
    let local_verifier = LocalDevelopmentReceiptVerifier;
    let (signature_mode, signature_verifier): (ReceiptVerifySignatureMode, &dyn SignatureVerifier) =
        match verifier.as_ref() {
            Some(verifier) => (ReceiptVerifySignatureMode::Production, verifier),
            None => (
                ReceiptVerifySignatureMode::LocalDevelopment,
                &local_verifier,
            ),
        };
    let context = ReceiptProofContext {
        signature_verifier: Some(signature_verifier),
        authority_verified: false,
        external_attestations_verified: false,
        verified_redaction_refs: BTreeSet::new(),
        verified_hash_commitments: BTreeSet::new(),
    };
    let verdict = verify_receipt_document_verdict(&document, &context, signature_mode);
    let output = if json {
        format!(
            "{}\n",
            serde_json::to_string_pretty(&verdict).map_err(VerifyCliError::Serialize)?
        )
    } else {
        render_single_receipt_verdict(&verdict)
    };
    Ok(VerifyCliResult {
        output,
        failed: !verdict.valid,
    })
}

fn read_single_receipt_input<R: Read>(
    input: &ReceiptInput,
    cwd: &Path,
    stdin: R,
) -> Result<Vec<u8>, VerifyCliError> {
    match input {
        ReceiptInput::Path(path) => {
            let path = if path.is_absolute() {
                path.clone()
            } else {
                cwd.join(path)
            };
            if let Ok(metadata) = fs::metadata(&path) {
                if metadata.len() > SINGLE_RECEIPT_MAX_BYTES as u64 {
                    return Err(single_receipt_too_large());
                }
            }
            let document = fs::read(&path).map_err(|error| {
                VerifyCliError::Store(format!(
                    "failed to read receipt {}: {error}",
                    path.display()
                ))
            })?;
            if document.len() > SINGLE_RECEIPT_MAX_BYTES {
                return Err(single_receipt_too_large());
            }
            Ok(document)
        }
        ReceiptInput::Stdin => read_limited_stdin(stdin),
    }
}

fn read_limited_stdin<R: Read>(stdin: R) -> Result<Vec<u8>, VerifyCliError> {
    let mut limited = stdin.take((SINGLE_RECEIPT_MAX_BYTES + 1) as u64);
    let mut document = Vec::new();
    limited.read_to_end(&mut document).map_err(|error| {
        VerifyCliError::Store(format!("failed to read receipt from stdin: {error}"))
    })?;
    if document.len() > SINGLE_RECEIPT_MAX_BYTES {
        return Err(single_receipt_too_large());
    }
    Ok(document)
}

fn production_verifier(
    env: &BTreeMap<String, String>,
) -> Result<Option<Ed25519ReceiptVerifier>, VerifyCliError> {
    let kid = non_empty_env(env, RUNX_RECEIPT_VERIFY_KID_ENV);
    let public_key = non_empty_env(env, RUNX_RECEIPT_VERIFY_ED25519_PUBLIC_KEY_BASE64_ENV);
    match (kid, public_key) {
        (None, None) => Ok(None),
        (Some(kid), Some(public_key)) => {
            Ed25519ReceiptVerifier::from_public_key_base64(kid.to_owned(), public_key)
                .map(Some)
                .map_err(|_| {
                    VerifyCliError::InvalidReceiptVerifier(format!(
                        "{RUNX_RECEIPT_VERIFY_ED25519_PUBLIC_KEY_BASE64_ENV} is not valid Ed25519 public key material"
                    ))
                })
        }
        _ => Err(VerifyCliError::InvalidReceiptVerifier(format!(
            "set both {RUNX_RECEIPT_VERIFY_KID_ENV} and {RUNX_RECEIPT_VERIFY_ED25519_PUBLIC_KEY_BASE64_ENV} for production verification"
        ))),
    }
}

fn load_receipts(root: &Path) -> Result<(Vec<Receipt>, Vec<FileIssue>), VerifyCliError> {
    let mut receipts = Vec::new();
    let mut issues = Vec::new();
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            return Err(VerifyCliError::Store(format!(
                "failed to read receipt dir {}: {error}",
                root.display()
            )));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| {
            VerifyCliError::Store(format!(
                "failed to read receipt dir {}: {error}",
                root.display()
            ))
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json")
            || path.file_name().and_then(|value| value.to_str()) == Some("index.json")
        {
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<Receipt>(&contents) {
                Ok(receipt) => receipts.push(receipt),
                Err(error) => issues.push(FileIssue {
                    file: path.display().to_string(),
                    message: format!("not a valid receipt: {error}"),
                }),
            },
            Err(error) => issues.push(FileIssue {
                file: path.display().to_string(),
                message: format!("unreadable: {error}"),
            }),
        }
    }
    receipts.sort_by(|left, right| left.id.cmp(&right.id));
    Ok((receipts, issues))
}

#[derive(Clone, Debug)]
struct ReceiptTree {
    root: Receipt,
    member_ids: BTreeSet<String>,
    /// Set when the chain above this root points at a receipt id that is not
    /// present in the store; the tree is then verified from the highest
    /// available node but reported as incomplete.
    parent_missing: Option<String>,
}

fn group_trees(receipts: &[Receipt]) -> Vec<ReceiptTree> {
    let by_id: BTreeMap<&str, &Receipt> = receipts
        .iter()
        .map(|receipt| (receipt.id.as_str(), receipt))
        .collect();

    let mut trees: BTreeMap<String, ReceiptTree> = BTreeMap::new();
    for receipt in receipts {
        let (root_id, parent_missing) = resolve_root(receipt, &by_id);
        let root = by_id
            .get(root_id.as_str())
            .copied()
            .unwrap_or(receipt)
            .clone();
        let tree = trees.entry(root_id).or_insert_with(|| ReceiptTree {
            root,
            member_ids: BTreeSet::new(),
            parent_missing: None,
        });
        tree.member_ids.insert(receipt.id.to_string());
        if let Some(missing) = parent_missing {
            tree.parent_missing.get_or_insert(missing);
        }
    }
    trees.into_values().collect()
}

/// Follow lineage parents to the highest receipt available in the store.
/// Returns the root id plus the first missing parent id, if the chain breaks.
fn resolve_root(receipt: &Receipt, by_id: &BTreeMap<&str, &Receipt>) -> (String, Option<String>) {
    let mut current = receipt;
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(current.id.to_string()) {
            // Cycles are reported by tree verification; anchor on the starting
            // receipt so the walk terminates.
            return (receipt.id.to_string(), None);
        }
        let Some(parent_id) = current
            .lineage
            .as_ref()
            .and_then(|lineage| lineage.parent.as_ref())
            .and_then(referenced_receipt_id)
        else {
            return (current.id.to_string(), None);
        };
        match by_id.get(parent_id) {
            Some(parent) => current = parent,
            None => return (current.id.to_string(), Some(parent_id.to_owned())),
        }
    }
}

fn referenced_receipt_id(reference: &Reference) -> Option<&str> {
    if reference.reference_type != ReferenceType::Receipt {
        return None;
    }
    reference
        .uri
        .strip_prefix(RECEIPT_REFERENCE_PREFIX)
        .filter(|id| !id.is_empty())
}

fn render_report(report: &VerifyReport) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "receipt dir: {}\nsignature mode: {}\n",
        report.receipt_dir, report.signature_mode
    ));
    if report.signature_mode == "local-development" {
        output.push_str(
            "note: set RUNX_RECEIPT_VERIFY_KID and RUNX_RECEIPT_VERIFY_ED25519_PUBLIC_KEY_BASE64 to verify production signatures\n",
        );
    }
    if report.trees.is_empty() {
        output.push_str("no receipts found\n");
    }
    for tree in &report.trees {
        let status = if tree.valid { "ok" } else { "INVALID" };
        output.push_str(&format!(
            "tree {} ({} receipt{}): {status}\n",
            tree.root_receipt_id,
            tree.receipt_count,
            if tree.receipt_count == 1 { "" } else { "s" },
        ));
        if let Some(missing) = &tree.parent_missing {
            output.push_str(&format!("  missing parent receipt: {missing}\n"));
        }
        for finding in &tree.findings {
            output.push_str(&format!(
                "  {} at {}: {}\n",
                finding.code,
                if finding.path.is_empty() {
                    "<root>"
                } else {
                    &finding.path
                },
                finding.message
            ));
        }
    }
    for issue in &report.unreadable_files {
        output.push_str(&format!("unreadable {}: {}\n", issue.file, issue.message));
    }
    output.push_str(if report.valid {
        "verification: ok\n"
    } else {
        "verification: FAILED\n"
    });
    output
}

fn render_single_receipt_verdict(verdict: &ReceiptVerifyVerdict) -> String {
    let mut output = String::new();
    output.push_str("receipt verification\n");
    output.push_str(&format!(
        "receipt: {}\n",
        verdict.receipt_id.as_deref().unwrap_or("<unparsed>")
    ));
    output.push_str(&format!(
        "signature: {} ({})\n",
        verdict.signature.status, verdict.signature.mode
    ));
    output.push_str(&format!("digest: {}\n", verdict.digest.status));
    output.push_str(&format!(
        "content address: {}\n",
        verdict.content_address.status
    ));
    output.push_str(&format!("lineage: {}\n", verdict.lineage.status));
    for finding in &verdict.findings {
        output.push_str(&format!(
            "  {} at {}: {}\n",
            finding.code,
            if finding.path.is_empty() {
                "<root>"
            } else {
                &finding.path
            },
            finding.message
        ));
    }
    output.push_str(if verdict.valid {
        "verification: ok\n"
    } else {
        "verification: FAILED\n"
    });
    output
}

struct LocalDevelopmentReceiptVerifier;

impl SignatureVerifier for LocalDevelopmentReceiptVerifier {
    fn verify(
        &self,
        _issuer: &runx_contracts::ReceiptIssuer,
        signature: &runx_contracts::ReceiptSignature,
        body_digest: &str,
    ) -> Result<(), SignatureVerificationFailure> {
        if !signature.value.starts_with("sig:sha256:") {
            return Err(SignatureVerificationFailure::MalformedSignature);
        }
        if signature.value == format!("sig:{body_digest}") {
            Ok(())
        } else {
            Err(SignatureVerificationFailure::SignatureMismatch)
        }
    }
}

fn non_empty_env<'a>(env: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    env.get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
}

fn single_receipt_too_large() -> VerifyCliError {
    invalid_args(format!(
        "--receipt input exceeds {SINGLE_RECEIPT_MAX_BYTES} bytes"
    ))
}

fn invalid_args(message: impl Into<String>) -> VerifyCliError {
    VerifyCliError::InvalidArgs(message.into())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;

    use super::*;
    use runx_contracts::ReceiptIssuerType;
    use runx_runtime::receipts::step_receipt_with_signature_policy;
    use runx_runtime::{
        Ed25519ReceiptSigner, InvocationStatus, LocalReceiptStore, RuntimeError, SkillOutput,
    };
    use serde::Deserialize;
    use serde_json as test_json;

    type JsonValue = test_json::Value;

    const CORPUS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/receipt-verify");
    const FIXTURE_KID: &str = "runx-cli-verify-fixture-key";
    const FIXTURE_SEED: [u8; 32] = [0x47; 32];

    #[derive(Debug, Deserialize)]
    struct CorpusVerifier {
        kid: String,
        public_key_base64: String,
    }

    #[derive(Debug, Deserialize)]
    struct CorpusCase {
        name: String,
        receipt: String,
        expected: String,
        signature_mode: String,
    }

    #[test]
    fn verifies_production_signed_receipt_store() -> Result<(), io::Error> {
        let temp = tempfile_dir()?;
        let receipt_dir = temp.join("receipts");
        let signer = fixture_signer().map_err(io::Error::other)?;
        let verifier = Ed25519ReceiptVerifier::new([signer.production_key()]);
        let receipt = production_signed_receipt(&signer)
            .map_err(|error| io::Error::other(error.to_string()))?;
        LocalReceiptStore::new(&receipt_dir)
            .write_receipt_with_policy(
                &receipt,
                RuntimeReceiptSignaturePolicy::production(&verifier),
            )
            .map_err(|error| io::Error::other(error.to_string()))?;

        let env = verifier_env(&signer);
        let result = run_verify_command(
            &[
                "verify".into(),
                "--receipt-dir".into(),
                receipt_dir.clone().into_os_string(),
                "--json".into(),
            ],
            &env,
            &temp,
        )
        .map_err(|error| io::Error::other(error.to_string()))?;

        assert!(
            !result.failed,
            "expected clean verification: {}",
            result.output
        );
        let report: JsonValue = serde_json::from_str(&result.output).map_err(io::Error::other)?;
        assert_eq!(report["valid"], JsonValue::Bool(true));
        assert_eq!(report["signature_mode"], "production");
        Ok(())
    }

    #[test]
    fn flags_tampered_receipt_body() -> Result<(), io::Error> {
        let temp = tempfile_dir()?;
        let receipt_dir = temp.join("receipts");
        let signer = fixture_signer().map_err(io::Error::other)?;
        let verifier = Ed25519ReceiptVerifier::new([signer.production_key()]);
        let receipt = production_signed_receipt(&signer)
            .map_err(|error| io::Error::other(error.to_string()))?;
        LocalReceiptStore::new(&receipt_dir)
            .write_receipt_with_policy(
                &receipt,
                RuntimeReceiptSignaturePolicy::production(&verifier),
            )
            .map_err(|error| io::Error::other(error.to_string()))?;

        // Tamper with the sealed body after signing.
        let receipt_file = receipt_dir.join(format!("{}.json", receipt.id));
        let tampered = fs::read_to_string(&receipt_file)?
            .replace("production-verified", "production-tampered");
        fs::write(&receipt_file, tampered)?;

        let env = verifier_env(&signer);
        let result = run_verify_command(
            &[
                "verify".into(),
                "--receipt-dir".into(),
                receipt_dir.into_os_string(),
            ],
            &env,
            &temp,
        )
        .map_err(|error| io::Error::other(error.to_string()))?;

        assert!(
            result.failed,
            "tampered receipt must fail: {}",
            result.output
        );
        assert!(result.output.contains("verification: FAILED"));
        Ok(())
    }

    #[test]
    fn missing_receipt_id_is_a_usage_error() -> Result<(), io::Error> {
        let temp = tempfile_dir()?;
        let receipt_dir = temp.join("receipts");
        fs::create_dir_all(&receipt_dir)?;
        let error = match run_verify_command(
            &[
                "verify".into(),
                "receipt_missing".into(),
                "--receipt-dir".into(),
                receipt_dir.into_os_string(),
            ],
            &BTreeMap::new(),
            &temp,
        ) {
            Ok(result) => {
                return Err(io::Error::other(format!(
                    "unknown receipt id must error: {}",
                    result.output
                )));
            }
            Err(error) => error,
        };
        assert!(matches!(error, VerifyCliError::InvalidArgs(_)));
        Ok(())
    }

    #[test]
    fn verifies_single_receipt_file_as_machine_verdict() -> Result<(), io::Error> {
        let temp = tempfile_dir()?;
        let signer = fixture_signer().map_err(io::Error::other)?;
        let receipt = production_signed_receipt(&signer)
            .map_err(|error| io::Error::other(error.to_string()))?;
        let receipt_file = temp.join("receipt.json");
        fs::write(
            &receipt_file,
            serde_json::to_vec_pretty(&receipt).map_err(io::Error::other)?,
        )?;

        let result = run_verify_command(
            &[
                "verify".into(),
                "--receipt".into(),
                receipt_file.into_os_string(),
                "--json".into(),
            ],
            &verifier_env(&signer),
            &temp,
        )
        .map_err(|error| io::Error::other(error.to_string()))?;

        assert!(
            !result.failed,
            "expected clean single receipt verdict: {}",
            result.output
        );
        let verdict: JsonValue = serde_json::from_str(&result.output).map_err(io::Error::other)?;
        assert_eq!(verdict["schema"], "runx.verify_verdict.v1");
        assert_eq!(verdict["valid"], JsonValue::Bool(true));
        assert_eq!(
            verdict["receipt_id"],
            JsonValue::String(receipt.id.to_string())
        );
        assert_eq!(verdict["signature"]["mode"], "production");
        assert_eq!(verdict["signature"]["status"], "valid");
        assert_eq!(verdict["digest"]["status"], "valid");
        assert_eq!(verdict["content_address"]["status"], "valid");
        assert_eq!(verdict["lineage"]["status"], "unverified");
        assert!(verdict["findings"].as_array().is_some_and(Vec::is_empty));
        Ok(())
    }

    #[test]
    fn verifies_single_receipt_from_stdin() -> Result<(), io::Error> {
        let temp = tempfile_dir()?;
        let signer = fixture_signer().map_err(io::Error::other)?;
        let receipt = production_signed_receipt(&signer)
            .map_err(|error| io::Error::other(error.to_string()))?;
        let input = serde_json::to_vec(&receipt).map_err(io::Error::other)?;

        let result = run_verify_command_with_stdin(
            &[
                "verify".into(),
                "--receipt".into(),
                "-".into(),
                "--json".into(),
            ],
            &verifier_env(&signer),
            &temp,
            io::Cursor::new(input),
        )
        .map_err(|error| io::Error::other(error.to_string()))?;

        assert!(!result.failed, "stdin verdict failed: {}", result.output);
        let verdict: JsonValue = serde_json::from_str(&result.output).map_err(io::Error::other)?;
        assert_eq!(
            verdict["receipt_id"],
            JsonValue::String(receipt.id.to_string())
        );
        assert_eq!(verdict["valid"], JsonValue::Bool(true));
        Ok(())
    }

    // rust-style-allow: long-function - malformed receipt regression covers capped stdin, invalid JSON, and machine verdict fields together.
    #[test]
    fn malformed_single_receipt_returns_invalid_verdict() -> Result<(), io::Error> {
        let temp = tempfile_dir()?;

        let result = run_verify_command_with_stdin(
            &[
                "verify".into(),
                "--receipt".into(),
                "-".into(),
                "--json".into(),
            ],
            &BTreeMap::new(),
            &temp,
            io::Cursor::new(br#"{"schema":"runx.receipt.v1","#.to_vec()),
        )
        .map_err(|error| io::Error::other(error.to_string()))?;

        assert!(result.failed, "malformed receipt must fail");
        let verdict: JsonValue = serde_json::from_str(&result.output).map_err(io::Error::other)?;
        assert_eq!(verdict["schema"], "runx.verify_verdict.v1");
        assert_eq!(verdict["valid"], JsonValue::Bool(false));
        assert_eq!(verdict["receipt_id"], JsonValue::Null);
        assert_eq!(verdict["findings"][0]["code"], "receipt_parse_error");
        Ok(())
    }

    #[test]
    fn single_receipt_rejects_store_selection_flags() -> Result<(), io::Error> {
        let temp = tempfile_dir()?;
        let error = match run_verify_command(
            &[
                "verify".into(),
                "receipt_1".into(),
                "--receipt".into(),
                "receipt.json".into(),
            ],
            &BTreeMap::new(),
            &temp,
        ) {
            Ok(result) => {
                return Err(io::Error::other(format!(
                    "--receipt must be mutually exclusive with receipt ids: {}",
                    result.output
                )));
            }
            Err(error) => error,
        };
        assert!(matches!(error, VerifyCliError::InvalidArgs(_)));
        Ok(())
    }

    #[test]
    fn single_receipt_stdin_is_size_capped() -> Result<(), io::Error> {
        let temp = tempfile_dir()?;
        let error = match run_verify_command_with_stdin(
            &["verify".into(), "--receipt".into(), "-".into()],
            &BTreeMap::new(),
            &temp,
            io::Cursor::new(vec![b' '; SINGLE_RECEIPT_MAX_BYTES + 1]),
        ) {
            Ok(result) => {
                return Err(io::Error::other(format!(
                    "oversized stdin must be a usage error: {}",
                    result.output
                )));
            }
            Err(error) => error,
        };
        assert!(matches!(error, VerifyCliError::InvalidArgs(_)));
        Ok(())
    }

    #[test]
    fn receipt_verify_corpus_replays_through_cli_surface() -> Result<(), io::Error> {
        let root = PathBuf::from(CORPUS_ROOT);
        let production_env = corpus_production_env(&root)?;
        for (case_dir, case) in corpus_cases(&root)? {
            let env = if case.signature_mode == "production" {
                production_env.clone()
            } else {
                BTreeMap::new()
            };
            let result = run_verify_command(
                &[
                    "verify".into(),
                    "--receipt".into(),
                    case_dir.join(&case.receipt).into_os_string(),
                    "--json".into(),
                ],
                &env,
                &root,
            )
            .map_err(|error| io::Error::other(error.to_string()))?;
            let actual: JsonValue =
                serde_json::from_str(&result.output).map_err(io::Error::other)?;
            let expected = expected_verdict(&case_dir, &case)?;

            assert_eq!(actual, expected, "corpus case {} drifted", case.name);
            assert_eq!(
                result.failed,
                !expected["valid"].as_bool().unwrap_or(false),
                "corpus case {} had inconsistent exit status",
                case.name
            );
        }
        Ok(())
    }

    fn corpus_production_env(root: &Path) -> Result<BTreeMap<String, String>, io::Error> {
        let verifier: CorpusVerifier =
            serde_json::from_str(&fs::read_to_string(root.join("verifier.json"))?)
                .map_err(io::Error::other)?;
        Ok(BTreeMap::from([
            (RUNX_RECEIPT_VERIFY_KID_ENV.to_owned(), verifier.kid),
            (
                RUNX_RECEIPT_VERIFY_ED25519_PUBLIC_KEY_BASE64_ENV.to_owned(),
                verifier.public_key_base64,
            ),
        ]))
    }

    fn corpus_cases(root: &Path) -> Result<Vec<(PathBuf, CorpusCase)>, io::Error> {
        let mut cases = Vec::new();
        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            let case_path = path.join("case.json");
            if !case_path.exists() {
                continue;
            }
            let case: CorpusCase =
                serde_json::from_str(&fs::read_to_string(case_path)?).map_err(io::Error::other)?;
            cases.push((path, case));
        }
        cases.sort_by(|left, right| left.1.name.cmp(&right.1.name));
        Ok(cases)
    }

    fn expected_verdict(case_dir: &Path, case: &CorpusCase) -> Result<JsonValue, io::Error> {
        serde_json::from_str(&fs::read_to_string(case_dir.join(&case.expected))?)
            .map_err(io::Error::other)
    }

    fn verifier_env(signer: &Ed25519ReceiptSigner) -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                RUNX_RECEIPT_VERIFY_KID_ENV.to_owned(),
                FIXTURE_KID.to_owned(),
            ),
            (
                RUNX_RECEIPT_VERIFY_ED25519_PUBLIC_KEY_BASE64_ENV.to_owned(),
                base64_standard(signer.public_key()),
            ),
        ])
    }

    fn fixture_signer() -> Result<Ed25519ReceiptSigner, runx_runtime::RuntimeReceiptSigningError> {
        Ed25519ReceiptSigner::from_seed(FIXTURE_KID, ReceiptIssuerType::Hosted, &FIXTURE_SEED)
    }

    fn production_signed_receipt(signer: &Ed25519ReceiptSigner) -> Result<Receipt, RuntimeError> {
        let verifier = Ed25519ReceiptVerifier::new([signer.production_key()]);
        let output = SkillOutput {
            status: InvocationStatus::Success,
            stdout:
                r#"{"artifact":{"artifact_id":"artifact_cli_verify","artifact_type":"artifact"}}"#
                    .to_owned(),
            stderr: String::new(),
            exit_code: Some(0),
            duration_ms: 10,
            metadata: BTreeMap::new(),
        };
        step_receipt_with_signature_policy(
            "cli-verify",
            "production-verified",
            1,
            &output,
            "2026-06-10T00:00:00Z",
            RuntimeReceiptSignaturePolicy::production_signing(signer, &verifier),
        )
    }

    fn base64_standard(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
            let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
            let triple = (b0 << 16) | (b1 << 8) | b2;
            output.push(TABLE[(triple >> 18) as usize & 0x3f] as char);
            output.push(TABLE[(triple >> 12) as usize & 0x3f] as char);
            output.push(if chunk.len() > 1 {
                TABLE[(triple >> 6) as usize & 0x3f] as char
            } else {
                '='
            });
            output.push(if chunk.len() > 2 {
                TABLE[triple as usize & 0x3f] as char
            } else {
                '='
            });
        }
        output
    }

    fn tempfile_dir() -> Result<PathBuf, io::Error> {
        let path = std::env::temp_dir().join(format!(
            "runx-cli-verify-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| io::Error::other(error.to_string()))?
                .as_nanos()
        ));
        fs::create_dir_all(&path)?;
        Ok(path)
    }
}
