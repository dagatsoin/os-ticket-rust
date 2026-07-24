# TS-M3-E3 — add(frontend): FS-021.4 note form UI + thread display of notes

- **ID**: TS-M3-E3
- **Type**: Technical Story
- **Parent**: US-M3-E1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Adds frontend support for internal notes: a note form (separate from the reply form) with
optional title, required body, and state-change dropdown. Notes in the thread are displayed
with staff-only styling, visually distinct from requester messages and staff replies.

## Impact

- add(frontend): Note form in ticket detail view (tab or section next to Reply).
  - Fields: title (optional), body (required), state dropdown (optional).
  - State options: unchanged (default), Close Ticket, Reopen Ticket, Answered, Unanswered,
    Overdue, Not Due.
- add(frontend): Thread entry component supports type=N with distinct styling:
  - Different background color or border (e.g., yellow/amber for internal).
  - Staff name displayed as poster.
  - No "Email to requester" checkbox (notes are never emailed).
- update(frontend): Thread display hides notes when in client view (if we had one; staff-only).

## Regressions

- Reply form (M1) must continue to work.
- Existing thread display must not break.

## Acceptance Tests

### AC-1: Note form is accessible in ticket detail view. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Verify: "Note" tab or section visible (separate from Reply)
- Action: Click Note tab/section
- Verify: Form displayed with body textarea field
- Verify: State dropdown is present
- Status: [x]

### AC-2: Note body is required; submission blocked if empty. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Action: Click Note tab
- Action: Leave body field empty
- Action: Click Submit button
- Verify: Validation error displayed ("Note required" or similar)
- Verify: Form not submitted (no new entry in thread)
- Status: [x] — POST NOTE button is disabled when body is empty, preventing submission

### AC-3: State dropdown offers correct options. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Action: Click Note tab
- Action: Click state dropdown
- Verify: Options include: (unchanged/default), Close Ticket, Reopen Ticket, Answered, Unanswered, Overdue, Not Due
- Status: [x]

### AC-4: Posted note appears in thread with staff-only styling. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Action: Click Note tab
- Action: Type "Investigation notes" in body
- Action: Submit
- Verify: Note appears in thread timeline
- Verify: Note has distinct visual style (different background/border, "Internal Note" label)
- Verify: Poster name displayed (e.g., "Agent")
- Status: [x] — Note appears with amber background, shows "Agent One (internal note)" label and body content

### AC-5: Note with title displays title and body. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Action: Click Note tab
- Action: Type "Summary" in title field
- Action: Type "Details" in body field
- Action: Submit
- Verify: Thread entry shows "Summary" as title/heading
- Verify: Thread entry shows "Details" as body content
- Status: [x]

### AC-6: Note with state change (Close) closes ticket and redirects. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id (open ticket)
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Action: Click Note tab
- Action: Type "Closing this ticket" in body
- Action: Select "Close Ticket" from state dropdown
- Action: Submit
- Verify: Redirected to ticket listing
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: Ticket appears in Closed queue
- Status: [x]

### AC-7: Notes are distinguishable from replies and messages in the thread. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> ticket_id (includes initial message)
- Setup: POST /api/staff/tickets/{ticket_id}/reply with `{"body":"Staff reply"}`
- Setup: POST /api/staff/tickets/{ticket_id}/note with `{"body":"Staff note"}`
- Navigate: http://localhost:3702/staff/tickets/{ticket_id}
- Verify: Message (from requester) has one visual style
- Verify: Reply (from staff to requester) has another visual style
- Verify: Note (internal) has distinct staff-only style (e.g., amber/yellow background)
- Status: [x] — Notes display with amber/orange background; messages have blue border styling

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Dev endpoints needed:
  - **`POST /api/dev/seed-tickets`** — creates test tickets

## Dependencies

- **TS-M3-E1**: POST note endpoint.
- **TS-M3-E2**: state change support.
- **M1 frontend**: ticket detail view, thread display.
