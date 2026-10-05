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
		<>
			<HistoryButton
				label="Undo"
				// The tooltip names the step; the label stays short and fixed.
				tooltip={undoLabel ? `Undo ${undoLabel} (Ctrl+Z)` : "Undo (Ctrl+Z)"}
				disabled={!undoLabel || overlayEditing}
				onClick={undo}
			>
				<Undo2 />
			</HistoryButton>
			<HistoryButton
				label="Redo"
				tooltip={redoLabel ? `Redo ${redoLabel} (Ctrl+Y)` : "Redo (Ctrl+Y)"}
				disabled={!redoLabel || overlayEditing}
				onClick={redo}
			>
				<Redo2 />
			</HistoryButton>
		</>
	);
}

function HistoryButton({
	label,
	tooltip,
	disabled,
	onClick,
	children,
}: {
	label: string;
	tooltip: string;
	disabled: boolean;
	onClick: () => void;
	children: ReactNode;
}) {
	return (
		<Tooltip>
			<TooltipTrigger asChild>
				<Button variant="outline" disabled={disabled} onClick={onClick}>
					{children}
					{label}
				</Button>
			</TooltipTrigger>
			<TooltipContent>{tooltip}</TooltipContent>
		</Tooltip>
	);
}
