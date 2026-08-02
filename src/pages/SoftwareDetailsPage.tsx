import { useQuery } from "@tanstack/react-query";
import { ArrowLeft } from "lucide-react";
import { Link, useParams } from "react-router-dom";

import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { LoadingState } from "@/components/LoadingState";
import { getSoftwareItem } from "@/services/kungerApi";
import { formatPackageManager, formatSoftwareCategory } from "@/utils/labels";

// Minimal working version -- full field coverage (dependencies, warnings,
// classification reasons, etc.) lands in the dedicated software-details
// milestone right after this one.
export function SoftwareDetailsPage() {
  const { id } = useParams<{ id: string }>();
  const {
    data: item,
    isPending,
    isError,
    error,
    refetch,
  } = useQuery({
    queryKey: ["software-item", id],
    queryFn: () => getSoftwareItem(id ?? ""),
    enabled: Boolean(id),
  });

  return (
    <div className="p-6">
      <Link
        to="/inventory"
        className="mb-4 flex w-fit items-center gap-1.5 text-sm text-neutral-400 hover:text-neutral-200"
      >
        <ArrowLeft className="h-4 w-4" aria-hidden="true" />
        Back to inventory
      </Link>

      {isPending ? (
        <LoadingState label="Loading item…" />
      ) : isError ? (
        <ErrorState
          message={error instanceof Error ? error.message : String(error)}
          onRetry={() => refetch()}
        />
      ) : !item ? (
        <EmptyState
          title="Item not found"
          description="This item may no longer be present in the latest scan."
        />
      ) : (
        <div>
          <h1 className="text-lg font-semibold text-neutral-100">{item.displayName}</h1>
          <p className="text-sm text-neutral-500">{item.packageName}</p>
          <dl className="mt-4 grid grid-cols-2 gap-x-6 gap-y-2 text-sm sm:grid-cols-3">
            <Field label="Category" value={formatSoftwareCategory(item.category)} />
            <Field label="Manager" value={formatPackageManager(item.packageManager)} />
            <Field label="Version" value={item.version ?? "—"} />
          </dl>
          {item.description && <p className="mt-4 text-sm text-neutral-400">{item.description}</p>}
        </div>
      )}
    </div>
  );
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs text-neutral-500">{label}</dt>
      <dd className="text-neutral-200">{value}</dd>
    </div>
  );
}
