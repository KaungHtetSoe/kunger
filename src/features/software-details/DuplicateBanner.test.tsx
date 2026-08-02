import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { DuplicateBanner } from "./DuplicateBanner";
import type { DuplicateGroup } from "@/types/domain";

function renderWithRouter(ui: React.ReactElement) {
  return render(<MemoryRouter>{ui}</MemoryRouter>);
}

describe("DuplicateBanner", () => {
  it("renders nothing when the item isn't part of any duplicate group", () => {
    const groups: DuplicateGroup[] = [
      {
        id: "dup:1",
        itemIds: ["apt:a", "flatpak:a"],
        reason: "same normalized name",
        confidence: "high",
      },
    ];

    const { container } = renderWithRouter(<DuplicateBanner itemId="apt:b" groups={groups} />);

    expect(container).toBeEmptyDOMElement();
  });

  it("shows the reason and links to the other installations when a match is found", () => {
    const groups: DuplicateGroup[] = [
      {
        id: "dup:1",
        itemIds: ["apt:a", "flatpak:a"],
        reason: "same normalized name",
        confidence: "high",
      },
    ];

    renderWithRouter(<DuplicateBanner itemId="apt:a" groups={groups} />);

    expect(screen.getByText(/same normalized name/)).toBeInTheDocument();
    expect(screen.getByText(/high/)).toBeInTheDocument();
    const link = screen.getByRole("link", { name: "flatpak:a" });
    expect(link).toHaveAttribute("href", expect.stringContaining("flatpak%3Aa"));
    expect(screen.queryByRole("link", { name: "apt:a" })).not.toBeInTheDocument();
  });
});
