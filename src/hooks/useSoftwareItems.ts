import { useQuery } from "@tanstack/react-query";

import { softwareItemsQueryKey } from "@/hooks/useScanStatus";
import { listSoftwareItems } from "@/services/kungerApi";
import type { ListSoftwareItemsRequest } from "@/types/commands";

export function useSoftwareItems(request: ListSoftwareItemsRequest) {
  return useQuery({
    queryKey: [...softwareItemsQueryKey, request],
    queryFn: () => listSoftwareItems(request),
    placeholderData: (previous) => previous,
  });
}
