export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function isPlainRecord(value: unknown): value is Readonly<Record<string, unknown>> {
  return isRecord(value);
}

export function asRecord(value: unknown): Record<string, unknown> | undefined {
  return isRecord(value) ? value : undefined;
}

/** Read `value[key]` when `value` is a record, returning undefined otherwise. */
export function readField(value: unknown, key: string): unknown {
  return asRecord(value)?.[key];
}

/** Read `value[key]` and return it only when it is itself a record. */
export function recordField(value: unknown, key: string): Readonly<Record<string, unknown>> | undefined {
  return asRecord(readField(value, key));
}

/** Narrow an unknown to a string, returning undefined otherwise (no throw). */
export function stringValue(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

/** Return the value when it is an array, otherwise an empty array. */
export function arrayValue(value: unknown): readonly unknown[] {
  return Array.isArray(value) ? value : [];
}

/** Type guard that filters out `undefined` (handy in `.filter(isDefined)`). */
export function isDefined<T>(value: T | undefined): value is T {
  return value !== undefined;
}

export function isNodeError(error: unknown): error is NodeJS.ErrnoException {
  return error instanceof Error && "code" in error;
}

export function isNotFound(error: unknown): boolean {
  return isNodeError(error) && error.code === "ENOENT";
}

export function isAlreadyExists(error: unknown): boolean {
  return isNodeError(error) && error.code === "EEXIST";
}

export function errorMessage(value: unknown): string {
  return value instanceof Error ? value.message : String(value);
}
