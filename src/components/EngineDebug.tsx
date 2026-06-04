import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import type { EngineSnapshot, EventCfg, TimedEventCfg } from "@/lib/types";

const pct = (v: number) => `${Math.round(v * 100)}%`;

const timestamp = (atMs: number) => {
	const d = new Date(atMs);
	const ms = String(d.getMilliseconds()).padStart(3, "0");
	return `${d.toLocaleTimeString([], { hour12: false })}.${ms}`;
};

function describeEvent(name: string, cfg: EventCfg | TimedEventCfg): string {
	if (!cfg.enabled) return `${name}: off`;
	const duration =
		"duration_ms" in cfg ? ` for ${(cfg as TimedEventCfg).duration_ms}ms` : "";
	return `${name}: ${pct(cfg.volume)}${duration}`;
}

export function EngineDebug() {
	const [snap, setSnap] = useState<EngineSnapshot | null>(null);
	const [path, setPath] = useState("");
	const [message, setMessage] = useState("");

	useEffect(() => {
		const unlisten = listen<EngineSnapshot>("engine:update", (event) => {
			setSnap(event.payload);
		});
		invoke<EngineSnapshot>("engine_status").then(setSnap, () => {});
		invoke<string>("config_path").then(setPath, () => {});
		return () => {
			unlisten.then((fn) => fn());
		};
	}, []);

	const runConfigCommand = async (cmd: "reload_config" | "reset_config") => {
		setMessage("");
		try {
			const { warnings } = await invoke<{ warnings: string[] }>(cmd);
			setMessage(
				warnings.length
					? `loaded with ${warnings.length} warning(s)`
					: cmd === "reset_config"
						? "reset to defaults (old file backed up to config.toml.bak)"
						: "config reloaded",
			);
		} catch (e) {
			setMessage(`error: ${e}`);
		}
	};

	const config = snap?.config;
	const warnings = snap?.config_warnings ?? [];

	return (
		<Card>
			<CardHeader>
				<CardTitle className="flex items-center gap-2">
					Event Engine
					{snap?.paused ? (
						<Badge variant="outline">paused — 100%</Badge>
					) : snap?.target != null ? (
						<Badge variant="destructive">reduced → {pct(snap.target)}</Badge>
					) : (
						<Badge variant="secondary">idle, 100%</Badge>
					)}
					{snap?.active.map((a) => (
						<Badge key={a.kind} variant="outline">
							{a.kind} {pct(a.volume)}
						</Badge>
					))}
				</CardTitle>
				<CardDescription>
					Live volume reduce/restore decisions from GSI events. Edit
					config.toml, then reload to apply.
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-4">
				<div className="flex flex-wrap items-center gap-2">
					<Button
						variant={snap?.paused ? "default" : "secondary"}
						onClick={() => invoke("set_paused", { paused: !snap?.paused })}
					>
						{snap?.paused ? "Resume" : "Pause"}
					</Button>
					<Button onClick={() => runConfigCommand("reload_config")}>
						Reload config
					</Button>
					<Button
						variant="secondary"
						onClick={() => runConfigCommand("reset_config")}
					>
						Reset to defaults
					</Button>
				</div>
				{message && (
					<p className="font-mono text-sm text-muted-foreground">{message}</p>
				)}
				{warnings.length > 0 && (
					<ul className="space-y-1 font-mono text-sm text-amber-500">
						{warnings.map((w) => (
							<li key={w}>⚠ {w}</li>
						))}
					</ul>
				)}
				{path && (
					<p className="font-mono text-xs text-muted-foreground">{path}</p>
				)}
				{config && (
					<p className="font-mono text-sm text-muted-foreground">
						{describeEvent("death", config.death)} ·{" "}
						{describeEvent("flash", config.flash)} ·{" "}
						{describeEvent("bomb", config.bomb)} ·{" "}
						{describeEvent("spectator", config.spectator)}
					</p>
				)}
				{snap && snap.log.length > 0 && (
					<div className="max-h-64 space-y-1 overflow-auto rounded-md bg-muted p-3 font-mono text-xs">
						{[...snap.log].reverse().map((entry, i) => (
							// biome-ignore lint/suspicious/noArrayIndexKey: append-only capped debug log; at_ms is not unique within a tick
							<div key={`${entry.at_ms}-${i}`} className="flex gap-2">
								<span className="shrink-0 text-muted-foreground">
									{timestamp(entry.at_ms)}
								</span>
								<span>{entry.trigger}</span>
								<span className="text-muted-foreground">
									→ {entry.decision}
								</span>
							</div>
						))}
					</div>
				)}
			</CardContent>
		</Card>
	);
}
