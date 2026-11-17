import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { NumberField } from "@/components/settings/NumberField";
import { SettingRow, SettingsSection } from "@/components/settings/section";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { useLoadedSettings } from "@/hooks/use-settings";
import { toggleLabel, withConfig } from "@/lib/settings";
import type { Config, EventCfg, TimedEventCfg } from "@/lib/types";
import { pct } from "@/lib/utils";

// Deathmatch mode has its own bar (DeathmatchBar).
type EventKey = Exclude<keyof Config, "deathmatch">;

const EVENTS: EventKey[] = ["flash", "death", "bomb", "spectator"];

export function AudioSection() {
	const { t } = useTranslation();
	const { settings, commit } = useLoadedSettings();

	const patch = <K extends EventKey>(key: K, f: (e: Config[K]) => Config[K]) =>
		withConfig((c) => ({ ...c, [key]: f(c[key]) }) as Config);

	return (
		<SettingsSection
			title={t("audio.title")}
			description={t("audio.description")}
		>
			{EVENTS.map((key) => {
				const label = t(`audio.events.${key}`);
				return (
					<EventRow
						key={key}
						label={label}
						when={t(`audio.when.${key}`)}
						cfg={settings.config[key]}
						onEnabled={(v) =>
							commit(
								toggleLabel(v, label),
								patch(key, (e) => ({ ...e, enabled: v })),
							)
						}
						onVolume={(v) =>
							commit(
								t("audio.volumeLabel", { label }),
								patch(key, (e) => ({ ...e, volume: v })),
							)
						}
						onDuration={(ms) =>
							commit(
								t("audio.durationLabel", { label }),
								patch(key, (e) => ({ ...e, duration_ms: ms })),
							)
						}
					/>
				);
			})}
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
	const { t } = useTranslation();
	// Follows the drag locally; saved only when the pointer is released.
	const [volume, setVolume] = useState(cfg.volume);
	useEffect(() => setVolume(cfg.volume), [cfg.volume]);
	const duration = "duration_ms" in cfg ? cfg.duration_ms : null;
	const hint =
		duration != null
			? t("audio.hintTimed", { when, pct: pct(cfg.volume), ms: duration })
			: t("audio.hint", { when, pct: pct(cfg.volume) });

	return (
		<SettingRow
			toggle={
				<Switch
					checked={cfg.enabled}
					onCheckedChange={onEnabled}
					aria-label={t("audio.enabledAria", { label })}
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
						unit={t("audio.unitMs")}
						disabled={!cfg.enabled}
						onCommit={onDuration}
						ariaLabel={t("audio.durationAria", { label })}
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
