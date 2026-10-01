import { Info } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

// One grid for every setting row so switches, labels and controls line up
// across sections: switch | label | control | value | extra.
const ROW_GRID =
	"grid grid-cols-[2.25rem_10rem_minmax(0,1fr)_3.5rem_7rem] items-center gap-x-3";

export function SettingsSection({
	title,
	description,
	action,
	children,
}: {
	title: string;
	description?: string;
	action?: ReactNode;
	children: ReactNode;
}) {
	return (
		<section className="space-y-4">
			<div className="flex items-start justify-between gap-4">
				<div className="space-y-1">
					<h3 className="text-sm font-semibold uppercase tracking-wide">
						{title}
					</h3>
					{description && (
						<p className="text-sm text-muted-foreground">{description}</p>
					)}
				</div>
				{action}
			</div>
			<div className="space-y-3">{children}</div>
		</section>
	);
}

export function SettingsSubheading({ children }: { children: ReactNode }) {
	return (
		<div className={ROW_GRID}>
			<h4 className="col-span-4 col-start-2 pt-1 text-sm font-medium text-muted-foreground">
				{children}
			</h4>
		</div>
	);
}

export function SettingRow({
	toggle,
	label,
	info,
	value,
	extra,
	wide = false,
	children,
}: {
	toggle?: ReactNode;
	label: string;
	info?: string;
	value?: ReactNode;
	extra?: ReactNode;
	// Lets the control use the value and extra columns too (button rows).
	wide?: boolean;
	children?: ReactNode;
}) {
	return (
		<div className={ROW_GRID}>
			<div className="flex">{toggle}</div>
			<div className="flex min-w-0 items-center gap-1.5">
				<Label className="truncate font-medium">{label}</Label>
				{info && (
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								variant="ghost"
								size="icon-xs"
								aria-label={`${label} info`}
								className="text-muted-foreground"
							>
								<Info />
							</Button>
						</TooltipTrigger>
						<TooltipContent>{info}</TooltipContent>
					</Tooltip>
				)}
			</div>
			<div
				className={cn("flex min-w-0 items-center gap-2", wide && "col-span-3")}
			>
				{children}
			</div>
			{!wide && (
				<>
					<div className="text-right font-mono text-sm tabular-nums text-muted-foreground">
						{value}
					</div>
					<div className="flex items-center gap-1">{extra}</div>
				</>
			)}
		</div>
	);
}
