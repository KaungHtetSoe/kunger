import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Loader2, RadarIcon, XCircle } from "lucide-react";

import { useNotifications } from "@/components/notificationContext";
import { scanStatusQueryKey, useScanStatus } from "@/hooks/useScanStatus";
import { cancelInventoryScan, startInventoryScan } from "@/services/kungerApi";
import type { ScanStatusResponse } from "@/types/commands";

export function ScanControls() {
  const { data } = useScanStatus();
  const { notify } = useNotifications();
  const queryClient = useQueryClient();

  const startMutation = useMutation({
    mutationFn: () => startInventoryScan(),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: scanStatusQueryKey }),
    onError: (error) => notify("error", `Could not start scan: ${messageOf(error)}`),
  });

  const cancelMutation = useMutation({
    mutationFn: cancelInventoryScan,
    onError: (error) => notify("error", `Could not cancel scan: ${messageOf(error)}`),
  });

  const isRunning = isRunningStatus(data);

  if (isRunning) {
    const elapsedSeconds = Math.max(0, Math.round(data.elapsedMs / 1000));
    return (
      <div className="flex items-center gap-3 rounded-md border border-neutral-800 bg-neutral-900 px-4 py-3">
        <Loader2 className="h-5 w-5 shrink-0 animate-spin text-sky-400" aria-hidden="true" />
        <div className="flex-1">
          <p className="text-sm font-medium text-neutral-100">Scanning system…</p>
          <p className="text-xs text-neutral-500">
            {elapsedSeconds}s elapsed. Providers run independently — one failing or timing out won't
            stop the others.
          </p>
        </div>
        <button
          type="button"
          onClick={() => cancelMutation.mutate()}
          disabled={cancelMutation.isPending}
          className="flex items-center gap-1.5 rounded-md border border-neutral-700 px-3 py-1.5 text-sm hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400 disabled:opacity-50"
        >
          <XCircle className="h-4 w-4" aria-hidden="true" />
          Cancel
        </button>
      </div>
    );
  }

  return (
    <button
      type="button"
      onClick={() => startMutation.mutate()}
      disabled={startMutation.isPending}
      className="flex items-center gap-2 rounded-md bg-sky-600 px-4 py-2 text-sm font-medium text-white hover:bg-sky-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-400 disabled:opacity-50"
    >
      <RadarIcon className="h-4 w-4" aria-hidden="true" />
      {startMutation.isPending ? "Starting…" : "Scan System"}
    </button>
  );
}

function isRunningStatus(
  data: ScanStatusResponse | undefined,
): data is Extract<ScanStatusResponse, { state: "running" }> {
  return data?.state === "running";
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
