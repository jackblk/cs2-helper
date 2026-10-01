// `Number("")` is 0, so blank text is rejected by the > 0 check.
export function parsePositive(text: string, integer = false): number | null {
	const n = Number(text);
	const value = integer ? Math.round(n) : n;
	return Number.isFinite(value) && value > 0 ? value : null;
}
