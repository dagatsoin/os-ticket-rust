import { describe, expect, it } from "vitest";
import { formatTimestamp } from "./formatTimestamp";

describe("formatTimestamp (per-entry thread timestamp)", () => {
  it("renders a valid RFC3339 timestamp as a locale date-time string", () => {
    const out = formatTimestamp("2026-07-23T14:05:09Z");
    expect(out).not.toBe("");
    // The exact locale format is environment-dependent, but a real render
    // must reflect the date (year 2026) rather than echo the raw input.
    expect(out).not.toBe("2026-07-23T14:05:09Z");
    expect(out).toContain("2026");
  });

  it("returns an empty string for an empty/missing value", () => {
    expect(formatTimestamp("")).toBe("");
    expect(formatTimestamp(undefined)).toBe("");
    expect(formatTimestamp(null)).toBe("");
  });

  it("returns the raw value unchanged when it is not a parseable date", () => {
    expect(formatTimestamp("not-a-date")).toBe("not-a-date");
  });
});
