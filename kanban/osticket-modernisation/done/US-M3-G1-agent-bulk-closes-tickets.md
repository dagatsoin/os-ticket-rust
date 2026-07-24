# US-M3-G1 — Agent bulk-closes, reopens, or deletes multiple tickets

- **ID**: US-M3-G1
- **Type**: User Story
- **Parent**: EPIC-M3-G
- **Labels**: User Story, M3
- **Scope**: medium

## Spec References

- FS-020.9 (mass / bulk actions from the queue)
- FS-021.21 (mass ticket actions handler)
- BS-020.9 (mass-management gating: admin OR can-delete OR can-close)
- BS-020.10 (bulk action set depends on queue)
- BS-020.11 (per-action permission re-check on bulk)
- EC-020.7 (partial bulk action)
- EC-020.8 (bulk action without permission)
- EC-020.9 (no tickets selected for bulk)
- EC-020.13 (unknown bulk action)

## Context

Epic: EPIC-M3-G — Bulk Actions.
M1/M2 had no bulk operations. This story adds the ability to select multiple tickets in any queue
and apply a single action (close, reopen, delete) to all of them at once.

## Description

As a support agent with close/delete permissions, I can select multiple tickets via checkboxes,
then click a bulk action button to close, reopen, or delete all selected tickets. The system
checks permission per action and reports partial success if some tickets fail.

## Impact

- Frontend (checkboxes, select-all, bulk action bar with queue-appropriate buttons)
- Backend (mass_process endpoint with per-action permission re-check)
- Database (batch status updates, deletions)
- Browser (desktop)

## Business Rules

- BS-020.9: Checkboxes and bulk actions are shown only when staff canManageTickets (admin OR
  can-delete OR can-close).
- BS-020.10: Available actions depend on queue: Closed -> Reopen; Open/Answered/Assigned -> Close;
  Overdue -> Close; Search -> Close + Reopen. Delete offered everywhere when staff can-delete.
- BS-020.11: Each bulk action independently re-checks its specific permission (reopen needs
  close-or-create; close needs close; delete needs delete) and reports partial success.

## Regressions

- Individual ticket actions (close, reopen, delete) from M3-C must still work.
- Queue listing must still render correctly when user lacks mass-manage permission (no checkboxes).

## Acceptance Criteria

### AC-1: Checkboxes appear only for staff with canManageTickets. [BROWSER]
- Setup: POST /api/dev/reset-db
- Setup: POST /api/dev/seed-staff → {username: "agent", password: "Agent123!", can_close_tickets: true, can_delete_tickets: true}
- Navigate: http://localhost:3702/staff/login
- Action: Type "agent" in username field
- Action: Type "Agent123!" in password field
- Action: Click "Sign In" button
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Each ticket row has a leading checkbox input element
- Verify: Table header contains a select-all checkbox
- Verify: Bulk action bar is visible below header
- Setup: POST /api/dev/update-staff → {username: "agent", can_close_tickets: false, can_delete_tickets: false, isadmin: false}
- Action: Press F5 to refresh the page
- Verify: No checkbox inputs are rendered in ticket rows
- Verify: Bulk action bar is not visible
- Status: [x]

### AC-2: Bulk action bar shows queue-appropriate actions. [BROWSER]
- Setup: POST /api/dev/seed-staff → {username: "agent", can_close_tickets: true, can_delete_tickets: true}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}, {status: "closed"}]
- Navigate: http://localhost:3702/staff/login
- Action: Log in as agent / Agent123!
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: Bulk action bar contains "Reopen" button
- Verify: Bulk action bar contains "Delete" button
- Verify: Bulk action bar does NOT contain "Close" button
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Bulk action bar contains "Close" button
- Verify: Bulk action bar contains "Delete" button
- Verify: Bulk action bar does NOT contain "Reopen" button
- Navigate: http://localhost:3702/staff/tickets?status=overdue
- Verify: Bulk action bar contains "Close" button
- Verify: Bulk action bar contains "Delete" button
- Status: [x]

### AC-3: Select-all/toggle-all helper works. [BROWSER]
- Setup: POST /api/dev/seed-tickets → 5 open tickets
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: At least 5 ticket rows are visible
- Action: Click the header select-all checkbox
- Verify: All row checkboxes are now checked (checked attribute present)
- Action: Click the header select-all checkbox again
- Verify: All row checkboxes are unchecked
- Status: [x]

### AC-4: Bulk close succeeds and reports count. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open", subject: "Bulk Test 1"}, {status: "open", subject: "Bulk Test 2"}, {status: "open", subject: "Bulk Test 3"}] → {ids: [101, 102, 103]}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: Click checkbox for ticket "Bulk Test 1"
- Action: Click checkbox for ticket "Bulk Test 2"
- Action: Click checkbox for ticket "Bulk Test 3"
- Action: Click "Close" button in bulk action bar
- Verify: Confirmation dialog appears with text mentioning "3 tickets"
- Action: Click "Confirm" button
- Verify: Success snackbar shows "3 tickets closed"
- Verify: The 3 tickets no longer appear in the listing
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: Tickets "Bulk Test 1", "Bulk Test 2", "Bulk Test 3" appear in closed queue
- Status: [x]

### AC-5: Bulk reopen succeeds for closed tickets. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "closed", subject: "Reopen Test 1"}, {status: "closed", subject: "Reopen Test 2"}]
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Action: Click checkbox for ticket "Reopen Test 1"
- Action: Click checkbox for ticket "Reopen Test 2"
- Action: Click "Reopen" button in bulk action bar
- Verify: Confirmation dialog appears
- Action: Click "Confirm" button
- Verify: Success snackbar shows "2 tickets reopened"
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Tickets "Reopen Test 1" and "Reopen Test 2" appear in open queue
- Status: [x]

### AC-6: Bulk delete succeeds with warning. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open", subject: "Delete Test 1"}, {status: "open", subject: "Delete Test 2"}]
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: Click checkbox for ticket "Delete Test 1"
- Action: Click checkbox for ticket "Delete Test 2"
- Action: Click "Delete" button in bulk action bar
- Verify: Confirmation dialog appears with warning about permanent deletion
- Action: Click "Delete" or "Confirm" button
- Verify: Success snackbar shows "2 tickets deleted"
- Verify: Tickets "Delete Test 1" and "Delete Test 2" are gone from the listing
- Status: [x]

### AC-7: Partial success — some tickets fail. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open", subject: "Partial 1"}, {status: "open", subject: "Partial 2"}, {status: "closed", subject: "Partial 3"}] → {ids: [201, 202, 203]}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: Click checkbox for ticket "Partial 1"
- Action: Click checkbox for ticket "Partial 2"
- Setup: POST /api/dev/inject-selection → {ticket_ids: [201, 202, 203]} (to include closed ticket in selection)
- Action: Click "Close" button in bulk action bar
- Action: Click "Confirm" button
- Verify: Snackbar shows partial success "2 of 3 tickets closed"
- Status: [x]

### AC-8: No tickets selected error. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: No checkboxes are checked
- Verify: "Close" button is disabled OR action triggers error
- Action: Attempt to click "Close" button
- Verify: Either button is disabled (no action) OR error message "No tickets selected" appears
- Status: [x]

## Checklist (children)

- [ ] TS-M3-G1 — Backend: mass_process endpoint + per-action permission
- [ ] TS-M3-G2 — Frontend: checkboxes + bulk action bar UI

## Test Infrastructure

**Credentials:**
- Primary: `agent` / `Agent123!` (TS-M1-A3 seed)

**Dev Endpoints Required (Setup only, NOT validation):**
- `POST /api/dev/reset-db` — reset to clean state
- `POST /api/dev/seed-staff` — create/update staff with specific permission flags
  - Body: `{username, password, can_close_tickets, can_delete_tickets, isadmin}`
- `POST /api/dev/update-staff` — update existing staff permissions
- `POST /api/dev/seed-tickets` — create tickets with specified status
  - Body: `[{status, subject}]`
  - Returns: `{ids: [...]}`
- `POST /api/dev/inject-selection` — (optional) inject ticket IDs into selection for edge case testing

**Seed Expansion Required (TS-M3-prep):**
- `can_close_tickets` permission flag on groups
- `can_delete_tickets` permission flag on groups

## Dependencies

- **EPIC-M3-A**: queue listing with ticket rows to select.
- **EPIC-M3-C**: single-ticket close/reopen logic (reused by bulk handler).
- **TS-M3-prep**: seed expansion for `can_close_tickets`, `can_delete_tickets` flags.

## Review feedback

Returned from M3 root E2E (pre-merge review failure). AC-4 reset to `[ ]`.

- **Root AC-9 (US-M3-G1) FAIL** — Bulk close is functionally correct (2 tickets moved to
  Closed, badges updated, MUI confirm dialog "Are you sure you want to close 2 ticket(s)?"
  works), BUT the success toast renders literally "undefined tickets closed" instead of the
  count / "Selected tickets closed". This is a broken string interpolation in the bulk-close
  success handler (the count/label value resolves to `undefined`). Fix the success-message
  interpolation so the toast shows the closed count (e.g. "3 tickets closed" / "Selected
  tickets closed") and re-verify AC-4's "Success snackbar shows '3 tickets closed'".
