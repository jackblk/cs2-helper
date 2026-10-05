import { invoke } from "@tauri-apps/api/core";
import {
	Ellipsis,
	FolderOpen,
	Pause,
	Play,
	Power,
	RefreshCw,
	RotateCcw,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { UndoRedo } from "@/components/UndoRedo";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Separator } from "@/components/ui/separator";
import { useLoadedSettings } from "@/hooks/use-settings";
import { openFolder } from "@/lib/folders";
import { toastError } from "@/lib/toast";
import type { ConfigPayload } from "@/lib/types";

export function SettingsFooter({
	cs2Found,
	paused,
}: {
	cs2Found: boolean;
	paused: boolean;
}) {
	const { t } = useTranslation();
	const { resetDefaults, adoptExternal } = useLoadedSettings();

	// Picks up hand edits to config.toml. Like the tray's reload it replaces
	// the settings and clears undo history; warnings toast on their own.
	const reloadConfig = async () => {
		try {
			const { config, app } = await invoke<ConfigPayload>("reload_config");
			adoptExternal({ config, app });
			toast.success(t("footer.reloaded"));
		} catch (e) {
			toastError(t("footer.reloadFailed"), e);
		}
	};

	return (
		<div className="flex items-center gap-2">
			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button variant="outline" size="icon" aria-label={t("footer.more")}>
						<Ellipsis />
					</Button>
				</DropdownMenuTrigger>
				{/* Sized to its items, not to the narrow trigger. */}
				<DropdownMenuContent align="start" className="w-max">
					<DropdownMenuItem onSelect={reloadConfig}>
						<RefreshCw />
						{t("footer.reloadConfig")}
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuItem
						disabled={!cs2Found}
						onSelect={() => openFolder("open_gsi_cfg_dir")}
					>
						<FolderOpen />
						{t("footer.openCs2Folder")}
					</DropdownMenuItem>
					<DropdownMenuItem onSelect={() => openFolder("open_config_dir")}>
						<FolderOpen />
						{t("footer.openAppFolder")}
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>
			<Separator orientation="vertical" className="mx-1 my-1" />
			<UndoRedo />
			<Separator orientation="vertical" className="mx-1 my-1" />
			<Button variant="outline" onClick={resetDefaults}>
				<RotateCcw />
				{t("footer.resetDefaults")}
			</Button>
			<QuitButton />
			<Button
				// Same width in both states so Pause/Resume never shifts the bar.
				className="w-26"
				onClick={() => invoke("set_paused", { paused: !paused })}
			>
				{paused ? <Play /> : <Pause />}
				{paused ? t("status.resume") : t("status.pause")}
			</Button>
		</div>
	);
}

// Quitting restores CS2's volume and stops the overlay and hotkeys, unlike
// closing the window (which hides to the tray), so it asks first.
function QuitButton() {
	const { t } = useTranslation();
	const quit = () =>
		invoke("quit_app").catch((e) => toastError(t("footer.quitFailed"), e));

	return (
		<Dialog>
			<DialogTrigger asChild>
				<Button
					variant="ghost"
					className="ml-auto text-destructive hover:bg-destructive/10 hover:text-destructive"
				>
					<Power />
					{t("footer.quit")}
				</Button>
			</DialogTrigger>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>{t("footer.quitTitle")}</DialogTitle>
					<DialogDescription>{t("footer.quitDescription")}</DialogDescription>
				</DialogHeader>
				<DialogFooter>
					<DialogClose asChild>
						<Button variant="outline">{t("footer.quitCancel")}</Button>
					</DialogClose>
					<Button variant="destructive" onClick={quit}>
						<Power />
						{t("footer.quit")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
