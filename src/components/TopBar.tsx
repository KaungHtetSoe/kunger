import { GlobalSearch } from "@/components/GlobalSearch";
import { ProviderStatusBadge } from "@/components/ProviderStatusBadge";
import { ScanStatusIndicator } from "@/components/ScanStatusIndicator";

export function TopBar() {
  return (
    <header className="flex h-14 shrink-0 items-center gap-4 border-b border-neutral-800 bg-neutral-950 px-4">
      <GlobalSearch />
      <div className="flex items-center gap-3">
        <ScanStatusIndicator />
        <ProviderStatusBadge />
      </div>
    </header>
  );
}
