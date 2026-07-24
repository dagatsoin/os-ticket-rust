// Authorized attachment download (TS-M2-B2). Chips are NOT plain anchors:
// clicking fetches WITH CREDENTIALS → blob → object-URL → a synthetic anchor
// click, so a 403/404 surfaces a VISIBLE inline error instead of navigating
// away (ROADMAP M2 Decisions §9). Realm-aware URL building (§8).
//
// @implements FS-022.10: authorized attachment download via session-bound routes.
// @implements EC-022.9: cross-ticket / unknown id → 404, surfaced as an inline error.
import type { Realm } from "../api/types";

/** The attachment list shape carried on every thread entry / canned detail (§7). */
export interface Attachment {
  id: number;
  name: string;
  size: number;
  mime: string;
}

/**
 * Build the realm-aware download URL for an attachment (§8).
 *  - client: session-bound, NO ticketId (the session already pins the ticket).
 *  - staff:  scoped under the ticket.
 * `ticketId` is REQUIRED for staff and ignored for client.
 */
export function attachmentDownloadUrl(
  realm: Realm,
  attachmentId: number,
  ticketId?: number,
): string {
  if (realm === "client") {
    return `/api/client/ticket/attachments/${attachmentId}`;
  }
  if (ticketId === undefined) {
    throw new Error("attachmentDownloadUrl: staff downloads require a ticketId");
  }
  return `/api/staff/tickets/${ticketId}/attachments/${attachmentId}`;
}

/** Outcome of a download attempt: either the file streamed, or a visible error. */
export type DownloadResult = { ok: true } | { ok: false; error: string };

/** Map a failed download response/exception to a visible inline message. */
export function downloadErrorMessage(status: number | null): string {
  if (status === 404 || status === 403) {
    return "This attachment is unavailable or you are not authorized to download it.";
  }
  if (status === null) {
    return "Could not download the attachment. Please try again.";
  }
  return `Could not download the attachment (error ${status}).`;
}

/** Injectable browser hooks so the fetch→blob→objectURL flow is unit-testable. */
export interface DownloadDeps {
  fetch: typeof fetch;
  createObjectURL: (blob: Blob) => string;
  revokeObjectURL: (url: string) => void;
  /** Trigger the actual save (default: synthetic anchor click). */
  triggerSave: (url: string, filename: string) => void;
}

function defaultTriggerSave(url: string, filename: string): void {
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.rel = "noopener";
  document.body.appendChild(a);
  a.click();
  a.remove();
}

function defaultDeps(): DownloadDeps {
  return {
    fetch: (...args) => fetch(...args),
    createObjectURL: (blob) => URL.createObjectURL(blob),
    revokeObjectURL: (url) => URL.revokeObjectURL(url),
    triggerSave: defaultTriggerSave,
  };
}

/**
 * Fetch an attachment with credentials, then trigger a browser download from a
 * blob object-URL. Returns a {@link DownloadResult}; the caller surfaces the
 * error inline near the chip (§9). Never throws.
 */
export async function downloadAttachment(
  realm: Realm,
  attachment: Attachment,
  ticketId: number | undefined,
  deps: Partial<DownloadDeps> = {},
): Promise<DownloadResult> {
  const d = { ...defaultDeps(), ...deps };
  const url = attachmentDownloadUrl(realm, attachment.id, ticketId);

  let res: Response;
  try {
    res = await d.fetch(url, { credentials: "include" });
  } catch {
    return { ok: false, error: downloadErrorMessage(null) };
  }

  if (!res.ok) {
    return { ok: false, error: downloadErrorMessage(res.status) };
  }

  const blob = await res.blob();
  const objectUrl = d.createObjectURL(blob);
  try {
    d.triggerSave(objectUrl, attachment.name);
  } finally {
    // Revoke on the next tick so the browser has started the download.
    setTimeout(() => d.revokeObjectURL(objectUrl), 0);
  }
  return { ok: true };
}
