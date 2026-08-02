import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { ManifestPreview } from "./ManifestPreview";
import type { ReinstallManifest } from "@/types/commands";

function manifest(overrides: Partial<ReinstallManifest> = {}): ReinstallManifest {
  return {
    schemaVersion: 1,
    exportedAt: "2026-01-01T00:00:00Z",
    reproducible: [
      {
        packageManager: "apt",
        installHint: "sudo apt install <package names>",
        packages: [
          { packageName: "ripgrep", displayName: "ripgrep", version: "14.1.0" },
          { packageName: "git", displayName: "Git", version: null },
        ],
      },
    ],
    manualReview: [
      {
        id: "manual:/usr/local/bin/mytool",
        displayName: "mytool",
        packageManager: "manual",
        reason: "Found in a local bin/lib/opt directory with no owning package manager.",
        paths: ["/home/alice/.local/bin/mytool"],
      },
    ],
    ...overrides,
  };
}

describe("ManifestPreview", () => {
  it("shows the reproducible count and package names with install hints", () => {
    render(<ManifestPreview manifest={manifest()} />);

    expect(screen.getByText("Can reinstall automatically (2)")).toBeInTheDocument();
    expect(screen.getByText("sudo apt install <package names>")).toBeInTheDocument();
    expect(screen.getByText("ripgrep")).toBeInTheDocument();
    expect(screen.getByText("git")).toBeInTheDocument();
  });

  it("shows manual review items with their reason and paths", () => {
    render(<ManifestPreview manifest={manifest()} />);

    expect(screen.getByText("Needs manual review (1)")).toBeInTheDocument();
    expect(screen.getByText("mytool")).toBeInTheDocument();
    expect(
      screen.getByText("Found in a local bin/lib/opt directory with no owning package manager."),
    ).toBeInTheDocument();
    expect(screen.getByText("/home/alice/.local/bin/mytool")).toBeInTheDocument();
  });

  it("shows a fallback message when a section is empty", () => {
    render(<ManifestPreview manifest={manifest({ manualReview: [] })} />);

    expect(screen.getByText("Needs manual review (0)")).toBeInTheDocument();
    expect(screen.getAllByText("Nothing in this category.")).toHaveLength(1);
  });
});
