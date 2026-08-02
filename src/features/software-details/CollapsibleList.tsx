import { useState } from "react";

interface CollapsibleListProps {
  items: string[];
  /** How many items to show before collapsing. */
  initialCount?: number;
  emptyLabel?: string;
  monospace?: boolean;
}

/**
 * For long lists (dependencies, install paths, ...): shows the first
 * `initialCount` items with a "Show all" toggle rather than a separate
 * lazy-loaded IPC call -- the full item (including these lists) already
 * comes back in one `get_software_item` response, so there's nothing
 * further to fetch; this is purely a display affordance for long lists.
 */
export function CollapsibleList({
  items,
  initialCount = 8,
  emptyLabel = "None",
  monospace = false,
}: CollapsibleListProps) {
  const [expanded, setExpanded] = useState(false);

  if (items.length === 0) {
    return <p className="text-sm italic text-neutral-600">{emptyLabel}</p>;
  }

  const visible = expanded ? items : items.slice(0, initialCount);
  const hiddenCount = items.length - visible.length;

  return (
    <div>
      <ul className={`space-y-1 text-sm text-neutral-300 ${monospace ? "font-mono text-xs" : ""}`}>
        {visible.map((item) => (
          <li key={item} className="break-all">
            {item}
          </li>
        ))}
      </ul>
      {hiddenCount > 0 && (
        <button
          type="button"
          onClick={() => setExpanded(true)}
          className="mt-2 text-xs text-sky-400 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
        >
          Show {hiddenCount} more
        </button>
      )}
      {expanded && items.length > initialCount && (
        <button
          type="button"
          onClick={() => setExpanded(false)}
          className="mt-2 text-xs text-neutral-500 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
        >
          Show less
        </button>
      )}
    </div>
  );
}
