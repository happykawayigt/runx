use serde::de::DeserializeOwned;

use crate::ParseError;

const DIVERGENT_BOOLISH: &[&str] = &["yes", "no", "on", "off"];

pub fn parse_yaml_document<T>(source: &str) -> Result<T, ParseError>
where
    T: DeserializeOwned,
{
    assert_yaml_parity_subset("yaml", source)?;
    serde_norway::from_str(source).map_err(|error| ParseError::InvalidYaml {
        field: "yaml".to_owned(),
        message: error.to_string(),
    })
}

pub fn assert_yaml_parity_subset(field: &str, source: &str) -> Result<(), ParseError> {
    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let Some(content) = strip_yaml_comment(line) else {
            continue;
        };
        let trimmed = content.trim();
        if trimmed.is_empty() || trimmed.starts_with("---") || trimmed.starts_with("...") {
            continue;
        }
        reject_embedded_colon_key(field, line_number, trimmed)?;
        reject_colon_space_plain_scalar(field, line_number, content)?;
    }
    Ok(())
}

/// `false` for scalars YAML would coerce to a non-string (booleans, numbers,
/// dates, special floats), so callers know to keep them quoted.
///
/// # Examples
///
/// ```
/// use runx_parser::yaml::yaml_scalar_subset_allows;
///
/// assert!(yaml_scalar_subset_allows("echo")); // plain string: safe
/// assert!(!yaml_scalar_subset_allows("yes")); // YAML boolean: ambiguous
/// ```
#[must_use]
pub fn yaml_scalar_subset_allows(literal: &str) -> bool {
    let trimmed = literal.trim();
    !is_boolish(trimmed)
        && !is_base_prefixed_number(trimmed)
        && !is_sexagesimal_like(trimmed)
        && !is_date_like(trimmed)
        && !is_special_float(trimmed)
}

pub fn assert_yaml_scalar_subset(field: &str, literal: &str) -> Result<(), ParseError> {
    if yaml_scalar_subset_allows(literal) {
        return Ok(());
    }
    Err(ParseError::UnsupportedScalar {
        field: field.to_owned(),
        literal: literal.to_owned(),
    })
}

fn strip_yaml_comment(line: &str) -> Option<&str> {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut previous = '\0';
    for (index, char) in line.char_indices() {
        match char {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote && previous != '\\' => in_double_quote = !in_double_quote,
            '#' if !in_single_quote && !in_double_quote && is_comment_start(line, index) => {
                return Some(&line[..index]);
            }
            _ => {}
        }
        previous = char;
    }
    Some(line)
}

fn is_comment_start(line: &str, index: usize) -> bool {
    index == 0 || line[..index].ends_with(char::is_whitespace)
}

fn reject_embedded_colon_key(
    field: &str,
    line_number: usize,
    trimmed: &str,
) -> Result<(), ParseError> {
    let Some(key) = top_level_plain_key(trimmed) else {
        return Ok(());
    };
    if key.contains(':') {
        return Err(ambiguous_yaml(field, line_number, trimmed));
    }
    Ok(())
}

// rust-style-allow: long-function because this quote-aware scanner keeps
// mapping delimiter detection in one place instead of splitting YAML parsing.
fn top_level_plain_key(trimmed: &str) -> Option<&str> {
    let bytes = trimmed.as_bytes();
    if bytes
        .first()
        .is_some_and(|byte| matches!(byte, b'-' | b'?' | b'{' | b'[' | b'"' | b'\''))
    {
        return None;
    }
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut previous = '\0';
    for (index, char) in trimmed.char_indices() {
        match char {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote && previous != '\\' => in_double_quote = !in_double_quote,
            ':' if !in_single_quote && !in_double_quote && is_mapping_delimiter(trimmed, index) => {
                return Some(trimmed[..index].trim());
            }
            _ => {}
        }
        previous = char;
    }
    None
}

fn is_mapping_delimiter(value: &str, index: usize) -> bool {
    value[index + 1..]
        .chars()
        .next()
        .is_none_or(char::is_whitespace)
}

fn reject_colon_space_plain_scalar(
    field: &str,
    line_number: usize,
    content: &str,
) -> Result<(), ParseError> {
    let Some((_, value)) = split_plain_mapping_value(content) else {
        return Ok(());
    };
    if plain_scalar_contains_colon_space(value) {
        return Err(ambiguous_yaml(field, line_number, value.trim()));
    }
    Ok(())
}

fn split_plain_mapping_value(content: &str) -> Option<(&str, &str)> {
    let trimmed = content.trim_start();
    let key = top_level_plain_key(trimmed)?;
    let delimiter_index = key.len();
    Some((key, &trimmed[delimiter_index + 1..]))
}

// rust-style-allow: long-function because the scalar exemptions and
// quote-aware colon scanner are one validation rule.
fn plain_scalar_contains_colon_space(value: &str) -> bool {
    let trimmed = value.trim_start();
    if trimmed.is_empty()
        || trimmed.starts_with(['"', '\'', '|', '>', '{', '['])
        || trimmed == "null"
        || matches!(trimmed, "true" | "false")
    {
        return false;
    }
    contains_unquoted_colon_space(trimmed)
}

fn contains_unquoted_colon_space(value: &str) -> bool {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut previous = '\0';
    for (index, char) in value.char_indices() {
        match char {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote && previous != '\\' => in_double_quote = !in_double_quote,
            ':' if !in_single_quote && !in_double_quote && is_mapping_delimiter(value, index) => {
                return true;
            }
            _ => {}
        }
        previous = char;
    }
    false
}

fn ambiguous_yaml(field: &str, line_number: usize, literal: &str) -> ParseError {
    ParseError::InvalidYaml {
        field: field.to_owned(),
        message: format!(
            "ambiguous YAML construct at line {line_number}; quote the value or key: {literal}"
        ),
    }
}

fn is_boolish(value: &str) -> bool {
    DIVERGENT_BOOLISH
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn is_base_prefixed_number(value: &str) -> bool {
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    unsigned.starts_with("0x") || unsigned.starts_with("0X") || unsigned.starts_with("0o")
}

fn is_sexagesimal_like(value: &str) -> bool {
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    let mut parts = unsigned.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    first.chars().all(|char| char.is_ascii_digit())
        && parts.clone().count() > 0
        && parts.all(|part| !part.is_empty() && part.chars().all(|char| char.is_ascii_digit()))
}

fn is_date_like(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 10
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn is_special_float(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        ".nan" | ".inf" | "+.inf" | "-.inf"
    )
}

#[cfg(test)]
mod tests {
    use super::{assert_yaml_scalar_subset, yaml_scalar_subset_allows};

    #[test]
    fn scalar_subset_rejects_divergent_forms() {
        for literal in ["yes", "ON", "0x10", "0o10", "12:34", "2026-05-18", ".nan"] {
            assert!(!yaml_scalar_subset_allows(literal), "{literal}");
        }
    }

    #[test]
    fn scalar_subset_allows_explicit_json_like_scalars() -> Result<(), crate::ParseError> {
        for literal in ["true", "false", "1", "1.5", "plain text", "\"yes\""] {
            assert_yaml_scalar_subset("fixture", literal)?;
        }
        Ok(())
    }
}
