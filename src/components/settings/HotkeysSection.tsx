import { HotkeyRecorder } from "@/components/settings/HotkeyRecorder";
import { SettingRow, SettingsSection } from "@/components/settings/section";
import { Switch } from "@/components/ui/switch";
import { useLoadedSettings } from "@/hooks/use-settings";
import { withApp } from "@/lib/settings";
import type { HotkeyCfg } from "@/lib/types";

export function HotkeysSection() {
	const { settings, commit } = useLoadedSettings();
	const pause = settings.app.hotkeys.pause;
	const patchPause = (f: (h: HotkeyCfg) => HotkeyCfg) =>
		withApp((a) => ({
			...a,
			hotkeys: { ...a.hotkeys, pause: f(a.hotkeys.pause) },
		}));

	return (
		<SettingsSection
			title="Hotkeys"
			description="Work anywhere, even in game. Keys are captured system-wide, so CS2 won't receive them."
		>
			<SettingRow
				wide
				toggle={
					<Switch
						checked={pause.enabled}
						aria-label="Pause hotkey enabled"
						onCheckedChange={(v) =>
							commit(
								v ? "Pause hotkey on" : "Pause hotkey off",
								patchPause((h) => ({ ...h, enabled: v })),
							)
						}
					/>
				}
				label="Pause / Resume"
			>
				<HotkeyRecorder
					value={pause.key}
					disabled={!pause.enabled}
					onChange={(key) =>
						commit(
							"Pause hotkey",
							patchPause((h) => ({ ...h, key })),
						)
					}
				/>
			</SettingRow>
		</SettingsSection>
	);
}
