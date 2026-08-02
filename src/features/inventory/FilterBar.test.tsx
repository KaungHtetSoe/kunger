import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { FilterBar } from "./FilterBar";
import { EMPTY_FILTERS } from "./filterTypes";

describe("FilterBar", () => {
  it("does not show a reset button when no filters are active", () => {
    render(<FilterBar filters={EMPTY_FILTERS} onChange={vi.fn()} />);

    expect(screen.queryByRole("button", { name: "Reset filters" })).not.toBeInTheDocument();
  });

  it("toggles a package manager filter on click", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<FilterBar filters={EMPTY_FILTERS} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "APT" }));

    expect(onChange).toHaveBeenCalledWith({ ...EMPTY_FILTERS, packageManagers: ["apt"] });
  });

  it("resets all filters when Reset is clicked", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    const activeFilters = { ...EMPTY_FILTERS, updateAvailableOnly: true };
    render(<FilterBar filters={activeFilters} onChange={onChange} />);

    await user.click(screen.getByRole("button", { name: "Reset filters" }));

    expect(onChange).toHaveBeenCalledWith(EMPTY_FILTERS);
  });

  it("sets a minimum confidence filter from the select", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<FilterBar filters={EMPTY_FILTERS} onChange={onChange} />);

    await user.selectOptions(screen.getByLabelText(/min\. confidence/i), "high");

    expect(onChange).toHaveBeenCalledWith({ ...EMPTY_FILTERS, minConfidence: "high" });
  });
});
