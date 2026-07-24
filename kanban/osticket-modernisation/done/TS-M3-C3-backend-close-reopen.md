# TS-M3-C3 — add(route): FS-021.11/12 close/reopen routes + lifecycle events

- **ID**: TS-M3-C3
- **Type**: Technical Story
- **Parent**: US-M3-C2
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

This technical story supports US-M3-C2 "Agent closes and reopens a ticket".
Implements the backend routes for closing and reopening tickets. Close sets status to closed,
clears overdue/due date, credits the closer, and logs an internal note + lifecycle event.
Reopen sets status to open, marks unanswered, annuls the prior close event, and logs a note.

## Impact

- add(route): `POST /api/staff/tickets/{id}/close`
  - Body: `{ "comments": "optional string" }`
  - Requires `can_close_tickets` permission
  - Validates: ticket is open (else 422 "Ticket is already closed!")
  - Sets: status='closed', closed=now, isoverdue=false, duedate=null, staff_id=actor (closer)
  - Logs internal note with comments or "Ticket closed (without comments)"
  - Records `closed` lifecycle event
  - Returns: `{ "success": true, "message": "Ticket #<number> status set to CLOSED" }`
  - Error: 403 (no permission), 422 (already closed), 404 (not found)

- add(route): `POST /api/staff/tickets/{id}/reopen`
  - Body: `{ "comments": "optional string" }`
  - Requires `can_close_tickets` OR `can_create_tickets` permission
  - Validates: ticket is closed (else 422 "Ticket is already open!")
  - Sets: status='open', reopened=now, isanswered=false
  - Logs internal note with comments or "Ticket reopened (without comments)"
  - Records `reopened` lifecycle event
  - Annuls prior `closed` event (set annulled=true on the most recent closed event for this ticket)
  - Returns: `{ "success": true, "message": "Ticket REOPENED" }`
  - Error: 403, 422 (already open), 404

- add(service): `ticket_service::close(ticket_id, staff_id, comments)`
  - Status change, clear overdue, log note, record event

- add(service): `ticket_service::reopen(ticket_id, staff_id, comments)`
  - Status change, mark unanswered, log note, record event, annul prior close

- add(model): lifecycle event recording
  - `ticket_event` table insert with state='closed'/'reopened', staff_id, timestamp
  - Annul support: UPDATE ticket_event SET annulled=true WHERE ticket_id=? AND state='closed' AND annulled=false

## Regressions

- The existing ticket detail route must continue to work after status changes.
- The status field must be persisted and reflected in API responses.

## Acceptance Tests

### AC-1: FS-021.11 — POST /api/staff/tickets/{id}/close succeeds for open ticket. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open"} -> {ticket_id, ticket_number}
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session_cookie}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/close
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"comments":"Issue resolved"}
- Expect: 200 `{"success":true,"message":"Ticket #{ticket_number} status set to CLOSED"}`
- Verify: SELECT status, closed, staff_id FROM ticket WHERE ticket_id = {ticket_id} -> status='closed', closed IS NOT NULL, staff_id = agent's id
- Status: [x]

### AC-2: FS-021.11 — Close fails if ticket is already closed. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed"} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/close
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {}
- Expect: 422 `{"error":{"message":"Ticket is already closed!"}}`
- Status: [x]

### AC-3: BS-021.4 — Close clears overdue flag and due date. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","isoverdue":true,"duedate":"2026-06-20T12:00:00Z"} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/close
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {}
- Expect: 200
- Verify: SELECT isoverdue, duedate FROM ticket WHERE ticket_id = {ticket_id} -> isoverdue=false, duedate IS NULL
- Status: [x]

### AC-4: FS-021.11 — Close logs internal note. [API-ONLY]
- Depends: AC-1 (close completed with comment)
- Verify: SELECT body FROM ticket_thread WHERE ticket_id = {ticket_id} AND type = 'N' ORDER BY id DESC LIMIT 1 -> contains "Issue resolved"
- Status: [x]

### AC-5: FS-021.11 — Close records a lifecycle event. [API-ONLY]
- Depends: AC-1 (close completed)
- Verify: SELECT state, staff_id FROM ticket_event WHERE ticket_id = {ticket_id} ORDER BY id DESC LIMIT 1 -> state='closed', staff_id = agent's id
- Status: [x]

### AC-6: FS-021.12 — POST /api/staff/tickets/{id}/reopen succeeds for closed ticket. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed"} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/reopen
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {"comments":"Needs follow-up"}
- Expect: 200 `{"success":true,"message":"Ticket REOPENED"}`
- Verify: SELECT status, reopened FROM ticket WHERE ticket_id = {ticket_id} -> status='open', reopened IS NOT NULL
- Status: [x]

### AC-7: FS-021.12 — Reopen fails if ticket is already open. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open"} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/reopen
- Headers: Cookie: {session_cookie}, Content-Type: application/json
- Body: {}
- Expect: 422 `{"error":{"message":"Ticket is already open!"}}`
- Status: [x]

### AC-8: FS-021.12 — Reopen marks ticket unanswered. [API-ONLY]
- Depends: AC-6 (reopen completed)
- Verify: SELECT isanswered FROM ticket WHERE ticket_id = {ticket_id} -> false
- Status: [x]

### AC-9: BS-021.14 — Reopen annuls the prior closed event. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open"} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/close (close it first)
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/reopen (then reopen)
- Headers: Cookie: {session_cookie}
- Expect: 200
- Verify: SELECT annulled FROM ticket_event WHERE ticket_id = {ticket_id} AND state = 'closed' ORDER BY id DESC LIMIT 1 -> annulled=true
- Status: [x]

### AC-10: FS-021.12 — Reopen records a lifecycle event. [API-ONLY]
- Depends: AC-6 (reopen completed)
- Verify: SELECT state FROM ticket_event WHERE ticket_id = {ticket_id} ORDER BY id DESC LIMIT 1 -> 'reopened'
- Status: [x]

### AC-11: BS-021.1 — Close returns 403 without can_close_tickets. [API-ONLY]
- Setup: POST /api/dev/set-group-perm {"group_id":1,"can_close_tickets":false}
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open"} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/close
- Headers: Cookie: {session_cookie}
- Expect: 403 `{"error":{"message":"Not allowed to close tickets"}}`
- Cleanup: POST /api/dev/set-group-perm {"group_id":1,"can_close_tickets":true}
- Status: [x]

### AC-12: FS-021.12 — Reopen succeeds with can_create_tickets even without can_close_tickets. [API-ONLY]
- Setup: POST /api/dev/set-group-perm {"group_id":1,"can_close_tickets":false,"can_create_tickets":true}
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed"} -> {ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/reopen
- Headers: Cookie: {session_cookie}
- Expect: 200 `{"success":true,"message":"Ticket REOPENED"}` (reopen allowed by create permission)
- Cleanup: POST /api/dev/set-group-perm {"group_id":1,"can_close_tickets":true}
- Status: [x]

## Test Infrastructure

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose | Request Shape |
|----------|---------|---------------|
| `POST /api/dev/seed-tickets` | Create test tickets | `{"count":N,"status":"...","isoverdue":bool,"duedate":"ISO"}` |
| `POST /api/dev/set-group-perm` | Toggle group permissions | `{"group_id":N,"can_close_tickets":bool,"can_create_tickets":bool}` |
| `POST /api/auth/login` | Get session cookie | `{"username":"...","password":"..."}` |

## Dependencies

- **M1 done**: ticket, staff, thread infrastructure.
- **TS-M3-prep**: permission flags.
- **EPIC-M3-D**: overdue flag (cleared on close).

**Note**: AC-3/AC-5 (overdue clearing) require overdue ticket setup from EPIC-M3-D.
Until M3-D lands, test by manually seeding `isoverdue=true`.
