/**
 * TypeScript mirror of Kunger's Tauri command request/response DTOs
 * (`src-tauri/src/commands/types.rs`). See `src/types/domain.ts` for the
 * underlying domain types these build on.
 */

import type {
  ClassificationConfidence,
  DuplicateGroup,
  InstallationReason,
  InstallationScope,
  InventorySummary,
  PackageManager,
  ProviderError,
  ProviderWarning,
  SoftwareCategory,
  SoftwareItem,
} from "./domain";

export const MAX_PAGE_SIZE = 500;

export interface ProviderStatusResponse {
  id: string;
  displayName: string;
  description: string;
  available: boolean;
}

export interface StartScanRequest {
  /** Per-provider timeout in milliseconds. Server defaults to 30s if omitted. */
  perProviderTimeoutMs?: number;
}

export type ScanStatusResponse =
  | { state: "idle"; lastSummary: InventorySummary | null }
  | { state: "running"; startedAt: string; elapsedMs: number };

export type SortField =
  "displayName" | "category" | "packageManager" | "version" | "installedSize" | "confidence";

export type SortDirection = "ascending" | "descending";

export interface ListSoftwareItemsRequest {
  /** 1-based page number. Defaults to 1. */
  page?: number;
  /** Defaults to 50, must be between 1 and {@link MAX_PAGE_SIZE}. */
  pageSize?: number;
  search?: string;
  categories?: SoftwareCategory[];
  packageManagers?: PackageManager[];
  scopes?: InstallationScope[];
  installationReasons?: InstallationReason[];
  updateAvailableOnly?: boolean;
  minConfidence?: ClassificationConfidence;
  sortBy?: SortField;
  sortDirection?: SortDirection;
}

export interface ListSoftwareItemsResponse {
  items: SoftwareItem[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface ProviderWarningsResponse {
  providerId: string;
  warnings: ProviderWarning[];
  error: ProviderError | null;
}

export type ExportFormat = "json" | "yaml" | "csv";

/**
 * `full` dumps every scanned field verbatim. `reinstallationManifest`
 * instead separates items whose package manager can reinstall them by name
 * from items Kunger can only flag for manual review (product spec FR-11).
 */
export type ExportMode = "full" | "reinstallationManifest";

export interface ExportRequest {
  format: ExportFormat;
  mode?: ExportMode;
}

export interface ExportResponse {
  schemaVersion: number;
  format: ExportFormat;
  content: string;
}

/** Mirrors `commands::export::ReinstallManifest` (only reachable when `mode: "reinstallationManifest"`). */
export interface ReinstallManifest {
  schemaVersion: number;
  exportedAt: string;
  reproducible: ReproducibleGroup[];
  manualReview: ManualReviewItem[];
}

export interface ReproducibleGroup {
  packageManager: PackageManager;
  installHint: string;
  packages: ReproduciblePackage[];
}

export interface ReproduciblePackage {
  packageName: string;
  displayName: string;
  version: string | null;
}

export interface ManualReviewItem {
  id: string;
  displayName: string;
  packageManager: PackageManager;
  reason: string;
  paths: string[];
}

/**
 * Structured error shape every Kunger command rejects with
 * (`commands::CommandError`) — `kind` is machine-readable, `message` is
 * for display.
 */
export interface CommandError {
  kind: "invalidRequest" | "notFound" | "conflict" | "internal";
  message: string;
}

export type { DuplicateGroup };
