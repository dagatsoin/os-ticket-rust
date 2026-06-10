# FS-090: Shared UI, Navigation & Data Export

## Overview

This specification defines the cross-cutting presentation and data-output infrastructure that every page of the osTicket helpdesk composes with — the parts that are not specific to any one domain (tickets, email, knowledge base, etc.) but are reused everywhere. It covers three concern groups:

1. **Navigation assembly** — the menu construction layer that produces the staff control-panel top tabs and per-tab sub-menus, the admin control-panel tabs and sub-menus, and the public client portal navigation links. Menus are assembled in code (not hand-authored per page), are permission- and configuration-conditional, and carry their own active-state model.
2. **Page chrome (header/footer)** — the shared HTML shell that wraps staff and client pages: the document head (title, asset references, extra-header injection point), the logged-in identity banner, the global system message bars (error / warning / notice), the per-page message bars, the navigation render loop, the footer copyright line, the web-triggered auto-cron beacon, and the shared overlay/loading elements.
3. **Generic table & data mechanics** — the pagination component (page window, "showing X–Y of N", page-link rendering, URL parameter handling), the result-set export pipeline (CSV and JSON exporters driven by a header→column map), the ticket-queue CSV export flow (search-token-backed), the file-download response helper, the report tabular/CSV export helper, and the full-database backup exporter.

This spec owns the **generic mechanics** of these surfaces. It explicitly does **not** own:

- The **dashboard / overview report data** itself (the statistics, plot series, date-range semantics) — that belongs to **FS-020** (staff ticket queue, dashboard & search). FS-090 documents only the shared *export/table* mechanics of the report helper (the CSV download path and the encoder reuse).
- **Enum sets, table names, and column schemas** — those are canonically owned by **FS-091** (reference data, enums & data model) and are referenced here, not restated.
- The **permission flags themselves** (`canCreateTickets`, `canManageFAQ`, etc.) and the **session/identity objects** (`$thisstaff`, `$thisclient`, `$ost`, `$cfg`) — owned by **FS-002** (staff auth) and **FS-001** (bootstrap). FS-090 documents only how navigation visibility *consumes* those flags.

All persistence is MySQL accessed through global procedural helpers; table names use a configurable prefix (default `ost_`). See FS-091 for the canonical schema and FS-001 for the bootstrap chain that populates the globals this spec reads.

---

## Functional Requirements

### FS-090.1: Staff Control-Panel Top Tabs

**Description**: The system shall assemble the staff control-panel top navigation as a fixed, ordered set of tabs, each carrying a display label, target page, and tooltip title.

**Acceptance Criteria**:
- The staff navigation object is constructed for a given staff identity and a panel name (default panel `staff`).
- The staff top tabs are exactly three, in this order:
  1. `dashboard` — label "Dashboard", target `dashboard.php`, title "Staff Dashboard".
  2. `tickets` — label "Tickets", target `tickets.php`, title "Ticket Queue".
  3. `kbase` — label "Knowledgebase", target `kb.php`, title "Knowledgebase".
- Tab labels are rendered verbatim from the assembled structure; the active tab is rendered with CSS class `active`, all others with class `inactive`.
- Each tab key is keyed by a lowercase identifier; the panel-qualified key form is `<panel>.<tabkey>` (e.g. `staff.tickets`) used to index sub-menus.

### FS-090.2: Staff Sub-Menus (Permission-Conditional)

**Description**: The system shall assemble a per-tab sub-menu list for the staff panel, conditionally including items based on the current staff member's permissions and ticket counts.

**Acceptance Criteria**:
- The `tickets` tab sub-menu always begins with a "Tickets" item (target `tickets.php`, icon class `Ticket`) flagged `droponly` (drop-down-only; see FS-090.9). When a staff identity is present:
  - If the staff member has at least one assigned ticket, a "My Tickets (N)" item is added (target `tickets.php?status=assigned`, icon class `assignedTickets`, `droponly`), where N is the assigned-ticket count.
  - If the staff member is permitted to create tickets, a "New Ticket" item is added (target `tickets.php?a=open`, icon class `newTicket`, `droponly`).
- The `dashboard` tab sub-menu is unconditional: "Dashboard" (`dashboard.php`, icon `logs`), "Staff Directory" (`directory.php`, icon `teams`), "My Profile" (`profile.php`, icon `users`).
- The `kbase` tab sub-menu always begins with "FAQs" (target `kb.php`, also matching URL `faq.php`, icon `kb`). When a staff identity is present:
  - If the staff member may manage the FAQ, a "Categories" item is added (`categories.php`, icon `faq-categories`).
  - If the staff member may manage canned responses, a "Canned Responses" item is added (`canned.php`, icon `canned`).
- Non-breaking spaces in labels (e.g. "My&nbsp;Tickets") are preserved literally as part of the label text.
- A sub-menu is registered under the panel-qualified key `<panel>.<tabkey>` only if it is non-empty.

### FS-090.3: Admin Control-Panel Tabs & Sub-Menus

**Description**: The system shall assemble the admin control-panel navigation as a distinct set of tabs and sub-menus (the admin navigation extends the staff navigation but overrides both the tab set and the sub-menu set; panel name `admin`).

**Acceptance Criteria**:
- Admin top tabs are exactly five, in order:
  1. `dashboard` — "Dashboard", `logs.php`, title "Admin Dashboard".
  2. `settings` — "Settings", `settings.php`, title "System Settings".
  3. `manage` — "Manage", `helptopics.php`, title "Manage Options".
  4. `emails` — "Emails", `emails.php`, title "Email Settings".
  5. `staff` — "Staff", `staff.php`, title "Manage Staff".
- Admin sub-menus (all unconditional — admin navigation does not gate sub-items by permission):
  - `dashboard`: "System Logs" (`logs.php`, icon `logs`).
  - `settings`: "System Preferences" (`settings.php?t=system`), "Tickets" (`settings.php?t=tickets`), "Emails" (`settings.php?t=emails`), "Pages" (`settings.php?t=pages`), "Knowledgebase" (`settings.php?t=kb`), "Autoresponder" (`settings.php?t=autoresp`), "Alerts & Notices" (`settings.php?t=alerts`) — each with its respective icon class.
  - `manage`: "Help Topics" (`helptopics.php`), "Ticket Filters" (`filters.php`), "SLA Plans" (`slas.php`), "API Keys" (`apikeys.php`), "Site Pages" (`pages.php`).
  - `emails`: "Emails" (`emails.php`), "Banlist" (`banlist.php`), "Templates" (`templates.php`), "Diagnostic" (`emailtest.php`).
  - `staff`: "Staff Members" (`staff.php`), "Teams" (`teams.php`), "Groups" (`groups.php`), "Departments" (`departments.php`).
- The admin sub-menu tab labels and the settings query parameter `t=<section>` are the canonical routing keys for the admin settings sections (the settings sections themselves are owned by FS-032).

### FS-090.4: Client Portal Navigation Links

**Description**: The system shall assemble the public client portal navigation as an ordered set of links whose membership depends on system configuration and whether the visitor is an authenticated client.

**Acceptance Criteria**:
- The client navigation is constructed for an optional client identity and an optional active-link key.
- Link assembly order and conditions:
  1. `home` — "Support Center Home" (`index.php`) — always present.
  2. `kb` — "Knowledgebase" (`kb/index.php`) — present **only if** the knowledge base is enabled in configuration.
  3. `new` — "Open New Ticket" (`open.php`) — always present.
  4. If a valid client identity is present:
     - If "show related tickets" is enabled in configuration, a `tickets` link "My Tickets (N)" (`tickets.php`) where N is the client's ticket count.
     - Otherwise a `tickets` link "View Ticket Thread" (`tickets.php?id=<ticketID>`) pointing at the single ticket the client is viewing.
  5. If **no** valid client identity is present, a `status` link "Check Ticket Status" (`view.php`).
- Client nav links are rendered with `ROOT_PATH` prepended to their target. Each link anchor always carries two space-separated CSS classes: an active marker that is the literal `active` when the link is the active one and the empty string otherwise, followed by a class equal to the link key (the key class is emitted for **every** link regardless of active state, not only the active one). (See FS-090.28 for the corrected render contract.)
- A client-portal page rendered without a navigation object (no `$nav`) renders a horizontal rule in place of the nav bar instead of the link list.

### FS-090.5: Active-State Model — Tabs

**Description**: The navigation shall maintain a single active top tab, exposing setters and getters that flip the active flag and clear any previously active tab.

**Acceptance Criteria**:
- Setting a tab active marks that tab's `active` flag true and, if a different tab was previously active, clears that prior tab's flag (mutual exclusion of the active tab).
- Setting an active tab may optionally also set an active sub-menu in the same call.
- A request to set a tab active that does not exist in the tab set has no effect and reports failure (returns false).
- The currently active tab key is retrievable.

### FS-090.6: Active-State Model — Sub-Menus

**Description**: The navigation shall track an active sub-menu within the active tab, addressable either by 1-based numeric index or by matching a sub-menu item's target.

**Acceptance Criteria**:
- A numeric active-sub-menu value is stored directly as the active menu index.
- A non-numeric value, given a tab, is matched against the targets (`href`) of that tab's sub-navigation; the first item whose target matches is selected and its 1-based position becomes the active menu index.
- Sub-menu items may be appended at runtime to the active tab's sub-menu list; appending with the active flag set makes the newly appended item the active sub-menu.
- The active sub-menu index is retrievable.
- A staff page may explicitly clear the active sub-menu (e.g. the ticket search flow sets the active sub-menu to a sentinel value when a search has no status), suppressing sub-menu highlight.

### FS-090.7: Sub-Menu Auto-Highlight by Current Script

**Description**: When no explicit active sub-menu has been set, the sub-navigation render shall auto-highlight the item that corresponds to the currently executing page.

**Acceptance Criteria**:
- If an explicit active-menu index is set and resolves to an existing sub-item, that item is highlighted (class `active` appended to its icon class).
- If an explicit active-menu index is set but out of range, it is treated as unset (reset to 0).
- When no explicit active-menu index is set, an item is auto-highlighted if either:
  - the current script base name (case-insensitive) is contained in the item's target (`href`), OR
  - the current script base name appears in the item's additional matching-URL list (`urls`).
- Example: the FAQ sub-item declares an extra matching URL `faq.php` so the "FAQs" sub-item highlights on both `kb.php` and `faq.php`.

### FS-090.8: Top-Nav Render with Hover Sub-Menu

**Description**: The staff/admin header shall render the top tabs as a list, attaching each inactive tab's sub-menu as a nested drop-down list.

**Acceptance Criteria**:
- Each top tab renders as a list item carrying class `active` or `inactive` and an anchor to the tab target with the tab label.
- For a tab that is **not** the active tab and has a sub-menu, the sub-menu is rendered as a nested list (drop-down) of anchors, each with the item's icon class, target, title, and label.
- The active tab does **not** render a nested hover drop-down (its sub-menu is rendered in the dedicated sub-nav bar instead — FS-090.9).

### FS-090.9: Sub-Nav Bar Render & `droponly` Suppression

**Description**: The staff/admin header shall render a dedicated sub-navigation bar for the active tab's sub-menu, omitting items marked drop-down-only.

**Acceptance Criteria**:
- The sub-nav bar iterates the active tab's sub-menu in order.
- An item flagged `droponly` is **skipped** in the sub-nav bar (it appears only in the hover drop-down, FS-090.8).
- A sub-item is rendered with class `active` (appended to its icon class) when it is the resolved active menu item per FS-090.6/FS-090.7; otherwise with just its icon class.
- Example: in the staff `tickets` tab the "Tickets", "My Tickets (N)", and "New Ticket" items are all `droponly` and therefore appear only in the hover drop-down, never in the sub-nav bar.

### FS-090.10: Staff Page Chrome — Head & Identity Banner

**Description**: The staff header shall emit the document head and a logged-in identity banner with panel-switch and account links.

**Acceptance Criteria**:
- Response content type is `text/html; charset=UTF-8`.
- The document title is the page title supplied by the request context if present, otherwise the literal "osTicket :: Staff Control Panel".
- The head references the staff stylesheet/script asset bundle and an extra-header injection point: any extra headers registered on the request context are emitted into the head (used, e.g., for the auto-refresh `<meta http-equiv="refresh">` tag the ticket queue adds per the staff member's configured refresh rate).
- The identity banner greets the staff member by first name ("Welcome, <FirstName>").
- The banner shows a panel-switch link: "Admin Panel" (`admin.php`) when the staff member is an admin and the current page is **not** an admin page; otherwise "Staff Panel" (`index.php`).
- The banner always shows "My Preferences" (`profile.php`) and "Log Out" (`logout.php?auth=<link-token>`), where the logout link carries an anti-CSRF link token.

### FS-090.11: Client Page Chrome — Head & Identity Banner

**Description**: The client header shall emit the document head and an identity line distinguishing authenticated clients from guests.

**Acceptance Criteria**:
- Response content type is `text/html; charset=UTF-8`.
- The document title is the configured helpdesk title if set, otherwise the literal "osTicket :: Support Ticket System"; the title is HTML-escaped.
- The head includes a responsive viewport meta tag, description/keywords meta tags, the public stylesheet bundle (screen + print), and the public script bundle.
- The header logo image is served via `logo.php` (path-prefixed with `ROOT_PATH`) with the configured helpdesk title as its alt text and is wrapped in an anchor to `index.php` carrying the fixed anchor title attribute "Support Center". The alt text is sourced from `$ost->getConfig()->getTitle()` (the same configured helpdesk title used for the document `<title>`, reached via the bootstrap config object rather than the local `$title` variable).
- When a valid client identity is present: the banner shows the client's name, a "My Tickets (N)" link (`tickets.php`) **only if** "show related tickets" is enabled, and a "Log Out" link (`logout.php?auth=<link-token>`).
- When no valid client identity is present but a navigation object exists: the banner shows "Guest User - Log In" (`login.php`).
- All client-side target paths are prefixed with `ROOT_PATH`.

### FS-090.12: System Message Bars (Global)

**Description**: The header chrome shall render at most one global system message bar reflecting the request context's error / warning / notice state, in priority order.

**Acceptance Criteria**:
- Exactly one of three global bars renders, evaluated in priority order: error first, then warning, then notice. If an error is set, the warning and notice are suppressed; if no error but a warning is set, the notice is suppressed.
- Bar container ids are `error_bar`, `warning_bar`, `notice_bar` respectively.
- This global bar reflects request-context state set by the bootstrap/application layer (FS-001), distinct from the per-page message bars in FS-090.13.

### FS-090.13: Per-Page Message Bars (Form Feedback)

**Description**: The content region of both staff and client chrome shall render at most one per-page message bar reflecting page-level form-handler feedback, in priority order.

**Acceptance Criteria**:
- Exactly one of three per-page bars renders, in priority order: error (`msg_error`, from the form-handler errors array's top-level `err` entry), then notice (`msg_notice`, from the page message variable), then warning (`msg_warning`, from the page warning variable).
- These bars are populated by individual page handlers (the `$errors['err']`, `$msg`, `$warn` page-local variables), not by the global request-context state of FS-090.12.
- A page may show both a global bar (FS-090.12) and a per-page bar (FS-090.13) simultaneously, as they draw from independent sources.

### FS-090.14: Footer Chrome & Auto-Cron Beacon

**Description**: The footer chrome shall render the copyright line, shared overlay/loading elements, and — on staff pages — a web-triggered auto-cron beacon.

**Acceptance Criteria**:
- The staff footer renders a copyright line "Copyright © 2006-<current year> osTicket.com. All Rights Reserved." with the current year computed at render time.
- The client footer renders "Copyright © <current year> osTicket.com - All rights reserved." plus a "Powered by osTicket" link.
- Both footers render a hidden overlay element (`#overlay`) and a hidden "Please Wait!" loading element (`#loading`) used by client-side scripts.
- On staff pages, **only when the current identity is a real staff member**, the footer embeds a 1×1 invisible image beacon pointing at `autocron.php`; this beacon is the web-triggered fallback that keeps the cron running on page views. The accompanying HTML comments explicitly warn against removing the beacon. (The auto-cron mechanism itself is owned by FS-043.)

### FS-090.15: Pagination — Window & Bounds Computation

**Description**: The system shall provide a reusable pagination component that, given a total record count, a requested page, and a page size, computes the result-window start offset, the page size limit, the total page count, and the current page.

**Acceptance Criteria**:
- The component is constructed from: total count, requested page, page size (default 20), and an optional base URL.
- The effective page size is at least 1 (a page size below 1 is clamped up to 1).
- The requested page is at least 1 (a page below 1 is clamped up to 1).
- The start offset is `(page - 1) × limit`, floored at 0.
- The total page count is `ceil(total / limit)`.
- If the limit exceeds the total, or the requested page exceeds the last page, the start offset is reset to 0 (out-of-range requests fall back to the first page's data window).
- The component exposes the start offset, the page-size limit, the total number of pages, and the current page (re-derived from the start offset and limit).

### FS-090.16: Pagination — "Showing X – Y of N"

**Description**: The pagination component shall render a human-readable summary of the currently displayed record range.

**Acceptance Criteria**:
- The summary text is of the form "Showing X - Y of N" where X is the 1-based first record on the page (`start + 1`), Y is the last record on the page, and N is the total.
- Y is `start + limit` when a full page remains, otherwise the total (the final partial page shows fewer records).
- When the total is zero, the summary renders "Showing 0" (no range).
- Calling pages append a domain-specific suffix to the summary (e.g. "… SLA plans", "… filters", "… categories", "… emails", "… help topics", "… API Keys", "… Templates", "… premade responses"); the base component contributes only the "Showing X - Y of N" portion.

### FS-090.17: Pagination — Page Links & Sliding Window

**Description**: The pagination component shall render a clickable list of page-number links with a sliding window around the current page and jump-back / jump-forward chevrons.

**Acceptance Criteria**:
- The displayed span is fixed at 5 pages on each side of the current page (a window of up to 11 page numbers centered on the current page), with edge-compensation so the window stays full near the first or last page.
- The current page renders as a non-link bracketed bold marker `[i]`; every other page in the window renders as an anchor to the base URL with `&p=<i>`.
- If the window does not start at page 1, a left double-angle chevron `«` link is prepended, jumping back by the displayed span (clamped at page 1).
- If the window does not reach the last page, a right double-angle chevron `»` link is appended, jumping forward by the displayed span (clamped at the last page).
- Page links are rendered inside a "Page: …" label by the calling pages.

### FS-090.18: Pagination — URL Construction & Query Vars

**Description**: The pagination component shall build its page-link base URL from an optional supplied URL and an optional query-variable string, defaulting to the current page when no URL is supplied.

**Acceptance Criteria**:
- If a base URL is supplied and does not already contain a `?`, a `?` is appended so query variables can follow.
- If no base URL is supplied, the current page constant (`THISPAGE`) followed by `?` is used.
- An optional query-variable string is appended to the base URL; the page parameter `&p=<i>` is added per-link on top of that.
- Calling pages supply the carry-over query string so that sort/order/filter/status parameters survive page navigation (e.g. the ticket queue sets the URL to `tickets.php` with the carried query string plus URL-encoded `sort` and `order`).

### FS-090.19: Page-Size Resolution Hierarchy

**Description**: The effective page size for paginated lists shall be resolved from a defined hierarchy of per-staff, per-request, and system-default sources.

**Acceptance Criteria**:
- A system default page size is derived at bootstrap from the configured page size, falling back to **25** when no configured value exists (`DEFAULT_PAGE_LIMIT`).
- For staff pages, the effective page size (`PAGE_LIMIT`) is the staff member's configured page limit if set, otherwise the system default.
- For client pages, the effective page size (`PAGE_LIMIT`) is the system default.
- On the ticket queue specifically, an explicit numeric `limit` request parameter overrides `PAGE_LIMIT` for that request.
- The requested page number is taken from the `p` request parameter (numeric), defaulting to page 1.

### FS-090.20: Result-Set Export — Header-Mapped Column Projection

**Description**: The system shall provide a generic result-set exporter that runs a query and projects its rows into an output stream using a caller-supplied map of source field → output column header.

**Acceptance Criteria**:
- The exporter is constructed from a SQL query, a header map (field key → human column name), and an optional filter flag.
- Any trailing `LIMIT` clause in the supplied query is stripped before execution (export is unbounded — it exports the full result set, not a single page).
- Only header-map fields that actually exist as columns in the first result row are included; the exporter records, for each included field, its human header, its key, and its column position in the result set (positional lookups avoid per-cell hashtable lookups).
- A header-map field that is not present in the query result is silently dropped from the output (no column, no error).
- The exporter can be driven row-by-row (`next` returns a positional record) or as an associative record keyed by field (`nextArray`).
- The export format is selected by a `how` argument resolving to a concrete exporter; supported formats are `csv` and `json`.

### FS-090.21: CSV Exporter Format Rules

**Description**: The CSV exporter shall emit a header row followed by one quoted row per record, escaping embedded quotes.

**Acceptance Criteria**:
- Every field — header and data — is wrapped in double quotes.
- Fields within a row are joined by a comma; the all-fields-quoted form is `"a","b","c"`.
- Each row is terminated by a newline (`\n`).
- A literal double-quote inside a value is escaped by doubling it (`"` → `""`).
- The first emitted line is the header row (the human column names from the header map).
- No explicit character-encoding transformation or byte-order mark is applied; values are emitted as stored (the response declares `text/csv` for download — FS-090.24).

### FS-090.22: JSON Exporter Format

**Description**: The system shall provide a JSON variant of the result-set exporter that emits the full result set as a JSON-encoded array of associative records.

**Acceptance Criteria**:
- The JSON exporter collects every row as an associative record (field key → value) and emits the entire collection encoded as JSON via the shared JSON encoder.
- The JSON exporter is selected when the export `how` argument is `json`.

### FS-090.23: Ticket-Queue CSV Export Flow (Token-Backed)

**Description**: The system shall export the staff ticket queue / search results to CSV using a session-stored query token rather than re-deriving the query from request parameters.

**Acceptance Criteria**:
- When the ticket-queue page renders a result set, it stores the executed query (the full search SQL, **including its trailing `LIMIT <start>,<limit>` clause** — the export pipeline strips that LIMIT later per FS-090.20) in the session under the key `search_<hash>`, where `<hash>` is the MD5 digest of the exact query string. It exposes an "Export" link of the form `?a=export&h=<hash>&status=<status>`, where `<status>` is the current queue status filter carried through verbatim.
- The export action requires a query token (`h`); a missing token yields the error "Query token required".
- The token is looked up in the session (`search_<token>`); a token not found in the session yields "Query token not found".
- On a valid token, the system exports the stored query as CSV with the fixed ticket column set (FS-090.25) under the filename `tickets-<YYYYMMDD>.csv`, where the date is the current date.
- A failure to produce output yields "Internal error: Unable to dump query results".
- The export is therefore bounded to a query the staff member has already run and whose results they were authorized to see (the export reuses the already-executed search SQL from session).

### FS-090.24: File-Download Response Helper

**Description**: The system shall provide a shared helper that sends an arbitrary payload to the browser as a file download with appropriate headers.

**Acceptance Criteria**:
- The helper is invoked with a filename, a MIME content type, and the payload data.
- It emits no-cache / revalidation headers (`Pragma: public`, `Expires: 0`, `Cache-Control: must-revalidate, post-check=0, pre-check=0`, `Cache-Control: public`).
- It emits the supplied content type, a binary transfer encoding, and a content-disposition attachment header carrying the base name of the supplied filename.
- For Internet-Explorer-on-Windows user agents, the content-disposition omits the `attachment;` keyword (a legacy IE download workaround) while still carrying the filename.
- When payload data is supplied, it emits a content-length header, prints the payload, and terminates the request.
- The CSV ticket export buffers the exporter's output and passes it to this helper with content type `text/<how>` (e.g. `text/csv`).

### FS-090.25: Ticket Export Column Set

**Description**: The system shall export tickets with a fixed, ordered set of human-named columns mapped from query result fields.

**Acceptance Criteria**:
- The ticket export column map (source field → output header), in order, is:
  - `ticketID` → "Ticket Id"
  - `created` → "Date"
  - `subject` → "Subject"
  - `name` → "From"
  - `priority_desc` → "Priority"
  - `dept_name` → "Department"
  - `helptopic` → "Help Topic"
  - `source` → "Source"
  - `status` → "Current Status"
  - `effective_date` → "Last Updated"
  - `duedate` → "Due Date"
  - `isoverdue` → "Overdue"
  - `isanswered` → "Answered"
  - `assigned` → "Assigned To"
  - `staff` → "Staff Assigned"
  - `team` → "Team Assigned"
  - `thread_count` → "Thread Count"
  - `attachments` → "Attachment Count"
- Any of these fields absent from the query result is dropped per the generic projection rule (FS-090.20); the export is resilient to the queue query omitting a column.

### FS-090.26: Report Tabular CSV Export Helper (Shared Mechanics Only)

**Description**: The reporting helper shall provide a CSV download of its computed tabular report data, reusing the file-download helper. (The report *data computation* — statistics, date range, plot series — is owned by FS-020; FS-090 owns only the export mechanic.)

**Acceptance Criteria**:
- The report tabular download composes a CSV by quoting and comma-joining the report's column headers as the first line, then quoting and comma-joining each data row on its own line.
- The CSV is sent via the shared file-download helper with content type `text/csv` and filename `<group>-report.csv`, where `<group>` is the **raw `group` request parameter value** (one of the grouping keys `dept` / `topic` / `staff`), defaulting to the literal `Department` when the request carries no `group` parameter. Note the default is the display word `Department` while a present parameter is the lowercase key (e.g. `dept-report.csv`), so the filename stem is not normalized.
- The report's tabular JSON view and plot JSON view are produced by the same helper via the shared JSON encoder.
- **Cross-reference**: the meaning of the grouping options (Department / Topics / Staff), the date-range semantics, the per-department/topic/staff permission filtering, and the statistic columns (Opened/Assigned/Overdue/Closed/Reopened/Service Time/Response Time) belong to **FS-020**; this requirement pins only that the report's CSV/JSON output flows through the shared export/download/encoder mechanics owned here.

### FS-090.27: Full-Database Backup Exporter

**Description**: The system shall provide a full-database backup exporter that streams a versioned, record-separated dump of every known table (schema + indexes + rows).

**Acceptance Criteria**:
- The exporter writes to a supplied output stream.
- The dump opens with a header block containing a backup signature and backup version, plus a metadata record carrying: the application version, the configured table prefix, the secret salt, the database type, and the list of upgrade streams.
- Plugins may amend the exported table list via the `export.tables` signal before the dump runs.
- For each table in the fixed table list, the exporter writes: a `table` block (table name with the configured prefix stripped, its column schema, and its indexes), then one block per data row, then an `end-table` block.
- Each written block is JSON-encoded and terminated by a record-separator byte (`\x1e`).
- A table that reports no columns aborts the dump with an error ("Cannot export table with no fields").
- The fixed table list comprises the platform's full table set (config, syslog, files+chunks, staff, departments, topics, groups+dept-access, teams+members, FAQs+attachments+topics+categories, canned responses+attachments, tickets+thread+attachments+priority+lock+event+email-info, emails+templates+template-groups, filters+rules, SLA, API keys, timezones, sessions, pages). The canonical table names and schemas are owned by **FS-091**.

### FS-090.28: Client Nav Link Render Contract (Always-Present Key Class)

**Description**: The client portal nav bar shall render each assembled link as a list item whose anchor carries a two-token class attribute and the `ROOT_PATH`-prefixed target.

**Acceptance Criteria**:
- Each client nav link renders as a list item containing an anchor with class attribute `"<active> <key>"`, where `<active>` is the literal `active` when the link's active flag is set and the empty string otherwise, and `<key>` is the link's map key (`home`, `kb`, `new`, `tickets`, `status`).
- The `<key>` class is emitted for **every** link unconditionally, so each nav link is always individually targetable by its key class regardless of active state (this corrects the earlier implication in FS-090.4 that the key class only accompanies the active link).
- The anchor target is `ROOT_PATH` concatenated with the link's `href`; the link label (`desc`, which may contain literal `&nbsp;`) is the anchor text.
- Links are emitted in nav-map insertion order; each list item is newline-separated in the output.

### FS-090.29: Pagination — Secondary Start-Offset Modulo Correction

**Description**: After the primary out-of-range reset (FS-090.15), the pagination constructor shall apply a second start-offset correction that snaps the start to a page-size boundary under a specific arithmetic condition.

**Acceptance Criteria**:
- Independently of the primary reset, when `(limit − 1) × start` exceeds the total record count, the start offset is decremented by `start mod limit`, snapping it down to the nearest lower multiple of the page size.
- This second correction is evaluated after the primary "reset start to 0 if limit > total or page beyond last page" branch (FS-090.15); both branches can run in the same construction, and a start already reset to 0 is unaffected (0 mod limit is 0).
- The correction ensures the start offset always lands on an exact page boundary even when an unusual page/limit/total combination produced a fractional or over-large offset; the current-page value is subsequently re-derived from the corrected start (FS-090.15).

### FS-090.30: Result-Set Exporter — Empty Result Set Header Behavior

**Description**: The result-set exporter's column projection (FS-090.20) is conditioned on the presence of at least one result row; when the query returns no rows, the exporter falls back to the raw, unprojected header map.

**Acceptance Criteria**:
- On construction, the header list is first initialized to the full list of human column names from the supplied header map (every map value, in map order).
- The intersection-with-query projection (FS-090.20 / BS-090.10) — which rebuilds the header list, the key list, and the positional lookup list from the columns actually present in the first row — runs **only if** the query returns at least one row.
- If the query returns zero rows, no projection occurs: the header list remains the full raw header-map names, and the key/lookup lists are never populated.
- Consequently an empty-result CSV export emits a header row containing **all** header-map column names (not the query-intersected subset) followed by no data rows; an empty-result JSON export emits an empty array.

### FS-090.31: Export Format Selection Has No Fallback

**Description**: The export dispatcher selects a concrete exporter strictly by exact match of the format argument against the supported set, with no default for unrecognized values.

**Acceptance Criteria**:
- The supported format arguments are exactly `csv` and `json`; the dispatcher looks the argument up in a fixed map of those two keys.
- An unrecognized format argument resolves to no exporter class and produces a fatal construction failure rather than silently defaulting to CSV or JSON.
- All in-tree callers pass a literal `csv` (ticket export) or the report's own inline path; no caller passes an externally-supplied or unvalidated format value to the dispatcher.

### FS-090.32: Pagination — Page-Link Window Edge Compensation

**Description**: The page-link window (FS-090.17) is centered on the current page but redistributes its span near the first and last pages so the displayed count of page numbers stays stable.

**Acceptance Criteria**:
- The raw window is `floor(currentPage − span)` to `ceil(currentPage + span)` with span fixed at 5.
- When the raw window start would fall below page 1, the deficit is carried forward as extra page numbers appended after the window's end (and symmetrically, when the raw window end would exceed the last page, the surplus is carried back as extra page numbers prepended before the window's start), so the rendered window keeps a consistent width near either edge rather than shrinking.
- The window start is clamped to at least page 1 and the window end is clamped to at most the last page after compensation.
- The jump-back chevron `«` (rendered only when the window start is greater than page 1) targets `windowStart − span`, clamped to page 1; the jump-forward chevron `»` (rendered only when the window end is less than the last page) targets `windowEnd + span`, clamped to the last page.

---

## Business Rules

### BS-090.1: Navigation Visibility Is Permission- and Configuration-Conditional

**Rule**: A navigation item that depends on a capability is included only when the current identity holds that capability, and an item that depends on a feature toggle is included only when that feature is enabled. The menu is assembled fresh per request from the live identity and configuration; it is never a static list.

**Rationale**: The navigation is the primary affordance surface; showing a link a user cannot act on (or a feature that is disabled) would produce dead ends and confusing permission errors. Assembling from live state keeps the menu truthful.

**Examples**:
- A staff member who cannot create tickets sees no "New Ticket" sub-item.
- A staff member who cannot manage canned responses sees no "Canned Responses" sub-item.
- The client "Knowledgebase" link is absent when the knowledge base is disabled in configuration.
- The client "My Tickets (N)" link only appears for an authenticated client when "show related tickets" is enabled; otherwise the client sees "View Ticket Thread" pointing at their one ticket.

### BS-090.2: Single Active Tab; Active Tab's Sub-Menu Moves to the Sub-Nav Bar

**Rule**: At most one top tab is active at a time; setting a new active tab clears the prior one. The active tab does not render a hover drop-down — its sub-menu renders in the dedicated sub-nav bar; only inactive tabs carry hover drop-downs.

**Rationale**: The split keeps the active context's sub-options always visible (sub-nav bar) while still allowing quick lateral navigation into other tabs' options via hover, without duplicating the active tab's options in two places.

**Examples**:
- On the Tickets tab, ticket sub-options render in the sub-nav bar; hovering the inactive Knowledgebase tab reveals its sub-menu as a drop-down.

### BS-090.3: `droponly` Items Are Hover-Only

**Rule**: A sub-menu item flagged drop-down-only appears exclusively in the hover drop-down and is omitted from the sub-nav bar.

**Rationale**: Some ticket actions (the base "Tickets" link, "My Tickets", "New Ticket") are navigation shortcuts better suited to a hover menu than to the persistent sub-nav bar, which is reserved for the active tab's primary sub-pages.

**Examples**:
- "My Tickets (N)" and "New Ticket" show in the Tickets hover drop-down but never in the sub-nav bar.

### BS-090.4: Sub-Menu Highlight Falls Back to Current-Script Matching

**Rule**: When no sub-menu item is explicitly marked active, the renderer highlights the sub-item whose target (or extra matching-URL list) matches the currently executing script; an explicit active index that is out of range is treated as no explicit index.

**Rationale**: Most pages do not bother to set an explicit active sub-menu; deriving it from the current URL keeps the highlight correct with no per-page bookkeeping, while the explicit path remains available for pages that share a script across multiple sub-items.

**Examples**:
- Visiting `faq.php` highlights the "FAQs" sub-item because it declares `faq.php` in its extra matching-URL list even though its primary target is `kb.php`.

### BS-090.5: Global System Bars Are Mutually Exclusive and Priority-Ordered

**Rule**: At most one global system message bar renders, chosen by strict priority: error suppresses warning and notice; warning suppresses notice.

**Rationale**: Stacking multiple system bars would compete for attention; surfacing the single most severe condition keeps the chrome legible. Per-page form-feedback bars (BS-090.6) are a separate channel and may co-occur.

**Examples**:
- A request with both a warning and a notice set shows only the warning bar.

### BS-090.6: Per-Page Feedback Bars Are Independent of Global Bars

**Rule**: The per-page message bars (`msg_error` / `msg_notice` / `msg_warning`) are populated from page-local form-handler variables and are independent of the global system bars; a page may display both a global bar and a per-page bar at once. The per-page bars are themselves mutually exclusive and priority-ordered (error → notice → warning).

**Rationale**: Global bars communicate system/request state (e.g. offline mode, configuration warnings) while per-page bars communicate the outcome of the action the user just took on this page; conflating them would lose information.

**Examples**:
- After a failed form submission, the content region shows `msg_error` from the handler's errors while a separate global `warning_bar` may still warn about a system condition.

### BS-090.7: Page-Size Resolution Precedence

**Rule**: The effective page size is the most specific available source: per-request override (ticket queue only) > per-staff configured limit > system configured page size > the hard-coded default of 25.

**Rationale**: Staff can tune their own list density; the queue allows a one-off override; and a guaranteed default keeps lists bounded even before any configuration exists.

**Examples**:
- A staff member with a configured page limit of 50 sees 50-row lists; on the ticket queue they may pass `limit=10` to get 10 rows for that view.
- A client page always uses the system default page size.
- With no configured page size anywhere, lists default to 25 rows (the pagination component's own constructor default is 20, but callers always pass `PAGE_LIMIT`).

### BS-090.8: Out-of-Range Page Requests Snap to the First Window

**Rule**: A requested page beyond the last page, or a page size larger than the total, resets the start offset to 0 so the export/list shows the first data window instead of an empty page.

**Rationale**: Bookmarked or stale page links (e.g. after records are deleted) should degrade gracefully to valid data rather than rendering an empty table.

**Examples**:
- Requesting page 9 of a 3-page result set displays page-1 data; the page-link strip still renders the real page numbers.

### BS-090.9: Export Strips the Page LIMIT — Full Result Set

**Rule**: The result-set exporter removes any trailing `LIMIT` clause from the supplied query so the export contains the entire matching set, not just the on-screen page.

**Rationale**: A CSV export is expected to deliver all matching records for offline analysis; honoring the screen pagination limit would silently truncate the export to one page.

**Examples**:
- A 4,000-ticket search shown 25 per page exports all 4,000 rows when the staff member clicks Export.

### BS-090.10: Export Columns Are the Intersection of the Header Map and the Query

**Rule**: The exporter emits a column for a header-map field only if that field is present in the query's first result row; absent fields are dropped silently with no error and no empty column.

**Rationale**: Different callers run slightly different queries; making the export tolerant of missing columns lets one fixed header map serve multiple query shapes without breaking when a column is omitted.

**Examples**:
- If a queue query omits the `team` column, the export simply has no "Team Assigned" column rather than failing.

### BS-090.11: CSV Values Are Always Quoted; Embedded Quotes Are Doubled

**Rule**: Every CSV field is wrapped in double quotes and any embedded double-quote is escaped by doubling it; rows are newline-terminated.

**Rationale**: Always-quoting sidesteps delimiter/whitespace ambiguity in values (commas, line breaks within text fields), and quote-doubling is the standard CSV escape so spreadsheet tools parse values containing quotes correctly.

**Examples**:
- A subject of `He said "hi"` is emitted as `"He said ""hi"""`.

### BS-090.12: Ticket Export Is Bound to a Pre-Authorized Session Query

**Rule**: The ticket CSV export operates only on a query previously executed and stored in the session under its hash; the export request carries the hash token, not raw query parameters, and a token not present in the session is rejected.

**Rationale**: Re-deriving and re-running an arbitrary query from request parameters at export time would risk exporting data outside the staff member's already-applied visibility scope; binding the export to the exact SQL the queue already ran preserves the original authorization and avoids parameter tampering.

**Examples**:
- Exporting with a hash for a query the current session never ran yields "Query token not found".

### BS-090.13: Auto-Cron Beacon Only for Real Staff Sessions

**Rule**: The web-triggered auto-cron beacon image is embedded in the staff footer only when the current identity is a genuine staff member, never on client pages or anonymous staff-area access.

**Rationale**: The beacon is the fallback that drives background cron on page loads; tying it to authenticated staff page views provides regular cron triggers from the most frequent authenticated traffic without exposing the cron trigger to the public portal.

**Examples**:
- A logged-in staff member's every page view fires one `autocron.php` beacon request; a client portal page emits no beacon.

### BS-090.14: Backup Dump Is Record-Separated and Self-Describing

**Rule**: The full-database backup is a stream of JSON blocks each terminated by the record-separator byte, opening with a signature + version + environment metadata block, and per table emits a schema block, row blocks, and an end-table marker; a table with no columns aborts the entire dump.

**Rationale**: The record-separator framing lets the restorer/upgrader read the stream block-by-block without parsing the whole file at once; the leading metadata (version, prefix, salt, db type, streams) lets the restorer validate compatibility before applying; aborting on a schema-less table prevents producing a silently corrupt backup.

**Examples**:
- A backup carries the table prefix and version so a restore into a differently prefixed install can remap names and confirm the source version.

### BS-090.15: Column Projection Requires a Non-Empty Result Set

**Rule**: The exporter's "emit only header-map columns present in the query" projection (BS-090.10) is contingent on the query returning at least one row. With zero rows the exporter cannot inspect column presence, so it falls back to emitting the full, raw header-map column set as the header line and no data.

**Rationale**: Column presence is determined by examining the first result row; with no rows there is nothing to examine. Falling back to the full header map yields a well-formed (if maximally wide) header rather than an empty file, but it means an empty export and a populated export can carry different column sets.

**Examples**:
- A ticket search with no matches exports a CSV with all 18 ticket header columns and no rows; the same search with one match drops any columns the query did not select.

### BS-090.16: The Report Inline CSV Builder Does Not Escape Embedded Quotes

**Rule**: Unlike the generic CSV exporter (BS-090.11), the reporting helper's hand-built tabular CSV wraps every value in double quotes and comma-joins them but performs **no** doubling of embedded double-quotes; a value containing a literal double-quote produces malformed CSV for that field.

**Rationale**: This is an observed divergence between the two CSV producers (also noted in KL-090.6): the generic exporter escapes quotes by doubling, the report's inline builder does not. The behavior is documented so consumers understand report CSVs are not quote-safe for values containing double-quote characters.

**Examples**:
- A department named `Sales "EU"` in a report grouping would be emitted as `"Sales "EU""`, which a strict CSV reader mis-parses, whereas the generic ticket exporter would emit `"Sales ""EU"""`.

---

## Data Requirements

### Navigation Item Shape

Each navigation item is an associative record. Fields consumed by the render layer:

| Field | Applies to | Role |
|-------|-----------|------|
| `desc` | tab, sub-item, client nav | Display label (may contain literal `&nbsp;`) |
| `href` | tab, sub-item, client nav | Target page / URL (relative; client nav prefixes `ROOT_PATH`) |
| `title` | tab, sub-item | Tooltip / window title |
| `iconclass` | sub-item | CSS icon class on the sub-item anchor |
| `active` | tab, client nav | Active-state flag set by the active-state model |
| `droponly` | sub-item | When true, item is hover-drop-down-only (omitted from sub-nav bar) |
| `urls` | sub-item | Extra script base names that also activate this item (auto-highlight) |

Sub-menus are indexed by the panel-qualified key `<panel>.<tabkey>` (e.g. `staff.tickets`, `admin.settings`).

### Navigation Structures (Literal)

| Panel | Tabs (ordered) |
|-------|----------------|
| Staff | `dashboard`, `tickets`, `kbase` |
| Admin | `dashboard`, `settings`, `manage`, `emails`, `staff` |
| Client | `home`, [`kb`], `new`, then `tickets` (authed) **or** `status` (guest) |

(Full per-tab sub-menu contents and their permission/config conditions are enumerated in FS-090.2–FS-090.4.)

### Pagination Component State

| Name | Meaning |
|------|---------|
| `total` | Total record count |
| `limit` | Effective page size (≥ 1) |
| `page` | Requested page (≥ 1) |
| `start` | Result-window start offset = `(page-1)·limit`, reset to 0 if out of range |
| `pages` | Total page count = `ceil(total/limit)` |
| `url` | Base URL + carried query vars for page links |

Constants: `DEFAULT_PAGE_LIMIT` (system default, **25** fallback), `PAGE_LIMIT` (per-staff or default), `THISPAGE` (current-page URL used as the default page-link base). Request parameters: `p` (page number), `limit` (ticket-queue per-request page-size override).

Fixed display span: **5** pages each side of the current page (the window-half-width); displayed window up to 11 page numbers.

### Export Pipeline Constructs

| Construct | Inputs | Output |
|-----------|--------|--------|
| Generic result-set exporter | SQL (LIMIT stripped), header map (field→column), filter flag | Header-projected row stream |
| CSV exporter | as above | Quoted, comma-joined, newline-terminated rows; header row first |
| JSON exporter | as above | JSON array of associative records (shared encoder) |
| Ticket export column map | (FS-090.25, 18 fixed columns) | Ticket CSV columns |
| File-download helper | filename, MIME type, payload | Download response with attachment headers |
| Full-database backup exporter | output stream | Record-separated (`\x1e`) JSON block stream |

Ticket export filename: `tickets-<YYYYMMDD>.csv`. Report CSV filename: `<group>-report.csv` (group default "Department"). Backup block separator: byte `\x1e`. CSV escape: `"` → `""`. Download content type for CSV: `text/csv`.

> **Note**: Table names, column schemas, and enum value sets referenced by the exporters and report (ticket status/source, event states, the full backup table list) are canonically owned by **FS-091**; this spec references them by role, not by literal schema.

### Page Chrome Variables Consumed

| Variable | Source spec | Role in chrome |
|----------|-------------|----------------|
| `$ost` | FS-001 | Page title, extra headers, global error/warning/notice, link token |
| `$cfg` | FS-001/FS-032 | Helpdesk title, KB-enabled toggle, show-related-tickets toggle |
| `$thisstaff` | FS-002 | First name, admin flag, staff-vs-not test, page limit, refresh rate |
| `$thisclient` | FS-010 | Client name, validity, ticket count |
| `$nav` | this spec | Assembled navigation object |
| `$errors['err']`, `$msg`, `$warn` | per page | Per-page message bars |

---

## User Flows / Interactions

### Flow A: Navigating the staff control panel
1. The staff member loads a control-panel page; the bootstrap layer builds the staff navigation object from the staff identity and panel.
2. The page sets the active tab (and optionally active sub-menu) for the section it represents.
3. The header renders the top tabs (active vs inactive), nesting inactive tabs' sub-menus as hover drop-downs and rendering the active tab's sub-menu in the sub-nav bar (omitting `droponly` items).
4. The sub-nav bar highlights the sub-item matching the current script (or the explicitly set one).

### Flow B: Paging through a list
1. A list page counts total matching records and constructs the pagination component with `total`, the requested page `p`, and `PAGE_LIMIT`.
2. The page sets the pagination URL to carry over its sort/filter/status query vars.
3. The page queries one window using the component's start offset and limit, and renders "Showing X – Y of N" plus the page-link strip.
4. Clicking a page number, the `«`/`»` chevrons re-requests the same page with a new `p`, preserving the carried query vars.

### Flow C: Exporting the ticket queue to CSV
1. The staff member runs a queue view / search; the page stores the executed SQL in the session under its hash and renders an "Export" link carrying the hash.
2. The staff member clicks Export; the export action validates the hash token against the session.
3. The stored query (LIMIT stripped) is run; rows are projected through the fixed ticket column map and emitted as quoted CSV.
4. The buffered CSV is sent via the file-download helper as `tickets-<YYYYMMDD>.csv` with attachment headers; the browser downloads the file.

### Flow D: Client portal chrome
1. A visitor loads a public portal page; the client navigation is assembled from the (optional) client identity and configuration toggles.
2. The header shows either the authenticated client's name + tickets/logout links or "Guest User - Log In".
3. The nav bar renders the conditional link set (Home, optional KB, Open New Ticket, then Tickets/Status).
4. The footer renders copyright + "Powered by osTicket" (no auto-cron beacon on client pages).

---

## Edge Cases

### EC-090.1: Empty result set pagination
When the total is zero, "Showing 0" renders with no range and the page-link strip is effectively empty; the start offset is 0.

### EC-090.2: Page number beyond the last page
A `p` beyond the last page snaps the data window to the first page (start offset 0) per BS-090.8; the page-link strip still shows the true page count, so the displayed "[i]" current marker may briefly desync from the requested `p` until re-derived from the (reset) start offset.

### EC-090.3: Page size larger than total
When the page size exceeds the total record count, the start offset resets to 0 and a single page is shown.

### EC-090.4: Export token missing or stale
An export request with no `h` token yields "Query token required"; an `h` whose `search_<token>` session entry is absent (expired session, different session, tampered hash) yields "Query token not found". No data is exported in either case.

### EC-090.5: Export query column omitted
If the queue query does not select a column named in the ticket header map (e.g. `team`), that column is silently dropped from the CSV (no header, no values, no error) per BS-090.10.

### EC-090.6: CSV value containing quotes, commas, or newlines
Embedded double-quotes are doubled; commas and newlines inside a value are tolerated because every field is wrapped in quotes. Note that a newline embedded in a value will appear inside the quoted field and split visually but parses correctly as one field in a compliant CSV reader.

### EC-090.7: Out-of-range explicit active sub-menu
If a page sets an explicit active-sub-menu index that exceeds the number of sub-items, the renderer treats it as unset and falls back to current-script matching (FS-090.7), avoiding a highlight pointing at a non-existent item.

### EC-090.8: Client page rendered without a navigation object
A client page rendered with no `$nav` renders a horizontal rule in place of the nav bar and still renders the identity line (guest or authenticated) and content region.

### EC-090.9: Internet-Explorer-on-Windows download
For IE-on-Windows user agents the download helper omits the `attachment;` keyword from the content-disposition header (a legacy compatibility workaround) while still sending the filename; the file still downloads.

### EC-090.10: Backup table with no columns
If a table in the backup list reports zero columns (missing/corrupt table), the backup aborts mid-stream with "Cannot export table with no fields" rather than emitting a truncated backup.

### EC-090.11: Non-breaking spaces in labels
Navigation labels embed literal `&nbsp;` entities (e.g. "My&nbsp;Tickets", "Open&nbsp;New&nbsp;Ticket"); these are part of the stored label and render as non-breaking spaces, keeping multi-word labels from wrapping.

### EC-090.12: Empty-result export emits all header-map columns
When the exported query returns zero rows, the column-intersection projection never runs (FS-090.30); the CSV therefore emits a header row containing **all** header-map column names (e.g. all 18 ticket columns) rather than the query-intersected subset, followed by no data rows. The JSON variant emits an empty array. This differs from the non-empty case where absent columns are dropped (BS-090.10).

### EC-090.13: Unrecognized export format
If the export dispatcher receives a format argument other than `csv` or `json`, the lookup yields no exporter class and construction fails fatally (FS-090.31); there is no silent fallback to a default format. In practice no in-tree caller supplies an unvalidated format.

### EC-090.14: Backup table-with-no-fields hard-terminates the process
A backup table reporting zero columns does not raise a recoverable error — after writing the diagnostic line to the error stream the exporter calls a hard process termination mid-stream (FS-090.27 / EC-090.10), leaving any partially written prior-table blocks already flushed to the output stream.

### EC-090.15: Page-link window near the first/last page
Near page 1 the missing left-hand page numbers are redistributed to the right (and vice versa near the last page) so the page-number strip keeps a stable width at the edges rather than shrinking (FS-090.32); the jump chevrons appear only when the (compensated) window does not already touch page 1 / the last page.

---

## Dependencies

- **FS-001 (App bootstrap & shared request lifecycle)** — provides the global request context (`$ost`, `$cfg`), the page title, extra-header registration, the global error/warning/notice state, the anti-CSRF link token, and the constants `THISPAGE`, `ROOT_PATH`, `ASSETS_PATH`, `INCLUDE_DIR`, `DEFAULT_PAGE_LIMIT`.
- **FS-002 (Staff authentication, sessions & access control)** — provides the staff identity object and its permission predicates (`canCreateTickets`, `canManageFAQ`, `canManageCannedResponses`, `getNumAssignedTickets`, `isAdmin`, `isStaff`, `getPageLimit`, `getRefreshRate`) consumed by navigation visibility and chrome.
- **FS-010 (Public client portal)** — provides the client identity object (`$thisclient`, validity, ticket count) consumed by the client header/nav.
- **FS-020 (Staff ticket queue, dashboard & search)** — owns the report/dashboard data computation whose CSV/JSON output flows through this spec's export/download mechanics; owns the queue query that this spec's ticket export consumes.
- **FS-032 (Admin — system settings)** — owns the configuration values (helpdesk title, KB-enabled, show-related-tickets, page size) that navigation/chrome read; owns the settings sections the admin `settings` sub-menu routes to.
- **FS-043 (External API & cron scheduler)** — owns the `autocron.php` mechanism the footer beacon triggers.
- **FS-061 (Upgrader & database migration streams)** — consumes the full-database backup format (record-separated blocks, version/prefix/salt/streams metadata) produced by the backup exporter; the upgrade-stream list embedded in the backup header is owned there.
- **FS-091 (Reference data, enums & data model)** — canonical owner of all table names, column schemas, and enum value sets referenced by the exporters, the report helper, and the backup table list.
- **JSON encoder (FS-003 infrastructure)** — the shared JSON encoder reused by the JSON exporter, the report JSON views, and the backup block writer.
- **File-download helper (FS-003 infrastructure, `Http`)** — the download response helper documented in FS-090.24 lives in the shared HTTP infrastructure; documented here because the export pipeline is its principal consumer.

---

## Known Limitations

### KL-090.1: No CSV encoding normalization / BOM
The CSV exporter performs no character-set normalization and emits no byte-order mark; values are written as stored. Spreadsheet tools that assume a locale-specific encoding may misrender non-ASCII content. The only declared encoding is the `text/csv` content type on the download.

### KL-090.2: Always-quoted CSV without RFC-4822-style line-ending guarantee
Rows are terminated with a bare `\n` (not `\r\n`); some strict CSV consumers expect CRLF line endings. Values are universally quoted but the line terminator is not configurable.

### KL-090.3: Pagination current-page marker derived from start offset, not requested page
After an out-of-range page request snaps to the first window (BS-090.8), the rendered "[i]" current-page marker is recomputed from the (reset) start offset, so the highlighted page can differ from the `p` the user requested. There is no redirect to a canonical in-range URL.

### KL-090.4: Two parallel highlight mechanisms for sub-menus
Sub-menu highlighting can be driven either by an explicit active index or by current-script URL matching, and the explicit path silently degrades to the implicit one when out of range (FS-090.7). The dual mechanism is a source of subtle mismatches when a script backs multiple sub-items.

### KL-090.5: IE-on-Windows download branch is legacy
The content-disposition workaround for Internet-Explorer-on-Windows is dead/legacy code for that long-obsolete browser family but remains in the download path; it has no effect on modern user agents.

### KL-090.6: Report CSV export does not reuse the generic CSV exporter
The report tabular CSV (FS-090.26) hand-builds its CSV string inline rather than routing through the generic CSV exporter class used by the ticket export, duplicating the quote/join logic. The two CSV producers can drift (e.g. embedded-quote escaping is explicit in the generic exporter but not in the report's inline builder).

### KL-090.7: Backup exporter emits secret salt in the header metadata
The full-database backup header includes the installation's secret salt and table prefix in plaintext within the metadata block, so a backup file is security-sensitive and must be handled as a secret; the exporter applies no encryption to the stream itself.

### KL-090.8: Navigation tab/sub-menu sets are hard-coded
The staff/admin/client menu structures are defined in code, not data; adding a section requires a code change. There is no per-install navigation customization beyond the permission/config gating already described.

---

## Future Considerations

- A canonical-URL redirect for out-of-range page requests would eliminate the current-page-marker desync (KL-090.3).
- Routing the report CSV through the generic CSV exporter would remove the duplicate quote/join logic (KL-090.6).
- Configurable CSV line endings / explicit UTF-8 BOM emission would improve spreadsheet interoperability (KL-090.1, KL-090.2).
- Encrypting or redacting secret material in the backup header would reduce the sensitivity of backup files (KL-090.7).
