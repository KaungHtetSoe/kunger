import { formatCount } from "@/utils/labels";

interface StatTileProps {
  label: string;
  value: number;
}

export function StatTile({ label, value }: StatTileProps) {
  return (
    <div className="rounded-md border border-neutral-800 bg-neutral-900 p-4">
      <p className="text-sm text-neutral-500">{label}</p>
      <p className="mt-1 text-2xl font-semibold tabular-nums text-neutral-100">
        {formatCount(value)}
      </p>
    </div>
  );
}
