import { useEffect } from "react";

// Text fields keep their native undo. The hotkey recorder swallows keys in
// the capture phase, so this bubble-phase listener never sees them.
function isTextField(target: EventTarget | null): boolean {
	return (
		target instanceof HTMLElement &&
		(target.isContentEditable ||
			target.tagName === "INPUT" ||
			target.tagName === "TEXTAREA")
	);
}

export function useUndoShortcuts(
	undo: () => void,
	redo: () => void,
	enabled: boolean,
) {
	useEffect(() => {
		if (!enabled) return;
		const onKeyDown = (e: KeyboardEvent) => {
			if (!e.ctrlKey || e.altKey || e.metaKey || isTextField(e.target)) return;
			if (e.code === "KeyZ" && !e.shiftKey) {
				e.preventDefault();
				undo();
			} else if (e.code === "KeyY" || (e.code === "KeyZ" && e.shiftKey)) {
				e.preventDefault();
				redo();
			}
		};
		window.addEventListener("keydown", onKeyDown);
		return () => window.removeEventListener("keydown", onKeyDown);
	}, [undo, redo, enabled]);
}
