// App version tooling. package.json is the source of truth: tauri.conf.json
// reads it via "../package.json", and Cargo.toml / Cargo.lock are kept in sync
// by `bump`.
//
//   node scripts/version.mjs bump 0.3.4   set the version everywhere
//   node scripts/version.mjs check [tag]  fail if the files (or tag) disagree

import { readFileSync, writeFileSync } from "node:fs";

const PACKAGE_JSON = "package.json";
const CARGO_TOML = "src-tauri/Cargo.toml";
const CARGO_LOCK = "src-tauri/Cargo.lock";
const TAURI_CONF = "src-tauri/tauri.conf.json";

const SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;
// First `version = "..."` line, which is the one in [package].
const CARGO_TOML_VERSION = /^version = "([^"]*)"/m;
const CARGO_LOCK_VERSION = /(name = "cs2-helper"\nversion = )"([^"]*)"/;

const read = (path) => readFileSync(path, "utf8");

function fail(message) {
	console.error(`error: ${message}`);
	process.exit(1);
}

function match(path, re, group) {
	const m = read(path).match(re);
	if (!m) fail(`no version found in ${path}`);
	return m[group];
}

function versions() {
	return {
		[PACKAGE_JSON]: JSON.parse(read(PACKAGE_JSON)).version,
		[CARGO_TOML]: match(CARGO_TOML, CARGO_TOML_VERSION, 1),
		[CARGO_LOCK]: match(CARGO_LOCK, CARGO_LOCK_VERSION, 2),
	};
}

function bump(version) {
	if (!SEMVER.test(version ?? "")) {
		fail(`usage: pnpm bump <x.y.z> (got "${version ?? ""}")`);
	}

	const pkg = JSON.parse(read(PACKAGE_JSON));
	pkg.version = version;
	writeFileSync(PACKAGE_JSON, `${JSON.stringify(pkg, null, "\t")}\n`);

	writeFileSync(
		CARGO_TOML,
		read(CARGO_TOML).replace(CARGO_TOML_VERSION, `version = "${version}"`),
	);
	writeFileSync(
		CARGO_LOCK,
		read(CARGO_LOCK).replace(CARGO_LOCK_VERSION, `$1"${version}"`),
	);

	check();
}

function check(tag) {
	const found = versions();
	const expected = tag ?? found[PACKAGE_JSON];
	const tauri = JSON.parse(read(TAURI_CONF)).version;

	let ok = true;
	if (tauri !== "../package.json") {
		console.error(`${TAURI_CONF}: "version" must be "../package.json"`);
		ok = false;
	}
	for (const [path, version] of Object.entries(found)) {
		const mark = version === expected ? "ok" : "MISMATCH";
		if (version !== expected) ok = false;
		console.log(`${mark.padEnd(8)} ${path}: ${version}`);
	}
	if (!ok) fail(`versions do not all match ${expected}`);
	console.log(`version ${expected}`);
}

const [command, arg] = process.argv.slice(2);
if (command === "bump") bump(arg);
else if (command === "check") check(arg);
else fail("usage: node scripts/version.mjs <bump x.y.z | check [tag]>");
