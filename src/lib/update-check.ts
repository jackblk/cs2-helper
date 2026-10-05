export const LATEST_RELEASE_API =
	"https://api.github.com/repos/jackblk/cs2-helper/releases/latest";
/** The homepage, whose Download button always fetches the latest release. */
export const DOWNLOAD_PAGE_URL = "https://jackblk.github.io/cs2-helper/";

type Parsed = [number, number, number, string | null];

/** `x.y.z` or `x.y.z-pre`, with an optional leading `v`. */
function parse(version: string): Parsed | null {
	const m = version.trim().match(/^v?(\d+)\.(\d+)\.(\d+)(?:-(.+))?$/);
	return m ? [Number(m[1]), Number(m[2]), Number(m[3]), m[4] ?? null] : null;
}

/** True when `latest` is a newer version than `current`. A prerelease sorts
 * before its release; unparseable input is never newer. */
export function isNewer(latest: string, current: string): boolean {
	const a = parse(latest);
	const b = parse(current);
	if (!a || !b) return false;
	for (let i = 0; i < 3; i++) {
		if (a[i] !== b[i]) return (a[i] as number) > (b[i] as number);
	}
	if (a[3] === b[3]) return false;
	if (a[3] === null) return true;
	if (b[3] === null) return false;
	return a[3] > b[3];
}

/** Tag of the latest published GitHub release, or null when offline or the
 * request fails. Drafts and prereleases are never "latest". */
export async function fetchLatestVersion(): Promise<string | null> {
	try {
		const res = await fetch(LATEST_RELEASE_API, {
			headers: { Accept: "application/vnd.github+json" },
		});
		if (!res.ok) return null;
		const data: { tag_name?: unknown } = await res.json();
		return typeof data.tag_name === "string" ? data.tag_name : null;
	} catch {
		return null;
	}
}
