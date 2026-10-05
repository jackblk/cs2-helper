import { RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AppSection } from "@/components/settings/AppSection";
import { AudioSection } from "@/components/settings/AudioSection";
import { DeathmatchBar } from "@/components/settings/DeathmatchBar";
import { HotkeysSection } from "@/components/settings/HotkeysSection";
import { OverlaySection } from "@/components/settings/OverlaySection";
import { SettingsFooter } from "@/components/settings/SettingsFooter";
import { SetupDialog } from "@/components/settings/SetupDialog";
import { StatusBar } from "@/components/settings/StatusBar";
import { Button } from "@/components/ui/button";
import { useEngine } from "@/hooks/use-engine";
import { useGsiStatus } from "@/hooks/use-gsi-status";
import { useSettings } from "@/hooks/use-settings";

// Module-level so the dialog auto-opens once per app run, not on every visit
// to the page.
let setupAutoOpened = false;

export function MainPage() {
	const { t } = useTranslation();
	const { settings, loadError, reload } = useSettings();
	const snap = useEngine();
	const { gsi, refresh } = useGsiStatus();
	const [setupOpen, setSetupOpen] = useState(false);

	const ready = settings != null && snap != null && gsi != null;
	const cfgInstalled = gsi?.cfg_installed;
	useEffect(() => {
		if (!ready || setupAutoOpened) return;
		setupAutoOpened = true;
		if (!cfgInstalled) setSetupOpen(true);
	}, [ready, cfgInstalled]);

	if (loadError) {
		return (
			<div className="flex items-center gap-3 text-sm text-destructive">
				<span>{t("main.loadFailed", { error: loadError })}</span>
				<Button variant="outline" size="sm" onClick={reload}>
					<RefreshCw />
					{t("main.retry")}
				</Button>
			</div>
		);
	}
	// Render nothing until everything is known, so the window fits once to
	// the real layout instead of to a placeholder.
	if (!settings || !snap || !gsi) return null;

	return (
		<div className="space-y-6">
			<div className="space-y-4">
				<StatusBar
					snap={snap}
					gsi={gsi}
					onOpenSetup={() => setSetupOpen(true)}
				/>
				<DeathmatchBar on={snap.deathmatch} />
			</div>
			<div className="space-y-4">
				<AudioSection />
				<OverlaySection />
				<HotkeysSection />
				<AppSection />
			</div>
			<SettingsFooter cs2Found={gsi.cfg_path != null} paused={snap.paused} />
			<SetupDialog
				open={setupOpen}
				onOpenChange={setSetupOpen}
				gsi={gsi}
				onChanged={refresh}
			/>
		</div>
	);
}
