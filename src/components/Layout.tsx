import { getVersion } from "@tauri-apps/api/app";
import { ArrowLeft, Heart, TriangleAlert } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, Outlet, useLocation, useNavigate } from "react-router-dom";
import { ConfigWarningsToast } from "@/components/ConfigWarningsToast";
import { LanguageMenu } from "@/components/LanguageMenu";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Toaster } from "@/components/ui/sonner";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { SettingsProvider } from "@/hooks/use-settings";
import { useUpdateCheck } from "@/hooks/use-update-check";
import { useFitWindowToContent } from "@/lib/fit-window";
import { DOWNLOAD_PAGE_URL } from "@/lib/update-check";

const DONATE_URL = "https://ko-fi.com/jackblk";

// The debug page is intentionally hidden behind a triple-click on the logo.
const DEBUG_UNLOCK_CLICKS = 3;
const CLICK_RESET_MS = 600;

export function Layout() {
	const { t } = useTranslation();
	const onDebug = useLocation().pathname === "/debug";
	const navigate = useNavigate();
	const mainRef = useRef<HTMLElement>(null);
	const contentRef = useRef<HTMLDivElement>(null);
	useFitWindowToContent(mainRef, contentRef);
	const [version, setVersion] = useState("");
	useEffect(() => {
		getVersion().then(setVersion);
	}, []);
	const latest = useUpdateCheck(version);
	const clicks = useRef(0);
	const resetTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

	const onLogoClick = () => {
		if (resetTimer.current) clearTimeout(resetTimer.current);
		clicks.current += 1;
		if (clicks.current >= DEBUG_UNLOCK_CLICKS) {
			clicks.current = 0;
			navigate("/debug");
			return;
		}
		resetTimer.current = setTimeout(() => {
			clicks.current = 0;
		}, CLICK_RESET_MS);
	};

	return (
		<TooltipProvider>
			<SettingsProvider>
				<main
					ref={mainRef}
					className="dark min-h-screen bg-background p-6 text-foreground"
				>
					<div ref={contentRef} className="mx-auto max-w-3xl space-y-6">
						<header className="flex items-center justify-between">
							<div className="flex items-center gap-2">
								<Button
									variant="ghost"
									size="icon"
									onClick={onLogoClick}
									aria-label="CS2 Helper"
									className="size-9"
								>
									<img src="/flashbang.svg" alt="" className="size-6" />
								</Button>
								<div className="flex items-baseline gap-2">
									<h1 className="font-heading text-xl font-semibold">
										CS2 Helper
									</h1>
									<a
										href="https://github.com/jackblk/cs2-helper"
										target="_blank"
										rel="noopener noreferrer"
										className="text-xs text-muted-foreground hover:text-foreground hover:underline"
									>
										{t("header.byline", { author: "jackblk" })}
									</a>
								</div>
							</div>
							<div className="flex items-center gap-2">
								{onDebug && (
									<Button asChild variant="ghost" size="sm">
										<Link to="/">
											<ArrowLeft />
											Back
										</Link>
									</Button>
								)}
								<Button
									asChild
									variant="ghost"
									size="sm"
									className="text-rose-400 hover:bg-rose-400/10 hover:text-rose-400"
								>
									<a
										href={DONATE_URL}
										target="_blank"
										rel="noopener noreferrer"
									>
										<Heart />
										{t("header.donate")}
									</a>
								</Button>
								{version &&
									(latest ? (
										<Tooltip>
											<TooltipTrigger asChild>
												<Badge
													asChild
													className="bg-amber-500/15 font-mono text-amber-400 [a]:hover:bg-amber-500/25"
												>
													<a
														href={DOWNLOAD_PAGE_URL}
														target="_blank"
														rel="noopener noreferrer"
													>
														<TriangleAlert data-icon="inline-start" />
														{version}
													</a>
												</Badge>
											</TooltipTrigger>
											<TooltipContent>
												{t("update.badgeTooltip", { version: latest })}
											</TooltipContent>
										</Tooltip>
									) : (
										<Badge variant="secondary" className="font-mono">
											{version}
										</Badge>
									))}
								<LanguageMenu />
							</div>
						</header>
						<Outlet />
					</div>
					<ConfigWarningsToast />
					<Toaster richColors position="top-center" />
				</main>
			</SettingsProvider>
		</TooltipProvider>
	);
}
