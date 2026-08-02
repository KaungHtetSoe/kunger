import { NavLink } from "react-router-dom";

import { NAV_ITEMS } from "@/app/navigation";

export function Sidebar() {
  return (
    <nav
      aria-label="Primary"
      className="flex w-56 shrink-0 flex-col gap-1 overflow-y-auto border-r border-neutral-800 bg-neutral-950 p-3"
    >
      <div className="mb-3 px-2">
        <p className="text-sm font-semibold tracking-tight text-neutral-100">Kunger</p>
        <p className="text-xs text-neutral-500">Software Inventory</p>
      </div>

      {NAV_ITEMS.map(({ label, to, icon: Icon }) => (
        <NavLink
          key={to}
          to={to}
          end={to === "/" || to === "/inventory"}
          className={({ isActive }) =>
            `flex items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400 ${
              isActive
                ? "bg-neutral-800 text-neutral-50"
                : "text-neutral-400 hover:bg-neutral-900 hover:text-neutral-200"
            }`
          }
        >
          <Icon className="h-4 w-4 shrink-0" />
          <span className="truncate">{label}</span>
        </NavLink>
      ))}
    </nav>
  );
}
