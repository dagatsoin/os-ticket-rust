# TS-M3-A1 — add(route): BS-020.1 status param handling + quick-stats endpoint

- **ID**: TS-M3-A1
- **Type**: Technical Story
- **Parent**: US-M3-A1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Extends the M1 queue route (`GET /api/staff/tickets`) to accept a `status` query param that
selects predefined queues, and adds a `GET /api/staff/tickets/stats` endpoint returning quick
counts per queue for the authenticated staff member.

The `status` param is "overloaded" (BS-020.1): it selects a queue, not a raw ticket status.
- `open` (default) → `status='open'` + answered/assigned visibility rules (BS-020.4)
- `answered` → `status='open' AND isanswered=true`
- `assigned` → `status='open' AND staff_id=me` (My Tickets)
- `overdue` → `status='open' AND isoverdue=true`
- `closed` → `status='closed'`

## Impact

- update(route): `GET /api/staff/tickets?status={open|answered|assigned|overdue|closed}`
  applies the appropriate WHERE clause. Unrecognized/missing status defaults to `open`.
  **Response shape**: `{ items: QueueItem[], rightmost_column: string, total: number }`
  (pagination params `page`, `limit` are handled by TS-M3-B3; sorting by TS-M3-B1)
- add(route): `GET /api/staff/tickets/stats` returns `{ open, answered, overdue, assigned, closed }`
  counts scoped to the staff member's visibility (FS-020.11).
- add(service): `ticket_service::get_staff_stats(staff_id, dept_ids, team_ids, show_assigned, show_answered)`
  computes the quick stats.

## Regressions

- The existing M1 queue route (no status param) must continue to work, defaulting to `open`.
- Visibility scoping (TS-M3-A2) is wired in by this ticket; until TS-M3-A2 lands, use a
  placeholder that returns all tickets (will be refined).

## Test Infrastructure

- **Backend**: http://localhost:3701 (must be running)
- **Session cookie**: Obtain via `POST /api/staff/login` with `{"username":"agent","password":"Agent123!"}`
- **Dev endpoint needed**: `POST /api/dev/seed-tickets` to create test tickets with configurable status/flags

## Acceptance Tests

### AC-1: BS-020.1 — GET /api/staff/tickets?status=open returns only open, non-answered, unassigned tickets (when toggles off). [API-ONLY]
- Setup: `cargo run -p tools --bin seed -- --reset` then update config: `psql -c "UPDATE config SET value='0' WHERE key IN ('show_assigned_tickets','show_answered_tickets')"`
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","isanswered":false,"staff_id":null},{"status":"open","isanswered":true,"staff_id":null},{"status":"open","isanswered":false,"staff_id":1}]}'`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=open`
- Expect: 200, items array contains only the open/unanswered/unassigned ticket
- Status: [x]

### AC-2: BS-020.1 — GET /api/staff/tickets?status=answered returns open+answered tickets. [API-ONLY]
- Setup: tickets from AC-1 still present (includes one open/answered ticket)
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=answered`
- Expect: 200, items array contains the open/answered ticket; no closed tickets
- Status: [x]

### AC-3: BS-020.1 — GET /api/staff/tickets?status=assigned returns only tickets assigned to me. [API-ONLY]
- Setup: tickets from AC-1 still present (includes one open/assigned-to-agent ticket)
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=assigned`
- Expect: 200, items array contains only tickets with staff_id=agent's id and status=open
- Status: [x]

### AC-4: BS-020.1 — GET /api/staff/tickets?status=overdue returns open+overdue tickets. [API-ONLY]
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","isoverdue":true}]}'`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=overdue`
- Expect: 200, items array contains only the overdue ticket; non-overdue tickets excluded
- Status: [x]

### AC-5: BS-020.1 — GET /api/staff/tickets?status=closed returns closed tickets. [API-ONLY]
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"closed"}]}'`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=closed`
- Expect: 200, items array contains only closed tickets
- Status: [x]

### AC-6: Default status — GET /api/staff/tickets (no param) defaults to open queue. [API-ONLY]
- Setup: seed and config from AC-1
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets`
- Expect: 200, response equivalent to ?status=open (same items as AC-1)
- Status: [x]

### AC-7: FS-020.11 — GET /api/staff/tickets/stats returns quick counts. [API-ONLY]
- Setup: `cargo run -p tools --bin seed -- --reset` then seed varied tickets: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open"},{"status":"open"},{"status":"open","isanswered":true},{"status":"open","isoverdue":true},{"status":"open","staff_id":1},{"status":"closed"},{"status":"closed"},{"status":"closed"}]}'`
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets/stats`
- Expect: 200, JSON `{"open":2,"answered":1,"overdue":1,"assigned":1,"closed":3}` (counts reflect visibility scope)
- Status: [x]

### AC-8: Unknown status value defaults to open. [API-ONLY]
- Setup: seed from AC-1
- Request: `curl -s -b 'session={{cookie}}' http://localhost:3701/api/staff/tickets?status=bogus`
- Expect: 200, response equivalent to ?status=open, no error returned
- Status: [x]

## Dependencies

- **M1 done**: the baseline `GET /api/staff/tickets` route exists.
- **TS-M3-A2**: visibility scoping (this ticket wires it in; if A2 is not ready, stub it).
- **TS-M3-prep**: seed expansion (config keys `show_assigned_tickets`, `show_answered_tickets`).
