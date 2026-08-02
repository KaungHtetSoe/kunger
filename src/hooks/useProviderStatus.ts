import { useQuery } from "@tanstack/react-query";

import { getProviderStatus } from "@/services/kungerApi";

export const providerStatusQueryKey = ["provider-status"] as const;

export function useProviderStatus() {
  return useQuery({
    queryKey: providerStatusQueryKey,
    queryFn: getProviderStatus,
  });
}
