import { invoke } from "@tauri-apps/api/core";
import { Circle, CircleCheck, Download, RefreshCw, Trash2 } from "lucide-react";
import type { ReactNode } from "react";
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
	const installed = gsi.cfg_installed;
	const found = gsi.cfg_path != null;

	const install = async () => {
		try {
			await invoke("install_gsi_cfg");
			toast.success("Installed GSI config. Restart CS2 to take effect");
		} catch (e) {
			toastError("Install failed", e);
		}
		onChanged();
	};

	const remove = async () => {
		try {
			await invoke("uninstall_gsi_cfg");
			toast.success("GSI config removed. Restart CS2 to stop it sending data");
		} catch (e) {
			toastError("Remove failed", e);
		}
		onChanged();
	};

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>Setup</DialogTitle>
					<DialogDescription>Connect CS2 Helper to CS2.</DialogDescription>
				</DialogHeader>
				<div className="space-y-5">
					<SetupStep ok={installed} title="1. GSI config installed">
						<div className="flex flex-wrap gap-2">
							<Button
								variant={installed ? "outline" : "default"}
								className="w-28"
								disabled={!found}
								onClick={install}
							>
								{installed ? <RefreshCw /> : <Download />}
								{installed ? "Reinstall" : "Install"}
							</Button>
							<Button
								variant="destructive"
								disabled={!installed}
								onClick={remove}
							>
								<Trash2 />
								Remove
							</Button>
						</div>
						{!found && (
							<p className="text-sm text-muted-foreground">
								CS2 installation not found. Install CS2, then try again.
							</p>
						)}
					</SetupStep>
					<SetupStep ok={isFresh(gsi)} title="2. Receiving data from CS2">
						<p className="text-sm text-muted-foreground">
							Restart CS2 if it is already running.
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
