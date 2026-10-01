import { Redo2, Undo2 } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { useSettings } from "@/hooks/use-settings";
import { useUndoShortcuts } from "@/hooks/use-undo-shortcuts";

export function UndoRedo() {
	const { undo, redo, undoLabel, redoLabel, overlayEditing } = useSettings();
	useUndoShortcuts(undo, redo, !overlayEditing);
	return (
		<div className="flex items-center gap-1">
			<HistoryButton
				label={undoLabel ? `Undo ${undoLabel}` : "Undo"}
				disabled={!undoLabel || overlayEditing}
				onClick={undo}
			>
				<Undo2 />
			</HistoryButton>
			<HistoryButton
				label={redoLabel ? `Redo ${redoLabel}` : "Redo"}
				disabled={!redoLabel || overlayEditing}
				onClick={redo}
			>
				<Redo2 />
			</HistoryButton>
		</div>
	);
}

function HistoryButton({
	label,
	disabled,
	onClick,
	children,
}: {
	label: string;
	disabled: boolean;
	onClick: () => void;
	children: ReactNode;
}) {
	return (
		<Tooltip>
			<TooltipTrigger asChild>
				<Button
					variant="ghost"
					size="icon"
					aria-label={label}
					disabled={disabled}
					onClick={onClick}
				>
					{children}
				</Button>
			</TooltipTrigger>
			<TooltipContent>{label}</TooltipContent>
		</Tooltip>
	);
}
