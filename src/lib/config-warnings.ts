// The engine re-emits the same warnings on every snapshot; a key that ignores
// order lets callers react only when the set actually changes.
export function warningsKey(warnings: readonly string[]): string {
	return [...warnings].sort().join("\n");
}
