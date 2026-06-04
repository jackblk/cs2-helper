// @ts-check
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

// https://astro.build/config
export default defineConfig({
	// Deployed to GitHub Pages project site: https://jackblk.github.io/cs2-helper/
	site: "https://jackblk.github.io",
	base: "/cs2-helper",
	vite: {
		plugins: [tailwindcss()],
	},
});
