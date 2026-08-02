import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ProvidersPage } from "./ProvidersPage";
import type { ProviderStatusResponse } from "@/types/commands";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

function renderPage(result: ProviderStatusResponse[] | "error") {
  invokeMock.mockImplementation((command: string) => {
    if (command === "get_provider_status") {
      return result === "error"
        ? Promise.reject(new Error("provider probe failed"))
        : Promise.resolve(result);
    }
    return Promise.resolve(null);
  });

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <ProvidersPage />
    </QueryClientProvider>,
  );
}

describe("ProvidersPage", () => {
  it("shows the loading state before providers resolve", () => {
    renderPage([]);

    expect(screen.getByText("Checking provider availability…")).toBeInTheDocument();
  });

  it("shows the error state on failure", async () => {
    renderPage("error");

    expect(await screen.findByText("Something went wrong")).toBeInTheDocument();
    expect(screen.getByText("provider probe failed")).toBeInTheDocument();
  });

  it("marks available and unavailable providers distinctly", async () => {
    renderPage([
      {
        id: "apt",
        displayName: "APT/dpkg",
        description: "Debian package manager",
        available: true,
      },
      { id: "flatpak", displayName: "Flatpak", description: "Sandboxed apps", available: false },
    ]);

    expect(await screen.findByText("APT/dpkg")).toBeInTheDocument();
    expect(screen.getByText("Available")).toBeInTheDocument();
    expect(screen.getByText("Not available on this system")).toBeInTheDocument();
  });
});
