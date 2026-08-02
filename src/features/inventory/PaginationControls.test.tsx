import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PaginationControls } from "./PaginationControls";

describe("PaginationControls", () => {
  it("shows the current range and total", () => {
    render(<PaginationControls page={2} pageSize={50} totalCount={120} onPageChange={vi.fn()} />);

    expect(screen.getByText("51–100 of 120")).toBeInTheDocument();
    expect(screen.getByText("Page 2 of 3")).toBeInTheDocument();
  });

  it("disables Previous on the first page and Next on the last page", () => {
    render(<PaginationControls page={1} pageSize={50} totalCount={10} onPageChange={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Previous page" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Next page" })).toBeDisabled();
  });

  it("calls onPageChange with the next page number", async () => {
    const user = userEvent.setup();
    const onPageChange = vi.fn();
    render(
      <PaginationControls page={1} pageSize={50} totalCount={120} onPageChange={onPageChange} />,
    );

    await user.click(screen.getByRole("button", { name: "Next page" }));

    expect(onPageChange).toHaveBeenCalledWith(2);
  });

  it("shows a no-items message when totalCount is zero", () => {
    render(<PaginationControls page={1} pageSize={50} totalCount={0} onPageChange={vi.fn()} />);

    expect(screen.getByText("No items")).toBeInTheDocument();
  });
});
