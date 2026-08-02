import { Badge } from "@/components/Badge";
import { CollapsibleList } from "@/features/software-details/CollapsibleList";
import type { ReinstallManifest } from "@/types/commands";
import { formatPackageManager } from "@/utils/labels";

interface ManifestPreviewProps {
  manifest: ReinstallManifest;
}

/**
 * Renders the reinstallation manifest's two sections side by side, on-page,
 * before the user downloads anything -- product spec FR-11 requires the
 * split between "can reproduce automatically" and "needs manual review" to
 * be clear, not just implicit in a downloaded file's structure.
 */
export function ManifestPreview({ manifest }: ManifestPreviewProps) {
  const reproducibleCount = manifest.reproducible.reduce(
    (total, group) => total + group.packages.length,
    0,
  );

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <section className="rounded-md border border-emerald-900 bg-emerald-950/20 p-4">
        <h2 className="text-sm font-medium text-emerald-300">
          Can reinstall automatically ({reproducibleCount})
        </h2>
        <p className="mt-1 text-xs text-neutral-500">
          Run each manager's command with the listed packages substituted in.
        </p>
        {manifest.reproducible.length === 0 ? (
          <p className="mt-3 text-sm italic text-neutral-600">Nothing in this category.</p>
        ) : (
          <div className="mt-3 flex flex-col gap-3">
            {manifest.reproducible.map((group) => (
              <div key={group.packageManager}>
                <div className="flex items-center gap-2">
                  <Badge label={formatPackageManager(group.packageManager)} tone="positive" />
                  <span className="text-xs text-neutral-500">{group.packages.length}</span>
                </div>
                <p className="mt-1 font-mono text-xs text-neutral-400">{group.installHint}</p>
                <div className="mt-1.5">
                  <CollapsibleList
                    items={group.packages.map((pkg) => pkg.packageName)}
                    initialCount={5}
                    monospace
                  />
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="rounded-md border border-amber-900 bg-amber-950/20 p-4">
        <h2 className="text-sm font-medium text-amber-300">
          Needs manual review ({manifest.manualReview.length})
        </h2>
        <p className="mt-1 text-xs text-neutral-500">
          No package registry entry -- Kunger cannot verify how to reproduce these.
        </p>
        {manifest.manualReview.length === 0 ? (
          <p className="mt-3 text-sm italic text-neutral-600">Nothing in this category.</p>
        ) : (
          <ul className="mt-3 flex flex-col gap-3 text-sm">
            {manifest.manualReview.map((item) => (
              <li key={item.id}>
                <div className="flex items-center gap-2">
                  <span className="text-neutral-200">{item.displayName}</span>
                  <Badge label={formatPackageManager(item.packageManager)} tone="neutral" />
                </div>
                <p className="mt-0.5 text-xs text-neutral-500">{item.reason}</p>
                {item.paths.length > 0 && (
                  <p className="mt-0.5 break-all font-mono text-xs text-neutral-600">
                    {item.paths.join(", ")}
                  </p>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
