import { getCurrentWindow } from "@tauri-apps/api/window";
import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter, Route, Routes } from "react-router-dom";
import { Layout } from "@/components/Layout";
import { DebugPage } from "@/pages/DebugPage";
import { MainPage } from "@/pages/MainPage";
import { Overlay } from "@/pages/Overlay";
import "./App.css";

const isOverlay = getCurrentWindow().label === "overlay";
if (isOverlay) {
	document.documentElement.classList.add("overlay-mode");
}

const root = ReactDOM.createRoot(
	document.getElementById("root") as HTMLElement,
);
root.render(
	<React.StrictMode>
		{isOverlay ? (
			<Overlay />
		) : (
			<HashRouter>
				<Routes>
					<Route element={<Layout />}>
						<Route index element={<MainPage />} />
						<Route path="debug" element={<DebugPage />} />
					</Route>
				</Routes>
			</HashRouter>
		)}
	</React.StrictMode>,
);
