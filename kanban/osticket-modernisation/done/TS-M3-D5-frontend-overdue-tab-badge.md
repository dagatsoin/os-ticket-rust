# TS-M3-D5 — add(frontend): FS-020.2 Overdue tab + due-date/overdue badge display

- **ID**: TS-M3-D5
- **Type**: Technical Story
- **Parent**: US-M3-D1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Adds frontend support for the Overdue tab and visual indicators: the Overdue tab with count
badge, a due-date column or tooltip in the listing, and an overdue badge/icon on tickets
that are past due.

## Impact

- update(frontend): queue tabs (from TS-M3-A4) include Overdue tab when `stats.overdue > 0`.
- add(frontend): clicking Overdue tab navigates to `/staff/tickets?status=overdue`.
- add(frontend): listing shows due-date column or tooltip (consider space constraints).
- add(frontend): tickets with `isoverdue=true` display an overdue badge (warning color) in
  the Subject column, similar to the lock icon pattern (FS-020.3).
- add(frontend): ticket detail view shows due date and overdue status.

## Regressions

- Existing queue tabs must continue to work.
- Listing columns for other queues must not be affected (Overdue tab may show different columns).

## Acceptance Tests

### AC-1: FS-020.2 — Overdue tab appears with count when overdue > 0. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"sla":"Urgent","created_hours_ago":5}` -> ticket_id
- Setup: POST /api/dev/mark-overdue/{ticket_id}
- Navigate: http://localhost:3702/staff/tickets
- Verify: Overdue tab is visible in queue tabs
- Verify: Overdue tab shows count badge >= 1
- Action: Click Overdue tab
- Verify: URL changes to `?status=overdue`
- Verify: Listing shows the overdue ticket
- Status: [x]

### AC-2: FS-020.3 — Overdue badge appears on overdue tickets in the listing. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"sla":"Urgent","created_hours_ago":5}` -> ticket_id
- Setup: POST /api/dev/mark-overdue/{ticket_id}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: The overdue ticket row shows visual overdue indicator (badge/icon/colored text) near Subject column
- Status: [x]

### AC-3: FS-021.13 — Due date is displayed for tickets with SLA. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}` (Support has Standard 24h SLA)
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Due date column or tooltip visible on ticket row
- Action: Click ticket to open detail view
- Verify: Due date displayed in ticket detail
- Status: [x]

### AC-4: Ticket detail shows overdue status and due date. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"sla":"Urgent","created_hours_ago":5}` -> ticket_id
- Setup: POST /api/dev/mark-overdue/{ticket_id}
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Verify: Overdue status indicator visible (badge, label, or text)
- Verify: Due date displayed
- Status: [x]

### AC-5: Overdue tab hidden when no overdue tickets. [BROWSER]
- Setup: POST /api/dev/reset-tickets to clear all tickets
- Setup: POST /api/dev/seed-tickets (creates non-overdue ticket)
- Navigate: http://localhost:3702/staff/tickets
- Verify: Overdue tab not displayed OR shows count 0 with muted styling
- Status: [x]

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Dev endpoints needed:
  - **`POST /api/dev/seed-tickets`** — accepts `sla`, `created_hours_ago` params
  - **`POST /api/dev/mark-overdue/{id}`** — sets isoverdue=true for testing
  - **`POST /api/dev/reset-tickets`** — clears all tickets for clean state

## Dependencies

- **TS-M3-A4**: queue tabs infrastructure.
- **TS-M3-D4**: backend returns isoverdue, duedate in listing.
