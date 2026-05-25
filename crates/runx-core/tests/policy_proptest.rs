use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use runx_contracts::{AuthorityTerm, JsonObject, JsonValue};
use runx_core::policy::{
    GraphScopeAdmissionDecision, GraphScopeAdmissionRequest, GraphScopeGrant, LocalAdmissionGrant,
    LocalAdmissionGrantStatus, LocalAdmissionOptions, LocalAdmissionSkill, LocalAdmissionSource,
    LocalExecutionPolicy, RetryAdmissionRequest, RetryPolicy, admit_graph_step_scopes,
    admit_local_skill, is_payment_authority_subset,
};
use serde_json::Value as SerdeJsonValue;
use serde_json::json;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn local_admission_is_deterministic(
        skill in local_admission_skill(),
        options in local_admission_options(),
    ) {
        let left = admit_local_skill(&skill, &options);
        let right = admit_local_skill(&skill, &options);
        let left_json = serde_json::to_string(&left).map_err(test_case_error)?;
        let right_json = serde_json::to_string(&right).map_err(test_case_error)?;

        prop_assert_eq!(left_json, right_json);
    }

    // The chosen connected-auth grant is intentionally not exposed through
    // AdmissionDecision. The first-match ordering property is asserted where
    // the selector is visible: policy::connected_auth::tests.

    #[test]
    fn graph_scope_admission_is_deterministic(
        request in graph_scope_request(),
    ) {
        let left = admit_graph_step_scopes(&request);
        let right = admit_graph_step_scopes(&request);
        let left_json = serde_json::to_string(&left).map_err(test_case_error)?;
        let right_json = serde_json::to_string(&right).map_err(test_case_error)?;

        prop_assert_eq!(left_json, right_json);
    }

    #[test]
    fn graph_scope_deduplication_is_idempotent(
        request in graph_scope_request(),
    ) {
        let first = admit_graph_step_scopes(&request);
        let normalized = request_from_decision(&first);
        let second = admit_graph_step_scopes(&normalized);

        prop_assert_eq!(first, second);
    }

    #[test]
    fn retry_admission_is_deterministic(
        request in retry_request(),
    ) {
        let left = runx_core::policy::admit_retry_policy(&request);
        let right = runx_core::policy::admit_retry_policy(&request);

        prop_assert_eq!(left, right);
    }

    #[test]
    fn payment_authority_comparison_is_deterministic(
        child in payment_authority_term(),
        parent in payment_authority_term(),
    ) {
        let left = is_payment_authority_subset(&child, &parent);
        let right = is_payment_authority_subset(&child, &parent);

        prop_assert_eq!(left, right);
    }
}

#[test]
fn payment_authority_allows_narrower_child() -> Result<(), serde_json::Error> {
    let parent = payment_term_json(
        "parent",
        &["quote", "reserve", "spend", "refund", "verify"],
        json!({
            "currency": "USD",
            "rails": ["card", "ach"],
            "max_per_call_minor": 10_000_u64,
            "max_per_run_minor": 25_000_u64,
            "quote_required": true,
            "reservation_required": true,
            "idempotency_required": true,
            "recovery_required": true,
            "receipt_before_success": true,
            "quote_ttl_ms": 300_000_u64,
            "approval_threshold_minor": 7_500_u64,
            "single_use_spend": true
        }),
        Some("2026-06-01T00:00:00Z"),
    )?;
    let child = payment_term_json(
        "child",
        &["reserve", "spend"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "realm": "prod",
            "counterparty": "merchant-123",
            "operation": "checkout",
            "max_per_call_minor": 2_500_u64,
            "max_per_run_minor": 10_000_u64,
            "quote_required": true,
            "reservation_required": true,
            "idempotency_required": true,
            "recovery_required": true,
            "receipt_before_success": true,
            "quote_ttl_ms": 120_000_u64,
            "approval_threshold_minor": 2_500_u64,
            "credential_form": "single_use_spend_capability",
            "single_use_spend": true
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;

    assert!(is_payment_authority_subset(&child, &parent));
    Ok(())
}

#[test]
fn payment_authority_resource_comparison_ignores_reference_decoration()
-> Result<(), serde_json::Error> {
    let parent = payment_term_json(
        "parent",
        &["reserve"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "max_per_call_minor": 2_000_u64
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;
    let child = payment_term_json(
        "child",
        &["reserve"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "max_per_call_minor": 1_000_u64
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;
    let parent = with_resource_label(parent, "Merchant account");
    let child = with_resource_label(child, "Checkout merchant account");

    assert!(is_payment_authority_subset(&child, &parent));
    Ok(())
}

#[test]
fn payment_authority_allows_reserve_without_single_use_spend_capability()
-> Result<(), serde_json::Error> {
    let parent = payment_term_json(
        "parent",
        &["reserve"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "max_per_call_minor": 2_000_u64,
            "reservation_required": true
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;
    let child = payment_term_json(
        "child",
        &["reserve"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "max_per_call_minor": 1_000_u64,
            "max_per_run_minor": 1_000_u64,
            "reservation_required": true
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;

    assert!(is_payment_authority_subset(&child, &parent));
    Ok(())
}

#[test]
fn payment_authority_denies_widening_dimensions() -> Result<(), serde_json::Error> {
    let parent = payment_term_json(
        "parent",
        &["spend"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "realm": "prod",
            "max_per_call_minor": 1_000_u64,
            "quote_required": true,
            "single_use_spend": true
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;

    let currency_widening = payment_term_json(
        "currency-widening",
        &["spend"],
        json!({
            "currency": "EUR",
            "rails": ["card"],
            "realm": "prod",
            "max_per_call_minor": 500_u64,
            "quote_required": true,
            "single_use_spend": true
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;
    let omitted_parent_realm = payment_term_json(
        "omitted-parent-realm",
        &["spend"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "max_per_call_minor": 500_u64,
            "quote_required": true,
            "single_use_spend": true
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;
    let disabled_required_boolean = payment_term_json(
        "disabled-required-boolean",
        &["spend"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "realm": "prod",
            "max_per_call_minor": 500_u64,
            "quote_required": false,
            "single_use_spend": true
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;

    assert!(!is_payment_authority_subset(&currency_widening, &parent));
    assert!(!is_payment_authority_subset(&omitted_parent_realm, &parent));
    assert!(!is_payment_authority_subset(
        &disabled_required_boolean,
        &parent
    ));
    Ok(())
}

#[test]
fn payment_authority_denies_missing_required_payment_guards() -> Result<(), serde_json::Error> {
    let parent = payment_term_json(
        "parent",
        &["reserve", "spend"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "max_per_call_minor": 2_000_u64,
            "single_use_spend": true
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;
    let missing_cap = payment_term_json(
        "missing-cap",
        &["spend"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "single_use_spend": true
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;
    let missing_expiry = payment_term_json(
        "missing-expiry",
        &["spend"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "max_per_call_minor": 1_000_u64,
            "single_use_spend": true
        }),
        None,
    )?;
    let missing_single_use = payment_term_json(
        "missing-single-use",
        &["spend"],
        json!({
            "currency": "USD",
            "rails": ["card"],
            "max_per_call_minor": 1_000_u64
        }),
        Some("2026-05-21T00:00:00Z"),
    )?;

    assert!(!is_payment_authority_subset(&missing_cap, &parent));
    assert!(!is_payment_authority_subset(&missing_expiry, &parent));
    assert!(!is_payment_authority_subset(&missing_single_use, &parent));
    Ok(())
}

fn request_from_decision(decision: &GraphScopeAdmissionDecision) -> GraphScopeAdmissionRequest {
    match decision {
        GraphScopeAdmissionDecision::Allow {
            step_id,
            requested_scopes,
            granted_scopes,
            grant_id,
            ..
        }
        | GraphScopeAdmissionDecision::Deny {
            step_id,
            requested_scopes,
            granted_scopes,
            grant_id,
            ..
        } => GraphScopeAdmissionRequest {
            step_id: step_id.clone(),
            requested_scopes: requested_scopes.clone(),
            grant: GraphScopeGrant {
                grant_id: grant_id.clone(),
                scopes: granted_scopes.clone(),
            },
        },
    }
}

fn local_admission_skill() -> impl Strategy<Value = LocalAdmissionSkill> {
    (
        safe_id(),
        source_type(),
        prop::option::of(command()),
        prop::collection::vec(arg(), 0..4),
        prop::option::of(1_i64..600),
        prop::option::of(auth_requirement()),
    )
        .prop_map(
            |(name, source_type, command, args, timeout_seconds, auth)| LocalAdmissionSkill {
                name,
                source: LocalAdmissionSource {
                    source_type,
                    command,
                    args: Some(args),
                    timeout_seconds,
                    sandbox: None,
                },
                auth,
                runtime: None,
            },
        )
}

fn local_admission_options() -> impl Strategy<Value = LocalAdmissionOptions> {
    (
        prop::option::of(prop::collection::vec(source_type(), 0..5)),
        prop::option::of(1_i64..600),
        prop::collection::vec(local_admission_grant(), 0..4),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                allowed_source_types,
                max_timeout_seconds,
                connected_grants,
                skip_connected_auth,
                strict_cli_tool_inline_code,
            )| LocalAdmissionOptions {
                allowed_source_types,
                max_timeout_seconds,
                connected_grants: Some(connected_grants),
                connected_auth_checked_at: Some("2026-05-22T00:00:00Z".to_owned()),
                skip_connected_auth: Some(skip_connected_auth),
                approved_sandbox_escalation: None,
                skip_sandbox_escalation: None,
                execution_policy: Some(LocalExecutionPolicy {
                    strict_cli_tool_inline_code: Some(strict_cli_tool_inline_code),
                }),
            },
        )
}

fn graph_scope_request() -> impl Strategy<Value = GraphScopeAdmissionRequest> {
    (
        safe_id(),
        prop::collection::vec(scope(), 0..6),
        prop::collection::vec(scope(), 0..6),
        prop::option::of(safe_id()),
    )
        .prop_map(|(step_id, requested_scopes, granted_scopes, grant_id)| {
            GraphScopeAdmissionRequest {
                step_id,
                requested_scopes,
                grant: GraphScopeGrant {
                    grant_id,
                    scopes: granted_scopes,
                },
            }
        })
}

fn retry_request() -> impl Strategy<Value = RetryAdmissionRequest> {
    (
        safe_id(),
        prop::option::of(0_i64..5),
        any::<bool>(),
        prop::option::of(idempotency_key()),
    )
        .prop_map(
            |(step_id, max_attempts, mutating, idempotency_key)| RetryAdmissionRequest {
                step_id,
                retry: max_attempts.map(|max_attempts| RetryPolicy { max_attempts }),
                mutating: Some(mutating),
                idempotency_key,
            },
        )
}

fn payment_authority_term() -> impl Strategy<Value = AuthorityTerm> {
    (
        safe_id(),
        prop::collection::vec(payment_verb(), 1..4),
        payment_currency(),
        prop::collection::vec(payment_rail(), 0..3),
        prop::option::of(1_u64..10_000),
        prop::option::of(1_u64..10_000),
        prop::option::of(1_u64..10_000),
        prop::option::of(1_u64..10_000),
        any::<bool>(),
        any::<bool>(),
        prop::option::of(safe_id()),
    )
        .prop_map(
            |(
                term_id,
                verbs,
                currency,
                rails,
                quote_cap,
                reserve_cap,
                spend_cap,
                refund_cap,
                quote_required,
                single_use_spend_capability,
                realm,
            )| {
                let mut payment = serde_json::Map::from_iter([
                    ("currency".to_owned(), json!(currency)),
                    ("rails".to_owned(), json!(rails)),
                    ("quote_required".to_owned(), json!(quote_required)),
                    (
                        "single_use_spend".to_owned(),
                        json!(single_use_spend_capability),
                    ),
                ]);
                if let Some(max_per_call_minor) = quote_cap.or(spend_cap) {
                    payment.insert("max_per_call_minor".to_owned(), json!(max_per_call_minor));
                }
                if let Some(max_per_run_minor) = reserve_cap.or(refund_cap) {
                    payment.insert("max_per_run_minor".to_owned(), json!(max_per_run_minor));
                }
                if let Some(realm) = realm {
                    payment.insert("realm".to_owned(), json!(realm));
                }

                payment_term_json(
                    &term_id,
                    &verbs.iter().map(String::as_str).collect::<Vec<_>>(),
                    SerdeJsonValue::Object(payment),
                    Some("2026-05-21T00:00:00Z"),
                )
                .ok()
            },
        )
        .prop_filter_map("generated payment terms deserialize", |term| term)
}

fn with_resource_label(mut term: AuthorityTerm, label: &str) -> AuthorityTerm {
    term.resource_ref.label = Some(label.to_owned().into());
    term
}

fn local_admission_grant() -> impl Strategy<Value = LocalAdmissionGrant> {
    (
        safe_id(),
        prop::collection::vec(scope(), 0..4),
        prop::option::of(prop::sample::select(&[
            LocalAdmissionGrantStatus::Active,
            LocalAdmissionGrantStatus::Revoked,
        ])),
    )
        .prop_map(|(grant_id, scopes, status)| LocalAdmissionGrant {
            grant_id,
            provider: "github".to_owned(),
            scopes,
            status,
            not_before: Some("2026-05-21T00:00:00Z".to_owned()),
            expires_at: Some("2026-05-23T00:00:00Z".to_owned()),
            scope_family: None,
            authority_kind: None,
            target_repo: None,
            target_locator: None,
        })
}

fn auth_requirement() -> impl Strategy<Value = JsonValue> {
    prop::collection::vec(scope(), 0..4).prop_map(|scopes| {
        let scope_values = scopes.into_iter().map(JsonValue::String).collect();
        JsonValue::Object(JsonObject::from([
            (
                "provider".to_owned(),
                JsonValue::String("github".to_owned()),
            ),
            ("type".to_owned(), JsonValue::String("connected".to_owned())),
            ("scopes".to_owned(), JsonValue::Array(scope_values)),
        ]))
    })
}

fn source_type() -> impl Strategy<Value = String> {
    prop::sample::select(&[
        "agent",
        "agent-step",
        "approval",
        "cli-tool",
        "mcp",
        "a2a",
        "catalog",
        "graph",
        "unsupported",
    ])
    .prop_map(str::to_owned)
}

fn command() -> impl Strategy<Value = String> {
    prop::sample::select(&["node", "python3", "/usr/bin/env", "bash", "runx-tool"])
        .prop_map(str::to_owned)
}

fn arg() -> impl Strategy<Value = String> {
    prop::sample::select(&[
        "-e",
        "-c",
        "--eval",
        "print('hi')",
        "PYTHONPATH=.",
        "script.js",
    ])
    .prop_map(str::to_owned)
}

fn scope() -> impl Strategy<Value = String> {
    prop::sample::select(&[
        "*",
        "repo:read",
        "repo:write",
        "repo:*",
        "repository:read",
        "repos:list",
        "checks:read",
        "checks:*",
        "checks2:read",
        "deploy:prod",
    ])
    .prop_map(str::to_owned)
}

fn safe_id() -> impl Strategy<Value = String> {
    prop::sample::select(&["read", "write", "deploy", "checks", "graph", "step"])
        .prop_map(str::to_owned)
}

fn idempotency_key() -> impl Strategy<Value = String> {
    prop::sample::select(&["", "retry-key", "deploy-1", "same-request"]).prop_map(str::to_owned)
}

fn payment_verb() -> impl Strategy<Value = String> {
    prop::sample::select(&["quote", "reserve", "spend", "refund", "verify"]).prop_map(str::to_owned)
}

fn payment_currency() -> impl Strategy<Value = String> {
    prop::sample::select(&["USD", "EUR", "AUD"]).prop_map(str::to_owned)
}

fn payment_rail() -> impl Strategy<Value = String> {
    prop::sample::select(&["card", "ach", "wire"]).prop_map(str::to_owned)
}

fn payment_term_json(
    term_id: &str,
    verbs: &[&str],
    payment: SerdeJsonValue,
    expires_at: Option<&str>,
) -> Result<AuthorityTerm, serde_json::Error> {
    let capabilities = if payment
        .as_object()
        .and_then(|object| object.get("single_use_spend"))
        .and_then(SerdeJsonValue::as_bool)
        .unwrap_or(false)
    {
        vec!["payment_single_use_spend"]
    } else {
        Vec::new()
    };
    let mut term = json!({
        "term_id": term_id,
        "principal_ref": reference_json("principal:agent"),
        "resource_ref": reference_json("payment:merchant"),
        "resource_family": "payment",
        "verbs": verbs,
        "bounds": {
            "payment": payment
        },
        "conditions": [],
        "approvals": [],
        "capabilities": capabilities,
        "issued_by_ref": reference_json("principal:issuer")
    });

    if let (Some(expires_at), Some(object)) = (expires_at, term.as_object_mut()) {
        object.insert("expires_at".to_owned(), json!(expires_at));
    }

    serde_json::from_value(term)
}

fn reference_json(uri: &str) -> SerdeJsonValue {
    json!({
        "type": "principal",
        "uri": uri
    })
}

fn test_case_error(error: serde_json::Error) -> TestCaseError {
    TestCaseError::fail(error.to_string())
}
