# TS-M3-F2 — add(route): FS-020.8 advanced search (multi-criteria filtering)

- **ID**: TS-M3-F2
- **Type**: Technical Story
- **Parent**: US-M3-F1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements advanced search: multiple optional criteria (keyword, status, department, assignee,
help topic, date range) that combine to filter the ticket listing. All criteria AND together
except for the assignee+closed-by combination which has special OR logic (BS-020.16).

## Impact

- update(route): `GET /api/staff/tickets?a=search` accepts additional query params:
  - `query` (keyword, optional, >= 3 chars if present)
  - `status` (open/answered/overdue/closed/any, optional)
  - `deptId` (department id, optional, bounded by access per BS-020.3)
  - `assignee` (s{staff_id}, t{team_id}, s0=unassigned, optional)
  - `staffId` (closed-by staff id, optional)
  - `topicId` (help topic id, optional)
  - `startDate`, `endDate` (created date bounds, optional)
- update(service): `ticket_service::advanced_search(criteria, staff_visibility)`:
  - Combines all criteria with AND.
  - Assignee filter only applies when status != closed.
  - Out-of-scope deptId is silently ignored.
  - Date bounds: created >= startDate, created <= endDate.
- update(response): when status=any (no status filter), return `status_column: true` so
  frontend knows to show Status column instead of Priority.

## Regressions

- Basic search (TS-M3-F1) must continue to work.
- Visibility scoping still applies.

## Acceptance Tests

### AC-1: FS-020.8 — Status filter: closed returns only closed tickets. [API-ONLY]
- Setup: POST /api/dev/reset-tickets
- Setup: POST /api/dev/seed-tickets (x2 open tickets)
- Setup: POST /api/dev/seed-tickets -> close (x2 closed tickets)
- Request: GET http://localhost:3701/api/staff/tickets?a=search&status=closed
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, exactly 2 tickets returned, all have status="closed"
- Status: [x]

### AC-2: FS-020.8 — Department filter returns tickets in that dept. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}` -> support_ticket
- Setup: POST /api/dev/seed-tickets with `{"dept":"Sales"}` -> sales_ticket
- Request: GET http://localhost:3701/api/staff/tickets?a=search&deptId={{support_dept_id}}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, only Support tickets returned
- Status: [x]

### AC-3: BS-020.3 — Out-of-scope deptId is silently ignored. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}` (agent has access)
- Setup: POST /api/dev/seed-tickets with `{"dept":"External"}` (agent has no access)
- Request: GET http://localhost:3701/api/staff/tickets?a=search&deptId={{external_dept_id}}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, returns tickets within agent's normal visibility (ignores out-of-scope filter)
- Status: [x]

### AC-4: FS-020.8 — Assignee filter: s{id} returns tickets assigned to that staff. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Setup: POST /api/staff/tickets/{ticket_id}/assign with `{"staffId":1}`
- Request: GET http://localhost:3701/api/staff/tickets?a=search&assignee=s1
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, ticket is returned
- Status: [x]

### AC-5: FS-020.8 — Assignee filter: s0 returns unassigned tickets. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> unassigned_ticket
- Setup: POST /api/dev/seed-tickets -> assigned_ticket, then assign
- Request: GET http://localhost:3701/api/staff/tickets?a=search&assignee=s0
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, only unassigned ticket returned
- Status: [x]

### AC-6: FS-020.8 — Assignee filter ignored when status=closed. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Setup: POST /api/staff/tickets/{ticket_id}/assign with `{"staffId":1}`
- Setup: POST /api/staff/tickets/{ticket_id}/close
- Request: GET http://localhost:3701/api/staff/tickets?a=search&status=closed&assignee=s2
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, closed ticket still returned (assignee filter not applied to closed)
- Status: [x]

### AC-7: FS-020.8 — Topic filter returns tickets with that topic. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"topic":"Billing"}`
- Setup: POST /api/dev/seed-tickets with `{"topic":"General"}`
- Request: GET http://localhost:3701/api/staff/tickets?a=search&topicId={{billing_topic_id}}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, only Billing tickets returned
- Status: [x]

### AC-8: FS-020.8 — Date range filter bounds created date. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"created":"2026-06-01"}`
- Setup: POST /api/dev/seed-tickets with `{"created":"2026-06-10"}`
- Setup: POST /api/dev/seed-tickets with `{"created":"2026-06-20"}`
- Request: GET http://localhost:3701/api/staff/tickets?a=search&startDate=2026-06-05&endDate=2026-06-15
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, only 2026-06-10 ticket returned
- Status: [x]

### AC-9: FS-020.8 — Multiple criteria AND together. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}` -> open Support
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}` -> close -> closed Support
- Setup: POST /api/dev/seed-tickets with `{"dept":"Sales"}` -> open Sales
- Request: GET http://localhost:3701/api/staff/tickets?a=search&status=open&deptId={{support_dept_id}}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, only the open Support ticket returned
- Status: [x]

### AC-10: EC-020.10 — status=any returns status_column flag. [API-ONLY]
- Request: GET http://localhost:3701/api/staff/tickets?a=search
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, response has `status_column: true`
- Status: [x]

### AC-11: EC-020.3 — Invalid date span rejected. [API-ONLY]
- Request: GET http://localhost:3701/api/staff/tickets?a=search&startDate=2026-06-20&endDate=2026-06-01
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 400, error.message = "Entered date span is invalid."
- Status: [x]

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Dev endpoints needed:
  - **`POST /api/dev/seed-tickets`** — accepts `dept`, `topic`, `created` params
  - **`POST /api/dev/reset-tickets`** — clears all tickets for clean state
  - **`POST /api/staff/tickets/{id}/assign`** — for assignee filter tests

## Dependencies

- **TS-M3-F1**: basic search infrastructure.
- **TS-M3-A2**: visibility scoping.
- **TS-M3-prep**: help topics seeded.
