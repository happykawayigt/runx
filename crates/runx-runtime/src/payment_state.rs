use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PAYMENT_STATE_SCHEMA_VERSION: &str = "runx.payment_state.v1";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentIdempotencyKey {
    pub rail: String,
    pub counterparty: String,
    pub key: String,
}

impl PaymentIdempotencyKey {
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

impl PaymentIdempotencyKey {
    fn index_key(&self) -> String {
        format!("{}\u{1f}{}\u{1f}{}", self.rail, self.counterparty, self.key)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentIdempotencyEntry {
    pub idempotency_key: PaymentIdempotencyKey,
    pub receipt_ref: String,
    pub rail_proof_ref: String,
    pub amount_minor: u64,
    pub currency: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpendCapabilityConsumption {
    pub capability_ref: String,
    pub idempotency_key: PaymentIdempotencyKey,
    pub receipt_ref: Option<String>,
    pub recovery_state: Option<PaymentRecoveryState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentRecoveryState {
    InFlight,
    Sealed,
    Escalated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MockRailMutation {
    pub idempotency_key: PaymentIdempotencyKey,
    pub rail: String,
    pub amount_minor: u64,
    pub currency: String,
    pub counterparty: String,
    pub status: MockRailMutationStatus,
    pub proof_ref: Option<String>,
    pub recovery_state: PaymentRecoveryState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MockRailMutationStatus {
    Partial,
    Fulfilled,
    Escalated,
}

#[derive(Clone, Debug)]
pub struct FileBackedPaymentStateStore {
    path: PathBuf,
    state: PaymentStateDocument,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaymentStateDocument {
    schema_version: String,
    idempotency_entries: BTreeMap<String, PaymentIdempotencyEntry>,
    consumed_spend_capabilities: BTreeMap<String, SpendCapabilityConsumption>,
    mock_rail_mutations: BTreeMap<String, MockRailMutation>,
}

impl Default for PaymentStateDocument {
    fn default() -> Self {
        Self {
            schema_version: PAYMENT_STATE_SCHEMA_VERSION.to_owned(),
            idempotency_entries: BTreeMap::new(),
            consumed_spend_capabilities: BTreeMap::new(),
            mock_rail_mutations: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum PaymentStateError {
    #[error("payment state path {path} has no parent directory")]
    MissingParent { path: PathBuf },
    #[error("failed to read payment state {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse payment state {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to create payment state directory {path}: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write payment state {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize payment state {path}: {source}")]
    Serialize {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported payment state schema version {schema_version}")]
    UnsupportedSchemaVersion { schema_version: String },
    #[error("idempotency key {idempotency_key} was already recorded")]
    IdempotencyAlreadyRecorded { idempotency_key: String },
    #[error("mock rail mutation for idempotency key {idempotency_key} was already recorded")]
    MockRailMutationAlreadyRecorded { idempotency_key: String },
    #[error("spend capability {capability_ref} was already consumed")]
    SpendCapabilityAlreadyConsumed { capability_ref: String },
}

impl FileBackedPaymentStateStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, PaymentStateError> {
        let path = path.into();
        let state = match fs::read_to_string(&path) {
            Ok(contents) => {
                let state: PaymentStateDocument =
                    serde_json::from_str(&contents).map_err(|source| PaymentStateError::Parse {
                        path: path.clone(),
                        source,
                    })?;
                if state.schema_version != PAYMENT_STATE_SCHEMA_VERSION {
                    return Err(PaymentStateError::UnsupportedSchemaVersion {
                        schema_version: state.schema_version,
                    });
                }
                state
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                PaymentStateDocument::default()
            }
            Err(source) => {
                return Err(PaymentStateError::Read {
                    path: path.clone(),
                    source,
                });
            }
        };
        Ok(Self { path, state })
    }

    pub fn lookup_idempotency(
        &self,
        key: &PaymentIdempotencyKey,
    ) -> Option<&PaymentIdempotencyEntry> {
        self.state.idempotency_entries.get(&key.index_key())
    }

    pub fn record_idempotency(
        &mut self,
        entry: PaymentIdempotencyEntry,
    ) -> Result<(), PaymentStateError> {
        let index_key = entry.idempotency_key.index_key();
        if self.state.idempotency_entries.contains_key(&index_key) {
            return Err(PaymentStateError::IdempotencyAlreadyRecorded {
                idempotency_key: index_key,
            });
        }
        self.state.idempotency_entries.insert(index_key, entry);
        self.persist()
    }

    pub fn lookup_consumed_spend_capability(
        &self,
        capability_ref: &str,
    ) -> Option<&SpendCapabilityConsumption> {
        self.state.consumed_spend_capabilities.get(capability_ref)
    }

    pub fn consume_spend_capability(
        &mut self,
        consumption: SpendCapabilityConsumption,
    ) -> Result<(), PaymentStateError> {
        if self
            .state
            .consumed_spend_capabilities
            .contains_key(&consumption.capability_ref)
        {
            return Err(PaymentStateError::SpendCapabilityAlreadyConsumed {
                capability_ref: consumption.capability_ref,
            });
        }
        self.state
            .consumed_spend_capabilities
            .insert(consumption.capability_ref.clone(), consumption);
        self.persist()
    }

    pub fn lookup_mock_rail_mutation(
        &self,
        key: &PaymentIdempotencyKey,
    ) -> Option<&MockRailMutation> {
        self.state.mock_rail_mutations.get(&key.index_key())
    }

    pub fn record_mock_rail_mutation(
        &mut self,
        mutation: MockRailMutation,
    ) -> Result<(), PaymentStateError> {
        let index_key = mutation.idempotency_key.index_key();
        if self.state.mock_rail_mutations.contains_key(&index_key) {
            return Err(PaymentStateError::MockRailMutationAlreadyRecorded {
                idempotency_key: index_key,
            });
        }
        self.state.mock_rail_mutations.insert(index_key, mutation);
        self.persist()
    }

    fn persist(&self) -> Result<(), PaymentStateError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| PaymentStateError::MissingParent {
                path: self.path.clone(),
            })?;
        fs::create_dir_all(parent).map_err(|source| PaymentStateError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
        write_json_atomically(&self.path, &self.state)
    }
}

fn write_json_atomically<T: Serialize>(path: &Path, value: &T) -> Result<(), PaymentStateError> {
    let parent = path
        .parent()
        .ok_or_else(|| PaymentStateError::MissingParent {
            path: path.to_path_buf(),
        })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("payment-state.json");
    let temp_path = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        monotonicish_nanos()
    ));

    let write_result = (|| {
        let mut file = File::create(&temp_path).map_err(|source| PaymentStateError::Write {
            path: temp_path.clone(),
            source,
        })?;
        serde_json::to_writer_pretty(&mut file, value).map_err(|source| {
            PaymentStateError::Serialize {
                path: temp_path.clone(),
                source,
            }
        })?;
        file.write_all(b"\n")
            .map_err(|source| PaymentStateError::Write {
                path: temp_path.clone(),
                source,
            })?;
        file.sync_all().map_err(|source| PaymentStateError::Write {
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
        PaymentStateError::Write {
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
