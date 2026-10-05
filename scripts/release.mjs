// Tags the current app version and pushes the tag, which starts the release
// workflow (.github/workflows/release.yml). Asks before tagging.
//
//   pnpm release

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { createInterface } from "node:readline/promises";

const REMOTE = "origin";

function fail(message) {
	console.error(`error: ${message}`);
	process.exit(1);
}

const git = (...args) =>
	execFileSync("git", args, { encoding: "utf8" }).trim();

// [major, minor, patch, prerelease]; a prerelease sorts before its release.
function parse(version) {
	const m = version.match(/^(\d+)\.(\d+)\.(\d+)(?:-(.+))?$/);
	return m && [Number(m[1]), Number(m[2]), Number(m[3]), m[4] ?? null];
}

function compare(a, b) {
	for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] - b[i];
	if (a[3] === b[3]) return 0;
	if (a[3] === null) return 1;
	if (b[3] === null) return -1;
	return a[3] < b[3] ? -1 : 1;
}

// Fails unless package.json, Cargo.toml and Cargo.lock agree.
try {
	execFileSync("node", ["scripts/version.mjs", "check"], { stdio: "inherit" });
} catch {
	process.exit(1);
}
const version = JSON.parse(readFileSync("package.json", "utf8")).version;

if (git("status", "--porcelain")) {
	fail("the working tree has uncommitted changes");
}
let unpushed;
try {
	unpushed = Number(git("rev-list", "--count", "@{upstream}..HEAD"));
} catch {
	fail("the current branch has no upstream; push it first");
}
if (unpushed > 0) {
	fail(`${unpushed} commit(s) not pushed yet; push the branch first`);
}

const remoteTags = git("ls-remote", "--tags", "--refs", REMOTE)
	.split("\n")
	.filter(Boolean)
	.map((line) => line.split("refs/tags/")[1]);
if (remoteTags.includes(version)) {
	fail(`tag ${version} is already on ${REMOTE}; bump the version first`);
}
if (git("tag", "--list", version)) {
	fail(`tag ${version} already exists locally; delete it or bump the version`);
}

const latest = remoteTags
	.map((tag) => ({ tag, parsed: parse(tag) }))
	.filter((t) => t.parsed)
	.sort((a, b) => compare(b.parsed, a.parsed))[0];
if (latest && compare(parse(version), latest.parsed) < 0) {
	console.warn(`warning: ${version} is lower than ${latest.tag}`);
}

const rl = createInterface({ input: process.stdin, output: process.stdout });
const answer = await rl.question(
	`Latest tag on GitHub: ${latest?.tag ?? "none"}. Current tag: ${version}. Do you want to release ${version}? [y/N] `,
);
rl.close();
if (answer.trim().toLowerCase() !== "y") {
	console.log("Cancelled.");
	process.exit(0);
}

execFileSync("git", ["tag", version], { stdio: "inherit" });
execFileSync("git", ["push", REMOTE, version], { stdio: "inherit" });
console.log(`Pushed ${version}. The release workflow builds a draft release.`);
