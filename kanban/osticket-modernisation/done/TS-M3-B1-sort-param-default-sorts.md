# TS-M3-B1 — update(route): FS-020.5 sort param handling + per-queue default sorts

- **ID**: TS-M3-B1
- **Type**: Technical Story
- **Parent**: US-M3-B1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Extends the queue route (`GET /api/staff/tickets`) to accept `sort` and `order` query params
and apply the appropriate ORDER BY clause. When no sort is specified, applies a per-queue
default sort based on FS-020.5.

## Impact

- update(route): `GET /api/staff/tickets?status={status}&sort={key}&order={ASC|DESC}`
  - Validates `sort` against allowed keys: `date`, `ID`, `pri`, `name`, `subj`, `status`,
    `assignee`, `staff`, `dept`.
  - Validates `order` as `ASC` or `DESC`; defaults to `DESC`.
  - Applies ORDER BY to the query.
- update(service): `ticket_service::list_tickets` accepts sort/order params.
- add(logic): per-queue default sort when no sort param is supplied:
  - **Overdue**: `priority_urgency ASC, due_date ASC NULLS LAST, effective_date ASC, created ASC`
  - **Closed**: `closed_date DESC, created DESC`
  - **Answered**: `last_response_date DESC, created DESC`
  - **All other open queues**: `priority_urgency ASC, effective_date DESC, created DESC`

## Sort Key to Column Mapping

| Sort key | SQL expression |
|----------|----------------|
| `date` | `tickets.created` |
| `ID` | `tickets.ticket_number` (cast to int for numeric sort) |
| `pri` | `priorities.urgency` (JOIN priorities) |
| `name` | `tickets.requester_name` |
| `subj` | `tickets.subject` |
| `status` | `tickets.status` |
| `assignee` | `COALESCE(staff.firstname || ' ' || staff.lastname, teams.name, '')` |
| `staff` | `staff.firstname || ' ' || staff.lastname` (closing staff on Closed queue) |
| `dept` | `departments.name` |

Note: `effective_date` is computed as `COALESCE(reopened_at, last_message_at, created)`.
Note: `due_date` is the explicit due date, or (if SLA exists) `created + sla_grace_hours`.

## Regressions

- The route without sort params must continue to work, applying the queue-appropriate default sort.
- Visibility scoping and status filtering from TS-M3-A* must be preserved.

## Acceptance Tests

### AC-1: FS-020.5 — sort=date&order=DESC orders by created date descending. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":5,"status":"open","vary_created":true} -> {ids}
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session_cookie}
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=date&order=DESC
- Headers: Cookie: {session_cookie}
- Expect: 200, tickets array ordered by created DESC (newest first)
- Status: [x]

### AC-2: FS-020.5 — sort=date&order=ASC orders by created date ascending. [API-ONLY]
- Depends: AC-1 (same setup)
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=date&order=ASC
- Headers: Cookie: {session_cookie}
- Expect: 200, tickets array ordered by created ASC (oldest first)
- Status: [x]

### AC-3: FS-020.5 — sort=ID orders by ticket number numerically. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"ticket_numbers":[100,99,101],"status":"open"} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=ID&order=ASC
- Headers: Cookie: {session_cookie}
- Expect: 200, tickets in order 99, 100, 101 (numeric, not string sort)
- Status: [x]

### AC-4: FS-020.5 — sort=pri orders by priority urgency. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"priorities":["low","normal","high","emergency"],"status":"open"} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=pri&order=ASC
- Headers: Cookie: {session_cookie}
- Expect: 200, order: emergency (urgency 4), high (3), normal (2), low (1)
- Status: [x]

### AC-5: FS-020.5 — sort=name orders by requester name. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"requesters":["Alice","Bob","Charlie"],"status":"open"} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=name&order=ASC
- Headers: Cookie: {session_cookie}
- Expect: 200, order: Alice, Bob, Charlie
- Status: [x]

### AC-6: FS-020.5 — sort=subj orders by subject. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"subjects":["Zebra issue","Apple problem","Mango bug"],"status":"open"} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=subj&order=ASC
- Headers: Cookie: {session_cookie}
- Expect: 200, order: Apple problem, Mango bug, Zebra issue
- Status: [x]

### AC-7: FS-020.5 — sort=assignee orders by assignee name. [API-ONLY]
- Setup: POST /api/dev/seed-staff [{"name":"Alice Agent"},{"name":"Bob Agent"}] -> {staff_ids}
- Setup: POST /api/dev/seed-tickets {"assignees":[null,"s{staff_ids[0]}","s{staff_ids[1]}"],"status":"open"} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=assignee&order=ASC
- Headers: Cookie: {session_cookie}
- Expect: 200, order: unassigned (empty), Alice Agent, Bob Agent
- Status: [x]

### AC-8: FS-020.5 — sort=dept orders by department name. [API-ONLY]
- Setup: POST /api/dev/seed-depts [{"name":"Support"},{"name":"Sales"},{"name":"Billing"}] -> {dept_ids}
- Setup: POST /api/dev/seed-tickets {"depts":[{dept_ids}],"status":"open"} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=dept&order=ASC
- Headers: Cookie: {session_cookie}
- Expect: 200, order: Billing, Sales, Support
- Status: [x]

### AC-9: FS-020.5 — invalid order value defaults to DESC. [API-ONLY]
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=date&order=INVALID
- Headers: Cookie: {session_cookie}
- Expect: 200, tickets ordered by date DESC (default); no error
- Status: [x]

### AC-10: FS-020.5 — default sort for Open queue is priority+effective_date. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":5,"status":"open","vary_priority":true,"vary_effective_date":true} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=open (no sort param)
- Headers: Cookie: {session_cookie}
- Expect: 200, sorted by priority urgency ASC (highest first), then effective_date DESC within same priority
- Status: [x]

### AC-11: FS-020.5 — default sort for Overdue queue is urgency+due_date. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":5,"status":"overdue","vary_priority":true,"vary_duedate":true} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=overdue (no sort param)
- Headers: Cookie: {session_cookie}
- Expect: 200, sorted by urgency ASC, due_date ASC NULLS LAST (most urgent, soonest due first)
- Status: [x]

### AC-12: FS-020.5 — default sort for Closed queue is closed_date DESC. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":5,"status":"closed","vary_closed":true} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=closed (no sort param)
- Headers: Cookie: {session_cookie}
- Expect: 200, sorted by closed_date DESC (most recently closed first)
- Status: [x]

### AC-13: FS-020.5 — default sort for Answered queue is last_response DESC. [API-ONLY]
- Setup: POST /api/dev/seed-tickets {"count":5,"status":"answered","vary_last_response":true} -> {ids}
- Request: GET http://localhost:3701/api/staff/tickets?status=answered (no sort param)
- Headers: Cookie: {session_cookie}
- Expect: 200, sorted by last_response_date DESC (most recently responded first)
- Status: [x]

### AC-14: Unknown sort key is ignored, uses default sort. [API-ONLY]
- Request: GET http://localhost:3701/api/staff/tickets?status=open&sort=bogus
- Headers: Cookie: {session_cookie}
- Expect: 200, uses default sort for Open queue; no error; tickets returned
- Status: [x]

## Test Infrastructure

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose | Request Shape |
|----------|---------|---------------|
| `POST /api/dev/seed-tickets` | Create tickets with specific attributes | `{"count":N,"status":"...","vary_*":bool,"priorities":[...],"subjects":[...],...}` |
| `POST /api/dev/seed-staff` | Create additional staff for assignee tests | `[{"name":"..."}]` |
| `POST /api/dev/seed-depts` | Create departments for dept sort tests | `[{"name":"..."}]` |

## Dependencies

- **TS-M3-A1**: status param handling (provides the base queue route).
- **TS-M3-A2**: visibility scoping (must be preserved when sorting).
- **TS-M3-prep**: seed with priorities table populated (for urgency sorting).
