# US-M3-C1 — Agent claims, assigns, and transfers a ticket

- **ID**: US-M3-C1
- **Type**: User Story
- **Parent**: EPIC-M3-C
- **Labels**: User Story, M3
- **Scope**: medium

## Spec References

- FS-021.7 (assign / reassign ticket)
- FS-021.8 (claim ticket — self-assign)
- FS-021.9 (release / unassign ticket — manager-only, deferred to M4)
- FS-021.10 (transfer between departments)
- BS-021.5 (assignment/transfer reopens closed tickets)
- BS-021.8 (assignment/transfer require >= 5 char comments; claims don't)
- BS-021.9 (no self-assignment alerts)

## Context

Epic: EPIC-M3-C — Close/Reopen/Assign Workflow.
M1 delivered only reply; tickets couldn't be assigned or transferred. This story adds assignment
routing: an agent can claim (self-assign) an unassigned ticket, assign it to another staff or
team, or transfer it to a different department. Release (unassign) requires department-manager
status, which is deferred to M4.

## Description

As an agent, I can claim an unassigned ticket to work it myself, assign it to another staff
member or team (with a required comment), and transfer it to another department (with a required
comment). If I assign/transfer a closed ticket, it reopens automatically.

## Impact

- Frontend (Claim button, Assign dialog with staff/team picker, Transfer dialog with dept picker)
- Backend (assign/claim/transfer routes, validation, internal notes, lifecycle events)
- Database (staff_id, team_id, dept_id on ticket; ticket_event for transferred)
- Browser (desktop)

## Business Rules

- BS-021.1: `canAssignTickets` required for assign/claim; `canTransferTickets` for transfer.
- BS-021.5: Assigning or transferring a closed ticket reopens it.
- BS-021.8: Assignment and transfer require comments >= 5 chars; claims default to "Ticket claimed by <name>".
- BS-021.9: No assignment alert on self-assignment (claim).
- FS-021.7: Reassigning to the same staff/team is rejected ("Ticket already assigned to...").
- FS-021.10: Transferring to the same department is rejected ("Ticket already in the department").
- FS-021.10: Transfer re-selects SLA based on the new department (SLA precedence per FS-021.13).
- FS-021.10: After transfer, access is re-checked; if the agent loses access, they return to queue.

## Regressions

- The M1 ticket detail view must still work after adding the Claim/Assign/Transfer buttons.
- Staff auth must still be enforced on all actions.

## Acceptance Criteria

### AC-1: Agent claims an unassigned open ticket. [BROWSER]
- Setup: POST /api/dev/reset -> reset DB to seed state
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: click on the unassigned ticket row to open detail view
- Verify: "Claim" button is visible in action toolbar
- Action: click "Claim"
- Verify: success toast "Ticket is now assigned to you!"
- Verify: "Assigned To" field shows "agent" (current user)
- Verify: agent remains on ticket detail view
- Status: [x]

### AC-2: Claim button is hidden when the ticket is already assigned or closed. [BROWSER]
- Depends: AC-1 (ticket now assigned to agent)
- Verify: "Claim" button is NOT visible (ticket is assigned)
- Action: click "Close" button, confirm
- Verify: ticket status changes to closed
- Verify: "Claim" button is NOT visible (ticket is closed)
- Status: [x]

### AC-3: Agent assigns a ticket to another staff member with a comment. [BROWSER]
- Setup: POST /api/dev/reset
- Setup: POST /api/dev/seed-staff {"name":"Bob Agent"} -> {bob_id}
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Action: click on the unassigned ticket to open detail view
- Action: click "Assign" button
- Verify: dialog opens with staff/team dropdown and comments textarea
- Action: select "Bob Agent" from dropdown
- Action: type "Routing to specialist" in comments (16 chars)
- Action: click Submit/OK
- Verify: success message "Ticket assigned successfully"
- Verify: navigated back to /staff/tickets queue
- Status: [x]

### AC-4: Assignment fails if comment is too short. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","assigned":false} -> {ticket_id}
- Navigate: open the unassigned ticket detail view
- Action: click "Assign" button
- Action: select any staff/team
- Action: type "Hi" in comments (3 chars < 5 required)
- Action: click Submit
- Verify: validation error "Comment too short" or "Assignment comments required"
- Verify: dialog stays open, form not submitted
- Status: [x]

### AC-5: Agent transfers a ticket to another department with a comment. [BROWSER]
- Setup: POST /api/dev/reset (seed has Support, Sales depts)
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"open","dept":"Support"} -> {ticket_id}
- Navigate: open the ticket in Support department
- Action: click "Transfer" button
- Verify: dialog opens with department dropdown (current dept excluded or disabled) and comments textarea
- Action: select "Sales" from dropdown
- Action: type "Billing issue" in comments (13 chars)
- Action: click Submit
- Verify: success message "Ticket transferred successfully to Sales"
- Verify: department field shows "Sales"
- Status: [x]

### AC-6: Transfer fails if comment is too short or same department. [BROWSER]
- Depends: AC-5 setup
- Navigate: open a ticket in Support department
- Action: click "Transfer"
- Action: select "Support" (same dept, if selectable)
- Action: type "Valid comment here" in comments
- Action: click Submit
- Verify: error "Ticket already in the department"
- Action: select "Sales"
- Action: clear comments, type "abc" (3 chars)
- Action: click Submit
- Verify: error "Transfer comments too short!"
- Status: [x]

### AC-7: Assigning a closed ticket reopens it. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":1,"status":"closed"} -> {ticket_id}
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Action: click on the closed ticket to open detail view
- Verify: ticket status shows "Closed"
- Action: click "Assign" button (should be available for closed tickets)
- Action: select a staff/team
- Action: type "Taking ownership" in comments (16 chars)
- Action: click Submit
- Verify: success message
- Verify: ticket status changes to "Open"
- Verify: assignment is applied
- Status: [x]

## Checklist (children)

- [ ] TS-M3-C1 — Backend: assign/claim routes + validation + internal notes
- [ ] TS-M3-C2 — Backend: transfer route + dept change + SLA re-select
- [ ] TS-M3-C4a — Frontend: Claim button + Assign dialog UI
- [ ] TS-M3-C4b — Frontend: Transfer dialog UI

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Requires TS-M3-prep seed expansion (second department Sales, team Tier 2, group-dept access).
- The agent's group must have `can_assign_tickets=1` and `can_transfer_tickets=1` (seeded by TS-M3-prep).

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose | Request Shape |
|----------|---------|---------------|
| `POST /api/dev/reset` | Reset DB to seed state | `{}` |
| `POST /api/dev/seed-tickets` | Create test tickets | `{"count":N,"status":"open"\|"closed","assigned":bool,"dept":"Support"\|"Sales"}` |
| `POST /api/dev/seed-staff` | Create additional staff | `{"name":"..."}` |
| `POST /api/auth/login` | Authenticate for browser session | `{"username":"agent","password":"Agent123!"}` |

## Dependencies

- **M1 done**: provides the baseline ticket/staff infrastructure.
- **TS-M3-prep**: seed expansion (second department, team, permissions).
- **TS-M3-C3**: close/reopen backend needed for testing AC-7 (assigning a closed ticket).
