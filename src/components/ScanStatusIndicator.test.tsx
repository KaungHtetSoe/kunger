import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ScanStatusIndicator } from "./ScanStatusIndicator";
import { NotificationProvider } from "@/components/NotificationProvider";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

function renderIndicator(scanStatus: unknown) {
  invokeMock.mockImplementation((command: string) =>
    command === "get_scan_status" ? Promise.resolve(scanStatus) : Promise.resolve(null),
  );

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <NotificationProvider>
        <ScanStatusIndicator />
      </NotificationProvider>
    </QueryClientProvider>,
  );
}

describe("ScanStatusIndicator", () => {
  it("shows 'No scan yet' when idle with no prior summary", async () => {
    renderIndicator({ state: "idle", lastSummary: null });

    expect(await screen.findByText("No scan yet")).toBeInTheDocument();
  });

  it("shows elapsed time while a scan is running", async () => {
    renderIndicator({ state: "running", startedAt: "2026-01-01T00:00:00Z", elapsedMs: 3000 });

    expect(await screen.findByText("Scanning… (3s)")).toBeInTheDocument();
  });

  it("shows the last completed scan time when idle with a prior summary", async () => {
    renderIndicator({
      state: "idle",
      lastSummary: { lastScanCompletedAt: "2026-01-01T12:00:00Z" },
    });

    expect(await screen.findByText(/Last scan:/)).toBeInTheDocument();
  });
});
