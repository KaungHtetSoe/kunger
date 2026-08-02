import { Search as SearchIcon } from "lucide-react";
import { useState } from "react";

import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { LoadingState } from "@/components/LoadingState";
import { FilterBar } from "@/features/inventory/FilterBar";
import { EMPTY_FILTERS, type InventoryFilters } from "@/features/inventory/filterTypes";
import { GroupedList } from "@/features/inventory/GroupedList";
import { PaginationControls } from "@/features/inventory/PaginationControls";
import { SoftwareTable } from "@/features/inventory/SoftwareTable";
import { ViewModeToggle, type ViewMode } from "@/features/inventory/ViewModeToggle";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import { useLocalStorageState } from "@/hooks/useLocalStorageState";
import { useSoftwareItems } from "@/hooks/useSoftwareItems";
import type { SortDirection, SortField } from "@/types/commands";
import type { SoftwareCategory } from "@/types/domain";

const TABLE_PAGE_SIZE = 50;
const GROUPED_PAGE_SIZE = 500;
const SEARCH_DEBOUNCE_MS = 250;

interface InventoryBrowserProps {
  title: string;
  description?: string;
  /** Pre-applied category constraint from the sidebar (e.g. "Applications"). Not user-editable here. */
  fixedCategories?: SoftwareCategory[];
  initialSearch?: string;
}

export function InventoryBrowser({
  title,
  description,
  fixedCategories,
  initialSearch,
}: InventoryBrowserProps) {
  const [search, setSearch] = useState(initialSearch ?? "");
  const debouncedSearch = useDebouncedValue(search, SEARCH_DEBOUNCE_MS);
  const [filters, setFilters] = useState<InventoryFilters>(EMPTY_FILTERS);
  const [sortBy, setSortBy] = useState<SortField>("displayName");
  const [sortDirection, setSortDirection] = useState<SortDirection>("ascending");
  const [page, setPage] = useState(1);
  const [viewMode, setViewMode] = useLocalStorageState<ViewMode>(
    "kunger.inventory.viewMode",
    "table",
  );

  const pageSize = viewMode === "table" ? TABLE_PAGE_SIZE : GROUPED_PAGE_SIZE;

  const { data, isPending, isError, error, refetch, isPlaceholderData } = useSoftwareItems({
    page: viewMode === "table" ? page : 1,
    pageSize,
    search: debouncedSearch.trim() || undefined,
    categories: fixedCategories,
    packageManagers: filters.packageManagers.length ? filters.packageManagers : undefined,
    scopes: filters.scopes.length ? filters.scopes : undefined,
    installationReasons: filters.installationReasons.length
      ? filters.installationReasons
      : undefined,
    updateAvailableOnly: filters.updateAvailableOnly || undefined,
    minConfidence: filters.minConfidence ?? undefined,
    sortBy,
    sortDirection,
  });

  function handleSort(field: SortField) {
    if (field === sortBy) {
      setSortDirection((current) => (current === "ascending" ? "descending" : "ascending"));
    } else {
      setSortBy(field);
      setSortDirection("ascending");
    }
    setPage(1);
  }

  function handleFiltersChange(next: InventoryFilters) {
    setFilters(next);
    setPage(1);
  }

  return (
    <div className="flex flex-col gap-4 p-6">
      <div className="flex items-center justify-between gap-4">
        <div>
          <h1 className="text-lg font-semibold text-neutral-100">{title}</h1>
          {description && <p className="text-sm text-neutral-500">{description}</p>}
        </div>
        <ViewModeToggle mode={viewMode} onChange={setViewMode} />
      </div>

      <div className="relative max-w-sm">
        <SearchIcon
          className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-neutral-500"
          aria-hidden="true"
        />
        <label htmlFor="inventory-search" className="sr-only">
          Filter this view by name, package, or description
        </label>
        <input
          id="inventory-search"
          type="search"
          value={search}
          onChange={(event) => {
            setSearch(event.target.value);
            setPage(1);
          }}
          placeholder="Filter by name, package, description…"
          className="w-full rounded-md border border-neutral-800 bg-neutral-900 py-1.5 pl-8 pr-3 text-sm text-neutral-100 placeholder:text-neutral-500 focus:border-neutral-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
        />
      </div>

      <FilterBar filters={filters} onChange={handleFiltersChange} />

      {isPending ? (
        <LoadingState label="Loading software items…" />
      ) : isError ? (
        <ErrorState
          message={error instanceof Error ? error.message : String(error)}
          onRetry={() => refetch()}
        />
      ) : !data || data.items.length === 0 ? (
        <EmptyState
          title="No matching software"
          description={
            search || filters.packageManagers.length || filters.scopes.length
              ? "No items match the current search and filters. Try clearing some of them."
              : "No scan has been run yet, or nothing in this category was found on this system."
          }
        />
      ) : (
        <div className={isPlaceholderData ? "opacity-60 transition-opacity" : undefined}>
          {viewMode === "table" ? (
            <>
              <SoftwareTable
                items={data.items}
                sortBy={sortBy}
                sortDirection={sortDirection}
                onSort={handleSort}
              />
              <div className="mt-3">
                <PaginationControls
                  page={page}
                  pageSize={TABLE_PAGE_SIZE}
                  totalCount={data.totalCount}
                  onPageChange={setPage}
                />
              </div>
            </>
          ) : (
            <>
              <GroupedList items={data.items} groupBy={viewMode} />
              {data.totalCount > GROUPED_PAGE_SIZE && (
                <p className="mt-3 text-xs text-neutral-600">
                  Showing the first {GROUPED_PAGE_SIZE} of {data.totalCount.toLocaleString()}{" "}
                  matching items. Use table view or narrow your filters to see the rest.
                </p>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
