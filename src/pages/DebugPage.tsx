import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { EngineDebug } from "@/components/EngineDebug";
import { GsiDebug } from "@/components/GsiDebug";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/components/ui/table";

type AudioSession = {
	pid: number;
	process_name: string;
	volume: number;
	muted: boolean;
};

// Module-level so the preview choice survives navigating away from the Debug
// page (the component unmounts, but the toggle should stay as the user left it).
let previewEnabled = false;

export function DebugPage() {
	const [sessions, setSessions] = useState<AudioSession[]>([]);
	const [target, setTarget] = useState("cs2.exe");
	const [volume, setVolume] = useState(0.5);
	const [status, setStatus] = useState("");
	const [busy, setBusy] = useState(false);
	const [preview, setPreview] = useState(previewEnabled);

	// Mirror the preview tickbox to the overlay so placement/size can be checked
	// without a live plant. Persists across navigation; not cleared on unmount.
	useEffect(() => {
		previewEnabled = preview;
		emit("overlay:preview", preview);
	}, [preview]);

	const run = useCallback(async (fn: () => Promise<void>) => {
		setBusy(true);
		setStatus("");
		try {
			await fn();
		} catch (e) {
			setStatus(`error: ${e}`);
		} finally {
			setBusy(false);
		}
	}, []);

	const refresh = () =>
		run(async () => {
			const list = await invoke<AudioSession[]>("list_audio_sessions");
			list.sort((a, b) => a.process_name.localeCompare(b.process_name));
			setSessions(list);
			setStatus(`${list.length} session(s)`);
		});

	const setProcessVolume = () =>
		run(async () => {
			const changed = await invoke<number>("set_process_volume", {
				process: target,
				volume,
			});
			setStatus(
				`set ${target} -> ${Math.round(volume * 100)}% (${changed} session(s))`,
			);
		});

	const getProcessVolume = () =>
		run(async () => {
			const v = await invoke<number | null>("get_process_volume", {
				process: target,
			});
			setStatus(
				v == null
					? `${target}: no session`
					: `${target}: ${Math.round(v * 100)}%`,
			);
		});

	return (
		<div className="space-y-6">
			<header className="space-y-1">
				<h2 className="font-heading text-lg font-semibold">Diagnostics</h2>
				<p className="text-sm text-muted-foreground">
					Verify per-process volume control in isolation. Play audio in any app,
					list sessions, then set its volume by process name.
				</p>
			</header>

			<Card>
				<CardHeader>
					<CardTitle>Overlay preview</CardTitle>
					<CardDescription>
						Force the bomb-timer overlay to show its idle placeholder so you can
						check placement and size without a live plant.
					</CardDescription>
				</CardHeader>
				<CardContent>
					<div className="flex items-center gap-2">
						<Checkbox
							id="overlay-preview"
							checked={preview}
							onCheckedChange={(v) => setPreview(v === true)}
						/>
						<Label htmlFor="overlay-preview">Show overlay preview</Label>
					</div>
				</CardContent>
			</Card>

			<GsiDebug />

			<EngineDebug />

			<Card>
				<CardHeader>
					<CardTitle>Control</CardTitle>
					<CardDescription>
						Target a process by executable name and set its output volume.
					</CardDescription>
				</CardHeader>
				<CardContent className="space-y-4">
					<div className="flex flex-wrap items-end gap-4">
						<div className="flex flex-col gap-1.5">
							<Label htmlFor="process">Process</Label>
							<Input
								id="process"
								value={target}
								onChange={(e) => setTarget(e.currentTarget.value)}
								placeholder="cs2.exe"
								className="w-48"
							/>
						</div>
						<div className="flex min-w-56 flex-1 flex-col gap-1.5">
							<Label>Volume: {Math.round(volume * 100)}%</Label>
							<Slider
								value={[volume * 100]}
								min={0}
								max={100}
								step={1}
								onValueChange={([v]) => setVolume(v / 100)}
								className="mt-2"
							/>
						</div>
					</div>
					<div className="flex flex-wrap gap-2">
						<Button disabled={busy} onClick={setProcessVolume}>
							Set volume
						</Button>
						<Button
							variant="secondary"
							disabled={busy}
							onClick={getProcessVolume}
						>
							Get volume
						</Button>
						<Button variant="secondary" disabled={busy} onClick={refresh}>
							Refresh sessions
						</Button>
					</div>
					{status && (
						<p className="font-mono text-sm text-muted-foreground">{status}</p>
					)}
				</CardContent>
			</Card>

			<Card className="p-0">
				<Table>
					<TableHeader>
						<TableRow>
							<TableHead>Process</TableHead>
							<TableHead className="text-right">PID</TableHead>
							<TableHead className="text-right">Volume</TableHead>
							<TableHead className="text-right">Muted</TableHead>
							<TableHead className="w-0" />
						</TableRow>
					</TableHeader>
					<TableBody>
						{sessions.length === 0 ? (
							<TableRow>
								<TableCell
									colSpan={5}
									className="text-center text-muted-foreground"
								>
									No sessions — click "Refresh sessions".
								</TableCell>
							</TableRow>
						) : (
							sessions.map((s) => (
								<TableRow key={`${s.pid}-${s.process_name}`}>
									<TableCell className="font-mono">{s.process_name}</TableCell>
									<TableCell className="text-right text-muted-foreground">
										{s.pid}
									</TableCell>
									<TableCell className="text-right">
										{Math.round(s.volume * 100)}%
									</TableCell>
									<TableCell className="text-right">
										{s.muted ? (
											<Badge variant="destructive">muted</Badge>
										) : (
											<Badge variant="secondary">no</Badge>
										)}
									</TableCell>
									<TableCell className="text-right">
										<Button
											variant="ghost"
											size="sm"
											onClick={() => setTarget(s.process_name)}
										>
											select
										</Button>
									</TableCell>
								</TableRow>
							))
						)}
					</TableBody>
				</Table>
			</Card>
		</div>
	);
}
