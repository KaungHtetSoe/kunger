import { useQuery } from "@tanstack/react-query";

import { getInventorySummary } from "@/services/kungerApi";
import { inventorySummaryQueryKey } from "@/hooks/useScanStatus";

export function useInventorySummary() {
  return useQuery({
    queryKey: inventorySummaryQueryKey,
    queryFn: getInventorySummary,
  });
}
