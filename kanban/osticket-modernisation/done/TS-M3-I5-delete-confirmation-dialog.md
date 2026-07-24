# TS-M3-I5 — add(frontend): FS-021.19 delete confirmation dialog

- **ID**: TS-M3-I5
- **Type**: Technical Story
- **Parent**: EPIC-M3-I
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the frontend delete flow per FS-021.19. The delete button is shown only when staff
has `canDeleteTickets` permission. Clicking it shows a confirmation dialog warning that deletion
is permanent. On confirm, calls DELETE endpoint and redirects to queue.

## Impact

- update(component): `TicketView` — add Delete button (conditional on canDeleteTickets).
- add(component): `DeleteConfirmDialog` — modal with warning "This action is irreversible. The
  ticket and all attachments will be permanently deleted." + Confirm/Cancel buttons.
- add(api): `apiClient.deleteTicket(ticketId)` — calls DELETE /api/staff/tickets/{ticketId}.
- update(routing): on successful delete, redirect to /staff/tickets with success message.

## Regressions

- Queue listing must reflect deleted ticket immediately (removed from list).

## Acceptance Tests

### AC-1: Delete button shown only for canDeleteTickets staff. [BROWSER]
- Setup: POST /api/dev/reset-db
- Setup: POST /api/dev/seed-staff → {username: "agent", password: "Agent123!", can_delete_tickets: true}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [100]}
- Navigate: http://localhost:3702/staff/login
- Action: Type "agent" in username field
- Action: Type "Agent123!" in password field
- Action: Click "Sign In" button
- Navigate: http://localhost:3702/staff/tickets/100
- Verify: "Delete" button is visible in action bar
- Setup: POST /api/dev/update-staff → {username: "agent", can_delete_tickets: false}
- Action: Press F5 to refresh
- Verify: "Delete" button is NOT visible
- Status: [x]

### AC-2: Clicking Delete shows confirmation dialog. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [101]}
- Navigate: http://localhost:3702/staff/tickets/101
- Action: Click "Delete" button
- Verify: Modal dialog appears
- Verify: Modal contains text "This action is irreversible"
- Verify: Modal contains text "permanently deleted"
- Verify: Modal has "Delete" button (danger/red style)
- Verify: Modal has "Cancel" button
- Status: [x]

### AC-3: Cancel closes dialog without action. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [102]}
- Navigate: http://localhost:3702/staff/tickets/102
- Action: Click "Delete" button
- Verify: Modal dialog appears
- Action: Click "Cancel" button
- Verify: Modal closes
- Verify: Ticket view is still displayed
- Verify: No DELETE network request was made
- Status: [x]

### AC-4: Confirm deletes ticket and redirects to queue. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open", ticket_number: "DEL100"}] → {ids: [103]}
- Navigate: http://localhost:3702/staff/tickets/103
- Action: Click "Delete" button
- Verify: Modal dialog appears
- Action: Click "Delete" or "Confirm" button in modal
- Verify: Network shows DELETE request to /api/staff/tickets/103
- Verify: Page redirects to /staff/tickets (queue listing)
- Verify: Success snackbar displays "Ticket #DEL100 deleted successfully"
- Verify: Ticket #103 does not appear in queue listing
- Status: [x]

### AC-5: Delete error shows error message. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [104]}
- Setup: POST /api/dev/simulate-error → {endpoint: "DELETE /api/staff/tickets/104", status: 500}
- Navigate: http://localhost:3702/staff/tickets/104
- Action: Click "Delete" button
- Action: Click "Delete" or "Confirm" button in modal
- Verify: Error snackbar appears with error message
- Verify: Modal closes
- Verify: Ticket view is still displayed (ticket not deleted)
- Status: [x]

### AC-6: Dialog is keyboard accessible. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [105]}
- Navigate: http://localhost:3702/staff/tickets/105
- Action: Click "Delete" button
- Verify: Modal dialog appears
- Action: Press Tab key
- Verify: Focus moves to "Cancel" button
- Action: Press Tab key again
- Verify: Focus moves to "Delete" button
- Action: Press Escape key
- Verify: Modal closes without deleting
- Verify: Ticket view is still displayed
- Status: [x]

## Test Infrastructure

**Dev Endpoints Required (Setup only):**
- `POST /api/dev/reset-db` — reset to clean state
- `POST /api/dev/seed-staff` — create/update staff with permission flags
- `POST /api/dev/update-staff` — update existing staff permissions
- `POST /api/dev/seed-tickets` — create tickets; returns `{ids: [...]}`
- `POST /api/dev/simulate-error` — (optional) inject API error for testing error handling

## Dependencies

- **TS-M3-I2**: backend delete endpoint.
- **EPIC-M3-A**: ticket view page structure.
- **TS-M3-prep**: delete permission flag.
