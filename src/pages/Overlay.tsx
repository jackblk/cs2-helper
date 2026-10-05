import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { cn } from "cn";
import { Check, Minus, Plus, X } from "lucide-react";
import {
	type CSSProperties,
	type PointerEvent,
	type ReactNode,
	useEffect,
	useRef,
	useState,
} from "react";
import { Button } from "@/components/ui/button";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { C4_FUSE_DEFAULT, type DefuseColor, defuseColor } from "@/lib/defuse";
import {
	clampToMonitor,
	OVERLAY_BASE,
	overlaySize,
	placeToolbar,
	SCALE_MAX,
	SCALE_MIN,
	STATUS_SLOT_H,
	stepScale,
	TOOLBAR_SIZE,
} from "@/lib/overlay-edit";
import type {
	ActiveReduction,
	AppConfig,
	EngineSnapshot,
	GameState,
	GsiUpdate,
	OverlayConfig,
	OverlayEditResult,
	OverlayEditSession,
} from "@/lib/types";
import { pct } from "@/lib/utils";

const DEFAULT_OVERLAY: OverlayConfig = {
	enabled: true,
	bomb_timer: true,
	volume_status: true,
	c4_timer_s: C4_FUSE_DEFAULT,
	safety_margin_s: 0,
	pos_x: null,
	pos_y: null,
	scale: 1,
};

// Show the badge for a short grace period after detonation so 0.0 is visible.
const GRACE_MS = 1500;

// Reading shown in edit mode, so the user positions a realistic badge.
const SAMPLE_S = 32.5;

// Length of the main page's Preview, which counts down from SAMPLE_S.
const PREVIEW_S = 5;

// Volume status shown in edit mode and the Preview.
const SAMPLE_STATUS: ActiveReduction = { kind: "flash", volume: 0.15 };

const STATUS_LABELS: Record<string, string> = {
	flash: "flash",
	death: "death",
	bomb: "bomb",
	spectator: "spec",
	deathmatch: "deathmatch mode",
};

// The reduction that sets the volume (the smallest wins), or null at 100%.
function topReduction(snap: EngineSnapshot | null): ActiveReduction | null {
	if (!snap || snap.paused || snap.target == null) return null;
	return snap.active.reduce<ActiveReduction | null>(
		(min, a) => (min == null || a.volume < min.volume ? a : min),
		null,
	);
}

const endEdit = (result: OverlayEditResult | null) =>
	invoke("overlay_edit_end", { result }).catch(() => {});

export function Overlay() {
	const [cfg, setCfg] = useState<OverlayConfig>(DEFAULT_OVERLAY);
	const [session, setSession] = useState<OverlayEditSession | null>(null);
	// Debug page toggle: show the idle placeholder even without a plant.
	const [preview, setPreview] = useState(false);
	const [state, setState] = useState<GameState | null>(null);
	const [engine, setEngine] = useState<EngineSnapshot | null>(null);
	const [remaining, setRemaining] = useState<number | null>(null);
	// Seconds left of the main page's timed preview.
	const [previewLeft, setPreviewLeft] = useState<number | null>(null);

	// performance.now() when the bomb was planted.
	const plantedAt = useRef<number | null>(null);
	// performance.now() when the timed preview ends.
	const previewEnd = useRef<number | null>(null);
	const prevBomb = useRef<string | null>(null);

	useEffect(() => {
		invoke<AppConfig>("get_app_settings").then(
			(a) => setCfg(a.overlay),
			() => {},
		);
		// A window created for the edit loads after `overlay:edit` fired.
		invoke<OverlayEditSession | null>("overlay_edit_session").then(
			setSession,
			() => {},
		);
		invoke<EngineSnapshot>("engine_status").then(setEngine, () => {});
		const unlisteners = [
			listen<OverlayConfig>("overlay:config", (e) => setCfg(e.payload)),
			listen<OverlayEditSession | null>("overlay:edit", (e) =>
				setSession(e.payload),
			),
			listen<boolean>("overlay:preview", (e) => setPreview(e.payload)),
			// Pressing Preview again restarts it.
			listen("overlay:preview-start", () => {
				previewEnd.current = performance.now() + PREVIEW_S * 1000;
			}),
			listen<GsiUpdate>("gsi:update", (e) => setState(e.payload.state)),
			listen<EngineSnapshot>("engine:update", (e) => setEngine(e.payload)),
		];
		return () => {
			for (const un of unlisteners) un.then((f) => f());
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
			if (previewEnd.current == null) {
				setPreviewLeft(null);
			} else {
				const left = (previewEnd.current - performance.now()) / 1000;
				if (left <= 0) {
					previewEnd.current = null;
					setPreviewLeft(null);
				} else {
					setPreviewLeft(left);
				}
			}
			raf = requestAnimationFrame(tick);
		};
		raf = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(raf);
	}, [cfg.c4_timer_s, cfg.safety_margin_s]);

	if (session) {
		return <EditSurface session={session} statusOn={cfg.volume_status} />;
	}

	const live = remaining != null && cfg.bomb_timer;
	// A live timer wins over the timed preview.
	const previewing = !live && previewLeft != null;
	let status = cfg.volume_status ? topReduction(engine) : null;
	if (previewing && cfg.volume_status) status ??= SAMPLE_STATUS;

	let badge: ReactNode = null;
	if (previewing) {
		badge = (
			<TimerBadge
				color="green"
				value={(SAMPLE_S - PREVIEW_S + previewLeft).toFixed(1)}
				caption="preview"
			/>
		);
	} else if (preview || live) {
		const color =
			remaining == null
				? null
				: defuseColor(
						state?.team ?? null,
						state?.defusekit ?? false,
						remaining,
					);
		badge = (
			<TimerBadge
				color={color}
				value={remaining == null ? "--.-" : remaining.toFixed(1)}
				caption={
					color == null ? "bomb timer" : color === "red" ? "run" : "defuse"
				}
			/>
		);
	}
	if (!badge && !status) return null;

	return (
		<OverlayFrame
			scale={cfg.scale}
			status={status && <StatusPill reduction={status} />}
			badge={badge}
		/>
	);
}

// Lays out OVERLAY_BASE in base px and scales the whole of it to the window
// (or edit box), so the status slot and the badge keep their proportions.
function OverlayFrame({
	scale,
	status,
	badge,
}: {
	scale: number;
	status: ReactNode;
	badge: ReactNode;
}) {
	return (
		<div
			style={{
				width: OVERLAY_BASE.width,
				height: OVERLAY_BASE.height,
				transform: `scale(${scale})`,
				transformOrigin: "top left",
			}}
			className="flex select-none flex-col"
		>
			<div
				style={{ height: STATUS_SLOT_H }}
				className="flex items-end justify-center"
			>
				{status}
			</div>
			<div className="flex flex-1 items-center justify-center">{badge}</div>
		</div>
	);
}

function StatusPill({ reduction }: { reduction: ActiveReduction }) {
	return (
		<div className="rounded-full bg-black/70 px-3 py-1 font-mono text-xs font-semibold uppercase tabular-nums tracking-widest text-amber-400 shadow-lg backdrop-blur-sm">
			{STATUS_LABELS[reduction.kind] ?? reduction.kind} {pct(reduction.volume)}
		</div>
	);
}

function TimerBadge({
	color,
	value,
	caption,
}: {
	color: DefuseColor | null;
	value: string;
	caption: string;
}) {
	return (
		<div
			className={cn(
				"rounded-lg bg-black/70 px-4 py-2 text-center font-mono tabular-nums tracking-tight shadow-lg backdrop-blur-sm",
				color == null
					? "text-white/40"
					: color === "green"
						? "text-emerald-400"
						: "text-red-500",
			)}
		>
			<div className="text-4xl font-bold leading-none">{value}</div>
			<div className="mt-1 text-[10px] uppercase tracking-widest text-white/70">
				{caption}
			</div>
		</div>
	);
}

// The window covers its monitor during an edit; client coordinates are
// logical px relative to the monitor's origin.
function EditSurface({
	session,
	statusOn,
}: {
	session: OverlayEditSession;
	statusOn: boolean;
}) {
	const { monitor } = session;
	const [scale, setScale] = useState(session.scale);
	const [origin, setOrigin] = useState({ x: session.x, y: session.y });
	const box = clampToMonitor({ ...origin, ...overlaySize(scale) }, monitor);
	const bar = placeToolbar(box, monitor);
	const grab = useRef<{ dx: number; dy: number } | null>(null);

	// The main window's Done/Cancel end the edit with this window's unsaved
	// geometry.
	const latest = useRef({ box, scale });
	latest.current = { box, scale };
	useEffect(() => {
		const un = listen<"ok" | "cancel">("overlay:edit-request", (e) => {
			const { box, scale } = latest.current;
			endEdit(e.payload === "ok" ? { x: box.x, y: box.y, scale } : null);
		});
		return () => {
			un.then((f) => f());
		};
	}, []);

	useEffect(() => {
		const onKey = (e: KeyboardEvent) => {
			if (e.key === "Escape") endEdit(null);
		};
		window.addEventListener("keydown", onKey);
		return () => window.removeEventListener("keydown", onKey);
	}, []);

	const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
		if (e.button !== 0) return;
		e.currentTarget.setPointerCapture(e.pointerId);
		grab.current = {
			dx: monitor.x + e.clientX - box.x,
			dy: monitor.y + e.clientY - box.y,
		};
	};
	const onPointerMove = (e: PointerEvent<HTMLDivElement>) => {
		const g = grab.current;
		if (!g) return;
		const next = clampToMonitor(
			{
				x: monitor.x + e.clientX - g.dx,
				y: monitor.y + e.clientY - g.dy,
				...overlaySize(scale),
			},
			monitor,
		);
		setOrigin({ x: next.x, y: next.y });
	};
	const onPointerUp = () => {
		grab.current = null;
	};

	return (
		<div className="relative h-screen w-screen select-none bg-black/20">
			<div
				onPointerDown={onPointerDown}
				onPointerMove={onPointerMove}
				onPointerUp={onPointerUp}
				onPointerCancel={onPointerUp}
				// An outline, unlike a border, leaves the frame the full box.
				className="absolute cursor-move rounded-lg outline-2 -outline-offset-2 outline-dashed outline-white/70"
				style={{
					left: box.x - monitor.x,
					top: box.y - monitor.y,
					width: box.width,
					height: box.height,
				}}
			>
				<OverlayFrame
					scale={scale}
					status={statusOn && <StatusPill reduction={SAMPLE_STATUS} />}
					badge={
						<TimerBadge
							color="green"
							value={SAMPLE_S.toFixed(1)}
							caption={`drag to move · ${scale.toFixed(2)}x`}
						/>
					}
				/>
			</div>
			<EditToolbar
				style={{ left: bar.x - monitor.x, top: bar.y - monitor.y }}
				onCancel={() => endEdit(null)}
				onSmaller={
					scale > SCALE_MIN
						? () => setScale((s) => stepScale(s, -1))
						: undefined
				}
				onLarger={
					scale < SCALE_MAX ? () => setScale((s) => stepScale(s, 1)) : undefined
				}
				onSave={() => endEdit({ x: box.x, y: box.y, scale })}
			/>
		</div>
	);
}

function EditToolbar({
	style,
	onCancel,
	onSmaller,
	onLarger,
	onSave,
}: {
	style: CSSProperties;
	onCancel: () => void;
	onSmaller?: () => void;
	onLarger?: () => void;
	onSave: () => void;
}) {
	return (
		// The overlay renders outside Layout, so it needs its own provider.
		<TooltipProvider>
			<div
				className="absolute flex items-center gap-1 rounded-lg bg-black/80 p-1 shadow-lg"
				style={{
					...style,
					width: TOOLBAR_SIZE.width,
					height: TOOLBAR_SIZE.height,
				}}
			>
				<ToolbarButton
					label="Cancel (Esc)"
					onClick={onCancel}
					className="text-red-500 hover:bg-red-500/15 hover:text-red-500"
				>
					<X />
				</ToolbarButton>
				<ToolbarButton label="Smaller" onClick={onSmaller}>
					<Minus />
				</ToolbarButton>
				<ToolbarButton label="Larger" onClick={onLarger}>
					<Plus />
				</ToolbarButton>
				<ToolbarButton
					label="Save"
					onClick={onSave}
					className="text-emerald-400 hover:bg-emerald-400/15 hover:text-emerald-400"
				>
					<Check />
				</ToolbarButton>
			</div>
		</TooltipProvider>
	);
}

function ToolbarButton({
	label,
	onClick,
	className = "text-white hover:bg-white/15 hover:text-white",
	children,
}: {
	label: string;
	onClick?: () => void;
	className?: string;
	children: ReactNode;
}) {
	return (
		<Tooltip>
			<TooltipTrigger asChild>
				<Button
					variant="ghost"
					size="icon"
					aria-label={label}
					disabled={!onClick}
					onClick={onClick}
					className={className}
				>
					{children}
				</Button>
			</TooltipTrigger>
			<TooltipContent>{label}</TooltipContent>
		</Tooltip>
	);
}
