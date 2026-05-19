//! Operational policy contracts for governed source, runner, target, and owner routing.
// rust-style-allow: large-file - this mirrors the TS operational policy surface in one parity module.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OperationalPolicySchema {
    #[serde(rename = "runx.operational_policy.v1")]
    V1,
}

impl fmt::Display for OperationalPolicySchema {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("runx.operational_policy.v1")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationalPolicyAction {
    ReplyOnly,
    IssueIntake,
    WorkPlan,
    IssueToPr,
    ManualReview,
    PrReview,
    PrFixUp,
    MergeAssist,
}

impl fmt::Display for OperationalPolicyAction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(action_name(*self))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationalPolicySourceProvider {
    Slack,
    Sentry,
    Github,
    File,
    Api,
    Other,
}

impl fmt::Display for OperationalPolicySourceProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Slack => "slack",
            Self::Sentry => "sentry",
            Self::Github => "github",
            Self::File => "file",
            Self::Api => "api",
            Self::Other => "other",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationalPolicyRunnerKind {
    Local,
    GithubActions,
    Aster,
    Other,
}

impl fmt::Display for OperationalPolicyRunnerKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Local => "local",
            Self::GithubActions => "github-actions",
            Self::Aster => "aster",
            Self::Other => "other",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationalPolicyRunnerState {
    Available,
    Disabled,
    Maintenance,
}

impl fmt::Display for OperationalPolicyRunnerState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Available => "available",
            Self::Disabled => "disabled",
            Self::Maintenance => "maintenance",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalPolicyDedupeStrategy {
    SourceFingerprint,
    ProviderSearch,
    Branch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalPolicyOutcomeCloseMode {
    Never,
    WhenVerified,
    WhenTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationalPolicyPublishMode {
    Reply,
    Comment,
    None,
}

impl fmt::Display for OperationalPolicyPublishMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Reply => "reply",
            Self::Comment => "comment",
            Self::None => "none",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OperationalPolicyMissingBehavior {
    #[serde(rename = "fail_closed")]
    FailClosed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationalPolicyDuplicateBehavior {
    Reuse,
    Comment,
    Block,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPolicy {
    pub schema: OperationalPolicySchema,
    pub schema_version: OperationalPolicySchema,
    pub policy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    pub sources: Vec<OperationalPolicySourceRule>,
    pub runners: Vec<OperationalPolicyRunnerRule>,
    pub owner_routes: Vec<OperationalPolicyOwnerRoute>,
    pub targets: Vec<OperationalPolicyTargetRule>,
    pub dedupe: OperationalPolicyDedupePolicy,
    pub outcomes: OperationalPolicyOutcomePolicy,
    pub permissions: OperationalPolicyAutomationPermissions,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPolicySourceRule {
    pub source_id: String,
    pub provider: OperationalPolicySourceProvider,
    pub allowed_locators: Vec<String>,
    pub allowed_actions: Vec<OperationalPolicyAction>,
    pub source_thread: OperationalPolicySourceThreadPolicy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentry: Option<OperationalPolicySentryPolicy>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPolicySourceThreadPolicy {
    pub required: bool,
    pub publish_mode: OperationalPolicyPublishMode,
    pub missing_behavior: OperationalPolicyMissingBehavior,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPolicySentryPolicy {
    pub production_only: bool,
    pub unresolved_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regressed_only: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPolicyRunnerRule {
    pub runner_id: String,
    pub kind: OperationalPolicyRunnerKind,
    pub state: OperationalPolicyRunnerState,
    pub allowed_actions: Vec<OperationalPolicyAction>,
    pub target_repos: Vec<String>,
    pub scafld_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPolicyOwnerRoute {
    pub route_id: String,
    pub owners: Vec<String>,
    pub target_repos: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPolicyTargetRule {
    pub repo: String,
    pub runner_ids: Vec<String>,
    pub allowed_actions: Vec<OperationalPolicyAction>,
    pub default_owner_route: String,
    pub scafld_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPolicyDedupePolicy {
    pub strategy: OperationalPolicyDedupeStrategy,
    pub key_fields: Vec<String>,
    pub on_duplicate: OperationalPolicyDuplicateBehavior,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPolicyOutcomePolicy {
    pub observe_provider: bool,
    pub verification_required: bool,
    pub close_source_issue: OperationalPolicyOutcomeCloseMode,
    pub publish_final_source_thread_update: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPolicyAutomationPermissions {
    pub auto_merge: bool,
    pub mutate_target_repo: bool,
    pub require_human_merge_gate: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OperationalPolicyValidationFinding {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationalPolicyError {
    Contract(OperationalPolicyValidationFinding),
    Semantic(OperationalPolicyValidationFinding),
}

impl OperationalPolicyError {
    pub fn finding(&self) -> &OperationalPolicyValidationFinding {
        match self {
            Self::Contract(finding) | Self::Semantic(finding) => finding,
        }
    }
}

impl fmt::Display for OperationalPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let finding = self.finding();
        write!(
            formatter,
            "{} failed validation ({}): {}",
            finding.path, finding.code, finding.message
        )
    }
}

impl std::error::Error for OperationalPolicyError {}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OperationalPolicyReadback {
    pub policy_id: String,
    pub schema_version: OperationalPolicySchema,
    pub valid: bool,
    pub findings: Vec<OperationalPolicyValidationFinding>,
    pub sources: Vec<OperationalPolicySourceReadback>,
    pub runners: Vec<OperationalPolicyRunnerReadback>,
    pub targets: Vec<OperationalPolicyTargetReadback>,
    pub outcomes: OperationalPolicyOutcomePolicy,
    pub permissions: OperationalPolicyAutomationPermissions,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OperationalPolicySourceReadback {
    pub source_id: String,
    pub provider: OperationalPolicySourceProvider,
    pub locator_count: usize,
    pub allowed_actions: Vec<OperationalPolicyAction>,
    pub source_thread_required: bool,
    pub publish_mode: OperationalPolicyPublishMode,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationalPolicyRunnerReadback {
    pub runner_id: String,
    pub kind: OperationalPolicyRunnerKind,
    pub state: OperationalPolicyRunnerState,
    pub target_repos: Vec<String>,
    pub allowed_actions: Vec<OperationalPolicyAction>,
    pub scafld_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationalPolicyTargetReadback {
    pub repo: String,
    pub runner_ids: Vec<String>,
    pub default_owner_route: String,
    pub owner_count: usize,
    pub allowed_actions: Vec<OperationalPolicyAction>,
    pub scafld_required: bool,
    pub available_runner_count: usize,
}

pub fn validate_operational_policy_contract(
    policy: &OperationalPolicy,
) -> Result<(), OperationalPolicyError> {
    validate_required_shape(policy).map_err(OperationalPolicyError::Contract)
}

pub fn lint_operational_policy_contract(
    policy: &OperationalPolicy,
) -> Result<Vec<OperationalPolicyValidationFinding>, OperationalPolicyError> {
    validate_operational_policy_contract(policy)?;
    Ok(collect_semantic_findings(policy))
}

pub fn validate_operational_policy_semantics(
    policy: &OperationalPolicy,
) -> Result<(), OperationalPolicyError> {
    let findings = lint_operational_policy_contract(policy)?;
    if let Some(finding) = findings.into_iter().next() {
        return Err(OperationalPolicyError::Semantic(finding));
    }
    Ok(())
}

pub fn project_operational_policy_readback(
    policy: &OperationalPolicy,
) -> Result<OperationalPolicyReadback, OperationalPolicyError> {
    let findings = lint_operational_policy_contract(policy)?;
    Ok(OperationalPolicyReadback {
        policy_id: policy.policy_id.clone(),
        schema_version: policy.schema_version,
        valid: findings.is_empty(),
        findings,
        sources: policy.sources.iter().map(source_readback).collect(),
        runners: policy.runners.iter().map(runner_readback).collect(),
        targets: policy
            .targets
            .iter()
            .map(|target| target_readback(policy, target))
            .collect(),
        outcomes: policy.outcomes.clone(),
        permissions: policy.permissions.clone(),
    })
}

fn source_readback(source: &OperationalPolicySourceRule) -> OperationalPolicySourceReadback {
    OperationalPolicySourceReadback {
        source_id: source.source_id.clone(),
        provider: source.provider,
        locator_count: source.allowed_locators.len(),
        allowed_actions: source.allowed_actions.clone(),
        source_thread_required: source.source_thread.required,
        publish_mode: source.source_thread.publish_mode,
    }
}

fn runner_readback(runner: &OperationalPolicyRunnerRule) -> OperationalPolicyRunnerReadback {
    OperationalPolicyRunnerReadback {
        runner_id: runner.runner_id.clone(),
        kind: runner.kind,
        state: runner.state,
        target_repos: runner.target_repos.clone(),
        allowed_actions: runner.allowed_actions.clone(),
        scafld_required: runner.scafld_required,
    }
}

fn target_readback(
    policy: &OperationalPolicy,
    target: &OperationalPolicyTargetRule,
) -> OperationalPolicyTargetReadback {
    let owner_count = policy
        .owner_routes
        .iter()
        .find(|route| route.route_id == target.default_owner_route)
        .map_or(0, |route| route.owners.len());
    let available_runner_count = target
        .runner_ids
        .iter()
        .filter_map(|runner_id| {
            policy
                .runners
                .iter()
                .find(|runner| &runner.runner_id == runner_id)
        })
        .filter(|runner| runner.state == OperationalPolicyRunnerState::Available)
        .count();

    OperationalPolicyTargetReadback {
        repo: target.repo.clone(),
        runner_ids: target.runner_ids.clone(),
        default_owner_route: target.default_owner_route.clone(),
        owner_count,
        allowed_actions: target.allowed_actions.clone(),
        scafld_required: target.scafld_required,
        available_runner_count,
    }
}

fn validate_required_shape(
    policy: &OperationalPolicy,
) -> Result<(), OperationalPolicyValidationFinding> {
    require_id(&policy.policy_id, "/policy_id", "policy_id")?;
    require_optional_date_time(&policy.created_at, "/created_at")?;
    require_non_empty(&policy.sources, "/sources", "sources")?;
    require_non_empty(&policy.runners, "/runners", "runners")?;
    require_non_empty(&policy.owner_routes, "/owner_routes", "owner_routes")?;
    require_non_empty(&policy.targets, "/targets", "targets")?;
    validate_sources(&policy.sources)?;
    validate_runners(&policy.runners)?;
    validate_owner_routes(&policy.owner_routes)?;
    validate_targets(&policy.targets)?;
    validate_dedupe(&policy.dedupe)?;
    validate_permissions(&policy.permissions)?;
    Ok(())
}

fn validate_sources(
    sources: &[OperationalPolicySourceRule],
) -> Result<(), OperationalPolicyValidationFinding> {
    for (index, source) in sources.iter().enumerate() {
        require_id(
            &source.source_id,
            &format!("/sources/{index}/source_id"),
            "source_id",
        )?;
        require_string_items(
            &source.allowed_locators,
            &format!("/sources/{index}/allowed_locators"),
            "allowed_locators",
        )?;
        require_non_empty(
            &source.allowed_actions,
            &format!("/sources/{index}/allowed_actions"),
            "allowed_actions",
        )?;
        if let Some(confidence) = source.minimum_confidence {
            require_unit_interval(
                confidence,
                &format!("/sources/{index}/minimum_confidence"),
                "minimum_confidence",
            )?;
        }
    }
    Ok(())
}

fn validate_runners(
    runners: &[OperationalPolicyRunnerRule],
) -> Result<(), OperationalPolicyValidationFinding> {
    for (index, runner) in runners.iter().enumerate() {
        require_id(
            &runner.runner_id,
            &format!("/runners/{index}/runner_id"),
            "runner_id",
        )?;
        require_non_empty(
            &runner.allowed_actions,
            &format!("/runners/{index}/allowed_actions"),
            "allowed_actions",
        )?;
        require_repo_items(
            &runner.target_repos,
            &format!("/runners/{index}/target_repos"),
            "target_repos",
        )?;
    }
    Ok(())
}

fn validate_owner_routes(
    routes: &[OperationalPolicyOwnerRoute],
) -> Result<(), OperationalPolicyValidationFinding> {
    for (index, route) in routes.iter().enumerate() {
        require_id(
            &route.route_id,
            &format!("/owner_routes/{index}/route_id"),
            "route_id",
        )?;
        require_string_items(
            &route.owners,
            &format!("/owner_routes/{index}/owners"),
            "owners",
        )?;
        require_repo_items(
            &route.target_repos,
            &format!("/owner_routes/{index}/target_repos"),
            "target_repos",
        )?;
        require_optional_string(&route.project, &format!("/owner_routes/{index}/project"))?;
        require_string_items_if_present(&route.labels, &format!("/owner_routes/{index}/labels"))?;
    }
    Ok(())
}

fn validate_targets(
    targets: &[OperationalPolicyTargetRule],
) -> Result<(), OperationalPolicyValidationFinding> {
    for (index, target) in targets.iter().enumerate() {
        require_repo_slug(&target.repo, &format!("/targets/{index}/repo"))?;
        require_string_items(
            &target.runner_ids,
            &format!("/targets/{index}/runner_ids"),
            "runner_ids",
        )?;
        require_non_empty(
            &target.allowed_actions,
            &format!("/targets/{index}/allowed_actions"),
            "allowed_actions",
        )?;
        require_id(
            &target.default_owner_route,
            &format!("/targets/{index}/default_owner_route"),
            "default_owner_route",
        )?;
        require_optional_string(
            &target.base_branch,
            &format!("/targets/{index}/base_branch"),
        )?;
    }
    Ok(())
}

fn validate_dedupe(
    dedupe: &OperationalPolicyDedupePolicy,
) -> Result<(), OperationalPolicyValidationFinding> {
    require_string_items(&dedupe.key_fields, "/dedupe/key_fields", "key_fields")
}

fn validate_permissions(
    permissions: &OperationalPolicyAutomationPermissions,
) -> Result<(), OperationalPolicyValidationFinding> {
    if permissions.auto_merge {
        return Err(finding(
            "literal_false",
            "/permissions/auto_merge",
            "permissions.auto_merge must be false.",
        ));
    }
    if !permissions.require_human_merge_gate {
        return Err(finding(
            "literal_true",
            "/permissions/require_human_merge_gate",
            "permissions.require_human_merge_gate must be true.",
        ));
    }
    Ok(())
}

fn collect_semantic_findings(
    policy: &OperationalPolicy,
) -> Vec<OperationalPolicyValidationFinding> {
    let mut findings = Vec::new();
    collect_duplicates(policy, &mut findings);
    collect_source_findings(policy, &mut findings);
    collect_target_findings(policy, &mut findings);
    collect_outcome_findings(policy, &mut findings);
    findings
}

fn collect_duplicates(
    policy: &OperationalPolicy,
    findings: &mut Vec<OperationalPolicyValidationFinding>,
) {
    duplicate_findings(
        policy
            .sources
            .iter()
            .map(|source| source.source_id.as_str()),
        "sources",
        "source_id",
        findings,
    );
    duplicate_findings(
        policy
            .runners
            .iter()
            .map(|runner| runner.runner_id.as_str()),
        "runners",
        "runner_id",
        findings,
    );
    duplicate_findings(
        policy
            .owner_routes
            .iter()
            .map(|route| route.route_id.as_str()),
        "owner_routes",
        "route_id",
        findings,
    );
    duplicate_findings(
        policy.targets.iter().map(|target| target.repo.as_str()),
        "targets",
        "repo",
        findings,
    );
}

fn collect_source_findings(
    policy: &OperationalPolicy,
    findings: &mut Vec<OperationalPolicyValidationFinding>,
) {
    for (source_index, source) in policy.sources.iter().enumerate() {
        let automates_issue_or_pr = source.allowed_actions.iter().any(|action| {
            matches!(
                action,
                OperationalPolicyAction::IssueToPr
                    | OperationalPolicyAction::PrFixUp
                    | OperationalPolicyAction::MergeAssist
            )
        });
        if automates_issue_or_pr
            && (!source.source_thread.required
                || source.source_thread.publish_mode == OperationalPolicyPublishMode::None)
        {
            findings.push(finding(
                "source_thread_required",
                &format!("/sources/{source_index}/source_thread"),
                &format!(
                    "source '{}' allows issue/PR automation but does not require source-thread publishing.",
                    source.source_id
                ),
            ));
        }
    }
}

fn collect_target_findings(
    policy: &OperationalPolicy,
    findings: &mut Vec<OperationalPolicyValidationFinding>,
) {
    let runner_ids = policy
        .runners
        .iter()
        .map(|runner| runner.runner_id.as_str())
        .collect::<BTreeSet<_>>();
    let owner_route_ids = policy
        .owner_routes
        .iter()
        .map(|route| route.route_id.as_str())
        .collect::<BTreeSet<_>>();

    for (target_index, target) in policy.targets.iter().enumerate() {
        collect_owner_route_findings(policy, target, target_index, &owner_route_ids, findings);
        collect_runner_findings(policy, target, target_index, &runner_ids, findings);
    }
}

fn collect_owner_route_findings(
    policy: &OperationalPolicy,
    target: &OperationalPolicyTargetRule,
    target_index: usize,
    owner_route_ids: &BTreeSet<&str>,
    findings: &mut Vec<OperationalPolicyValidationFinding>,
) {
    if !owner_route_ids.contains(target.default_owner_route.as_str()) {
        findings.push(finding(
            "unknown_owner_route",
            &format!("/targets/{target_index}/default_owner_route"),
            &format!(
                "target '{}' references unknown owner route '{}'.",
                target.repo, target.default_owner_route
            ),
        ));
    }
    let owner_route = policy
        .owner_routes
        .iter()
        .find(|route| route.route_id == target.default_owner_route);
    if owner_route.is_some_and(|route| !route.target_repos.contains(&target.repo)) {
        findings.push(finding(
            "owner_route_target_mismatch",
            &format!("/targets/{target_index}/default_owner_route"),
            &format!(
                "owner route '{}' does not cover target repo '{}'.",
                target.default_owner_route, target.repo
            ),
        ));
    }
}

fn collect_runner_findings(
    policy: &OperationalPolicy,
    target: &OperationalPolicyTargetRule,
    target_index: usize,
    runner_ids: &BTreeSet<&str>,
    findings: &mut Vec<OperationalPolicyValidationFinding>,
) {
    let mut coverage = target
        .allowed_actions
        .iter()
        .map(|action| (*action, false))
        .collect::<BTreeMap<_, _>>();

    for (runner_index, runner_id) in target.runner_ids.iter().enumerate() {
        let runner = policy
            .runners
            .iter()
            .find(|runner| runner.runner_id == *runner_id);
        if !runner_ids.contains(runner_id.as_str()) {
            findings.push(finding(
                "unknown_runner",
                &format!("/targets/{target_index}/runner_ids/{runner_index}"),
                &format!(
                    "target '{}' references unknown runner '{}'.",
                    target.repo, runner_id
                ),
            ));
            continue;
        }
        if let Some(runner) = runner {
            collect_runner_target_findings(target, target_index, runner_index, runner, findings);
            mark_action_coverage(target, runner, &mut coverage);
        }
    }
    collect_action_coverage_findings(target, target_index, coverage, findings);
}

fn collect_runner_target_findings(
    target: &OperationalPolicyTargetRule,
    target_index: usize,
    runner_index: usize,
    runner: &OperationalPolicyRunnerRule,
    findings: &mut Vec<OperationalPolicyValidationFinding>,
) {
    if !runner.target_repos.contains(&target.repo) {
        findings.push(finding(
            "runner_target_mismatch",
            &format!("/targets/{target_index}/runner_ids/{runner_index}"),
            &format!(
                "runner '{}' does not allow target repo '{}'.",
                runner.runner_id, target.repo
            ),
        ));
    }
    if target.scafld_required && !runner.scafld_required {
        findings.push(finding(
            "runner_scafld_mismatch",
            &format!("/targets/{target_index}/runner_ids/{runner_index}"),
            &format!(
                "target '{}' requires scafld but runner '{}' does not.",
                target.repo, runner.runner_id
            ),
        ));
    }
}

fn mark_action_coverage(
    target: &OperationalPolicyTargetRule,
    runner: &OperationalPolicyRunnerRule,
    coverage: &mut BTreeMap<OperationalPolicyAction, bool>,
) {
    if runner.state != OperationalPolicyRunnerState::Available {
        return;
    }
    for action in &target.allowed_actions {
        if runner.allowed_actions.contains(action) {
            coverage.insert(*action, true);
        }
    }
}

fn collect_action_coverage_findings(
    target: &OperationalPolicyTargetRule,
    target_index: usize,
    coverage: BTreeMap<OperationalPolicyAction, bool>,
    findings: &mut Vec<OperationalPolicyValidationFinding>,
) {
    for (action, covered) in coverage {
        if !covered {
            findings.push(finding(
                "target_action_without_runner",
                &format!("/targets/{target_index}/allowed_actions"),
                &format!(
                    "target '{}' allows '{}' but no available runner supports it.",
                    target.repo,
                    action_name(action)
                ),
            ));
        }
    }
}

fn collect_outcome_findings(
    policy: &OperationalPolicy,
    findings: &mut Vec<OperationalPolicyValidationFinding>,
) {
    if policy.outcomes.publish_final_source_thread_update
        && !policy
            .sources
            .iter()
            .any(|source| source.source_thread.required)
    {
        findings.push(finding(
            "outcome_without_source_thread",
            "/outcomes/publish_final_source_thread_update",
            "final source-thread updates require at least one source with source_thread.required=true.",
        ));
    }
    if policy.outcomes.close_source_issue == OperationalPolicyOutcomeCloseMode::WhenVerified
        && !policy.outcomes.verification_required
    {
        findings.push(finding(
            "close_without_verification",
            "/outcomes/close_source_issue",
            "close_source_issue=when_verified requires verification_required=true.",
        ));
    }
    if policy.permissions.mutate_target_repo
        && policy.targets.iter().any(|target| !target.scafld_required)
    {
        findings.push(finding(
            "mutation_without_scafld",
            "/permissions/mutate_target_repo",
            "mutating target repo policy requires every target to set scafld_required=true.",
        ));
    }
}

fn duplicate_findings<'a>(
    ids: impl Iterator<Item = &'a str>,
    collection_name: &str,
    field_name: &str,
    findings: &mut Vec<OperationalPolicyValidationFinding>,
) {
    let mut seen = BTreeSet::new();
    for (index, id) in ids.enumerate() {
        if seen.insert(id) {
            continue;
        }
        findings.push(finding(
            "duplicate_id",
            &format!("/{collection_name}/{index}/{field_name}"),
            &format!("{collection_name}.{field_name} '{id}' must be unique."),
        ));
    }
}

fn require_id(
    value: &str,
    path: &str,
    field: &str,
) -> Result<(), OperationalPolicyValidationFinding> {
    if !value.is_empty() && value.chars().all(is_id_char) {
        return Ok(());
    }
    Err(finding(
        "invalid_id",
        path,
        &format!("{field} must match ^[A-Za-z0-9_.:-]+$."),
    ))
}

fn require_repo_slug(value: &str, path: &str) -> Result<(), OperationalPolicyValidationFinding> {
    let mut parts = value.split('/');
    let owner = parts.next();
    let repo = parts.next();
    if parts.next().is_none()
        && owner.is_some_and(valid_repo_part)
        && repo.is_some_and(valid_repo_part)
    {
        return Ok(());
    }
    Err(finding(
        "invalid_repo",
        path,
        "repo must match owner/repo with non-empty slug parts.",
    ))
}

fn require_repo_items(
    values: &[String],
    path: &str,
    field: &str,
) -> Result<(), OperationalPolicyValidationFinding> {
    require_non_empty(values, path, field)?;
    for (index, value) in values.iter().enumerate() {
        require_repo_slug(value, &format!("{path}/{index}"))?;
    }
    Ok(())
}

fn require_string_items(
    values: &[String],
    path: &str,
    field: &str,
) -> Result<(), OperationalPolicyValidationFinding> {
    require_non_empty(values, path, field)?;
    require_string_items_if_present(values, path)
}

fn require_string_items_if_present(
    values: &[String],
    path: &str,
) -> Result<(), OperationalPolicyValidationFinding> {
    for (index, value) in values.iter().enumerate() {
        if value.is_empty() {
            return Err(finding(
                "empty_string",
                &format!("{path}/{index}"),
                "string entries must not be empty.",
            ));
        }
    }
    Ok(())
}

fn require_optional_string(
    value: &Option<String>,
    path: &str,
) -> Result<(), OperationalPolicyValidationFinding> {
    if value.as_ref().is_some_and(String::is_empty) {
        return Err(finding("empty_string", path, "value must not be empty."));
    }
    Ok(())
}

fn require_optional_date_time(
    value: &Option<String>,
    path: &str,
) -> Result<(), OperationalPolicyValidationFinding> {
    match value.as_deref() {
        Some(value) if !matches_ts_date_time_pattern(value) => Err(finding(
            "date_time",
            path,
            "value must match YYYY-MM-DDTHH:MM:SS(.fraction)?Z.",
        )),
        _ => Ok(()),
    }
}

fn matches_ts_date_time_pattern(value: &str) -> bool {
    let Some(prefix) = value.strip_suffix('Z') else {
        return false;
    };
    let Some((seconds_prefix, fraction)) = prefix.split_once('.') else {
        return matches_date_time_without_zone(prefix);
    };
    matches_date_time_without_zone(seconds_prefix)
        && !fraction.is_empty()
        && fraction.chars().all(|character| character.is_ascii_digit())
}

fn matches_date_time_without_zone(value: &str) -> bool {
    value.len() == 19
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value.as_bytes().get(10) == Some(&b'T')
        && value.as_bytes().get(13) == Some(&b':')
        && value.as_bytes().get(16) == Some(&b':')
        && value.chars().enumerate().all(|(index, character)| {
            matches!(index, 4 | 7 | 10 | 13 | 16) || character.is_ascii_digit()
        })
}

fn require_non_empty<T>(
    values: &[T],
    path: &str,
    field: &str,
) -> Result<(), OperationalPolicyValidationFinding> {
    if values.is_empty() {
        return Err(finding(
            "min_items",
            path,
            &format!("{field} must contain at least one entry."),
        ));
    }
    Ok(())
}

fn require_unit_interval(
    value: f64,
    path: &str,
    field: &str,
) -> Result<(), OperationalPolicyValidationFinding> {
    if (0.0..=1.0).contains(&value) {
        return Ok(());
    }
    Err(finding(
        "range",
        path,
        &format!("{field} must be between 0 and 1."),
    ))
}

fn valid_repo_part(value: &str) -> bool {
    !value.is_empty() && value.chars().all(is_repo_char)
}

fn is_id_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | ':' | '-')
}

fn is_repo_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
}

fn action_name(action: OperationalPolicyAction) -> &'static str {
    match action {
        OperationalPolicyAction::ReplyOnly => "reply-only",
        OperationalPolicyAction::IssueIntake => "issue-intake",
        OperationalPolicyAction::WorkPlan => "work-plan",
        OperationalPolicyAction::IssueToPr => "issue-to-pr",
        OperationalPolicyAction::ManualReview => "manual-review",
        OperationalPolicyAction::PrReview => "pr-review",
        OperationalPolicyAction::PrFixUp => "pr-fix-up",
        OperationalPolicyAction::MergeAssist => "merge-assist",
    }
}

fn finding(code: &str, path: &str, message: &str) -> OperationalPolicyValidationFinding {
    OperationalPolicyValidationFinding {
        code: code.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    }
}
