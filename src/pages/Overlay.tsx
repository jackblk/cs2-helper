import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import { C4_FUSE_DEFAULT, defuseColor } from "@/lib/defuse";
import type {
	AppConfig,
	GameState,
	GsiUpdate,
	OverlayConfig,
} from "@/lib/types";
import { cn } from "@/lib/utils";

const DEFAULT_OVERLAY: OverlayConfig = {
	enabled: true,
	c4_timer_s: C4_FUSE_DEFAULT,
	safety_margin_s: 0,
	pos_x: null,
	pos_y: null,
	scale: 1,
};

// Show the badge for a short grace period after detonation so 0.0 is visible.
const GRACE_MS = 1500;

// Reading shown while the settings Size slider previews a new size, so the
// user sees a realistic badge instead of the dim placeholder.
const SAMPLE_S = 32.5;

export function Overlay() {
	const [cfg, setCfg] = useState<OverlayConfig>(DEFAULT_OVERLAY);
	const [editing, setEditing] = useState(false);
	// Debug page toggle: show the idle placeholder even without a plant.
	const [preview, setPreview] = useState(false);
	const [sizePreview, setSizePreview] = useState(false);
	const [state, setState] = useState<GameState | null>(null);
	const [remaining, setRemaining] = useState<number | null>(null);

	// performance.now() when the bomb was planted.
	const plantedAt = useRef<number | null>(null);
	const prevBomb = useRef<string | null>(null);

	useEffect(() => {
		invoke<AppConfig>("get_app_settings").then(
			(a) => setCfg(a.overlay),
			() => {},
		);
		const unCfg = listen<OverlayConfig>("overlay:config", (e) =>
			setCfg(e.payload),
		);
		const unEdit = listen<boolean>("overlay:edit", (e) =>
			setEditing(e.payload),
		);
		const unPreview = listen<boolean>("overlay:preview", (e) =>
			setPreview(e.payload),
		);
		const unSample = listen<boolean>("overlay:sample", (e) =>
			setSizePreview(e.payload),
		);
		const unGsi = listen<GsiUpdate>("gsi:update", (e) =>
			setState(e.payload.state),
		);
		return () => {
			unCfg.then((f) => f());
			unEdit.then((f) => f());
			unPreview.then((f) => f());
			unSample.then((f) => f());
			unGsi.then((f) => f());
		};
	}, []);

	useEffect(() => {
		const bomb = state?.bomb ?? null;
		if (bomb === "planted" && prevBomb.current !== "planted") {
			plantedAt.current = performance.now();
		} else if (bomb !== "planted" && bomb !== "defusing") {
			// "defused", "exploded", or gone.
			plantedAt.current = null;
		}
		prevBomb.current = bomb;
	}, [state?.bomb]);

	useEffect(() => {
		let raf = 0;
		const tick = () => {
			if (plantedAt.current == null) {
				setRemaining(null);
			} else {
				const elapsed = (performance.now() - plantedAt.current) / 1000;
				const left = cfg.c4_timer_s - elapsed - cfg.safety_margin_s;
				if (left <= -GRACE_MS / 1000) {
					plantedAt.current = null;
					setRemaining(null);
				} else {
					setRemaining(Math.max(0, left));
				}
			}
			raf = requestAnimationFrame(tick);
		};
		raf = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(raf);
	}, [cfg.c4_timer_s, cfg.safety_margin_s]);

	// Edit mode: a left press anywhere in the window drags it. Started from JS
	// rather than `data-tauri-drag-region`, which only fires when the pressed
	// element itself carries the attribute, so pressing the badge (a child)
	// would not drag.
	useEffect(() => {
		if (!editing) return;
		const onMouseDown = (e: MouseEvent) => {
			if (e.button === 0) getCurrentWindow().startDragging();
		};
		window.addEventListener("mousedown", onMouseDown);
		return () => window.removeEventListener("mousedown", onMouseDown);
	}, [editing]);

	const show = editing || preview || sizePreview || remaining != null;
	if (!show) return null;

	const shown = remaining ?? (sizePreview && !editing ? SAMPLE_S : null);
	const idle = !editing && shown == null;

	const color =
		remaining == null
			? "green"
			: defuseColor(state?.team ?? null, state?.defusekit ?? false, remaining);

	return (
		<div
			className={cn(
				"flex h-screen w-screen select-none flex-col items-center justify-center font-mono",
				editing &&
					"cursor-move rounded-lg border-2 border-dashed border-white/70",
			)}
		>
			<div
				// The window is resized by `scale` on the Rust side; scale the badge
				// too so its text grows with the window.
				style={{ transform: `scale(${cfg.scale})` }}
				className={cn(
					"rounded-lg px-4 py-2 text-center tabular-nums tracking-tight shadow-lg",
					"bg-black/70 backdrop-blur-sm",
					idle
						? "text-white/40"
						: color === "green"
							? "text-emerald-400"
							: "text-red-500",
				)}
			>
				<div className="text-4xl font-bold leading-none">
					{shown == null ? "--.-" : shown.toFixed(1)}
				</div>
				<div className="mt-1 text-[10px] uppercase tracking-widest text-white/70">
					{editing
						? "drag to move"
						: shown == null
							? "bomb timer"
							: color === "red"
								? "run"
								: "defuse"}
				</div>
			</div>
		</div>
	);
}
