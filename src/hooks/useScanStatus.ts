import { useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import { getScanStatus, scanEvents } from "@/services/kungerApi";
import type { ScanStatusResponse } from "@/types/commands";

export const scanStatusQueryKey = ["scan-status"] as const;
export const inventorySummaryQueryKey = ["inventory-summary"] as const;
export const softwareItemsQueryKey = ["software-items"] as const;
export const duplicateGroupsQueryKey = ["duplicate-groups"] as const;
export const providerWarningsQueryKey = ["provider-warnings"] as const;

/**
 * Tracks scan status, polling once per second while a scan is running and
 * otherwise relying on the `scan-*` Tauri events to invalidate the cache
 * immediately rather than polling continuously while idle.
 */
export function useScanStatus() {
  const queryClient = useQueryClient();

  const query = useQuery<ScanStatusResponse>({
    queryKey: scanStatusQueryKey,
    queryFn: getScanStatus,
    refetchInterval: (q) => (q.state.data?.state === "running" ? 1000 : false),
  });

  useEffect(() => {
    const invalidateStatus = () => queryClient.invalidateQueries({ queryKey: scanStatusQueryKey });
    const invalidateResults = () => {
      queryClient.invalidateQueries({ queryKey: inventorySummaryQueryKey });
      queryClient.invalidateQueries({ queryKey: softwareItemsQueryKey });
      queryClient.invalidateQueries({ queryKey: duplicateGroupsQueryKey });
      queryClient.invalidateQueries({ queryKey: providerWarningsQueryKey });
    };

    const unlistenPromises = [
      scanEvents.onStarted(invalidateStatus),
      scanEvents.onCompleted(() => {
        invalidateStatus();
        invalidateResults();
      }),
      scanEvents.onFailed(invalidateStatus),
      scanEvents.onCancelled(invalidateStatus),
    ];

    return () => {
      unlistenPromises.forEach((pending) => {
        pending.then((unlisten) => unlisten()).catch(() => {});
      });
    };
  }, [queryClient]);

  return query;
}
