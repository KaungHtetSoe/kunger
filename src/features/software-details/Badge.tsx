import { Badge } from "@/components/Badge";
import type { BadgeProps } from "@/components/Badge";

const CONFIDENCE_TONE: Record<string, BadgeProps["tone"]> = {
  certain: "positive",
  high: "positive",
  medium: "neutral",
  low: "caution",
  unknown: "neutral",
};

export function ConfidenceBadge({ confidence }: { confidence: string }) {
  return (
    <Badge label={`${confidence} confidence`} tone={CONFIDENCE_TONE[confidence] ?? "neutral"} />
  );
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
