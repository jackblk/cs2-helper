import { describe, expect, it } from "vitest";
import {
	clampToMonitor,
	overlaySize,
	placeToolbar,
	SCALE_MAX,
	SCALE_MIN,
	stepScale,
	TOOLBAR_GAP,
	TOOLBAR_SIZE,
} from "./overlay-edit";

const monitor = { x: 0, y: 0, width: 1920, height: 1080 };

describe("placeToolbar", () => {
	it("centres the toolbar above the overlay", () => {
		const overlay = { x: 800, y: 500, width: 240, height: 96 };
		expect(placeToolbar(overlay, monitor)).toEqual({
			x: 800 + (240 - TOOLBAR_SIZE.width) / 2,
			y: 500 - TOOLBAR_GAP - TOOLBAR_SIZE.height,
			side: "above",
		});
	});

	it("flips below when there is no room above", () => {
		const overlay = { x: 800, y: 20, width: 240, height: 96 };
		const p = placeToolbar(overlay, monitor);
		expect(p.side).toBe("below");
		expect(p.y).toBe(20 + 96 + TOOLBAR_GAP);
	});

	it("stays on screen at the left edge at 0.5x", () => {
		const overlay = { x: 0, y: 500, width: 120, height: 48 };
		expect(placeToolbar(overlay, monitor).x).toBe(0);
	});

	it("stays on screen at the right edge at 0.5x", () => {
		const overlay = { x: 1920 - 120, y: 500, width: 120, height: 48 };
		expect(placeToolbar(overlay, monitor).x).toBe(1920 - TOOLBAR_SIZE.width);
	});

	it("respects a monitor that does not start at the origin", () => {
		const left = { x: -1920, y: 0, width: 1920, height: 1080 };
		const overlay = { x: -1920, y: 0, width: 240, height: 96 };
		expect(placeToolbar(overlay, left)).toEqual({
			x: -1920 + (240 - TOOLBAR_SIZE.width) / 2,
			y: 96 + TOOLBAR_GAP,
			side: "below",
		});
	});
});

describe("clampToMonitor", () => {
	it("leaves a rect inside the monitor unchanged", () => {
		const r = { x: 10, y: 10, width: 240, height: 96 };
		expect(clampToMonitor(r, monitor)).toEqual(r);
	});

	it("pulls a rect back from the right and bottom edges", () => {
		const r = { x: 1900, y: 1070, width: 240, height: 96 };
		expect(clampToMonitor(r, monitor)).toEqual({
			...r,
			x: 1920 - 240,
			y: 1080 - 96,
		});
	});

	it("pulls a rect back from the left and top edges", () => {
		const r = { x: -50, y: -5, width: 240, height: 96 };
		expect(clampToMonitor(r, monitor)).toEqual({ ...r, x: 0, y: 0 });
	});
});

describe("stepScale", () => {
	it("steps by 0.05 without float drift", () => {
		expect(stepScale(1.15, 1)).toBe(1.2);
		expect(stepScale(1.05, -1)).toBe(1);
	});

	it("clamps to the scale bounds", () => {
		expect(stepScale(SCALE_MAX, 1)).toBe(SCALE_MAX);
		expect(stepScale(SCALE_MIN, -1)).toBe(SCALE_MIN);
		expect(stepScale(3, -1)).toBe(SCALE_MAX);
	});
});

describe("overlaySize", () => {
	it("scales the base size", () => {
		expect(overlaySize(0.5)).toEqual({ width: 120, height: 48 });
	});
});
