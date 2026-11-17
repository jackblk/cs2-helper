import { invoke } from "@tauri-apps/api/core";
import { Keyboard } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { toastError } from "@/lib/toast";

const MODIFIER_CODE = /^(Control|Shift|Alt|Meta|OS)(Left|Right)?$/;

// Uses the physical `code` so the shortcut does not depend on keyboard layout.
// Returns null for a bare modifier press (keep waiting for the real key).
function shortcutFromEvent(e: KeyboardEvent): string | null {
	if (MODIFIER_CODE.test(e.code)) return null;
	const key = e.code.replace(/^Key/, "").replace(/^Digit/, "");
	const mods: string[] = [];
	if (e.ctrlKey) mods.push("Ctrl");
	if (e.altKey) mods.push("Alt");
	if (e.shiftKey) mods.push("Shift");
	if (e.metaKey) mods.push("Super");
	return [...mods, key].join("+");
}

// The live hotkeys are unregistered while recording (otherwise a bound key
// would run its action instead of reaching the webview) and re-registered
// from the saved config afterwards.
export function HotkeyRecorder({
	value,
	disabled = false,
	onChange,
}: {
	value: string;
	disabled?: boolean;
	onChange: (key: string) => void;
}) {
	const { t } = useTranslation();
	const [recording, setRecording] = useState(false);

	useEffect(() => {
		if (!recording) return;
		const stop = () => {
			setRecording(false);
			invoke("hotkey_capture_end").catch(() => {});
		};
		const onKeyDown = (e: KeyboardEvent) => {
			e.preventDefault();
			e.stopPropagation();
			if (e.code === "Escape") return stop();
			const shortcut = shortcutFromEvent(e);
			if (shortcut == null) return;
			onChange(shortcut);
			stop();
		};
		window.addEventListener("keydown", onKeyDown, true);
		window.addEventListener("blur", stop);
		return () => {
			window.removeEventListener("keydown", onKeyDown, true);
			window.removeEventListener("blur", stop);
		};
	}, [recording, onChange]);

	// Leaving the page mid-recording must not leave the hotkey unregistered.
	const recordingRef = useRef(false);
	recordingRef.current = recording;
	useEffect(
		() => () => {
			if (recordingRef.current) invoke("hotkey_capture_end").catch(() => {});
		},
		[],
	);

	const start = async () => {
		try {
			await invoke("hotkey_capture_start");
			setRecording(true);
		} catch (e) {
			toastError(t("hotkeys.recordFailed"), e);
		}
	};

	return (
		<Button
			variant={recording ? "secondary" : "outline"}
			disabled={disabled}
			onClick={recording ? undefined : start}
			// Fixed width: the label changes while recording.
			className="w-48 font-mono"
		>
			<Keyboard />
			{recording ? t("hotkeys.recording") : value}
		</Button>
	);
}
