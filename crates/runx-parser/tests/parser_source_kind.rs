use runx_parser::{
    InputMode, SourceKind, ValidateSkillOptions, parse_skill_markdown, validate_skill,
    validate_skill_with_options,
};

fn parse_strict(markdown: &str) -> Result<runx_parser::ValidatedSkill, String> {
    let raw = parse_skill_markdown(markdown).map_err(|error| error.to_string())?;
    validate_skill(raw).map_err(|error| error.to_string())
}

#[test]
fn cli_tool_source_parses_to_typed_kind_and_input_mode() -> Result<(), String> {
    let skill = parse_strict(
        r#"---
name: cli-skill
source:
  type: cli-tool
  command: node
  input_mode: stdin
---
# CLI
"#,
    )?;
    assert_eq!(skill.source.source_type, SourceKind::CliTool);
    assert_eq!(skill.source.input_mode, Some(InputMode::Stdin));
    // The typed kind serializes back to the original wire string.
    assert_eq!(skill.source.source_type.as_str(), "cli-tool");
    Ok(())
}

#[test]
fn default_source_is_agent_kind() -> Result<(), String> {
    // A skill with no explicit source defaults to the `agent` source; the typed
    // `SourceKind` must carry an `Agent` variant for that (the built-in default).
    let raw = parse_skill_markdown(
        r#"---
name: portable-agent
inputs:
  prompt:
    type: string
    required: true
---
# Portable agent
"#,
    )
    .map_err(|error| error.to_string())?;
    let skill = validate_skill_with_options(raw, ValidateSkillOptions::lenient())
        .map_err(|error| error.to_string())?;
    assert_eq!(skill.source.source_type, SourceKind::Agent);
    Ok(())
}

#[test]
fn http_source_parses_url_and_method() -> Result<(), String> {
    let skill = parse_strict(
        r#"---
name: http-skill
source:
  type: http
  url: https://api.example.test/v1/pets
  method: POST
---
# HTTP
"#,
    )?;
    assert_eq!(skill.source.source_type, SourceKind::Http);
    assert_eq!(skill.source.source_type.as_str(), "http");
    assert_eq!(
        skill.source.url.as_deref(),
        Some("https://api.example.test/v1/pets")
    );
    assert_eq!(skill.source.method.as_deref(), Some("POST"));
    Ok(())
}

#[test]
fn http_source_requires_a_url() -> Result<(), String> {
    let raw = parse_skill_markdown(
        r#"---
name: http-no-url
source:
  type: http
---
# HTTP
"#,
    )
    .map_err(|error| error.to_string())?;
    assert!(
        validate_skill(raw).is_err(),
        "an http source without a url must fail closed"
    );
    Ok(())
}

#[test]
fn http_source_rejects_an_unsupported_method() -> Result<(), String> {
    let raw = parse_skill_markdown(
        r#"---
name: http-bad-method
source:
  type: http
  url: https://api.example.test/v1/pets
  method: PATCH
---
# HTTP
"#,
    )
    .map_err(|error| error.to_string())?;
    assert!(
        validate_skill(raw).is_err(),
        "an unsupported http method must fail closed"
    );
    Ok(())
}

#[test]
fn unknown_source_type_fails_closed() -> Result<(), String> {
    let raw = parse_skill_markdown(
        r#"---
name: bogus
source:
  type: not-a-real-source
---
# Bogus
"#,
    )
    .map_err(|error| error.to_string())?;
    assert!(
        validate_skill(raw).is_err(),
        "an unknown source.type must fail closed at parse time"
    );
    Ok(())
}
