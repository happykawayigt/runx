use std::collections::BTreeSet;

use runx_contracts::{AuthorityVerb, JsonObject, JsonValue};
use runx_core::state_machine::AuthorityAdmissionWitness;

use super::{EffectAdmission, EffectStepRequest, RuntimeEffect, RuntimeEffectError};

pub const PROVIDER_PERMISSION_EFFECT_FAMILY: &str = "provider_permission";

#[derive(Clone, Debug, Default)]
pub struct ProviderPermissionEffect;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderPermissionAdmission {
    pub grant_id: String,
    pub required_scopes: Vec<String>,
    pub granted_scopes: Vec<String>,
}

impl RuntimeEffect for ProviderPermissionEffect {
    fn family(&self) -> &'static str {
        PROVIDER_PERMISSION_EFFECT_FAMILY
    }

    fn admit(
        &self,
        request: EffectStepRequest<'_>,
    ) -> Result<Option<EffectAdmission>, RuntimeEffectError> {
        let Some(policy) = provider_permission_policy(request.step.policy.as_ref()) else {
            return Ok(None);
        };
        let Some(plan) = provider_permission_plan(&request, policy) else {
            return Ok(None);
        };
        if !plan.missing_scopes.is_empty() {
            return Err(provider_permission_denial(&request, &plan));
        }

        let witness = provider_permission_witness(&request, &plan);
        Ok(Some(EffectAdmission::new(
            PROVIDER_PERMISSION_EFFECT_FAMILY,
            plan.verb.clone(),
            witness,
            ProviderPermissionAdmission {
                grant_id: plan.grant_id,
                required_scopes: plan.required_scopes,
                granted_scopes: plan.granted_scopes,
            },
        )))
    }
}

#[derive(Debug)]
struct ProviderPermissionPlan {
    grant_id: String,
    required_scopes: Vec<String>,
    granted_scopes: Vec<String>,
    missing_scopes: Vec<String>,
    verb: AuthorityVerb,
}

fn provider_permission_plan(
    request: &EffectStepRequest<'_>,
    policy: &JsonObject,
) -> Option<ProviderPermissionPlan> {
    let required_scopes = string_array_field(policy, "required_scopes")
        .filter(|scopes| !scopes.is_empty())
        .unwrap_or_else(|| request.step.scopes.clone());
    if required_scopes.is_empty() {
        return None;
    }
    let granted_scopes = string_array_field(policy, "granted_scopes").unwrap_or_default();
    let missing_scopes = missing_scopes(&required_scopes, &granted_scopes);
    let verb = verb_field(policy).unwrap_or_else(|| default_verb(request.step.mutating));
    let grant_id = string_field(policy, "grant_id")
        .unwrap_or("local-provider-grant")
        .to_owned();

    Some(ProviderPermissionPlan {
        grant_id,
        required_scopes,
        granted_scopes,
        missing_scopes,
        verb,
    })
}

fn default_verb(mutating: bool) -> AuthorityVerb {
    if mutating {
        AuthorityVerb::Write
    } else {
        AuthorityVerb::Read
    }
}

fn provider_permission_denial(
    request: &EffectStepRequest<'_>,
    plan: &ProviderPermissionPlan,
) -> RuntimeEffectError {
    RuntimeEffectError::Denied {
        family: PROVIDER_PERMISSION_EFFECT_FAMILY.to_owned(),
        verb: plan.verb.clone(),
        message: format!(
            "step '{}' requires scopes [{}], but grant '{}' only provides [{}]",
            request.step.id,
            plan.required_scopes.join(", "),
            plan.grant_id,
            plan.granted_scopes.join(", ")
        ),
    }
}

fn provider_permission_witness(
    request: &EffectStepRequest<'_>,
    plan: &ProviderPermissionPlan,
) -> AuthorityAdmissionWitness {
    AuthorityAdmissionWitness {
        verb: plan.verb.clone(),
        parent_term_id: format!("provider-permission:{}", plan.grant_id),
        child_term_id: format!(
            "provider-permission:{}:{}",
            request.step.id,
            plan.required_scopes.join("+")
        ),
        idempotency_key: request.step.idempotency_key.clone(),
        capability_ref: None,
    }
}

fn provider_permission_policy(policy: Option<&JsonObject>) -> Option<&JsonObject> {
    policy?
        .get(PROVIDER_PERMISSION_EFFECT_FAMILY)
        .and_then(JsonValue::as_object)
}

fn string_field<'a>(object: &'a JsonObject, key: &str) -> Option<&'a str> {
    object.get(key).and_then(JsonValue::as_str)
}

fn string_array_field(object: &JsonObject, key: &str) -> Option<Vec<String>> {
    Some(
        object
            .get(key)?
            .as_array()?
            .iter()
            .filter_map(JsonValue::as_str)
            .map(str::to_owned)
            .collect(),
    )
}

fn verb_field(object: &JsonObject) -> Option<AuthorityVerb> {
    match string_field(object, "verb")? {
        "read" => Some(AuthorityVerb::Read),
        "write" => Some(AuthorityVerb::Write),
        "comment" => Some(AuthorityVerb::Comment),
        "review" => Some(AuthorityVerb::Review),
        "merge" => Some(AuthorityVerb::Merge),
        "create" => Some(AuthorityVerb::Create),
        "update" => Some(AuthorityVerb::Update),
        "delete" => Some(AuthorityVerb::Delete),
        "execute" => Some(AuthorityVerb::Execute),
        _ => None,
    }
}

fn missing_scopes(required: &[String], granted: &[String]) -> Vec<String> {
    let granted = granted.iter().collect::<BTreeSet<_>>();
    required
        .iter()
        .filter(|scope| !granted.contains(scope))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io;
    use std::path::Path;

    use runx_contracts::{JsonObject, JsonValue};
    use runx_parser::GraphStep;

    use super::*;

    #[test]
    fn admits_when_required_scopes_are_granted() -> Result<(), io::Error> {
        let effect = ProviderPermissionEffect;
        let step = test_step(
            "read_issue",
            vec!["repo.read"],
            vec!["repo.read"],
            false,
            "read",
        );
        let inputs = JsonObject::new();
        let env = BTreeMap::new();

        let result = effect.admit(EffectStepRequest {
            step: &step,
            inputs: &inputs,
            env: &env,
            graph_dir: Path::new("."),
        });
        let admission = match result {
            Ok(Some(admission)) => admission,
            other => {
                return Err(io::Error::other(format!(
                    "unexpected provider permission admission: {other:?}"
                )));
            }
        };

        assert_eq!(admission.family(), PROVIDER_PERMISSION_EFFECT_FAMILY);
        assert_eq!(admission.verb(), AuthorityVerb::Read);
        let context = match admission.context::<ProviderPermissionAdmission>() {
            Some(context) => context,
            None => {
                return Err(io::Error::other(
                    "missing provider permission admission context",
                ));
            }
        };
        assert_eq!(context.required_scopes, vec!["repo.read"]);
        assert_eq!(context.granted_scopes, vec!["repo.read"]);
        Ok(())
    }

    #[test]
    fn denies_when_required_scope_is_not_granted() -> Result<(), io::Error> {
        let effect = ProviderPermissionEffect;
        let step = test_step(
            "comment_issue",
            vec!["repo.write"],
            vec!["repo.read"],
            true,
            "write",
        );
        let inputs = JsonObject::new();
        let env = BTreeMap::new();

        let result = effect.admit(EffectStepRequest {
            step: &step,
            inputs: &inputs,
            env: &env,
            graph_dir: Path::new("."),
        });
        let error = match result {
            Err(error) => error,
            other => {
                return Err(io::Error::other(format!(
                    "unexpected provider permission result: {other:?}"
                )));
            }
        };

        match error {
            RuntimeEffectError::Denied {
                family,
                verb: AuthorityVerb::Write,
                message,
            } if family == PROVIDER_PERMISSION_EFFECT_FAMILY
                && message.contains("repo.write")
                && message.contains("repo.read") =>
            {
                Ok(())
            }
            other => Err(io::Error::other(format!(
                "unexpected denial error: {other:?}"
            ))),
        }
    }

    fn test_step(
        id: &str,
        required_scopes: Vec<&str>,
        granted_scopes: Vec<&str>,
        mutating: bool,
        verb: &str,
    ) -> GraphStep {
        let mut permission = JsonObject::new();
        permission.insert(
            "grant_id".to_owned(),
            JsonValue::String("github-mcp-read".to_owned()),
        );
        permission.insert("verb".to_owned(), JsonValue::String(verb.to_owned()));
        permission.insert(
            "granted_scopes".to_owned(),
            JsonValue::Array(
                granted_scopes
                    .into_iter()
                    .map(|scope| JsonValue::String(scope.to_owned()))
                    .collect(),
            ),
        );
        let mut policy = JsonObject::new();
        policy.insert(
            PROVIDER_PERMISSION_EFFECT_FAMILY.to_owned(),
            JsonValue::Object(permission),
        );
        GraphStep {
            id: id.to_owned(),
            label: None,
            skill: None,
            tool: None,
            run: None,
            instructions: None,
            artifacts: None,
            runner: None,
            inputs: JsonObject::new(),
            context: BTreeMap::new(),
            context_edges: Vec::new(),
            context_skills: Vec::new(),
            scopes: required_scopes
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            allowed_tools: None,
            retry: None,
            policy: Some(policy),
            fanout_group: None,
            mutating,
            idempotency_key: Some(format!("{id}-key")),
        }
    }
}
