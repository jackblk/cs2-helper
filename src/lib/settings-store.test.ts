import { describe, expect, it, vi } from "vitest";
import { createSettingsStore, type StoreNotice } from "./settings-store";

type S = { v: number };

function deferred() {
	let resolve!: () => void;
	const promise = new Promise<void>((res) => {
		resolve = res;
	});
	return { promise, resolve };
}

const flush = () => new Promise((r) => setTimeout(r, 0));

function setup(save: (s: S) => Promise<void> = async () => {}) {
	const notices: StoreNotice[] = [];
	const saveMock = vi.fn(save);
	const store = createSettingsStore<S>({
		save: saveMock,
		notify: (n) => notices.push(n),
	});
	store.replace({ v: 0 });
	return { store, save: saveMock, notices };
}

describe("createSettingsStore", () => {
	it("shows a commit immediately and records it once saved", async () => {
		const { store, save, notices } = setup();
		store.commit("a", () => ({ v: 1 }));
		expect(store.getSnapshot().current).toEqual({ v: 1 });
		await store.idle();
		expect(save).toHaveBeenCalledWith({ v: 1 });
		expect(store.getSnapshot().history.past).toEqual([
			{ label: "a", before: { v: 0 }, after: { v: 1 } },
		]);
		expect(notices).toEqual([{ kind: "saved", label: "a" }]);
	});

	it("ignores a commit that changes nothing", async () => {
		const { store, save } = setup();
		store.commit("a", (s) => ({ ...s }));
		await store.idle();
		expect(save).not.toHaveBeenCalled();
		expect(store.getSnapshot().history.past).toEqual([]);
	});

	it("runs saves one at a time, in commit order", async () => {
		const gate = deferred();
		const calls: number[] = [];
		const { store } = setup(async (s) => {
			calls.push(s.v);
			if (s.v === 1) await gate.promise;
		});
		store.commit("a", () => ({ v: 1 }));
		store.commit("b", () => ({ v: 2 }));
		await flush();
		expect(calls).toEqual([1]);
		gate.resolve();
		await store.idle();
		expect(calls).toEqual([1, 2]);
	});

	it("reverts and drops queued commits when a save fails", async () => {
		const { store, save, notices } = setup(async (s) => {
			if (s.v === 1) throw new Error("taken");
		});
		store.commit("a", () => ({ v: 1 }));
		store.commit("b", (s) => ({ v: s.v + 1 }));
		expect(store.getSnapshot().current).toEqual({ v: 2 });
		await store.idle();
		expect(save).toHaveBeenCalledTimes(1);
		expect(store.getSnapshot().current).toEqual({ v: 0 });
		expect(store.getSnapshot().history.past).toEqual([]);
		expect(notices).toEqual([
			{ kind: "save-failed", error: new Error("taken") },
		]);
	});

	it("undo saves the previous state and redo re-applies it", async () => {
		const { store, save, notices } = setup();
		store.commit("a", () => ({ v: 1 }));
		await store.undo();
		expect(store.getSnapshot().current).toEqual({ v: 0 });
		expect(save).toHaveBeenLastCalledWith({ v: 0 });
		expect(store.getSnapshot().history.future).toHaveLength(1);
		await store.redo();
		expect(store.getSnapshot().current).toEqual({ v: 1 });
		expect(save).toHaveBeenLastCalledWith({ v: 1 });
		expect(notices.map((n) => n.kind)).toEqual(["saved", "undone", "redone"]);
	});

	it("leaves the stack and state alone when an undo save fails", async () => {
		let failNext = false;
		const { store, notices } = setup(async () => {
			if (failNext) throw new Error("disk full");
		});
		store.commit("a", () => ({ v: 1 }));
		await store.idle();
		failNext = true;
		await store.undo();
		expect(store.getSnapshot().current).toEqual({ v: 1 });
		expect(store.getSnapshot().history.past).toHaveLength(1);
		expect(store.getSnapshot().history.future).toHaveLength(0);
		expect(notices[notices.length - 1]).toEqual({
			kind: "undo-failed",
			error: new Error("disk full"),
		});
	});

	it("clears redo when a new change is committed after undo", async () => {
		const { store } = setup();
		store.commit("a", () => ({ v: 1 }));
		await store.undo();
		store.commit("b", () => ({ v: 5 }));
		await store.idle();
		expect(store.getSnapshot().history.future).toEqual([]);
		expect(store.getSnapshot().history.past).toEqual([
			{ label: "b", before: { v: 0 }, after: { v: 5 } },
		]);
	});

	it("undo and redo with nothing to apply do nothing", async () => {
		const { store, save, notices } = setup();
		await store.undo();
		await store.redo();
		expect(save).not.toHaveBeenCalled();
		expect(notices).toEqual([]);
	});

	it("replace clears history and discards queued and in-flight saves", async () => {
		const gate = deferred();
		const { store, save } = setup(async (s) => {
			if (s.v === 1) await gate.promise;
		});
		store.commit("a", () => ({ v: 1 }));
		store.commit("b", () => ({ v: 2 }));
		await flush();
		store.replace({ v: 9 });
		gate.resolve();
		await store.idle();
		expect(save).toHaveBeenCalledTimes(1);
		expect(store.getSnapshot().current).toEqual({ v: 9 });
		expect(store.getSnapshot().history.past).toEqual([]);
	});

	it("run records a backend-computed change as one undoable step", async () => {
		const { store, notices } = setup();
		await store.run("Reset to defaults", "Reset failed", async () => ({
			v: 5,
		}));
		expect(store.getSnapshot().current).toEqual({ v: 5 });
		expect(store.getSnapshot().history.past).toEqual([
			{ label: "Reset to defaults", before: { v: 0 }, after: { v: 5 } },
		]);
		expect(notices).toEqual([{ kind: "applied", label: "Reset to defaults" }]);
	});

	it("run reports a failure and keeps the state", async () => {
		const { store, notices } = setup();
		await store.run("Reset to defaults", "Reset failed", async () => {
			throw new Error("locked");
		});
		expect(store.getSnapshot().current).toEqual({ v: 0 });
		expect(store.getSnapshot().history.past).toEqual([]);
		expect(notices).toEqual([
			{ kind: "failed", title: "Reset failed", error: new Error("locked") },
		]);
	});

	it("notifies subscribers on every change", () => {
		const { store } = setup();
		const listener = vi.fn();
		const unsubscribe = store.subscribe(listener);
		store.commit("a", () => ({ v: 1 }));
		expect(listener).toHaveBeenCalled();
		unsubscribe();
		listener.mockClear();
		store.replace({ v: 3 });
		expect(listener).not.toHaveBeenCalled();
	});
});

describe("commits racing undo and run", () => {
	type P = { a: number; b: number };

	function setupPair(block: (s: P) => boolean, gate: Promise<void>) {
		const save = vi.fn(async (s: P) => {
			if (block(s)) await gate;
		});
		const store = createSettingsStore<P>({ save, notify: () => {} });
		store.replace({ a: 0, b: 0 });
		return { store, save };
	}

	it("applies a commit made during an in-flight undo on top of the undone state", async () => {
		const gate = deferred();
		const { store, save } = setupPair(
			(s) => s.a === 0 && s.b === 0,
			gate.promise,
		);
		store.commit("a", (s) => ({ ...s, a: 1 }));
		await store.idle();
		store.undo();
		await flush();
		store.commit("b", (s) => ({ ...s, b: 1 }));
		gate.resolve();
		await store.idle();
		expect(save).toHaveBeenLastCalledWith({ a: 0, b: 1 });
		expect(store.getSnapshot().current).toEqual({ a: 0, b: 1 });
		expect(store.getSnapshot().history.past).toEqual([
			{ label: "b", before: { a: 0, b: 0 }, after: { a: 0, b: 1 } },
		]);
	});

	it("applies a commit made during an in-flight run on top of its result", async () => {
		const gate = deferred();
		const { store, save } = setupPair(() => false, gate.promise);
		store.run("Reset to defaults", "Reset failed", async () => {
			await gate.promise;
			return { a: 5, b: 5 };
		});
		await flush();
		store.commit("b", (s) => ({ ...s, b: 7 }));
		gate.resolve();
		await store.idle();
		expect(save).toHaveBeenLastCalledWith({ a: 5, b: 7 });
		expect(store.getSnapshot().current).toEqual({ a: 5, b: 7 });
	});

	it("keeps an edit queued behind a slow save visible after the save lands", async () => {
		const gate = deferred();
		const { store } = setupPair((s) => s.a === 1, gate.promise);
		store.commit("a", (s) => ({ ...s, a: 1 }));
		store.commit("b", (s) => ({ ...s, b: 1 }));
		await flush();
		gate.resolve();
		await flush();
		expect(store.getSnapshot().current).toEqual({ a: 1, b: 1 });
		await store.idle();
		expect(store.getSnapshot().current).toEqual({ a: 1, b: 1 });
	});
});
