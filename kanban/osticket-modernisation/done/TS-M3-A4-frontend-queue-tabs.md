# TS-M3-A4 — add(ui): FS-020.2 queue tabs UI with counts + status routing

- **ID**: TS-M3-A4
- **Type**: Technical Story
- **Parent**: US-M3-A1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the frontend queue tabs component that displays the predefined queues (Open, Answered,
My Tickets, Overdue, Closed) with count badges, handles tab clicks to update the URL `?status=`
param, and highlights the active tab.

Per FS-020.2:
- **Open**: shown always; count = `open` (+ `answered` if `show_answered_tickets=1`)
- **Answered**: shown only when `show_answered_tickets=0` AND `answered > 0`
- **My Tickets** (`status=assigned`): shown only when `assigned > 0`
- **Overdue** (`status=overdue`): shown only when `overdue > 0`
- **Closed**: shown always

The tabs fetch counts from `GET /api/staff/tickets/stats` and update when the user navigates.

## Impact

- add(ui): `QueueTabs` component rendering the tab bar with MUI Tabs or custom styling.
- add(ui): each tab shows a Badge with the count; zero-count tabs are conditionally hidden.
- update(ui): `TicketQueue` page reads `?status=` from URL and passes it to the API; tab clicks
  update the URL (React Router `useSearchParams` or similar).
- add(ui): the rightmost column header changes based on `rightmost_column` from the API response.
- update(store): MobX store fetches quick stats on mount and after mutations.
- **Note**: The visibility config values (`show_answered_tickets`, `show_assigned_tickets`) are
  included in the `/api/staff/tickets/stats` response so the frontend knows which tabs to
  show/hide without a separate config fetch.

## Regressions

- M1's queue page had no tabs; ensure the default (no status param) still works and shows Open.
- The ticket list table must remain functional; this ticket adds tabs above it.

## Test Infrastructure

- **Frontend**: http://localhost:3702 (Vite dev server must be running)
- **Backend**: http://localhost:3701 (must be running)
- **Credentials**: `agent` / `Agent123!`
- **Dev endpoint needed**: `POST /api/dev/seed-tickets` with configurable status/flags for test setup

## Acceptance Tests

### AC-1: Queue tabs render with counts fetched from /stats endpoint. [BROWSER]
- Setup: `cargo run -p tools --bin seed -- --reset`
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open"},{"status":"open"},{"status":"closed"}]}'`
- Navigate: http://localhost:3702/staff/login
- Action: Type "agent" in username field
- Action: Type "Agent123!" in password field
- Action: Click Login button
- Navigate: http://localhost:3702/staff/tickets
- Verify: Open tab visible with badge showing "2"
- Verify: Closed tab visible with badge showing "1"
- Status: [x]

### AC-2: Clicking a tab updates the URL and filters the listing. [BROWSER]
- Setup: logged in from AC-1, tickets seeded
- Navigate: http://localhost:3702/staff/tickets
- Action: Click the "Closed" tab
- Verify: URL contains `?status=closed`
- Verify: Closed tab has active/highlighted styling
- Verify: Ticket listing shows only the closed ticket(s)
- Status: [x]

### AC-3: Active tab is highlighted based on URL param. [BROWSER]
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","isoverdue":true}]}'`
- Navigate: http://localhost:3702/staff/tickets?status=overdue
- Verify: Overdue tab is visible and has active/highlighted styling
- Status: [x] — Overdue tab visible with orange badge and active underline styling when navigating to ?status=overdue

### AC-4: Default (no status param) activates Open tab. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets
- Verify: URL has no `?status=` param (or is empty)
- Verify: Open tab has active/highlighted styling
- Status: [x]

### AC-5: My Tickets tab is hidden when assigned count is 0. [BROWSER]
- Setup: `cargo run -p tools --bin seed -- --reset` (no assigned tickets)
- Navigate: http://localhost:3702/staff/tickets
- Verify: "My Tickets" tab is NOT visible in the tab bar
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","staff_id":1}]}'`
- Action: Refresh page (F5 or browser refresh)
- Verify: "My Tickets" tab is now visible with badge showing "1"
- Status: [x]

### AC-6: Answered tab visibility depends on show_answered_tickets config. [BROWSER]
- Setup: `psql -U postgres -d osticket_dev -c "UPDATE config SET value='0' WHERE key='show_answered_tickets'"`
- Setup: `curl -X POST http://localhost:3701/api/dev/seed-tickets -H 'Content-Type: application/json' -d '{"tickets":[{"status":"open","isanswered":true}]}'`
- Navigate: http://localhost:3702/staff/tickets
- Verify: "Answered" tab is visible with badge showing "1"
- Setup: `psql -U postgres -d osticket_dev -c "UPDATE config SET value='1' WHERE key='show_answered_tickets'"`
- Action: Refresh page
- Verify: "Answered" tab is NOT visible (answered tickets folded into Open count)
- Status: [x]

### AC-7: Rightmost column header changes per queue. [BROWSER]
- Setup: `psql -U postgres -d osticket_dev -c "UPDATE config SET value='1' WHERE key='show_assigned_tickets'"`
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Table has column header "Assigned To" as the rightmost column
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: Table has column header "Closed By" as the rightmost column
- Status: [x]

### AC-8: Quick stats refresh after a ticket mutation. [BROWSER]
- Setup: seed one open ticket, note the Open count badge value (N)
- Navigate: http://localhost:3702/staff/tickets
- Verify: Open tab shows count N
- Action: Close the ticket via API: `curl -X POST http://localhost:3701/api/staff/tickets/{{ticket_id}}/close -b 'session={{cookie}}'`
- Verify: Open tab count decreases to N-1 (without full page refresh)
- Verify: Closed tab count increases by 1
- Manual: If close endpoint not available, manually trigger via DevTools network tab or wait for TS-M3-C3
- Status: [x]

## Test Infrastructure

- Uses the seeded staff credentials `agent` / `Agent123!`.
- Requires backend routes from TS-M3-A1/A2/A3.
- The AC-8 "close ticket" action may depend on TS-M3-C (close/reopen); stub the UI or test via
  direct API call + manual refresh if needed.

## Dependencies

- **M1 done**: baseline queue UI exists.
- **TS-M3-A1**: status param handling + quick-stats endpoint.
- **TS-M3-A3**: rightmost_column metadata in response.
