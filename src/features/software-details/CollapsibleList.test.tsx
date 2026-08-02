import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CollapsibleList } from "./CollapsibleList";

describe("CollapsibleList", () => {
  it("shows the empty label when there are no items", () => {
    render(<CollapsibleList items={[]} emptyLabel="No dependencies" />);

    expect(screen.getByText("No dependencies")).toBeInTheDocument();
  });

  it("shows all items without a toggle when under the initial count", () => {
    render(<CollapsibleList items={["a", "b", "c"]} initialCount={8} />);

    expect(screen.getByText("a")).toBeInTheDocument();
    expect(screen.getByText("c")).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("collapses long lists behind a Show more toggle", async () => {
    const user = userEvent.setup();
    const items = Array.from({ length: 10 }, (_, i) => `item-${i}`);
    render(<CollapsibleList items={items} initialCount={4} />);

    expect(screen.getByText("item-3")).toBeInTheDocument();
    expect(screen.queryByText("item-4")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Show 6 more" }));

    expect(screen.getByText("item-9")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Show less" }));

    expect(screen.queryByText("item-9")).not.toBeInTheDocument();
  });
});
