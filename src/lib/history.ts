export type HistoryEntry<T> = { label: string; before: T; after: T };

export type History<T> = {
	past: HistoryEntry<T>[];
	future: HistoryEntry<T>[];
};

export const HISTORY_CAP = 50;

export function emptyHistory<T>(): History<T> {
	return { past: [], future: [] };
}

export function record<T>(h: History<T>, entry: HistoryEntry<T>): History<T> {
	return { past: [...h.past, entry].slice(-HISTORY_CAP), future: [] };
}

export function undone<T>(h: History<T>): History<T> {
	if (h.past.length === 0) return h;
	const entry = h.past[h.past.length - 1];
	return { past: h.past.slice(0, -1), future: [entry, ...h.future] };
}

export function redone<T>(h: History<T>): History<T> {
	if (h.future.length === 0) return h;
	const [entry, ...rest] = h.future;
	return { past: [...h.past, entry], future: rest };
}
