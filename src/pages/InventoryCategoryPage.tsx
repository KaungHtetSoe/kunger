import { useParams, useSearchParams } from "react-router-dom";

import { NAV_ITEMS } from "@/app/navigation";
import { InventoryBrowser } from "@/features/inventory/InventoryBrowser";

export function InventoryCategoryPage() {
  const { category } = useParams<{ category?: string }>();
  const [searchParams] = useSearchParams();
  const search = searchParams.get("q") ?? undefined;

  const navItem = NAV_ITEMS.find(
    (item) => item.to === (category ? `/inventory/${category}` : "/inventory"),
  );

  return (
    <InventoryBrowser
      // Remounts fresh (search/filters/page/sort all reset) whenever the
      // sidebar category changes or a new global search comes in via the
      // top bar -- simpler and more predictable than syncing internal
      // state to these props after the fact.
      key={`${category ?? "all"}:${search ?? ""}`}
      title={navItem?.label ?? "All Software"}
      fixedCategories={navItem?.categories}
      initialSearch={search}
    />
  );
}
