#[test]
fn redaction_removes_secret_prefixes_bearers_and_urls() {
    let display = runx_runtime::redact_sensitive_text(
        "failed SECRET_CREDENTIAL_BODY_DO_NOT_LEAK bearer Bearer abc.def https://auth.example/authorize?code=SECRET_AUTHORIZE_QUERY_DO_NOT_LEAK",
    );

    assert!(!display.contains("SECRET_CREDENTIAL_BODY_DO_NOT_LEAK"));
    assert!(!display.contains("abc.def"));
    assert!(!display.contains("SECRET_AUTHORIZE_QUERY_DO_NOT_LEAK"));
    assert!(!display.contains("https://auth.example"));
    assert!(display.contains("[redacted]"));
    assert!(display.contains("[redacted-url]"));
}

#[test]
fn redaction_keeps_non_secret_text_readable() {
    let display = runx_runtime::redact_sensitive_text(
        "provider requires a credential; retry after setting GITHUB_TOKEN",
    );

    assert_eq!(
        display,
        "provider requires a credential; retry after setting GITHUB_TOKEN"
    );
}
