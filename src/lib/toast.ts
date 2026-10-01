import { toast } from "sonner";

export const ERROR_TOAST_MS = 8000;

export function toastError(title: string, error: unknown) {
	toast.error(title, { description: String(error), duration: ERROR_TOAST_MS });
}
