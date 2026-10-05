import { invoke } from "@tauri-apps/api/core";
import { cn } from "cn";
import { Info, Play, Square } from "lucide-react";
import { useEffect, useState } from "react";
import { HotkeyRecorder } from "@/components/settings/HotkeyRecorder";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Slider } from "@/components/ui/slider";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { useLoadedSettings } from "@/hooks/use-settings";
import { withApp, withConfig } from "@/lib/settings";
import { toastError } from "@/lib/toast";
import { pct } from "@/lib/utils";

// Manual mode: on/off is the engine's runtime state (off at every launch),
// the volume and hotkey are saved settings.
export function DeathmatchBar({ on }: { on: boolean }) {
	const { settings, commit } = useLoadedSettings();
	const saved = settings.config.deathmatch.volume;
	const hotkey = settings.app.hotkeys.deathmatch;

	// Follows the drag locally; saved only when the pointer is released.
	const [volume, setVolume] = useState(saved);
	useEffect(() => setVolume(saved), [saved]);

	const toggle = async () => {
		try {
			await invoke("set_deathmatch", { on: !on });
		} catch (e) {
			toastError("Couldn't switch Deathmatch mode", e);
		}
	};

	return (
		<Card size="sm">
			<CardContent className="flex items-center gap-3">
				<div className="flex w-40 shrink-0 items-center gap-1.5">
					<span className="text-sm font-medium">Deathmatch mode</span>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="ghost"
								size="icon-xs"
								aria-label="Deathmatch mode info"
								className="text-muted-foreground"
							>
								<Info />
							</Button>
						</TooltipTrigger>
						<TooltipContent>
							While on, holds CS2 at this volume in any game mode. Switch it on
							when you join a deathmatch and off when you leave. It starts off
							every launch.
						</TooltipContent>
					</Tooltip>
				</div>
				<Slider
					value={[volume * 100]}
					min={0}
					max={100}
					step={5}
					aria-label="Deathmatch mode volume"
					onValueChange={([v]) => setVolume(v / 100)}
					onValueCommit={([v]) =>
						commit(
							"Deathmatch volume",
							withConfig((c) => ({ ...c, deathmatch: { volume: v / 100 } })),
						)
					}
				/>
				<span className="w-10 shrink-0 text-right font-mono text-sm tabular-nums text-muted-foreground">
					{pct(volume)}
				</span>
				<HotkeyRecorder
					value={hotkey.key}
					disabled={false}
					onChange={(key) =>
						commit(
							"Deathmatch hotkey",
							withApp((a) => ({
								...a,
								hotkeys: {
									...a.hotkeys,
									deathmatch: { ...a.hotkeys.deathmatch, key },
								},
							})),
						)
					}
				/>
				<Button
					variant={on ? "destructive" : "outline"}
					// Same width in both states so Start/Stop never shifts the bar.
					className={cn(
						"w-24",
						!on && "text-emerald-500 hover:text-emerald-500",
					)}
					onClick={toggle}
				>
					{on ? <Square /> : <Play />}
					{on ? "Stop" : "Start"}
				</Button>
			</CardContent>
		</Card>
	);
}
