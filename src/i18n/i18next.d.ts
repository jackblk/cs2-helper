import "i18next";
import type { en } from "@/i18n/en";

// Typed keys: `t("...")` must name a key of the English strings.
declare module "i18next" {
	interface CustomTypeOptions {
		defaultNS: "translation";
		resources: { translation: typeof en };
	}
}
