// Shared display formatter for the per-entry thread timestamp.
//
// The backend now sends each thread entry's `created` as an RFC3339 string
// (`YYYY-MM-DDThh:mm:ssZ`); the client portal and the staff ticket view both
// render it via this helper so the format stays consistent across realms.
//
// @implements FS-010 / BS-021: thread entries carry a per-entry created date.

/**
 * Format an RFC3339 timestamp for display as a locale date-time.
 *  - empty / null / undefined → "" (nothing to show).
 *  - unparseable value → returned unchanged (never fabricate).
 *  - valid value → `Date#toLocaleString` (viewer's locale + timezone).
 */
export function formatTimestamp(created: string | null | undefined): string {
  if (!created) {
    return "";
  }
  const date = new Date(created);
  if (Number.isNaN(date.getTime())) {
    return created;
  }
  return date.toLocaleString();
}
