import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { DashboardPage } from "./DashboardPage";
import { NotificationProvider } from "@/components/NotificationProvider";
import type { InventorySummary } from "@/types/domain";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

function baseSummary(overrides: Partial<InventorySummary> = {}): InventorySummary {
  return {
    status: "completed",
    totalItems: 120,
    itemsByCategory: { application: 30 },
    itemsByPackageManager: { apt: 100, flatpak: 20 },
    providersWithWarnings: [],
    providersWithErrors: [],
    duplicateGroupCount: 0,
    lastScanStartedAt: "2026-01-01T00:00:00Z",
    lastScanCompletedAt: "2026-01-01T00:00:05Z",
    scanDurationMs: 5000,
    ...overrides,
  };
}

function renderDashboard(
  scanStatus: unknown = { state: "idle", lastSummary: null },
  summary: InventorySummary | null = null,
  summaryShouldFail = false,
) {
  invokeMock.mockImplementation((command: string) => {
    if (command === "get_scan_status") return Promise.resolve(scanStatus);
    if (command === "get_inventory_summary") {
      return summaryShouldFail
        ? Promise.reject(new Error("database is locked"))
        : Promise.resolve(summary);
    }
    return Promise.resolve(null);
  });

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <NotificationProvider>
        <MemoryRouter>
          <DashboardPage />
        </MemoryRouter>
      </NotificationProvider>
    </QueryClientProvider>,
  );
}

describe("DashboardPage", () => {
  it("shows the loading state before the summary resolves", () => {
    renderDashboard();

    expect(screen.getByText("Loading inventory summary…")).toBeInTheDocument();
  });

  it("shows an empty state when no scan has ever completed", async () => {
    renderDashboard();

    expect(await screen.findByText("No scan has been run yet")).toBeInTheDocument();
  });

  it("shows the error state and can retry", async () => {
    renderDashboard(undefined, null, true);

    expect(await screen.findByText("Something went wrong")).toBeInTheDocument();
    expect(screen.getByText("database is locked")).toBeInTheDocument();
  });

  it("renders summary stats and the package manager breakdown when data is present", async () => {
    renderDashboard(undefined, baseSummary());

    expect(await screen.findByText("120")).toBeInTheDocument();
    expect(screen.getByText("APT")).toBeInTheDocument();
    expect(screen.getByText("Flatpak")).toBeInTheDocument();
  });

  it("shows a warning banner linking to Providers when providers failed", async () => {
    renderDashboard(undefined, baseSummary({ providersWithErrors: ["flatpak"] }));

    expect(await screen.findByText(/1 provider/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "View providers" })).toHaveAttribute(
      "href",
      expect.stringContaining("/providers"),
    );
  });

  it("hides the empty state while a scan is running instead of flashing it", async () => {
    renderDashboard({ state: "running", startedAt: "2026-01-01T00:00:00Z", elapsedMs: 1000 }, null);

    expect(await screen.findByText("Scanning system…")).toBeInTheDocument();
    expect(screen.queryByText("No scan has been run yet")).not.toBeInTheDocument();
  });
});
