import { Link } from "react-router-dom";

import type { SoftwareItem } from "@/types/domain";
import { formatPackageManager, formatSoftwareCategory } from "@/utils/labels";
import type { ViewMode } from "@/features/inventory/ViewModeToggle";

interface GroupedListProps {
  items: SoftwareItem[];
  groupBy: Extract<ViewMode, "groupedByCategory" | "groupedByManager">;
}

export function GroupedList({ items, groupBy }: GroupedListProps) {
  const groups = new Map<string, SoftwareItem[]>();

  for (const item of items) {
    const label =
      groupBy === "groupedByCategory"
        ? formatSoftwareCategory(item.category)
        : formatPackageManager(item.packageManager);
    const existing = groups.get(label) ?? [];
    existing.push(item);
    groups.set(label, existing);
  }

  const sortedGroups = [...groups.entries()].sort((a, b) => b[1].length - a[1].length);

  return (
    <div className="flex flex-col gap-4">
      {sortedGroups.map(([label, groupItems]) => (
        <section key={label} className="rounded-md border border-neutral-800">
          <header className="flex items-center justify-between border-b border-neutral-800 bg-neutral-900/50 px-3 py-2">
            <h3 className="text-sm font-medium text-neutral-100">{label}</h3>
            <span className="text-xs text-neutral-500">{groupItems.length}</span>
          </header>
          <ul className="divide-y divide-neutral-900">
            {groupItems.map((item) => (
              <li
                key={item.id}
                className="flex items-center justify-between px-3 py-2 text-sm hover:bg-neutral-900/60"
              >
                <Link
                  to={`/software/${encodeURIComponent(item.id)}`}
                  className="text-neutral-200 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
                >
                  {item.displayName}
                </Link>
                <span className="text-xs text-neutral-500">{item.version ?? "—"}</span>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
