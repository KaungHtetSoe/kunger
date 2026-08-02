interface BadgeProps {
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

const CONFIDENCE_TONE: Record<string, BadgeProps["tone"]> = {
  certain: "positive",
  high: "positive",
  medium: "neutral",
  low: "caution",
  unknown: "neutral",
};

export function ConfidenceBadge({ confidence }: { confidence: string }) {
  return <Badge label={`${confidence} confidence`} tone={CONFIDENCE_TONE[confidence] ?? "neutral"} />;
}

const RISK_TONE: Record<string, BadgeProps["tone"]> = {
  high: "critical",
  medium: "caution",
  low: "positive",
  unknown: "neutral",
};

export function RiskBadge({ riskLevel }: { riskLevel: string }) {
  if (riskLevel === "unknown") {
    return null;
  }
  return <Badge label={`${riskLevel} risk`} tone={RISK_TONE[riskLevel] ?? "neutral"} />;
}
