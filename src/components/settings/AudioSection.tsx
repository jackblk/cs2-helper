import { useEffect, useState } from "react";
import { NumberField } from "@/components/settings/NumberField";
import { SettingRow, SettingsSection } from "@/components/settings/section";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { useLoadedSettings } from "@/hooks/use-settings";
import { withConfig } from "@/lib/settings";
import type { Config, EventCfg, TimedEventCfg } from "@/lib/types";
import { pct } from "@/lib/utils";

type EventKey = keyof Config;

const EVENTS: { key: EventKey; label: string; when: string }[] = [
	{ key: "flash", label: "Flash", when: "When you're flashed" },
	{ key: "death", label: "Death", when: "At death" },
	{ key: "bomb", label: "Bomb", when: "When the bomb explodes" },
	{ key: "spectator", label: "Spectator", when: "When you're spectating" },
];

export function AudioSection() {
	const { settings, commit } = useLoadedSettings();

	const patch = <K extends EventKey>(key: K, f: (e: Config[K]) => Config[K]) =>
		withConfig((c) => ({ ...c, [key]: f(c[key]) }) as Config);

	return (
		<SettingsSection
			title="Audio events"
			description="Volumes are absolute targets. When several events overlap, the lowest wins. Volume always restores to 100%."
		>
			{EVENTS.map(({ key, label, when }) => (
				<EventRow
					key={key}
					label={label}
					when={when}
					cfg={settings.config[key]}
					onEnabled={(v) =>
						commit(
							`${label} ${v ? "on" : "off"}`,
							patch(key, (e) => ({ ...e, enabled: v })),
						)
					}
					onVolume={(v) =>
						commit(
							`${label} volume`,
							patch(key, (e) => ({ ...e, volume: v })),
						)
					}
					onDuration={(ms) =>
						commit(
							`${label} duration`,
							patch(key, (e) => ({ ...e, duration_ms: ms })),
						)
					}
				/>
			))}
		</SettingsSection>
	);
}

function EventRow({
	label,
	when,
	cfg,
	onEnabled,
	onVolume,
	onDuration,
}: {
	label: string;
	when: string;
	cfg: EventCfg | TimedEventCfg;
	onEnabled: (v: boolean) => void;
	onVolume: (v: number) => void;
	onDuration: (ms: number) => void;
}) {
	// Follows the drag locally; saved only when the pointer is released.
	const [volume, setVolume] = useState(cfg.volume);
	useEffect(() => setVolume(cfg.volume), [cfg.volume]);
	const duration = "duration_ms" in cfg ? cfg.duration_ms : null;
	const hint =
		duration != null
			? `${when}, reduce audio to ${pct(cfg.volume)} for ${duration} ms.`
			: `${when}, reduce audio to ${pct(cfg.volume)}.`;

	return (
		<SettingRow
			toggle={
				<Switch
					checked={cfg.enabled}
					onCheckedChange={onEnabled}
					aria-label={`${label} enabled`}
				/>
			}
			label={label}
			info={hint}
			value={pct(volume)}
			extra={
				duration != null && (
					<NumberField
						value={duration}
						integer
						step={250}
						unit="ms"
						disabled={!cfg.enabled}
						onCommit={onDuration}
						ariaLabel={`${label} duration in milliseconds`}
					/>
				)
			}
		>
			<Slider
				value={[volume * 100]}
				min={0}
				max={100}
				step={5}
				disabled={!cfg.enabled}
				onValueChange={([v]) => setVolume(v / 100)}
				onValueCommit={([v]) => onVolume(v / 100)}
			/>
		</SettingRow>
	);
}
