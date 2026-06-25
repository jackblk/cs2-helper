import { describe, expect, it } from "vitest";
import {
	DEFUSE_NO_KIT,
	DEFUSE_WITH_KIT,
	defuseColor,
	defuseNeeded,
} from "./defuse";

describe("defuseNeeded", () => {
	it("CT with kit needs 5s", () => {
		expect(defuseNeeded("CT", true)).toBe(DEFUSE_WITH_KIT);
	});
	it("CT without kit needs 10s", () => {
		expect(defuseNeeded("CT", false)).toBe(DEFUSE_NO_KIT);
	});
	it("T assumes enemy kit (5s, safe case)", () => {
		expect(defuseNeeded("T", false)).toBe(DEFUSE_WITH_KIT);
	});
	it("unknown team assumes 5s", () => {
		expect(defuseNeeded(null, false)).toBe(DEFUSE_WITH_KIT);
	});
});

describe("defuseColor", () => {
	it("green when remaining >= needed (CT, kit)", () => {
		expect(defuseColor("CT", true, 5.0)).toBe("green");
		expect(defuseColor("CT", true, 5.1)).toBe("green");
	});
	it("red when remaining < needed (CT, kit)", () => {
		expect(defuseColor("CT", true, 4.9)).toBe("red");
	});
	it("CT no kit flips at 10s", () => {
		expect(defuseColor("CT", false, 10.0)).toBe("green");
		expect(defuseColor("CT", false, 9.9)).toBe("red");
	});
	it("T flips at 5s", () => {
		expect(defuseColor("T", false, 5.0)).toBe("green");
		expect(defuseColor("T", false, 4.9)).toBe("red");
	});
});
