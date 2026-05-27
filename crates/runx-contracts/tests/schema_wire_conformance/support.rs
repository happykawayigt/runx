use std::path::PathBuf;

use serde_json::Value;

pub(super) struct SchemaDirRetriever {
    pub(super) dir: PathBuf,
}

impl jsonschema::Retrieve for SchemaDirRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let expected = uri.to_string();
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let raw = std::fs::read_to_string(entry.path())?;
            let schema: Value = serde_json::from_str(&raw)?;
            if schema.get("$id").and_then(Value::as_str) == Some(expected.as_str()) {
                return Ok(schema);
            }
        }
        Err(format!("schema reference not found: {expected}").into())
    }
}

pub(super) fn committed_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas")
}
