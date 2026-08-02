import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn((command: string) => {
    if (command === "get_scan_status") {
      return Promise.resolve({ state: "idle", lastSummary: null });
    }
    if (command === "get_provider_status") {
      return Promise.resolve([]);
    }
    return Promise.resolve(null);
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

describe("App", () => {
  it("renders the shell with the Dashboard as the default route", async () => {
    render(<App />);

    expect(screen.getByText("Kunger")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Primary" })).toBeInTheDocument();
    expect(await screen.findByText("No scan has been run yet")).toBeInTheDocument();
  });

  it("navigates to the Providers page", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("link", { name: /providers/i }));

    expect(await screen.findByRole("heading", { name: "Providers" })).toBeInTheDocument();
  });
});
