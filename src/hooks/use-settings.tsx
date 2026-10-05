import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
	createContext,
	type ReactNode,
	useCallback,
	useContext,
	useEffect,
	useMemo,
	useState,
	useSyncExternalStore,
} from "react";
import { toast } from "sonner";
import i18n from "@/i18n";
import { withOverlay } from "@/lib/settings";
import { createSettingsStore, type StoreNotice } from "@/lib/settings-store";
import { toastError } from "@/lib/toast";
import type {
	AppConfig,
	ConfigPayload,
	EngineSnapshot,
	OverlayEditResult,
	OverlayEditSession,
	Settings,
} from "@/lib/types";

const SAVE_TOAST_ID = "settings-save";

type SettingsContextValue = {
	settings: Settings | null;
	loadError: string | null;
	reload: () => void;
	commit: (label: string, update: (s: Settings) => Settings) => void;
	resetDefaults: () => void;
	adoptExternal: (s: Settings) => void;
	undo: () => void;
	redo: () => void;
	undoLabel: string | null;
	redoLabel: string | null;
	overlayEditing: boolean;
};

const SettingsContext = createContext<SettingsContextValue | null>(null);

function showNotice(n: StoreNotice) {
	const { t } = i18n;
	switch (n.kind) {
		case "failed":
			return toastError(n.title, n.error);
		case "save-failed":
			return toastError(t("notice.saveFailed"), n.error);
		case "undo-failed":
			return toastError(t("notice.undoFailed"), n.error);
		case "redo-failed":
			return toastError(t("notice.redoFailed"), n.error);
	}
	const text =
		n.kind === "saved"
			? t("notice.saved", { label: n.label })
			: n.kind === "undone"
				? t("notice.undone", { label: n.label })
				: n.kind === "redone"
					? t("notice.redone", { label: n.label })
					: n.label;
	// One toast updated in place, so a burst of edits never stacks up.
	toast.success(text, { id: SAVE_TOAST_ID });
}

// Lives in Layout so history survives a visit to the Debug page.
export function SettingsProvider({ children }: { children: ReactNode }) {
	const [store] = useState(() =>
		createSettingsStore<Settings>({
			save: async (s) => {
				await invoke("save_config", { config: s.config, app: s.app });
			},
			notify: showNotice,
		}),
	);
	const { current, history } = useSyncExternalStore(
		store.subscribe,
		store.getSnapshot,
	);
	const [loadError, setLoadError] = useState<string | null>(null);
	const [overlayEditing, setOverlayEditing] = useState(false);

	const load = useCallback(async () => {
		setLoadError(null);
		try {
			const [snap, app] = await Promise.all([
				invoke<EngineSnapshot>("engine_status"),
				invoke<AppConfig>("get_app_settings"),
			]);
			if (!snap.config) throw new Error("the engine has no config yet");
			store.replace({ config: snap.config, app });
		} catch (e) {
			setLoadError(String(e));
		}
	}, [store]);

	useEffect(() => {
		void load();
		const unlisteners = [
			listen<Settings>("settings:changed", (e) => store.replace(e.payload)),
			listen<OverlayEditSession | null>("overlay:edit", (e) =>
				setOverlayEditing(e.payload != null),
			),
			listen<OverlayEditResult | null>("overlay:edit-result", (e) => {
				const r = e.payload;
				if (!r) return;
				void store.commit(
					i18n.t("overlay.positionLabel"),
					withOverlay((o) => ({
						...o,
						pos_x: r.x,
						pos_y: r.y,
						scale: r.scale,
					})),
				);
			}),
		];
		invoke<OverlayEditSession | null>("overlay_edit_session").then(
			(s) => setOverlayEditing(s != null),
			() => {},
		);
		return () => {
			for (const un of unlisteners) un.then((fn) => fn());
		};
	}, [load, store]);

	const value = useMemo<SettingsContextValue>(
		() => ({
			settings: current,
			loadError,
			reload: () => void load(),
			commit: (label, update) => void store.commit(label, update),
			resetDefaults: () =>
				void store.run(
					i18n.t("footer.resetDefaults"),
					i18n.t("notice.resetFailed"),
					async () => {
						const { config, app } = await invoke<ConfigPayload>("reset_config");
						return { config, app };
					},
				),
			adoptExternal: (s) => store.replace(s),
			undo: () => void store.undo(),
			redo: () => void store.redo(),
			undoLabel:
				history.past.length > 0
					? history.past[history.past.length - 1].label
					: null,
			redoLabel: history.future.length > 0 ? history.future[0].label : null,
			overlayEditing,
		}),
		[current, history, loadError, load, store, overlayEditing],
	);

	return (
		<SettingsContext.Provider value={value}>
			{children}
		</SettingsContext.Provider>
	);
}

export function useSettings(): SettingsContextValue {
	const ctx = useContext(SettingsContext);
	if (!ctx) throw new Error("useSettings must be used inside SettingsProvider");
	return ctx;
}

// For components that MainPage renders only after settings have loaded.
export function useLoadedSettings() {
	const ctx = useSettings();
	if (!ctx.settings) throw new Error("settings are not loaded yet");
	return { ...ctx, settings: ctx.settings };
}
