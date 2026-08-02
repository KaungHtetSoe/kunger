import { describe, expect, it } from "vitest";
import { formatCount, formatPackageManager, formatSoftwareCategory } from "./labels";

describe("formatPackageManager", () => {
  it("maps known package managers to human-readable labels", () => {
    expect(formatPackageManager("apt")).toBe("APT");
    expect(formatPackageManager("appImage")).toBe("AppImage");
  });
});

describe("formatSoftwareCategory", () => {
  it("maps known categories to human-readable labels", () => {
    expect(formatSoftwareCategory("commandLineTool")).toBe("Command-line Tool");
    expect(formatSoftwareCategory("developmentPackage")).toBe("Development Package");
  });
});

describe("formatCount", () => {
  it("uses locale grouping under 10,000", () => {
    expect(formatCount(1284)).toBe("1,284");
  });

  it("compacts to K above 10,000", () => {
    expect(formatCount(12_900)).toBe("12.9K");
  });

  it("compacts to M above 1,000,000", () => {
    expect(formatCount(4_200_000)).toBe("4.2M");
  });
});
