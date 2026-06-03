// rust-style-allow: large-file because tool-manifest build keeps source/schema
// hashing, raw payload normalization, output binding shape, and stable JSON
// emission together so the TS doctor and the rust runtime agree byte-for-byte.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use runx_contracts::sha256_prefixed;
use runx_contracts::tools::{
    BuiltToolItem, JsonPayload, JsonPayloadObject, RuntimeCommand, ToolBuildReport,
    ToolBuildReportSchema, ToolBuildStatus, ToolManifest, ToolManifestSchema, ToolOutput,
};
use serde::Deserialize;

use super::error::ToolCatalogError;
use super::hash::sha256_stable;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolBuildOptions {
    pub root: PathBuf,
    pub tool_path: Option<PathBuf>,
    pub all: bool,
    pub toolkit_version: String,
}

#[derive(Deserialize)]
struct RawToolManifest {
    #[serde(default)]
    name: String,
    version: Option<String>,
    description: Option<String>,
    source: runx_contracts::tools::ToolSource,
    #[serde(default)]
    inputs: BTreeMap<String, runx_contracts::tools::ToolInput>,
    output: Option<ToolOutput>,
    #[serde(default)]
    scopes: Vec<String>,
    risk: Option<JsonPayload>,
    runtime: Option<RuntimeCommand>,
    retry: Option<runx_contracts::tools::ToolRetryPolicy>,
    idempotency: Option<runx_contracts::tools::ToolIdempotencyPolicy>,
    mutating: Option<bool>,
    runx: Option<JsonPayloadObject>,
}

pub fn build_tool_catalogs(
    options: &ToolBuildOptions,
) -> Result<ToolBuildReport, ToolCatalogError> {
    let tool_dirs = if options.all {
        discover_tool_directories(&options.root)?
    } else {
        vec![resolve_tool_path(
            &options.root,
            options.tool_path.as_deref(),
        )?]
    };
    let mut built = Vec::new();
    let mut errors = Vec::new();
    for tool_dir in tool_dirs {
        match build_tool_manifest(&options.root, &tool_dir, &options.toolkit_version) {
            Ok(item) => built.push(item),
            Err(error) => errors.push(format!(
                "{}: {}",
                project_path(&options.root, &tool_dir),
                error.concise_message()
            )),
        }
    }
    Ok(ToolBuildReport {
        schema: ToolBuildReportSchema::V1,
        status: if errors.is_empty() {
            ToolBuildStatus::Success
        } else {
            ToolBuildStatus::Failure
        },
        built,
        errors,
    })
}

fn build_tool_manifest(
    root: &Path,
    tool_dir: &Path,
    toolkit_version: &str,
) -> Result<BuiltToolItem, ToolCatalogError> {
    let manifest_path = tool_dir.join("manifest.json");
    let source = fs::read_to_string(&manifest_path)
        .map_err(|error| ToolCatalogError::io("reading tool manifest", &manifest_path, error))?;
    let raw: RawToolManifest = serde_json::from_str(&source)
        .map_err(|error| ToolCatalogError::json("parsing tool manifest", &manifest_path, error))?;
    let raw_payload: JsonPayload = serde_json::from_str(&source)
        .map_err(|error| ToolCatalogError::json("parsing tool manifest", &manifest_path, error))?;
    let JsonPayload::Object(raw_object) = raw_payload else {
        return Err(ToolCatalogError::InvalidManifest {
            path: manifest_path,
            message: "manifest.json must be an object.".to_owned(),
        });
    };
    let output = raw
        .output
        .unwrap_or_else(|| normalize_tool_output(raw.runx.as_ref()));
    let source_hash = hash_tool_source(tool_dir)?;
    let schema_hash = schema_hash(&raw_object, &output);
    let manifest = ToolManifest {
        schema: ToolManifestSchema::V1,
        name: raw.name,
        version: raw.version,
        description: raw.description,
        source: raw.source,
        runtime: raw
            .runtime
            .unwrap_or_else(|| runtime_from_source(&raw_object)),
        inputs: raw.inputs,
        output,
        scopes: raw.scopes,
        risk: raw.risk,
        retry: raw.retry,
        idempotency: raw.idempotency,
        mutating: raw.mutating,
        runx: raw.runx,
        source_hash,
        schema_hash,
        toolkit_version: Some(toolkit_version.to_owned()),
    };
    validate_manifest(&manifest, &manifest_path)?;
    write_manifest(&manifest_path, &manifest)?;
    Ok(BuiltToolItem {
        path: project_path(root, tool_dir),
        manifest: project_path(root, &manifest_path),
        source_hash: manifest.source_hash,
        schema_hash: manifest.schema_hash,
    })
}

fn normalize_tool_output(runx: Option<&JsonPayloadObject>) -> ToolOutput {
    let artifacts = runx
        .and_then(|runx| runx.get("artifacts"))
        .and_then(|value| match value {
            JsonPayload::Object(value) => Some(value),
            _ => None,
        });
    let wrap_as = artifacts
        .and_then(|artifacts| artifacts.get("wrap_as"))
        .and_then(|value| match value {
            JsonPayload::String(value) => Some(value.clone()),
            _ => None,
        });
    let mut extra = JsonPayloadObject::new();
    if let Some(JsonPayload::Object(named_emits)) =
        artifacts.and_then(|artifacts| artifacts.get("named_emits"))
    {
        extra.insert(
            "named_emits".to_owned(),
            JsonPayload::Object(named_emits.clone()),
        );
    }
    ToolOutput {
        packet: None,
        wrap_as,
        named_emits: BTreeMap::new(),
        outputs: BTreeMap::new(),
        extra,
    }
}

fn runtime_from_source(raw_object: &JsonPayloadObject) -> RuntimeCommand {
    let source = raw_object.get("source").and_then(|value| match value {
        JsonPayload::Object(value) => Some(value),
        _ => None,
    });
    let command = source
        .and_then(|source| source.get("command"))
        .and_then(|value| match value {
            JsonPayload::String(value) => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "node".to_owned());
    let args = source
        .and_then(|source| source.get("args"))
        .and_then(|value| match value {
            JsonPayload::Array(values) => Some(
                values
                    .iter()
                    .filter_map(|value| match value {
                        JsonPayload::String(value) => Some(value.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .filter(|args| !args.is_empty())
        .unwrap_or_else(|| vec!["./run.mjs".to_owned()]);
    RuntimeCommand {
        command,
        args,
        cwd: None,
        env: BTreeMap::new(),
    }
}

fn schema_hash(raw: &JsonPayloadObject, output: &ToolOutput) -> String {
    let mut payload = JsonPayloadObject::new();
    if let Some(inputs) = raw.get("inputs") {
        payload.insert("inputs".to_owned(), inputs.clone());
    }
    payload.insert("output".to_owned(), tool_output_payload(output));
    if let Some(artifacts) = raw.get("runx").and_then(|value| match value {
        JsonPayload::Object(value) => value.get("artifacts"),
        _ => None,
    }) {
        payload.insert("artifacts".to_owned(), artifacts.clone());
    }
    sha256_stable(&JsonPayload::Object(payload))
}

fn tool_output_payload(output: &ToolOutput) -> JsonPayload {
    let mut object = output.extra.clone();
    if let Some(packet) = &output.packet {
        object.insert("packet".to_owned(), JsonPayload::String(packet.clone()));
    }
    if let Some(wrap_as) = &output.wrap_as {
        object.insert("wrap_as".to_owned(), JsonPayload::String(wrap_as.clone()));
    }
    if !output.named_emits.is_empty() {
        let mut named = JsonPayloadObject::new();
        for (label, key) in &output.named_emits {
            named.insert(label.clone(), JsonPayload::String(key.clone()));
        }
        object.insert("named_emits".to_owned(), JsonPayload::Object(named));
    }
    if !output.outputs.is_empty() {
        let mut outputs = JsonPayloadObject::new();
        for (name, binding) in &output.outputs {
            outputs.insert(name.clone(), tool_output_binding_payload(binding));
        }
        object.insert("outputs".to_owned(), JsonPayload::Object(outputs));
    }
    JsonPayload::Object(object)
}

fn tool_output_binding_payload(binding: &runx_contracts::tools::ToolOutputBinding) -> JsonPayload {
    let mut object = binding.extra.clone();
    if let Some(packet) = &binding.packet {
        object.insert("packet".to_owned(), JsonPayload::String(packet.clone()));
    }
    if let Some(wrap_as) = &binding.wrap_as {
        object.insert("wrap_as".to_owned(), JsonPayload::String(wrap_as.clone()));
    }
    JsonPayload::Object(object)
}

pub(crate) fn hash_tool_source(tool_dir: &Path) -> Result<String, ToolCatalogError> {
    let roots = [tool_dir.join("src/index.ts"), tool_dir.join("run.mjs")];
    let files = tool_source_closure(&roots)?;
    let mut bytes = Vec::new();
    let hash_root = fs::canonicalize(tool_dir).unwrap_or_else(|_| tool_dir.to_path_buf());
    for file_path in &files {
        bytes.extend(source_hash_path(&hash_root, file_path).as_bytes());
        bytes.push(0);
        bytes.extend(
            fs::read(file_path)
                .map_err(|error| ToolCatalogError::io("reading tool source", file_path, error))?,
        );
        bytes.push(0);
    }
    if files.is_empty() {
        bytes.extend(b"no-source");
    }
    Ok(sha256_prefixed(&bytes))
}

fn tool_source_closure(roots: &[PathBuf]) -> Result<Vec<PathBuf>, ToolCatalogError> {
    let mut pending = roots.to_vec();
    let mut seen = BTreeSet::new();
    let mut index = 0;
    while index < pending.len() {
        let source_path = pending[index].clone();
        index += 1;
        if !source_path.exists() {
            continue;
        }
        let source_path = fs::canonicalize(&source_path)
            .map_err(|error| ToolCatalogError::io("resolving tool source", &source_path, error))?;
        if !seen.insert(source_path.clone()) {
            continue;
        }
        let source = fs::read_to_string(&source_path)
            .map_err(|error| ToolCatalogError::io("reading tool source", &source_path, error))?;
        for specifier in local_import_specifiers(&source) {
            if let Some(dependency) = resolve_local_source_import(&source_path, &specifier)? {
                pending.push(dependency);
            }
        }
    }
    Ok(seen.into_iter().collect())
}

fn local_import_specifiers(source: &str) -> Vec<String> {
    let mut specifiers = Vec::new();
    let mut chars = source.char_indices().peekable();
    while let Some((start, quote)) = chars.next() {
        if quote != '"' && quote != '\'' {
            continue;
        }
        let mut escaped = false;
        let mut end = start + quote.len_utf8();
        for (index, character) in chars.by_ref() {
            end = index;
            if escaped {
                escaped = false;
                continue;
            }
            if character == '\\' {
                escaped = true;
                continue;
            }
            if character == quote {
                break;
            }
        }
        let value = &source[start + quote.len_utf8()..end];
        if value.starts_with("./") || value.starts_with("../") {
            specifiers.push(value.to_owned());
        }
    }
    specifiers
}

fn resolve_local_source_import(
    from_file: &Path,
    specifier: &str,
) -> Result<Option<PathBuf>, ToolCatalogError> {
    let clean_specifier = specifier
        .split(['?', '#'])
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(specifier);
    let base = from_file
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(clean_specifier);
    for candidate in source_import_candidates(&base) {
        if candidate.exists() {
            return fs::canonicalize(&candidate)
                .map(Some)
                .map_err(|error| ToolCatalogError::io("resolving tool source", &candidate, error));
        }
    }
    Ok(None)
}

fn source_import_candidates(base: &Path) -> Vec<PathBuf> {
    let Some(extension) = base.extension().and_then(|extension| extension.to_str()) else {
        let extensions = [".ts", ".tsx", ".mts", ".cts", ".js", ".mjs", ".cjs"];
        return extensions
            .iter()
            .map(|extension| PathBuf::from(format!("{}{}", base.display(), extension)))
            .chain(
                extensions
                    .iter()
                    .map(|extension| base.join(format!("index{extension}"))),
            )
            .collect();
    };
    let mut candidates = vec![base.to_path_buf()];
    match extension {
        "js" => {
            candidates.push(base.with_extension("ts"));
            candidates.push(base.with_extension("tsx"));
        }
        "mjs" => {
            candidates.push(base.with_extension("mts"));
            candidates.push(base.with_extension("ts"));
        }
        "cjs" => {
            candidates.push(base.with_extension("cts"));
            candidates.push(base.with_extension("ts"));
        }
        _ => {}
    }
    candidates
}

fn source_hash_path(root: &Path, file_path: &Path) -> String {
    let root_components = path_component_strings(root);
    let file_components = path_component_strings(file_path);
    let common_len = root_components
        .iter()
        .zip(&file_components)
        .take_while(|(left, right)| left == right)
        .count();
    if common_len == 0 {
        return file_path.to_string_lossy().replace('\\', "/");
    }
    let mut parts = Vec::new();
    for _ in common_len..root_components.len() {
        parts.push("..".to_owned());
    }
    parts.extend(file_components[common_len..].iter().cloned());
    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}

fn path_component_strings(path: &Path) -> Vec<String> {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect()
}

fn validate_manifest(manifest: &ToolManifest, path: &Path) -> Result<(), ToolCatalogError> {
    let json = serde_json::to_string(manifest)
        .map_err(|error| ToolCatalogError::json("serializing tool manifest", path, error))?;
    let raw = runx_parser::parse_tool_manifest_json(&json).map_err(|error| {
        ToolCatalogError::InvalidManifest {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    })?;
    runx_parser::validate_tool_manifest(raw).map_err(|error| {
        ToolCatalogError::InvalidManifest {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    })?;
    Ok(())
}

fn write_manifest(path: &Path, manifest: &ToolManifest) -> Result<(), ToolCatalogError> {
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|error| ToolCatalogError::json("serializing tool manifest", path, error))?;
    fs::write(path, format!("{json}\n"))
        .map_err(|error| ToolCatalogError::io("writing tool manifest", path, error))
}

fn discover_tool_directories(root: &Path) -> Result<Vec<PathBuf>, ToolCatalogError> {
    let tools_root = root.join("tools");
    let mut directories = Vec::new();
    for namespace in read_dirs(&tools_root)? {
        for tool in read_dirs(&namespace)? {
            if tool.join("manifest.json").exists() {
                directories.push(tool);
            }
        }
    }
    directories.sort();
    Ok(directories)
}

fn read_dirs(path: &Path) -> Result<Vec<PathBuf>, ToolCatalogError> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(ToolCatalogError::io("reading directory", path, error)),
    };
    let mut dirs = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| ToolCatalogError::io("reading directory", path, error))?;
        let file_type = entry.file_type().map_err(|error| {
            ToolCatalogError::io("reading directory entry", entry.path(), error)
        })?;
        if file_type.is_dir() {
            dirs.push(entry.path());
        }
    }
    dirs.sort();
    Ok(dirs)
}

fn resolve_tool_path(root: &Path, tool_path: Option<&Path>) -> Result<PathBuf, ToolCatalogError> {
    let Some(tool_path) = tool_path else {
        return Err(ToolCatalogError::InvalidRequest(
            "runx tool build requires a tool directory or --all".to_owned(),
        ));
    };
    if tool_path.is_absolute() {
        Ok(tool_path.to_path_buf())
    } else {
        Ok(root.join(tool_path))
    }
}

pub(crate) fn project_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map_or(path, |path| path)
        .to_string_lossy()
        .replace('\\', "/")
}
