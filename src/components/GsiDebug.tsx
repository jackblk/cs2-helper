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
import type { GsiStatus, GsiUpdate } from "@/lib/types";

function formatAge(ms: number | null): string {
	if (ms == null) return "never";
	if (ms < 1500) return "just now";
	return `${Math.round(ms / 1000)}s ago`;
}

export function GsiDebug() {
	const [status, setStatus] = useState<GsiStatus | null>(null);
	const [update, setUpdate] = useState<GsiUpdate | null>(null);
	const [count, setCount] = useState(0);
	const [message, setMessage] = useState("");

	useEffect(() => {
		const unlisten = listen<GsiUpdate>("gsi:update", (event) => {
			setUpdate(event.payload);
			setCount((c) => c + 1);
		});
		const refresh = () =>
			invoke<GsiStatus>("gsi_status").then(setStatus, () => {});
		refresh();
		const timer = setInterval(refresh, 1000);
		return () => {
			clearInterval(timer);
			unlisten.then((fn) => fn());
		};
	}, []);

	const installCfg = async () => {
		setMessage("");
		try {
			const path = await invoke<string>("install_gsi_cfg");
			setMessage(`Installed: ${path} — restart CS2 to apply.`);
		} catch (e) {
			setMessage(`error: ${e}`);
		}
	};

	const state = update?.state;

	return (
		<Card>
			<CardHeader>
				<CardTitle className="flex items-center gap-2">
					GSI Server
					{status?.running ? (
						<Badge variant="secondary">listening :{status.port}</Badge>
					) : (
						<Badge variant="destructive">not running</Badge>
					)}
					{status?.cfg_installed ? (
						<Badge variant="secondary">cfg installed</Badge>
					) : (
						<Badge variant="destructive">cfg missing</Badge>
					)}
				</CardTitle>
				<CardDescription>
					Live game state from CS2. Install the cfg, restart CS2, then join a
					match — payloads appear below.
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-4">
				<div className="flex flex-wrap items-center gap-2">
					<Button onClick={installCfg}>Install GSI config</Button>
					<span className="font-mono text-sm text-muted-foreground">
						{count} payload(s), last:{" "}
						{formatAge(status?.last_payload_age_ms ?? null)}
					</span>
				</div>
				{message && (
					<p className="font-mono text-sm text-muted-foreground">{message}</p>
				)}
				{status && !status.cfg_installed && (
					<p className="font-mono text-sm text-muted-foreground">
						target: {status.cfg_path ?? "CS2 installation not found"}
					</p>
				)}
				{state && (
					<dl className="grid grid-cols-2 gap-x-6 gap-y-1 font-mono text-sm sm:grid-cols-3">
						<dt className="text-muted-foreground">player</dt>
						<dd className="sm:col-span-2">
							{state.player_name ?? "—"}
							{state.observing || !state.is_local_player
								? " (spectating)"
								: " (you)"}
						</dd>
						<dt className="text-muted-foreground">activity</dt>
						<dd className="sm:col-span-2">{state.activity ?? "—"}</dd>
						<dt className="text-muted-foreground">map / phase</dt>
						<dd className="sm:col-span-2">
							{state.map_name ?? "—"} / {state.round_phase ?? "—"}
						</dd>
						<dt className="text-muted-foreground">health</dt>
						<dd className="sm:col-span-2">{state.health ?? "—"}</dd>
						<dt className="text-muted-foreground">flashed</dt>
						<dd className="sm:col-span-2">{state.flashed ?? "—"}</dd>
						<dt className="text-muted-foreground">bomb</dt>
						<dd className="sm:col-span-2">{state.bomb ?? "—"}</dd>
					</dl>
				)}
				{update != null && (
					<pre className="max-h-64 overflow-auto rounded-md bg-muted p-3 font-mono text-xs">
						{JSON.stringify(update.raw, null, 2)}
					</pre>
				)}
			</CardContent>
		</Card>
	);
}
