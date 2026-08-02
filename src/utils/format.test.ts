import { describe, expect, it } from "vitest";
import { formatDuration } from "./format";

describe("formatDuration", () => {
  it("formats sub-second durations in milliseconds", () => {
    expect(formatDuration(450)).toBe("450ms");
  });

  it("formats sub-minute durations in seconds with one decimal", () => {
    expect(formatDuration(12_300)).toBe("12.3s");
  });

  it("formats durations over a minute as minutes and seconds", () => {
    expect(formatDuration(65_000)).toBe("1m 5s");
  });
});
