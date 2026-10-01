import { describe, expect, it } from "vitest";
import { warningsKey } from "./config-warnings";

describe("warningsKey", () => {
	it("is the same for the same set in any order", () => {
		expect(warningsKey(["b", "a"])).toBe(warningsKey(["a", "b"]));
	});

	it("is empty for no warnings", () => {
		expect(warningsKey([])).toBe("");
	});

	it("differs when the set changes", () => {
		expect(warningsKey(["a"])).not.toBe(warningsKey(["a", "b"]));
	});
});
