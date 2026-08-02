import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { ExportPage } from "./ExportPage";
import { NotificationProvider } from "@/components/NotificationProvider";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

const manifestContent = JSON.stringify({
  schemaVersion: 1,
  exportedAt: "2026-01-01T00:00:00Z",
  reproducible: [
    {
      packageManager: "apt",
      installHint: "sudo apt install <package names>",
      packages: [{ packageName: "ripgrep", displayName: "ripgrep", version: "14.1.0" }],
    },
  ],
  manualReview: [],
});

function renderPage() {
  invokeMock.mockImplementation((command: string, args: unknown) => {
    if (command === "get_inventory_summary") {
      return Promise.resolve({
        status: "completed",
        totalItems: 42,
        itemsByCategory: {},
        itemsByPackageManager: {},
        providersWithWarnings: [],
        providersWithErrors: [],
        duplicateGroupCount: 0,
        lastScanStartedAt: null,
        lastScanCompletedAt: null,
        scanDurationMs: null,
      });
    }
    if (command === "export_inventory") {
      const request = (args as { request: { format: string; mode: string } }).request;
      if (request.mode === "reinstallationManifest") {
        return Promise.resolve({ schemaVersion: 1, format: "json", content: manifestContent });
      }
      return Promise.resolve({
        schemaVersion: 1,
        format: request.format,
        content: "exported-content",
      });
    }
    return Promise.resolve(null);
  });

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <NotificationProvider>
        <MemoryRouter>
          <ExportPage />
        </MemoryRouter>
      </NotificationProvider>
    </QueryClientProvider>,
  );
}

describe("ExportPage", () => {
  it("shows the full-inventory item count by default", async () => {
    renderPage();

    expect(await screen.findByText(/Will export 42 items/)).toBeInTheDocument();
  });

  it("shows the manifest preview when switching to reinstallation manifest mode", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole("button", { name: "Reinstallation manifest" }));

    expect(await screen.findByText("Can reinstall automatically (1)")).toBeInTheDocument();
    expect(screen.getByText("ripgrep")).toBeInTheDocument();
  });

  it("always shows the privacy notice about installation paths", () => {
    renderPage();

    expect(
      screen.getByText(/contain your home directory and therefore your username/i),
    ).toBeInTheDocument();
  });

  it("downloads the export and shows a success notification on click", async () => {
    const user = userEvent.setup();
    const clickSpy = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
    URL.createObjectURL = vi.fn(() => "blob:mock");
    URL.revokeObjectURL = vi.fn();

    renderPage();

    await user.click(screen.getByRole("button", { name: "Download export" }));

    await waitFor(() => expect(clickSpy).toHaveBeenCalledTimes(1));
    expect(await screen.findByText(/Export saved/)).toBeInTheDocument();

    clickSpy.mockRestore();
  });
});
