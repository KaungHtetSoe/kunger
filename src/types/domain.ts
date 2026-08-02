/**
 * TypeScript mirror of Kunger's Rust domain types
 * (`src-tauri/src/domain/`). Field names and enum variant strings match
 * serde's `camelCase` output exactly (see ADR-0009) — these are not
 * hand-guessed, they reflect the actual `#[serde(rename_all = "camelCase")]`
 * derive on every corresponding Rust type.
 *
 * Timestamps are ISO 8601 / RFC 3339 strings (chrono's default
 * `Serialize` output for `DateTime<Utc>`), not `Date` objects — convert
 * with `new Date(value)` where needed.
 */

export type SoftwareCategory =
  | "application"
  | "commandLineTool"
  | "library"
  | "font"
  | "runtime"
  | "developmentPackage"
  | "theme"
  | "iconPack"
  | "firmware"
  | "driver"
  | "kernelComponent"
  | "systemService"
  | "desktopComponent"
  | "documentation"
  | "languagePack"
  | "miscellaneous"
  | "unclassified";

export type PackageManager =
  "apt" | "flatpak" | "snap" | "appImage" | "pip" | "pipx" | "npm" | "cargo" | "manual" | "unknown";

export type InstallationScope = "system" | "user" | "unknown";

export type InstallationReason = "manual" | "automatic" | "unknown";

export type ClassificationConfidence = "unknown" | "low" | "medium" | "high" | "certain";

export type InventoryStatus =
  | "notStarted"
  | "scanning"
  | "completed"
  | "completedWithWarnings"
  | "partiallyFailed"
  | "failed"
  | "cancelled";

export type ProviderStatus =
  "notRun" | "success" | "partialSuccess" | "unavailable" | "failed" | "timedOut" | "cancelled";

export type SoftwareRiskLevel = "unknown" | "low" | "medium" | "high";

/** Mirrors `domain::SoftwareItem`. */
export interface SoftwareItem {
  id: string;
  packageName: string;
  displayName: string;
  description: string | null;
  version: string | null;
  architecture: string | null;

  category: SoftwareCategory;
  secondaryCategories: SoftwareCategory[];

  packageManager: PackageManager;
  packageSource: string | null;
  scope: InstallationScope;

  installPaths: string[];
  executablePaths: string[];
  desktopFilePaths: string[];
  iconPath: string | null;

  packageSection: string | null;
  installedSizeBytes: number | null;
  installedAt: string | null;

  installationReason: InstallationReason;

  dependencies: string[];
  reverseDependencies: string[];

  updateAvailable: boolean;
  availableVersion: string | null;

  repository: string | null;
  homepage: string | null;
  license: string | null;

  classificationConfidence: ClassificationConfidence;
  classificationReasons: string[];
  riskLevel: SoftwareRiskLevel;

  metadata: Record<string, string>;
  warnings: string[];
}

/** Mirrors `domain::ProviderWarning`. */
export interface ProviderWarning {
  message: string;
  itemId: string | null;
  context: string | null;
}

/**
 * Mirrors `domain::ProviderError`, an externally-tagged enum
 * (`#[serde(tag = "kind", content = "message")]` — ADR-0009).
 */
export type ProviderErrorKind =
  "commandNotFound" | "timeout" | "permissionDenied" | "malformedOutput" | "cancelled" | "other";

export interface ProviderError {
  kind: ProviderErrorKind;
  message: string;
}

/** Mirrors `domain::ProviderInventory`. */
export interface ProviderInventory {
  providerId: string;
  status: ProviderStatus;
  items: SoftwareItem[];
  warnings: ProviderWarning[];
  error: ProviderError | null;
  startedAt: string;
  completedAt: string | null;
  durationMs: number | null;
  providerVersion: string | null;
}

/** Mirrors `domain::InventorySummary`. */
export interface InventorySummary {
  status: InventoryStatus;
  totalItems: number;
  itemsByCategory: Partial<Record<SoftwareCategory, number>>;
  itemsByPackageManager: Partial<Record<PackageManager, number>>;
  providersWithWarnings: string[];
  providersWithErrors: string[];
  duplicateGroupCount: number;
  lastScanStartedAt: string | null;
  lastScanCompletedAt: string | null;
  scanDurationMs: number | null;
}

/** Mirrors `domain::DuplicateGroup`. */
export interface DuplicateGroup {
  id: string;
  itemIds: string[];
  reason: string;
  confidence: ClassificationConfidence;
}
