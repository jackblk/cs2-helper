import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter, Route, Routes } from "react-router-dom";
import { Layout } from "@/components/Layout";
import { DebugPage } from "@/pages/DebugPage";
import { MainPage } from "@/pages/MainPage";
import "./App.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<HashRouter>
			<Routes>
				<Route element={<Layout />}>
					<Route index element={<MainPage />} />
					<Route path="debug" element={<DebugPage />} />
				</Route>
			</Routes>
		</HashRouter>
	</React.StrictMode>,
);
