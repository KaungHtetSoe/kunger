import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { DuplicatesPage } from "./DuplicatesPage";
import type { DuplicateGroup } from "@/types/domain";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

function renderPage(result: DuplicateGroup[] | "error") {
  invokeMock.mockImplementation((command: string) => {
    if (command === "list_duplicate_groups") {
      return result === "error"
        ? Promise.reject(new Error("cache unavailable"))
        : Promise.resolve(result);
    }
    return Promise.resolve(null);
  });

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <DuplicatesPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("DuplicatesPage", () => {
  it("shows the loading state before groups resolve", () => {
    renderPage([]);

    expect(screen.getByText("Loading duplicate groups…")).toBeInTheDocument();
  });

  it("shows an empty state when there are no duplicate groups", async () => {
    renderPage([]);

    expect(await screen.findByText("No likely duplicates found")).toBeInTheDocument();
  });

  it("shows the error state on failure", async () => {
    renderPage("error");

    expect(await screen.findByText("Something went wrong")).toBeInTheDocument();
    expect(screen.getByText("cache unavailable")).toBeInTheDocument();
  });

  it("lists each group's confidence, reason, and member links", async () => {
    renderPage([
      {
        id: "dup:1",
        itemIds: ["apt:firefox", "flatpak:firefox"],
        reason: "same normalized name, different package manager",
        confidence: "high",
      },
    ]);

    expect(await screen.findByText("high confidence")).toBeInTheDocument();
    expect(screen.getByText(/same normalized name/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "apt:firefox" })).toHaveAttribute(
      "href",
      expect.stringContaining("apt%3Afirefox"),
    );
    expect(screen.getByRole("link", { name: "flatpak:firefox" })).toBeInTheDocument();
  });
});
