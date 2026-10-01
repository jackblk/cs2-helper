import {
	currentMonitor,
	getCurrentWindow,
	LogicalPosition,
	LogicalSize,
} from "@tauri-apps/api/window";
import { type RefObject, useEffect } from "react";

// Keep the window's height equal to its content's height, so the page never
// needs a scrollbar. Capped to the monitor's work area (the page scrolls only
// beyond that), and the window is nudged up if it would run off the bottom.
// Width is left to the user; a maximized window is left alone.
//
// `container` is the padded full-height element, `content` its natural-height
// child: the window needs content height + container padding.
export function useFitWindowToContent(
	container: RefObject<HTMLElement | null>,
	content: RefObject<HTMLElement | null>,
) {
	useEffect(() => {
		const outer = container.current;
		const inner = content.current;
		if (!outer || !inner) return;
		const win = getCurrentWindow();

		let frame = 0;
		let running = false;
		let again = false;

		const fitNow = async () => {
			// Window calls are async; never overlap two fits.
			if (running) {
				again = true;
				return;
			}
			running = true;
			try {
				if (await win.isMaximized()) return;
				const style = getComputedStyle(outer);
				const want = Math.ceil(
					inner.getBoundingClientRect().height +
						Number.parseFloat(style.paddingTop) +
						Number.parseFloat(style.paddingBottom),
				);
				const scale = await win.scaleFactor();
				const size = (await win.innerSize()).toLogical(scale);
				// Title bar + borders: outer height minus inner height.
				const chrome =
					(await win.outerSize()).toLogical(scale).height - size.height;

				const mon = await currentMonitor();
				const area = mon
					? {
							top: mon.workArea.position.y / mon.scaleFactor,
							height: mon.workArea.size.height / mon.scaleFactor,
						}
					: null;
				const height = area ? Math.min(want, area.height - chrome) : want;

				if (Math.abs(height - size.height) >= 1) {
					await win.setSize(new LogicalSize(size.width, height));
				}
				if (area) {
					const pos = (await win.outerPosition()).toLogical(scale);
					const bottom = area.top + area.height;
					if (pos.y + chrome + height > bottom) {
						await win.setPosition(
							new LogicalPosition(
								pos.x,
								Math.max(area.top, bottom - chrome - height),
							),
						);
					}
				}
			} catch {
				// Best effort: a failed fit just leaves the window as it was.
			} finally {
				running = false;
				if (again) {
					again = false;
					schedule();
				}
			}
		};

		// Coalesce bursts of size changes into one fit per frame.
		const schedule = () => {
			cancelAnimationFrame(frame);
			frame = requestAnimationFrame(() => void fitNow());
		};

		const observer = new ResizeObserver(schedule);
		observer.observe(inner);
		schedule();
		return () => {
			observer.disconnect();
			cancelAnimationFrame(frame);
		};
	}, [container, content]);
}
