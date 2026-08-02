export interface BadgeProps {
  label: string;
  tone: "neutral" | "positive" | "caution" | "critical";
}

const TONE_CLASSES: Record<BadgeProps["tone"], string> = {
  neutral: "bg-neutral-800 text-neutral-300",
  positive: "border border-emerald-900 bg-emerald-950 text-emerald-400",
  caution: "border border-amber-900 bg-amber-950 text-amber-400",
  critical: "border border-red-900 bg-red-950 text-red-400",
};

export function Badge({ label, tone }: BadgeProps) {
  return (
    <span
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium capitalize ${TONE_CLASSES[tone]}`}
    >
      {label}
    </span>
  );
}
