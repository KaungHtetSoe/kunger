import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

describe("App", () => {
  it("renders the Kunger shell heading and tagline", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "Kunger" })).toBeInTheDocument();
    expect(screen.getByText("Linux Software Inventory Manager")).toBeInTheDocument();
  });
});
