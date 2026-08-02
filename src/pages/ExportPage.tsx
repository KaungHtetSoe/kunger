import { useMutation, useQuery } from "@tanstack/react-query";
import { Download, ShieldAlert } from "lucide-react";
import { useState } from "react";

import { ErrorState } from "@/components/ErrorState";
import { LoadingState } from "@/components/LoadingState";
import { useNotifications } from "@/components/notificationContext";
import { ManifestPreview } from "@/features/export/ManifestPreview";
import { useInventorySummary } from "@/hooks/useInventorySummary";
import { exportInventory } from "@/services/kungerApi";
import type { ExportFormat, ExportMode, ReinstallManifest } from "@/types/commands";
import { downloadTextFile } from "@/utils/download";

const FORMATS: { value: ExportFormat; label: string; mimeType: string; extension: string }[] = [
  { value: "json", label: "JSON", mimeType: "application/json", extension: "json" },
  { value: "yaml", label: "YAML", mimeType: "application/yaml", extension: "yaml" },
  { value: "csv", label: "CSV", mimeType: "text/csv", extension: "csv" },
];

export function ExportPage() {
  const [mode, setMode] = useState<ExportMode>("full");
  const [format, setFormat] = useState<ExportFormat>("json");
  const { notify } = useNotifications();
  const summary = useInventorySummary();

  const preview = useQuery({
    queryKey: ["export-manifest-preview"],
    queryFn: async () => {
      const response = await exportInventory({ format: "json", mode: "reinstallationManifest" });
      return JSON.parse(response.content) as ReinstallManifest;
    },
    enabled: mode === "reinstallationManifest",
  });

  const download = useMutation({
    mutationFn: () => exportInventory({ format, mode }),
    onSuccess: (response) => {
      const formatMeta = FORMATS.find((f) => f.value === format);
      const datestamp = new Date().toISOString().slice(0, 10);
      const suffix = mode === "full" ? "inventory" : "reinstall-manifest";
      downloadTextFile(
        response.content,
        `kunger-${suffix}-${datestamp}.${formatMeta?.extension ?? format}`,
        formatMeta?.mimeType ?? "text/plain",
      );
      notify("success", "Export saved to your downloads.");
    },
    onError: (error) => {
      notify("error", `Export failed: ${error instanceof Error ? error.message : String(error)}`);
    },
  });

  return (
    <div className="flex flex-col gap-4 p-6">
      <div>
        <h1 className="text-lg font-semibold text-neutral-100">Export</h1>
        <p className="text-sm text-neutral-500">
          Save the current inventory to a file. Kunger never sends this data anywhere -- export is
          local-only.
        </p>
      </div>

      <div className="flex items-start gap-2 rounded-md border border-amber-900 bg-amber-950/30 px-4 py-3 text-sm text-amber-200">
        <ShieldAlert className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
        <p>
          Exports can include full installation paths, which typically contain your home directory
          and therefore your username (e.g. <span className="font-mono">/home/alice/...</span>).
          Review the file before sharing it with anyone.
        </p>
      </div>

      <div className="flex flex-wrap items-end gap-6 rounded-md border border-neutral-800 bg-neutral-900/50 p-4">
        <fieldset>
          <legend className="mb-1.5 text-xs text-neutral-500">Mode</legend>
          <div className="flex gap-1.5">
            <ToggleButton
              label="Full technical inventory"
              active={mode === "full"}
              onClick={() => setMode("full")}
            />
            <ToggleButton
              label="Reinstallation manifest"
              active={mode === "reinstallationManifest"}
              onClick={() => setMode("reinstallationManifest")}
            />
          </div>
        </fieldset>

        <fieldset>
          <legend className="mb-1.5 text-xs text-neutral-500">Format</legend>
          <div className="flex gap-1.5">
            {FORMATS.map((f) => (
              <ToggleButton
                key={f.value}
                label={f.label}
                active={format === f.value}
                onClick={() => setFormat(f.value)}
              />
            ))}
          </div>
        </fieldset>

        <button
          type="button"
          onClick={() => download.mutate()}
          disabled={download.isPending}
          className="ml-auto flex items-center gap-1.5 rounded-md border border-sky-700 bg-sky-950 px-3 py-1.5 text-sm text-sky-200 hover:bg-sky-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 disabled:opacity-50"
        >
          <Download className="h-4 w-4" aria-hidden="true" />
          {download.isPending ? "Preparing…" : "Download export"}
        </button>
      </div>

      {mode === "full" ? (
        <p className="text-sm text-neutral-500">
          {summary.data
            ? `Will export ${summary.data.totalItems.toLocaleString()} items with every scanned field.`
            : "Will export every scanned field from the latest scan."}
        </p>
      ) : preview.isPending ? (
        <LoadingState label="Building manifest preview…" />
      ) : preview.isError ? (
        <ErrorState
          message={preview.error instanceof Error ? preview.error.message : String(preview.error)}
          onRetry={() => preview.refetch()}
        />
      ) : preview.data ? (
        <ManifestPreview manifest={preview.data} />
      ) : null}
    </div>
  );
}

function ToggleButton({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={`rounded-md border px-3 py-1.5 text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400 ${
        active
          ? "border-sky-700 bg-sky-950 text-sky-200"
          : "border-neutral-700 text-neutral-400 hover:bg-neutral-800"
      }`}
    >
      {label}
    </button>
  );
}
