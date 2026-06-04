// Shared types mirroring the Rust structs serialized across the Tauri boundary.

export type EventCfg = { enabled: boolean; volume: number };
export type TimedEventCfg = EventCfg & { duration_ms: number };

export type Config = {
	death: TimedEventCfg;
	flash: EventCfg;
	bomb: TimedEventCfg;
	spectator: EventCfg;
};

export type AppConfig = {
	start_with_windows: boolean;
	start_minimized: boolean;
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
};

export type GsiUpdate = { raw: unknown; state: GameState };
