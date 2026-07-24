# US-M3-B1 — Agent sorts the queue and paginates through results

- **ID**: US-M3-B1
- **Type**: User Story
- **Parent**: EPIC-M3-B
- **Labels**: User Story, M3
- **Scope**: medium

## Spec References

- FS-020.5 (sortable columns & sticky sort)
- FS-020.6 (pagination & page size)
- FS-090.19 (page-size resolution hierarchy: personal -> system default -> 25)
- BS-020.8 (sticky per-queue sort persists in session)

## Context

Epic: EPIC-M3-B — Sorting + Pagination.
M1's queue was unsorted and unpaginated. This story adds clickable column headers to sort the
listing, per-queue sticky sort memory so the agent's preferred ordering persists, and pagination
controls to page through large result sets.

## Description

As a support agent, I can click column headers to sort the queue (Date, ID, Priority, Subject,
Name, Assignee, Department), see an indicator showing the current sort direction, and page
through results using pagination controls. My sort preference is remembered per queue so when I
return to that queue later, my last sort is restored.

## Impact

- Frontend (sortable column headers, sort direction indicator, pagination controls)
- Backend (sort param handling, per-queue default sorts, sticky sort session storage, pagination)
- Browser (desktop)

## Business Rules

- BS-020.8: The chosen sort column and direction are remembered per queue within the session and
  re-applied when the staff member returns to that queue without an explicit sort. Each queue
  keeps its own remembered sort independently.
- FS-020.5: Default sorts vary by queue — Overdue sorts by urgency+due-date; Closed by close-date;
  Answered by last-response; others by priority+effective-date.
- FS-020.6: Page size is resolved from the staff member's personal setting, else system default,
  else 25. A `limit` param overrides for the current request.
- FS-090.19: The pagination range is 5-50 step 5; the `p` param selects the page (default 1).

## Regressions

- The existing M1 queue listing must continue to work without sort/pagination params (uses
  defaults).
- Visibility scoping and status filtering from EPIC-M3-A must be preserved when sorting/paging.
- Sort and pagination params must be preserved across tab switches and page reloads.

## Acceptance Criteria

### AC-1: Clicking a column header sorts the listing by that column. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":15,"status":"open"} -> {ids}
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: the Date column header is clickable (styled as a link or has a sort indicator)
- Action: click the Date header
- Verify: URL includes `sort=date&order=DESC`; listing is ordered by date; Date header shows descending indicator
- Action: click the Date header again
- Verify: order toggles to ASC; Date header shows ascending indicator
- Status: [x]

### AC-2: Sorting is sticky per queue — returning to the queue restores the last sort. [BROWSER]
- Depends: AC-1
- Setup: on the Open queue, click Subject header to sort by Subject ASC
- Verify: URL includes `sort=subj&order=ASC`
- Navigate: click the Closed queue tab
- Navigate: click back to the Open queue tab
- Verify: Open queue is still sorted by Subject ASC; Subject header shows ascending indicator
- Status: [x]

### AC-3: Each queue has its own independent sticky sort. [BROWSER]
- Depends: AC-1
- Setup: on Open queue, click Date header twice to get Date DESC
- Navigate: click Closed queue tab
- Action: click ID header to sort by ID ASC
- Navigate: click Open queue tab
- Verify: sorted by Date DESC (Date header shows descending indicator)
- Navigate: click Closed queue tab
- Verify: sorted by ID ASC (ID header shows ascending indicator)
- Status: [x]

### AC-4: Default sort varies by queue when no sort has been chosen. [BROWSER]
- Setup: POST /api/auth/logout -> clear session
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> fresh session
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: listing shows tickets with highest priority first; no explicit sort indicator (multi-column default)
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: listing is sorted by close-date descending (most recently closed first)
- Navigate: http://localhost:3702/staff/tickets?status=overdue
- Verify: listing is sorted by urgency ASC, due-date ASC (most urgent, soonest due first)
- Status: [x]

### AC-5: Pagination controls appear when results exceed page size; clicking pages works. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":30,"status":"open"} -> {ids}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: "Showing 1-25 of 30" (or similar) is displayed
- Verify: pagination controls visible (page 1 highlighted, page 2 link, Next button)
- Action: click page 2
- Verify: URL includes `p=2`; "Showing 26-30 of 30" displayed; 5 tickets shown
- Status: [x]

### AC-6: The limit param overrides page size for the current request. [BROWSER]
- Depends: AC-5
- Navigate: http://localhost:3702/staff/tickets?status=open&limit=10
- Verify: "Showing 1-10 of 30" displayed; 10 tickets shown per page
- Action: click page 2
- Verify: URL preserves `limit=10`; page 2 shows tickets 11-20
- Status: [x]

### AC-7: Sort and pagination params are preserved across page navigation. [BROWSER]
- Depends: AC-5
- Navigate: http://localhost:3702/staff/tickets?status=open&sort=date&order=ASC&limit=10
- Action: click page 3
- Verify: URL is `?status=open&sort=date&order=ASC&limit=10&p=3`; sort and limit preserved
- Status: [x]

### AC-8: Sortable columns include Date, ID, Priority, Subject, Name, and the variable column. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Date, ID, Priority, Subject, Name column headers are clickable (cursor pointer, hover effect)
- Verify: variable column (Assigned To on open queue) is also sortable
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: variable column (Closed By) is sortable
- Status: [x]

## Checklist (children)

- [ ] TS-M3-B1 — Backend: sort param handling + per-queue default sorts
- [ ] TS-M3-B2 — Backend: sticky sort (session-based per queue)
- [ ] TS-M3-B3 — Backend: pagination with limit param + total count
- [ ] TS-M3-B4 — Frontend: sortable column headers + pagination controls

## Test Infrastructure

- Uses the seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Requires dev endpoint to create bulk test tickets: `POST /api/dev/seed-tickets`.
- Page-size config key `default_page_size` from TS-M3-prep seed.

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose | Request Shape | Response Shape |
|----------|---------|---------------|----------------|
| `POST /api/dev/seed-tickets` | Create bulk test tickets | `{"count":N,"status":"open"\|"closed"\|"overdue"}` | `{"ids":[1,2,3...]}` |
| `POST /api/dev/reset` | Reset DB to seed state | `{}` | `{"success":true}` |

## Dependencies

- **EPIC-M3-A done or in parallel**: the queue tabs/visibility must exist for sorting/pagination
  to add value; can be developed in parallel with TS-M3-A* tickets.
- **TS-M3-prep**: seed expansion (config keys like `default_page_size`).
