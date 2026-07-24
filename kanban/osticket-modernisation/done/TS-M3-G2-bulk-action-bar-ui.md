# TS-M3-G2 — add(frontend): BS-020.9 checkboxes + bulk action bar UI

- **ID**: TS-M3-G2
- **Type**: Technical Story
- **Parent**: US-M3-G1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Adds the frontend bulk selection and action UI per FS-020.9. Per-row checkboxes are rendered
only when the staff member canManageTickets (BS-020.9). A header select-all checkbox toggles
all rows. The bulk action bar shows queue-appropriate buttons (BS-020.10): Close on open queues,
Reopen on closed queue, Delete everywhere (when staff can-delete).

Clicking a bulk action button shows a confirmation dialog, then POSTs to `POST /api/staff/tickets/bulk`.
The response is displayed as a success/partial/error snackbar.

## Impact

- add(component): `BulkActionBar` — renders buttons based on current queue status + staff permissions.
- update(component): `TicketListRow` — render leading checkbox when `canManageTickets`.
- update(component): `TicketListHeader` — render select-all checkbox.
- add(state): `TicketQueueStore.selectedTicketIds: Set<number>` — tracks checked tickets.
- add(api): `apiClient.bulkTicketAction(action, ticketIds)` — calls TS-M3-G1 endpoint.

## Regressions

- Queue listing layout must remain unchanged when bulk UI is hidden.
- Clicking a ticket row (not checkbox) still navigates to ticket detail.

## Acceptance Tests

### AC-1: BS-020.9 — checkboxes render only for canManageTickets staff. [BROWSER]
- Setup: POST /api/dev/reset-db
- Setup: POST /api/dev/seed-staff → {username: "agent", password: "Agent123!", can_close_tickets: true, can_delete_tickets: true}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}, {status: "open"}]
- Navigate: http://localhost:3702/staff/login
- Action: Type "agent" in username field
- Action: Type "Agent123!" in password field
- Action: Click "Sign In" button
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Each ticket row has a checkbox input element
- Verify: Header row contains select-all checkbox
- Verify: Bulk action bar is visible
- Setup: POST /api/dev/update-staff → {username: "agent", can_close_tickets: false, can_delete_tickets: false, isadmin: false}
- Action: Press F5 to refresh
- Verify: No checkbox inputs in ticket rows
- Verify: No bulk action bar visible
- Status: [x]

### AC-2: BS-020.10 — bulk bar shows Close on open queue, Reopen on closed queue. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}, {status: "closed"}]
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Bulk action bar contains "Close" button
- Verify: Bulk action bar does NOT contain "Reopen" button
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: Bulk action bar contains "Reopen" button
- Verify: Bulk action bar does NOT contain "Close" button
- Status: [x]

### AC-3: Select-all toggles all checkboxes. [BROWSER]
- Setup: POST /api/dev/seed-tickets → 3 open tickets
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: At least 3 ticket rows visible
- Action: Click header select-all checkbox
- Verify: All row checkboxes are checked
- Action: Click header select-all checkbox again
- Verify: All row checkboxes are unchecked
- Status: [x]

### AC-4: Bulk close flow — confirmation, POST, success message. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open", subject: "Close Flow 1"}, {status: "open", subject: "Close Flow 2"}]
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: Click checkbox for ticket "Close Flow 1"
- Action: Click checkbox for ticket "Close Flow 2"
- Action: Click "Close" button in bulk action bar
- Verify: Confirmation dialog appears
- Action: Click "Confirm" button
- Verify: Network request POST /api/staff/tickets/bulk with action=close
- Verify: Success snackbar shows "2 tickets closed"
- Verify: Tickets "Close Flow 1" and "Close Flow 2" removed from listing
- Status: [x]

### AC-5: Bulk delete flow — permanent deletion warning. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open", subject: "Delete Flow 1"}, {status: "open", subject: "Delete Flow 2"}]
- Setup: POST /api/dev/seed-staff → {username: "agent", can_delete_tickets: true}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: Click checkbox for ticket "Delete Flow 1"
- Action: Click checkbox for ticket "Delete Flow 2"
- Action: Click "Delete" button in bulk action bar
- Verify: Confirmation dialog contains text "permanently deleted" or "cannot be recovered"
- Action: Click "Confirm" or "Delete" button
- Verify: Success snackbar shows "2 tickets deleted"
- Status: [x]

### AC-6: EC-020.9 — action button disabled when no tickets selected. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: No checkboxes are currently checked
- Verify: "Close" button has disabled attribute OR is visually disabled
- Verify: "Delete" button has disabled attribute OR is visually disabled
- Status: [x]

### AC-7: Partial success — snackbar shows "2 of 3 tickets closed". [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}, {status: "open"}, {status: "closed"}] → {ids: [101, 102, 103]}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: Select tickets with IDs 101, 102
- Setup: POST /api/dev/inject-selection → {ticket_ids: [101, 102, 103]} (include closed ticket)
- Action: Click "Close" button in bulk action bar
- Action: Click "Confirm" button
- Verify: Snackbar shows "2 of 3 tickets closed"
- Status: [x]

## Test Infrastructure

**Dev Endpoints Required (Setup only):**
- `POST /api/dev/reset-db` — reset to clean state
- `POST /api/dev/seed-staff` — create/update staff with permission flags
- `POST /api/dev/update-staff` — update existing staff permissions
- `POST /api/dev/seed-tickets` — create tickets with specified status/subject
- `POST /api/dev/inject-selection` — inject ticket IDs for edge case testing

## Dependencies

- **TS-M3-G1**: backend endpoint `POST /api/staff/tickets/bulk`.
- **EPIC-M3-A**: queue listing with TicketListRow and TicketListHeader components.
- **TS-M3-prep**: staff group flags for canManageTickets evaluation.
