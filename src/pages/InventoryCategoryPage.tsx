import { Package } from "lucide-react";
import { useParams, useSearchParams } from "react-router-dom";

import { EmptyState } from "@/components/EmptyState";
import { NAV_ITEMS } from "@/app/navigation";

export function InventoryCategoryPage() {
  const { category } = useParams<{ category?: string }>();
  const [searchParams] = useSearchParams();
  const search = searchParams.get("q");

  const navItem = NAV_ITEMS.find(
    (item) => item.to === (category ? `/inventory/${category}` : "/inventory"),
  );
  const label = navItem?.label ?? "All Software";

  return (
    <EmptyState
      icon={<Package className="h-8 w-8" />}
      title="No scan has been run yet"
      description={
        search
          ? `No cached results to search for "${search}" — run a scan first.`
          : `Once a scan runs, ${label.toLowerCase()} found on this system will be listed here.`
      }
    />
  );
}
