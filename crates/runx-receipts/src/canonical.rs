use std::collections::BTreeMap;

use runx_contracts::{JsonNumber, JsonValue, Receipt, sha256_prefixed};

use crate::ReceiptError;

pub fn canonical_receipt_json(receipt: &Receipt) -> Result<String, ReceiptError> {
    let value = receipt_json(receipt)?;
    canonical_json_value(&value)
}

pub fn canonical_receipt_digest(receipt: &Receipt) -> Result<String, ReceiptError> {
    canonical_receipt_json(receipt).map(|json| sha256_prefixed(json.as_bytes()))
}

pub fn canonical_receipt_body_json(receipt: &Receipt) -> Result<String, ReceiptError> {
    let mut value = receipt_json(receipt)?;
    strip_body_proof_fields(&mut value);
    canonical_json_value(&value)
}

pub fn canonical_receipt_body_digest(receipt: &Receipt) -> Result<String, ReceiptError> {
    canonical_receipt_body_json(receipt).map(|json| sha256_prefixed(json.as_bytes()))
}

fn receipt_json(receipt: &Receipt) -> Result<JsonValue, ReceiptError> {
    let value = serde_json::to_value(receipt).map_err(|source| ReceiptError::Serialization {
        message: source.to_string(),
    })?;
    serde_json::from_value(value).map_err(|source| ReceiptError::Serialization {
        message: source.to_string(),
    })
}

/// The signed body commits every flat field except the envelope's own
/// `signature` and `digest`. `metadata` is a runtime-local read aid and is not
/// part of the signed body.
fn strip_body_proof_fields(value: &mut JsonValue) {
    if let JsonValue::Object(map) = value {
        map.remove("signature");
        map.remove("digest");
        map.remove("metadata");
    }
}

fn canonical_json_value(value: &JsonValue) -> Result<String, ReceiptError> {
    match value {
        JsonValue::Null => Ok("null".to_owned()),
        JsonValue::Bool(value) => Ok(value.to_string()),
        JsonValue::Number(value) => Ok(canonical_json_number(value)),
        JsonValue::String(value) => {
            serde_json::to_string(value).map_err(|source| ReceiptError::Serialization {
                message: source.to_string(),
            })
        }
        JsonValue::Array(items) => {
            let body = items
                .iter()
                .map(canonical_json_value)
                .collect::<Result<Vec<_>, _>>()?
                .join(",");
            Ok(format!("[{body}]"))
        }
        JsonValue::Object(map) => {
            let ordered = map.iter().collect::<BTreeMap<_, _>>();
            let body = ordered
                .into_iter()
                .map(|(key, value)| {
                    let key = serde_json::to_string(key).map_err(|source| {
                        ReceiptError::Serialization {
                            message: source.to_string(),
                        }
                    })?;
                    Ok(format!("{key}:{}", canonical_json_value(value)?))
                })
                .collect::<Result<Vec<_>, ReceiptError>>()?
                .join(",");
            Ok(format!("{{{body}}}"))
        }
    }
}

fn canonical_json_number(value: &JsonNumber) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use runx_contracts::JsonValue;
    use runx_contracts::Receipt;
    use serde::Deserialize;

    use super::{
        ReceiptError, canonical_receipt_body_digest, canonical_receipt_body_json,
        canonical_receipt_digest, canonical_receipt_json, sha256_prefixed,
    };

    const SUCCESS_RECEIPT: &str =
        include_str!("../../../fixtures/contracts/harness-spine/receipt-success.json");
    const ABNORMAL_RECEIPT: &str =
        include_str!("../../../fixtures/contracts/harness-spine/receipt-abnormal.json");
    const POST_MERGE_OBSERVER_RECEIPT: &str = include_str!(
        "../../../fixtures/contracts/harness-spine/post-merge-observer-merged-verified.json"
    );
    const RECEIPT_ORACLE: &str = include_str!(
        "../../../fixtures/contracts/canonical-json/runx-receipt-c14n-v1.oracles.json"
    );

    #[derive(Debug, Deserialize)]
    struct Fixture {
        expected: Receipt,
    }

    #[derive(Debug, Deserialize)]
    struct ReceiptOracleFixture {
        canonicalization: String,
        cases: Vec<ReceiptOracleCase>,
    }

    #[derive(Debug, Deserialize)]
    struct ReceiptOracleCase {
        name: String,
        fixture: String,
        full_canonical_json: String,
        full_sha256: String,
        body_canonical_json: String,
        body_sha256: String,
    }

    #[test]
    fn sha256_prefixes_digest() {
        assert_eq!(
            sha256_prefixed(b"runx"),
            "sha256:8186b7035bea2f66ebe27c1f5cf7de4e94ef935e259a2f3160352adffc752f28"
        );
    }

    #[test]
    fn canonical_receipt_json_is_stable_and_sorted() -> Result<(), ReceiptError> {
        let receipt = fixture()?;
        let first = canonical_receipt_json(&receipt)?;
        let second = canonical_receipt_json(&receipt)?;

        assert_eq!(first, second);
        assert!(first.contains("\"created_at\":\""));
        assert!(canonical_receipt_digest(&receipt)?.starts_with("sha256:"));
        Ok(())
    }

    #[test]
    fn body_commitment_excludes_signature_and_seal_derived_fields() -> Result<(), ReceiptError> {
        let mut receipt = fixture()?;
        let baseline_json = canonical_receipt_body_json(&receipt)?;
        let baseline_digest = canonical_receipt_body_digest(&receipt)?;

        receipt.signature.value = "base64:changed".to_owned();
        receipt.digest = "sha256:changed".to_owned();

        assert_eq!(canonical_receipt_body_json(&receipt)?, baseline_json);
        assert_eq!(canonical_receipt_body_digest(&receipt)?, baseline_digest);
        Ok(())
    }

    #[test]
    fn body_commitment_excludes_metadata_read_aid() -> Result<(), ReceiptError> {
        let mut receipt = fixture()?;
        let baseline_digest = canonical_receipt_body_digest(&receipt)?;

        receipt.metadata.get_or_insert_default().insert(
            "skill_name".to_owned(),
            JsonValue::String("changed-read-aid".to_owned()),
        );

        assert_eq!(canonical_receipt_body_digest(&receipt)?, baseline_digest);
        Ok(())
    }

    #[test]
    fn receipt_oracle_matches_rust_canonical_json() -> Result<(), ReceiptError> {
        let oracle: ReceiptOracleFixture = serde_json::from_str(RECEIPT_ORACLE).map_err(
            |source| ReceiptError::Serialization {
                message: source.to_string(),
            },
        )?;
        assert_eq!(oracle.canonicalization, "runx.receipt.c14n.v1");

        for case in oracle.cases {
            let receipt = fixture_by_path(&case.fixture)?;
            assert_eq!(
                canonical_receipt_json(&receipt)?,
                case.full_canonical_json,
                "{} full canonical JSON drifted",
                case.name
            );
            assert_eq!(
                canonical_receipt_digest(&receipt)?,
                case.full_sha256,
                "{} full digest drifted",
                case.name
            );
            assert_eq!(
                canonical_receipt_body_json(&receipt)?,
                case.body_canonical_json,
                "{} body canonical JSON drifted",
                case.name
            );
            assert_eq!(
                canonical_receipt_body_digest(&receipt)?,
                case.body_sha256,
                "{} body digest drifted",
                case.name
            );
        }
        Ok(())
    }

    fn fixture() -> Result<Receipt, ReceiptError> {
        serde_json::from_str::<Fixture>(SUCCESS_RECEIPT)
            .map(|fixture| fixture.expected)
            .map_err(|source| ReceiptError::Serialization {
                message: source.to_string(),
            })
    }

    fn fixture_by_path(path: &str) -> Result<Receipt, ReceiptError> {
        let json = match path {
            "harness-spine/receipt-abnormal.json" => ABNORMAL_RECEIPT,
            "harness-spine/receipt-success.json" => SUCCESS_RECEIPT,
            "harness-spine/post-merge-observer-merged-verified.json" => POST_MERGE_OBSERVER_RECEIPT,
            _ => {
                return Err(ReceiptError::Serialization {
                    message: format!("unknown receipt oracle fixture: {path}"),
                });
            }
        };
        serde_json::from_str::<Fixture>(json)
            .map(|fixture| fixture.expected)
            .map_err(|source| ReceiptError::Serialization {
                message: source.to_string(),
            })
    }
}
