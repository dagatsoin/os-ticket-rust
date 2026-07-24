# US-M3-E1 — Agent posts an internal note on a ticket

- **ID**: US-M3-E1
- **Type**: User Story
- **Parent**: EPIC-M3-E
- **Labels**: User Story, M3
- **Scope**: medium

## Spec References

- FS-021.4 (post internal note: body required, optional title, optional state change)
- FS-021.22 (thread model: Note type `N`, poster name, staff_id)
- BS-021.10 (internal notes are never e-mailed to the requester)
- FS-020.3 (thread count badge in listing)

## Context

Epic: EPIC-M3-E — Internal Notes.
M1 had only replies (type R); agents could not post private notes. This story adds internal
note posting: a private thread entry visible only to staff, never sent to the requester.

## Description

As a support agent, I can post an internal note on a ticket to document private information
(investigation notes, escalation context, etc.) that is visible only to staff. Optionally,
I can change the ticket state (close/reopen/answered/unanswered) when posting the note.

## Impact

- Frontend (note form UI, thread display of notes with staff-only styling)
- Backend (POST note endpoint, note validation, state change integration)
- Database (thread_entry type N)
- Browser (desktop)

## Business Rules

- BS-021.10: Internal notes are never e-mailed to the requester.
- FS-021.4: Note body is required ("Note required"); title/summary is optional.
- FS-021.4: State change dropdown: closed, open (reopen), answered, unanswered, overdue, notdue.
- FS-021.4: State change permissions: closed requires canCloseTickets; overdue/notdue/unassigned
  require department manager (deferred to M4 for manager checks; M3 allows all state changes).

## Regressions

- Existing reply flow (M1) must continue to work.
- Thread display must distinguish notes (staff-only) from messages/replies.

## Acceptance Criteria

### AC-1: Agent can post an internal note with required body. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Action: Click "Note" tab or section
- Action: Type "Internal investigation notes" in body field
- Action: Click Submit/Post Note button
- Verify: Note appears in thread as new entry
- Verify: Note has distinct staff-only styling (different background/border color)
- Status: [x]

### AC-2: Empty note body is rejected with validation error. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Action: Click "Note" tab
- Action: Leave body field empty
- Action: Click Submit button
- Verify: Error message "Note required" displayed
- Verify: Note not posted (thread unchanged)
- Status: [x]

### AC-3: Note has optional title/summary. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Action: Click "Note" tab
- Action: Type "Summary Title" in title field, "Detailed body" in body field
- Action: Submit
- Verify: Note in thread shows both title "Summary Title" and body "Detailed body"
- Action: Post another note with only body (title empty)
- Verify: Second note displays without title heading
- Status: [x]

### AC-4: Note with state change closes the ticket. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id (open ticket)
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Action: Click "Note" tab
- Action: Type "Closing note" in body
- Action: Select "Close Ticket" from state dropdown
- Action: Submit
- Verify: Redirected to ticket listing
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: Ticket appears in Closed queue
- Status: [x]

### AC-5: Note with state change reopens a closed ticket. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Setup: POST /api/staff/tickets/{ticket_id}/close
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Action: Click "Note" tab
- Action: Type "Reopening for further investigation" in body
- Action: Select "Reopen Ticket" from state dropdown
- Action: Submit
- Verify: Ticket status changes to open
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Ticket appears in Open queue
- Status: [x]

### AC-6: Notes are not visible in the public client thread view. [API-ONLY]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Setup: POST /api/staff/tickets/{ticket_id}/note with `{"body":"Secret note"}`
- Request: GET http://localhost:3701/api/tickets/{ticket_id}/thread (client endpoint, no auth)
- Expect: 200, response does not contain "Secret note" or type="N" entries
- Expect: Only type="M" (message) and type="R" (reply) entries visible
- Status: [x]

### AC-7: Thread count badge in listing includes notes. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Setup: POST /api/staff/tickets/{ticket_id}/reply with `{"body":"Reply 1"}`
- Setup: POST /api/staff/tickets/{ticket_id}/reply with `{"body":"Reply 2"}`
- Setup: POST /api/staff/tickets/{ticket_id}/note with `{"body":"Note 1"}`
- Navigate: http://localhost:3702/staff/tickets
- Verify: Thread count badge for ticket shows (4) (1 message + 2 replies + 1 note)
- Status: [x]

## Checklist (children)

- [ ] TS-M3-E1 — Backend: POST note endpoint + validation
- [ ] TS-M3-E2 — Backend: note-form state change (close/reopen via note)
- [ ] TS-M3-E3 — Frontend: note form UI + thread display of notes (staff-only)

## Test Infrastructure

- Uses the seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Requires M1 thread model (thread_entry table supports type N).
- Close/reopen actions from EPIC-M3-C are integrated via state change dropdown.

## Dependencies

- **M1 done**: thread model, reply endpoint.
- **EPIC-M3-C**: close/reopen actions (state change dropdown integrates with these).
