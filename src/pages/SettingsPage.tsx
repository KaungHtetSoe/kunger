import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { useNotifications } from "@/components/notificationContext";
import { inventorySummaryQueryKey, softwareItemsQueryKey } from "@/hooks/useScanStatus";
import { rebuildCache } from "@/services/kungerApi";

export function SettingsPage() {
  const { notify } = useNotifications();
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);

  const mutation = useMutation({
    mutationFn: rebuildCache,
    onSuccess: () => {
      notify("success", "Cache rebuilt. Run a new scan to repopulate it.");
      queryClient.invalidateQueries({ queryKey: inventorySummaryQueryKey });
      queryClient.invalidateQueries({ queryKey: softwareItemsQueryKey });
      setConfirming(false);
    },
    onError: (error) => {
      notify(
        "error",
        `Could not rebuild the cache: ${error instanceof Error ? error.message : String(error)}`,
      );
    },
  });

  return (
    <div className="max-w-2xl p-6">
      <h1 className="mb-1 text-lg font-semibold text-neutral-100">Settings</h1>
      <p className="mb-6 text-sm text-neutral-500">
        Kunger is read-only: it never installs, updates, or removes software. There is nothing here
        that changes your system — only how Kunger's local cache behaves.
      </p>

      <section className="rounded-md border border-neutral-800 p-4">
        <h2 className="text-sm font-medium text-neutral-100">Rebuild cache</h2>
        <p className="mt-1 text-sm text-neutral-500">
          Clears all cached scan history from Kunger's local database. This does not affect your
          system in any way — the cache is always fully rebuildable from a fresh scan.
        </p>

        {confirming ? (
          <div className="mt-3 flex items-center gap-2">
            <button
              type="button"
              onClick={() => mutation.mutate()}
              disabled={mutation.isPending}
              className="rounded-md border border-red-800 bg-red-950 px-3 py-1.5 text-sm text-red-200 hover:bg-red-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-500 disabled:opacity-50"
            >
              {mutation.isPending ? "Rebuilding…" : "Confirm rebuild"}
            </button>
            <button
              type="button"
              onClick={() => setConfirming(false)}
              disabled={mutation.isPending}
              className="rounded-md border border-neutral-700 px-3 py-1.5 text-sm hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
            >
              Cancel
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={() => setConfirming(true)}
            className="mt-3 rounded-md border border-neutral-700 px-3 py-1.5 text-sm hover:bg-neutral-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
          >
            Rebuild cache
          </button>
        )}
      </section>
    </div>
  );
}
