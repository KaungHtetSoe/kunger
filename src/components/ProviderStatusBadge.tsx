import { Link } from "react-router-dom";
import { Plug } from "lucide-react";

import { useProviderStatus } from "@/hooks/useProviderStatus";

export function ProviderStatusBadge() {
  const { data, isPending } = useProviderStatus();

  if (isPending || !data) {
    return null;
  }

  const availableCount = data.filter((provider) => provider.available).length;
  const allAvailable = availableCount === data.length;

  return (
    <Link
      to="/providers"
      className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-neutral-400 hover:bg-neutral-900 hover:text-neutral-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
      title={`${availableCount} of ${data.length} providers available`}
    >
      <Plug
        className={`h-3.5 w-3.5 ${allAvailable ? "text-emerald-500" : "text-amber-500"}`}
        aria-hidden="true"
      />
      {availableCount}/{data.length} providers
    </Link>
  );
}
