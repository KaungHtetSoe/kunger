import { useQuery } from "@tanstack/react-query";
import { ArrowLeft } from "lucide-react";
import { Link, useParams } from "react-router-dom";

import { Badge } from "@/components/Badge";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { LoadingState } from "@/components/LoadingState";
import { ConfidenceBadge, RiskBadge } from "@/features/software-details/Badge";
import { CollapsibleList } from "@/features/software-details/CollapsibleList";
import { DuplicateBanner } from "@/features/software-details/DuplicateBanner";
import { Field, Section } from "@/features/software-details/Section";
import { useDuplicateGroups } from "@/hooks/useDuplicateGroups";
import { getSoftwareItem } from "@/services/kungerApi";
import { formatBytes, formatDateTime } from "@/utils/format";
import { formatPackageManager, formatSoftwareCategory } from "@/utils/labels";

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
  const duplicateGroups = useDuplicateGroups();

  return (
    <div className="mx-auto flex max-w-4xl flex-col gap-4 p-6">
      <Link
        to="/inventory"
        className="flex w-fit items-center gap-1.5 text-sm text-neutral-400 hover:text-neutral-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
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
        <div className="flex flex-col gap-4">
          <header className="flex flex-col gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="text-lg font-semibold text-neutral-100">{item.displayName}</h1>
              <Badge label={formatSoftwareCategory(item.category)} tone="neutral" />
              <ConfidenceBadge confidence={item.classificationConfidence} />
              <RiskBadge riskLevel={item.riskLevel} />
            </div>
            <p className="font-mono text-xs text-neutral-500">{item.id}</p>
            {item.description && <p className="text-sm text-neutral-400">{item.description}</p>}
          </header>

          {duplicateGroups.data && (
            <DuplicateBanner itemId={item.id} groups={duplicateGroups.data} />
          )}

          <Section title="Overview">
            <dl className="grid grid-cols-2 gap-x-6 gap-y-3 sm:grid-cols-3">
              <Field label="Package name" value={item.packageName} />
              <Field label="Version" value={item.version} />
              <Field label="Architecture" value={item.architecture} />
              <Field label="Package manager" value={formatPackageManager(item.packageManager)} />
              <Field label="Package source" value={item.packageSource} />
              <Field label="Package section" value={item.packageSection} />
              <Field label="Scope" value={item.scope} />
              <Field label="Installation reason" value={item.installationReason} />
              <Field label="Installed size" value={formatBytes(item.installedSizeBytes)} />
              <Field
                label="Installed at"
                value={item.installedAt ? formatDateTime(item.installedAt) : null}
              />
              <Field label="Repository" value={item.repository} />
              <Field label="Homepage" value={item.homepage} />
              <Field label="License" value={item.license} />
            </dl>
            {item.secondaryCategories.length > 0 && (
              <div className="mt-3">
                <dt className="text-xs text-neutral-500">Also matches</dt>
                <dd className="mt-1 flex flex-wrap gap-1.5">
                  {item.secondaryCategories.map((category) => (
                    <Badge key={category} label={formatSoftwareCategory(category)} tone="neutral" />
                  ))}
                </dd>
              </div>
            )}
          </Section>

          <Section title="Update">
            <dl className="grid grid-cols-2 gap-x-6 gap-y-3 sm:grid-cols-3">
              <Field label="Update available" value={item.updateAvailable ? "Yes" : "No"} />
              <Field label="Available version" value={item.availableVersion} />
            </dl>
          </Section>

          <Section
            title="Classification"
            subtitle="Inferred by Kunger's classification engine, not reported by the package manager."
          >
            <dl className="grid grid-cols-2 gap-x-6 gap-y-3 sm:grid-cols-3">
              <Field label="Confidence" value={item.classificationConfidence} />
              <Field label="Risk level" value={item.riskLevel} />
            </dl>
            {item.classificationReasons.length > 0 && (
              <div className="mt-3">
                <dt className="text-xs text-neutral-500">Reasons</dt>
                <ul className="mt-1 list-inside list-disc text-sm text-neutral-300">
                  {item.classificationReasons.map((reason) => (
                    <li key={reason}>{reason}</li>
                  ))}
                </ul>
              </div>
            )}
          </Section>

          <Section title="Paths">
            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <dt className="mb-1 text-xs text-neutral-500">Install paths</dt>
                <CollapsibleList items={item.installPaths} monospace />
              </div>
              <div>
                <dt className="mb-1 text-xs text-neutral-500">Executables</dt>
                <CollapsibleList items={item.executablePaths} monospace />
              </div>
              <div>
                <dt className="mb-1 text-xs text-neutral-500">Desktop entries</dt>
                <CollapsibleList items={item.desktopFilePaths} monospace />
              </div>
              <Field label="Icon path" value={item.iconPath} />
            </div>
          </Section>

          <Section title="Dependencies">
            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <dt className="mb-1 text-xs text-neutral-500">Depends on</dt>
                <CollapsibleList items={item.dependencies} />
              </div>
              <div>
                <dt className="mb-1 text-xs text-neutral-500">Depended on by</dt>
                <CollapsibleList items={item.reverseDependencies} />
              </div>
            </div>
          </Section>

          {Object.keys(item.metadata).length > 0 && (
            <Section
              title="Metadata"
              subtitle="Raw key-value data collected from the package manager."
            >
              <dl className="grid grid-cols-2 gap-x-6 gap-y-3 sm:grid-cols-3">
                {Object.entries(item.metadata).map(([key, value]) => (
                  <Field key={key} label={key} value={value} />
                ))}
              </dl>
            </Section>
          )}

          {item.warnings.length > 0 && (
            <Section title="Warnings">
              <ul className="list-inside list-disc text-sm text-amber-400">
                {item.warnings.map((warning) => (
                  <li key={warning}>{warning}</li>
                ))}
              </ul>
            </Section>
          )}
        </div>
      )}
    </div>
  );
}
