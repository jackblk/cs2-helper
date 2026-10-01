import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useLoadedSettings } from "@/hooks/use-settings";
import { openFolder } from "@/lib/folders";

export function SettingsFooter({ cs2Found }: { cs2Found: boolean }) {
	const { resetDefaults } = useLoadedSettings();
	return (
		<div className="flex items-center gap-2">
			<Button variant="outline" onClick={resetDefaults}>
				Reset to defaults
			</Button>
			<div className="ml-auto flex items-center gap-2">
				<Button
					variant="outline"
					disabled={!cs2Found}
					onClick={() => openFolder("open_gsi_cfg_dir")}
				>
					<FolderOpen />
					Open CS2 config
				</Button>
				<Button variant="outline" onClick={() => openFolder("open_config_dir")}>
					<FolderOpen />
					Open app config
				</Button>
			</div>
		</div>
	);
}
