// CS2 timings, in seconds.
// One rule colors both sides: green means "a defuse is still possible"; only
// its read flips (CT: go defuse / run, T: defend / round won).

export const DEFUSE_WITH_KIT = 5.0;
export const DEFUSE_NO_KIT = 10.0;
export const C4_FUSE_DEFAULT = 40.0;

export type DefuseColor = "green" | "red";

// T and unknown sides assume an enemy kit (the fastest possible defuse).
export function defuseNeeded(team: string | null, hasKit: boolean): number {
	if (team === "CT") return hasKit ? DEFUSE_WITH_KIT : DEFUSE_NO_KIT;
	return DEFUSE_WITH_KIT;
}

export function defuseColor(
	team: string | null,
	hasKit: boolean,
	remaining: number,
): DefuseColor {
	return remaining >= defuseNeeded(team, hasKit) ? "green" : "red";
}
