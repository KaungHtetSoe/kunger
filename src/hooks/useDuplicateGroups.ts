import { useQuery } from "@tanstack/react-query";

import { duplicateGroupsQueryKey } from "@/hooks/useScanStatus";
import { listDuplicateGroups } from "@/services/kungerApi";

export function useDuplicateGroups() {
  return useQuery({
    queryKey: duplicateGroupsQueryKey,
    queryFn: listDuplicateGroups,
  });
}
