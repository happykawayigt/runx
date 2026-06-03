export type SkillSearchSource = "runx-registry" | string;
export type SkillSearchTrustTier = "first_party" | "verified" | "community";
export type SkillRunnerMode = "portable" | "profiled";

export interface SkillSearchResult {
  readonly skill_id: string;
  readonly name: string;
  readonly summary?: string;
  readonly owner: string;
  readonly version?: string;
  readonly digest?: string;
  readonly source: SkillSearchSource;
  readonly source_label: string;
  readonly source_type: string;
  readonly trust_tier: SkillSearchTrustTier;
  readonly required_scopes: readonly string[];
  readonly tags: readonly string[];
  readonly profile_mode: SkillRunnerMode;
  readonly runner_names: readonly string[];
  readonly profile_digest?: string;
  readonly profile_trust_tier?: SkillSearchTrustTier;
  readonly trust_signals?: readonly {
    readonly id: string;
    readonly label: string;
    readonly status: string;
    readonly value: string;
  }[];
  readonly add_command: string;
  readonly run_command: string;
}
