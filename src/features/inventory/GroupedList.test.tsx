import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { GroupedList } from "./GroupedList";
import type { SoftwareItem } from "@/types/domain";

function item(overrides: Partial<SoftwareItem>): SoftwareItem {
  return {
    id: "apt:example",
    packageName: "example",
    displayName: "Example",
    description: null,
    version: null,
    architecture: null,
    category: "application",
    secondaryCategories: [],
    packageManager: "apt",
    packageSource: null,
    scope: "system",
    installPaths: [],
    executablePaths: [],
    desktopFilePaths: [],
    iconPath: null,
    packageSection: null,
    installedSizeBytes: null,
    installedAt: null,
    installationReason: "manual",
    dependencies: [],
    reverseDependencies: [],
    updateAvailable: false,
    availableVersion: null,
    repository: null,
    homepage: null,
    license: null,
    classificationConfidence: "high",
    classificationReasons: [],
    riskLevel: "unknown",
    metadata: {},
    warnings: [],
    ...overrides,
  };
}

function renderWithRouter(ui: React.ReactElement) {
  return render(<MemoryRouter>{ui}</MemoryRouter>);
}

describe("GroupedList", () => {
  it("groups items by category and shows counts, largest group first", () => {
    const items = [
      item({ id: "apt:a", displayName: "A", category: "application" }),
      item({ id: "apt:b", displayName: "B", category: "library" }),
      item({ id: "apt:c", displayName: "C", category: "library" }),
    ];

    renderWithRouter(<GroupedList items={items} groupBy="groupedByCategory" />);

    const headings = screen.getAllByRole("heading", { level: 3 }).map((h) => h.textContent);
    expect(headings).toEqual(["Library", "Application"]);
    expect(screen.getByText("2")).toBeInTheDocument();
  });

  it("groups items by package manager", () => {
    const items = [
      item({ id: "apt:a", displayName: "A", packageManager: "apt" }),
      item({ id: "flatpak:b", displayName: "B", packageManager: "flatpak" }),
    ];

    renderWithRouter(<GroupedList items={items} groupBy="groupedByManager" />);

    expect(screen.getByRole("heading", { name: "APT" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Flatpak" })).toBeInTheDocument();
  });
});
