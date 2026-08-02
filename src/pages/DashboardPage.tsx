import { AlertTriangle, LayoutDashboard } from "lucide-react";
import { Link } from "react-router-dom";

import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { LoadingState } from "@/components/LoadingState";
import { PackageManagerBreakdown } from "@/features/dashboard/PackageManagerBreakdown";
import { ScanControls } from "@/features/dashboard/ScanControls";
import { StatGrid } from "@/features/dashboard/StatGrid";
import { useInventorySummary } from "@/hooks/useInventorySummary";
import { useScanStatus } from "@/hooks/useScanStatus";
import { formatDateTime, formatDuration } from "@/utils/format";

export function DashboardPage() {
  const { data: status } = useScanStatus();
  const { data: summary, isPending, isError, error, refetch } = useInventorySummary();

  const isRunning = status?.state === "running";

  return (
    <div className="flex flex-col gap-6 p-6">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold text-neutral-100">Dashboard</h1>
          <p className="text-sm text-neutral-500">
            A read-only inventory of software installed on this system.
          </p>
        </div>
        <ScanControls />
      </div>

      {isPending ? (
        <LoadingState label="Loading inventory summary…" />
      ) : isError ? (
        <ErrorState
          message={error instanceof Error ? error.message : String(error)}
          onRetry={() => refetch()}
        />
      ) : !summary ? (
        isRunning ? null : (
          <EmptyState
            icon={<LayoutDashboard className="h-8 w-8" />}
            title="No scan has been run yet"
            description='Click "Scan System" above to inventory the software installed on this machine.'
          />
        )
      ) : (
        <div className="flex flex-col gap-6">
          {summary.providersWithErrors.length > 0 && (
            <div className="flex items-center gap-2 rounded-md border border-amber-800 bg-amber-950 px-4 py-2 text-sm text-amber-200">
              <AlertTriangle className="h-4 w-4 shrink-0" aria-hidden="true" />
              <span>
                {summary.providersWithErrors.length} provider
                {summary.providersWithErrors.length === 1 ? "" : "s"} failed during the last scan.{" "}
                <Link to="/providers" className="underline hover:no-underline">
                  View providers
                </Link>
              </span>
            </div>
          )}

          <StatGrid summary={summary} />

          <div className="rounded-md border border-neutral-800 p-4">
            <h2 className="mb-3 text-sm font-medium text-neutral-100">
              Package manager distribution
            </h2>
            <PackageManagerBreakdown itemsByPackageManager={summary.itemsByPackageManager} />
          </div>

          <p className="text-xs text-neutral-600">
            Last scan:{" "}
            {summary.lastScanCompletedAt ? formatDateTime(summary.lastScanCompletedAt) : "unknown"}
            {summary.scanDurationMs !== null && (
              <> · Duration: {formatDuration(summary.scanDurationMs)}</>
            )}
          </p>
        </div>
      )}
    </div>
  );
}
