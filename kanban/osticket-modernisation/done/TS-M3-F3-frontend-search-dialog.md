# TS-M3-F3 — add(frontend): FS-020.7-8 search box + advanced search dialog UI

- **ID**: TS-M3-F3
- **Type**: Technical Story
- **Parent**: US-M3-F1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Adds frontend support for search: a quick-search text box above the listing, an "Advanced"
link to open a dialog with multi-criteria filters, and proper display of search results
(including "(Search Results)" label and Status column when no status filter).

## Impact

- add(frontend): Search box above the ticket listing.
  - Text input for keyword.
  - Submit triggers `GET /api/staff/tickets?a=search&query={keyword}`.
  - "Advanced" link opens the advanced search dialog.
- add(frontend): Advanced search dialog (modal or side panel).
  - Fields: Keyword, Status (Any/Open/Answered/Overdue/Closed), Department (dropdown),
    Assignee (dropdown), Help Topic (dropdown), Start Date, End Date.
  - Department dropdown only shows departments agent has access to.
  - Submit combines all filled criteria into search params.
- update(frontend): Search results display:
  - Listing header shows "(Search Results)" suffix.
  - When `status_column: true` in response, show Status column instead of Priority.
  - Clear/reset search button to return to normal queue view.

## Regressions

- Queue tabs and normal listing must continue to work.
- Search results must integrate with sort/pagination (EPIC-M3-B).

## Acceptance Tests

### AC-1: Search box is visible above the listing. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets
- Verify: Search text input visible above ticket listing
- Verify: "Advanced" link or search settings button present
- Status: [x]

### AC-2: Quick search submits keyword and shows results. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"subject":"test keyword"}`
- Navigate: http://localhost:3702/staff/tickets
- Action: Type "test" in search box
- Action: Press Enter or click Search
- Verify: URL includes `?a=search&query=test`
- Verify: Listing shows matching ticket
- Verify: Header includes "(Search Results)"
- Status: [x]

### AC-3: Search < 3 chars shows error message. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets
- Action: Type "ab" in search box
- Action: Submit search
- Verify: Error "Search term must be more than 3 chars" displayed
- Verify: Listing shows normal queue (fallback), not empty
- Status: [x]

### AC-4: Advanced search dialog opens with filter fields. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Advanced" link
- Verify: Dialog/panel appears
- Verify: Fields present: Keyword, Status, Department, Assignee, Help Topic, Date Range (Start/End)
- Status: [x]

### AC-5: Advanced search Department dropdown shows only accessible depts. [BROWSER]
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Advanced"
- Action: Click Department dropdown
- Verify: "Support" listed (agent has access)
- Verify: "External" NOT listed (agent has no access)
- Status: [x]

### AC-6: Advanced search filters combine correctly. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}` (open)
- Setup: POST /api/dev/seed-tickets with `{"dept":"Support"}` -> close
- Setup: POST /api/dev/seed-tickets with `{"dept":"Sales"}` (open)
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Advanced"
- Action: Select Status=Open
- Action: Select Department=Support
- Action: Submit
- Verify: Only open Support tickets in results
- Status: [x]

### AC-7: Search with no status shows Status column. [BROWSER]
- Setup: POST /api/dev/seed-tickets (open)
- Setup: POST /api/dev/seed-tickets -> close
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Advanced"
- Action: Select Status=Any (or leave unset)
- Action: Submit
- Verify: Listing shows Status column (not Priority)
- Verify: Open tickets have "Open" label bolded
- Status: [x]

### AC-8: Clear search returns to normal queue view. [BROWSER]
- Setup: POST /api/dev/seed-tickets
- Navigate: http://localhost:3702/staff/tickets
- Action: Search for "test"
- Verify: Search results displayed
- Action: Click Clear/Reset search button
- Verify: URL no longer has search params
- Verify: Normal queue listing displayed
- Status: [x]

### AC-9: Date range pickers work correctly. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"created":"2026-06-01"}`
- Setup: POST /api/dev/seed-tickets with `{"created":"2026-06-10"}`
- Setup: POST /api/dev/seed-tickets with `{"created":"2026-06-20"}`
- Navigate: http://localhost:3702/staff/tickets
- Action: Click "Advanced"
- Action: Set Start Date = 2026-06-01
- Action: Set End Date = 2026-06-15
- Action: Submit search
- Verify: Only tickets from 2026-06-01 and 2026-06-10 appear
- Status: [x]

### AC-10: Search results integrate with pagination. [BROWSER]
- Setup: POST /api/dev/seed-tickets with `{"subject":"pagination test"}` x30 tickets
- Navigate: http://localhost:3702/staff/tickets
- Action: Search for "pagination"
- Verify: 30 results, paginated (page 1 of N)
- Action: Click page 2
- Verify: URL preserves search params (?a=search&query=pagination&page=2)
- Verify: Page 2 results are still search results
- Status: [x]

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Dev endpoints needed:
  - **`POST /api/dev/seed-tickets`** — accepts `subject`, `dept`, `created` params
  - **`POST /api/dev/reset-tickets`** — clears all tickets for clean state

## Dependencies

- **TS-M3-F1**: basic search endpoint.
- **TS-M3-F2**: advanced search endpoint.
- **TS-M3-A4**: queue tabs / listing infrastructure.
- **EPIC-M3-B**: pagination integration.
