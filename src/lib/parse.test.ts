import { describe, expect, it } from "vitest";
import { parsePositive } from "./parse";

describe("parsePositive", () => {
	it("accepts positive numbers", () => {
		expect(parsePositive("1500")).toBe(1500);
		expect(parsePositive("2.5")).toBe(2.5);
		expect(parsePositive(" 12 ")).toBe(12);
	});

	it("rejects blank, zero, negative and non-numeric text", () => {
		expect(parsePositive("")).toBeNull();
		expect(parsePositive("   ")).toBeNull();
		expect(parsePositive("0")).toBeNull();
		expect(parsePositive("-5")).toBeNull();
		expect(parsePositive("abc")).toBeNull();
	});

	it("rounds integer fields and rejects values that round to zero", () => {
		expect(parsePositive("1499.6", true)).toBe(1500);
		expect(parsePositive("0.3", true)).toBeNull();
	});
});
