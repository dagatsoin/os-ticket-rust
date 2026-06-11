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

**Routes (ROADMAP M2 Decisions §8):** the client route is **session-bound with NO ticketId param** —
the client session already pins the ticket. Cross-ticket / unknown ids return **404** (no existence
leak). Blobs read from `BLOB_ROOT` (§1); streaming uses `tokio-util` io (§4).

## Impact

- add(route): `GET /api/client/ticket/attachments/{attachmentId}` (client realm, **no ticketId param** §8) — resolves `ticket_attachment` → `attachment_file`, checks the attachment's parent ticket **is the one the client session is bound to**, streams the blob.
- add(route): `GET /api/staff/tickets/{ticketId}/attachments/{attachmentId}` (staff realm) — staff-session variant (keeps the explicit ticketId).
- update(http): both set `Content-Disposition: attachment; filename="<name>"` and the stored MIME (octet-stream fallback), stream from `BlobStore.open` (FS-022.11).
- Mismatched/unknown id, or an attachment whose parent ticket the session can't access, returns **404** with no bytes (no existence leak §8).

## Regressions

- New read-only routes; touch no write path.

## Acceptance Tests

### AC-1: FS-022.10/.11 — an authorized client downloads the blob with a Content-Disposition filename. [API-ONLY]
- Client logged into the parent ticket → GET `/api/client/ticket/attachments/{attachmentId}` → 200, correct bytes, `Content-Disposition` filename, correct MIME.
- Status: [ ]

### AC-2: BS-022.8/D2/EC-022.9 — a client logged into a DIFFERENT ticket is denied (404, no existence leak). [API-ONLY]
- Client session for ticket B requests ticket A's attachment id → **404**, no bytes (§8).
- Status: [ ]

### AC-3: EC-022.7 — an unknown/mismatched attachment id is rejected (404). [API-ONLY]
- GET a non-existent attachment id, and an id belonging to another ticket → **404**, shared error envelope (no existence leak §8).
- Status: [ ]

### AC-4: staff download — an authorized staff session downloads via the staff route. [API-ONLY]
- Staff login → GET the staff attachment route → 200, bytes, Content-Disposition.
- Status: [ ]

## Dependencies

- TS-M2-A1 (blob store + ticket_attachment), TS-M1-C1/D1 (staff/client session gates).
