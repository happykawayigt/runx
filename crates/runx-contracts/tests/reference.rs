use runx_contracts::{Reference, ReferenceType};

#[test]
fn reference_type_as_str_is_stable_snake_case() {
    assert_eq!(ReferenceType::Receipt.as_str(), "receipt");
    assert_eq!(ReferenceType::Act.as_str(), "act");
    assert_eq!(ReferenceType::Verification.as_str(), "verification");
    assert_eq!(ReferenceType::ExternalUrl.as_str(), "external_url");
}

#[test]
fn reference_runx_builds_canonical_scheme_uri() {
    let reference = Reference::runx(ReferenceType::Act, "abc");
    assert_eq!(reference.uri, "runx:act:abc");
    assert_eq!(reference.reference_type, ReferenceType::Act);
    assert!(reference.provider.is_none());
    assert!(reference.locator.is_none());
    assert!(reference.label.is_none());
    assert!(reference.proof_kind.is_none());
}

#[test]
fn reference_with_uri_preserves_explicit_uri() {
    let reference = Reference::with_uri(ReferenceType::Harness, "runx:harness:custom-id");
    assert_eq!(reference.uri, "runx:harness:custom-id");
    assert_eq!(reference.reference_type, ReferenceType::Harness);
    assert!(reference.provider.is_none());
    assert!(reference.proof_kind.is_none());
}
