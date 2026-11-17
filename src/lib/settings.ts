import i18n from "@/i18n";
import type { AppConfig, Config, OverlayConfig, Settings } from "@/lib/types";

// Undo/redo label of an on/off change.
export const toggleLabel = (on: boolean, name: string) =>
	i18n.t(on ? "common.on" : "common.off", { name });

export const withConfig =
	(f: (c: Config) => Config) =>
	(s: Settings): Settings => ({ ...s, config: f(s.config) });

export const withApp =
	(f: (a: AppConfig) => AppConfig) =>
	(s: Settings): Settings => ({ ...s, app: f(s.app) });

export const withOverlay = (f: (o: OverlayConfig) => OverlayConfig) =>
	withApp((a) => ({ ...a, overlay: f(a.overlay) }));
