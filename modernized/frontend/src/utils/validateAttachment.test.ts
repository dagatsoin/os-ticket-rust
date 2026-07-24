import { describe, expect, it } from "vitest";
import {
  ALLOWED_EXTENSIONS,
  MAX_FILE_SIZE_BYTES,
  validateAttachment,
} from "./validateAttachment";

/** Build a File with a chosen byte size without allocating the bytes wastefully. */
function fileOf(name: string, size: number, type = "application/octet-stream"): File {
  const file = new File(["x"], name, { type });
  Object.defineProperty(file, "size", { value: size });
  return file;
}

describe("validateAttachment (TS-M2-A4 shared pre-check)", () => {
  it("accepts each seeded allowed extension under the cap", () => {
    for (const ext of ALLOWED_EXTENSIONS) {
      expect(validateAttachment(fileOf(`doc${ext}`, 1000))).toBeUndefined();
    }
  });

  it("is case-insensitive on the extension", () => {
    expect(validateAttachment(fileOf("INVOICE.PDF", 1000))).toBeUndefined();
  });

  it("rejects a disallowed type with an inline message", () => {
    const msg = validateAttachment(fileOf("evil.exe", 1000));
    expect(msg).toMatch(/invalid file type/i);
  });

  it("rejects an extension-less file", () => {
    expect(validateAttachment(fileOf("README", 1000))).toMatch(/invalid file type/i);
  });

  it("accepts a permitted file exactly at the cap", () => {
    expect(validateAttachment(fileOf("big.pdf", MAX_FILE_SIZE_BYTES))).toBeUndefined();
  });

  it("rejects a permitted type over the cap as too big", () => {
    const msg = validateAttachment(fileOf("big.pdf", MAX_FILE_SIZE_BYTES + 1));
    expect(msg).toMatch(/too big/i);
  });
});
