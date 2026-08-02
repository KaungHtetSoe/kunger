import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { GlobalSearch } from "./GlobalSearch";

function LocationProbe({ onChange }: { onChange: (value: string) => void }) {
  const location = useLocation();
  onChange(location.pathname + location.search);
  return null;
}

function renderWithRouter() {
  let location = "";
  render(
    <MemoryRouter initialEntries={["/dashboard"]}>
      <Routes>
        <Route path="*" element={<GlobalSearch />} />
      </Routes>
      <LocationProbe onChange={(pathname) => (location = pathname)} />
    </MemoryRouter>,
  );
  return {
    getLocation: () => location,
  };
}

describe("GlobalSearch", () => {
  it("navigates to /inventory with the query on submit", async () => {
    const user = userEvent.setup();
    const { getLocation } = renderWithRouter();

    await user.type(screen.getByLabelText("Search installed software"), "firefox");
    await user.keyboard("{Enter}");

    expect(getLocation()).toBe("/inventory?q=firefox");
  });

  it("trims whitespace and navigates to plain /inventory when the query is blank", async () => {
    const user = userEvent.setup();
    const { getLocation } = renderWithRouter();

    await user.type(screen.getByLabelText("Search installed software"), "   ");
    await user.keyboard("{Enter}");

    expect(getLocation()).toBe("/inventory");
  });

  it("URL-encodes special characters in the query", async () => {
    const user = userEvent.setup();
    const { getLocation } = renderWithRouter();

    await user.type(screen.getByLabelText("Search installed software"), "c++ tools");
    await user.keyboard("{Enter}");

    expect(getLocation()).toBe("/inventory?q=c%2B%2B%20tools");
  });
});
