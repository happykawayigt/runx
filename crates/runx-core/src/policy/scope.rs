use std::collections::BTreeSet;

/// Whether `granted_scope` covers `requested_scope`.
///
/// The universal `*` grant is gated behind `allow_universal_wildcard`: callers
/// must pass `true` only when the granting source is trusted (e.g. first-party
/// scope propagation), never for untrusted/connected provider grants. Exact and
/// `prefix:*` grants match exactly one scope segment under `prefix:`. For
/// example, `repo:*` covers `repo:read` but not `repo:admin:keys`.
pub(crate) fn scope_allows(
    granted_scope: &str,
    requested_scope: &str,
    allow_universal_wildcard: bool,
) -> bool {
    if granted_scope == "*" {
        return allow_universal_wildcard;
    }
    if granted_scope == requested_scope {
        return true;
    }

    granted_scope
        .strip_suffix('*')
        .filter(|prefix| prefix.ends_with(':'))
        .and_then(|prefix| requested_scope.strip_prefix(prefix))
        .is_some_and(|suffix| !suffix.is_empty() && !suffix.contains(':'))
}

pub(crate) fn unique_strings(values: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();

    for value in values {
        if seen.insert(value.clone()) {
            unique.push(value.clone());
        }
    }

    unique
}

#[cfg(test)]
mod tests {
    use super::{scope_allows, unique_strings};

    #[test]
    fn universal_wildcard_requires_trust() {
        assert!(scope_allows("*", "repo:read", true));
        assert!(!scope_allows("*", "repo:read", false));
    }

    #[test]
    fn prefix_wildcard_allows_strict_prefix_matches() {
        assert!(scope_allows("repo:*", "repo:read", false));
        assert!(!scope_allows("repo:*", "repo:admin:keys", false));
        assert!(!scope_allows("repo:*", "deploy:prod", false));
        assert!(!scope_allows("repo:*", "repository:read", false));
        assert!(!scope_allows(":*", "repo:read", false));
    }

    #[test]
    fn unique_strings_preserves_first_seen_order() {
        let values = vec![
            "repo:read".to_owned(),
            "repo:write".to_owned(),
            "repo:read".to_owned(),
        ];

        assert_eq!(unique_strings(&values), vec!["repo:read", "repo:write"]);
    }
}
