import { useConfigWarningsToast } from "@/hooks/use-config-warnings-toast";
import { useEngine } from "@/hooks/use-engine";
import { useSettings } from "@/hooks/use-settings";

const NO_WARNINGS: string[] = [];

export function ConfigWarningsToast() {
	const snap = useEngine();
	const { resetDefaults } = useSettings();
	useConfigWarningsToast(snap?.config_warnings ?? NO_WARNINGS, resetDefaults);
	return null;
}
