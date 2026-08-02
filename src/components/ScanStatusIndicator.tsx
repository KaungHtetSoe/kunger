import { Loader2, PlayCircle } from "lucide-react";
import { useEffect, useRef } from "react";

import { useNotifications } from "@/components/notificationContext";
import { useScanStatus } from "@/hooks/useScanStatus";
import { scanEvents } from "@/services/kungerApi";

export function ScanStatusIndicator() {
  const { data, isPending } = useScanStatus();
  const { notify } = useNotifications();
  const hasAttachedListeners = useRef(false);

  useEffect(() => {
    if (hasAttachedListeners.current) return;
    hasAttachedListeners.current = true;

    const unlistenPromises = [
      scanEvents.onCompleted((summary) =>
        notify("success", `Scan completed — ${summary.totalItems} items found.`),
      ),
      scanEvents.onFailed((message) => notify("error", `Scan failed: ${message}`)),
      scanEvents.onCancelled(() => notify("info", "Scan cancelled.")),
    ];

    return () => {
      unlistenPromises.forEach((pending) => {
        pending.then((unlisten) => unlisten()).catch(() => {});
      });
    };
  }, [notify]);

  if (isPending || !data) {
    return <span className="text-xs text-neutral-600">—</span>;
  }

  if (data.state === "running") {
    const elapsedSeconds = Math.max(0, Math.round(data.elapsedMs / 1000));
    return (
      <span className="flex items-center gap-1.5 text-xs text-neutral-300">
        <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" />
        Scanning… ({elapsedSeconds}s)
      </span>
    );
  }

  if (!data.lastSummary) {
    return (
      <span className="flex items-center gap-1.5 text-xs text-neutral-500">
        <PlayCircle className="h-3.5 w-3.5" aria-hidden="true" />
        No scan yet
      </span>
    );
  }

  const completedAt = data.lastSummary.lastScanCompletedAt
    ? new Date(data.lastSummary.lastScanCompletedAt).toLocaleString()
    : "unknown time";

  return <span className="text-xs text-neutral-500">Last scan: {completedAt}</span>;
}
