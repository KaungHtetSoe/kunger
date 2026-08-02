import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { SoftwareDetailsPage } from "./SoftwareDetailsPage";
import type { DuplicateGroup, SoftwareItem } from "@/types/domain";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

function baseItem(overrides: Partial<SoftwareItem>): SoftwareItem {
  return {
    id: "apt:firefox",
    packageName: "firefox",
    displayName: "Firefox",
    description: "A web browser",
    version: "128.0",
    architecture: "amd64",
    category: "application",
    secondaryCategories: [],
    packageManager: "apt",
    packageSource: "jammy/main",
    scope: "system",
    installPaths: ["/usr/lib/firefox"],
    executablePaths: ["/usr/bin/firefox"],
    desktopFilePaths: ["/usr/share/applications/firefox.desktop"],
    iconPath: null,
    packageSection: "web",
    installedSizeBytes: 250 * 1024 * 1024,
    installedAt: "2026-01-01T00:00:00Z",
    installationReason: "manual",
    dependencies: ["libc6"],
    reverseDependencies: [],
    updateAvailable: true,
    availableVersion: "129.0",
    repository: "jammy",
    homepage: "https://firefox.com",
    license: "MPL-2.0",
    classificationConfidence: "certain",
    classificationReasons: ["matched known desktop application package"],
    riskLevel: "low",
    metadata: { Maintainer: "Mozilla" },
    warnings: ["Could not resolve icon theme"],
    ...overrides,
  };
}

function renderPage(item: SoftwareItem | null, duplicateGroups: DuplicateGroup[] = []) {
  invokeMock.mockImplementation((command: string) => {
    if (command === "get_software_item") return Promise.resolve(item);
    if (command === "list_duplicate_groups") return Promise.resolve(duplicateGroups);
    return Promise.resolve(null);
  });

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });

  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={["/software/apt%3Afirefox"]}>
        <Routes>
          <Route path="/software/:id" element={<SoftwareDetailsPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("SoftwareDetailsPage", () => {
  it("renders full item details across all sections", async () => {
    renderPage(baseItem({}));

    expect(await screen.findByRole("heading", { name: "Firefox" })).toBeInTheDocument();
    expect(screen.getByText("A web browser")).toBeInTheDocument();
    expect(screen.getByText("128.0")).toBeInTheDocument();
    expect(screen.getByText("250.0 MB")).toBeInTheDocument();
    expect(screen.getByText("certain confidence")).toBeInTheDocument();
    expect(screen.getByText("low risk")).toBeInTheDocument();
    expect(screen.getByText("matched known desktop application package")).toBeInTheDocument();
    expect(screen.getByText("/usr/lib/firefox")).toBeInTheDocument();
    expect(screen.getByText("libc6")).toBeInTheDocument();
    expect(screen.getByText("Mozilla")).toBeInTheDocument();
    expect(screen.getByText("Could not resolve icon theme")).toBeInTheDocument();
  });

  it("shows Not available for null fields instead of leaving them blank", async () => {
    renderPage(baseItem({ architecture: null, repository: null }));

    await screen.findByRole("heading", { name: "Firefox" });
    expect(screen.getAllByText("Not available").length).toBeGreaterThanOrEqual(2);
  });

  it("shows an empty state when the item is not found", async () => {
    renderPage(null);

    expect(await screen.findByText("Item not found")).toBeInTheDocument();
  });

  it("shows a duplicate banner when the item is part of a duplicate group", async () => {
    renderPage(baseItem({}), [
      {
        id: "dup:1",
        itemIds: ["apt:firefox", "flatpak:firefox"],
        reason: "same normalized name",
        confidence: "high",
      },
    ]);

    expect(await screen.findByText(/same normalized name/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "flatpak:firefox" })).toBeInTheDocument();
  });
});
