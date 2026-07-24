# TS-M3-C4 — add(ui): EPIC-M3-C workflow action buttons + dialogs

- **ID**: TS-M3-C4
- **Type**: Technical Story
- **Parent**: US-M3-C1, US-M3-C2
- **Labels**: Technical Story, M3
- **Scope**: medium

## Context

This technical story supports both US-M3-C1 (claims/assigns/transfers) and US-M3-C2 (closes/reopens).
Implements the frontend UI for all workflow actions: Claim button, Assign dialog, Transfer dialog,
Close button/dialog, and Reopen button/dialog. Each button's visibility is conditional on ticket
state and staff permissions.

## Impact

- add(ui): **Claim** button on ticket detail
  - Visible: ticket is open AND unassigned AND staff has can_assign_tickets
  - Action: POST /api/staff/tickets/{id}/claim, show success toast, refresh ticket state

- add(ui): **Assign** button + dialog on ticket detail
  - Visible: ticket is open OR closed (assigning closed ticket reopens) AND staff has can_assign_tickets
  - Dialog: staff/team selector (dropdown or autocomplete), comments textarea (required, min 5 chars)
  - Assignee format: staff shown as "s{id}" internally, teams as "t{id}"
  - Action: POST /api/staff/tickets/{id}/assign, on success navigate to queue (non-claim)

- add(ui): **Transfer** button + dialog on ticket detail
  - Visible: staff has can_transfer_tickets
  - Dialog: department dropdown (excluding current), comments textarea (required, min 5 chars)
  - Action: POST /api/staff/tickets/{id}/transfer, on success show message, handle access_lost redirect

- add(ui): **Close** button + optional comment dialog on ticket detail
  - Visible: ticket is open AND staff has can_close_tickets
  - Dialog: optional comments textarea
  - Action: POST /api/staff/tickets/{id}/close, on success navigate to queue

- add(ui): **Reopen** button + optional comment dialog on ticket detail
  - Visible: ticket is closed AND (staff has can_close_tickets OR can_create_tickets)
  - Dialog: optional comments textarea
  - Action: POST /api/staff/tickets/{id}/reopen, on success refresh ticket state

- update(ui): ticket detail view
  - Show workflow action buttons in a toolbar or dropdown menu
  - Display "Assigned To" field (staff name or team name or "Unassigned")
  - Display "Closed By" field on closed tickets
  - Fetch permissions from session/API to conditionally render buttons

- add(store): workflow action methods in MobX store
  - claimTicket(id), assignTicket(id, assignee, comments), transferTicket(id, deptId, comments)
  - closeTicket(id, comments), reopenTicket(id, comments)
  - Handle API calls, loading states, error display

## Regressions

- The M1/M2 ticket detail view (thread display, reply composer) must remain functional.
- The reply flow must not be blocked by the new toolbar.

## Acceptance Tests

### AC-1: Claim button is visible for open unassigned ticket with permission. [BROWSER]
- Setup: POST /api/dev/reset
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: click on the unassigned ticket to view detail
- Verify: "Claim" button is visible in the action toolbar
- Status: [x]

### AC-2: Claim button is hidden for assigned ticket. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":"s1"} -> {ticket_id}
- Navigate: open the assigned ticket detail view
- Verify: "Claim" button is NOT visible
- Status: [x]

### AC-3: Claim button is hidden for closed ticket. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed"} -> {ticket_id}
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Action: click on the closed ticket to view detail
- Verify: "Claim" button is NOT visible
- Status: [x]

### AC-4: Clicking Claim calls API and shows success message. [BROWSER]
- Depends: AC-1 (on unassigned ticket detail)
- Action: click "Claim" button
- Verify: success toast "Ticket is now assigned to you!"
- Verify: "Assigned To" field updates to show "agent"
- Status: [x]

### AC-5: Assign button opens dialog with staff/team selector. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Navigate: open the ticket detail view
- Action: click "Assign" button
- Verify: dialog opens with dropdown/autocomplete listing staff and teams
- Verify: dialog has comments textarea
- Status: [x]

### AC-6: Assign dialog validates comment length. [BROWSER]
- Depends: AC-5 (dialog open)
- Action: select any staff from dropdown
- Action: type "Hi" in comments (3 chars)
- Action: click Submit
- Verify: validation error "Comment too short" or "min 5 chars"
- Verify: dialog stays open
- Status: [x] — Dialog shows "Required, minimum 5 characters" helper text; ASSIGN button disabled until valid

### AC-7: Assign dialog submits and navigates to queue on success. [BROWSER]
- Depends: AC-5 (dialog open)
- Action: select a staff from dropdown
- Action: type "Routing to specialist" in comments (19 chars)
- Action: click Submit
- Verify: success message displayed
- Verify: navigated back to /staff/tickets queue
- Status: [x]

### AC-8: Transfer button opens dialog with department selector. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","dept":"Support"} -> {ticket_id}
- Navigate: open the ticket detail view
- Action: click "Transfer" button
- Verify: dialog opens with department dropdown
- Verify: current department (Support) is excluded or disabled
- Verify: dialog has comments textarea
- Status: [x]

### AC-9: Transfer dialog validates same-department and comment length. [BROWSER]
- Depends: AC-8 (dialog open)
- Action: attempt to select same department (Support) if selectable
- Verify: error "Ticket already in the department" or selection disabled
- Action: select different department (Sales)
- Action: type "abc" in comments (3 chars)
- Action: click Submit
- Verify: validation error "Transfer comments too short!"
- Status: [x] — Dialog shows "Required, minimum 5 characters" helper text; TRANSFER button disabled until valid

### AC-10: Transfer dialog submits and handles access_lost. [BROWSER]
- Depends: AC-8 (dialog open)
- Action: select "Sales" department (agent has access)
- Action: type "Billing issue" in comments (13 chars)
- Action: click Submit
- Verify: success message "Ticket transferred successfully to Sales"
- Verify: department field updates to "Sales"
- Status: [x]

### AC-11: Close button is visible for open ticket with permission. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open"} -> {ticket_id}
- Navigate: open the ticket detail view
- Verify: "Close" button is visible in action toolbar
- Status: [x]

### AC-12: Close button is hidden for closed ticket. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed"} -> {ticket_id}
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Action: click on the closed ticket to view detail
- Verify: "Close" button is NOT visible
- Status: [x]

### AC-13: Clicking Close calls API and navigates to queue. [BROWSER]
- Depends: AC-11 (on open ticket with Close button visible)
- Action: click "Close" button
- Verify: optional comment dialog appears (or direct confirm)
- Action: optionally enter comment, confirm
- Verify: success message displayed
- Verify: navigated back to /staff/tickets queue
- Status: [x]

### AC-14: Reopen button is visible for closed ticket with permission. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed"} -> {ticket_id}
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Action: click on the closed ticket to view detail
- Verify: "Reopen" button is visible in action toolbar
- Status: [x]

### AC-15: Reopen button is hidden for open ticket. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open"} -> {ticket_id}
- Navigate: open the ticket detail view
- Verify: "Reopen" button is NOT visible
- Status: [x]

### AC-16: Clicking Reopen calls API and refreshes ticket state. [BROWSER]
- Depends: AC-14 (on closed ticket with Reopen button visible)
- Action: click "Reopen" button
- Verify: optional comment dialog appears
- Action: optionally enter comment, confirm
- Verify: success message "Ticket REOPENED"
- Verify: ticket status indicator changes to "Open"
- Verify: agent stays on ticket detail view
- Status: [x]

### AC-17: Buttons hidden when staff lacks permission. [BROWSER]
- Setup: POST /api/dev/set-group-perm {"group_id":1,"can_assign_tickets":false}
- Navigate: open an unassigned open ticket detail
- Verify: "Claim" button NOT visible
- Verify: "Assign" button NOT visible
- Setup: POST /api/dev/set-group-perm {"group_id":1,"can_assign_tickets":true,"can_transfer_tickets":false}
- Navigate: refresh ticket detail
- Verify: "Transfer" button NOT visible
- Setup: POST /api/dev/set-group-perm {"group_id":1,"can_transfer_tickets":true,"can_close_tickets":false,"can_create_tickets":false}
- Navigate: refresh ticket detail
- Verify: "Close" button NOT visible
- Verify: "Reopen" button NOT visible (on closed ticket)
- Cleanup: POST /api/dev/set-group-perm {"group_id":1,"can_close_tickets":true,"can_create_tickets":true}
- Status: [x]

## Test Infrastructure

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose | Request Shape |
|----------|---------|---------------|
| `POST /api/dev/reset` | Reset DB to seed state | `{}` |
| `POST /api/dev/seed-tickets` | Create test tickets | `{"count":N,"status":"...","assigned":bool\|"s{id}","dept":"..."}` |
| `POST /api/dev/set-group-perm` | Toggle group permissions | `{"group_id":N,"can_*":bool}` |
| `POST /api/auth/login` | Authenticate for browser session | `{"username":"...","password":"..."}` |

## Dependencies

- **M1 done**: ticket detail view, staff auth, MobX stores.
- **TS-M3-C1**: assign/claim API routes.
- **TS-M3-C2**: transfer API route.
- **TS-M3-C3**: close/reopen API routes.
- **TS-M3-prep**: staff list, team list, department list for selectors.
