import US from "country-flag-icons/react/3x2/US";
import VN from "country-flag-icons/react/3x2/VN";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { en } from "@/i18n/en";
import { vi } from "@/i18n/vi";

// Mirrors `LANGUAGES` in src-tauri/src/i18n.rs. `name` is the language's own
// name, so it reads correctly whatever the current language. `Flag` is an
// SVG because Windows does not draw flag emoji. English is US spelling.
export const LANGUAGES = [
	{ code: "en", name: "English", Flag: US },
	{ code: "vi", name: "Tiếng Việt", Flag: VN },
] as const;

export const DEFAULT_LANGUAGE = "en";

void i18n.use(initReactI18next).init({
	resources: { en: { translation: en }, vi: { translation: vi } },
	lng: DEFAULT_LANGUAGE,
	fallbackLng: DEFAULT_LANGUAGE,
	// React already escapes rendered strings.
	interpolation: { escapeValue: false },
});

export default i18n;
