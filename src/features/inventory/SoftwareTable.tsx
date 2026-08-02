import { ArrowDown, ArrowUp, ArrowUpDown, CircleAlert } from "lucide-react";
import { Link } from "react-router-dom";

import type { SortDirection, SortField } from "@/types/commands";
import type { SoftwareItem } from "@/types/domain";
import { formatSoftwareCategory, formatPackageManager } from "@/utils/labels";
import { formatBytes } from "@/utils/format";

interface Column {
  key: SortField;
  label: string;
}

const COLUMNS: Column[] = [
  { key: "displayName", label: "Name" },
  { key: "category", label: "Category" },
  { key: "packageManager", label: "Manager" },
  { key: "version", label: "Version" },
  { key: "installedSize", label: "Size" },
  { key: "confidence", label: "Confidence" },
];

interface SoftwareTableProps {
  items: SoftwareItem[];
  sortBy: SortField;
  sortDirection: SortDirection;
  onSort: (field: SortField) => void;
}

export function SoftwareTable({ items, sortBy, sortDirection, onSort }: SoftwareTableProps) {
  return (
    <div className="overflow-x-auto rounded-md border border-neutral-800">
      <table className="w-full min-w-[720px] border-collapse text-sm">
        <thead>
          <tr className="border-b border-neutral-800 text-left text-xs uppercase tracking-wide text-neutral-500">
            {COLUMNS.map((column) => (
              <th key={column.key} scope="col" className="px-3 py-2 font-medium">
                <button
                  type="button"
                  onClick={() => onSort(column.key)}
                  className="flex items-center gap-1 hover:text-neutral-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
                >
                  {column.label}
                  <SortIcon active={sortBy === column.key} direction={sortDirection} />
                </button>
              </th>
            ))}
            <th scope="col" className="px-3 py-2 font-medium">
              Scope
            </th>
            <th scope="col" className="px-3 py-2 font-medium">
              Install reason
            </th>
            <th scope="col" className="px-3 py-2 font-medium">
              Update
            </th>
          </tr>
        </thead>
        <tbody>
          {items.map((item) => (
            <tr
              key={item.id}
              className="border-b border-neutral-900 last:border-0 hover:bg-neutral-900/60"
            >
              <td className="px-3 py-2">
                <Link
                  to={`/software/${encodeURIComponent(item.id)}`}
                  className="font-medium text-neutral-100 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
                >
                  {item.displayName}
                </Link>
                <p className="text-xs text-neutral-500">{item.packageName}</p>
              </td>
              <td className="px-3 py-2 text-neutral-300">
                {formatSoftwareCategory(item.category)}
              </td>
              <td className="px-3 py-2 text-neutral-300">
                {formatPackageManager(item.packageManager)}
              </td>
              <td className="px-3 py-2 text-neutral-300">{item.version ?? "—"}</td>
              <td className="px-3 py-2 text-neutral-300">{formatBytes(item.installedSizeBytes)}</td>
              <td className="px-3 py-2 text-neutral-300 capitalize">
                {item.classificationConfidence}
              </td>
              <td className="px-3 py-2 text-neutral-300 capitalize">{item.scope}</td>
              <td className="px-3 py-2 text-neutral-300 capitalize">{item.installationReason}</td>
              <td className="px-3 py-2">
                {item.updateAvailable ? (
                  <span
                    className="flex items-center gap-1 text-amber-400"
                    title={item.availableVersion ?? undefined}
                  >
                    <CircleAlert className="h-3.5 w-3.5" aria-hidden="true" />
                    <span className="text-xs">Available</span>
                  </span>
                ) : (
                  <span className="text-xs text-neutral-600">Up to date</span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function SortIcon({ active, direction }: { active: boolean; direction: SortDirection }) {
  if (!active) {
    return <ArrowUpDown className="h-3 w-3 text-neutral-600" aria-hidden="true" />;
  }
  return direction === "ascending" ? (
    <ArrowUp className="h-3 w-3" aria-hidden="true" />
  ) : (
    <ArrowDown className="h-3 w-3" aria-hidden="true" />
  );
}
