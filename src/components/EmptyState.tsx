import type { ReactNode } from "react";

interface EmptyStateProps {
  title: string;
  description?: string;
  action?: ReactNode;
  icon?: ReactNode;
}

/**
 * A genuinely-empty or not-yet-available state. Never used to fake data —
 * only for "there is nothing here yet" / "this isn't built yet", stated
 * honestly.
 */
export function EmptyState({ title, description, action, icon }: EmptyStateProps) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-12 text-center">
      {icon && <div className="text-neutral-600">{icon}</div>}
      <p className="text-sm font-medium text-neutral-200">{title}</p>
      {description && <p className="max-w-sm text-sm text-neutral-500">{description}</p>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}
