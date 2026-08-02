import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ScanControls } from "./ScanControls";
import { NotificationProvider } from "@/components/NotificationProvider";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

function renderControls(scanStatus: unknown) {
  invokeMock.mockImplementation((command: string) => {
    if (command === "get_scan_status") return Promise.resolve(scanStatus);
    if (command === "start_inventory_scan") return Promise.resolve(null);
    if (command === "cancel_inventory_scan") return Promise.resolve(null);
    return Promise.resolve(null);
  });

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <NotificationProvider>
        <ScanControls />
      </NotificationProvider>
    </QueryClientProvider>,
  );
}

describe("ScanControls", () => {
  it("shows a Scan System button when idle and starts a scan on click", async () => {
    const user = userEvent.setup();
    renderControls({ state: "idle", lastSummary: null });

    const button = await screen.findByRole("button", { name: /scan system/i });
    await user.click(button);

    expect(invokeMock).toHaveBeenCalledWith(
      "start_inventory_scan",
      expect.objectContaining({ request: expect.anything() }),
    );
  });

  it("shows progress and a Cancel button while a scan is running", async () => {
    renderControls({ state: "running", startedAt: "2026-01-01T00:00:00Z", elapsedMs: 4200 });

    expect(await screen.findByText("Scanning system…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeInTheDocument();
  });

  it("cancels a running scan on click", async () => {
    const user = userEvent.setup();
    renderControls({ state: "running", startedAt: "2026-01-01T00:00:00Z", elapsedMs: 4200 });

    await user.click(await screen.findByRole("button", { name: "Cancel" }));

    expect(invokeMock).toHaveBeenCalledWith("cancel_inventory_scan");
  });
});
