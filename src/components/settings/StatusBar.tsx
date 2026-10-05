import { cn } from "cn";
import { Check, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
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
import { pct } from "@/lib/utils";

const EVENT_CHIPS = ["flash", "death", "bomb", "spectator"] as const;

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
	const { t } = useTranslation();
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
					{paused
						? t("status.paused")
						: fresh
							? t("status.running")
							: t("status.waiting")}
				</Badge>
				<span
					className={cn(
						"w-28 shrink-0 whitespace-nowrap text-sm tabular-nums",
						audio < 1 ? "text-destructive" : "text-muted-foreground",
					)}
				>
					{t("status.audio", { pct: pct(audio) })}
				</span>
				<div className="flex items-center gap-1">
					{EVENT_CHIPS.map((kind) => {
						const label = t(`status.chips.${kind}`);
						const active = paused
							? undefined
							: snap.active.find((a) => a.kind === kind);
						return (
							<Tooltip key={kind}>
								<TooltipTrigger asChild>
									<Badge
										variant={active ? "destructive" : "outline"}
										className={cn(
											"min-w-12 justify-center",
											!active && "text-muted-foreground",
										)}
									>
										{label}
									</Badge>
								</TooltipTrigger>
								<TooltipContent>
									{active
										? t("status.chipActive", {
												label,
												pct: pct(active.volume),
											})
										: t("status.chipInactive", { label })}
								</TooltipContent>
							</Tooltip>
						);
					})}
				</div>
				<div className="ml-auto flex items-center gap-2">
					<Button
						variant={gsi.cfg_installed ? "ghost" : "outline"}
						className={cn(
							"w-24",
							!gsi.cfg_installed && "border-amber-500/60 text-amber-500",
						)}
						onClick={onOpenSetup}
					>
						{gsi.cfg_installed ? (
							<>
								<Check className="text-emerald-500" />
								{t("status.gsi")}
							</>
						) : (
							<>
								<TriangleAlert />
								{t("status.setUp")}
							</>
						)}
					</Button>
				</div>
			</CardContent>
		</Card>
	);
}
