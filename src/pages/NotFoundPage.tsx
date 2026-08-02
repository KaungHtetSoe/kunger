import { CompassIcon } from "lucide-react";

import { EmptyState } from "@/components/EmptyState";

export function NotFoundPage() {
  return <EmptyState icon={<CompassIcon className="h-8 w-8" />} title="Page not found" />;
}
