import {
  authorityProofSchemaVersion,
  validateAuthorityProofContract,
  type AuthorityProofContract,
  type CredentialEnvelopeContract,
  type ScopeAdmissionContract,
} from "@runxhq/contracts";

import { hashString } from "../util/hash.js";
import { isRecord } from "../util/types.js";

export interface ConnectedAuthRequirement {
  readonly provider: string;
  readonly scopes: readonly string[];
  readonly scope_family?: string;
  readonly authority_kind?: "read_only" | "constructive" | "destructive";
  readonly target_repo?: string;
  readonly target_locator?: string;
}

export interface AuthorityProofGrant {
  readonly grant_id: string;
  readonly provider: string;
  readonly scopes: readonly string[];
  readonly status?: "active" | "revoked";
  readonly scope_family?: string;
  readonly authority_kind?: "read_only" | "constructive" | "destructive";
  readonly target_repo?: string;
  readonly target_locator?: string;
}

export interface AuthorityProofSandboxDeclaration {
  readonly profile?: string;
  readonly cwdPolicy?: string;
  readonly cwd_policy?: string;
  readonly network?: boolean;
  readonly requireEnforcement?: boolean;
  readonly require_enforcement?: boolean;
}

export interface AuthorityProofApproval {
  readonly gate: {
    readonly id: string;
    readonly type?: string;
    readonly reason?: string;
  };
  readonly approved: boolean;
}

export interface BuildAuthorityProofOptions {
  readonly runId?: string;
  readonly skillName: string;
  readonly sourceType: string;
  readonly auth?: unknown;
  readonly grants?: readonly AuthorityProofGrant[];
  readonly scopeAdmission?: ScopeAdmissionContract;
  readonly credential?: CredentialEnvelopeContract;
  readonly sandboxDeclaration?: AuthorityProofSandboxDeclaration;
  readonly sandboxMetadata?: unknown;
  readonly approval?: AuthorityProofApproval;
  readonly mutating?: boolean;
}

export type CredentialBindingDecision =
  | { readonly status: "allow"; readonly reasons: readonly string[] }
  | { readonly status: "deny"; readonly reasons: readonly string[] };

export function connectedAuthRequirement(auth: unknown): ConnectedAuthRequirement | undefined {
  if (auth === undefined || auth === null || auth === false) {
    return undefined;
  }
  if (!isRecord(auth)) {
    return {
      provider: "unknown",
      scopes: [],
    };
  }

  const type = auth.type;
  if (type === "env" || type === "none" || type === "local") {
    return undefined;
  }

  return {
    provider: typeof auth.provider === "string" ? auth.provider : typeof type === "string" ? type : "unknown",
    scopes: Array.isArray(auth.scopes) ? auth.scopes.filter((scope): scope is string => typeof scope === "string") : [],
    scope_family: typeof auth.scope_family === "string" ? auth.scope_family : undefined,
    authority_kind: isAuthorityKind(auth.authority_kind) ? auth.authority_kind : undefined,
    target_repo: typeof auth.target_repo === "string" ? auth.target_repo : undefined,
    target_locator: typeof auth.target_locator === "string" ? auth.target_locator : undefined,
  };
}

export function buildLocalScopeAdmission(
  auth: unknown,
  grants: readonly AuthorityProofGrant[] = [],
  options: {
    readonly deniedBeforeGrantResolution?: boolean;
  } = {},
): ScopeAdmissionContract {
  const requirement = connectedAuthRequirement(auth);
  if (!requirement) {
    return {
      status: "allow",
      requested_scopes: [],
      granted_scopes: [],
      decision_summary: "no connected auth requested",
    };
  }

  if (options.deniedBeforeGrantResolution) {
    return {
      status: "deny",
      requested_scopes: [...uniqueStrings(requirement.scopes)],
      granted_scopes: [],
      reasons: ["structural policy denied before connected auth grant resolution"],
      decision_summary: "structural policy denied before grant resolution",
    };
  }

  const grant = findMatchingGrant(requirement, grants);
  if (!grant) {
    return {
      status: "deny",
      requested_scopes: [...uniqueStrings(requirement.scopes)],
      granted_scopes: [],
      reasons: [`connected auth grant required for provider '${requirement.provider}'`],
      decision_summary: "no matching active grant resolved",
    };
  }

  return {
    status: "allow",
    requested_scopes: [...uniqueStrings(requirement.scopes)],
    granted_scopes: [...uniqueStrings(grant.scopes)],
    grant_id: grant.grant_id,
    decision_summary: "matching active grant admitted",
  };
}

export function buildAuthorityProof(options: BuildAuthorityProofOptions): AuthorityProofContract {
  const requirement = connectedAuthRequirement(options.auth);
  const scopeAdmission = options.scopeAdmission ?? buildLocalScopeAdmission(options.auth, options.grants ?? []);
  const sandbox = summarizeAuthoritySandbox(options.sandboxMetadata, options.sandboxDeclaration, options.approval);
  const credentialMaterial = credentialMaterialProof(options.credential, requirement, scopeAdmission);
  const proof = pruneUndefined({
    schema_version: authorityProofSchemaVersion,
    run_id: options.runId,
    skill_name: options.skillName,
    source_type: options.sourceType,
    requested: {
      connected_auth: Boolean(requirement),
      scopes: requirement ? [...uniqueStrings(requirement.scopes)] : [],
      mutating: options.mutating === true,
      scope_family: requirement?.scope_family,
      authority_kind: requirement?.authority_kind,
      target_repo: requirement?.target_repo,
      target_locator: requirement?.target_locator,
      sandbox_profile: sandbox?.profile,
    },
    scope_admission: scopeAdmission,
    credential_material: credentialMaterial,
    sandbox,
    approval_gate: options.approval
      ? {
          gate_id: options.approval.gate.id,
          gate_type: options.approval.gate.type ?? "unspecified",
          decision: options.approval.approved ? "approved" : "denied",
          reason: options.approval.gate.reason,
        }
      : undefined,
    redaction: {
      status: "applied",
      secret_material: "omitted",
      stdout: "hashed",
      stderr: "hashed",
      metadata_secret_keys: [
        "token-like metadata keys",
        "api-key-like metadata keys",
        "password-like metadata keys",
        "client-secret-like metadata keys",
        "raw-secret-like metadata keys",
      ],
    },
  });
  return validateAuthorityProofContract(proof);
}

export function buildAuthorityProofMetadata(
  options: BuildAuthorityProofOptions,
): Readonly<Record<string, unknown>> {
  return {
    authority_proof: buildAuthorityProof(options),
  };
}

export function validateCredentialBinding(options: {
  readonly auth?: unknown;
  readonly grants?: readonly AuthorityProofGrant[];
  readonly scopeAdmission: ScopeAdmissionContract;
  readonly credential?: CredentialEnvelopeContract;
}): CredentialBindingDecision {
  const requirement = connectedAuthRequirement(options.auth);
  const credential = options.credential;
  if (!credential) {
    if (requirement && options.scopeAdmission.status === "allow" && options.scopeAdmission.grant_id) {
      return {
        status: "deny",
        reasons: ["credential material was not resolved for admitted connected auth grant"],
      };
    }
    return {
      status: "allow",
      reasons: ["no credential material resolved"],
    };
  }

  const reasons: string[] = [];
  if (!requirement) {
    reasons.push("credential material resolved for a skill with no connected auth requirement");
    return {
      status: "deny",
      reasons,
    };
  }

  if (options.scopeAdmission.status !== "allow" || !options.scopeAdmission.grant_id) {
    reasons.push("credential material resolved without an admitted connected auth grant");
    return {
      status: "deny",
      reasons,
    };
  }

  const admittedGrant = (options.grants ?? []).find((grant) => grant.grant_id === options.scopeAdmission.grant_id);
  if (!admittedGrant) {
    reasons.push(`credential admission references grant '${options.scopeAdmission.grant_id}' that was not resolved`);
    return {
      status: "deny",
      reasons,
    };
  }

  if (credential.grant_id !== admittedGrant.grant_id) {
    reasons.push(`credential grant_id '${credential.grant_id}' does not match admitted grant '${admittedGrant.grant_id}'`);
  }
  if (credential.provider !== requirement.provider || credential.provider !== admittedGrant.provider) {
    reasons.push(`credential provider '${credential.provider}' does not match admitted provider '${admittedGrant.provider}'`);
  }

  const missingRequestedScopes = options.scopeAdmission.requested_scopes.filter(
    (scope) => !credential.scopes.some((credentialScope) => scopeAllows(credentialScope, scope)),
  );
  if (missingRequestedScopes.length > 0) {
    reasons.push(`credential scopes do not include admitted request scope(s): ${missingRequestedScopes.join(", ")}`);
  }

  const outOfGrantScopes = credential.scopes.filter(
    (scope) => !admittedGrant.scopes.some((grantedScope) => scopeAllows(grantedScope, scope)),
  );
  if (outOfGrantScopes.length > 0) {
    reasons.push(`credential scopes exceed admitted grant scope(s): ${outOfGrantScopes.join(", ")}`);
  }

  const expectedReference = grantReference(admittedGrant);
  if (expectedReference) {
    if (!credential.grant_reference) {
      reasons.push("credential grant_reference is missing for a targeted admitted grant");
    } else {
      reasons.push(...grantReferenceMismatches(expectedReference, credential.grant_reference));
    }
  } else if (credential.grant_reference) {
    reasons.push("credential grant_reference is present but the admitted grant is not targeted");
  }

  return reasons.length > 0
    ? { status: "deny", reasons }
    : { status: "allow", reasons: ["credential material matches admitted grant"] };
}

function credentialMaterialProof(
  credential: CredentialEnvelopeContract | undefined,
  requirement: ConnectedAuthRequirement | undefined,
  scopeAdmission: ScopeAdmissionContract,
): AuthorityProofContract["credential_material"] {
  if (credential) {
    return pruneUndefined({
      status: "resolved",
      grant_id: credential.grant_id,
      provider: credential.provider,
      connection_id: credential.connection_id,
      scopes: [...credential.scopes],
      grant_reference: credential.grant_reference,
      material_ref_hash: hashString(credential.material_ref),
    }) as AuthorityProofContract["credential_material"];
  }
  if (!requirement) {
    return {
      status: "not_requested",
    };
  }
  return pruneUndefined({
    status: scopeAdmission.status === "deny" ? "denied" : "not_resolved",
    grant_id: scopeAdmission.grant_id,
    provider: requirement.provider,
    scopes: [...uniqueStrings(requirement.scopes)],
    scope_family: requirement.scope_family,
    authority_kind: requirement.authority_kind,
    target_repo: requirement.target_repo,
    target_locator: requirement.target_locator,
  }) as AuthorityProofContract["credential_material"];
}

function summarizeAuthoritySandbox(
  metadata: unknown,
  declaration: AuthorityProofSandboxDeclaration | undefined,
  approval: AuthorityProofApproval | undefined,
): AuthorityProofContract["sandbox"] | undefined {
  const record = isRecord(metadata) ? metadata : undefined;
  const declarationProfile = nonEmptyString(declaration?.profile);
  const profile = nonEmptyString(record?.profile) ?? declarationProfile;
  if (!profile) {
    return undefined;
  }

  const network = isRecord(record?.network) ? record.network : undefined;
  const filesystem = isRecord(record?.filesystem) ? record.filesystem : undefined;
  const runtime = isRecord(record?.runtime) ? record.runtime : undefined;
  const approvalMetadata = isRecord(record?.approval) ? record.approval : undefined;
  const cwdPolicy = nonEmptyString(record?.cwd_policy)
    ?? nonEmptyString(declaration?.cwd_policy)
    ?? nonEmptyString(declaration?.cwdPolicy);
  const requireEnforcement = booleanValue(record?.require_enforcement)
    ?? booleanValue(declaration?.require_enforcement)
    ?? booleanValue(declaration?.requireEnforcement);

  return pruneUndefined({
    profile,
    cwd_policy: cwdPolicy,
    require_enforcement: requireEnforcement,
    network: network || typeof declaration?.network === "boolean"
      ? pruneUndefined({
          declared: booleanValue(network?.declared) ?? booleanValue(declaration?.network),
          enforcement: nonEmptyString(network?.enforcement),
        })
      : undefined,
    filesystem: filesystem
      ? pruneUndefined({
          enforcement: nonEmptyString(filesystem.enforcement),
          readonly_paths: booleanValue(filesystem.readonly_paths),
          writable_paths_enforced: booleanValue(filesystem.writable_paths_enforced),
          private_tmp: booleanValue(filesystem.private_tmp),
        })
      : undefined,
    runtime: runtime
      ? pruneUndefined({
          enforcer: nonEmptyString(runtime.enforcer),
          reason: nonEmptyString(runtime.reason),
        })
      : undefined,
    approval_required: booleanValue(approvalMetadata?.required) ?? (profile === "unrestricted-local-dev"),
    approval_approved: booleanValue(approvalMetadata?.approved) ?? approval?.approved,
  }) as AuthorityProofContract["sandbox"];
}

function findMatchingGrant(
  requirement: ConnectedAuthRequirement,
  grants: readonly AuthorityProofGrant[],
): AuthorityProofGrant | undefined {
  return grants.find(
    (grant) =>
      grant.provider === requirement.provider &&
      grant.status !== "revoked" &&
      requirement.scopes.every((scope) => grant.scopes.some((grantedScope) => scopeAllows(grantedScope, scope))) &&
      grantReferenceMatches(requirement, grant),
  );
}

function grantReferenceMatches(
  requirement: ConnectedAuthRequirement,
  grant: AuthorityProofGrant,
): boolean {
  if (!hasGrantReference(requirement)) {
    return !hasGrantReference(grant);
  }
  return grant.scope_family === requirement.scope_family &&
    grant.authority_kind === requirement.authority_kind &&
    grant.target_repo === requirement.target_repo &&
    grant.target_locator === requirement.target_locator;
}

function hasGrantReference(
  value: Pick<ConnectedAuthRequirement, "scope_family" | "authority_kind" | "target_repo" | "target_locator">,
): boolean {
  return Boolean(value.scope_family || value.authority_kind || value.target_repo || value.target_locator);
}

function grantReference(grant: AuthorityProofGrant): CredentialEnvelopeContract["grant_reference"] | undefined {
  if (!hasGrantReference(grant)) {
    return undefined;
  }
  return pruneUndefined({
    grant_id: grant.grant_id,
    scope_family: grant.scope_family,
    authority_kind: grant.authority_kind,
    target_repo: grant.target_repo,
    target_locator: grant.target_locator,
  }) as CredentialEnvelopeContract["grant_reference"];
}

function grantReferenceMismatches(
  expected: NonNullable<CredentialEnvelopeContract["grant_reference"]>,
  actual: NonNullable<CredentialEnvelopeContract["grant_reference"]>,
): readonly string[] {
  const reasons: string[] = [];
  for (const key of ["grant_id", "scope_family", "authority_kind", "target_repo", "target_locator"] as const) {
    if (actual[key] !== expected[key]) {
      reasons.push(`credential grant_reference.${key} does not match admitted grant`);
    }
  }
  return reasons;
}

function scopeAllows(grantedScope: string, requestedScope: string, allowUniversalWildcard = false): boolean {
  if (grantedScope === "*") {
    return allowUniversalWildcard;
  }
  if (grantedScope === requestedScope) {
    return true;
  }
  if (grantedScope.endsWith(":*")) {
    const suffix = requestedScope.slice(grantedScope.length - 1);
    return requestedScope.startsWith(grantedScope.slice(0, -1)) && suffix.length > 0 && !suffix.includes(":");
  }
  return false;
}

function uniqueStrings(values: readonly string[]): readonly string[] {
  return Array.from(new Set(values));
}

function nonEmptyString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : undefined;
}

function booleanValue(value: unknown): boolean | undefined {
  return typeof value === "boolean" ? value : undefined;
}

function isAuthorityKind(value: unknown): value is ConnectedAuthRequirement["authority_kind"] {
  return value === "read_only" || value === "constructive" || value === "destructive";
}

function pruneUndefined(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map((entry) => pruneUndefined(entry));
  }
  if (!isRecord(value)) {
    return value;
  }

  const result: Record<string, unknown> = {};
  for (const [key, entry] of Object.entries(value)) {
    if (entry === undefined) {
      continue;
    }
    const pruned = pruneUndefined(entry);
    if (isEmptyRecord(pruned)) {
      continue;
    }
    result[key] = pruned;
  }
  return result;
}

function isEmptyRecord(value: unknown): boolean {
  return isRecord(value) && Object.keys(value).length === 0;
}
