import { invoke } from "@tauri-apps/api/core";
import { Ellipsis, FolderOpen, RefreshCw, RotateCcw } from "lucide-react";
import { toast } from "sonner";
import { UndoRedo } from "@/components/UndoRedo";
import { Button } from "@/components/ui/button";
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

export function SettingsFooter({ cs2Found }: { cs2Found: boolean }) {
	const { resetDefaults, adoptExternal } = useLoadedSettings();

	// Picks up hand edits to config.toml. Like the tray's reload it replaces
	// the settings and clears undo history; warnings toast on their own.
	const reloadConfig = async () => {
		try {
			const { config, app } = await invoke<ConfigPayload>("reload_config");
			adoptExternal({ config, app });
			toast.success("Config reloaded");
		} catch (e) {
			toastError("Reload failed", e);
		}
	};

	return (
		<div className="flex items-center gap-2">
			<UndoRedo />
			<Separator orientation="vertical" className="mx-1 my-1" />
			<Button variant="outline" onClick={resetDefaults}>
				<RotateCcw />
				Reset to defaults
			</Button>
			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button
						variant="outline"
						size="icon"
						aria-label="More actions"
						className="ml-auto"
					>
						<Ellipsis />
					</Button>
				</DropdownMenuTrigger>
				{/* Sized to its items, not to the narrow trigger. */}
				<DropdownMenuContent align="end" className="w-max">
					<DropdownMenuItem onSelect={reloadConfig}>
						<RefreshCw />
						Reload config
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuItem
						disabled={!cs2Found}
						onSelect={() => openFolder("open_gsi_cfg_dir")}
					>
						<FolderOpen />
						Open CS2 config folder
					</DropdownMenuItem>
					<DropdownMenuItem onSelect={() => openFolder("open_config_dir")}>
						<FolderOpen />
						Open app config folder
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}
