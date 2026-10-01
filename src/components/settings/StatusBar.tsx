import { invoke } from "@tauri-apps/api/core";
import { Check, TriangleAlert } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { isFresh } from "@/hooks/use-gsi-status";
import type { EngineSnapshot, GsiStatus } from "@/lib/types";
import { cn, pct } from "@/lib/utils";

const EVENT_CHIPS = [
	{ kind: "flash", label: "Flash" },
	{ kind: "death", label: "Death" },
	{ kind: "bomb", label: "Bomb" },
	{ kind: "spectator", label: "Spec" },
] as const;

// Every slot has a fixed width so state changes never move or wrap anything.
export function StatusBar({
	snap,
	gsi,
	onOpenSetup,
}: {
	snap: EngineSnapshot;
	gsi: GsiStatus;
	onOpenSetup: () => void;
}) {
	const paused = snap.paused;
	const fresh = isFresh(gsi);
	const audio = paused || snap.target == null ? 1 : snap.target;

	return (
		<Card size="sm">
			<CardContent className="flex items-center gap-3">
				<Badge
					variant={!paused && fresh ? "secondary" : "outline"}
					className="w-28 justify-center"
				>
					{paused ? "Paused" : fresh ? "Running" : "Waiting for CS2"}
				</Badge>
				<span
					className={cn(
						"w-20 shrink-0 text-sm tabular-nums",
						audio < 1 ? "text-destructive" : "text-muted-foreground",
					)}
				>
					Audio {pct(audio)}
				</span>
				<div className="flex items-center gap-1">
					{EVENT_CHIPS.map((chip) => {
						const active = paused
							? undefined
							: snap.active.find((a) => a.kind === chip.kind);
						return (
							<Tooltip key={chip.kind}>
								<TooltipTrigger asChild>
									<Badge
										variant={active ? "destructive" : "outline"}
										className={cn(
											"w-12 justify-center",
											!active && "text-muted-foreground",
										)}
									>
										{chip.label}
									</Badge>
								</TooltipTrigger>
								<TooltipContent>
									{active
										? `${chip.label}: ${pct(active.volume)}`
										: `${chip.label}: inactive`}
								</TooltipContent>
							</Tooltip>
						);
					})}
				</div>
				<div className="ml-auto flex items-center gap-2">
					<Button
						variant={gsi.cfg_installed ? "ghost" : "outline"}
						className={cn(
							"w-20",
							!gsi.cfg_installed && "border-amber-500/60 text-amber-500",
						)}
						onClick={onOpenSetup}
					>
						{gsi.cfg_installed ? (
							<>
								<Check className="text-emerald-500" />
								GSI
							</>
						) : (
							<>
								<TriangleAlert />
								Set up
							</>
						)}
					</Button>
					<Button
						variant={paused ? "default" : "destructive"}
						className="w-20"
						onClick={() => invoke("set_paused", { paused: !paused })}
					>
						{paused ? "Resume" : "Pause"}
					</Button>
				</div>
			</CardContent>
		</Card>
	);
}
