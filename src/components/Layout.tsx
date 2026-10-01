import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useRef, useState } from "react";
import { Link, Outlet, useLocation, useNavigate } from "react-router-dom";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useFitWindowToContent } from "@/lib/fit-window";

// The debug page is intentionally hidden behind a triple-click on the logo.
const DEBUG_UNLOCK_CLICKS = 3;
const CLICK_RESET_MS = 600;

export function Layout() {
	const onDebug = useLocation().pathname === "/debug";
	const navigate = useNavigate();
	const mainRef = useRef<HTMLElement>(null);
	const contentRef = useRef<HTMLDivElement>(null);
	useFitWindowToContent(mainRef, contentRef);
	const [version, setVersion] = useState("");
	useEffect(() => {
		getVersion().then(setVersion);
	}, []);
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
									by jackblk
								</a>
							</div>
						</div>
						<div className="flex items-center gap-2">
							{onDebug && (
								<Button asChild variant="ghost" size="sm">
									<Link to="/">← Back</Link>
								</Button>
							)}
							{version && (
								<Badge variant="secondary" className="font-mono">
									{version}
								</Badge>
							)}
						</div>
					</header>
					<Outlet />
				</div>
				<Toaster richColors position="top-right" />
			</main>
		</TooltipProvider>
	);
}
