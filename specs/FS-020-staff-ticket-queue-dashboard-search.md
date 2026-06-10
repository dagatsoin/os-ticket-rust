# FS-020: Staff Ticket Queue, Dashboard & Search

## Overview

The Staff Ticket Queue, Dashboard & Search specification defines how authenticated staff members find, list, filter, sort, paginate, search, and bulk-act on tickets, plus the staff dashboard that visualises ticket activity over time and tabular statistics grouped by department, help topic, or staff. It is the landing surface of the staff control panel: both the application root (`scp/index.php`) and the dashboard fall through to the ticket queue (`scp/index.php` simply requires `scp/tickets.php`).

This specification owns:

- The **ticket queue page** (`scp/tickets.php` + `include/staff/tickets.inc.php`) — its predefined queues/tabs (Open, Answered, My Tickets, Overdue, Closed, New Ticket link), the listing table and its columns, **department- and assignment-based visibility scoping**, sortable columns with sticky per-queue sort memory, pagination, the per-queue mass/bulk action bar, and CSV export of the current query.
- **Basic and advanced ticket search** (the keyword box and the "Advanced Ticket Search" dialog: keyword, status, department, assignee, closed-by, help topic, and date-range criteria).
- The **staff dashboard** (`scp/dashboard.php` + `include/ajax.reports.php` `OverviewReportAjaxAPI`) — the ticket-activity line chart over a selectable timeframe/period, and the tabular statistics grouped by department/topic/staff with Opened/Assigned/Overdue/Closed/Reopened counts plus average Service Time and Response Time, including the per-table CSV download.

This specification does **NOT** own the single-ticket view, reply/note/assign/transfer/close/reopen/lock/edit/print mechanics, or ticket creation — those POST actions are dispatched from `scp/tickets.php` but belong to **FS-021** (single ticket workflow) and **FS-011** (ticket submission). Only the queue-listing, search-listing, dashboard, and the mass-action bar are in scope here; this spec references FS-021 for the per-ticket action handlers it shares an entry script with. Pagination mechanics are shared shell behavior cross-referenced to **FS-090**; export mechanics (`Export::saveTickets`, `Http::download`) are owned by **FS-090**; canonical enums, table/column names, and permission flags are owned by **FS-091**.

---

## Functional Requirements

### FS-020.1: Queue Entry & Default Landing

**Description**: The system shall serve the staff ticket queue as the default staff landing surface, with the application root and dashboard both routing into the ticket workflow.

**Acceptance Criteria**:
- The staff control-panel root (`scp/index.php`) contains no logic of its own; it `require`s `scp/tickets.php`, so visiting the staff root lands on the ticket queue.
- Access requires an authenticated staff session; the queue view template (`include/staff/tickets.inc.php`) aborts with "Access Denied" unless the request is inside the staff control panel (`OSTSCPINC` defined) AND a current staff object exists AND `isStaff()` is true (see FS-002 for the staff session/auth gate).
- When no ticket is selected and no special action (`a`) is requested, the queue listing template (`tickets.inc.php`) is rendered between the staff header and footer partials.
- When `status` (and no search) is unset/unrecognized, the queue defaults to the **Open** queue (`status='open'`).
- If the staff member has an auto-refresh rate configured and the request is a plain queue view (not a POST, no action), the page injects a meta-refresh header set to `auto_refresh_rate × 60` seconds (see BS-020.13).

### FS-020.2: Predefined Queues (Status Tabs)

**Description**: The system shall present a set of predefined ticket queues, selectable via the `status` request parameter, each with its own visibility, filter, default sort, and displayed columns. The set of queue sub-menu tabs shown is driven by the acting staff member's quick stats and configuration.

**Acceptance Criteria**:
- The `status` request value is "overloaded": it selects which queue logic runs, distinct from the real ticket `status` column applied to the query. The recognized values and their behavior:

  | `status` value | Real status filter | Extra filter | Result label | Assigned-To column |
  |----------------|--------------------|--------------|--------------|--------------------|
  | `open` (default) | `status='open'` | answered/assigned visibility rules (BS-020.4) | "Open Tickets" | conditional |
  | `answered` | `status='open'` | `isanswered=1` | "Answered Tickets" | shown |
  | `assigned` | `status='open'` | `staff_id` = current staff | "My Tickets" / "Assigned Tickets" | hidden (already mine) |
  | `overdue` | `status='open'` | `isoverdue=1` | "Overdue Tickets" | shown |
  | `closed` | `status='closed'` | — | "Closed Tickets" | shown as "Closed By" |

- The queue sub-menu (left nav) is assembled from the acting staff member's quick stats (`getTicketsStats()` → `Ticket::getStaffStats()`, see FS-020.11). Each entry shows a count via `number_format`:
  - **Open**: shown always. If "show answered tickets" is enabled system-wide, the Open count = `open + answered`; otherwise the Open count = `open` and a separate **Answered** tab is shown only when `answered > 0`.
  - **My Tickets** (`status=assigned`): shown only when `assigned > 0`. If `assigned > 10`, a warning banner "N tickets assigned to you! Do something about it!" is raised (only if no warning is already set).
  - **Overdue** (`status=overdue`): shown only when `overdue > 0`. If `overdue > 10`, a system notice "N overdue tickets!" is raised (only if none set).
  - **Closed**: shown always; labelled "My Closed Tickets" with the staff member's own closed count when the staff member is access-limited (`showAssignedOnly()`), otherwise "Closed Tickets" with the global-scope closed count.
  - **New Ticket** (`a=open`): shown only when the staff member can create tickets (handler in FS-011).
- Exactly one sub-menu entry is marked active, matching the current `status` (Open is active when `status` is empty or `open`).
- When a search is performed with no `status`, no sub-menu entry is marked active.

### FS-020.3: Ticket Listing Table & Columns

**Description**: The system shall render the resulting tickets as a paginated table with sortable column headers, per-row preview/access links, and visual status indicators.

**Acceptance Criteria**:
- The table caption shows the "Showing X – Y of N" range (from pagination, FS-090) followed by the result-type label (e.g., "Open Tickets"; with " (Search Results)" appended on search).
- A leading checkbox column is rendered only when the staff member can mass-manage tickets (`canManageTickets()`, BS-020.9). Each checkbox carries the internal `ticket_id` as `tids[]`.
- Columns, left to right (header → sort key in FS-020.5):
  1. **Ticket** — external ticket number (`ticketID`), linking to the single-ticket view (`tickets.php?id=<ticket_id>`); rendered bold when the ticket is open, not answered, and not locked. The link icon reflects the ticket `source` (e.g., email/phone/web) and acts as a hover preview. The cell `title` shows the requester email.
  2. **Date** — ticket created date, formatted via the system date helper.
  3. **Subject** — truncated to 40 characters, links to the ticket; shows a flag icon ("locked" if locked by someone else, else "overdue" if overdue); appends a thread-count badge `(N)` when the conversation has more than one entry, and a paperclip icon when the ticket has attachments.
  4. **From** — requester name, truncated to 22 characters (truncation honors an `@` boundary for emails).
  5. **Priority** OR **Status** — in normal queues a Priority cell rendered with the priority's background color and description; in a status-agnostic search (search with no status) this column becomes **Status** ("Open" bolded) instead.
  6. **Assigned To** / **Closed By** / **Department** — the rightmost column varies (BS-020.5): "Assigned To" (staff or team name with an icon) when assigned tickets are shown, "Closed By" (closing staff name) on the Closed queue, otherwise "Department" (department name).
- A per-row flag icon distinguishes locked vs overdue tickets; lock detection joins only locks held by a **different** staff member that have not expired (`expire > NOW()`).
- When the result set is empty the table body shows "There are no tickets here. (Leave a little early today)." (or "Query returned 0 results." when a query produced nothing).
- Long text values are truncated for display: staff/team/department names to 40 chars, subject to 40, requester name to 22.

### FS-020.4: Visibility Scoping (Department + Assignment)

**Description**: The system shall restrict which tickets a staff member can see in any queue based on department access, team membership, and direct assignment — independent of the selected queue's status filter.

**Acceptance Criteria**:
- The base visibility predicate (applied to every queue and search) is the OR of:
  1. Tickets directly assigned to the staff member that are **open** (`ticket.staff_id = me AND status='open'`) — always included, regardless of department.
  2. **If the staff member is NOT access-limited** (`showAssignedOnly()` false): tickets in any department the staff member has access to (`ticket.dept_id IN (my depts)`).
  3. **If the staff member belongs to teams**: tickets assigned to one of those teams that are **open** (`ticket.team_id IN (my teams) AND status='open'`).
- An access-limited ("assigned only") staff member therefore sees only tickets assigned to them (directly or via team) and not the whole department.
- Department access derives from the staff member's group↔department access plus home department (see FS-031/FS-091 for the access model). An empty department list contributes `dept_id IN (0)` (i.e., no department match).
- The same department-membership constraint gates advanced-search department selection: a `deptId` search filter is honored only if that department is in the staff member's accessible departments (otherwise silently ignored).
- Per-ticket access is independently re-checked when a specific ticket id is opened (`checkStaffAccess()`); the queue-level predicate is the listing scope, not the per-ticket authorization (per-ticket handling is FS-021).

### FS-020.5: Sortable Columns & Sticky Sort

**Description**: The system shall allow staff to sort the listing by clickable column headers, remember the last chosen sort per queue within the session, and apply queue-appropriate default sort orders.

**Acceptance Criteria**:
- The sortable keys and their underlying sort expressions:

  | Sort key (`sort`) | Sort expression |
  |-------------------|-----------------|
  | `date` | ticket created timestamp |
  | `ID` | numeric ticket number (`ticketID*1`) |
  | `pri` | priority urgency |
  | `name` | requester name |
  | `subj` | subject |
  | `status` | ticket status |
  | `assignee` | computed assignee name |
  | `staff` | closing/assigned staff name |
  | `dept` | department name |

- Order direction (`order`) is constrained to `ASC` or `DESC`; any other value is ignored. Default direction is `DESC`. Each header link toggles to the negated current order.
- **Sticky sort**: when a `sort` is chosen for a queue, the chosen `sort`+`order` are stored in the session keyed by queue (`<queue>_tickets`), and reused on the next visit to that queue if no explicit `sort` is supplied. Each predefined queue keeps its own remembered sort.
- **Default sort by queue** (used when no sort is set and none is remembered):
  - **Answered**: by last response then created (no priority sorting).
  - **Closed**: by closed date then created (no priority sorting).
  - **Overdue**: priority urgency ASC, then due-date null-last, then due date ASC, then effective date ASC, then created.
  - **All other open queues**: priority urgency ASC, then effective date DESC, then created.
- "effective_date" is computed as reopened-date, else last-message date, else created date; "duedate" is the explicit due date, else (when an active SLA exists) created + SLA grace-period hours, else null. (SLA/priority models are FS-091/FS-032.)
- The active header carries a CSS class reflecting the current order (lower-cased `asc`/`desc`) so the UI can show an up/down indicator. When no display sort is selected the order-indicator variable defaults to the key `urgency`, which is not a visible column header, so no column shows an active-sort indicator in that case.
- The sort/order chosen are echoed back into the pagination URL and column-header links so they persist across paging; the effective order direction defaults to `DESC` when neither a request `order` nor a remembered `order` resolves to `ASC`/`DESC`. For multi-expression default sorts, the resolved order direction is spliced before each comma-separated term that is not already explicitly `ASC`/`DESC`.

### FS-020.6: Pagination & Page Size

**Description**: The system shall paginate the listing and let staff page through results, honoring a configurable page size.

**Acceptance Criteria**:
- The page size (`PAGE_LIMIT`) is resolved per **FS-090.19** (personal → system default → `DEFAULT_PAGE_LIMIT`, with the 5–50-step-5 configurable range).
- A `limit` query parameter, when numeric, overrides the resolved page size for the current request and is preserved across page links and export (queue-only override owned by FS-020).
- The current page is taken from the `p` query parameter (numeric, default 1). The pagination control renders a windowed list of page links with previous/next jump links centered on the current page (window span 5 each side; mechanics owned by FS-090 `Pagenate`).
- The total count is computed from the same WHERE/JOIN predicate as the listing (a `COUNT(DISTINCT ticket.ticket_id)` query) so the "Showing … of N" count reflects the filtered/searched scope.
- Pagination links preserve the active query string including status, search criteria, sort, order, and limit.

### FS-020.7: Basic (Keyword) Search

**Description**: The system shall provide a quick keyword search box above the listing that finds tickets by number, requester email, or free text.

**Acceptance Criteria**:
- A search is triggered when `a=search`. The keyword (`query`) must be **at least 3 characters**; a shorter (or empty when basic search was submitted) term is rejected with "Search term must be more than 3 chars" and the view falls back to the regular (non-search) listing rather than an error page.
- Keyword resolution branches by the term:
  - **Purely numeric** term → matches the ticket number prefix (`ticketID LIKE '<term>%'`).
  - **Valid email** (contains `@` and passes email validation) → exact requester-email match (`ticket.email = '<term>'`).
  - **Otherwise** → a "deep search" across requester email, requester name, subject, and the thread body and thread title (`LIKE '%<term>%'`), which additionally joins the ticket thread table.
- The basic search posts via GET to the queue page, carrying the keyword; an "[advanced]" link opens the advanced-search dialog (FS-020.8).
- On search, the result label gains a " (Search Results)" suffix; when a search has no status the listing shows a Status column instead of Priority (FS-020.3).

### FS-020.8: Advanced Search

**Description**: The system shall provide an advanced-search dialog allowing staff to combine keyword, status, department, assignee, closed-by, help topic, and date-range criteria.

**Acceptance Criteria**:
- The advanced-search dialog posts `a=search` with the following optional criteria, each constraining the listing predicate when supplied:
  - **Keyword** (`query`) — optional; same ≥3-char rule and same numeric/email/deep-search branching as basic search (FS-020.7).
  - **Status** — "Any Status", Open, Answered (only offered when "show answered tickets" is off), Overdue, or Closed. "Any Status" means no real-status constraint (all open-state semantics apply per FS-020.2).
  - **Department** (`deptId`) — restricted to the staff member's accessible departments; an out-of-scope department is ignored (BS-020.3).
  - **Assigned To** (`assignee`) — "Anyone", "Unassigned" (`s0`), "Me", any staff member (`s<id>`), or any team (`t<id>`). Staff prefix `s`, team prefix `t`. An assignee filter is only applied when the status is **not** "closed" (the case-insensitive comparison `strcasecmp(status,'closed')` must be truthy, i.e. status differs from "closed"). (Plain numeric with no prefix is treated as a staff id; a `t`-prefixed value filters by team, an `s`-prefixed value filters by staff.) The applied assignee predicate is wrapped as `(status='open' AND <staff_id|team_id>=<id>)`.
  - **Closed By** (`staffId`) — "Anyone" (value `0`, which is falsy and applies no filter), "Me", or any staff member; combined with assignee/status to also surface tickets closed by that staff member.
  - **Help Topic** (`topicId`) — "All Help Topics" or a specific topic.
  - **Date Range** (`startDate`, `endDate`) — bounds the ticket created date (`created >= startDate`, `created <= endDate`). Each bound must be at least 8 characters to be parsed (otherwise that bound is treated as unset / 0).
- The assignee/closed-by combination has three distinct query paths (BS-020.16):
  1. **Assignee supplied AND `staffId` supplied AND no `status`** → the predicate is `(assignee-open-clause) OR (staff_id=staffId AND status='closed')` — surfaces open-assigned-to-X plus closed-by-X in one result set.
  2. **Assignee supplied AND `staffId` is set (`isset`) but a `status` IS present** → the predicate is `(assignee-open-clause) OR status='closed'` — ORs in **all** closed tickets (within visibility scope), not just those closed by the named staff member.
  3. **Assignee NOT supplied (or status is closed) BUT `staffId` supplied** → a standalone `AND (staff_id=staffId AND status='closed')` clause is added with no assignee OR-branch.
- The dialog includes an asynchronous result-count area (a spinner + count placeholder) to preview how many tickets the criteria would return.
- All criteria that are applied are reflected in the query string so the search is preserved across sort, pagination, and export.

### FS-020.9: Mass / Bulk Actions From the Queue

**Description**: The system shall let qualified staff select multiple tickets in the listing and apply a single bulk action to all selected tickets.

**Acceptance Criteria**:
- The bulk-action bar and the per-row checkboxes are shown only when the staff member can mass-manage tickets (`canManageTickets()` — admin OR can-delete OR can-close, BS-020.9).
- Select-all / select-none / select-toggle helpers operate on the row checkboxes.
- The available bulk actions depend on the current queue:
  - **Closed queue** → Reopen.
  - **Open / Answered / Assigned queues** → Overdue (mark overdue) + Close.
  - **Overdue queue** → Close.
  - **Search (default)** → Close + Reopen.
  - **Delete** is additionally offered in every queue when the staff member can delete tickets.
- A bulk submit posts `a=mass_process` with `do=<action>` and the selected `tids[]`. The handler requires `canManageTickets()` and at least one selected ticket, then per action:
  - **reopen** — requires can-close OR can-create; reopens each closed selected ticket and logs an internal note.
  - **close** — requires can-close; closes each open selected ticket and logs a note.
  - **mark_overdue** — flags each not-already-overdue selected ticket overdue and logs a note.
  - **delete** — requires can-delete; deletes each selected ticket and writes a warning to the system log recording who deleted how many.
- Each bulk action reports an outcome message: all-succeeded ("Selected tickets (N) …"), partial ("N of M selected tickets …"), or failure ("Unable to … selected tickets"). The mutation logic for reopen/close/overdue/delete is owned by FS-021; this spec owns the selection, gating, and queue→action mapping.
- A confirmation dialog gates close/reopen/mark_overdue/delete before the bulk POST is sent.

### FS-020.10: Export Current Query to CSV

**Description**: The system shall let staff export the currently listed/searched tickets to a CSV file.

**Acceptance Criteria**:
- The listing renders an "Export" link only when at least one ticket is shown; it points to the queue page with `a=export`, a query token `h=<hash>`, and the current `status`. (Queue-side trigger is FS-020's.)
- The exact listing query (after all visibility, status, search, sort, and pagination predicates EXCEPT it is the query produced for the current page) is stored server-side in the session keyed by `search_<md5(query)>`, and the export link references that md5 token. (Session query-token storage is FS-020's.)
- On `a=export` with no selected ticket: the handler requires a query token (`h`), looks up the stored query for that token in the session, and streams the results. The export filename (`tickets-<YYYYMMDD>.csv`) and the literal error strings for missing/unknown token and dump failure are owned by **FS-090.23 / BS-090.12**.
- CSV generation/streaming is delegated to the shared export utility — see FS-090 for export mechanics.

### FS-020.11: Quick Ticket Stats

**Description**: The system shall compute the acting staff member's quick ticket counts used to drive the queue sub-menu, warnings, and notices.

**Acceptance Criteria**:
- Quick stats (`Ticket::getStaffStats`) return counts for the keys: `open`, `answered`, `overdue`, `assigned`, `closed`, scoped to the staff member's visibility (assigned-to-me-open OR my-team-open OR my-department). The visibility predicate is the OR of: tickets directly assigned to the staff member that are open; the staff member's teams' open tickets (when the staff member belongs to teams); and — only when the staff member is **not** access-limited (`showAssignedOnly()` false) and has at least one accessible department — tickets in those departments. (Implemented as a single computed `getStaffStats` query, not the listing query; the resulting counts may not exactly equal a queue's row count because the visibility ORs are composed differently.)
- `open` counts open & not-answered tickets in scope; `answered` counts open & answered; `overdue` counts open & overdue; `assigned` counts open tickets assigned to the staff member; `closed` counts closed tickets in scope.
- When the staff member is not configured to see assigned tickets (neither the system-wide `show_assigned_tickets` config nor the staff override returns true), the unassigned constraint (`staff_id=0`) is appended **only to the `open` count**, NOT to the `answered`, `overdue`, `assigned`, or `closed` counts (BS-020.17). (The listing Open queue applies the same `staff_id=0` constraint to the whole Open listing; the stat query applies it only to the `open` UNION branch.)
- The staff `show_assigned_tickets` override (`Staff::showAssignedTickets()`) feeds both the Open-queue assigned-visibility toggle (BS-020.4) and this `open`-count branch (BS-020.18); the admin-or-manager gate that decides whether the flag is honoured is owned by FS-002.
- Stats are cached per request (`getTicketsStats()` memoizes into `$this->stats['tickets']`) and reset (`resetStats()`) after a POST that produced no errors so the sub-menu counts reflect the just-made change.

### FS-020.12: Dashboard — Ticket Activity Chart

**Description**: The system shall present a staff dashboard with a line chart of ticket activity (by event state) over a selectable timeframe and period.

**Acceptance Criteria**:
- The dashboard page (`scp/dashboard.php`) requires a staff session, marks the "dashboard" nav tab active, and renders a "Ticket Activity" section with a timeframe form and a line-chart area, then a "Statistics" section (FS-020.13).
- The timeframe form offers a free-text **start** field (placeholder "Last month") and a **period** select with options: "Up to today" (`now`), "One Week" (`+7 days`), "Two Weeks" (`+14 days`), "One Month" (`+1 month`), and "One Quarter" (`+3 months`); a Refresh button reloads the chart.
- Chart data is fetched asynchronously from the overview report graph endpoint (`ajax.php/report/overview/graph`) in JSON, returning, per ticket-event **state** observed in the window, a daily time series of event counts, with missing days filled as zero.
- The date range resolves the start via the start field (default "last month") and the stop via the period; a period beginning with `+` is computed relative to the start date.
- Recognized event states are taken from the ticket-event log over the window (e.g., created/assigned/overdue/closed/reopened); the chart legend lets the user toggle individual state series on/off.
- Event rows flagged annulled are excluded from the counts.

### FS-020.13: Dashboard — Tabular Statistics

**Description**: The system shall present tabular ticket statistics grouped by department, help topic, or staff, with activity counts and average service/response times, downloadable as CSV.

**Acceptance Criteria**:
- The statistics block offers tab groups fetched from the overview report (`ajax.php/report/overview/table/groups`): **Department** (`dept`), **Topics** (`topic`), and **Staff** (`staff`). The default/first group is Department.
- For the selected group, the table reports per group row: **Opened**, **Assigned**, **Overdue**, **Closed**, **Reopened** counts (derived from the ticket-event log within the timeframe, excluding annulled events), plus average **Service Time** (avg whole days from created to closed) and average **Response Time** (avg whole days from a message to its first response), each formatted to 1 decimal.
- **Visibility scoping of the statistics**:
  - **Department** rows are limited to the staff member's accessible departments.
  - **Topic** rows are unrestricted.
  - **Staff** rows are limited to the staff member themself, plus (if the staff member manages departments) staff in those managed departments, plus (if the staff member can view staff stats) staff in their accessible departments.
- The table data is retrievable as JSON (for in-page rendering) and as a CSV download named `<group>-report.csv` (default group label "Department"); CSV streaming uses the shared download helper (FS-090).
- The statistics timeframe/period reuse the same start/period inputs as the activity chart (FS-020.12).

---

## Business Rules

### BS-020.1: `status` Parameter Is Overloaded (Queue Selector vs Real Status)

**Rule**: The `status` request parameter selects a predefined queue and is distinct from the real ticket status applied to the query. Some queue values (overdue, assigned, answered) map to real status `open` plus an additional flag filter; only `closed` maps to real status `closed`.

**Rationale**: A small fixed set of working queues is more useful to staff than exposing every raw status; the overload keeps the queue model compact while reusing the same listing machinery.

**Examples**:
- `status=overdue` runs against `status='open' AND isoverdue=1`.
- `status=assigned` runs against `status='open' AND staff_id=me`.
- `status=closed` runs against `status='closed'`.
- An unrecognized/empty `status` (without a search) defaults to the Open queue.

### BS-020.2: Department + Assignment Visibility Is Always Enforced

**Rule**: Every listing and search is wrapped in a visibility predicate restricting results to tickets the staff member may see: their open assigned tickets (always), their team's open tickets (if any), and their department's tickets (unless they are access-limited).

**Rationale**: Staff must not see tickets outside their remit; assignment and team membership intentionally widen visibility beyond the home department for the tickets that concern them.

**Examples**:
- A help-desk agent in Department "Support" sees all Support tickets plus any ticket assigned directly to them in another department.
- An access-limited agent sees only tickets assigned to them or their team, not the whole department.

### BS-020.3: Search Department Filter Is Bounded By Access

**Rule**: A `deptId` supplied in (advanced) search is applied only if that department is among the staff member's accessible departments; otherwise it is silently ignored.

**Rationale**: Search must not become a privilege-escalation path to view tickets in departments the staff member cannot access.

**Examples**:
- An agent with access to "Support" searching with `deptId=Sales` gets the Sales filter ignored and results limited to their normal scope.

### BS-020.4: Open Queue Answered/Assigned Visibility Toggles

**Rule**: On the Open queue (real status open, not a search), answered tickets are hidden unless "show answered tickets" is enabled, and assigned tickets are hidden (and the Assigned-To column suppressed) unless the system (`show_assigned_tickets` config) OR the staff member's effective override is configured to show assigned tickets. Whether the staff override is honoured is gated by the admin-or-manager primitive owned by FS-002 (see BS-020.18). When assigned tickets are hidden the constraint applied is `staff_id=0`, which does **not** account for team assignments — a ticket assigned to a team but no individual still counts as unassigned (KL-020.4).

**Rationale**: The Open queue is meant to surface work that needs action; answered or already-assigned tickets are filtered out by default to keep it actionable.

**Examples**:
- With "show answered" off, an answered open ticket does not appear on the Open queue but does appear on the Answered queue.
- With assigned-tickets display off, the Open queue shows only unassigned (`staff_id=0`) tickets and omits the Assigned-To column.

### BS-020.5: Rightmost Column Varies By Queue

**Rule**: The final listing column is "Assigned To" when assigned tickets are shown, "Closed By" on the Closed queue, and "Department" otherwise.

**Rationale**: The most relevant secondary attribute differs per queue — who owns the work (open), who resolved it (closed), or where it sits (department views).

**Examples**:
- Closed queue shows "Closed By: Jane Doe".
- Open queue with assigned display off shows "Department: Support".

### BS-020.6: Keyword Search Minimum Length

**Rule**: A search keyword must be at least 3 characters; shorter terms are rejected with a warning and the view falls back to the plain queue rather than erroring.

**Rationale**: Very short terms produce unbounded, low-value `LIKE` scans; the floor protects performance and result quality while degrading gracefully.

**Examples**:
- Searching "ab" yields "Search term must be more than 3 chars" and the default Open listing.
- Searching "abc" runs a deep search.

### BS-020.7: Keyword Resolution Strategy

**Rule**: Numeric keywords match the ticket number prefix; valid-email keywords match the requester email exactly; all other keywords trigger a deep `LIKE` search across email, name, subject, thread body, and thread title.

**Rationale**: Staff most often search by ticket number or requester email (fast, indexed-style matches); free text falls back to a broad content scan.

**Examples**:
- `12345` → ticket number prefix match.
- `jane@x.com` → exact email match.
- `printer jam` → deep content search joining the thread table.

### BS-020.8: Sticky Per-Queue Sort

**Rule**: The chosen sort column and direction are remembered per queue within the session and re-applied when the staff member returns to that queue without an explicit sort.

**Rationale**: Staff expect a queue to retain the ordering they last set while moving around the panel, but each queue can have a different natural ordering.

**Examples**:
- Sorting the Open queue by Date DESC persists when returning to Open; the Closed queue independently remembers its own sort.

### BS-020.9: Mass-Management Gating

**Rule**: Bulk actions, per-row checkboxes, and the mass-action bar are available only to staff who can mass-manage tickets — defined as administrators OR staff who can delete OR staff who can close tickets.

**Rationale**: Bulk operations are high-impact; they are restricted to staff trusted with destructive/closing permissions.

**Examples**:
- A reply-only agent sees the listing without checkboxes and cannot mass-process.
- An admin sees checkboxes and the full bulk-action bar.

### BS-020.10: Bulk Action Set Depends On Queue

**Rule**: The set of bulk actions offered is queue-specific (Closed→Reopen; Open/Answered/Assigned→Overdue+Close; Overdue→Close; Search→Close+Reopen), with Delete added in any queue for staff who can delete.

**Rationale**: Only transitions valid for the tickets in a given queue are offered, reducing no-op or invalid bulk operations.

**Examples**:
- On the Closed queue the only state action is Reopen.
- On the Overdue queue the only state action is Close.

### BS-020.11: Per-Action Permission Re-Check On Bulk

**Rule**: Each bulk action independently re-verifies the specific permission it needs (reopen needs close-or-create; close needs close; delete needs delete) in addition to the mass-management gate, and reports partial success when only some tickets could be acted on.

**Rationale**: Defense-in-depth: the action handler must not assume the rendered button set was authoritative, and individual tickets may fail a transition (already in target state, etc.).

**Examples**:
- A bulk close where 3 of 5 selected tickets are already closed reports "3 of 5 selected tickets closed".

### BS-020.12: Export Query Token Is Session-Bound

**Rule**: Export references the listing query via an md5 token stored in the session; export only proceeds if the token resolves to a stored query in the current session.

**Rationale**: The export reproduces exactly the query the staff member sees (including their visibility scope) without trusting client-supplied SQL or filters, and the session binding prevents replay by another user.

**Examples**:
- A valid `h=<hash>` whose query is in the session exports the CSV (filename owned by FS-090.23 / BS-090.12).
- A stale `h` after session loss yields the "token not found" error string owned by FS-090.23 / BS-090.12.

### BS-020.13: Auto-Refresh Only On Plain Queue Views

**Rule**: A staff member's configured auto-refresh rate injects a meta-refresh (rate × 60 seconds) only on a plain queue view — not on POSTs and not when an action (`a`) is in progress.

**Rationale**: Auto-refreshing during an edit, search, or form submission would lose work; refresh is limited to passive queue viewing.

**Examples**:
- Watching the Open queue with a 5-minute refresh rate reloads every 300 seconds.
- Mid-search or mid-edit, no auto-refresh is injected.

### BS-020.14: Quick-Stat Warning & Notice Thresholds

**Rule**: When a staff member has more than 10 tickets assigned to them, a warning banner is raised; when more than 10 overdue tickets are in scope, a system notice is raised — each only if no warning/notice is already set.

**Rationale**: Lightweight nudges surface backlog without overriding more important existing messages.

**Examples**:
- 12 assigned tickets → "12 tickets assigned to you! Do something about it!".
- 15 overdue tickets → "15 overdue tickets!" notice.

### BS-020.15: Dashboard Statistics Visibility Scoping

**Rule**: Dashboard department stats are limited to the staff member's accessible departments; staff stats are limited to themself plus managed-department staff plus (if permitted) accessible-department staff; topic stats are unrestricted.

**Rationale**: Aggregate reporting must respect the same access boundaries as ticket visibility, while topic-level rollups carry no per-ticket exposure.

**Examples**:
- A manager sees staff stats for everyone in the departments they manage; a plain agent sees only their own staff stats.

### BS-020.16: Assignee + Closed-By Search Has Three Query Shapes

**Rule**: The advanced-search assignee and closed-by criteria combine into three distinct query shapes depending on which are present and whether a status is set: (1) assignee + `staffId` + no status → open-assigned-to-X OR closed-by-X; (2) assignee + `staffId` set + a status present → open-assigned-to-X OR all-closed; (3) `staffId` only (no assignee, or status is closed) → standalone closed-by-X clause. The assignee filter itself is suppressed entirely when the search status equals "closed".

**Rationale**: The combined query lets a staff member see both what is currently assigned to someone and what they have already resolved; the branch on whether a status is present is a quirk of the overloaded `status` parameter and produces a broader (all-closed) result when a status is supplied alongside a closed-by filter.

**Examples**:
- Assignee = "Me", Closed By = "Me", Any Status → open tickets assigned to me plus closed tickets I closed.
- Assignee = "Me", Closed By = "Me", Status = Open → open tickets assigned to me plus every closed ticket in scope.

### BS-020.17: Unassigned Stat Constraint Applies Only To The Open Count

**Rule**: In the quick-stats query, when the staff member is not permitted to see assigned tickets, the `staff_id=0` (unassigned) constraint is appended only to the `open` count branch — the `answered`, `overdue`, `assigned`, and `closed` count branches are unaffected.

**Rationale**: The Open tab is the only one expected to surface "actionable, not-yet-claimed" work; the other tabs intentionally still count assigned tickets (e.g. the `assigned` count is by definition about assigned tickets, and the overdue/answered/closed tabs are explicitly chosen by the staff member).

**Examples**:
- An agent who cannot see assigned tickets has an Open count of only unassigned open tickets, but their Overdue count still includes overdue tickets that are assigned to others in their department.

### BS-020.18: Staff "Show Assigned Tickets" Override Feeds The Open-Queue Toggle

**Rule**: The per-staff `show_assigned_tickets` flag (`Staff::showAssignedTickets()`) feeds the Open-queue assigned-visibility toggle (BS-020.4) and the quick-stats `open`-count branch (BS-020.17). The admin-or-manager gate that determines when this flag is actually honoured is the access/flag-honouring primitive owned by **FS-002** (see FS-002, ~line 200); when the flag is not honoured only the system-wide `show_assigned_tickets` config governs assigned-ticket visibility.

**Rationale**: This spec owns how the resolved override drives the Open queue and the quick stats; FS-002 owns who is allowed to set it effectively.

**Examples**:
- When FS-002 honours the override for a staff member, their Open queue and `open` quick-stat include assigned tickets even when the system default hides them; otherwise the system-wide config alone governs.

---

## Data Requirements

> Canonical table names, column names, ticket status/source enums, priority/SLA models, and staff permission flags are owned by **FS-091**. Listed here are the functional fields the queue/dashboard read.

### Request / State Inputs

| Parameter | Role |
|-----------|------|
| `status` | Queue selector (open/answered/assigned/overdue/closed) — overloaded (BS-020.1) |
| `a` | Action: `search`, `export`, `mass_process`, `open` (create, FS-011), `edit`/`print` (FS-021) |
| `query` | Search keyword (≥3 chars) |
| `deptId`, `topicId`, `assignee`, `staffId`, `startDate`, `endDate` | Advanced-search criteria |
| `sort`, `order` | Sort column key + direction (ASC/DESC) |
| `p`, `limit` | Page number; per-request page-size override |
| `tids[]`, `do` | Selected ticket ids and bulk action |
| `basic_search` | Presence marks a basic-search submit; an empty `query` with `basic_search` set triggers the ≥3-char rejection warning |
| `t` | Opaque pass-through token echoed into the search query string (`&t=…`) on a search request; preserved across sort/pagination/export links but applies no listing filter |
| `h` | Export query token (md5 of stored query) |
| Session `<queue>_tickets` | Sticky per-queue `sort`+`order` |
| Session `search_<md5>` | Stored listing query for export |

### Listing Row Fields (read per ticket)

Internal id, external ticket number, department id/name, assigned staff id/team id (+ resolved names), subject, requester name/email, status, source, overdue flag, answered flag, created date, priority (color/description/urgency), attachment count, thread count, computed due date, computed effective date, computed assignee name, lock presence (non-self, unexpired), help topic (with parent topic).

### Quick Stats

`open`, `answered`, `overdue`, `assigned`, `closed` counts scoped to the staff member's visibility (FS-020.11).

### Dashboard Data

- **Chart**: per ticket-event state, a daily series of non-annulled event counts over the window (states discovered from the event log).
- **Table**: per group (department/topic/staff) row of Opened/Assigned/Overdue/Closed/Reopened counts + avg Service Time + avg Response Time.

### Page Size Resolution

`PAGE_LIMIT` resolution (personal → system default → `DEFAULT_PAGE_LIMIT`, configurable range 5–50 step 5) owned by FS-090.19; the queue-only `limit` per-request override is FS-020's.

---

## User Flows / Interactions

### Flow 1: Work the Open Queue
1. Staff lands on the staff root, which renders the Open queue (default).
2. The sub-menu shows Open / (Answered) / My Tickets / Overdue / Closed with counts.
3. Staff sorts by Date, pages through results, opens a ticket via its number/subject link (FS-021).

### Flow 2: Keyword Search
1. Staff types ≥3 chars in the basic search box and submits.
2. The keyword resolves (number / email / deep search) and the listing shows results with a "(Search Results)" label.
3. Staff clicks "[advanced]" to add status, department, assignee, topic, or date-range filters and re-searches.

### Flow 3: Bulk Close
1. A mass-manage-capable staff member ticks several open tickets.
2. They click "Close"; a confirmation dialog appears; on confirm the form posts `a=mass_process do=close tids[]`.
3. Each open selected ticket is closed and noted; an outcome message reports full/partial success; the sub-menu counts refresh.

### Flow 4: Export the Current Listing
1. The current listing query is stored under an md5 token in the session.
2. Staff clicks "Export"; the handler resolves the token and streams the CSV (filename owned by FS-090.23 / BS-090.12).

### Flow 5: Dashboard Review
1. Staff opens the dashboard; the activity chart loads for the default timeframe.
2. Staff sets a start date and period and refreshes; the chart re-plots per-state daily counts.
3. Staff switches the statistics tab between Department / Topics / Staff and optionally downloads the group's CSV.

---

## Edge Cases & Error Scenarios

### EC-020.1: Sub-3-Character Search
A keyword under 3 chars (or empty with basic search submitted) is rejected with "Search term must be more than 3 chars" and the view degrades to the plain queue rather than showing an error page.

### EC-020.2: Out-of-Scope Department Filter
A search `deptId` outside the staff member's access is silently dropped; results stay within the staff member's normal visibility (BS-020.3).

### EC-020.3: Invalid Date Span
If the start date is in the future, or the start is after the end (with a non-zero end), the date filter is rejected with "Entered date span is invalid. Selection ignored." and both bounds are cleared.

### EC-020.4: Empty Result Set
A query returning no rows shows "There are no tickets here. (Leave a little early today)." (or "Query returned 0 results.") and renders no pagination/action bar.

### EC-020.5: Missing / Stale Export Token
`a=export` with no `h`, an `h` whose query is not in the session, and an export failure each surface a distinct error (literal strings owned by FS-090.23 / BS-090.12).

### EC-020.6: Lock Held By Another Staff
A ticket locked by a different, unexpired staff lock is flagged "locked" in the listing; locks held by the viewing staff member or expired locks are not flagged (and do not appear bold-open).

### EC-020.7: Partial Bulk Action
When only some selected tickets satisfy the action's preconditions, the action reports "N of M selected tickets …" rather than full success or failure (BS-020.11).

### EC-020.8: Bulk Action Without Permission
A `mass_process` POST from a staff member lacking the specific permission (or lacking the mass-management gate) is rejected with a permission-denied message and no tickets are changed.

### EC-020.9: No Tickets Selected For Bulk
A `mass_process` with empty/absent `tids[]` yields "No tickets selected. You must select at least one ticket."

### EC-020.10: Search With No Status Column Swap
A search with no status renders a Status column in place of Priority, and bolds "Open" status rows; normal queues always render the colored Priority column.

### EC-020.11: Sticky Sort Across Queues
Switching from Open (sorted by Date) to Closed shows Closed's own remembered/default sort, not Open's — each queue's sort is independent (BS-020.8).

### EC-020.12: Page Beyond Range
A `p` beyond the available pages is normalized by the pagination component (start reset toward a valid range) so an out-of-range page does not error (mechanics in FS-090).

### EC-020.13: Unknown Bulk Action
A `mass_process` POST whose `do` value is none of `reopen`/`close`/`mark_overdue`/`delete` (after the mass-management and non-empty-selection gates pass) is rejected with "Unknown or unsupported action - get technical help" and no tickets are changed.

### EC-020.14: Out-Of-Scope Search Department Filter (Detail)
The `deptId` department filter is honored only when the value is in `getDepts()` (the staff member's accessible-department list); an out-of-scope value is dropped without error, and the advanced-search Department select itself only lists departments that are both system-defined and in the staff member's accessible set (BS-020.3).

---

## Dependencies

| Dependency | Specification | Relationship |
|------------|---------------|--------------|
| Staff session, auth gate, `OSTSCPINC`/`isStaff()` | FS-002 | Guards queue/dashboard access |
| Department/group access model, teams | FS-031 / FS-091 | Provides the staff member's accessible departments + teams used in visibility scoping |
| Permission flags (`canManageTickets`, `canCloseTickets`, `canDeleteTickets`, `canCreateTickets`, `canViewStaffStats`, `showAssignedOnly`, etc.) | FS-091 | Gate columns, bulk actions, and stat visibility |
| Single-ticket view & per-ticket actions (reply/note/assign/transfer/close/reopen/lock/edit/print) | FS-021 | Shares the `scp/tickets.php` entry script; queue rows link into it; bulk mutation logic lives there |
| Ticket creation (`a=open`) | FS-011 | "New Ticket" sub-menu + open handler |
| Pagination (`Pagenate`) | FS-090 | Page windowing, "Showing … of N", page links |
| CSV export (`Export::saveTickets`) and CSV download (`Http::download`) | FS-090 | Queue export + dashboard table download mechanics |
| Navigation (`StaffNav` sub-menu, tab activation, warnings/notices) | FS-001 / FS-090 | Renders the queue tabs and dashboard tab |
| Ticket / thread / event / priority / SLA / lock models, enums, table names | FS-091 | Canonical data model the queries read |
| System config (`show_answered_tickets`, `show_assigned_tickets`, `max_page_size`) | FS-032 / FS-091 | Drives queue toggles and default page size |
| Date/format helpers (`Format::db_date`, `Format::truncate`) | FS-003 | Cell formatting |
| Validation (`Validator::is_email`) | FS-003 | Keyword email detection |

---

## Known Limitations

### KL-020.1: `status` Overloading Is Confusing By Design
The `status` parameter conflates queue selection with the real ticket status, and several queues (overdue/assigned/answered) silently rewrite it to `open` plus a flag. Source comments themselves call this out ("you've got to just have faith!"). The conflation makes the listing logic hard to follow and easy to break.

### KL-020.2: Deep Search Is Unbounded `LIKE` Scans
Free-text deep search runs leading-and-trailing-wildcard `LIKE` across email, name, subject, and the joined thread body/title — a full scan with no full-text indexing. On large datasets this is slow; the 3-char floor only partially mitigates it.

### KL-020.3: No Per-Staff Filtering For Admins/Managers In Queue
A TODO in the code notes that admins/managers cannot yet limit the queue to a single staff member's tickets (`$staffId=0; //Nothing for now…`). The listing only filters to "me" via the Assigned queue; arbitrary per-staff queue filtering is only available through advanced search's assignee/closed-by criteria, not as a first-class queue.

### KL-020.4: Assigned-Tickets Open-Queue Filter Ignores Team Assignments
When the Open queue hides assigned tickets, it constrains to `staff_id=0` only and does not factor in team assignments (`XXX: NOT factoring in team assignments`), so a ticket assigned to a team but no individual can still appear as "unassigned" on the Open queue.

### KL-020.5: Dashboard Timezone & Annulled Handling Are Incomplete
The activity chart carries TODOs for user↔DB timezone offset handling, and the ticket-event annulled column handling is partial (`XXX: Implement annulled column`); cross-timezone day bucketing and fully consistent annulment filtering are not guaranteed.

### KL-020.6: Export Reflects The Current Page's Query, Bound To Session
Export is keyed to the exact stored query (including its `LIMIT`), and depends on the session still holding the `search_<md5>` entry. A lost or rotated session invalidates the export link, and the export is not a streaming re-run of the full unpaginated result set independent of the viewing context.

### KL-020.7: Static "Statistics" Group Set
The dashboard tabular grouping is fixed to Department / Topics / Staff; there is no custom grouping, custom date presets beyond the five period options, or per-priority/per-SLA breakdown in this version.

---

## Future Considerations

- Replace `status` overloading with explicit, named saved queues / custom filters.
- Full-text indexing for keyword/deep search to remove unbounded `LIKE` scans.
- First-class per-staff and per-team queue filtering for managers/admins (resolve KL-020.3/KL-020.4).
- Timezone-correct dashboard bucketing and complete annulled-event handling.
- Streaming, context-independent export of the full filtered result set.
- Configurable dashboard groupings, custom date ranges, and priority/SLA breakdowns.
