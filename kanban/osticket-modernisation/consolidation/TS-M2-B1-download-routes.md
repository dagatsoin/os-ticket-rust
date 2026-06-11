# TS-M2-B1 — add(route): FS-022.10 client + staff attachment download routes

- **ID**: TS-M2-B1
- **Type**: Technical Story
- **Parent**: US-M2-3
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Streams a stored blob back to an authorized requester. **DEVIATION D2 (pinned):** authorization is a
**session check on the parent ticket** (the requester's realm session must be able to access the
attachment's parent ticket), preserving BS-022.8's observable behaviour without the legacy
session-bound MD5 hash (KL-022.5 obsoleted).

## Impact

- add(route): `GET /api/tickets/{ticketId}/attachments/{attachmentId}` (client realm) — resolves `ticket_attachment` → `attachment_file`, checks the client session owns the parent ticket, streams the blob.
- add(route): `GET /api/staff/tickets/{ticketId}/attachments/{attachmentId}` (staff realm) — staff-session variant.
- update(http): both set `Content-Disposition: attachment; filename="<name>"` and the stored MIME (octet-stream fallback), stream from `BlobStore.open` (FS-022.11).
- Mismatched/unknown id, or an attachment whose parent ticket the session can't access, returns 404/403 with no bytes.

## Regressions

- New read-only routes; touch no write path.

## Acceptance Tests

### AC-1: FS-022.10/.11 — an authorized client downloads the blob with a Content-Disposition filename. [API-ONLY]
- Client logged into the parent ticket → GET the attachment → 200, correct bytes, `Content-Disposition` filename, correct MIME.
- Status: [ ]

### AC-2: BS-022.8/D2/EC-022.9 — a client logged into a DIFFERENT ticket is denied. [API-ONLY]
- Client session for ticket B requests ticket A's attachment → 403/404, no bytes.
- Status: [ ]

### AC-3: EC-022.7 — an unknown/mismatched attachment id is rejected. [API-ONLY]
- GET a non-existent attachment id, and an id belonging to another ticket → 404/403, shared error envelope.
- Status: [ ]

### AC-4: staff download — an authorized staff session downloads via the staff route. [API-ONLY]
- Staff login → GET the staff attachment route → 200, bytes, Content-Disposition.
- Status: [ ]

## Dependencies

- TS-M2-A1 (blob store + ticket_attachment), TS-M1-C1/D1 (staff/client session gates).
