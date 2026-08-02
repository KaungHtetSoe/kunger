import { Copy } from "lucide-react";

import { EmptyState } from "@/components/EmptyState";

export function DuplicatesPage() {
  return (
    <EmptyState
      icon={<Copy className="h-8 w-8" />}
      title="No scan has been run yet"
      description="Likely duplicate installations across package managers (e.g. an app installed via both APT and Flatpak) will be listed here after a scan."
    />
  );
}
