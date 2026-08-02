import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { InventoryBrowser } from "./InventoryBrowser";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

function emptyResponse() {
  return Promise.resolve({ items: [], totalCount: 0, page: 1, pageSize: 50 });
}

function renderBrowser(props: Partial<React.ComponentProps<typeof InventoryBrowser>> = {}) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <InventoryBrowser title="All Software" {...props} />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

function listSoftwareItemsCalls() {
  return invokeMock.mock.calls.filter(([command]) => command === "list_software_items");
}

describe("InventoryBrowser", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    invokeMock.mockImplementation(() => emptyResponse());
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("debounces the search box instead of querying on every keystroke", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    renderBrowser();

    await act(async () => {
      await Promise.resolve();
    });
    const callsBeforeTyping = listSoftwareItemsCalls().length;

    const input = screen.getByLabelText(/filter this view/i);
    await user.type(input, "fire");

    // Mid-typing: no new query yet, since each keystroke resets the debounce timer.
    expect(listSoftwareItemsCalls().length).toBe(callsBeforeTyping);

    await act(async () => {
      vi.advanceTimersByTime(250);
      await Promise.resolve();
    });

    const searchCalls = listSoftwareItemsCalls().filter(
      ([, request]) => (request as { request: { search?: string } }).request.search === "fire",
    );
    expect(searchCalls.length).toBeGreaterThan(0);
  });

  it("resets to page 1 when the search box changes", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    invokeMock.mockImplementation((command: string) =>
      command === "list_software_items"
        ? Promise.resolve({
            items: Array.from({ length: 1 }, (_, i) => ({
              id: `apt:pkg-${i}`,
              packageName: `pkg-${i}`,
              displayName: `Pkg ${i}`,
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
            })),
            totalCount: 120,
            page: 1,
            pageSize: 50,
          })
        : Promise.resolve(null),
    );
    renderBrowser();

    await act(async () => {
      await Promise.resolve();
    });

    await user.click(await screen.findByRole("button", { name: "Next page" }));

    await act(async () => {
      await Promise.resolve();
    });

    const input = screen.getByLabelText(/filter this view/i);
    await user.type(input, "x");

    await act(async () => {
      vi.advanceTimersByTime(250);
      await Promise.resolve();
    });

    const lastSearchCall = listSoftwareItemsCalls()
      .reverse()
      .find(([, request]) => (request as { request: { search?: string } }).request.search === "x");
    expect((lastSearchCall?.[1] as { request: { page: number } }).request.page).toBe(1);
  });
});
