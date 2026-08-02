/**
 * Typed wrappers around Kunger's Tauri commands
 * (`src-tauri/src/commands/`). Every function here corresponds 1:1 to a
 * `#[tauri::command]` — see that module for the Rust-side contract and
 * validation rules. Kept deliberately thin: no caching, retry, or state
 * management here (that's `docs/DECISIONS.md` ADR-0008's job, for the
 * frontend shell milestone to decide).
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  ExportRequest,
  ExportResponse,
  ListSoftwareItemsRequest,
  ListSoftwareItemsResponse,
  ProviderStatusResponse,
  ProviderWarningsResponse,
  ScanStatusResponse,
  StartScanRequest,
} from "@/types/commands";
import type { DuplicateGroup, InventorySummary, SoftwareItem } from "@/types/domain";

export function getProviderStatus(): Promise<ProviderStatusResponse[]> {
  return invoke("get_provider_status");
}

export function startInventoryScan(request: StartScanRequest = {}): Promise<void> {
  return invoke("start_inventory_scan", { request });
}

export function getScanStatus(): Promise<ScanStatusResponse> {
  return invoke("get_scan_status");
}

export function cancelInventoryScan(): Promise<void> {
  return invoke("cancel_inventory_scan");
}

export function getInventorySummary(): Promise<InventorySummary | null> {
  return invoke("get_inventory_summary");
}

export function listSoftwareItems(
  request: ListSoftwareItemsRequest = {},
): Promise<ListSoftwareItemsResponse> {
  return invoke("list_software_items", { request });
}

export function getSoftwareItem(id: string): Promise<SoftwareItem | null> {
  return invoke("get_software_item", { id });
}

export function listDuplicateGroups(): Promise<DuplicateGroup[]> {
  return invoke("list_duplicate_groups");
}

export function listProviderWarnings(): Promise<ProviderWarningsResponse[]> {
  return invoke("list_provider_warnings");
}

export function exportInventory(request: ExportRequest): Promise<ExportResponse> {
  return invoke("export_inventory", { request });
}

export function rebuildCache(): Promise<void> {
  return invoke("rebuild_cache");
}

/**
 * Scan lifecycle events emitted by `commands::events::TauriScanEventEmitter`.
 * Each `on*` helper returns the `UnlistenFn` Tauri gives back — call it in
 * a cleanup effect to avoid leaking the listener.
 */
export const scanEvents = {
  onStarted(handler: () => void): Promise<UnlistenFn> {
    return listen<void>("scan-started", () => handler());
  },
  onCompleted(handler: (summary: InventorySummary) => void): Promise<UnlistenFn> {
    return listen<InventorySummary>("scan-completed", (event) => handler(event.payload));
  },
  onFailed(handler: (message: string) => void): Promise<UnlistenFn> {
    return listen<string>("scan-failed", (event) => handler(event.payload));
  },
  onCancelled(handler: () => void): Promise<UnlistenFn> {
    return listen<void>("scan-cancelled", () => handler());
  },
};
