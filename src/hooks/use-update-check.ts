import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import i18n from "@/i18n";
import {
	DOWNLOAD_PAGE_URL,
	fetchLatestVersion,
	isNewer,
} from "@/lib/update-check";

const UPDATE_TOAST_ID = "update-available";

// Once per app run: the main webview lives until exit, so module state
// outlives Layout remounts.
let check: Promise<string | null> | null = null;
let toastShown = false;

/** The newer GitHub release version, or null while unknown, when offline or
 * when `current` is up to date. The first result also shows a toast. */
export function useUpdateCheck(current: string): string | null {
	const [latest, setLatest] = useState<string | null>(null);

	useEffect(() => {
		if (!current) return;
		let alive = true;
		check ??= fetchLatestVersion();
		check.then((tag) => {
			if (!alive || !tag || !isNewer(tag, current)) return;
			setLatest(tag);
			if (toastShown) return;
			toastShown = true;
			toast.warning(i18n.t("update.title", { version: tag }), {
				id: UPDATE_TOAST_ID,
				// The app often starts hidden in the tray; keep it until seen.
				duration: Number.POSITIVE_INFINITY,
				closeButton: true,
				description: i18n.t("update.description", { current }),
				action: {
					label: i18n.t("update.download"),
					onClick: () => openUrl(DOWNLOAD_PAGE_URL),
				},
			});
		});
		return () => {
			alive = false;
		};
	}, [current]);

	return latest;
}
