# US-M3-I1 — Agent edits ticket properties

- **ID**: US-M3-I1
- **Type**: User Story
- **Parent**: EPIC-M3-I
- **Labels**: User Story, M3
- **Scope**: medium

## Spec References

- FS-021.15 (edit ticket properties)
- BS-021.1 (per-action permission gating — canEditTickets)
- BS-021.6 (est. due date = explicit due date else SLA grace)
- BS-021.7 (SLA follows department on change)
- EC-021.9 (due date in past / on closed ticket / without time)

## Context

Epic: EPIC-M3-I — Edit + Delete + Lock.
M1/M2 had no edit. This story adds the ability to edit a ticket's requester details, routing
(dept/topic), priority, SLA, source, and explicit due date. A reason note is required.

## Description

As a support agent with edit permission, I can open an edit form for a ticket and change its
properties: requester name/email/phone, department, help topic, priority, SLA, source, and due
date. I must provide a reason note. Saving recomputes SLA/overdue and logs the edit.

## Impact

- Frontend (edit form UI with validation)
- Backend (update endpoint with field validation, SLA recompute, activity log)
- Database (ticket row update, internal note insertion)
- Browser (desktop)

## Business Rules

- BS-021.1: Requires `canEditTickets` permission; otherwise 403 "Permission denied".
- BS-021.6: Editing due date clears overdue flag if the new date is in the future.
- BS-021.7: Editing department re-selects SLA when current SLA is empty/transient.
- FS-021.15: Due date cannot be set on a closed ticket; must be in the future; time is required.

## Regressions

- Ticket view page must still render correctly after edit.
- SLA/overdue flags must not break existing ticket listing filters.

## Acceptance Criteria

### AC-1: Edit form loads with current ticket values. [BROWSER]
- Setup: POST /api/dev/reset-db
- Setup: POST /api/dev/seed-staff → {username: "agent", password: "Agent123!", can_edit_tickets: true}
- Setup: POST /api/dev/seed-tickets → [{dept: "Support", priority: "Normal", name: "Alice Smith", email: "alice@example.com", phone: "555-1234"}] → {ids: [100]}
- Navigate: http://localhost:3702/staff/login
- Action: Type "agent" in username field
- Action: Type "Agent123!" in password field
- Action: Click "Sign In" button
- Navigate: http://localhost:3702/staff/tickets/100
- Verify: "Edit" button is visible in action bar
- Action: Click "Edit" button
- Verify: Edit form/modal opens
- Verify: Name field contains "Alice Smith"
- Verify: Email field contains "alice@example.com"
- Verify: Phone field contains "555-1234"
- Verify: Department dropdown shows "Support" selected
- Verify: Priority dropdown shows "Normal" selected
- Verify: SLA dropdown is present
- Verify: Source dropdown is present
- Verify: Due date field is present
- Status: [x]

### AC-2: Successful edit saves changes and logs activity. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{dept: "Support", priority: "Normal"}] → {ids: [101]}
- Navigate: http://localhost:3702/staff/tickets/101
- Action: Click "Edit" button
- Action: Select "High" from priority dropdown
- Action: Type "Escalating per manager request" in reason textarea
- Action: Click "Save" button
- Verify: Success snackbar shows "Ticket updated successfully"
- Verify: Ticket view shows priority badge "High"
- Action: Scroll to ticket thread / internal notes section
- Verify: Internal note entry shows "Ticket Updated" with reason text "Escalating per manager request"
- Status: [x]

### AC-3: Reason note is required. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{priority: "Normal"}] → {ids: [102]}
- Navigate: http://localhost:3702/staff/tickets/102
- Action: Click "Edit" button
- Action: Select "High" from priority dropdown
- Action: Leave reason textarea empty
- Action: Click "Save" button
- Verify: Validation error appears "Reason for the update required"
- Verify: Form is NOT submitted (no network request or page change)
- Status: [x]

### AC-4: Due date validation — must be in the future. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [103]}
- Navigate: http://localhost:3702/staff/tickets/103
- Action: Click "Edit" button
- Action: Set due date to yesterday's date using date picker
- Action: Type "Testing" in reason field
- Action: Click "Save" button
- Verify: Validation error appears "Due date must be in the future"
- Status: [x]

### AC-5: Due date cannot be set on closed ticket. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "closed"}] → {ids: [104]}
- Navigate: http://localhost:3702/staff/tickets/104
- Action: Click "Edit" button
- Verify: Due date field is disabled OR has tooltip "Due date cannot be set on a closed ticket"
- Status: [x]

### AC-6: Editing department re-selects SLA. [BROWSER]
- Setup: POST /api/dev/seed-departments → [{name: "Support", sla: "Default 24h"}, {name: "Sales", sla: "Express 4h"}]
- Setup: POST /api/dev/seed-tickets → [{dept: "Support"}] → {ids: [105]}
- Navigate: http://localhost:3702/staff/tickets/105
- Action: Click "Edit" button
- Action: Select "Sales" from department dropdown
- Action: Type "Routing to Sales team" in reason field
- Action: Click "Save" button
- Verify: Success snackbar appears
- Verify: Ticket view shows SLA updated (reflects Sales department SLA)
- Status: [x]

### AC-7: Editing due date clears overdue flag. [BROWSER]
- Setup: POST /api/dev/seed-tickets → [{status: "open", isoverdue: true}] → {ids: [106]}
- Navigate: http://localhost:3702/staff/tickets/106
- Verify: Overdue badge/indicator is visible
- Action: Click "Edit" button
- Action: Set due date to 7 days from now
- Action: Type "Extending deadline" in reason field
- Action: Click "Save" button
- Verify: Success snackbar appears
- Verify: Overdue badge/indicator is NO longer visible
- Status: [x]

### AC-8: Agent without canEditTickets sees no Edit button. [BROWSER]
- Setup: POST /api/dev/seed-staff → {username: "agent", password: "Agent123!", can_edit_tickets: false}
- Setup: POST /api/dev/seed-tickets → [{status: "open"}] → {ids: [107]}
- Navigate: http://localhost:3702/staff/login
- Action: Log in as agent / Agent123!
- Navigate: http://localhost:3702/staff/tickets/107
- Verify: "Edit" button is NOT visible in action bar
- Status: [x]

## Checklist (children)

- [ ] TS-M3-I1 — Backend: edit/update endpoint + validation
- [ ] TS-M3-I4 — Frontend: edit form UI

## Test Infrastructure

**Credentials:**
- Primary: `agent` / `Agent123!` (TS-M1-A3 seed)

**Dev Endpoints Required (Setup only, NOT validation):**
- `POST /api/dev/reset-db` — reset to clean state
- `POST /api/dev/seed-staff` — create/update staff with permission flags
  - Body: `{username, password, can_edit_tickets}`
- `POST /api/dev/seed-tickets` — create tickets with specified properties
  - Body: `[{dept, priority, name, email, phone, status, isoverdue}]`
  - Returns: `{ids: [...]}`
- `POST /api/dev/seed-departments` — create departments with SLA assignments
  - Body: `[{name, sla}]`

**Seed Expansion Required (TS-M3-prep):**
- `can_edit_tickets` permission flag on groups
- Multiple departments with different SLAs for AC-6

## Dependencies

- **EPIC-M3-D**: SLA infrastructure for due date / SLA recomputation.
- **TS-M3-prep**: seed expansion for edit permission flag, multiple departments/SLAs.
