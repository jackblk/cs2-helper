import { emit } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuLabel,
	DropdownMenuRadioGroup,
	DropdownMenuRadioItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useSettings } from "@/hooks/use-settings";
import i18n, { LANGUAGES } from "@/i18n";
import { withApp } from "@/lib/settings";

// The saved `app.language` drives the UI language. Applying it here, in the
// header that every page renders, also covers reloads, resets and tray
// changes; the event tells the overlay window.
export function LanguageMenu() {
	const { t } = useTranslation();
	const { settings, commit } = useSettings();
	const language = settings?.app.language;
	const current = LANGUAGES.find((l) => l.code === language) ?? LANGUAGES[0];

	useEffect(() => {
		if (!language || language === i18n.language) return;
		void i18n.changeLanguage(language);
		void emit("app:language", language);
	}, [language]);

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button
					variant="ghost"
					size="icon"
					aria-label={t("header.language")}
					disabled={!settings}
				>
					<current.Flag className="h-3.5 w-auto rounded-xs" />
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end" className="w-max">
				<DropdownMenuLabel>{t("header.language")}</DropdownMenuLabel>
				<DropdownMenuSeparator />
				<DropdownMenuRadioGroup
					value={language}
					onValueChange={(code) =>
						commit(
							// In the new language: the "Saved" toast around it is
							// shown after the switch.
							i18n.getFixedT(code)("header.language"),
							withApp((a) => ({ ...a, language: code })),
						)
					}
				>
					{LANGUAGES.map((l) => (
						<DropdownMenuRadioItem key={l.code} value={l.code}>
							<l.Flag className="h-3 w-auto rounded-xs" />
							{l.name}
						</DropdownMenuRadioItem>
					))}
				</DropdownMenuRadioGroup>
			</DropdownMenuContent>
		</DropdownMenu>
	);
}
