import { describe, expect, it } from "vitest";
import { isNewer } from "./update-check";

describe("isNewer", () => {
	it("compares each part numerically", () => {
		expect(isNewer("0.4.1", "0.4.0")).toBe(true);
		expect(isNewer("0.10.0", "0.9.9")).toBe(true);
		expect(isNewer("1.0.0", "0.99.99")).toBe(true);
	});
	it("is false for the same or an older version", () => {
		expect(isNewer("0.4.0", "0.4.0")).toBe(false);
		expect(isNewer("0.3.3", "0.4.0")).toBe(false);
	});
	it("accepts a leading v", () => {
		expect(isNewer("v0.4.1", "0.4.0")).toBe(true);
	});
	it("sorts a prerelease before its release", () => {
		expect(isNewer("0.5.0", "0.5.0-beta.1")).toBe(true);
		expect(isNewer("0.5.0-beta.1", "0.5.0")).toBe(false);
		expect(isNewer("0.5.0-beta.2", "0.5.0-beta.1")).toBe(true);
	});
	it("is false for unparseable input", () => {
		expect(isNewer("latest", "0.4.0")).toBe(false);
		expect(isNewer("0.4.1", "")).toBe(false);
	});
});
