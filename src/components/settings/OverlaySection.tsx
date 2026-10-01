import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { NumberField } from "@/components/settings/NumberField";
import {
	SettingRow,
	SettingsSection,
	SettingsSubheading,
} from "@/components/settings/section";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useLoadedSettings } from "@/hooks/use-settings";
import { DEFAULT_OVERLAY_SCALE } from "@/lib/overlay-edit";
import { withOverlay } from "@/lib/settings";
import { toastError } from "@/lib/toast";

// Same width in both states so Edit/Done never shifts the row.
const EDIT_BUTTON = "w-28";

export function OverlaySection() {
	const { settings, commit, overlayEditing } = useLoadedSettings();
	const o = settings.app.overlay;

	const startEdit = async () => {
		try {
			await invoke("overlay_edit_start");
		} catch (e) {
			toastError("Couldn't edit overlay", e);
		}
	};

	// The overlay window holds the unsaved geometry, so it ends a Done.
	const finishEdit = () => emit("overlay:edit-request", "ok");
	// Cancel needs no geometry. Ending it directly means it still works when
	// the overlay page never answers (`edit_end` is idempotent).
	const cancelEdit = () =>
		invoke("overlay_edit_end", { result: null }).catch(() => {});

	return (
		<SettingsSection
			title="Overlay"
			description="Draws over CS2. Needs borderless windowed, not fullscreen."
			action={
				<Switch
					checked={o.enabled}
					aria-label="Overlay enabled"
					onCheckedChange={(v) =>
						commit(
							v ? "Overlay on" : "Overlay off",
							withOverlay((x) => ({ ...x, enabled: v })),
						)
					}
				/>
			}
		>
			<SettingRow label="Position & size" wide>
				{overlayEditing ? (
					<>
						<Button className={EDIT_BUTTON} onClick={finishEdit}>
							Done
						</Button>
						<Button
							variant="outline"
							className={EDIT_BUTTON}
							onClick={cancelEdit}
						>
							Cancel
						</Button>
					</>
				) : (
					<>
						<Button
							variant="outline"
							className={EDIT_BUTTON}
							onClick={startEdit}
						>
							Edit overlay
						</Button>
						<Button
							variant="outline"
							className={EDIT_BUTTON}
							onClick={() =>
								commit(
									"Overlay reset",
									withOverlay((x) => ({
										...x,
										pos_x: null,
										pos_y: null,
										scale: DEFAULT_OVERLAY_SCALE,
									})),
								)
							}
						>
							Reset
						</Button>
					</>
				)}
			</SettingRow>
			<SettingsSubheading>Bomb timer</SettingsSubheading>
			<SettingRow label="Fuse length">
				<NumberField
					value={o.c4_timer_s}
					step={1}
					unit="sec"
					disabled={!o.enabled}
					ariaLabel="Bomb fuse seconds"
					onCommit={(n) =>
						commit(
							"Bomb fuse length",
							withOverlay((x) => ({ ...x, c4_timer_s: n })),
						)
					}
				/>
			</SettingRow>
		</SettingsSection>
	);
}
