import { useTranslation } from "react-i18next";
import { HotkeyRecorder } from "@/components/settings/HotkeyRecorder";
import { SettingRow, SettingsSection } from "@/components/settings/section";
import { Switch } from "@/components/ui/switch";
import { useLoadedSettings } from "@/hooks/use-settings";
import { withApp } from "@/lib/settings";
import type { HotkeyCfg } from "@/lib/types";

export function HotkeysSection() {
	const { t } = useTranslation();
	const { settings, commit } = useLoadedSettings();
	const pause = settings.app.hotkeys.pause;
	const patchPause = (f: (h: HotkeyCfg) => HotkeyCfg) =>
		withApp((a) => ({
			...a,
			hotkeys: { ...a.hotkeys, pause: f(a.hotkeys.pause) },
		}));

	return (
		<SettingsSection
			title={t("hotkeys.title")}
			description={t("hotkeys.description")}
		>
			<SettingRow
				wide
				toggle={
					<Switch
						checked={pause.enabled}
						aria-label={t("hotkeys.pauseAria")}
						onCheckedChange={(v) =>
							commit(
								t(v ? "common.on" : "common.off", {
									name: t("hotkeys.pauseLabel"),
								}),
								patchPause((h) => ({ ...h, enabled: v })),
							)
						}
					/>
				}
				label={t("hotkeys.pause")}
			>
				<HotkeyRecorder
					value={pause.key}
					disabled={!pause.enabled}
					onChange={(key) =>
						commit(
							t("hotkeys.pauseLabel"),
							patchPause((h) => ({ ...h, key })),
						)
					}
				/>
			</SettingRow>
		</SettingsSection>
	);
}
