// rust-style-allow: large-file because effect state owns persisted idempotency,
// capability consumption, mutation recovery, and the step-persistence transaction
// at the runtime trust boundary.
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use runx_contracts::{JsonObject, JsonValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::payment::packets::{PaymentPacketError, PaymentRailPacket, read_payment_rail_packet};
use crate::payment::supervisor::{
    PaymentSupervisorProof, PaymentSupervisorProofMatch, validate_payment_supervisor_proof,
};

pub const EFFECT_STATE_SCHEMA_VERSION: &str = "runx.effect_state.v1";
pub const RUNX_EFFECT_STATE_PATH_ENV: &str = "RUNX_EFFECT_STATE_PATH";
const EFFECT_STATE_LOCK_TIMEOUT: Duration = Duration::from_secs(5);
const EFFECT_STATE_LOCK_RETRY: Duration = Duration::from_millis(10);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectIdempotencyKey {
    pub rail: String,
    pub counterparty: String,
    pub key: String,
}

impl EffectIdempotencyKey {
    pub fn new(
        rail: impl Into<String>,
        counterparty: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        Self {
            rail: rail.into(),
            counterparty: counterparty.into(),
            key: key.into(),
        }
    }
}

impl EffectIdempotencyKey {
    fn index_key(&self) -> String {
        format!("{}\u{1f}{}\u{1f}{}", self.rail, self.counterparty, self.key)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectIdempotencyEntry {
    pub idempotency_key: EffectIdempotencyKey,
    pub receipt_ref: String,
    pub receipt_created_at: String,
    pub receipt_digest: String,
    pub rail_proof_ref: String,
    pub supervisor_proof: PaymentSupervisorProof,
    pub amount_minor: u64,
    pub currency: String,
    pub outputs: JsonObject,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectCapabilityConsumption {
    pub capability_ref: String,
    pub idempotency_key: EffectIdempotencyKey,
    pub receipt_ref: Option<String>,
    pub recovery_state: Option<EffectRecoveryState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectRecoveryState {
    InFlight,
    Sealed,
    Escalated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectMutation {
    pub idempotency_key: EffectIdempotencyKey,
    pub rail: String,
    pub amount_minor: u64,
    pub currency: String,
    pub counterparty: String,
    pub status: EffectMutationStatus,
    pub proof_ref: Option<String>,
    pub recovery_state: EffectRecoveryState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectMutationStatus {
    Partial,
    Fulfilled,
    Escalated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectStepStateInput {
    pub family: &'static str,
    pub idempotency_key: EffectIdempotencyKey,
    pub spend_capability_ref: String,
    pub rail: String,
    pub counterparty: String,
    pub amount_minor: u64,
    pub currency: String,
    pub act_id: String,
}

#[derive(Debug)]
pub struct FileBackedEffectStateStore {
    path: PathBuf,
    state: EffectStateDocument,
}

#[derive(Debug)]
struct EffectStateLock {
    path: PathBuf,
    _file: File,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectStateDocument {
    schema_version: String,
    families: BTreeMap<String, EffectFamilyState>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectFamilyState {
    idempotency_entries: BTreeMap<String, EffectIdempotencyEntry>,
    consumed_spend_capabilities: BTreeMap<String, EffectCapabilityConsumption>,
    rail_mutations: BTreeMap<String, EffectMutation>,
}

impl Default for EffectStateDocument {
    fn default() -> Self {
        Self {
            schema_version: EFFECT_STATE_SCHEMA_VERSION.to_owned(),
            families: BTreeMap::new(),
        }
    }
}

impl EffectStateDocument {
    fn family(&self, family: &str) -> Option<&EffectFamilyState> {
        self.families.get(family)
    }

    fn family_mut(&mut self, family: &'static str) -> &mut EffectFamilyState {
        self.families.entry(family.to_owned()).or_default()
    }
}

#[derive(Debug, Error)]
pub enum EffectStateError {
    #[error("effect state path {path} has no parent directory")]
    MissingParent { path: PathBuf },
    #[error("failed to read effect state {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse effect state {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("effect state {path} has unsupported schema version {version}")]
    UnsupportedSchemaVersion { path: PathBuf, version: String },
    #[error("failed to create effect state directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write effect state {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to lock effect state {path}: {message}")]
    Lock { path: PathBuf, message: String },
    #[error("failed to serialize effect state {path}: {source}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("idempotency key {idempotency_key} was already recorded")]
    IdempotencyAlreadyRecorded { idempotency_key: String },
    #[error("rail mutation for idempotency key {idempotency_key} was already recorded")]
    EffectMutationAlreadyRecorded { idempotency_key: String },
    #[error("spend capability {capability_ref} was already consumed")]
    SpendCapabilityAlreadyConsumed { capability_ref: String },
    #[error("failed to serialize replay-safe payment outputs: {source}")]
    ReplayOutputSerialize {
        #[source]
        source: serde_json::Error,
    },
    #[error("payment supervisor proof is required before sealing rail proof {proof_ref}")]
    MissingSupervisorProof { proof_ref: String },
    #[error("payment supervisor proof mismatch: {message}")]
    SupervisorProof { message: String },
    #[error(transparent)]
    PaymentPacket(#[from] PaymentPacketError),
}

impl FileBackedEffectStateStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, EffectStateError> {
        let path = path.into();
        let state = load_effect_state(&path)?;
        Ok(Self { path, state })
    }

    pub fn lookup_idempotency(
        &self,
        family: &str,
        key: &EffectIdempotencyKey,
    ) -> Option<&EffectIdempotencyEntry> {
        self.state
            .family(family)
            .and_then(|state| state.idempotency_entries.get(&key.index_key()))
    }

    pub fn record_idempotency(
        &mut self,
        family: &'static str,
        entry: EffectIdempotencyEntry,
    ) -> Result<(), EffectStateError> {
        let index_key = entry.idempotency_key.index_key();
        if self
            .state
            .family(family)
            .is_some_and(|state| state.idempotency_entries.contains_key(&index_key))
        {
            return Err(EffectStateError::IdempotencyAlreadyRecorded {
                idempotency_key: index_key,
            });
        }
        self.with_locked_state(|state| {
            let state = state.family_mut(family);
            if state.idempotency_entries.contains_key(&index_key) {
                return Err(EffectStateError::IdempotencyAlreadyRecorded {
                    idempotency_key: index_key.clone(),
                });
            }
            state.idempotency_entries.insert(index_key, entry);
            Ok(())
        })
    }

    pub fn lookup_consumed_spend_capability(
        &self,
        family: &str,
        capability_ref: &str,
    ) -> Option<&EffectCapabilityConsumption> {
        self.state
            .family(family)
            .and_then(|state| state.consumed_spend_capabilities.get(capability_ref))
    }

    pub fn consume_spend_capability(
        &mut self,
        family: &'static str,
        consumption: EffectCapabilityConsumption,
    ) -> Result<(), EffectStateError> {
        let capability_ref = consumption.capability_ref.clone();
        if self.state.family(family).is_some_and(|state| {
            state
                .consumed_spend_capabilities
                .contains_key(&capability_ref)
        }) {
            return Err(EffectStateError::SpendCapabilityAlreadyConsumed { capability_ref });
        }
        self.with_locked_state(|state| {
            let state = state.family_mut(family);
            if state
                .consumed_spend_capabilities
                .contains_key(&capability_ref)
            {
                return Err(EffectStateError::SpendCapabilityAlreadyConsumed {
                    capability_ref: capability_ref.clone(),
                });
            }
            state
                .consumed_spend_capabilities
                .insert(capability_ref, consumption);
            Ok(())
        })
    }

    pub fn lookup_mutation(
        &self,
        family: &str,
        key: &EffectIdempotencyKey,
    ) -> Option<&EffectMutation> {
        self.state
            .family(family)
            .and_then(|state| state.rail_mutations.get(&key.index_key()))
    }

    pub fn escalate_mutation(
        &mut self,
        family: &'static str,
        key: &EffectIdempotencyKey,
    ) -> Result<Option<EffectMutation>, EffectStateError> {
        if !self
            .state
            .family(family)
            .is_some_and(|state| state.rail_mutations.contains_key(&key.index_key()))
        {
            return Ok(None);
        }
        self.with_locked_state(|state| {
            let state = state.family_mut(family);
            let Some(mutation) = state.rail_mutations.get_mut(&key.index_key()) else {
                return Ok(None);
            };
            mutation.status = EffectMutationStatus::Escalated;
            mutation.recovery_state = EffectRecoveryState::Escalated;
            Ok(Some(mutation.clone()))
        })
    }

    pub fn record_mutation(
        &mut self,
        family: &'static str,
        mutation: EffectMutation,
    ) -> Result<(), EffectStateError> {
        let index_key = mutation.idempotency_key.index_key();
        if self
            .state
            .family(family)
            .is_some_and(|state| state.rail_mutations.contains_key(&index_key))
        {
            return Err(EffectStateError::EffectMutationAlreadyRecorded {
                idempotency_key: index_key,
            });
        }
        self.with_locked_state(|state| {
            let state = state.family_mut(family);
            if state.rail_mutations.contains_key(&index_key) {
                return Err(EffectStateError::EffectMutationAlreadyRecorded {
                    idempotency_key: index_key.clone(),
                });
            }
            state.rail_mutations.insert(index_key, mutation);
            Ok(())
        })
    }

    fn with_locked_state<T>(
        &mut self,
        update: impl FnOnce(&mut EffectStateDocument) -> Result<T, EffectStateError>,
    ) -> Result<T, EffectStateError> {
        let _lock = EffectStateLock::acquire(&self.path)?;
        let mut state = load_effect_state(&self.path)?;
        let result = update(&mut state)?;
        persist_effect_state(&self.path, &state)?;
        self.state = state;
        Ok(result)
    }
}

impl EffectStateLock {
    fn acquire(path: &Path) -> Result<Self, EffectStateError> {
        let parent = path
            .parent()
            .ok_or_else(|| EffectStateError::MissingParent {
                path: path.to_path_buf(),
            })?;
        fs::create_dir_all(parent).map_err(|source| EffectStateError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
        let lock_path = effect_state_lock_path(path);
        let started = Instant::now();
        loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(file) => {
                    return Ok(Self {
                        path: lock_path,
                        _file: file,
                    });
                }
                Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
                    if started.elapsed() >= EFFECT_STATE_LOCK_TIMEOUT {
                        return Err(EffectStateError::Lock {
                            path: path.to_path_buf(),
                            message: format!("timed out waiting for lock {}", lock_path.display()),
                        });
                    }
                    thread::sleep(EFFECT_STATE_LOCK_RETRY);
                }
                Err(source) => {
                    return Err(EffectStateError::Lock {
                        path: path.to_path_buf(),
                        message: source.to_string(),
                    });
                }
            }
        }
    }
}

impl Drop for EffectStateLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn load_effect_state(path: &Path) -> Result<EffectStateDocument, EffectStateError> {
    match fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|source| EffectStateError::Parse {
                path: path.to_path_buf(),
                source,
            })
            .and_then(|state: EffectStateDocument| {
                if state.schema_version == EFFECT_STATE_SCHEMA_VERSION {
                    Ok(state)
                } else {
                    Err(EffectStateError::UnsupportedSchemaVersion {
                        path: path.to_path_buf(),
                        version: state.schema_version,
                    })
                }
            }),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(EffectStateDocument::default())
        }
        Err(source) => Err(EffectStateError::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn persist_effect_state(path: &Path, state: &EffectStateDocument) -> Result<(), EffectStateError> {
    let parent = path
        .parent()
        .ok_or_else(|| EffectStateError::MissingParent {
            path: path.to_path_buf(),
        })?;
    fs::create_dir_all(parent).map_err(|source| EffectStateError::CreateDirectory {
        path: parent.to_path_buf(),
        source,
    })?;
    write_json_atomically(path, state)
}

fn effect_state_lock_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("effect-state.json");
    path.with_file_name(format!(".{file_name}.lock"))
}

pub fn consumed_spend_capability_recorded(
    env: &BTreeMap<String, String>,
    cwd: &Path,
    family: &'static str,
    capability_ref: &str,
) -> Result<bool, EffectStateError> {
    let Some(path) = resolve_effect_state_path(env, cwd) else {
        return Ok(false);
    };
    let store = FileBackedEffectStateStore::open(&path)?;
    Ok(store
        .lookup_consumed_spend_capability(family, capability_ref)
        .is_some())
}

pub fn lookup_effect_idempotency_entry(
    env: &BTreeMap<String, String>,
    cwd: &Path,
    family: &'static str,
    key: &EffectIdempotencyKey,
) -> Result<Option<EffectIdempotencyEntry>, EffectStateError> {
    let Some(path) = resolve_effect_state_path(env, cwd) else {
        return Ok(None);
    };
    let store = FileBackedEffectStateStore::open(&path)?;
    Ok(store.lookup_idempotency(family, key).cloned())
}

pub fn lookup_effect_mutation(
    env: &BTreeMap<String, String>,
    cwd: &Path,
    family: &'static str,
    key: &EffectIdempotencyKey,
) -> Result<Option<EffectMutation>, EffectStateError> {
    let Some(path) = resolve_effect_state_path(env, cwd) else {
        return Ok(None);
    };
    let store = FileBackedEffectStateStore::open(&path)?;
    Ok(store.lookup_mutation(family, key).cloned())
}

pub fn escalate_effect_mutation(
    env: &BTreeMap<String, String>,
    cwd: &Path,
    family: &'static str,
    key: &EffectIdempotencyKey,
) -> Result<Option<EffectMutation>, EffectStateError> {
    let Some(path) = resolve_effect_state_path(env, cwd) else {
        return Ok(None);
    };
    let mut store = FileBackedEffectStateStore::open(&path)?;
    store.escalate_mutation(family, key)
}

// rust-style-allow: long-function because effect state persistence binds
// authority, output, receipt, and recovery-state invariants in one transaction.
pub fn persist_effect_step_state(
    env: &BTreeMap<String, String>,
    cwd: &Path,
    input: &EffectStepStateInput,
    outputs: &JsonObject,
    receipt: &runx_contracts::Receipt,
    supervisor_proof: Option<&PaymentSupervisorProof>,
) -> Result<(), EffectStateError> {
    let Some(path) = resolve_effect_state_path(env, cwd) else {
        return Ok(());
    };
    let rail_packet = read_payment_rail_packet(outputs)?;
    let recovery_state = payment_recovery_state(rail_packet.as_ref());
    let rail_touched = rail_packet
        .as_ref()
        .and_then(|packet| packet.result.as_ref())
        .and_then(|result| result.status.as_deref())
        .is_some();

    let mut store = FileBackedEffectStateStore::open(&path)?;

    if rail_touched
        && store
            .lookup_consumed_spend_capability(input.family, &input.spend_capability_ref)
            .is_none()
    {
        store.consume_spend_capability(
            input.family,
            EffectCapabilityConsumption {
                capability_ref: input.spend_capability_ref.clone(),
                idempotency_key: input.idempotency_key.clone(),
                receipt_ref: Some(receipt.id.to_string()),
                recovery_state: Some(recovery_state.clone()),
            },
        )?;
    }

    let proof_ref = rail_packet
        .as_ref()
        .and_then(|packet| packet.proof.as_ref())
        .map(|proof| proof.proof_ref.as_str());

    if let Some(proof_ref) = proof_ref
        && store
            .lookup_idempotency(input.family, &input.idempotency_key)
            .is_none()
    {
        // Validate the supervisor proof only when sealing a new record. A
        // duplicate persist for an already-sealed idempotency key keeps the
        // first record; the sealed-replay path is the guard against forged
        // replays of an existing key.
        let supervisor_proof =
            validate_sealed_supervisor_proof(input, receipt, proof_ref, supervisor_proof)?;
        let result = rail_packet
            .as_ref()
            .and_then(|packet| packet.result.as_ref());
        store.record_idempotency(
            input.family,
            EffectIdempotencyEntry {
                idempotency_key: input.idempotency_key.clone(),
                receipt_ref: receipt.id.to_string(),
                receipt_created_at: receipt.created_at.to_string(),
                receipt_digest: receipt.digest.to_string(),
                rail_proof_ref: proof_ref.to_owned(),
                supervisor_proof: supervisor_proof.clone(),
                amount_minor: result
                    .and_then(|result| result.amount_minor)
                    .unwrap_or(input.amount_minor),
                currency: result
                    .and_then(|result| result.currency.as_deref())
                    .unwrap_or(&input.currency)
                    .to_owned(),
                outputs: replay_safe_outputs(outputs)?,
            },
        )?;
    }

    if rail_touched
        && store
            .lookup_mutation(input.family, &input.idempotency_key)
            .is_none()
    {
        let result = rail_packet
            .as_ref()
            .and_then(|packet| packet.result.as_ref());
        store.record_mutation(
            input.family,
            EffectMutation {
                idempotency_key: input.idempotency_key.clone(),
                rail: result
                    .and_then(|result| result.rail.as_deref())
                    .unwrap_or(&input.rail)
                    .to_owned(),
                amount_minor: result
                    .and_then(|result| result.amount_minor)
                    .unwrap_or(input.amount_minor),
                currency: result
                    .and_then(|result| result.currency.as_deref())
                    .unwrap_or(&input.currency)
                    .to_owned(),
                counterparty: result
                    .and_then(|result| result.counterparty.as_deref())
                    .unwrap_or(&input.counterparty)
                    .to_owned(),
                status: rail_mutation_status(&recovery_state),
                proof_ref: proof_ref.map(str::to_owned),
                recovery_state,
            },
        )?;
    }

    Ok(())
}

fn validate_sealed_supervisor_proof<'a>(
    input: &EffectStepStateInput,
    receipt: &runx_contracts::Receipt,
    proof_ref: &str,
    supervisor_proof: Option<&'a PaymentSupervisorProof>,
) -> Result<&'a PaymentSupervisorProof, EffectStateError> {
    let proof = supervisor_proof.ok_or_else(|| EffectStateError::MissingSupervisorProof {
        proof_ref: proof_ref.to_owned(),
    })?;
    validate_payment_supervisor_proof(
        proof,
        PaymentSupervisorProofMatch {
            proof_ref,
            rail: &input.rail,
            counterparty: &input.counterparty,
            amount_minor: input.amount_minor,
            currency: &input.currency,
            idempotency_key: &input.idempotency_key.key,
            spend_capability_ref: &input.spend_capability_ref,
            act_id: &input.act_id,
            receipt_ref: &receipt.id,
            receipt_digest: &receipt.digest,
        },
    )
    .map_err(|source| EffectStateError::SupervisorProof {
        message: source.to_string(),
    })?;
    Ok(proof)
}

fn replay_safe_outputs(outputs: &JsonObject) -> Result<JsonObject, EffectStateError> {
    let mut safe_outputs = outputs.clone();
    sanitize_replay_payload(&mut safe_outputs);

    let mut stdout_payload = safe_outputs.clone();
    stdout_payload.remove("stdout");
    stdout_payload.remove("stderr");
    stdout_payload.remove("status");
    sanitize_replay_payload(&mut stdout_payload);

    let stdout = serde_json::to_string(&JsonValue::Object(stdout_payload))
        .map_err(|source| EffectStateError::ReplayOutputSerialize { source })?;
    safe_outputs.insert("stdout".to_owned(), JsonValue::String(stdout));
    safe_outputs
        .entry("stderr".to_owned())
        .or_insert_with(|| JsonValue::String(String::new()));
    safe_outputs
        .entry("status".to_owned())
        .or_insert_with(|| JsonValue::String("success".to_owned()));
    Ok(safe_outputs)
}

fn sanitize_replay_payload(payload: &mut JsonObject) {
    let Some(JsonValue::Object(packet)) = payload.get_mut("payment_rail_packet") else {
        return;
    };
    let Some(JsonValue::Object(data)) = packet.get_mut("data") else {
        return;
    };
    if let Some(JsonValue::Object(proof)) = data.get_mut("rail_proof") {
        proof.remove("rail_session_material_ref");
    }
}

fn payment_recovery_state(packet: Option<&PaymentRailPacket>) -> EffectRecoveryState {
    match packet {
        Some(PaymentRailPacket {
            recovery_status: Some(status),
            ..
        }) if status == "sealed" => EffectRecoveryState::Sealed,
        Some(PaymentRailPacket {
            recovery_status: Some(status),
            ..
        }) if status == "terminal_decline" || status == "escalated" => {
            EffectRecoveryState::Escalated
        }
        Some(PaymentRailPacket {
            recovery_status: Some(status),
            ..
        }) if status == "recoverable_timeout" || status == "partial" || status == "in_flight" => {
            EffectRecoveryState::InFlight
        }
        Some(PaymentRailPacket { proof: Some(_), .. }) => EffectRecoveryState::Sealed,
        _ => EffectRecoveryState::InFlight,
    }
}

fn rail_mutation_status(recovery_state: &EffectRecoveryState) -> EffectMutationStatus {
    match recovery_state {
        EffectRecoveryState::Sealed => EffectMutationStatus::Fulfilled,
        EffectRecoveryState::Escalated => EffectMutationStatus::Escalated,
        EffectRecoveryState::InFlight => EffectMutationStatus::Partial,
    }
}

pub fn resolve_effect_state_path(env: &BTreeMap<String, String>, cwd: &Path) -> Option<PathBuf> {
    env.get(RUNX_EFFECT_STATE_PATH_ENV)
        .filter(|value| !value.trim().is_empty())
        .map(|value| resolve_path(Path::new(value), cwd))
        .or_else(|| {
            env.get(crate::receipts::paths::RUNX_RECEIPT_DIR_ENV)
                .filter(|value| !value.trim().is_empty())
                .map(|value| resolve_path(Path::new(value), cwd).join("effect-state.json"))
        })
}

fn resolve_path(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn write_json_atomically<T: Serialize>(path: &Path, value: &T) -> Result<(), EffectStateError> {
    let parent = path
        .parent()
        .ok_or_else(|| EffectStateError::MissingParent {
            path: path.to_path_buf(),
        })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("effect-state.json");
    let temp_path = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        monotonicish_nanos()
    ));

    let write_result = (|| {
        let mut file = File::create(&temp_path).map_err(|source| EffectStateError::Write {
            path: temp_path.clone(),
            source,
        })?;
        serde_json::to_writer_pretty(&mut file, value).map_err(|source| {
            EffectStateError::Serialize {
                path: temp_path.clone(),
                source,
            }
        })?;
        file.write_all(b"\n")
            .map_err(|source| EffectStateError::Write {
                path: temp_path.clone(),
                source,
            })?;
        file.sync_all().map_err(|source| EffectStateError::Write {
            path: temp_path.clone(),
            source,
        })?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    fs::rename(&temp_path, path).map_err(|source| {
        let _ = fs::remove_file(&temp_path);
        EffectStateError::Write {
            path: path.to_path_buf(),
            source,
        }
    })
}

fn monotonicish_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}
