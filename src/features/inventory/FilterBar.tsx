import type {
  ClassificationConfidence,
  InstallationReason,
  InstallationScope,
  PackageManager,
} from "@/types/domain";
import { formatPackageManager } from "@/utils/labels";
import { EMPTY_FILTERS, type InventoryFilters } from "@/features/inventory/filterTypes";

const PACKAGE_MANAGERS: PackageManager[] = [
  "apt",
  "flatpak",
  "snap",
  "appImage",
  "pip",
  "pipx",
  "npm",
  "cargo",
  "manual",
  "unknown",
];

const SCOPES: InstallationScope[] = ["system", "user", "unknown"];
const REASONS: InstallationReason[] = ["manual", "automatic", "unknown"];
const CONFIDENCES: ClassificationConfidence[] = ["unknown", "low", "medium", "high", "certain"];

interface FilterBarProps {
  filters: InventoryFilters;
  onChange: (filters: InventoryFilters) => void;
}

export function FilterBar({ filters, onChange }: FilterBarProps) {
  const hasActiveFilters =
    filters.packageManagers.length > 0 ||
    filters.scopes.length > 0 ||
    filters.installationReasons.length > 0 ||
    filters.updateAvailableOnly ||
    filters.minConfidence !== null;

  function toggle<T>(list: T[], value: T): T[] {
    return list.includes(value) ? list.filter((v) => v !== value) : [...list, value];
  }

  return (
    <div className="flex flex-wrap items-center gap-4 rounded-md border border-neutral-800 bg-neutral-900/50 p-3 text-sm">
      <MultiSelect
        label="Manager"
        options={PACKAGE_MANAGERS}
        selected={filters.packageManagers}
        format={formatPackageManager}
        onToggle={(value) =>
          onChange({ ...filters, packageManagers: toggle(filters.packageManagers, value) })
        }
      />
      <MultiSelect
        label="Scope"
        options={SCOPES}
        selected={filters.scopes}
        onToggle={(value) => onChange({ ...filters, scopes: toggle(filters.scopes, value) })}
      />
      <MultiSelect
        label="Install reason"
        options={REASONS}
        selected={filters.installationReasons}
        onToggle={(value) =>
          onChange({ ...filters, installationReasons: toggle(filters.installationReasons, value) })
        }
      />

      <label className="flex items-center gap-1.5 text-neutral-300">
        <input
          type="checkbox"
          checked={filters.updateAvailableOnly}
          onChange={(event) => onChange({ ...filters, updateAvailableOnly: event.target.checked })}
          className="rounded border-neutral-700 bg-neutral-900"
        />
        Updates available only
      </label>

      <label className="flex items-center gap-1.5 text-neutral-300">
        Min. confidence
        <select
          value={filters.minConfidence ?? ""}
          onChange={(event) =>
            onChange({
              ...filters,
              minConfidence: event.target.value
                ? (event.target.value as ClassificationConfidence)
                : null,
            })
          }
          className="rounded-md border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm capitalize focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
        >
          <option value="">Any</option>
          {CONFIDENCES.map((confidence) => (
            <option key={confidence} value={confidence} className="capitalize">
              {confidence}
            </option>
          ))}
        </select>
      </label>

      {hasActiveFilters && (
        <button
          type="button"
          onClick={() => onChange(EMPTY_FILTERS)}
          className="ml-auto rounded-md border border-neutral-700 px-2 py-1 text-xs text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
        >
          Reset filters
        </button>
      )}
    </div>
  );
}

interface MultiSelectProps<T extends string> {
  label: string;
  options: T[];
  selected: T[];
  onToggle: (value: T) => void;
  format?: (value: T) => string;
}

function MultiSelect<T extends string>({
  label,
  options,
  selected,
  onToggle,
  format,
}: MultiSelectProps<T>) {
  return (
    <div className="flex items-center gap-1.5">
      <span className="text-neutral-500">{label}:</span>
      <div className="flex flex-wrap gap-1">
        {options.map((option) => {
          const isSelected = selected.includes(option);
          return (
            <button
              key={option}
              type="button"
              onClick={() => onToggle(option)}
              aria-pressed={isSelected}
              className={`rounded-full border px-2 py-0.5 text-xs capitalize focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400 ${
                isSelected
                  ? "border-sky-700 bg-sky-950 text-sky-200"
                  : "border-neutral-700 text-neutral-400 hover:bg-neutral-800"
              }`}
            >
              {format ? format(option) : option}
            </button>
          );
        })}
      </div>
    </div>
  );
}
