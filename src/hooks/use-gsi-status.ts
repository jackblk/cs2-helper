import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import type { GsiStatus } from "@/lib/types";

const POLL_MS = 1000;

export function useGsiStatus() {
	const [gsi, setGsi] = useState<GsiStatus | null>(null);
	const refresh = useCallback(() => {
		invoke<GsiStatus>("gsi_status").then(setGsi, () => {});
	}, []);
	useEffect(() => {
		refresh();
		const timer = setInterval(refresh, POLL_MS);
		return () => clearInterval(timer);
	}, [refresh]);
	return { gsi, refresh };
}
