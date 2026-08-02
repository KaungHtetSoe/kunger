import { Copy } from "lucide-react";
import { Link } from "react-router-dom";

import { Badge } from "@/components/Badge";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { LoadingState } from "@/components/LoadingState";
import { useDuplicateGroups } from "@/hooks/useDuplicateGroups";

export function DuplicatesPage() {
  const { data: groups, isPending, isError, error, refetch } = useDuplicateGroups();

  return (
    <div className="p-6">
      <h1 className="mb-1 text-lg font-semibold text-neutral-100">Duplicates</h1>
      <p className="mb-6 text-sm text-neutral-500">
        Software that appears to be installed more than once, via different package managers. Kunger
        only flags these -- it never merges or removes anything automatically.
      </p>

      {isPending ? (
        <LoadingState label="Loading duplicate groups…" />
      ) : isError ? (
        <ErrorState
          message={error instanceof Error ? error.message : String(error)}
          onRetry={() => refetch()}
        />
      ) : !groups || groups.length === 0 ? (
        <EmptyState
          icon={<Copy className="h-8 w-8" />}
          title="No likely duplicates found"
          description="Either no scan has been run yet, or the last scan found no software installed more than once across different package managers."
        />
      ) : (
        <ul className="flex flex-col gap-3">
          {groups.map((group) => (
            <li key={group.id} className="rounded-md border border-neutral-800 p-4">
              <div className="flex items-center gap-2">
                <Badge label={`${group.confidence} confidence`} tone="neutral" />
              </div>
              <p className="mt-1.5 text-sm text-neutral-300">{group.reason}</p>
              <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1">
                {group.itemIds.map((id) => (
                  <Link
                    key={id}
                    to={`/software/${encodeURIComponent(id)}`}
                    className="font-mono text-xs text-sky-400 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
                  >
                    {id}
                  </Link>
                ))}
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
