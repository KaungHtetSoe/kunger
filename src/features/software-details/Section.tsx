import type { ReactNode } from "react";

interface SectionProps {
  title: string;
  subtitle?: string;
  children: ReactNode;
}

export function Section({ title, subtitle, children }: SectionProps) {
  return (
    <section className="rounded-md border border-neutral-800 p-4">
      <h2 className="text-sm font-medium text-neutral-100">{title}</h2>
      {subtitle && <p className="mb-3 mt-0.5 text-xs text-neutral-500">{subtitle}</p>}
      <div className={subtitle ? "" : "mt-3"}>{children}</div>
    </section>
  );
}

/** A label/value pair. Renders "Not available" (not blank) for null values,
 * so the user knows Kunger checked rather than silently omitting the field. */
export function Field({ label, value }: { label: string; value: string | number | null }) {
  return (
    <div>
      <dt className="text-xs text-neutral-500">{label}</dt>
      <dd
        className={value === null ? "text-sm italic text-neutral-600" : "text-sm text-neutral-200"}
      >
        {value === null ? "Not available" : value}
      </dd>
    </div>
  );
}
