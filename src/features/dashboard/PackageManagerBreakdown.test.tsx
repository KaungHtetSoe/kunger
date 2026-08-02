import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { PackageManagerBreakdown } from "./PackageManagerBreakdown";

describe("PackageManagerBreakdown", () => {
  it("renders a row per non-zero package manager, sorted by count descending", () => {
    render(<PackageManagerBreakdown itemsByPackageManager={{ apt: 500, flatpak: 20, snap: 0 }} />);

    const rows = screen.getAllByText(/APT|Flatpak|Snap/);
    expect(rows.map((el) => el.textContent)).toEqual(["APT", "Flatpak"]);
  });

  it("shows a fallback message when there is no data", () => {
    render(<PackageManagerBreakdown itemsByPackageManager={{}} />);

    expect(screen.getByText("No package manager data.")).toBeInTheDocument();
  });

  it("displays the formatted count for each manager", () => {
    render(<PackageManagerBreakdown itemsByPackageManager={{ apt: 1284 }} />);

    expect(screen.getByText("1,284")).toBeInTheDocument();
  });
});
