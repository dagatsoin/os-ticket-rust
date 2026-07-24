# TS-M3-D4 — add(service): BS-032.10 overdue detection + flag on listing

- **ID**: TS-M3-D4
- **Type**: Technical Story
- **Parent**: US-M3-D1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements overdue detection: computes whether a ticket is overdue based on its due date,
sets the isoverdue flag, and includes it in the listing response. The automated sweep
(cron-driven checkOverdue) is M6; this ticket handles manual flag setting and listing display.

## Impact

- add(service): BS-032.10 `ticket_service::is_overdue(ticket) -> bool`:
  - Returns true if ticket.duedate is set and current time >= duedate.
- update(service): ticket listing (`GET /api/staff/tickets?status=overdue`) filters by
  `status='open' AND isoverdue=true`.
- update(route): ticket listing response includes `isoverdue`, `duedate` per row.
- add(service): BS-021.4 on close, clear isoverdue and duedate.
- add(dev-endpoint): `POST /api/dev/mark-overdue/{id}` to manually set isoverdue=true for testing.

## Regressions

- Existing listing queries must not break; isoverdue is already on the ticket table.
- The close action (EPIC-M3-C) must integrate with clearing isoverdue.

## Acceptance Tests

### AC-1: BS-032.10 — Listing includes isoverdue flag per ticket. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"sla":"Urgent","created_hours_ago":5}`
- Setup: POST /api/dev/mark-overdue/{id}
- Request: GET http://localhost:3701/api/staff/tickets?status=open
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, ticket row has `isoverdue: true`
- Status: [x]

### AC-2: FS-020.2 — Overdue queue returns only open+overdue tickets. [API-ONLY]
- Setup: POST /api/dev/reset-tickets
- Setup: POST /api/dev/seed-tickets -> ticket A (open, mark overdue)
- Setup: POST /api/dev/seed-tickets -> ticket B (open, not overdue)
- Setup: POST /api/dev/seed-tickets -> ticket C (close it, mark overdue)
- Request: GET http://localhost:3701/api/staff/tickets?status=overdue
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, only ticket A returned; B and C excluded
- Status: [x]

### AC-3: BS-021.4 — Closing clears isoverdue and duedate. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"sla":"Urgent","created_hours_ago":5}` -> ticket_id
- Setup: POST /api/dev/mark-overdue/{ticket_id}
- Request: POST http://localhost:3701/api/staff/tickets/{ticket_id}/close
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Expect: isoverdue = false, duedate = null
- Status: [x]

### AC-4: BS-032.10 — Listing includes duedate per ticket. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}`
- Request: GET http://localhost:3701/api/staff/tickets?status=open
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, ticket row has `duedate` field (ISO timestamp string)
- Status: [x]

### AC-5: Dev endpoint marks ticket overdue. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Request: POST http://localhost:3701/api/dev/mark-overdue/{ticket_id}
- Expect: 200
- Request: GET http://localhost:3701/api/staff/tickets/{ticket_id}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: isoverdue = true
- Status: [x]

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Dev endpoints needed:
  - **`POST /api/dev/seed-tickets`** — accepts `sla`, `created_hours_ago` params
  - **`POST /api/dev/mark-overdue/{id}`** — sets isoverdue=true for testing
  - **`POST /api/dev/reset-tickets`** — clears all tickets for clean state

## Dependencies

- **TS-M3-D1**: ticket.isoverdue, ticket.duedate columns.
- **TS-M3-D3**: due-date computation.
- **EPIC-M3-C**: close action (for AC-3).
