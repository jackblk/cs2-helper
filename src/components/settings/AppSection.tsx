import { useTranslation } from "react-i18next";
import { SettingsSection } from "@/components/settings/section";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { useLoadedSettings } from "@/hooks/use-settings";
import { toggleLabel, withApp } from "@/lib/settings";

export function AppSection() {
	const { t } = useTranslation();
	const { settings, commit } = useLoadedSettings();
	const app = settings.app;
	return (
		<SettingsSection title={t("app.title")}>
			<div className="flex flex-wrap items-center gap-x-8 gap-y-2">
				<CheckboxField
					id="startup"
					label={t("app.runOnStartup")}
					checked={app.start_with_windows}
					onChange={(v) =>
						commit(
							toggleLabel(v, t("app.runOnStartup")),
							withApp((a) => ({ ...a, start_with_windows: v })),
						)
					}
				/>
				<CheckboxField
					id="start-minimized"
					label={t("app.startMinimized")}
					checked={app.start_minimized}
					onChange={(v) =>
						commit(
							toggleLabel(v, t("app.startMinimized")),
							withApp((a) => ({ ...a, start_minimized: v })),
						)
					}
				/>
			</div>
		</SettingsSection>
	);
}

function CheckboxField({
	id,
	label,
	checked,
	onChange,
}: {
	id: string;
	label: string;
	checked: boolean;
	onChange: (v: boolean) => void;
}) {
	return (
		<div className="flex items-center gap-2">
			<Checkbox
				id={id}
				checked={checked}
				onCheckedChange={(v) => onChange(v === true)}
			/>
			<Label htmlFor={id}>{label}</Label>
		</div>
	);
}
