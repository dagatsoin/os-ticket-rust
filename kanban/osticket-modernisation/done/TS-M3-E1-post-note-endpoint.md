# TS-M3-E1 — add(route): FS-021.4 POST note endpoint + validation

- **ID**: TS-M3-E1
- **Type**: Technical Story
- **Parent**: US-M3-E1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the POST endpoint for internal notes. Notes are thread entries of type `N`,
visible only to staff. The endpoint validates the note body (required) and optional title.

## Impact

- add(route): `POST /api/staff/tickets/{id}/note` — creates an internal note.
  - Request body: `{ "body": "...", "title": "..." (optional), "state": "..." (optional) }`
  - Response: the created thread entry (id, type=N, body, title, staff_id, created).
  - Errors: 400 if body empty ("Note required"), 404 if ticket not found, 403 if no access.
- update(service): `thread_service::post_note(ticket_id, staff_id, body, title)`.
- update(db): insert thread_entry with type=N, staff_id set, poster_name set.

## Regressions

- Existing POST reply endpoint must continue to work.
- Thread listing for staff must include type N entries.
- Thread listing for clients must exclude type N entries.

## Acceptance Tests

### AC-1: FS-021.4 — POST note creates a type N thread entry. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Internal note"}`
- Expect: 201, response has type="N", body="Internal note", staff_id is set (not null)
- Status: [x]

### AC-2: FS-021.4 — Empty body returns 400 "Note required". [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":""}`
- Expect: 400, error.message = "Note required"
- Status: [x]

### AC-3: FS-021.4 — Note with title stores both fields. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Details","title":"Summary"}`
- Expect: 201, response has title="Summary", body="Details"
- Status: [x]

### AC-4: FS-021.22 — Note has poster_name set to staff name. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Test note"}`
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}/thread
- Expect: Note entry has poster_name matching staff name (e.g., "Agent" for seeded agent user)
- Status: [x]

### AC-5: BS-021.10 — Notes excluded from client thread endpoint. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Setup: POST /api/staff/tickets/{ticket_id}/note with `{"body":"Secret"}`
- Request: GET http://localhost:3701/api/tickets/{ticket_id}/thread (no auth - client endpoint)
- Expect: 200, response does NOT contain any entry with type="N"
- Status: [x]

### AC-6: FS-021.4 — Staff thread endpoint includes notes. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Setup: POST /api/staff/tickets/{ticket_id}/note with `{"body":"Staff note"}`
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}/thread
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, response contains entry with type="N", body="Staff note"
- Status: [x]

### AC-7: 403 if staff has no access to ticket. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"dept":"External"}` (agent has no access)
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Should fail"}`
- Expect: 403, error.message indicates access denied
- Status: [x]

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Dev endpoints needed:
  - **`POST /api/dev/seed-tickets`** — accepts `dept` param for access control tests

## Dependencies

- **M1 done**: thread_entry table, staff auth.
- **TS-M3-A2**: visibility scoping (access check).
