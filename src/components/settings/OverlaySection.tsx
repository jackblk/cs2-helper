import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { cn } from "cn";
import { Check, Eye, Move, Power, PowerOff, RotateCcw, X } from "lucide-react";
import { useTranslation } from "react-i18next";
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
	const { t } = useTranslation();
	const { settings, commit, overlayEditing } = useLoadedSettings();
	const o = settings.app.overlay;

	const startEdit = async () => {
		try {
			await invoke("overlay_edit_start");
		} catch (e) {
			toastError(t("overlay.editFailed"), e);
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
			title={t("overlay.title")}
			description={t("overlay.description")}
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
							t(o.enabled ? "common.off" : "common.on", {
								name: t("overlay.title"),
							}),
							withOverlay((x) => ({ ...x, enabled: !x.enabled })),
						)
					}
				>
					{o.enabled ? <PowerOff /> : <Power />}
					{o.enabled ? t("overlay.disable") : t("overlay.enable")}
				</Button>
			}
		>
			<SettingRow
				label={t("overlay.positionSize")}
				info={t("overlay.positionInfo")}
				wide
			>
				{overlayEditing ? (
					<>
						<Button className={EDIT_BUTTON} onClick={finishEdit}>
							<Check />
							{t("overlay.done")}
						</Button>
						<Button
							variant="outline"
							className={EDIT_BUTTON}
							onClick={cancelEdit}
						>
							<X />
							{t("overlay.cancel")}
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
							{t("overlay.edit")}
						</Button>
						<Button
							variant="outline"
							className={EDIT_BUTTON}
							disabled={!o.enabled}
							onClick={() =>
								commit(
									t("overlay.resetLabel"),
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
							{t("overlay.reset")}
						</Button>
						<Button
							variant="outline"
							className={EDIT_BUTTON}
							disabled={!o.enabled}
							// The overlay page runs the 5 s countdown itself.
							onClick={() => emit("overlay:preview-start")}
						>
							<Eye />
							{t("overlay.preview")}
						</Button>
					</>
				)}
			</SettingRow>
			<SettingRow
				toggle={
					<Switch
						checked={o.bomb_timer}
						disabled={!o.enabled}
						aria-label={t("overlay.bombTimerAria")}
						onCheckedChange={(v) =>
							commit(
								t(v ? "common.on" : "common.off", {
									name: t("overlay.bombTimer"),
								}),
								withOverlay((x) => ({ ...x, bomb_timer: v })),
							)
						}
					/>
				}
				label={t("overlay.bombTimer")}
				info={t("overlay.bombTimerInfo")}
			>
				<NumberField
					value={o.c4_timer_s}
					step={1}
					unit={t("overlay.unitSec")}
					disabled={!o.enabled || !o.bomb_timer}
					ariaLabel={t("overlay.fuseAria")}
					onCommit={(n) =>
						commit(
							t("overlay.fuseLabel"),
							withOverlay((x) => ({ ...x, c4_timer_s: n })),
						)
					}
				/>
			</SettingRow>
			<SettingRow
				toggle={
					<Switch
						checked={o.volume_status}
						disabled={!o.enabled}
						aria-label={t("overlay.volumeStatusAria")}
						onCheckedChange={(v) =>
							commit(
								t(v ? "common.on" : "common.off", {
									name: t("overlay.volumeStatus"),
								}),
								withOverlay((x) => ({ ...x, volume_status: v })),
							)
						}
					/>
				}
				label={t("overlay.volumeStatus")}
				info={t("overlay.volumeStatusInfo")}
			/>
		</SettingsSection>
	);
}
