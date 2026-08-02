import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { InventoryCategoryPage } from "./InventoryCategoryPage";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

function renderAt(path: string) {
  invokeMock.mockImplementation((command: string) => {
    if (command === "list_software_items") {
      return Promise.resolve({ items: [], totalCount: 0, page: 1, pageSize: 50 });
    }
    return Promise.resolve(null);
  });

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="/inventory" element={<InventoryCategoryPage />} />
          <Route path="/inventory/:category" element={<InventoryCategoryPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("InventoryCategoryPage", () => {
  it("shows 'All Software' as the title with no category param", () => {
    renderAt("/inventory");

    expect(screen.getByRole("heading", { name: "All Software" })).toBeInTheDocument();
  });

  it("shows the matching nav item's label as the title for a known category", () => {
    renderAt("/inventory/application");

    expect(screen.getByRole("heading", { name: "Applications" })).toBeInTheDocument();
  });

  it("pre-populates the search box from the ?q= query param", () => {
    renderAt("/inventory?q=firefox");

    expect(screen.getByLabelText(/filter this view/i)).toHaveValue("firefox");
  });
});
