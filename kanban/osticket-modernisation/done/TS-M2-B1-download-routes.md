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

> Setup (all): `cargo run -p tools --bin seed -- --reset`; backend on :3701; fixtures in `/tmp/qa-fixtures`.
> Seed **ticket-A** (email `mia@example.com`) with an `M` `invoice.pdf` attachment (capture its
> `attachmentId` from `GET /api/staff/tickets/{A}`), and **ticket-B** (different email). Client login
> to per-ticket cookie jars — `curl -c /tmp/qa-clientA.jar -X POST .../api/client/login` (ticket-A
> number + email), likewise `/tmp/qa-clientB.jar` for ticket-B.

### AC-1: FS-022.10/.11 — an authorized client downloads the blob with a Content-Disposition filename. [API-ONLY]
- Request: `curl -i -b /tmp/qa-clientA.jar http://localhost:3701/api/client/ticket/attachments/{attachmentId} -o /tmp/dl.pdf`.
- Verify: HTTP 200; `Content-Disposition: attachment; filename="invoice.pdf"`; `Content-Type` is the stored MIME (octet-stream fallback); `cmp /tmp/dl.pdf /tmp/qa-fixtures/invoice.pdf` is byte-identical.
- Status: [x]

### AC-2: BS-022.8/D2/EC-022.9 — a client logged into a DIFFERENT ticket is denied (404, no existence leak). [API-ONLY]
- Request: `curl -i -b /tmp/qa-clientB.jar http://localhost:3701/api/client/ticket/attachments/{attachmentId}` (ticket-A's id, ticket-B session).
- Verify: HTTP 404, empty body (no bytes, §8).
- Status: [x]

### AC-3: EC-022.7 — an unknown/mismatched attachment id is rejected (404). [API-ONLY]
- Request: `curl -i -b /tmp/qa-clientA.jar .../api/client/ticket/attachments/999999`.
- Verify: HTTP 404 with the shared error envelope; the response is indistinguishable from AC-2's cross-ticket 404 (no existence leak §8).
- Status: [x]

### AC-4: staff download — an authorized staff session downloads via the staff route. [API-ONLY]
- Request: staff login to `/tmp/qa-staff.jar`, then `curl -i -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/{A}/attachments/{attachmentId} -o /tmp/dls.pdf`.
- Verify: HTTP 200, bytes match `invoice.pdf`, `Content-Disposition` filename present.
- Status: [x]

## Dependencies

- TS-M2-A1 (blob store + ticket_attachment), TS-M1-C1/D1 (staff/client session gates).
