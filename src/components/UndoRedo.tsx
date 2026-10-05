import { Redo2, Undo2 } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { useSettings } from "@/hooks/use-settings";
import { useUndoShortcuts } from "@/hooks/use-undo-shortcuts";

export function UndoRedo() {
	const { t } = useTranslation();
	const { undo, redo, undoLabel, redoLabel, overlayEditing } = useSettings();
	useUndoShortcuts(undo, redo, !overlayEditing);
	return (
		<>
			<HistoryButton
				label={t("footer.undo")}
				// The tooltip names the step; the label stays short and fixed.
				tooltip={
					undoLabel
						? t("footer.undoTip", { label: undoLabel })
						: t("footer.undoTipEmpty")
				}
				disabled={!undoLabel || overlayEditing}
				onClick={undo}
			>
				<Undo2 />
			</HistoryButton>
			<HistoryButton
				label={t("footer.redo")}
				tooltip={
					redoLabel
						? t("footer.redoTip", { label: redoLabel })
						: t("footer.redoTipEmpty")
				}
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
