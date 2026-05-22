//! Aster operator contracts: targets, opportunities, selections, feed entries, reflections.
use serde::{Deserialize, Serialize};

use crate::{ActForm, ActRef, AuthorityResourceFamily, Closure, Fingerprint, Links, Reference};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetSchema {
    #[serde(rename = "runx.target.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpportunitySchema {
    #[serde(rename = "runx.opportunity.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThesisAssessmentSchema {
    #[serde(rename = "runx.thesis_assessment.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionSchema {
    #[serde(rename = "runx.selection.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillBindingSchema {
    #[serde(rename = "runx.skill_binding.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetTransitionEntrySchema {
    #[serde(rename = "runx.target_transition_entry.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionCycleSchema {
    #[serde(rename = "runx.selection_cycle.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReflectionEntrySchema {
    #[serde(rename = "runx.reflection_entry.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedEntrySchema {
    #[serde(rename = "runx.feed_entry.v1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetLifecycleState {
    Candidate,
    Eligible,
    Active,
    CoolingDown,
    Blocked,
    Retired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetCooldownState {
    None,
    CoolingDown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThesisProofStrength {
    Weak,
    Moderate,
    Strong,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityCostLevel {
    None,
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionCycleState {
    Open,
    Closed,
    Deferred,
    NoAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetCooldown {
    pub state: TargetCooldownState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub schema: TargetSchema,
    pub target_id: String,
    pub target_ref: Reference,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub lifecycle_state: TargetLifecycleState,
    #[serde(default)]
    pub authority_refs: Vec<Reference>,
    pub fingerprint: Fingerprint,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Links>,
    pub cooldown: TargetCooldown,
    #[serde(default)]
    pub verification_recipe_refs: Vec<Reference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owner_refs: Vec<Reference>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Opportunity {
    pub schema: OpportunitySchema,
    pub opportunity_id: String,
    pub target_ref: Reference,
    pub summary: String,
    pub proposed_form: ActForm,
    pub value_score: u32,
    pub risk_score: u32,
    pub freshness_expires_at: String,
    pub fingerprint: Fingerprint,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Links>,
    #[serde(default)]
    pub source_refs: Vec<Reference>,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    pub discovered_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThesisAssessment {
    pub schema: ThesisAssessmentSchema,
    pub assessment_id: String,
    pub target_ref: Reference,
    pub opportunity_ref: Reference,
    pub thesis_ref: Reference,
    pub score: u32,
    #[serde(default)]
    pub rubric_refs: Vec<Reference>,
    pub proof_strength: ThesisProofStrength,
    pub authority_cost: AuthorityCostLevel,
    pub rationale: String,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    pub assessed_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub schema: SelectionSchema,
    pub selection_id: String,
    pub cycle_ref: Reference,
    pub opportunity_ref: Reference,
    #[serde(default)]
    pub candidate_refs: Vec<Reference>,
    pub rank: u32,
    pub score: u32,
    pub selected: bool,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_until: Option<String>,
    pub decision_ref: Option<Reference>,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    pub selected_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillBinding {
    pub schema: SkillBindingSchema,
    pub binding_id: String,
    pub skill_ref: Reference,
    pub scope_family: AuthorityResourceFamily,
    #[serde(default)]
    pub allowed_act_forms: Vec<ActForm>,
    #[serde(default)]
    pub authority_refs: Vec<Reference>,
    #[serde(default)]
    pub policy_refs: Vec<Reference>,
    pub harness_template_ref: Option<Reference>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetTransitionEntry {
    pub schema: TargetTransitionEntrySchema,
    pub entry_id: String,
    pub target_ref: Reference,
    pub from_state: Option<TargetLifecycleState>,
    pub to_state: TargetLifecycleState,
    pub reason_code: String,
    pub summary: String,
    #[serde(default)]
    pub source_refs: Vec<Reference>,
    pub decision_ref: Option<Reference>,
    pub receipt_ref: Option<Reference>,
    pub recorded_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionCycle {
    pub schema: SelectionCycleSchema,
    pub cycle_id: String,
    pub state: SelectionCycleState,
    pub started_at: String,
    pub closed_at: Option<String>,
    #[serde(default)]
    pub input_refs: Vec<Reference>,
    #[serde(default)]
    pub target_refs: Vec<Reference>,
    #[serde(default)]
    pub opportunity_refs: Vec<Reference>,
    #[serde(default)]
    pub ranked_selection_refs: Vec<Reference>,
    pub chosen_selection_ref: Option<Reference>,
    pub decision_ref: Option<Reference>,
    pub receipt_ref: Option<Reference>,
    pub no_action_closure: Option<Closure>,
    pub fingerprint: Fingerprint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReflectionEntry {
    pub schema: ReflectionEntrySchema,
    pub reflection_id: String,
    pub target_ref: Option<Reference>,
    pub opportunity_ref: Option<Reference>,
    pub selection_ref: Option<Reference>,
    pub decision_ref: Option<Reference>,
    #[serde(default)]
    pub receipt_refs: Vec<Reference>,
    #[serde(default)]
    pub act_refs: Vec<ActRef>,
    pub summary: String,
    #[serde(default)]
    pub lessons: Vec<String>,
    #[serde(default)]
    pub follow_up_refs: Vec<Reference>,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    pub recorded_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedEntry {
    pub schema: FeedEntrySchema,
    pub feed_entry_id: String,
    pub public_at: String,
    pub title: String,
    pub summary: String,
    pub target_ref: Option<Reference>,
    pub opportunity_ref: Option<Reference>,
    pub selection_ref: Option<Reference>,
    #[serde(default)]
    pub decision_refs: Vec<Reference>,
    #[serde(default)]
    pub receipt_refs: Vec<Reference>,
    #[serde(default)]
    pub act_refs: Vec<ActRef>,
    #[serde(default)]
    pub verification_refs: Vec<Reference>,
    #[serde(default)]
    pub evidence_refs: Vec<Reference>,
    #[serde(default)]
    pub artifact_refs: Vec<Reference>,
    pub redaction_policy_ref: Reference,
    #[serde(default)]
    pub redaction_refs: Vec<Reference>,
}
