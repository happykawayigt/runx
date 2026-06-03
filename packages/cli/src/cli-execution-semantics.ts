export const GOVERNED_DISPOSITIONS = [
  "completed",
  "needs_agent",
  "policy_denied",
  "approval_required",
  "observing",
  "escalated",
] as const;

export type GovernedDisposition = (typeof GOVERNED_DISPOSITIONS)[number];
export type OutcomeState = "pending" | "complete" | "expired";

export interface ReceiptOutcome {
  readonly code?: string;
  readonly summary?: string;
  readonly observed_at?: string;
  readonly data?: Readonly<Record<string, unknown>>;
}

export interface ReceiptSurfaceRef {
  readonly type: string;
  readonly uri: string;
  readonly label?: string;
}

export interface InputContextCapture {
  readonly capture?: boolean;
  readonly source?: string;
  readonly max_bytes?: number;
  readonly snapshot?: unknown;
}

export interface ExecutionSemantics {
  readonly disposition?: GovernedDisposition;
  readonly outcome_state?: OutcomeState;
  readonly outcome?: ReceiptOutcome;
  readonly input_context?: InputContextCapture;
  readonly surface_refs?: readonly ReceiptSurfaceRef[];
  readonly evidence_refs?: readonly ReceiptSurfaceRef[];
}
