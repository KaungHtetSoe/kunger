import { CheckCircle2, XCircle } from "lucide-react";

import { ErrorState } from "@/components/ErrorState";
import { LoadingState } from "@/components/LoadingState";
import { useProviderStatus } from "@/hooks/useProviderStatus";

export function ProvidersPage() {
  const { data, isPending, isError, error, refetch } = useProviderStatus();

  if (isPending) {
    return <LoadingState label="Checking provider availability…" />;
  }

  if (isError) {
    return (
      <ErrorState
        message={error instanceof Error ? error.message : String(error)}
        onRetry={() => refetch()}
      />
    );
  }

  return (
    <div className="p-6">
      <h1 className="mb-1 text-lg font-semibold text-neutral-100">Providers</h1>
      <p className="mb-6 text-sm text-neutral-500">
        Inventory sources Kunger can scan. Availability is checked independently of running a scan.
      </p>

      <ul className="divide-y divide-neutral-800 rounded-md border border-neutral-800">
        {data.map((provider) => (
          <li key={provider.id} className="flex items-start gap-3 p-4">
            {provider.available ? (
              <CheckCircle2
                className="mt-0.5 h-5 w-5 shrink-0 text-emerald-500"
                aria-hidden="true"
              />
            ) : (
              <XCircle className="mt-0.5 h-5 w-5 shrink-0 text-neutral-600" aria-hidden="true" />
            )}
            <div>
              <p className="text-sm font-medium text-neutral-100">{provider.displayName}</p>
              <p className="text-sm text-neutral-500">{provider.description}</p>
              <p className="mt-1 text-xs text-neutral-600">
                {provider.available ? "Available" : "Not available on this system"}
              </p>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}
