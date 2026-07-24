# TS-M3-B4 — add(frontend): sortable column headers + pagination controls

- **ID**: TS-M3-B4
- **Type**: Technical Story
- **Parent**: US-M3-B1
- **Labels**: Technical Story, M3
- **Scope**: medium

## Context

Adds frontend UI for sorting and pagination: clickable column headers that toggle sort
direction, a visual indicator showing the current sort column and direction, and pagination
controls (page numbers, prev/next, page size info) that preserve all query params.

## Impact

- update(component): `TicketList` — column headers are clickable and trigger sort.
- add(component): `SortableHeader` — renders a column header with sort indicator (ascending/
  descending arrow) and click handler.
- add(component): `Pagination` — renders "Showing X-Y of N" info, page number links, prev/next
  buttons.
- update(store): `ticketStore` — tracks current sort/order/page/pageSize; updates URL params
  on change.
- add(hook): `useSortableColumns` — manages sort state and provides click handlers for headers.
- update(route): `/staff/tickets` reads sort/order/p/limit from URL and passes to API.

## Sortable Columns

Headers that should be sortable (with their sort keys for the API):
- Date (`date`)
- Ticket ID (`ID`)
- Subject (`subj`)
- From/Name (`name`)
- Priority (`pri`)
- Variable column: Assigned To (`assignee`), Closed By (`staff`), or Department (`dept`)

## Sort Indicator

- When a column is sorted ascending: show an up arrow (or "asc" indicator).
- When a column is sorted descending: show a down arrow (or "desc" indicator).
- When a column is not the active sort: no indicator (or neutral/both-arrows icon).
- Use MUI's `TableSortLabel` or similar for consistent styling.

## Pagination Controls

- Display: "Showing 1-25 of 127"
- Page links: windowed (e.g., 1 2 3 ... 5 6 or just 1 2 3 4 5 for small totals)
- Prev/Next buttons (disabled when on first/last page)
- Current page is highlighted
- Page links preserve all current query params (status, sort, order, limit)

## URL Param Handling

The queue page URL should reflect the current state:
- `/staff/tickets?status=open&sort=date&order=DESC&limit=10&p=2`
- Clicking a sort header updates `sort` and `order` in URL; resets `p` to 1.
- Clicking a page link updates `p` in URL; preserves sort/order/limit/status.
- On initial load, read params from URL and apply to store/API request.

## Sticky Sort Integration

On initial load:
1. If URL has explicit `sort` param: use it.
2. Else: fetch `/api/staff/tickets/sort-prefs` and use session preference if present.
3. Else: let backend apply default sort (frontend shows no explicit indicator).

When user sorts:
- The API call includes the sort param; backend stores it in session.
- Frontend updates URL to include the sort param.

## Regressions

- Queue tabs and visibility from TS-M3-A4 must still work.
- Sorting/pagination params must be preserved when switching tabs (each tab can have different
  sort, but pagination resets to page 1 on tab switch).

## Acceptance Tests

### AC-1: Column headers are clickable and show sort indicator. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":10,"status":"open"} -> {ids}
- Setup: POST /api/auth/login {"username":"agent","password":"Agent123!"} -> {session}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: Date, Ticket ID, Subject, From, Priority column headers have cursor:pointer, hover effect
- Action: click the Date header
- Verify: Date header shows sort indicator (arrow); URL includes `sort=date`
- Status: [x]

### AC-2: Clicking a sorted column toggles direction. [BROWSER]
- Depends: AC-1
- Navigate: http://localhost:3702/staff/tickets?status=open&sort=date&order=DESC
- Verify: Date header shows descending indicator (down arrow)
- Action: click Date header
- Verify: URL updates to `sort=date&order=ASC`; Date header shows ascending indicator (up arrow)
- Status: [x]

### AC-3: Clicking a different column changes sort column, defaults to DESC. [BROWSER]
- Depends: AC-1
- Navigate: http://localhost:3702/staff/tickets?status=open&sort=date&order=ASC
- Action: click Subject header
- Verify: URL updates to `sort=subj&order=DESC`; Subject header shows descending indicator; Date header has no indicator
- Status: [x]

### AC-4: Pagination info displays correctly. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":30,"status":"open"} -> {ids}
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: "Showing 1-25 of 30" (or similar) text is displayed
- Verify: page 1 is highlighted; page 2 link is visible; Next button is enabled
- Status: [x]

### AC-5: Clicking page link navigates and updates URL. [BROWSER]
- Depends: AC-4
- Action: click page 2 link
- Verify: URL includes `p=2`; "Showing 26-30 of 30" displayed; page 2 is highlighted
- Status: [x]

### AC-6: Prev/Next buttons work correctly. [BROWSER]
- Setup: POST /api/dev/seed-tickets {"count":50,"status":"open"} -> {ids}
- Navigate: http://localhost:3702/staff/tickets?status=open (page 1)
- Verify: Prev button is disabled; Next button is enabled
- Action: click Next
- Verify: on page 2; Prev is enabled; Next is disabled (last page for 50 tickets / 25 per page)
- Action: click Prev
- Verify: back on page 1
- Status: [x]

### AC-7: Sorting resets to page 1. [BROWSER]
- Depends: AC-4
- Navigate: http://localhost:3702/staff/tickets?status=open&p=2
- Verify: on page 2
- Action: click a sort header (e.g., Subject)
- Verify: URL has `p=1` or no p param; showing first page of results
- Status: [x]

### AC-8: All params preserved across pagination. [BROWSER]
- Depends: AC-4
- Navigate: http://localhost:3702/staff/tickets?status=open&sort=date&order=DESC&limit=10
- Action: click page 3
- Verify: URL is `?status=open&sort=date&order=DESC&limit=10&p=3` (all params preserved)
- Status: [x]

### AC-9: Sticky sort is loaded on initial page load. [BROWSER]
- Setup: navigate to http://localhost:3702/staff/tickets?status=open&sort=name&order=ASC (stores in session)
- Navigate: http://localhost:3702/staff/tickets (clear URL, go elsewhere)
- Navigate: http://localhost:3702/staff/tickets?status=open (no sort in URL)
- Verify: Name header shows ascending indicator; tickets sorted by name (sticky pref from session)
- Status: [x]

### AC-10: Variable column is sortable. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: "Assigned To" column header is clickable
- Action: click Assigned To header
- Verify: URL includes `sort=assignee`; Assigned To header shows sort indicator
- Navigate: http://localhost:3702/staff/tickets?status=closed
- Verify: "Closed By" column header is clickable
- Action: click Closed By header
- Verify: URL includes `sort=staff`; Closed By header shows sort indicator
- Status: [x]

### AC-11: Empty state shows no pagination. [BROWSER]
- Setup: POST /api/dev/reset (clear all tickets)
- Navigate: http://localhost:3702/staff/tickets?status=open
- Verify: "No tickets found" or similar empty state message displayed
- Verify: no pagination controls visible
- Status: [x]

## Test Infrastructure

### Dev Endpoints Required for Test Setup

| Endpoint | Purpose |
|----------|---------|
| `POST /api/dev/seed-tickets` | Create test tickets with specified count |
| `POST /api/dev/reset` | Clear all test data |
| `POST /api/auth/login` | Authenticate for browser session |

## Dependencies

- **TS-M3-B1**: backend sort param handling.
- **TS-M3-B2**: backend sticky sort (provides `/api/staff/tickets/sort-prefs` endpoint).
- **TS-M3-B3**: backend pagination (provides pagination metadata in response).
- **TS-M3-A4**: frontend queue tabs (column headers integrate with existing table).
