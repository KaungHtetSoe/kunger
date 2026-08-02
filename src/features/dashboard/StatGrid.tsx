import { StatTile } from "@/features/dashboard/StatTile";
import type { InventorySummary } from "@/types/domain";

const SYSTEM_CATEGORIES = [
  "systemService",
  "kernelComponent",
  "driver",
  "firmware",
  "desktopComponent",
] as const;

interface StatGridProps {
  summary: InventorySummary;
}

export function StatGrid({ summary }: StatGridProps) {
  const byCategory = summary.itemsByCategory;
  const systemComponents = SYSTEM_CATEGORIES.reduce(
    (total, category) => total + (byCategory[category] ?? 0),
    0,
  );
  const detectedPackageManagers = Object.values(summary.itemsByPackageManager).filter(
    (count) => (count ?? 0) > 0,
  ).length;

  const tiles: { label: string; value: number }[] = [
    { label: "Total items", value: summary.totalItems },
    { label: "Applications", value: byCategory.application ?? 0 },
    { label: "Libraries", value: byCategory.library ?? 0 },
    { label: "Fonts", value: byCategory.font ?? 0 },
    { label: "Runtimes", value: byCategory.runtime ?? 0 },
    { label: "Development packages", value: byCategory.developmentPackage ?? 0 },
    { label: "System components", value: systemComponents },
    { label: "Unclassified", value: byCategory.unclassified ?? 0 },
    { label: "Package managers detected", value: detectedPackageManagers },
    { label: "Providers with warnings", value: summary.providersWithWarnings.length },
    { label: "Likely duplicate groups", value: summary.duplicateGroupCount },
  ];

  return (
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
      {tiles.map((tile) => (
        <StatTile key={tile.label} label={tile.label} value={tile.value} />
      ))}
    </div>
  );
}
