import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { EngineSnapshot } from "@/lib/types";

export function useEngine(): EngineSnapshot | null {
	const [snap, setSnap] = useState<EngineSnapshot | null>(null);
	useEffect(() => {
		const unlisten = listen<EngineSnapshot>("engine:update", (e) =>
			setSnap(e.payload),
		);
		invoke<EngineSnapshot>("engine_status").then(setSnap, () => {});
		return () => {
			unlisten.then((fn) => fn());
		};
	}, []);
	return snap;
}
