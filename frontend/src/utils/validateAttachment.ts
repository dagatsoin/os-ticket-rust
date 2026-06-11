// Shared client-side attachment pre-check (TS-M2-A4 — OWNED here; reused by TS-M2-D3).
//
// A UX convenience that mirrors the SEEDED backend config (allow-list + 1 MB cap)
// so an obviously-bad file is rejected before a round trip. ROADMAP M2 Decisions §11:
// the backend (A2 + A3) remains AUTHORITATIVE — this is a pre-flight only and may
// drift from the live config until M4 exposes it. Hard-coded by design.
//
// @implements FS-011.7: /open attachment client-side type + size pre-check.
// @implements KL: front-end allow-list/cap is a hard-coded pre-check; backend authoritative.

/** Seeded allow-list (TS-M2-A2: `allowed_filetypes` = `.pdf,.png,.jpg,.txt,.doc`). */
export const ALLOWED_EXTENSIONS = [".pdf", ".png", ".jpg", ".txt", ".doc"] as const;

/** Seeded cap (TS-M2-A2: `max_file_size` = 1 MB). */
export const MAX_FILE_SIZE_BYTES = 1024 * 1024;

/** Human-readable allow-list, for helper text under the file input. */
export const ALLOWED_EXTENSIONS_LABEL = ALLOWED_EXTENSIONS.join(", ");

/** Human-readable cap, for helper text. */
export const MAX_FILE_SIZE_LABEL = "1 MB";

/** The lower-cased extension of a filename including the leading dot, or "". */
function extensionOf(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot === -1 ? "" : name.slice(dot).toLowerCase();
}

/**
 * Pre-check a locally-selected file against the seeded allow-list + size cap.
 * Returns an inline error message string, or `undefined` when the file passes.
 * The backend re-validates and remains authoritative (§11).
 */
export function validateAttachment(file: File): string | undefined {
  const ext = extensionOf(file.name);
  if (!ALLOWED_EXTENSIONS.includes(ext as (typeof ALLOWED_EXTENSIONS)[number])) {
    return `Invalid file type. Allowed: ${ALLOWED_EXTENSIONS_LABEL}.`;
  }
  if (file.size > MAX_FILE_SIZE_BYTES) {
    return `File is too big (max ${MAX_FILE_SIZE_LABEL}).`;
  }
  return undefined;
}
