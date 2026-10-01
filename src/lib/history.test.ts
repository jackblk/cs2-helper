import { describe, expect, it } from "vitest";
import { emptyHistory, HISTORY_CAP, record, redone, undone } from "./history";

const entry = (n: number) => ({ label: `e${n}`, before: n - 1, after: n });

describe("history", () => {
	it("record appends and clears the redo stack", () => {
		const h = { past: [entry(1)], future: [entry(2)] };
		expect(record(h, entry(3))).toEqual({
			past: [entry(1), entry(3)],
			future: [],
		});
	});

	it("record keeps only the newest HISTORY_CAP entries", () => {
		let h = emptyHistory<number>();
		for (let i = 1; i <= HISTORY_CAP + 5; i++) h = record(h, entry(i));
		expect(h.past).toHaveLength(HISTORY_CAP);
		expect(h.past[0]).toEqual(entry(6));
	});

	it("undone moves the newest entry to the front of the redo stack", () => {
		const h = { past: [entry(1), entry(2)], future: [entry(3)] };
		expect(undone(h)).toEqual({
			past: [entry(1)],
			future: [entry(2), entry(3)],
		});
	});

	it("redone moves the first redo entry back onto the past", () => {
		const h = { past: [entry(1)], future: [entry(2), entry(3)] };
		expect(redone(h)).toEqual({
			past: [entry(1), entry(2)],
			future: [entry(3)],
		});
	});

	it("undone and redone leave an empty side unchanged", () => {
		const h = emptyHistory<number>();
		expect(undone(h)).toBe(h);
		expect(redone(h)).toBe(h);
	});
});
