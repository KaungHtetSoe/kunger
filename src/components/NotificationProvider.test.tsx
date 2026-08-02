import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { NotificationProvider } from "./NotificationProvider";
import { useNotifications } from "./notificationContext";

function TestTrigger() {
  const { notify } = useNotifications();
  return (
    <button type="button" onClick={() => notify("success", "Scan completed")}>
      Trigger
    </button>
  );
}

describe("NotificationProvider", () => {
  it("shows a notification after notify() is called and dismisses it on click", async () => {
    const user = userEvent.setup();
    render(
      <NotificationProvider>
        <TestTrigger />
      </NotificationProvider>,
    );

    await user.click(screen.getByRole("button", { name: "Trigger" }));

    expect(await screen.findByText("Scan completed")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Dismiss notification" }));

    expect(screen.queryByText("Scan completed")).not.toBeInTheDocument();
  });

  it("throws a clear error when useNotifications is used outside the provider", () => {
    function Broken() {
      useNotifications();
      return null;
    }

    expect(() => render(<Broken />)).toThrow(
      /useNotifications must be used within a NotificationProvider/,
    );
  });
});
