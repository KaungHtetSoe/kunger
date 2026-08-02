import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { NotFoundPage } from "./NotFoundPage";

describe("NotFoundPage", () => {
  it("renders a not-found message", () => {
    render(<NotFoundPage />);

    expect(screen.getByText("Page not found")).toBeInTheDocument();
  });
});
