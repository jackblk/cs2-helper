import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { cn } from "cn";
import { Check, Eye, Move, Power, PowerOff, RotateCcw, X } from "lucide-react";
import { NumberField } from "@/components/settings/NumberField";
import { SettingRow, SettingsSection } from "@/components/settings/section";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useLoadedSettings } from "@/hooks/use-settings";
import { DEFAULT_OVERLAY_SCALE } from "@/lib/overlay-edit";
import { withOverlay } from "@/lib/settings";
import { toastError } from "@/lib/toast";

// Same width in both states so Edit/Done never shifts the row.
const EDIT_BUTTON = "w-32";
// Fits "Disable", so Enable/Disable never resizes the header.
const ENABLE_BUTTON = "w-22";

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
			off={!o.enabled}
			action={
				<Button
					variant="ghost"
					size="sm"
					className={cn(
						ENABLE_BUTTON,
						o.enabled
							? "text-destructive hover:bg-destructive/10 hover:text-destructive"
							: "text-emerald-500 hover:bg-emerald-500/10 hover:text-emerald-500",
					)}
					onClick={() =>
						commit(
							o.enabled ? "Overlay off" : "Overlay on",
							withOverlay((x) => ({ ...x, enabled: !x.enabled })),
						)
					}
				>
					{o.enabled ? <PowerOff /> : <Power />}
					{o.enabled ? "Disable" : "Enable"}
				</Button>
			}
		>
			<SettingRow
				label="Position & size"
				info="Drag and resize the overlay on your screen. Preview shows it for 5 seconds."
				wide
			>
				{overlayEditing ? (
					<>
						<Button className={EDIT_BUTTON} onClick={finishEdit}>
							<Check />
							Done
						</Button>
						<Button
							variant="outline"
							className={EDIT_BUTTON}
							onClick={cancelEdit}
						>
							<X />
							Cancel
						</Button>
					</>
				) : (
					<>
						<Button
							variant="outline"
							className={EDIT_BUTTON}
							disabled={!o.enabled}
							onClick={startEdit}
						>
							<Move />
							Edit overlay
						</Button>
						<Button
							variant="outline"
							className={EDIT_BUTTON}
							disabled={!o.enabled}
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
							<RotateCcw />
							Reset
						</Button>
						<Button
							variant="outline"
							className={EDIT_BUTTON}
							disabled={!o.enabled}
							// The overlay page runs the 5 s countdown itself.
							onClick={() => emit("overlay:preview-start")}
						>
							<Eye />
							Preview
						</Button>
					</>
				)}
			</SettingRow>
			<SettingRow
				toggle={
					<Switch
						checked={o.bomb_timer}
						disabled={!o.enabled}
						aria-label="Bomb timer enabled"
						onCheckedChange={(v) =>
							commit(
								v ? "Bomb timer on" : "Bomb timer off",
								withOverlay((x) => ({ ...x, bomb_timer: v })),
							)
						}
					/>
				}
				label="Bomb timer"
				info="Shows the time left after the bomb is planted. Set the fuse length to match the game mode."
			>
				<NumberField
					value={o.c4_timer_s}
					step={1}
					unit="sec"
					disabled={!o.enabled || !o.bomb_timer}
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
