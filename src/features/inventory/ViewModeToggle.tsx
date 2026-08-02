import { LayoutGrid, List, Rows3 } from "lucide-react";

export type ViewMode = "table" | "groupedByCategory" | "groupedByManager";

const OPTIONS: { mode: ViewMode; label: string; icon: typeof List }[] = [
  { mode: "table", label: "Table", icon: List },
  { mode: "groupedByCategory", label: "By category", icon: Rows3 },
  { mode: "groupedByManager", label: "By manager", icon: LayoutGrid },
];

interface ViewModeToggleProps {
  mode: ViewMode;
  onChange: (mode: ViewMode) => void;
}

export function ViewModeToggle({ mode, onChange }: ViewModeToggleProps) {
  return (
    <div
      role="group"
      aria-label="View mode"
      className="flex gap-1 rounded-md border border-neutral-800 p-0.5"
    >
      {OPTIONS.map(({ mode: optionMode, label, icon: Icon }) => (
        <button
          key={optionMode}
          type="button"
          onClick={() => onChange(optionMode)}
          aria-pressed={mode === optionMode}
          className={`flex items-center gap-1.5 rounded px-2 py-1 text-xs focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400 ${
            mode === optionMode
              ? "bg-neutral-800 text-neutral-100"
              : "text-neutral-400 hover:text-neutral-200"
          }`}
        >
          <Icon className="h-3.5 w-3.5" aria-hidden="true" />
          {label}
        </button>
      ))}
    </div>
  );
}
