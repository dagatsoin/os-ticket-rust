# TS-M3-I4 — add(frontend): FS-021.15 edit form UI

- **ID**: TS-M3-I4
- **Type**: Technical Story
- **Parent**: US-M3-I1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements the frontend edit form per FS-021.15. The edit button is shown only when staff has
`canEditTickets` permission. The form loads current ticket values, validates input, and POSTs
to the update endpoint. Due date field is disabled on closed tickets.

## Impact

- add(component): `TicketEditForm` — form with fields: name, email, phone, phone_ext, dept_id
  (dropdown), topic_id (dropdown), priority_id (dropdown), sla_id (dropdown), source (dropdown),
  due_date (datetime picker), reason (required textarea).
- update(component): `TicketView` — add Edit button (conditional on canEditTickets), opens edit
  form as modal or navigates to edit page.
- add(api): `apiClient.updateTicket(ticketId, updates)` — calls PUT /api/staff/tickets/{ticketId}.
- add(validation): client-side validation for email format, reason required, due date in future.

## Regressions

- Ticket view must still render correctly after edit.
- Navigation between ticket view and edit must work smoothly.

## Acceptance Tests

### AC-1: Edit button shown only for canEditTickets staff. [BROWSER]
- Setup: POST /api/dev/reset-db
- Setup: POST /api/dev/seed-staff → {username: "agent", password: "Agent123!", can_edit_tickets: true}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [100]}
- Navigate: http://localhost:3702/staff/login
- Action: Type "agent" in username field
- Action: Type "Agent123!" in password field
- Action: Click "Sign In" button
- Navigate: http://localhost:3702/staff/tickets/100
- Verify: "Edit" button is visible in action bar
- Setup: POST /api/dev/update-staff → {username: "agent", can_edit_tickets: false}
- Action: Press F5 to refresh
- Verify: "Edit" button is NOT visible
- Status: [x]

### AC-2: Edit form loads current ticket values. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{name: "Alice", email: "alice@example.com", priority: "Normal", dept: "Support"}] → {ids: [101]}
- Navigate: http://localhost:3702/staff/tickets/101
- Action: Click "Edit" button
- Verify: Name field contains "Alice"
- Verify: Email field contains "alice@example.com"
- Verify: Priority dropdown shows "Normal" selected
- Verify: Department dropdown shows "Support" selected
- Status: [x]

### AC-3: Form validation — reason required. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{priority: "Normal"}] → {ids: [102]}
- Navigate: http://localhost:3702/staff/tickets/102
- Action: Click "Edit" button
- Action: Select "High" from priority dropdown
- Action: Leave reason textarea empty
- Action: Click "Save" button
- Verify: Validation error displays "Reason for the update required"
- Verify: Form is not submitted (page does not navigate away)
- Status: [x]

### AC-4: Form validation — due date must be in future. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [103]}
- Navigate: http://localhost:3702/staff/tickets/103
- Action: Click "Edit" button
- Action: Set due date to yesterday using date picker
- Action: Type "Test reason" in reason field
- Action: Click "Save" button
- Verify: Validation error displays "Due date must be in the future"
- Status: [x]

### AC-5: Due date field disabled on closed ticket. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "closed"}] → {ids: [104]}
- Navigate: http://localhost:3702/staff/tickets/104
- Action: Click "Edit" button
- Verify: Due date field has disabled attribute
- Verify: Due date field has tooltip or label "Cannot set due date on closed ticket"
- Status: [x]

### AC-6: Successful edit redirects to ticket view with success message. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{priority: "Normal"}] → {ids: [105]}
- Navigate: http://localhost:3702/staff/tickets/105
- Action: Click "Edit" button
- Action: Select "High" from priority dropdown
- Action: Type "Escalating issue" in reason textarea
- Action: Click "Save" button
- Verify: Page redirects to ticket view (URL is /staff/tickets/105)
- Verify: Success snackbar displays "Ticket updated successfully"
- Verify: Priority badge shows "High"
- Status: [x]

### AC-7: Source dropdown shows valid options. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [106]}
- Navigate: http://localhost:3702/staff/tickets/106
- Action: Click "Edit" button
- Action: Click on source dropdown to open it
- Verify: Dropdown contains "Phone" option
- Verify: Dropdown contains "Email" option
- Verify: Dropdown contains "Web" option
- Verify: Dropdown contains "API" option
- Verify: Dropdown contains "Other" option
- Status: [x]

### AC-8: Department/topic/priority/SLA dropdowns fetch from API. [BROWSER]
- Setup: POST /api/dev/seed-departments → [{name: "Support"}, {name: "Sales"}]
- Setup: POST /api/dev/seed-topics → [{name: "General"}, {name: "Billing"}]
- Setup: POST /api/dev/seed-slas → [{name: "Default"}, {name: "Express"}]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [107]}
- Navigate: http://localhost:3702/staff/tickets/107
- Action: Click "Edit" button
- Verify: Department dropdown contains "Support" and "Sales"
- Verify: Help topic dropdown contains "General" and "Billing"
- Verify: SLA dropdown contains "Default" and "Express"
- Verify: Priority dropdown contains "Low", "Normal", "High", "Emergency"
- Status: [x]

## Test Infrastructure

**Dev Endpoints Required (Setup only):**
- `POST /api/dev/reset-db` — reset to clean state
- `POST /api/dev/seed-staff` — create/update staff with permission flags
- `POST /api/dev/update-staff` — update existing staff permissions
- `POST /api/dev/seed-tickets` — create tickets with specified properties
- `POST /api/dev/seed-departments` — create departments for dropdown
- `POST /api/dev/seed-topics` — create help topics for dropdown
- `POST /api/dev/seed-slas` — create SLAs for dropdown

## Dependencies

- **TS-M3-I1**: backend update endpoint.
- **EPIC-M3-A**: ticket view page structure.
- **TS-M3-prep**: reference data endpoints for dropdowns (may need to add /api/staff/slas).
