import type { ExecutionSemantics } from "./runner-local/execution-semantics.js";

export interface RawSkillIR {
  readonly frontmatter: Record<string, unknown>;
  readonly rawFrontmatter: string;
  readonly body: string;
}

export interface RawRunnerManifestIR {
  readonly document: Record<string, unknown>;
  readonly raw: string;
}

export interface RawToolManifestIR {
  readonly document: Record<string, unknown>;
  readonly raw: string;
}

export interface RawGraphIR {
  readonly document: Record<string, unknown>;
}

export interface SkillInput {
  readonly type: string;
  readonly required: boolean;
  readonly description?: string;
  readonly default?: unknown;
}

export interface SkillRetryPolicy {
  readonly maxAttempts: number;
}

export interface SkillIdempotencyPolicy {
  readonly key?: string;
}

export type SkillSandboxProfile = "readonly" | "workspace-write" | "network" | "unrestricted-local-dev";

export interface SkillSandbox {
  readonly profile: SkillSandboxProfile;
  readonly cwdPolicy?: "skill-directory" | "workspace" | "custom";
  readonly envAllowlist?: readonly string[];
  readonly network?: boolean;
  readonly writablePaths: readonly string[];
  readonly requireEnforcement?: boolean;
  readonly approvedEscalation?: boolean;
  readonly raw: Record<string, unknown>;
}

export interface GraphContextEdge {
  readonly input: string;
  readonly fromStep: string;
  readonly output: string;
}

export interface GraphRetryPolicy {
  readonly maxAttempts: number;
  readonly backoffMs?: number;
}

export type FanoutSyncStrategy = "all" | "any" | "quorum";
export type FanoutBranchFailurePolicy = "halt" | "continue";
export type FanoutThresholdAction = "pause" | "escalate";
export type FanoutConflictAction = "pause" | "escalate";

export interface FanoutThresholdGate {
  readonly step: string;
  readonly field: string;
  readonly above: number;
  readonly action: FanoutThresholdAction;
}

export interface FanoutConflictGate {
  readonly field: string;
  readonly steps: readonly string[];
  readonly action: FanoutConflictAction;
}

export interface FanoutGroupPolicy {
  readonly groupId: string;
  readonly strategy: FanoutSyncStrategy;
  readonly minSuccess?: number;
  readonly onBranchFailure: FanoutBranchFailurePolicy;
  readonly thresholdGates: readonly FanoutThresholdGate[];
  readonly conflictGates: readonly FanoutConflictGate[];
}

export interface GraphTransitionGate {
  readonly to: string;
  readonly field: string;
  readonly equals?: unknown;
  readonly notEquals?: unknown;
}

export interface GraphPolicy {
  readonly transitions: readonly GraphTransitionGate[];
}

export interface GraphStep {
  readonly id: string;
  readonly label?: string;
  readonly skill?: string;
  readonly tool?: string;
  readonly run?: Readonly<Record<string, unknown>>;
  readonly instructions?: string;
  readonly artifacts?: Readonly<Record<string, unknown>>;
  readonly runner?: string;
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly context: Readonly<Record<string, string>>;
  readonly contextEdges: readonly GraphContextEdge[];
  readonly scopes: readonly string[];
  readonly allowedTools?: readonly string[];
  readonly retry?: GraphRetryPolicy;
  readonly policy?: Readonly<Record<string, unknown>>;
  readonly fanoutGroup?: string;
  readonly mutating: boolean;
  readonly idempotencyKey?: string;
}

export interface ExecutionGraph {
  readonly name: string;
  readonly owner?: string;
  readonly steps: readonly GraphStep[];
  readonly fanoutGroups: Readonly<Record<string, FanoutGroupPolicy>>;
  readonly policy?: GraphPolicy;
  readonly raw: RawGraphIR;
}

export interface SkillSource {
  readonly type: string;
  readonly command?: string;
  readonly args: readonly string[];
  readonly cwd?: string;
  readonly timeoutSeconds?: number;
  readonly inputMode?: "args" | "stdin" | "none";
  readonly sandbox?: SkillSandbox;
  readonly server?: {
    readonly command: string;
    readonly args: readonly string[];
    readonly cwd?: string;
  };
  readonly catalogRef?: string;
  readonly tool?: string;
  readonly arguments?: Readonly<Record<string, unknown>>;
  readonly agentCardUrl?: string;
  readonly agentIdentity?: string;
  readonly agent?: string;
  readonly task?: string;
  readonly hook?: string;
  readonly outputs?: Readonly<Record<string, unknown>>;
  readonly graph?: ExecutionGraph;
  readonly raw: Record<string, unknown>;
}

export interface SkillArtifactContract {
  readonly emits?: readonly string[];
  readonly namedEmits?: Readonly<Record<string, string>>;
  readonly wrapAs?: string;
}

export interface SkillQualityProfile {
  readonly heading: "Quality Profile";
  readonly content: string;
}

export interface ValidatedSkill {
  readonly name: string;
  readonly description?: string;
  readonly body: string;
  readonly source: SkillSource;
  readonly inputs: Readonly<Record<string, SkillInput>>;
  readonly auth?: unknown;
  readonly risk?: unknown;
  readonly runtime?: unknown;
  readonly retry?: SkillRetryPolicy;
  readonly idempotency?: SkillIdempotencyPolicy;
  readonly mutating?: boolean;
  readonly artifacts?: SkillArtifactContract;
  readonly qualityProfile?: SkillQualityProfile;
  readonly allowedTools?: readonly string[];
  readonly execution?: ExecutionSemantics;
  readonly runx?: Record<string, unknown>;
  readonly raw: RawSkillIR;
}

export interface SkillRunnerDefinition {
  readonly name: string;
  readonly default: boolean;
  readonly source: SkillSource;
  readonly inputs: Readonly<Record<string, SkillInput>>;
  readonly auth?: unknown;
  readonly risk?: unknown;
  readonly runtime?: unknown;
  readonly retry?: SkillRetryPolicy;
  readonly idempotency?: SkillIdempotencyPolicy;
  readonly mutating?: boolean;
  readonly artifacts?: SkillArtifactContract;
  readonly allowedTools?: readonly string[];
  readonly execution?: ExecutionSemantics;
  readonly runx?: Record<string, unknown>;
  readonly raw: Record<string, unknown>;
}

export type PostRunReflectPolicy = "auto" | "always" | "never";

export type CatalogKind = "skill" | "graph";
export type CatalogAudience = "public" | "builder" | "operator";
export type CatalogVisibility = "public" | "private";

export interface CatalogMetadata {
  readonly kind: CatalogKind;
  readonly audience: CatalogAudience;
  readonly visibility: CatalogVisibility;
}

export interface HarnessCallerFixture {
  readonly answers?: Readonly<Record<string, unknown>>;
  readonly approvals?: Readonly<Record<string, boolean>>;
}

export interface ReceiptExpectation {
  readonly [key: string]: unknown;
  readonly schema?: "runx.receipt.v1";
  readonly status?: "sealed" | "failure";
  readonly source_type?: string;
  readonly body_digest?: string;
  readonly receipt_digest?: string;
  readonly harness_id?: string;
  readonly state?: string;
  readonly disposition?: string;
  readonly reason_code?: string;
  readonly child_receipt_refs?: readonly string[];
  readonly act_ids?: readonly string[];
  readonly owner?: string;
}

export interface HarnessExpectation {
  readonly status?: "sealed" | "failure" | "needs_agent" | "policy_denied" | "escalated";
  readonly receipt?: ReceiptExpectation;
  readonly steps?: readonly string[];
}

export interface RunnerHarnessCase {
  readonly name: string;
  readonly runner?: string;
  readonly inputs: Readonly<Record<string, unknown>>;
  readonly env: Readonly<Record<string, string>>;
  readonly caller: HarnessCallerFixture;
  readonly expect: HarnessExpectation;
}

export interface RunnerHarnessManifest {
  readonly cases: readonly RunnerHarnessCase[];
}

export interface SkillRunnerManifest {
  readonly skill?: string;
  readonly catalog?: CatalogMetadata;
  readonly runners: Readonly<Record<string, SkillRunnerDefinition>>;
  readonly harness?: RunnerHarnessManifest;
  readonly raw: RawRunnerManifestIR;
}

export interface ValidatedTool {
  readonly name: string;
  readonly description?: string;
  readonly source: SkillSource;
  readonly inputs: Readonly<Record<string, SkillInput>>;
  readonly scopes: readonly string[];
  readonly risk?: unknown;
  readonly runtime?: unknown;
  readonly retry?: SkillRetryPolicy;
  readonly idempotency?: SkillIdempotencyPolicy;
  readonly mutating?: boolean;
  readonly artifacts?: SkillArtifactContract;
  readonly runx?: Record<string, unknown>;
  readonly raw: RawToolManifestIR;
}

export interface SkillInstallOrigin {
  readonly source: string;
  readonly source_label: string;
  readonly ref: string;
  readonly skill_id?: string;
  readonly version?: string;
  readonly digest?: string;
  readonly profile_digest?: string;
  readonly runner_names?: readonly string[];
  readonly trust_tier?: string;
}
