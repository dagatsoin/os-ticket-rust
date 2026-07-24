# TS-M3-I2 — add(route): FS-021.19 delete endpoint + cascade

- **ID**: TS-M3-I2
- **Type**: Technical Story
- **Parent**: EPIC-M3-I
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the backend ticket delete endpoint per FS-021.19. Deletion is permanent and cascades
to thread entries and their attachments. Requires `canDeleteTickets` permission. A debug log
records who deleted which ticket.

## Impact

- add(route): `DELETE /api/staff/tickets/{ticketId}`
- add(service): `ticket_service::delete_ticket(staff, ticket_id)` — verifies permission, deletes
  ticket row, cascades to thread entries, purges orphaned attachment blobs.
- update(logging): record deletion in system log (debug level): "Ticket #X deleted by username".

## Regressions

- Bulk delete (TS-M3-G1) should reuse this service function.
- Attachment purge must not affect attachments shared by other tickets (content-addressed).

## Acceptance Tests

> Setup (all): `cargo run -p tools --bin seed -- --reset`; backend on :3701; staff login to cookie
> jar `/tmp/qa-staff.jar` (agent with `can_delete_tickets=true`).

### AC-1: FS-021.19 — delete removes ticket and returns success. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "open", ticket_number: "XYZ123"}] → {ids: [50]}
- Setup: POST /api/dev/login → {username: "agent", password: "Agent123!"} → cookie jar /tmp/qa-staff.jar
- Request: `curl -s -b /tmp/qa-staff.jar -X DELETE http://localhost:3701/api/staff/tickets/50`
- Expect: HTTP 200, body contains `"Ticket #XYZ123 deleted successfully"`
- Request: `curl -s -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/50`
- Expect: HTTP 404
- Status: [x]

### AC-2: BS-021.16 — delete cascades to thread entries. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [51]}
- Setup: POST /api/dev/seed-thread-entries → {ticket_id: 51, entries: [{type: "reply"}, {type: "reply"}]}
- Request: `curl -s -b /tmp/qa-staff.jar -X DELETE http://localhost:3701/api/staff/tickets/51`
- Expect: HTTP 200
- Request: `curl -s -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/51/thread`
- Expect: HTTP 404 (ticket and thread gone)
- Verify: Query DB: SELECT count(*) FROM ticket_thread WHERE ticket_id=51 → 0
- Status: [x]

### AC-3: BS-021.16 — delete cascades to attachments (orphan purge). [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [52]}
- Setup: POST /api/dev/seed-attachment → {ticket_id: 52, filename: "unique-test.pdf", file_hash: "abc123unique"}
- Request: `curl -s -b /tmp/qa-staff.jar -X DELETE http://localhost:3701/api/staff/tickets/52`
- Expect: HTTP 200
- Verify: Query DB: SELECT count(*) FROM ticket_attachment WHERE ticket_id=52 → 0
- Verify: Query DB: SELECT count(*) FROM attachment_file WHERE file_hash='abc123unique' → 0
- Verify: File system: BLOB_ROOT/abc123unique does not exist
- Status: [x]

### AC-4: Attachment shared by another ticket is NOT purged. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}, {status: "open"}] → {ids: [53, 54]}
- Setup: POST /api/dev/seed-attachment → {ticket_id: 53, filename: "shared.pdf", file_hash: "sharedHash"}
- Setup: POST /api/dev/seed-attachment → {ticket_id: 54, filename: "shared.pdf", file_hash: "sharedHash"}
- Request: `curl -s -b /tmp/qa-staff.jar -X DELETE http://localhost:3701/api/staff/tickets/53`
- Expect: HTTP 200
- Request: `curl -s -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/54/attachments`
- Expect: Response includes attachment with file_hash "sharedHash"
- Verify: Query DB: SELECT count(*) FROM attachment_file WHERE file_hash='sharedHash' → 1
- Status: [x]

### AC-5: canDeleteTickets permission gate. [API-ONLY]
- Setup: POST /api/dev/seed-staff → {username: "agent2", password: "Agent123!", can_delete_tickets: false}
- Setup: POST /api/dev/login → {username: "agent2"} → cookie jar /tmp/qa-agent2.jar
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [55]}
- Request: `curl -s -b /tmp/qa-agent2.jar -X DELETE http://localhost:3701/api/staff/tickets/55`
- Expect: HTTP 403, body contains `"Permission denied"`
- Status: [x]

### AC-6: Delete non-existent ticket returns 404. [API-ONLY]
- Request: `curl -s -b /tmp/qa-staff.jar -X DELETE http://localhost:3701/api/staff/tickets/99999`
- Expect: HTTP 404, body contains `"Ticket not found"`
- Status: [x]

### AC-7: Deletion is logged. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "open", ticket_number: "DEL001"}] → {ids: [56]}
- Request: `curl -s -b /tmp/qa-staff.jar -X DELETE http://localhost:3701/api/staff/tickets/56`
- Expect: HTTP 200
- Verify: System log or debug output contains "Ticket #DEL001 deleted by agent"
- Status: [x]

## Test Infrastructure

**Dev Endpoints Required (Setup only):**
- `POST /api/dev/seed-tickets` — create tickets; returns `{ids: [...]}`
- `POST /api/dev/seed-staff` — create staff with permission flags
- `POST /api/dev/seed-thread-entries` — create thread entries for a ticket
- `POST /api/dev/seed-attachment` — attach file to ticket with specific hash
- `POST /api/dev/login` — authenticate and return session cookie

## Dependencies

- **TS-M2-A1**: attachment storage, BlobStore for orphan purge.
- **TS-M3-prep**: seed expansion for delete permission flag.
