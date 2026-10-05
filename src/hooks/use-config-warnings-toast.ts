import { createElement, useEffect, useRef } from "react";
import { toast } from "sonner";
import i18n from "@/i18n";
import { warningsKey } from "@/lib/config-warnings";

const WARNINGS_TOAST_ID = "config-warnings";

export function useConfigWarningsToast(
	warnings: readonly string[],
	onReset: () => void,
) {
	const key = warningsKey(warnings);
	const shownKey = useRef("");
	// The effect must re-run only when the set changes, not on every snapshot.
	const latest = useRef({ warnings, onReset });
	latest.current = { warnings, onReset };

	useEffect(() => {
		if (key === shownKey.current) return;
		shownKey.current = key;
		if (!key) {
			toast.dismiss(WARNINGS_TOAST_ID);
			return;
		}
		toast.warning(i18n.t("warnings.title"), {
			id: WARNINGS_TOAST_ID,
			duration: Number.POSITIVE_INFINITY,
			closeButton: true,
			description: createElement(
				"ul",
				{ className: "mt-1 space-y-0.5 font-mono text-xs" },
				latest.current.warnings.map((w) => createElement("li", { key: w }, w)),
			),
			action: {
				label: i18n.t("warnings.reset"),
				onClick: () => latest.current.onReset(),
			},
		});
	}, [key]);
}
