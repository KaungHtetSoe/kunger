import type { PackageManager } from "@/types/domain";
import { formatCount, formatPackageManager } from "@/utils/labels";

interface PackageManagerBreakdownProps {
  itemsByPackageManager: Partial<Record<PackageManager, number>>;
}

export function PackageManagerBreakdown({ itemsByPackageManager }: PackageManagerBreakdownProps) {
  const entries = Object.entries(itemsByPackageManager) as [PackageManager, number][];
  const sorted = entries.filter(([, count]) => count > 0).sort((a, b) => b[1] - a[1]);
  const maxCount = Math.max(...sorted.map(([, count]) => count), 1);

  if (sorted.length === 0) {
    return <p className="text-sm text-neutral-500">No package manager data.</p>;
  }

  return (
    <div className="space-y-3">
      {sorted.map(([manager, count]) => (
        <div key={manager} className="flex items-center gap-3">
          <span className="w-24 shrink-0 truncate text-sm text-neutral-400">
            {formatPackageManager(manager)}
          </span>
          <div className="h-5 flex-1 rounded-sm bg-neutral-900">
            <div
              className="h-5 rounded-r-sm bg-sky-500"
              style={{ width: `${Math.max((count / maxCount) * 100, 2)}%` }}
            />
          </div>
          <span className="w-12 shrink-0 text-right text-sm tabular-nums text-neutral-300">
            {formatCount(count)}
          </span>
        </div>
      ))}
    </div>
  );
}
