import { LayoutDashboard } from "lucide-react";

import { EmptyState } from "@/components/EmptyState";

export function DashboardPage() {
  // Scan summary stats and the "Scan System" trigger are added in the
  // dashboard milestone that follows this one — this honestly reflects a
  // system with no scan history yet in the meantime.
  return (
    <EmptyState
      icon={<LayoutDashboard className="h-8 w-8" />}
      title="No scan has been run yet"
      description="Kunger hasn't inventoried this system yet. Once a scan runs, your software inventory summary will appear here."
    />
  );
}
