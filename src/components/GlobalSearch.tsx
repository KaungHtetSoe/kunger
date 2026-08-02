import { Search } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";

export function GlobalSearch() {
  const [value, setValue] = useState("");
  const navigate = useNavigate();

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const trimmed = value.trim();
    navigate(trimmed ? `/inventory?q=${encodeURIComponent(trimmed)}` : "/inventory");
  }

  return (
    <form onSubmit={handleSubmit} role="search" className="flex-1">
      <label htmlFor="global-search" className="sr-only">
        Search installed software
      </label>
      <div className="relative max-w-md">
        <Search
          className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-neutral-500"
          aria-hidden="true"
        />
        <input
          id="global-search"
          type="search"
          value={value}
          onChange={(event) => setValue(event.target.value)}
          placeholder="Search name, package, description…"
          className="w-full rounded-md border border-neutral-800 bg-neutral-900 py-1.5 pl-8 pr-3 text-sm text-neutral-100 placeholder:text-neutral-500 focus:border-neutral-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-neutral-400"
        />
      </div>
    </form>
  );
}
