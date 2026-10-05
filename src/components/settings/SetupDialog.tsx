import { invoke } from "@tauri-apps/api/core";
import { Circle, CircleCheck, Download, RefreshCw, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { isFresh } from "@/hooks/use-gsi-status";
import { toastError } from "@/lib/toast";
import type { GsiStatus } from "@/lib/types";

export function SetupDialog({
	open,
	onOpenChange,
	gsi,
	onChanged,
}: {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	gsi: GsiStatus;
	onChanged: () => void;
}) {
	const { t } = useTranslation();
	const installed = gsi.cfg_installed;
	const found = gsi.cfg_path != null;

	const install = async () => {
		try {
			await invoke("install_gsi_cfg");
			toast.success(t("setup.installed"));
		} catch (e) {
			toastError(t("setup.installFailed"), e);
		}
		onChanged();
	};

	const remove = async () => {
		try {
			await invoke("uninstall_gsi_cfg");
			toast.success(t("setup.removed"));
		} catch (e) {
			toastError(t("setup.removeFailed"), e);
		}
		onChanged();
	};

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>{t("setup.title")}</DialogTitle>
					<DialogDescription>{t("setup.description")}</DialogDescription>
				</DialogHeader>
				<div className="space-y-5">
					<SetupStep ok={installed} title={t("setup.step1")}>
						<div className="flex flex-wrap gap-2">
							<Button
								variant={installed ? "outline" : "default"}
								className="w-28"
								disabled={!found}
								onClick={install}
							>
								{installed ? <RefreshCw /> : <Download />}
								{installed ? t("setup.reinstall") : t("setup.install")}
							</Button>
							<Button
								variant="destructive"
								disabled={!installed}
								onClick={remove}
							>
								<Trash2 />
								{t("setup.remove")}
							</Button>
						</div>
						{!found && (
							<p className="text-sm text-muted-foreground">
								{t("setup.notFound")}
							</p>
						)}
					</SetupStep>
					<SetupStep ok={isFresh(gsi)} title={t("setup.step2")}>
						<p className="text-sm text-muted-foreground">
							{t("setup.restartHint")}
						</p>
					</SetupStep>
				</div>
			</DialogContent>
		</Dialog>
	);
}

function SetupStep({
	ok,
	title,
	children,
}: {
	ok: boolean;
	title: string;
	children: ReactNode;
}) {
	return (
		<div className="flex gap-3">
			{ok ? (
				<CircleCheck className="size-5 shrink-0 text-emerald-500" />
			) : (
				<Circle className="size-5 shrink-0 text-muted-foreground" />
			)}
			<div className="space-y-2">
				<p className="text-sm font-medium">{title}</p>
				{children}
			</div>
		</div>
	);
}
