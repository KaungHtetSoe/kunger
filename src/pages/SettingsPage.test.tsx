import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { SettingsPage } from "./SettingsPage";
import { NotificationProvider } from "@/components/NotificationProvider";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

function renderPage(shouldFail = false) {
  invokeMock.mockImplementation((command: string) => {
    if (command === "rebuild_cache") {
      return shouldFail ? Promise.reject(new Error("disk full")) : Promise.resolve(null);
    }
    return Promise.resolve(null);
  });

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <NotificationProvider>
        <SettingsPage />
      </NotificationProvider>
    </QueryClientProvider>,
  );
}

describe("SettingsPage", () => {
  it("requires a confirmation click before rebuilding the cache", async () => {
    const user = userEvent.setup();
    renderPage();

    expect(screen.queryByRole("button", { name: "Confirm rebuild" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Rebuild cache" }));

    expect(screen.getByRole("button", { name: "Confirm rebuild" })).toBeInTheDocument();
  });

  it("cancels back to the initial state without calling rebuild_cache", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole("button", { name: "Rebuild cache" }));
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("button", { name: "Confirm rebuild" })).not.toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith("rebuild_cache", expect.anything());
  });

  it("shows a success notification after confirming the rebuild", async () => {
    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByRole("button", { name: "Rebuild cache" }));
    await user.click(screen.getByRole("button", { name: "Confirm rebuild" }));

    expect(await screen.findByText(/Cache rebuilt/)).toBeInTheDocument();
  });

  it("shows an error notification when the rebuild fails", async () => {
    const user = userEvent.setup();
    renderPage(true);

    await user.click(screen.getByRole("button", { name: "Rebuild cache" }));
    await user.click(screen.getByRole("button", { name: "Confirm rebuild" }));

    expect(await screen.findByText(/disk full/)).toBeInTheDocument();
  });
});
