use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use runx_contracts::schema::NonEmptyString;
use runx_contracts::{
    ContextArtifactMeta, ContextArtifactProducer, ContextEntry, ContextEntryVersion, JsonObject,
    JsonValue, sha256_prefixed,
};

use crate::RuntimeError;
use crate::registry::{RegistryResolveOptions, create_file_registry_store, resolve_registry_skill};

const CONTEXT_ENTRY_TYPE: &str = "runx.skill.context";
const CONTEXT_PRODUCER_SKILL: &str = "runx-runtime";
const CONTEXT_PRODUCER_RUNNER: &str = "skill-context";
const PENDING_RUN_ID: &str = "rx_pending";
const MAX_CONTEXT_SKILLS: usize = 12;
const MAX_CONTEXT_SKILL_BYTES: usize = 64 * 1024;
const MAX_CONTEXT_SKILLS_TOTAL_BYTES: usize = 256 * 1024;

pub(crate) fn load_context_skills(
    step_id: &str,
    graph_dir: &Path,
    refs: &[String],
    env: &BTreeMap<String, String>,
    created_at: &str,
) -> Result<Vec<ContextEntry>, RuntimeError> {
    if refs.len() > MAX_CONTEXT_SKILLS {
        return Err(RuntimeError::InvalidRunStep {
            step_id: step_id.to_owned(),
            reason: format!(
                "context_skills declares {} skills; the maximum is {MAX_CONTEXT_SKILLS}",
                refs.len()
            ),
        });
    }

    let mut seen = BTreeSet::new();
    let mut total_bytes = 0usize;
    refs.iter()
        .map(|reference| {
            if !seen.insert(reference.as_str()) {
                return Err(RuntimeError::InvalidRunStep {
                    step_id: step_id.to_owned(),
                    reason: format!("context skill '{reference}' is declared more than once"),
                });
            }
            let entry = load_context_skill(step_id, graph_dir, reference, env, created_at)?;
            total_bytes += usize::try_from(entry.meta.size_bytes).unwrap_or(usize::MAX);
            if total_bytes > MAX_CONTEXT_SKILLS_TOTAL_BYTES {
                return Err(RuntimeError::InvalidRunStep {
                    step_id: step_id.to_owned(),
                    reason: format!(
                        "context_skills resolved to more than {MAX_CONTEXT_SKILLS_TOTAL_BYTES} bytes"
                    ),
                });
            }
            Ok(entry)
        })
        .collect()
}

fn load_context_skill(
    step_id: &str,
    graph_dir: &Path,
    reference: &str,
    env: &BTreeMap<String, String>,
    created_at: &str,
) -> Result<ContextEntry, RuntimeError> {
    if is_registry_ref(reference) {
        return load_registry_context_skill(step_id, reference, env, created_at);
    }
    load_local_context_skill(step_id, graph_dir, reference, env, created_at)
}

fn load_local_context_skill(
    step_id: &str,
    graph_dir: &Path,
    reference: &str,
    env: &BTreeMap<String, String>,
    created_at: &str,
) -> Result<ContextEntry, RuntimeError> {
    if Path::new(reference).is_absolute() {
        return Err(RuntimeError::InvalidRunStep {
            step_id: step_id.to_owned(),
            reason: format!("context skill '{reference}' must be a relative path or registry ref"),
        });
    }
    let skill_dir = graph_dir.join(reference);
    let skill_path = skill_dir.join("SKILL.md");
    let metadata = fs::metadata(&skill_path)
        .map_err(|source| RuntimeError::io(format!("reading {}", skill_path.display()), source))?;
    validate_context_skill_size(
        step_id,
        reference,
        usize::try_from(metadata.len()).unwrap_or(usize::MAX),
    )?;
    let markdown = fs::read_to_string(&skill_path)
        .map_err(|source| RuntimeError::io(format!("reading {}", skill_path.display()), source))?;
    let raw = runx_parser::parse_skill_markdown(&markdown)?;
    let skill = runx_parser::validate_skill(raw).map_err(RuntimeError::from)?;
    let digest = sha256_prefixed(markdown.as_bytes());
    let mut data = JsonObject::new();
    insert_string(&mut data, "ref", reference);
    insert_string(&mut data, "source", "local-path");
    insert_string(&mut data, "content_kind", "skill-markdown");
    insert_string(&mut data, "security_boundary", "untrusted-agent-context");
    insert_string(&mut data, "name", &skill.name);
    let skill_path_display = skill_path.to_string_lossy();
    insert_string(&mut data, "path", skill_path_display.as_ref());
    insert_string(&mut data, "sha256", &digest);
    insert_string(&mut data, "content", &markdown);
    skill_context_entry(SkillContextEntryInput {
        step_id,
        reference,
        env,
        created_at,
        digest: &digest,
        size_bytes: markdown.len() as u64,
        data,
    })
}

fn load_registry_context_skill(
    step_id: &str,
    reference: &str,
    env: &BTreeMap<String, String>,
    created_at: &str,
) -> Result<ContextEntry, RuntimeError> {
    let Some(registry_dir) = env.get("RUNX_REGISTRY_DIR") else {
        return Err(RuntimeError::InvalidRunStep {
            step_id: step_id.to_owned(),
            reason: format!(
                "context skill '{reference}' is a registry ref, but RUNX_REGISTRY_DIR is not configured"
            ),
        });
    };
    let store = create_file_registry_store(registry_dir);
    let registry_url = env.get("RUNX_REGISTRY_URL").cloned();
    let resolution = resolve_registry_skill(
        &store,
        reference,
        RegistryResolveOptions {
            version: None,
            registry_url,
        },
    )
    .map_err(|error| RuntimeError::InvalidRunStep {
        step_id: step_id.to_owned(),
        reason: format!("context skill registry ref '{reference}' could not be resolved: {error}"),
    })?
    .ok_or_else(|| RuntimeError::InvalidRunStep {
        step_id: step_id.to_owned(),
        reason: format!("context skill registry ref '{reference}' was not found"),
    })?;

    let digest = prefixed_digest(&resolution.digest);
    validate_context_skill_size(step_id, reference, resolution.markdown.len())?;
    let mut data = JsonObject::new();
    insert_string(&mut data, "ref", reference);
    insert_string(&mut data, "source", &resolution.source);
    insert_string(&mut data, "content_kind", "skill-markdown");
    insert_string(&mut data, "security_boundary", "untrusted-agent-context");
    insert_string(&mut data, "source_label", &resolution.source_label);
    insert_string(&mut data, "skill_id", &resolution.skill_id);
    insert_string(&mut data, "name", &resolution.name);
    insert_string(&mut data, "version", &resolution.version);
    insert_string(&mut data, "sha256", &digest);
    insert_string(&mut data, "content", &resolution.markdown);
    skill_context_entry(SkillContextEntryInput {
        step_id,
        reference,
        env,
        created_at,
        digest: &digest,
        size_bytes: resolution.markdown.len() as u64,
        data,
    })
}

struct SkillContextEntryInput<'a> {
    step_id: &'a str,
    reference: &'a str,
    env: &'a BTreeMap<String, String>,
    created_at: &'a str,
    digest: &'a str,
    size_bytes: u64,
    data: JsonObject,
}

fn skill_context_entry(input: SkillContextEntryInput<'_>) -> Result<ContextEntry, RuntimeError> {
    let artifact_id = sha256_prefixed(
        format!(
            "{CONTEXT_ENTRY_TYPE}\0{}\0{}",
            input.reference, input.digest
        )
        .as_bytes(),
    );
    Ok(ContextEntry {
        entry_type: Some(non_empty(CONTEXT_ENTRY_TYPE)?),
        version: ContextEntryVersion::V1,
        data: input.data,
        meta: ContextArtifactMeta {
            artifact_id: non_empty(artifact_id)?,
            run_id: non_empty(
                input
                    .env
                    .get(crate::execution::runner::RUNX_RUN_ID_ENV)
                    .map(String::as_str)
                    .unwrap_or(PENDING_RUN_ID),
            )?,
            step_id: Some(non_empty(input.step_id)?),
            producer: ContextArtifactProducer {
                skill: non_empty(CONTEXT_PRODUCER_SKILL)?,
                runner: non_empty(CONTEXT_PRODUCER_RUNNER)?,
            },
            created_at: non_empty(input.created_at)?,
            hash: non_empty(input.digest)?,
            size_bytes: input.size_bytes,
            parent_artifact_id: None,
            receipt_id: None,
            redacted: false,
        },
    })
}

fn is_registry_ref(reference: &str) -> bool {
    reference.starts_with("registry:")
        || reference.starts_with("runx-registry:")
        || reference.starts_with("runx://skill/")
}

fn validate_context_skill_size(
    step_id: &str,
    reference: &str,
    size_bytes: usize,
) -> Result<(), RuntimeError> {
    if size_bytes <= MAX_CONTEXT_SKILL_BYTES {
        return Ok(());
    }
    Err(RuntimeError::InvalidRunStep {
        step_id: step_id.to_owned(),
        reason: format!(
            "context skill '{reference}' is {size_bytes} bytes; the maximum is {MAX_CONTEXT_SKILL_BYTES}"
        ),
    })
}

fn prefixed_digest(digest: &str) -> String {
    if digest.starts_with("sha256:") {
        digest.to_owned()
    } else {
        format!("sha256:{digest}")
    }
}

fn non_empty(value: impl Into<String>) -> Result<NonEmptyString, RuntimeError> {
    NonEmptyString::new(value.into()).ok_or_else(|| RuntimeError::ReceiptInvalid {
        message: "skill context artifact included an empty required field".to_owned(),
    })
}

fn insert_string(object: &mut JsonObject, key: &str, value: &str) {
    object.insert(key.to_owned(), JsonValue::String(value.to_owned()));
}
