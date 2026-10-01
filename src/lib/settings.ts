import type { AppConfig, Config, OverlayConfig, Settings } from "@/lib/types";

export const withConfig =
	(f: (c: Config) => Config) =>
	(s: Settings): Settings => ({ ...s, config: f(s.config) });

export const withApp =
	(f: (a: AppConfig) => AppConfig) =>
	(s: Settings): Settings => ({ ...s, app: f(s.app) });

export const withOverlay = (f: (o: OverlayConfig) => OverlayConfig) =>
	withApp((a) => ({ ...a, overlay: f(a.overlay) }));
