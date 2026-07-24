# US-M3-C2 — Agent closes and reopens a ticket

- **ID**: US-M3-C2
- **Type**: User Story
- **Parent**: EPIC-M3-C
- **Labels**: User Story, M3
- **Scope**: small

## Spec References

- FS-021.11 (close ticket)
- FS-021.12 (reopen ticket)
- BS-021.4 (closing clears overdue/due date, credits closer)
- BS-021.14 (reopen annuls prior close event for statistics)
- EC-021.8 (redundant status action rejected)

## Context

Epic: EPIC-M3-C — Close/Reopen/Assign Workflow.
M1 delivered only reply; tickets couldn't be closed. This story adds the close/reopen lifecycle:
an agent can close an open ticket (with optional comment), and reopen a closed ticket.

## Description

As an agent, I can close an open ticket when the issue is resolved, and reopen a closed ticket
if it needs further work. Closing clears the overdue flag, credits me as the closer, and returns
me to the queue. Reopening marks the ticket unanswered and annuls the prior close event.

## Impact

- Frontend (Close button on open tickets, Reopen button on closed tickets, optional comment dialog)
- Backend (close/reopen routes, status change, lifecycle events, internal notes)
- Database (status, isoverdue, duedate, staff_id as closer, ticket_event)
- Browser (desktop)

## Business Rules

- BS-021.1: `canCloseTickets` required for close; `canCloseTickets` OR `canCreateTickets` for reopen.
- BS-021.4: Closing sets status=closed, clears overdue flag, nulls due date, stamps close time, sets staff_id to closer.
- BS-021.14: Reopening records a `reopened` event and annuls the prior `closed` event.
- FS-021.12: Reopening sets status=open, stamps reopened time, sets answered=0 (unanswered).
- EC-021.8: Closing an already-closed ticket shows "Ticket is already closed!"; reopening an already-open shows "Ticket is already open!".

## Regressions

- The M1 ticket detail view must still work after adding Close/Reopen buttons.
- The Close/Reopen actions must not interfere with the reply flow.

## Acceptance Criteria

### AC-1: Agent closes an open ticket. [BROWSER]
- Setup: POST /api/dev/reset -> reset DB to seed state
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open"} -> {ticket_id, ticket_number}
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: click on the open ticket to view detail
- Verify: "Close" button is visible in action toolbar
- Action: click "Close"
- Verify: optional comment dialog appears (or direct confirm)
- Action: optionally enter "Issue resolved", confirm
- Verify: success message "Ticket #{ticket_number} status set to CLOSED"
- Verify: navigated back to /staff/tickets queue
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: the ticket appears in the Closed queue
- Status: [x]

### AC-2: Closed ticket shows Reopen button; Open ticket does not. [BROWSER]
- Depends: AC-1 (ticket is now closed)
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Action: click on the closed ticket to view detail
- Verify: "Reopen" button is visible
- Verify: "Close" button is NOT visible
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: click on an open ticket to view detail
- Verify: "Close" button is visible
- Verify: "Reopen" button is NOT visible
- Status: [x]

### AC-3: Agent reopens a closed ticket. [BROWSER]
- Depends: AC-1 (ticket is closed)
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Action: click on the closed ticket to view detail
- Action: click "Reopen"
- Verify: optional comment dialog appears
- Action: optionally enter "Needs follow-up", confirm
- Verify: success message "Ticket REOPENED"
- Verify: ticket status indicator changes to "Open"
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: the ticket appears in the Open queue
- Status: [x]

### AC-4: Reopening marks the ticket as unanswered. [BROWSER]
- Depends: AC-3 (ticket just reopened)
- Verify: ticket detail shows "Unanswered" badge or indicator (answered=false)
- Status: [x]

### AC-5: Closing clears overdue flag. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","isoverdue":true} -> {ticket_id}
- Navigate: open the overdue ticket detail view
- Verify: "Overdue" badge is displayed
- Action: click "Close", confirm
- Verify: ticket is closed
- Action: click "Reopen" (or navigate to closed, reopen)
- Verify: "Overdue" badge is NOT shown (overdue cleared on close)
- Status: [x]

### AC-6: Redundant close/reopen shows appropriate error. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed"} -> {ticket_id}
- Navigate: open the closed ticket detail view
- Verify: "Close" button is NOT visible (correct UI behavior)
- Manual: if testing via API - POST /api/staff/tickets/{ticket_id}/close
- Verify: error "Ticket is already closed!" (API returns 422)
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open"} -> {ticket_id2}
- Navigate: open the open ticket detail view
- Verify: "Reopen" button is NOT visible (correct UI behavior)
- Manual: if testing via API - POST /api/staff/tickets/{ticket_id2}/reopen
- Verify: error "Ticket is already open!" (API returns 422)
- Status: [x]

## Checklist (children)

- [ ] TS-M3-C3 — Backend: close/reopen routes + lifecycle events + internal notes
- [ ] TS-M3-C4c — Frontend: Close/Reopen buttons + optional comment dialog

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- The agent's group must have `can_close_tickets=1` (seeded by TS-M3-prep).

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose | Request Shape |
|----------|---------|---------------|
| `POST /api/dev/reset` | Reset DB to seed state | `{}` |
| `POST /api/dev/seed-tickets` | Create test tickets | `{"count":N,"status":"open"\|"closed","isoverdue":bool}` |
| `POST /api/auth/login` | Authenticate for browser session | `{"username":"agent","password":"Agent123!"}` |

## Dependencies

- **M1 done**: provides the baseline ticket infrastructure.
- **TS-M3-prep**: seed expansion (permission flags).
- **EPIC-M3-D** (SLA/overdue): provides the overdue flag infrastructure for AC-5.
