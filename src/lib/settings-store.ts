import {
	emptyHistory,
	type History,
	record,
	redone,
	undone,
} from "@/lib/history";

// The store's own failures carry a kind, not text, so the caller words them
// (and translates them); `run` failures use the caller's `failTitle`.
export type StoreNotice =
	| { kind: "saved" | "undone" | "redone" | "applied"; label: string }
	| { kind: "save-failed" | "undo-failed" | "redo-failed"; error: unknown }
	| { kind: "failed"; title: string; error: unknown };

export type StoreSnapshot<T> = { current: T | null; history: History<T> };

const same = (a: unknown, b: unknown) =>
	JSON.stringify(a) === JSON.stringify(b);

// Autosave with undo/redo. `saved` is the last state the backend accepted;
// `current` is `saved` with the not-yet-saved commits replayed on top. Jobs
// run one at a time in commit order, so a slow save can never land after a
// newer one, and each commit is applied to `saved` only when it runs, so it
// lands on top of an undo or reset queued before it.
export function createSettingsStore<T>({
	save,
	notify,
}: {
	save: (value: T) => Promise<void>;
	notify: (notice: StoreNotice) => void;
}) {
	let snapshot: StoreSnapshot<T> = {
		current: null,
		history: emptyHistory<T>(),
	};
	let saved: T | null = null;
	let pending: { update: (value: T) => T }[] = [];
	// Bumped when queued jobs no longer apply: a failed save they were built
	// on, or settings replaced from outside.
	let generation = 0;
	let tail: Promise<void> = Promise.resolve();
	const listeners = new Set<() => void>();

	const set = (patch: Partial<StoreSnapshot<T>>) => {
		const current =
			saved == null ? null : pending.reduce<T>((s, p) => p.update(s), saved);
		snapshot = { ...snapshot, ...patch, current };
		for (const listener of listeners) listener();
	};

	const enqueue = (job: (stale: () => boolean) => Promise<void>) => {
		const gen = generation;
		const stale = () => gen !== generation;
		tail = tail.then(() => (stale() ? undefined : job(stale)));
		return tail;
	};

	const fail = (notice: StoreNotice) => {
		generation += 1;
		pending = [];
		set({});
		notify(notice);
	};

	return {
		getSnapshot: () => snapshot,

		subscribe(listener: () => void) {
			listeners.add(listener);
			return () => {
				listeners.delete(listener);
			};
		},

		replace(value: T) {
			generation += 1;
			saved = value;
			pending = [];
			set({ history: emptyHistory<T>() });
		},

		commit(label: string, update: (value: T) => T): Promise<void> {
			const shown = snapshot.current;
			if (shown == null || same(shown, update(shown))) return tail;
			const entry = { update };
			pending.push(entry);
			set({});
			return enqueue(async (stale) => {
				const before = saved as T;
				const after = update(before);
				const settle = () => {
					pending = pending.filter((p) => p !== entry);
				};
				if (same(before, after)) {
					settle();
					set({});
					return;
				}
				try {
					await save(after);
				} catch (e) {
					if (!stale()) fail({ kind: "save-failed", error: e });
					return;
				}
				if (stale()) return;
				saved = after;
				settle();
				set({ history: record(snapshot.history, { label, before, after }) });
				notify({ kind: "saved", label });
			});
		},

		run(label: string, failTitle: string, op: () => Promise<T>): Promise<void> {
			return enqueue(async (stale) => {
				const before = saved;
				let after: T;
				try {
					after = await op();
				} catch (e) {
					if (!stale()) fail({ kind: "failed", title: failTitle, error: e });
					return;
				}
				if (stale()) return;
				saved = after;
				set({
					history:
						before == null
							? snapshot.history
							: record(snapshot.history, { label, before, after }),
				});
				notify({ kind: "applied", label });
			});
		},

		undo(): Promise<void> {
			return enqueue(async (stale) => {
				const { past } = snapshot.history;
				if (past.length === 0) return;
				const entry = past[past.length - 1];
				try {
					await save(entry.before);
				} catch (e) {
					if (!stale()) notify({ kind: "undo-failed", error: e });
					return;
				}
				if (stale()) return;
				saved = entry.before;
				set({ history: undone(snapshot.history) });
				notify({ kind: "undone", label: entry.label });
			});
		},

		redo(): Promise<void> {
			return enqueue(async (stale) => {
				const { future } = snapshot.history;
				if (future.length === 0) return;
				const entry = future[0];
				try {
					await save(entry.after);
				} catch (e) {
					if (!stale()) notify({ kind: "redo-failed", error: e });
					return;
				}
				if (stale()) return;
				saved = entry.after;
				set({ history: redone(snapshot.history) });
				notify({ kind: "redone", label: entry.label });
			});
		},

		idle: () => tail,
	};
}

export type SettingsStore<T> = ReturnType<typeof createSettingsStore<T>>;
