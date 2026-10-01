import { useEffect, useState } from "react";
import { Input } from "@/components/ui/input";
import { parsePositive } from "@/lib/parse";

// Commits on blur or Enter rather than per keystroke, so typing "1500" never
// saves 1, 15 and 150 on the way. Invalid text reverts.
export function NumberField({
	value,
	onCommit,
	step,
	unit,
	disabled = false,
	integer = false,
	ariaLabel,
}: {
	value: number;
	onCommit: (n: number) => void;
	step: number;
	unit: string;
	disabled?: boolean;
	integer?: boolean;
	ariaLabel: string;
}) {
	const [text, setText] = useState(String(value));
	useEffect(() => setText(String(value)), [value]);

	const commit = () => {
		const n = parsePositive(text, integer);
		if (n == null || n === value) {
			setText(String(value));
			return;
		}
		onCommit(n);
	};

	return (
		<div className="flex items-center gap-1">
			<Input
				type="number"
				min={integer ? 1 : 0}
				step={step}
				value={text}
				disabled={disabled}
				aria-label={ariaLabel}
				onChange={(e) => setText(e.currentTarget.value)}
				onBlur={commit}
				onKeyDown={(e) => {
					if (e.key === "Enter") e.currentTarget.blur();
					if (e.key === "Escape") setText(String(value));
				}}
				className="h-8 w-20"
			/>
			<span className="text-xs text-muted-foreground">{unit}</span>
		</div>
	);
}
