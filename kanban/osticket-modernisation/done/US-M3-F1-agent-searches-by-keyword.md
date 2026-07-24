# US-M3-F1 — Agent searches by keyword and sees results

- **ID**: US-M3-F1
- **Type**: User Story
- **Parent**: EPIC-M3-F
- **Labels**: User Story, M3
- **Scope**: medium

## Spec References

- FS-020.7 (basic keyword search: ticket#, email, deep text)
- FS-020.8 (advanced search: status, dept, assignee, topic, date range)
- BS-020.6 (keyword minimum 3 characters)
- BS-020.7 (keyword resolution strategy: numeric=ticket#, email=exact, other=deep)
- BS-020.3 (search dept filter bounded by access)
- EC-020.1 (sub-3-character search rejected)
- EC-020.10 (search with no status: Status column replaces Priority)

## Context

Epic: EPIC-M3-F — Search.
M1 had no search. This story adds both basic keyword search (by ticket number, email, or
free-text) and an advanced search dialog with filters for status, department, assignee,
date range, etc.

## Description

As a support agent, I can search for tickets using a keyword (ticket number, requester email,
or free text) and see matching results. I can also use advanced search to filter by status,
department, assignee, help topic, and date range.

## Impact

- Frontend (search box, advanced search dialog, search results display)
- Backend (search endpoint with keyword resolution + advanced criteria)
- Database (joins thread table for deep search)
- Browser (desktop)

## Business Rules

- BS-020.6: Search keyword must be >= 3 characters; shorter terms rejected with warning.
- BS-020.7: Numeric keyword = ticket# prefix match; email keyword = exact requester match;
  other = deep search across email, name, subject, thread body.
- BS-020.3: Search dept filter is bounded by staff access (out-of-scope dept silently ignored).
- EC-020.10: Search with no status shows Status column instead of Priority.

## Regressions

- Queue tabs and listing must continue to work alongside search.
- Search results must respect visibility scoping (TS-M3-A2).

## Acceptance Criteria

### AC-1: Basic search by ticket number prefix. [BROWSER]
- Setup: POST /api/dev/reset-tickets
- Setup: POST /api/dev/seed-tickets with `{"ticketId":"12345"}`
- Setup: POST /api/dev/seed-tickets with `{"ticketId":"12367"}`
- Setup: POST /api/dev/seed-tickets with `{"ticketId":"99999"}`
- Navigate: http://localhost:3702/staff/tickets
- Action: Type "123" in search box
- Action: Press Enter or click Search
- Verify: Results include ticket #12345
- Verify: Results include ticket #12367
- Verify: Results do NOT include #99999
- Status: [x]

### AC-2: Basic search by requester email (exact match). [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"email":"alice@example.com"}`
- Setup: POST /api/dev/seed-tickets with `{"email":"bob@example.com"}`
- Navigate: http://localhost:3702/staff/tickets
- Action: Type "alice@example.com" in search box
- Action: Submit search
- Verify: Only ticket from alice@example.com appears
- Verify: Ticket from bob@example.com NOT shown
- Status: [x]

### AC-3: Basic search by free text (deep search). [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"subject":"Printer jam in building 5"}`
- Navigate: http://localhost:3702/staff/tickets
- Action: Type "printer jam" in search box
- Action: Submit search
- Verify: Ticket with subject "Printer jam in building 5" appears in results
- Status: [x]

### AC-4: Search keyword < 3 chars rejected with error. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets
- Action: Type "ab" in search box
- Action: Submit search
- Verify: Error message "Search term must be more than 3 chars" displayed
- Verify: Listing shows normal queue (not empty results)
- Status: [x]

### AC-5: Advanced search dialog opens and shows filter options. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Advanced" link/button near search box
- Verify: Dialog/panel opens
- Verify: Fields present: Keyword, Status, Department, Assignee, Help Topic, Date Range (Start/End)
- Status: [x]

### AC-6: Advanced search filters by status. [BROWSER]
- Setup: POST /api/dev/seed-tickets -> open ticket
- Setup: POST /api/dev/seed-tickets -> close it
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Advanced"
- Action: Select Status = "Closed"
- Action: Submit search
- Verify: Only closed tickets appear in results
- Status: [x]

### AC-7: Advanced search filters by department (bounded by access). [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}`
- Setup: POST /api/dev/seed-tickets with `{"dept":"External"}` (agent has no access)
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Advanced"
- Verify: Department dropdown shows only "Support" (not "External")
- Action: Select Department = "Support"
- Action: Submit search
- Verify: Only Support tickets appear
- Status: [x]

### AC-8: Advanced search filters by date range. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"created":"2026-06-01"}`
- Setup: POST /api/dev/seed-tickets with `{"created":"2026-06-10"}`
- Setup: POST /api/dev/seed-tickets with `{"created":"2026-06-15"}`
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Advanced"
- Action: Set Start Date = 2026-06-01
- Action: Set End Date = 2026-06-10
- Action: Submit search
- Verify: Only tickets from 2026-06-01 and 2026-06-10 appear
- Verify: Ticket from 2026-06-15 NOT shown
- Status: [x]

### AC-9: Search results show "(Search Results)" label. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets
- Action: Type "test" in search box
- Action: Submit search
- Verify: Listing header shows "(Search Results)" suffix or label
- Status: [x]

### AC-10: Search with no status filter shows Status column instead of Priority. [BROWSER]
- Setup: POST /api/dev/seed-tickets (open ticket)
- Setup: POST /api/dev/seed-tickets -> close it
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Advanced"
- Action: Select Status = "Any Status" (or leave unset)
- Action: Submit search
- Verify: Listing shows Status column (not Priority column)
- Verify: Open tickets have "Open" label bolded
- Status: [x]

## Checklist (children)

- [ ] TS-M3-F1 — Backend: basic search (number/email/deep-text)
- [ ] TS-M3-F2 — Backend: advanced search (multi-criteria filtering)
- [ ] TS-M3-F3 — Frontend: search box + advanced search dialog UI

## Test Infrastructure

- Uses the seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Dev endpoint: `POST /api/dev/seed-tickets` to create tickets with specific emails/subjects.
- Requires TS-M3-A2 visibility scoping.

## Dependencies

- **EPIC-M3-A done**: search operates on the same listing with visibility scoping.
- **EPIC-M3-B done**: search results use the same sort/pagination infrastructure.
