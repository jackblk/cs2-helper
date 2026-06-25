// Pure bomb-defuse decision logic. CS2 defaults: 5s defuse with kit, 10s without.
// One shared rule colors both sides (see spec): the color means "a defuse is
// still possible"; only its emotional read flips (CT act/run vs T defend/won).

export const DEFUSE_WITH_KIT = 5.0;
export const DEFUSE_NO_KIT = 10.0;
export const C4_FUSE_DEFAULT = 40.0;

export type DefuseColor = "green" | "red";

/** Seconds of defuse needed given side and kit. T / unknown assume 5s (the safe
 *  case: a CT with a kit is the fastest possible defuse). */
export function defuseNeeded(team: string | null, hasKit: boolean): number {
	if (team === "CT") return hasKit ? DEFUSE_WITH_KIT : DEFUSE_NO_KIT;
	return DEFUSE_WITH_KIT;
}

/** Green when there is still (mathematically) enough time to finish a defuse. */
export function defuseColor(
	team: string | null,
	hasKit: boolean,
	remaining: number,
): DefuseColor {
	return remaining >= defuseNeeded(team, hasKit) ? "green" : "red";
}
