# TS-M3-E2 — add(route): FS-021.4 note-form state change (close/reopen via note)

- **ID**: TS-M3-E2
- **Type**: Technical Story
- **Parent**: US-M3-E1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Extends the POST note endpoint to support an optional state change. When posting a note,
the agent can select a state (closed, open, answered, unanswered, overdue, notdue) and
the ticket state is updated accordingly after the note is posted.

## Impact

- update(route): `POST /api/staff/tickets/{id}/note` accepts optional `state` field.
  - Valid values: `closed`, `open` (reopen), `answered`, `unanswered`, `overdue`, `notdue`, `unchanged` (or omitted).
- update(service): after posting note, if state is set:
  - `closed` -> call close logic (BS-021.4: clear overdue/duedate)
  - `open` -> call reopen logic (BS-021.14: annul prior close, set answered=0)
  - `answered` -> set isanswered=true
  - `unanswered` -> set isanswered=false
  - `overdue` -> set isoverdue=true (manager-only, deferred to M4; M3 allows)
  - `notdue` -> clear isoverdue (manager-only, deferred to M4; M3 allows)
- update(response): note response includes `state_changed: true|false`.

## Regressions

- Posting a note without state field must work (no state change).
- State change failures should not prevent note posting (note posts, state change logged as failed).

## Acceptance Tests

### AC-1: FS-021.4 — Note with state=closed closes the ticket. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id (open ticket)
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Closing","state":"closed"}`
- Expect: 201, note posted
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Expect: status="closed", isoverdue=false, duedate=null
- Status: [x]

### AC-2: FS-021.4 — Note with state=open reopens a closed ticket. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Setup: POST /api/staff/tickets/{ticket_id}/close
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Reopening","state":"open"}`
- Expect: 201, note posted
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Expect: status="open", isanswered=false
- Status: [x]

### AC-3: FS-021.4 — Note with state=answered sets isanswered=true. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id (isanswered=false by default)
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Marking answered","state":"answered"}`
- Expect: 201
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Expect: isanswered=true
- Status: [x]

### AC-4: FS-021.4 — Note with state=unanswered sets isanswered=false. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Setup: Set ticket isanswered=true (via reply or dev endpoint)
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Marking unanswered","state":"unanswered"}`
- Expect: 201
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Expect: isanswered=false
- Status: [x]

### AC-5: FS-021.4 — Note with state=overdue sets isoverdue=true. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id (isoverdue=false by default)
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Marking overdue","state":"overdue"}`
- Expect: 201
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Expect: isoverdue=true
- Status: [x]

### AC-6: FS-021.4 — Note with state=notdue clears isoverdue. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Setup: POST /api/dev/mark-overdue/{ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Clearing overdue","state":"notdue"}`
- Expect: 201
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Expect: isoverdue=false
- Status: [x]

### AC-7: FS-021.4 — Note without state field does not change ticket state. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id (status=open)
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Just a note"}`
- Expect: 201
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Expect: status="open" (unchanged)
- Status: [x]

### AC-8: FS-021.4 — state=unchanged is a no-op. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id (status=open)
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Explicit no change","state":"unchanged"}`
- Expect: 201, note posted
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Expect: status="open" (unchanged)
- Status: [x]

### AC-9: FS-021.4 — Invalid state value returns 400. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/note
- Headers: Authorization: Bearer {{staff_token}}, Content-Type: application/json
- Body: `{"body":"Bad state","state":"bogus"}`
- Expect: 400, error.message = "Invalid state value"
- Status: [x]

## Dependencies

- **TS-M3-E1**: base POST note endpoint.
- **EPIC-M3-C**: close/reopen service logic.
- **TS-M3-D4**: overdue flag handling.

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Dev endpoints needed:
  - **`POST /api/dev/seed-tickets`** — creates test tickets
  - **`POST /api/dev/mark-overdue/{id}`** — for state=notdue test (AC-6)

**Note**: State changes close/reopen require EPIC-M3-C. M4 will add manager permission
checks for overdue/notdue states.
