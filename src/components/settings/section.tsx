import { cn } from "cn";
import { Info } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardAction,
	CardContent,
	CardHeader,
} from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";

// One grid for every setting row so switches, labels and controls line up
// across sections: switch | label | control | value | extra.
const ROW_GRID =
	"grid grid-cols-[2.25rem_10rem_minmax(0,1fr)_3.5rem_7rem] items-center gap-x-3";

export function SettingsSection({
	title,
	description,
	action,
	off = false,
	children,
}: {
	title: string;
	description?: string;
	// Section-wide control (e.g. Enable/Disable), right of the title.
	action?: ReactNode;
	// Dims the row labels; the rows disable their own controls.
	off?: boolean;
	children: ReactNode;
}) {
	return (
		<Card className="gap-0 py-0">
			<CardHeader className="gap-x-3 border-b py-4">
				{action && <CardAction>{action}</CardAction>}
				<div className="space-y-1">
					<h3 className="text-sm font-semibold uppercase tracking-wide">
						{title}
					</h3>
					{description && (
						<p className="text-sm text-muted-foreground">{description}</p>
					)}
				</div>
			</CardHeader>
			{/* `group` + data-disabled is what dims shadcn's Label. */}
			<CardContent className="group space-y-3 py-4" data-disabled={off}>
				{children}
			</CardContent>
		</Card>
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
	const { t } = useTranslation();
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
								aria-label={t("common.infoAria", { label })}
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
