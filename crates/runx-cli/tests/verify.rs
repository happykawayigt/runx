#[test]
fn verify_json_failure_envelope_keeps_stderr_clean() -> Result<(), Box<dyn std::error::Error>> {
    let output = crate::support::isolated_runx_command("verify-json-error")?
        .args(["verify", "--receipt", "missing.json", "--json"])
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8(output.stderr)?, "");
    let value = serde_json::from_slice::<serde_json::Value>(&output.stdout)?;
    assert_eq!(value["status"], "failure");
    assert_eq!(value["error"]["code"], "runtime_error");
    assert!(
        value["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("failed to read receipt"))
    );

    Ok(())
}
