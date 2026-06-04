import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
	Check,
	ChevronDown,
	ChevronRight,
	FolderOpen,
	Info,
	X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import type { AppConfig, Config, EngineSnapshot, GsiStatus } from "@/lib/types";

const FRESH_MS = 10000;

const pct = (v: number) => `${Math.round(v * 100)}%`;

type Draft = { config: Config; app: AppConfig };

function eq(a: Draft, b: Draft): boolean {
	return JSON.stringify(a) === JSON.stringify(b);
}

function EventRow({
	label,
	enabled,
	volume,
	durationMs,
	hintPrefix,
	onEnabled,
	onVolume,
	onDuration,
}: {
	label: string;
	enabled: boolean;
	volume: number;
	durationMs?: number;
	// Tooltip lead-in, e.g. "When you're flashed" or "At death". Renders as
	// "{hintPrefix}, reduce audio to X%" (+ " for Y ms" on timed events).
	hintPrefix: string;
	onEnabled: (v: boolean) => void;
	onVolume: (v: number) => void;
	onDuration?: (ms: number) => void;
}) {
	const timed = durationMs != null && onDuration != null;
	// Local text so the field can be cleared/retyped; committed only when valid.
	const [durText, setDurText] = useState(
		durationMs != null ? String(durationMs) : "",
	);
	useEffect(() => {
		setDurText(durationMs != null ? String(durationMs) : "");
	}, [durationMs]);

	const hint = timed
		? `${hintPrefix}, reduce audio to ${pct(volume)} for ${durationMs} ms.`
		: `${hintPrefix}, reduce audio to ${pct(volume)}.`;

	return (
		<div className="flex items-center gap-3">
			<div className="flex w-36 shrink-0 items-center gap-1.5">
				<Switch checked={enabled} onCheckedChange={onEnabled} />
				<Label className="font-medium">{label}</Label>
				<Tooltip>
					<TooltipTrigger asChild>
						<button
							type="button"
							className="text-muted-foreground transition-colors hover:text-foreground"
							aria-label={`${label} info`}
						>
							<Info className="size-3.5" />
						</button>
					</TooltipTrigger>
					<TooltipContent>{hint}</TooltipContent>
				</Tooltip>
			</div>
			<Slider
				value={[volume * 100]}
				min={0}
				max={100}
				step={5}
				disabled={!enabled}
				onValueChange={([v]) => onVolume(v / 100)}
				className="min-w-0 flex-1"
			/>
			<span className="w-10 shrink-0 text-right font-mono text-sm text-muted-foreground">
				{pct(volume)}
			</span>
			{timed ? (
				<div className="flex w-28 shrink-0 items-center gap-1">
					<Input
						type="number"
						min={1}
						step={250}
						disabled={!enabled}
						value={durText}
						onChange={(e) => {
							const text = e.currentTarget.value;
							setDurText(text);
							const ms = Number(text);
							if (Number.isFinite(ms) && ms > 0) onDuration(Math.round(ms));
						}}
						className="h-8 w-20"
						aria-label={`${label} duration in milliseconds`}
					/>
					<span className="text-xs text-muted-foreground">ms</span>
				</div>
			) : (
				// Spacer so volume sliders align across timed/untimed rows.
				<div className="w-28 shrink-0" />
			)}
		</div>
	);
}

function ChecklistRow({ ok, label }: { ok: boolean; label: string }) {
	return (
		<div className="flex items-center gap-2 text-sm">
			{ok ? (
				<Check className="size-4 text-emerald-500" />
			) : (
				<X className="size-4 text-muted-foreground" />
			)}
			<span className={ok ? "" : "text-muted-foreground"}>{label}</span>
		</div>
	);
}

export function MainPage() {
	const [snap, setSnap] = useState<EngineSnapshot | null>(null);
	const [gsi, setGsi] = useState<GsiStatus | null>(null);
	const [draft, setDraft] = useState<Draft | null>(null);
	const [saved, setSaved] = useState<Draft | null>(null);
	// null = follow install state (collapsed once done); a bool = user override.
	const [setupOpen, setSetupOpen] = useState<boolean | null>(null);
	// Latest `saved`, read inside the seed effect without making it a dependency
	// (which would re-fire — and loop — on every setSaved).
	const savedRef = useRef<Draft | null>(null);
	savedRef.current = saved;

	useEffect(() => {
		const unlisten = listen<EngineSnapshot>("engine:update", (e) =>
			setSnap(e.payload),
		);
		invoke<EngineSnapshot>("engine_status").then(setSnap, () => {});
		const refreshGsi = () =>
			invoke<GsiStatus>("gsi_status").then(setGsi, () => {});
		refreshGsi();
		const timer = setInterval(refreshGsi, 1000);
		return () => {
			clearInterval(timer);
			unlisten.then((fn) => fn());
		};
	}, []);

	// Seed the editable draft from the engine config, and reseed when the config
	// changes externally (tray reload/reset) while the draft is clean. The engine
	// emits a snapshot on every gameplay event carrying an identical config, so
	// ignore snapshots whose config content already matches what we have saved.
	useEffect(() => {
		const cfg = snap?.config;
		if (!cfg) return;
		if (
			savedRef.current &&
			JSON.stringify(cfg) === JSON.stringify(savedRef.current.config)
		) {
			return;
		}
		invoke<AppConfig>("get_app_settings").then(
			(app) => {
				const next: Draft = { config: cfg, app };
				// Don't clobber an in-progress edit.
				setDraft((cur) =>
					cur && savedRef.current && !eq(cur, savedRef.current) ? cur : next,
				);
				setSaved(next);
			},
			() => {},
		);
	}, [snap?.config]);

	const paused = snap?.paused ?? false;
	const fresh =
		gsi?.last_payload_age_ms != null && gsi.last_payload_age_ms < FRESH_MS;

	const cfgInstalled = gsi?.cfg_installed ?? false;
	// Collapse automatically once setup is done; expand if not. A user click
	// (setupOpen non-null) overrides until they navigate away.
	const setupExpanded = setupOpen ?? !cfgInstalled;

	const installCfg = async () => {
		try {
			await invoke<string>("install_gsi_cfg");
			toast.success("Installed GSI to CS2. Restart CS2 to take effect");
		} catch (e) {
			toast.error("Install failed", { description: String(e) });
		}
	};

	const openCfgDir = async () => {
		try {
			await invoke("open_gsi_cfg_dir");
		} catch (e) {
			toast.error("Couldn't open folder", { description: String(e) });
		}
	};

	const openConfigDir = async () => {
		try {
			await invoke("open_config_dir");
		} catch (e) {
			toast.error("Couldn't open folder", { description: String(e) });
		}
	};

	const dirty = draft != null && saved != null && !eq(draft, saved);

	const patchConfig = (f: (c: Config) => Config) =>
		setDraft((d) => (d ? { ...d, config: f(d.config) } : d));

	const save = async () => {
		if (!draft) return;
		try {
			await invoke("save_config", { config: draft.config, app: draft.app });
			setSaved(draft);
			toast.success("Settings saved");
		} catch (e) {
			toast.error("Save failed", { description: String(e) });
		}
	};

	const discard = () => setDraft(saved);

	const resetDefaults = async () => {
		try {
			// reset_config returns the new (default) config, so seed the form
			// directly — don't wait for the async engine:update, which can race
			// command completion and leave the form stuck.
			const { config, app } = await invoke<{
				config: Config;
				app: AppConfig;
			}>("reset_config");
			const next: Draft = { config, app };
			setDraft(next);
			setSaved(next);
			toast.success("Reset to defaults", {
				description: "Old file backed up to config.toml.bak.",
			});
		} catch (e) {
			toast.error("Reset failed", { description: String(e) });
		}
	};

	return (
		<div className="space-y-6">
			<Card>
				<Collapsible
					open={setupExpanded}
					onOpenChange={(open) => setSetupOpen(open)}
				>
					<CardHeader>
						<CollapsibleTrigger asChild>
							<Button
								variant="ghost"
								className="-mx-2 h-auto w-[calc(100%+1rem)] justify-start gap-2 px-2 py-1 hover:bg-transparent"
							>
								<CardTitle className="flex items-center gap-2">
									Setup
									{cfgInstalled ? (
										<Check className="size-5 text-emerald-500" />
									) : (
										<X className="size-5 text-muted-foreground" />
									)}
								</CardTitle>
								{setupExpanded ? (
									<ChevronDown className="ml-auto size-4 text-muted-foreground" />
								) : (
									<ChevronRight className="ml-auto size-4 text-muted-foreground" />
								)}
							</Button>
						</CollapsibleTrigger>
						{setupExpanded && (
							<CardDescription>
								One-time setup so CS2 can report game events to the app.
							</CardDescription>
						)}
					</CardHeader>
					<CollapsibleContent>
						<CardContent className="space-y-4">
							<ChecklistRow ok={cfgInstalled} label="GSI config installed" />
							<div className="flex flex-wrap items-center gap-2">
								<Button onClick={installCfg}>Install GSI config</Button>
								<Button
									variant="outline"
									onClick={openCfgDir}
									disabled={gsi != null && gsi.cfg_path == null}
								>
									<FolderOpen className="size-4" />
									Open CS2 Config folder
								</Button>
							</div>
							{gsi && !gsi.cfg_installed && gsi.cfg_path == null && (
								<p className="text-sm text-muted-foreground">
									CS2 installation not found — install CS2, then try again.
								</p>
							)}
						</CardContent>
					</CollapsibleContent>
				</Collapsible>
			</Card>

			<Card>
				<CardHeader>
					<div className="flex items-start justify-between gap-2">
						<div className="space-y-1.5">
							<CardTitle className="flex items-center gap-2">
								Status
								{paused ? (
									<Badge variant="outline">Paused</Badge>
								) : fresh ? (
									<Badge variant="secondary">Running</Badge>
								) : (
									<Badge variant="outline">Waiting for CS2</Badge>
								)}
							</CardTitle>
							<CardDescription className="flex flex-wrap items-center gap-2">
								Audio:
								{paused ? (
									<Badge variant="outline">paused — 100%</Badge>
								) : snap?.target != null ? (
									<Badge variant="destructive">
										reduced → {pct(snap.target)}
									</Badge>
								) : (
									<Badge variant="secondary">idle, 100%</Badge>
								)}
								{!paused &&
									snap?.active.map((a) => (
										<Badge key={a.kind} variant="outline">
											{a.kind} {pct(a.volume)}
										</Badge>
									))}
							</CardDescription>
						</div>
						<Button
							variant={paused ? "default" : "secondary"}
							onClick={() => invoke("set_paused", { paused: !paused })}
						>
							{paused ? "Resume" : "Pause"}
						</Button>
					</div>
				</CardHeader>
			</Card>

			<Card>
				<CardHeader>
					<CardTitle>Settings</CardTitle>
					<CardDescription>
						Volumes are absolute targets. When several events overlap, the
						lowest wins. Volume always restores to 100%.
					</CardDescription>
				</CardHeader>
				<CardContent className="space-y-4">
					{draft ? (
						<>
							<div className="space-y-3">
								<EventRow
									label="Flash"
									hintPrefix="When you're flashed"
									enabled={draft.config.flash.enabled}
									volume={draft.config.flash.volume}
									onEnabled={(v) =>
										patchConfig((c) => ({
											...c,
											flash: { ...c.flash, enabled: v },
										}))
									}
									onVolume={(v) =>
										patchConfig((c) => ({
											...c,
											flash: { ...c.flash, volume: v },
										}))
									}
								/>
								<EventRow
									label="Death"
									hintPrefix="At death"
									enabled={draft.config.death.enabled}
									volume={draft.config.death.volume}
									durationMs={draft.config.death.duration_ms}
									onEnabled={(v) =>
										patchConfig((c) => ({
											...c,
											death: { ...c.death, enabled: v },
										}))
									}
									onVolume={(v) =>
										patchConfig((c) => ({
											...c,
											death: { ...c.death, volume: v },
										}))
									}
									onDuration={(ms) =>
										patchConfig((c) => ({
											...c,
											death: { ...c.death, duration_ms: ms },
										}))
									}
								/>
								<EventRow
									label="Bomb"
									hintPrefix="When the bomb explodes"
									enabled={draft.config.bomb.enabled}
									volume={draft.config.bomb.volume}
									durationMs={draft.config.bomb.duration_ms}
									onEnabled={(v) =>
										patchConfig((c) => ({
											...c,
											bomb: { ...c.bomb, enabled: v },
										}))
									}
									onVolume={(v) =>
										patchConfig((c) => ({
											...c,
											bomb: { ...c.bomb, volume: v },
										}))
									}
									onDuration={(ms) =>
										patchConfig((c) => ({
											...c,
											bomb: { ...c.bomb, duration_ms: ms },
										}))
									}
								/>
								<EventRow
									label="Spectator"
									hintPrefix="When you're spectating"
									enabled={draft.config.spectator.enabled}
									volume={draft.config.spectator.volume}
									onEnabled={(v) =>
										patchConfig((c) => ({
											...c,
											spectator: { ...c.spectator, enabled: v },
										}))
									}
									onVolume={(v) =>
										patchConfig((c) => ({
											...c,
											spectator: { ...c.spectator, volume: v },
										}))
									}
								/>
							</div>

							<Separator />

							<div className="space-y-2">
								<div className="flex items-center gap-2">
									<Checkbox
										id="startup"
										checked={draft.app.start_with_windows}
										onCheckedChange={(v) =>
											setDraft((d) =>
												d
													? {
															...d,
															app: { ...d.app, start_with_windows: v === true },
														}
													: d,
											)
										}
									/>
									<Label htmlFor="startup">Run on startup</Label>
								</div>
								<div className="flex items-center gap-2">
									<Checkbox
										id="start-minimized"
										checked={draft.app.start_minimized}
										onCheckedChange={(v) =>
											setDraft((d) =>
												d
													? {
															...d,
															app: { ...d.app, start_minimized: v === true },
														}
													: d,
											)
										}
									/>
									<Label htmlFor="start-minimized">
										Start minimized to tray
									</Label>
								</div>
							</div>

							<div className="flex items-center gap-2">
								<Button variant="outline" onClick={resetDefaults}>
									Reset to defaults
								</Button>
								<Button variant="outline" onClick={openConfigDir}>
									<FolderOpen className="size-4" />
									Open config folder
								</Button>
								<div className="ml-auto flex items-center gap-2">
									{dirty && (
										<span className="text-sm text-amber-500">
											• unsaved changes
										</span>
									)}
									<Button
										variant="secondary"
										disabled={!dirty}
										onClick={discard}
									>
										Discard
									</Button>
									<Button disabled={!dirty} onClick={save}>
										Save
									</Button>
								</div>
							</div>
						</>
					) : (
						<p className="text-sm text-muted-foreground">Loading settings…</p>
					)}

					{snap && snap.config_warnings.length > 0 && (
						<div className="space-y-2 rounded-md border border-amber-500/40 p-3">
							<p className="text-sm text-amber-500">
								Your config file had problems and some values fell back to
								defaults:
							</p>
							<ul className="space-y-1 font-mono text-xs text-amber-500">
								{snap.config_warnings.map((w) => (
									<li key={w}>⚠ {w}</li>
								))}
							</ul>
							<p className="text-xs text-amber-500/80">
								Use “Reset to defaults” above to start clean.
							</p>
						</div>
					)}
				</CardContent>
			</Card>
		</div>
	);
}
