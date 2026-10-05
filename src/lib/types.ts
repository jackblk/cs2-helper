// Mirrors of the Rust structs serialized across the Tauri boundary.

export type EventCfg = { enabled: boolean; volume: number };
export type TimedEventCfg = EventCfg & { duration_ms: number };

export type Config = {
	death: TimedEventCfg;
	flash: EventCfg;
	bomb: TimedEventCfg;
	spectator: EventCfg;
	// Volume only: Deathmatch mode's on/off is the engine's runtime state.
	deathmatch: { volume: number };
};

export type OverlayConfig = {
	enabled: boolean;
	bomb_timer: boolean;
	volume_status: boolean;
	c4_timer_s: number;
	safety_margin_s: number;
	pos_x: number | null;
	pos_y: number | null;
	scale: number;
};

export type HotkeyCfg = { enabled: boolean; key: string };

export type HotkeysConfig = { pause: HotkeyCfg; deathmatch: HotkeyCfg };

export type AppConfig = {
	start_with_windows: boolean;
	start_minimized: boolean;
	// A code from LANGUAGES in src/i18n.
	language: string;
	overlay: OverlayConfig;
	hotkeys: HotkeysConfig;
};

export type ActiveReduction = { kind: string; volume: number };
export type LogEntry = { at_ms: number; trigger: string; decision: string };

export type EngineSnapshot = {
	target: number | null;
	active: ActiveReduction[];
	log: LogEntry[];
	config: Config | null;
	config_warnings: string[];
	paused: boolean;
	deathmatch: boolean;
};

export type GsiStatus = {
	running: boolean;
	port: number;
	last_payload_age_ms: number | null;
	cfg_path: string | null;
	cfg_installed: boolean;
};

export type GameState = {
	provider_steamid: string | null;
	player_steamid: string | null;
	is_local_player: boolean;
	observing: boolean;
	player_name: string | null;
	activity: string | null;
	map_name: string | null;
	map_phase: string | null;
	round_phase: string | null;
	bomb: string | null;
	win_team: string | null;
	health: number | null;
	armor: number | null;
	helmet: boolean | null;
	flashed: number | null;
	team: string | null;
	defusekit: boolean | null;
};

export type GsiUpdate = { raw: unknown; state: GameState };

export type Settings = { config: Config; app: AppConfig };

export type ConfigPayload = Settings & { warnings: string[] };

export type Rect = { x: number; y: number; width: number; height: number };

// Logical px. `x`/`y`/`scale` are the overlay's geometry when editing began.
export type OverlayEditSession = {
	monitor: Rect;
	x: number;
	y: number;
	scale: number;
};

export type OverlayEditResult = { x: number; y: number; scale: number };
