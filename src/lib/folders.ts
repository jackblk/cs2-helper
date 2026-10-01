import { invoke } from "@tauri-apps/api/core";
import { toastError } from "@/lib/toast";

export async function openFolder(cmd: "open_gsi_cfg_dir" | "open_config_dir") {
	try {
		await invoke(cmd);
	} catch (e) {
		toastError("Couldn't open folder", e);
	}
}
