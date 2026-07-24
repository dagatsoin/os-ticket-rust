# TS-M3-I1 — add(route): FS-021.15 edit/update endpoint + validation

- **ID**: TS-M3-I1
- **Type**: Technical Story
- **Parent**: US-M3-I1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the backend ticket edit/update endpoint per FS-021.15. The endpoint accepts updated
ticket properties, validates them, re-selects SLA if department changed, clears overdue if due
date is now in the future, and logs an internal "Ticket Updated" note with the reason.

## Impact

- add(route): `PUT /api/staff/tickets/{ticketId}` with JSON body:
  ```json
  {
    "name": "New Name",
    "email": "new@example.com",
    "phone": "555-1234",
    "phone_ext": "123",
    "dept_id": 2,
    "topic_id": 1,
    "priority_id": 3,
    "sla_id": 2,
    "source": "phone",
    "due_date": "2026-07-01T14:00:00Z",
    "reason": "Escalating per manager request"
  }
  ```
- add(service): `ticket_service::update_ticket(staff, ticket_id, updates)` — validates, updates,
  recomputes SLA/overdue, logs activity note.
- update(ticket): after update, if dept changed and SLA is empty/transient, call `select_sla_id()`
  with new department precedence (FS-021.13).

## Regressions

- Ticket read routes must return updated values after edit.
- Queue listing must reflect changed priority/dept/status.

## Acceptance Tests

> Setup (all): `cargo run -p tools --bin seed -- --reset`; backend on :3701; staff login to cookie
> jar `/tmp/qa-staff.jar` (agent with `can_edit_tickets=true`).

### AC-1: FS-021.15 — successful edit updates ticket and logs note. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{priority_id: 2}] → {ids: [10]}
- Setup: POST /api/dev/login → {username: "agent", password: "Agent123!"} → cookie jar /tmp/qa-staff.jar
- Request: `curl -s -b /tmp/qa-staff.jar -X PUT http://localhost:3701/api/staff/tickets/10 -H "Content-Type: application/json" -d '{"priority_id":3,"reason":"Escalating"}'`
- Expect: HTTP 200, response contains `"id": 10` and `"priority_id": 3`
- Request: `curl -s -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/10`
- Expect: Response contains `"priority_id": 3`
- Request: `curl -s -b /tmp/qa-staff.jar http://localhost:3701/api/staff/tickets/10/thread`
- Expect: Response contains Note entry with "Ticket Updated" and "Escalating"
- Status: [x]

### AC-2: FS-021.15 — reason is required. [API-ONLY]
- Request: `curl -s -b /tmp/qa-staff.jar -X PUT http://localhost:3701/api/staff/tickets/10 -H "Content-Type: application/json" -d '{"priority_id":3}'`
- Expect: HTTP 422, body contains `"Reason for the update required"`
- Status: [x]

### AC-3: EC-021.9 — due date must be in the future. [API-ONLY]
- Request: `curl -s -b /tmp/qa-staff.jar -X PUT http://localhost:3701/api/staff/tickets/10 -H "Content-Type: application/json" -d '{"due_date":"2020-01-01T00:00:00Z","reason":"test"}'`
- Expect: HTTP 422, body contains `"Due date must be in the future"`
- Status: [x]

### AC-4: EC-021.9 — due date cannot be set on closed ticket. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "closed"}] → {ids: [11]}
- Request: `curl -s -b /tmp/qa-staff.jar -X PUT http://localhost:3701/api/staff/tickets/11 -H "Content-Type: application/json" -d '{"due_date":"2027-01-01T00:00:00Z","reason":"test"}'`
- Expect: HTTP 422, body contains `"Due date can NOT be set on a closed ticket"`
- Status: [x]

### AC-5: BS-021.6 — editing due date clears overdue flag. [API-ONLY]
- Setup: POST /api/dev/seed-tickets → [{status: "open", isoverdue: true}] → {ids: [12]}
- Request: `curl -s -b /tmp/qa-staff.jar -X PUT http://localhost:3701/api/staff/tickets/12 -H "Content-Type: application/json" -d '{"due_date":"2027-01-01T00:00:00Z","reason":"extending deadline"}'`
- Expect: HTTP 200, response contains `"isoverdue": false`
- Status: [x]

### AC-6: BS-021.7 — editing department re-selects SLA. [API-ONLY]
- Setup: POST /api/dev/seed-departments → [{id: 1, name: "Support", sla_id: 1}, {id: 2, name: "Sales", sla_id: 2}]
- Setup: POST /api/dev/seed-tickets → [{dept_id: 1}] → {ids: [13]}
- Request: `curl -s -b /tmp/qa-staff.jar -X PUT http://localhost:3701/api/staff/tickets/13 -H "Content-Type: application/json" -d '{"dept_id":2,"reason":"routing to Sales"}'`
- Expect: HTTP 200, response contains `"sla_id": 2` (Sales dept SLA)
- Status: [x]

### AC-7: canEditTickets permission gate. [API-ONLY]
- Setup: POST /api/dev/seed-staff → {username: "agent2", password: "Agent123!", can_edit_tickets: false}
- Setup: POST /api/dev/login → {username: "agent2"} → cookie jar /tmp/qa-agent2.jar
- Request: `curl -s -b /tmp/qa-agent2.jar -X PUT http://localhost:3701/api/staff/tickets/10 -H "Content-Type: application/json" -d '{"priority_id":3,"reason":"test"}'`
- Expect: HTTP 403, body contains `"Permission denied"`
- Status: [x]

### AC-8: Validation — email format, phone extension requires phone. [API-ONLY]
- Request: `curl -s -b /tmp/qa-staff.jar -X PUT http://localhost:3701/api/staff/tickets/10 -H "Content-Type: application/json" -d '{"email":"invalid","reason":"test"}'`
- Expect: HTTP 422, body contains error on email field
- Request: `curl -s -b /tmp/qa-staff.jar -X PUT http://localhost:3701/api/staff/tickets/10 -H "Content-Type: application/json" -d '{"phone_ext":"123","reason":"test"}'`
- Expect: HTTP 422, body contains `"Phone extension requires a phone number"`
- Status: [x]

## Test Infrastructure

**Dev Endpoints Required (Setup only):**
- `POST /api/dev/seed-tickets` — create tickets with specified properties; returns `{ids: [...]}`
- `POST /api/dev/seed-staff` — create staff with permission flags
- `POST /api/dev/seed-departments` — create departments with SLA assignments
- `POST /api/dev/login` — authenticate and return session cookie

## Dependencies

- **EPIC-M3-D**: SLA infrastructure for `select_sla_id()` logic.
- **EPIC-M3-E**: internal note posting for "Ticket Updated" log.
- **TS-M3-prep**: seed expansion for edit permission flag, multiple depts/SLAs.
