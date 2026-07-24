# US-M3-D1 — Overdue tickets appear in the Overdue tab

- **ID**: US-M3-D1
- **Type**: User Story
- **Parent**: EPIC-M3-D
- **Labels**: User Story, M3
- **Scope**: medium

## Spec References

- FS-021.13 (due date, SLA selection, overdue marking)
- FS-020.2 (overdue queue tab: `status='open' AND isoverdue=1`)
- FS-032.11 (SLA overdue computation: grace_period math)
- FS-091.6 (SLA plan reference: grace_period hours)
- BS-021.4 (closing clears overdue flag and due date)
- BS-032.10 (ticket is overdue on grace-period violation)

## Context

Epic: EPIC-M3-D — SLA + Overdue.
M1/M2 had no SLA or overdue handling. This story delivers the user-visible behavior: tickets
that exceed their SLA grace period appear in the Overdue tab, display their due date and an
overdue badge, and the overdue flag clears when the ticket is closed.

## Description

As a support agent, I can see tickets that have exceeded their SLA grace period in the Overdue
queue tab with a visual overdue badge, and I can see each ticket's due date based on its SLA.
When I close an overdue ticket, the overdue flag is cleared.

## Impact

- Frontend (Overdue tab, due date display, overdue badge on Subject column)
- Backend (SLA selection precedence, due-date computation, overdue flag on listing)
- Database (sla_plan table, ticket.isoverdue, ticket.duedate)
- Browser (desktop)

## Business Rules

- BS-032.10: A ticket is overdue when current time >= created + grace_period hours (or
  reopened + grace_period for reopened tickets).
- BS-021.4: Closing clears the overdue flag and due date.
- FS-021.13 SLA precedence: explicit trump > department SLA > topic SLA > system default.
- FS-020.2: Overdue tab filters `status='open' AND isoverdue=1`.

## Regressions

- The existing queue tabs (Open, Answered, Assigned, Closed) must continue to work.
- Closing a ticket (M3-C) must clear the overdue flag.

## Acceptance Criteria

### AC-1: Tickets past their SLA due date appear in the Overdue tab with an overdue badge. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"sla":"Urgent","created_hours_ago":5}` to create overdue ticket
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Overdue" tab
- Verify: Overdue tab shows count badge > 0
- Verify: The seeded ticket appears in the listing
- Verify: Ticket row displays overdue badge/icon in Subject column
- Status: [x]

### AC-2: Each ticket's due date is displayed in the listing or ticket view. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"sla":"Standard"}` to create ticket with 24h SLA
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Ticket row shows due date column or tooltip
- Action: Click ticket to open detail view
- Verify: Ticket detail shows due date (created + 24 hours)
- Status: [x]

### AC-3: Closing an overdue ticket clears the overdue flag; the ticket no longer appears in Overdue tab. [BROWSER]
- Depends: AC-1
- Setup: Use the overdue ticket from AC-1
- Navigate: http://localhost:3702/staff/tickets?status=overdue
- Verify: Ticket appears in Overdue listing
- Action: Click ticket to open detail view
- Action: Click "Close" action button
- Verify: Redirected to listing; ticket no longer in Overdue tab
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: Ticket appears in Closed listing
- Status: [x]

### AC-4: SLA selection follows precedence: department SLA > topic SLA > system default. [API-ONLY]
- Setup: Run seed to ensure Support dept has SLA "Standard" (24h), topic "Billing" has SLA "Urgent" (4h)
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support","topic":"Billing"}`
- Request: GET http://localhost:3701/api/staff/tickets/{id}
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, response.sla_name = "Standard", response.sla_id matches Standard SLA id
- Status: [x]

### AC-5: Quick-stats endpoint returns correct overdue count. [API-ONLY]
- Setup: POST /api/dev/reset-tickets to clear all tickets
- Setup: POST /api/dev/seed-tickets with `{"sla":"Urgent","created_hours_ago":5}` (x2 overdue)
- Setup: POST /api/dev/seed-tickets with `{"sla":"Standard"}` (x1 non-overdue)
- Request: GET http://localhost:3701/api/staff/tickets/stats
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, response.overdue = 2
- Status: [x]

## Checklist (children)

- [ ] TS-M3-D1 — Schema: sla_plan table (if not already in M1 schema)
- [ ] TS-M3-D2 — Seed: two SLA plans (Standard 24h, Urgent 4h)
- [ ] TS-M3-D3 — Backend: SLA selection precedence + due-date computation
- [ ] TS-M3-D4 — Backend: overdue detection + flag on listing
- [ ] TS-M3-D5 — Frontend: Overdue tab + due-date/overdue badge display

## Test Infrastructure

- Uses the seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Requires TS-M3-D2 seed (SLA plans).
- Dev endpoints needed:
  - **`POST /api/dev/seed-tickets`** — accepts `sla`, `dept`, `topic`, `created_hours_ago` params
  - **`POST /api/dev/reset-tickets`** — clears all tickets for clean test state
- Relies on close action from EPIC-M3-C for AC-3.

## Dependencies

- **EPIC-M3-A done**: Overdue tab is one of the predefined queues rendered by TS-M3-A4.
- **EPIC-M3-C (close action)**: needed to verify AC-3 (close clears overdue).
- **TS-M3-prep**: help topics with SLA assignment.
