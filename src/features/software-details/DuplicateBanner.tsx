import { Copy } from "lucide-react";
import { Link } from "react-router-dom";

import type { DuplicateGroup } from "@/types/domain";

interface DuplicateBannerProps {
  itemId: string;
  groups: DuplicateGroup[];
}

export function DuplicateBanner({ itemId, groups }: DuplicateBannerProps) {
  const matchingGroup = groups.find((group) => group.itemIds.includes(itemId));
  if (!matchingGroup) {
    return null;
  }

  const otherIds = matchingGroup.itemIds.filter((id) => id !== itemId);

  return (
    <div className="flex items-start gap-2 rounded-md border border-amber-800 bg-amber-950 px-4 py-3 text-sm text-amber-200">
      <Copy className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
      <div>
        <p>
          Possibly installed more than once, via a different package manager (
          {matchingGroup.confidence} confidence). {matchingGroup.reason}
        </p>
        <p className="mt-1 flex flex-wrap gap-x-3">
          {otherIds.map((id) => (
            <Link
              key={id}
              to={`/software/${encodeURIComponent(id)}`}
              className="underline hover:no-underline"
            >
              {id}
            </Link>
          ))}
        </p>
      </div>
    </div>
  );
}
