import type { Rect } from "@/lib/types";

export type Size = { width: number; height: number };

// Mirrors BASE_W / BASE_H in src-tauri/src/overlay.rs.
export const OVERLAY_BASE: Size = { width: 240, height: 128 };
// The volume status slot at the top of OVERLAY_BASE; the badge fills the rest.
export const STATUS_SLOT_H = 32;
// Mirrors `scale` in src-tauri/src/config/default.toml.
export const DEFAULT_OVERLAY_SCALE = 1;

export const SCALE_MIN = 0.5;
export const SCALE_MAX = 2.5;
export const SCALE_STEP = 0.05;

// Four size-8 icon buttons, gap-1, p-1.
export const TOOLBAR_SIZE: Size = { width: 148, height: 40 };
export const TOOLBAR_GAP = 8;

export function overlaySize(scale: number): Size {
	return {
		width: OVERLAY_BASE.width * scale,
		height: OVERLAY_BASE.height * scale,
	};
}

export function stepScale(scale: number, dir: 1 | -1): number {
	const next = Math.round((scale + dir * SCALE_STEP) * 100) / 100;
	return Math.min(SCALE_MAX, Math.max(SCALE_MIN, next));
}

// `hi < lo` (item larger than the room) pins to `lo`.
const clamp = (v: number, lo: number, hi: number) =>
	Math.min(Math.max(v, lo), Math.max(lo, hi));

export function clampToMonitor(r: Rect, m: Rect): Rect {
	return {
		...r,
		x: clamp(r.x, m.x, m.x + m.width - r.width),
		y: clamp(r.y, m.y, m.y + m.height - r.height),
	};
}

export function placeToolbar(
	overlay: Rect,
	monitor: Rect,
	toolbar: Size = TOOLBAR_SIZE,
	gap: number = TOOLBAR_GAP,
): { x: number; y: number; side: "above" | "below" } {
	const x = clamp(
		overlay.x + (overlay.width - toolbar.width) / 2,
		monitor.x,
		monitor.x + monitor.width - toolbar.width,
	);
	const above = overlay.y - gap - toolbar.height;
	if (above >= monitor.y) return { x, y: above, side: "above" };
	return { x, y: overlay.y + overlay.height + gap, side: "below" };
}
