# FS-033: Admin — System Logs, Site Pages & Content

## Overview

This specification defines the administrator-facing **System Log viewer**, the
administrator-facing **Site Pages** content-management feature, and the two read-only
**AJAX content/configuration endpoints** that serve supporting data to the staff control
panel. It covers four closely-related concerns reachable only from an authenticated
administrator's session:

1. **System Log management** (`scp/logs.php` + `include/staff/syslogs.inc.php`): a
   filterable, sortable, paginated table of the application's internal event log
   (`syslog` table — the `ost_syslog` store described canonically in FS-091), with
   single-entry detail viewing and bulk manual deletion.
2. **Log lifecycle / auto-purge**: the read-and-management half of the system log — the
   single-record load model (`Log` class) and the time-based **auto-purge** sweep
   (`purgeLogs()`) that the cron scheduler (FS-043) invokes to age out old entries
   according to a configurable grace period.
3. **Site Pages management** (`scp/pages.php` + `class.page.php` + `pages.inc.php` /
   `page.inc.php`): a CRUD + enable/disable + bulk-action admin screen over DB-backed
   editable content "pages" (`page` table — `ost_page`), each typed `landing` / `offline` /
   `thank-you` / `other`. These pages drive the public landing page, the offline-mode
   banner, the post-submission thank-you page, and slug-addressed custom "other" pages.
4. **Supporting AJAX endpoints**: `ContentAjaxAPI` (`/content/log/<id>`,
   `/content/ticket_variables`) which renders an HTML log-detail popover and a static
   ticket-variable reference card, and `ConfigAjaxAPI` (`/config/scp`) which returns a
   small JSON bundle of client-side upload/locking configuration.

> **Scope boundary — write-side logging belongs to FS-003.** The act of *writing* a log
> entry (the `osTicket::log()` / `logDebug()` / `logWarning()` / `logError()` /
> `logDBError()` family and the log-level gating that decides whether a message is
> persisted at all) is owned by **FS-003** (Cryptography, validation & formatting
> infrastructure). This spec documents only the **viewing, detail-rendering, manual
> deletion, and auto-purge** of already-persisted log records. The log-level setting
> (`log_level`) and grace-period setting (`log_graceperiod`) are configured under
> **FS-032** (system settings); they are referenced here as inputs, not owned.

> **Scope boundary — "Site Pages" IS a real feature in this 1.7 snapshot.** The §3
> work-list named this spec "logs, **pages** & content", and a full **Site Pages**
> content-management feature is present: `scp/pages.php` (entry), `include/class.page.php`
> (the `Page` entity over the `ost_page` table), and `include/staff/pages.inc.php` (list)
> + `include/staff/page.inc.php` (add/edit form). These pages back the public landing
> page, the offline banner, the post-submission thank-you page, and slug-addressed custom
> "other" pages. The `PAGE_LIMIT` constant referenced by the **log** viewer is a separate
> concern — it is a pagination page-size, unrelated to the Site Pages content entity (which
> the page list also paginates with the same `PAGE_LIMIT`). The Site Pages requirements are
> FS-033.11 – FS-033.16 below.

> **Scope boundary — page consumption sites belong to their own specs.** This spec owns the
> **admin CRUD** over page records. The *rendering* of a page at its consumption site —
> the landing page (`index.php`, FS-010), the offline banner (`offline.php`, FS-001), the
> thank-you page after web submission (`open.php`, FS-011), and the public slug-addressed
> custom-page servlet (`pages/index.php`, FS-010) — is referenced here but owned by those
> specs. The configuration that *binds* a page to the landing/offline/thank-you slot
> (`landing_page_id`, `offline_page_id`, `thank-you_page_id`) is owned by FS-032 system
> settings; this spec consumes those bindings only to compute "default/in-use" page
> protection (BS-033.8).

---

## Functional Requirements

### FS-033.1: System Log Viewer Entry Point & Access Gate

**Description**: The system shall expose a System Logs administration screen, reachable
only by an authenticated administrator, that lists internal event-log records.

**Acceptance Criteria**:
- The entry script (`scp/logs.php`) requires the admin bootstrap (`admin.inc.php`) before
  any processing; this enforces that the requester is a logged-in staff member whose
  account carries administrator privileges (the standard admin-control-panel gate — see
  FS-002 / FS-001 for the canonical auth chain).
- The viewer body (`syslogs.inc.php`) additionally re-asserts the gate: if the admin
  constant is not defined, or there is no current staff session, or the current staff
  member is not an administrator, the page emits `Access Denied` and stops. (The viewer
  partial guards itself independently of the entry script.)
- On load the screen renders a page heading **"System Logs"**, a filter form, and the
  results table.
- The active top-level navigation tab while on this screen is **`dashboard`** (the logs
  screen is grouped under the dashboard tab, not a dedicated nav tab).

### FS-033.2: Log Filtering — Type and Date Span

**Description**: The system shall allow the administrator to narrow the displayed log
records by log **type** and by a **created-date span**.

**Acceptance Criteria**:
- A filter form (method GET, target `logs.php`) presents:
  - A **Date Span**: a "From" start-date field (`startDate`) and a "to" end-date field
    (`endDate`), both free-text date inputs with autocomplete disabled.
  - A **Type** dropdown with four options: `All` (empty value, default selected),
    `Errors` (`Error`), `Warnings` (`Warning`), `Debug` (`Debug`).
  - A submit control labelled **"Go!"**.
- **Type filtering**: when a recognized type is supplied (`error`, `warning`, or `debug`,
  case-insensitive), only records whose `log_type` equals that type are shown, and the
  results caption reflects the chosen scope (`Errors` / `Warnings` / `Debug logs`). Any
  unrecognized or empty type means **no type restriction** and the caption is `All logs`.
- **Date filtering**: a supplied `startDate` of at least 8 characters restricts results to
  records `created` on or after the parsed start instant; a supplied `endDate` of at least
  8 characters restricts to records `created` on or before the parsed end instant. Both
  bounds are parsed from free-text date strings.
- The chosen filter values are echoed back into the form fields (HTML-escaped) so the
  filter state is visible after submission.
- The selected type is carried forward in pagination and sort links (preserved across
  page changes — see FS-033.4 / FS-033.5).

### FS-033.3: Log Results Table

**Description**: The system shall display matching log records in a tabular list, one row
per record, with a per-row selection checkbox and a clickable title that opens the
detail popover.

**Acceptance Criteria**:
- The table renders columns, left to right: a **selection checkbox**, **Log Title**,
  **Log Type**, **Log Date**, **IP Address**.
- Each row is keyed by the record's `log_id` (used as the row element id and the checkbox
  value).
- The **Log Title** cell is a link (`log/<log_id>`, carrying a `tip` class) that, on
  interaction, fetches and displays the record's full detail (see FS-033.8); the title
  text is HTML-escaped.
- The **Log Type** cell shows the record's type verbatim (`Error` / `Warning` / `Debug`).
- The **Log Date** cell shows the record's `created` timestamp formatted as a
  day-date-time string.
- The **IP Address** cell shows the originating IP recorded with the entry.
- A results caption reflects the current page range and scope, e.g. the "showing X–Y of N"
  range followed by the scope title (`All logs` / `Errors` / `Warnings` / `Debug logs`);
  when the result set is empty the caption reads **"No logs found!"**.
- When at least one record is shown, a selection toolbar offers **All**, **None**, and
  **Toggle** shortcuts that set the row checkboxes accordingly.
- If a prior bulk action was submitted with validation errors, previously-checked rows are
  re-checked on redisplay so the selection is not lost.

### FS-033.4: Log Sorting

**Description**: The system shall allow the administrator to sort the log list by clicking
column headers, toggling ascending/descending order.

**Acceptance Criteria**:
- Sortable columns and their sort keys: **Log Title** (`title`), **Log Type** (`type`),
  **Log Date** (`date`), **IP Address** (`ip`). An `id` sort key also exists and is the
  default sort field.
- The recognized sort keys map to underlying record fields: `id`→log id, `title`→title,
  `type`→type, `ip`→IP address, `date`/`created`→created timestamp; an unrecognized sort
  request falls back to sorting by **created date**.
- Sort direction is `ASC` or `DESC` (case-insensitive in the request); an unrecognized or
  absent direction defaults to **`DESC`** (newest/highest first).
- The default sort, with no sort parameters, is by **id, descending**.
- Each column header link carries the current direction as a CSS hint and toggles the
  order on the next click (clicking a sorted column flips its direction).
- Sort, type, and date-span parameters are preserved together in the header links so
  sorting does not discard the active filter.

### FS-033.5: Log Pagination

**Description**: The system shall paginate the log list using the standard page-size limit
and render page-navigation links.

**Acceptance Criteria**:
- The total matching count is computed (honoring the active type/date filter) and the list
  is paginated at the standard **`PAGE_LIMIT`** records per page.

  > **Note**: `PAGE_LIMIT` resolves to the current staff member's configured page-size, or
  > else the system default page size (default **25**) — see FS-090 (shared
  > pagination) / FS-032 (page-size setting). It is a row-count limit, **not** a
  > content-page entity.
- The requested page number is taken from the `p` query parameter (numeric; defaults to
  page 1).
- Page-navigation links are rendered below the table and preserve the active type/date
  filter in their URLs (`logs.php` is set as the pagination base URL with the current
  filter query string).
- When there are no results, no page links and no bulk-action controls are shown.

### FS-033.6: Bulk Manual Deletion of Log Entries

**Description**: The system shall allow the administrator to permanently delete one or
more selected log records in a single action.

**Acceptance Criteria**:
- The results table is wrapped in a POST form (target `logs.php`) carrying a CSRF token, a
  hidden command field `do=mass_process`, and a hidden action field `a` set by the
  delete control.
- A **"Delete Selected Entries"** submit control triggers the deletion of all checked rows.
- On POST with `do=mass_process` and action `delete`:
  - If no rows are selected (the `ids` field is empty / not an array / zero-length), the
    action is rejected with the error **"You must select at least one log to delete"** and
    no records are removed.
  - Otherwise the selected records (matched by `log_id`) are deleted permanently.
  - On success where the number deleted equals the number selected, a success message
    **"Selected logs deleted successfully"** is shown.
  - Where fewer than selected were deleted (a partial outcome), a warning **"N of M
    selected logs deleted"** is shown.
  - Where the deletion affects zero rows (and no prior error), the error **"Unable to
    delete selected logs"** is shown.
- A POST with `do=mass_process` but an unrecognized action yields the error **"Unknown
  action - get technical help"**; a POST with an unrecognized `do` command yields **"Unknown
  command/action"**.
- Deletion is **irreversible** — the confirmation dialog warns "Deleted logs CANNOT be
  recovered." A confirmation dialog (`Please Confirm` → "Are you sure you want to DELETE
  selected logs?") gates the destructive submit (No, Cancel / Yes, Do it!).

### FS-033.7: Automatic Log Purge (Grace-Period Sweep)

**Description**: The system shall automatically age out old system-log records based on a
configurable retention grace period, executed as part of the scheduled cron run.

**Acceptance Criteria**:
- A purge operation (`purgeLogs()`) deletes every system-log record whose `created`
  timestamp is older than the configured **log grace period** measured in **months**
  (records satisfying `created + graceperiod months ≤ now`).
- If no grace period is configured (empty/non-numeric `log_graceperiod` setting), the
  purge is a **no-op** and returns without deleting anything (logs are retained
  indefinitely).
- The purge sweep is invoked by the cron scheduler's `PurgeLogs` task during the standard
  cron run (alongside mail fetch, overdue-ticket sweep, lock cleanup, and orphaned-file
  cleanup — see FS-043 for the full cron task set and its trigger cadence). There is no
  separate manual "purge now" control in the admin UI; ongoing aging-out is entirely
  cron-driven, and the only manual removal path is the bulk delete of FS-033.6.
- The grace-period value (`log_graceperiod`) and the log level that governs whether new
  entries are written (`log_level`) are configured under FS-032 system settings; this
  requirement consumes them but does not own their configuration UI.

  > **Note**: The purge implementation also carries a `// TODO: Activity logs` marker —
  > activity-log purging is **not implemented** in this version (see KL-033.4). Only the
  > system log is purged.

### FS-033.8: Single Log Record Detail (Content AJAX — `/content/log/<id>`)

**Description**: The system shall serve, on demand, an HTML detail popover for a single log
record identified by its id.

**Acceptance Criteria**:
- A content AJAX endpoint accepts a numeric log id and loads the matching record.
- A log record is looked up only when the id is present, numeric, and resolves to a stored
  record whose loaded id matches the requested id; otherwise the lookup yields nothing.
- On a successful lookup the endpoint returns an HTML fragment (fixed width ~500px)
  containing: the record's **title** (bold), the record's **log body text** rendered for
  display (with comma separators expanded to `", "` for readability), and a footer line
  showing **"Log Date:"** (the formatted created day-date-time) and **"IP Address:"** (the
  recorded IP).
- When the id is missing or does not resolve to a record, the endpoint returns an error
  fragment (~295px wide) reading **"Error: Unknown or invalid log ID"**.
- This endpoint backs the log-title links in the results table (FS-033.3): clicking a
  log title fetches and displays this popover.
- The endpoint is mounted under the staff AJAX dispatcher at `/content/log/<id>` (GET).

### FS-033.9: Ticket-Variable Reference Card (Content AJAX — `/content/ticket_variables`)

**Description**: The system shall serve a static reference card enumerating the ticket
template variables available for substitution in email templates and auto-responses.

**Acceptance Criteria**:
- A content AJAX endpoint (`/content/ticket_variables`, GET) returns a fixed HTML fragment
  (~680px wide) titled **"Ticket Variables"** with a note that non-base variables depend
  on usage context and to consult external documentation.
- The card is organized into two groups: **Base Variables** and **Other Variables**.
- The **Base Variables** group lists the ticket-scoped substitution tokens with
  human-readable descriptions, including (among others): `%{ticket.id}`,
  `%{ticket.number}`, `%{ticket.email}`, `%{ticket.name}`, `%{ticket.subject}`,
  `%{ticket.phone}`, `%{ticket.status}`, `%{ticket.priority}`, `%{ticket.assigned}`,
  `%{ticket.create_date}`, `%{ticket.due_date}`, `%{ticket.close_date}`,
  `%{ticket.auth_token}`, `%{ticket.client_link}`, `%{ticket.staff_link}`, plus a set of
  **expandable** variables (`%{ticket.topic}`, `%{ticket.dept}`, `%{ticket.staff}`,
  `%{ticket.team}`).
- The **Other Variables** group lists context tokens such as `%{message}`, `%{response}`,
  `%{comments}`, `%{note}`, `%{assignee}`, `%{assigner}`, `%{url}`, and `%{reset_link}`,
  each with a description.
- This card is **static content** — it does not query the database and is identical on
  every request; it is a help/reference popover consumed by the email-template and
  auto-response editors (see FS-040).

  > **Note**: The canonical authority for what each variable resolves to at send time is
  > the variable-substitution engine (`VariableReplacer`) documented under FS-040
  > (templates & outbound mail). This requirement owns only the **reference card content**,
  > not the substitution behavior.

### FS-033.10: Staff Configuration Bundle (Config AJAX — `/config/scp`)

**Description**: The system shall serve a small JSON configuration bundle to the staff
control-panel front-end so client-side scripts can honor server-side limits and formats.

**Acceptance Criteria**:
- A config AJAX endpoint (`/config/scp`, GET) returns a JSON object containing:
  - **`lock_time`** — the ticket auto-lock duration, expressed in **seconds** (the
    configured lock time in hours multiplied by 3600).
  - **`date_format`** — the system date-format string used for client-side date rendering.
  - **`max_file_uploads`** — the maximum number of files a **staff** member may attach in
    one operation (integer).
- The values are sourced from the system configuration (FS-032); this endpoint exposes a
  read-only projection of them for the staff UI.

> **Companion client endpoint (not in this admin spec's primary scope):** The same AJAX
> handler also defines a **`client`** projection (`file_types`, `max_file_size`,
> `max_file_uploads` for clients), serving the public client portal. It is mentioned here
> for completeness because it lives in the same `ConfigAjaxAPI` handler, but the client
> upload-config consumer is owned by FS-010/FS-011 (public portal). See KL-033.5.

### FS-033.11: Site Pages List & Access Gate

**Description**: The system shall expose a Site Pages administration screen, reachable only
by an authenticated administrator, that lists all editable content pages.

**Acceptance Criteria**:
- The entry script (`scp/pages.php`) requires the admin bootstrap (`admin.inc.php`) before
  any processing; the list/edit partials additionally re-assert the gate (admin constant
  defined AND current staff member is an administrator) and emit `Access Denied` and stop
  otherwise. (The add/edit partial also tolerates a missing staff session in its guard.)
- The active top-level navigation tab while on the pages screen is **`manage`** (the pages
  screen is grouped under the manage tab — distinct from the logs screen, which sits under
  `dashboard`).
- On load the list screen renders a heading **"Site Pages"**, an **"Add New Page"** link
  (`pages.php?a=add`), and a results table.
- The list query joins each page to its help-topic usage count (`topic.page_id`) so the
  "in-use" indicator can be computed per row.

### FS-033.12: Site Pages Results Table, Sorting & Pagination

**Description**: The system shall display all content pages in a sortable, paginated table.

**Acceptance Criteria**:
- The table renders columns, left to right: a **selection checkbox**, **Name** (a link to
  the edit form `pages.php?id=<id>`), **Status**, **Date Added**, **Last Updated**.
- The **Status** cell shows `Active` or, for an inactive page, **`Disabled`** (emphasized),
  and appends an **`(in-use)`** marker when the page is in use (has linked help topics, or
  is one of the configured default pages — see BS-033.8).
- Sortable columns and their sort keys: **Name** (`name`), **Status** (`status` →
  `isactive`), **Date Added** (`created`), **Last Updated** (`updated`). An unrecognized
  sort key falls back to **`name`**, and the default sort direction is **`ASC`** (so the
  default ordering is by name, ascending — distinct from the log viewer's id-descending
  default, BS-033.3).
- Sort direction is `ASC` or `DESC` (case-insensitive); an unrecognized/absent direction
  defaults to **`ASC`**. Each column header toggles direction on the next click.
- The list paginates at the standard **`PAGE_LIMIT`** rows per page; the page number comes
  from the `p` query parameter (numeric, defaults to 1); page links preserve the active
  sort/order in their URLs.
- When no pages exist, the caption reads **"No pages found!"** and no selection toolbar or
  bulk-action controls are shown; otherwise an **All / None / Toggle** selection toolbar is
  offered.

### FS-033.13: Create & Edit a Site Page

**Description**: The system shall allow an administrator to create a new content page and to
edit an existing one through a single add/edit form.

**Acceptance Criteria**:
- The add/edit form (`page.inc.php`) is shown when `a=add` is requested (heading
  **"Add New Page"**, submit **"Add Page"**, `do=add`) or when a valid `id` resolves to an
  existing page (heading **"Update Page"**, submit **"Save Changes"**, `do=update`).
- The form collects: **Name** (required, text), **Type** (required; a select with a
  placeholder "Select Page Type" plus the four type options — see BS-033.7), **Status**
  (radio: Active=1 / Disabled=0; defaults to Disabled on a new page), **Page body**
  (required, rich-text), and **Admin Notes** (optional internal notes).
- On a **create** (`do=add`): a new page record is inserted with `created`/`updated` set to
  now; on success the message **"Page added successfully"** is shown and the view returns to
  the list (the `a=add` flag is cleared); on failure (with no field-specific error) the
  error **"Unable to add page. Try again!"** is shown.
- On an **update** (`do=update`): if no valid page is loaded the error
  **"Invalid or unknown page"** is shown; otherwise the record is updated (`updated` set to
  now); on success the message **"Page updated successfully"** is shown and the view returns
  to the list; on failure (with no field-specific error) the error
  **"Unable to update page. Try again!"** is shown.
- A request carrying an `id` that does not resolve to an existing page sets the error
  **"Unknown or invalid page"** before any POST handling.
- The page body is stored after HTML-safening (sanitized rich text); the page name is
  stripped of tags and trimmed before storage.
- For a page of type **`other`** with a name, the edit form additionally shows the page's
  **Public URL** — `pages/<slug>` derived from the page name (slugified) and the system base
  URL — through which the public custom-page servlet serves the page (see BS-033.9).
- The form notes that **ticket variables are only supported in thank-you pages** (so the
  `%{ticket.*}` reference card of FS-033.9 applies to `thank-you` page bodies, not to
  landing/offline/other bodies).

### FS-033.14: Site Page Field Validation & Uniqueness

**Description**: The system shall validate page input on create/update and enforce a unique
page name.

**Acceptance Criteria**:
- **Type** is required and must be one of `landing` / `offline` / `thank-you` / `other`;
  a missing type yields "Type required" and an out-of-range value yields "Invalid
  selection".
- **Name** is required ("Name required"); the (tag-stripped, trimmed) name must be unique
  across pages — a name already used by a different page yields **"Name already exists"**.
- **Page body** is required ("Page body is required").
- On update, an internal consistency guard fires "Internal error. Try again" if the
  submitted hidden `id` does not match the page being saved.
- Any present validation error aborts the save (no partial write) and the form is
  re-rendered with the submitted values and per-field error messages.

### FS-033.15: Enable / Disable / Delete Pages (Bulk Actions)

**Description**: The system shall allow an administrator to bulk enable, disable, or delete
selected pages, subject to in-use protection.

**Acceptance Criteria**:
- The list is wrapped in a POST form (target `pages.php`) carrying a CSRF token, a hidden
  `do=mass_process` command, a hidden action field `a`, and three submit controls:
  **Enable**, **Disable**, **Delete**.
- If no rows are selected, the action is rejected with **"You must select at least one
  page."** and nothing changes.
- **In-use protection (pre-check):** if any selected id is a configured default page
  (landing/offline/thank-you binding) AND the action is **not** `enable`, the whole batch is
  rejected with **"One or more of the selected pages is in-use and CANNOT be
  disabled/deleted."** — i.e. default pages may be (re-)enabled but never disabled or
  deleted in bulk.
- **Enable** sets `isactive=1` on all selected pages; on full success
  **"Selected pages enabled"**, on partial **"N of M selected pages enabled"**, on no-effect
  **"Unable to enable selected pages"**.
- **Disable** iterates and disables each selected page individually; a page that **is in
  use** (linked topics or a default page) refuses to disable and is skipped; on full success
  **"Selected pages disabled"**, on partial **"N of M selected pages disabled"**, on
  no-effect **"Unable to disable selected pages"**.
- **Delete** iterates and deletes each selected page individually; an **in-use** page refuses
  to delete and is skipped; on full success **"Selected pages deleted successfully"**, on
  partial **"i of M selected pages deleted"**, on no-effect **"Unable to delete selected
  pages"**. Deletion is permanent — the confirmation dialog warns "Deleted pages CANNOT be
  recovered." A `Please Confirm` dialog gates Enable / Disable / Delete (each with its own
  confirmation copy).
- On deleting a page, any help topics that referenced it have their `page_id` reset to `0`
  (the reference is cleared, not left dangling).
- A POST under `do=mass_process` with an unrecognized action yields **"Unknown action - get
  technical help."**; a POST with an unrecognized `do` command yields **"Unknown
  action/command"**.

### FS-033.16: Single-Page Enable/Disable/Delete Guards (Entity Rules)

**Description**: The page entity shall enforce status and lifecycle guards independently of
the bulk UI.

**Acceptance Criteria**:
- A page **update** that sets the page inactive while it **is in use** is rejected with the
  error **"A page currently in-use CANNOT be disabled!"** (field error "Page is in-use!"),
  and the save is aborted.
- **Disable** on an already-inactive page is a successful no-op (returns success without a
  write); disable on an in-use page fails.
- **Delete** on an in-use page fails (no rows removed).
- A page is considered **in use** when it has at least one linked help topic OR its id is one
  of the configured default pages (BS-033.8).

---

## Business Rules

### BS-033.1: Log Viewer Is Administrator-Only
**Rule**: The System Logs screen and the `/content/log/<id>` and `/config/scp` AJAX
endpoints are restricted to authenticated staff members holding administrator privileges;
non-admins receive an access-denied response.
**Rationale**: The system log can contain sensitive diagnostic detail (error text, IP
addresses, internal operations), and bulk deletion is destructive — both are
administrative responsibilities.
**Examples**:
- A logged-in administrator opens `logs.php` and sees the full filterable list.
- A non-admin staff member who reaches the viewer partial receives `Access Denied`.

### BS-033.2: Three Log Severity Levels (Error / Warning / Debug)
**Rule**: Every persisted system-log record carries exactly one of three severity types —
**Error**, **Warning**, or **Debug** — and the viewer's type filter offers exactly these
three plus an "All" (no-filter) option.
**Rationale**: The write-side collapses the broader syslog priority spectrum into three
"Windows-style" levels (Error=1, Warning=2, Debug=3); the viewer mirrors that three-level
model.
**Examples**:
- A database error is stored as type `Error`; a non-critical condition as `Warning`; a
  diagnostic trace as `Debug`.
- Filtering by `Warnings` shows only `Warning` records.

> **Note**: The mapping of underlying severities to these three levels, and the log-level
> threshold (`log_level`) that decides whether a given message is persisted at all, are
> owned by FS-003 (write-side) / FS-032 (the level setting). See the scope boundary in the
> Overview.

### BS-033.3: Default Ordering Is Newest-First by Id
**Rule**: Absent any sort selection, the log list is ordered by record id descending,
which surfaces the most-recently inserted records first.
**Rationale**: Operators investigating an incident want the latest events at the top.
**Examples**:
- Opening the viewer with no parameters shows the highest `log_id` rows first.
- Clicking the "Log Date" header sorts by created timestamp, defaulting to descending,
  then toggling to ascending on the next click.

### BS-033.4: Invalid Date Span Is Rejected, Not Applied
**Rule**: A date-span filter is ignored (and an error surfaced) when the start instant is
in the future, or when the start is later than a non-zero end instant; the unfiltered (or
remaining-valid) result set is shown instead.
**Rationale**: A nonsensical span would silently return nothing; rejecting it with a clear
message ("Entered date span is invalid. Selection ignored.") avoids confusing "no results"
states.
**Examples**:
- A start date in the future → the span is dropped and the message is shown.
- Start after end (both set) → the span is dropped and the message is shown.

### BS-033.5: Manual Deletion Is Permanent and Confirmed
**Rule**: Deleting selected log entries removes them irrecoverably; the action requires an
explicit selection and an explicit confirmation before it executes.
**Rationale**: There is no soft-delete, archive, or undo for log records; a guard against
accidental loss is the confirmation dialog and the empty-selection rejection.
**Examples**:
- Submitting the delete form with no rows checked is rejected with "You must select at
  least one log to delete".
- Confirming the dialog deletes the checked rows and reports a success / partial / failure
  message accordingly.

### BS-033.6: Auto-Purge Is Grace-Period-Gated and Cron-Driven
**Rule**: System-log records older than the configured grace period (in whole months) are
deleted automatically by the cron purge sweep; if no grace period is set, no automatic
deletion occurs.
**Rationale**: Operators choose their retention horizon; a missing/zero/non-numeric grace
period means "retain forever" and the sweep does nothing rather than wiping the log.
**Examples**:
- Grace period = 6 → records older than 6 months are purged on each cron run.
- Grace period unset → the sweep returns immediately and retains all records; only manual
  deletion can remove them.

### BS-033.7: Content & Config AJAX Are Read-Only Projections
**Rule**: The `/content/...` and `/config/...` AJAX endpoints only read and render existing
state (a single log record, a static reference card, or current configuration values); they
never mutate persisted data.
**Rationale**: They are presentation/help helpers for the staff UI, distinct from the
destructive log-deletion path (which is a POST to `logs.php`, not an AJAX content call).
**Examples**:
- `/content/log/42` renders a popover for log 42 without changing it.
- `/config/scp` returns current upload/lock/date settings without altering them.

### BS-033.8: A Site Page Has Exactly One of Four Types
**Rule**: Every content page carries exactly one **type** — `landing`, `offline`,
`thank-you`, or `other`. The type is required and validated against this closed set on every
save.
**Rationale**: Type determines where the page is consumed: `landing` → the public landing
page; `offline` → the offline-mode banner; `thank-you` → the post-submission page (and the
only type that supports ticket-variable substitution in its body); `other` → a public,
slug-addressed standalone page.
**Examples**:
- A page typed `thank-you` may use `%{ticket.*}` variables in its body; a `landing` page may
  not.
- Saving a page with no type → "Type required"; with `banner` → "Invalid selection".

### BS-033.9: Default (Bound) Pages Are In-Use and Protected
**Rule**: A page bound to the landing / offline / thank-you slot (its id equals
`landing_page_id`, `offline_page_id`, or `thank-you_page_id` in system config), or a page
referenced by at least one help topic, is considered **in use** and cannot be disabled or
deleted; it may only be (re-)enabled.
**Rationale**: Disabling or deleting the page that backs a live consumption site (landing,
offline banner, thank-you, or a help topic) would break that site; the in-use guard prevents
operators from removing a page the system still depends on.
**Examples**:
- The page wired as the landing page appears with an `(in-use)` marker and refuses bulk
  disable/delete.
- A page with no help-topic links and not bound to any slot can be freely disabled or
  deleted.

### BS-033.10: Custom "Other" Pages Are Served Publicly by Slug
**Rule**: A page of type `other` that is **active** is reachable on the public portal at
`pages/<slug>`, where `<slug>` is the slugified page name; the public servlet matches the
request path to a page by slug and renders its body inside the client header/footer. Inactive
or non-`other` pages return 404 at that route.
**Rationale**: The `other` type is the only one exposed as a free-standing public URL;
landing/offline/thank-you pages are rendered only at their dedicated consumption sites.
**Examples**:
- An active `other` page named "Terms of Service" is served at `pages/terms-of-service`.
- The same page while Disabled returns "Page Not Found" (404).

---

## Data Requirements

The System Log viewer reads, and the purge/delete paths write to, the **system log store**
(`syslog` table, prefixed `ost_syslog`). The canonical column-level schema and enum sets
live in **FS-091** (Reference data, enums & data model) and are **referenced, not restated**
here. Functionally, each log record exposes the following attributes consumed by this
domain:

| Attribute | Functional meaning | Consumed by |
|-----------|--------------------|-------------|
| log id | Stable record identifier; row key, sort key, deletion target, detail lookup key | FS-033.3, .4, .6, .8 |
| title | Short human-readable label of the event | FS-033.3, .8 |
| log type | Severity level — one of `Error` / `Warning` / `Debug` (BS-033.2) | FS-033.2, .3, .4 |
| log (body) | Full event text / diagnostic detail | FS-033.8 |
| IP address | Originating IP recorded when the entry was written | FS-033.3, .8 |
| created | Timestamp the entry was written; sort/filter/purge basis | FS-033.2, .3, .4, .5, .7, .8 |
| updated | Last-touched timestamp (present in the record; not surfaced in the list) | (recognized as a sortable field internally) |

**Single-record load model**: a lightweight record loader (the `Log` class) hydrates one
log record by id and exposes read accessors for the attributes above (id, type, title,
body text, IP, created date). A static lookup helper returns the loaded record only when
the id is numeric and the loaded id matches the requested id, otherwise nothing. This
loader is the data source for the `/content/log/<id>` popover (FS-033.8). It performs no
writes.

**Configuration inputs consumed (owned by FS-032):**

| Setting (functional) | Used for |
|----------------------|----------|
| `log_level` | Write-side gate (FS-003) — referenced only |
| `log_graceperiod` | Auto-purge retention horizon in months (FS-033.7) |
| page size (`PAGE_LIMIT` source) | List pagination size (FS-033.5, FS-033.12) |
| lock time | `/config/scp` `lock_time` (FS-033.10) |
| date format | `/config/scp` `date_format` (FS-033.10) |
| staff max file uploads | `/config/scp` `max_file_uploads` (FS-033.10) |
| client file types / max file size / client max uploads | `/config/client` companion projection (FS-010/011) |
| `landing_page_id` / `offline_page_id` / `thank-you_page_id` | Default-page binding → in-use protection (BS-033.9) |

**Site Pages store** (`page` table, prefixed `ost_page`; canonical schema in FS-091): each
content page (the `Page` entity) exposes the following attributes consumed by this domain:

| Attribute | Functional meaning | Consumed by |
|-----------|--------------------|-------------|
| id | Stable record identifier; row key, edit target, default-page match key | FS-033.12–.16 |
| name | Unique page name; also slug source for `other` public URLs | FS-033.13, .14; BS-033.10 |
| type | One of `landing` / `offline` / `thank-you` / `other` (BS-033.8) | FS-033.13, .14 |
| body | The page content (HTML-safened rich text; `thank-you` supports ticket variables) | FS-033.13 |
| notes | Internal admin notes | FS-033.13 |
| isactive | Active (1) / Disabled (0) status | FS-033.12, .13, .15, .16 |
| created / updated | Insert / last-modified timestamps | FS-033.12 |
| topics (derived) | Count of help topics referencing the page; drives "in-use" | FS-033.12; BS-033.9 |

The `Page` entity supports create/update (with name-uniqueness + required-field
validation), enable/disable, and delete (cascading the referencing help topics' `page_id`
back to `0`). A static lookup helper returns a loaded page only when the id is numeric and
the loaded id matches the requested id, otherwise nothing.

---

## User Flows / Interactions

### Flow 1: Review and Filter the System Log
1. An administrator opens the System Logs screen from the dashboard area.
2. The list loads showing the most recent records first (id descending), paginated at the
   standard page size.
3. The admin picks a type (e.g., `Errors`) and/or enters a date span, then submits "Go!".
4. The list re-renders restricted to matching records; the caption reflects the scope and
   page range; the filter values remain populated in the form.
5. The admin clicks a column header (e.g., "Log Date") to sort, and pages through results;
   the active filter is preserved across sorting and paging.

### Flow 2: Inspect a Single Log Entry
1. From the list, the admin clicks a log title.
2. The content AJAX endpoint returns the detail popover: title, full body text, created
   date, and IP address.
3. If the underlying id is missing or invalid, the popover instead shows
   "Error: Unknown or invalid log ID".

### Flow 3: Bulk-Delete Log Entries
1. The admin selects rows (individually, or via All / None / Toggle).
2. The admin clicks "Delete Selected Entries".
3. A confirmation dialog warns the deletion is permanent; the admin confirms.
4. The selected records are deleted; a success / partial-count / failure message is shown,
   and the list re-renders.
5. If the admin submitted with no selection, the action is rejected with a guidance error
   and nothing is deleted.

### Flow 4: Automatic Aging-Out (no user action)
1. On each scheduled cron run, the purge task evaluates the configured grace period.
2. If a grace period is set, records older than that many months are deleted.
3. If no grace period is set, nothing is deleted (logs retained until manually removed).

### Flow 5: Front-End Pulls Config / Reference Content
1. A staff-control-panel script requests `/config/scp` and receives the JSON bundle
   (`lock_time` in seconds, `date_format`, `max_file_uploads`).
2. A template/auto-response editor requests `/content/ticket_variables` and displays the
   static ticket-variable reference card.

### Flow 6: Create / Edit a Site Page
1. An administrator opens the Site Pages screen from the manage area and sees the full page
   list (sorted by name ascending, paginated).
2. The admin clicks "Add New Page" (or a page name to edit), filling Name, Type, Status, the
   rich-text body, and optional admin notes.
3. On submit the input is validated (required type/name/body; unique name); on success the
   admin is returned to the list with a confirmation message; on a validation error the form
   re-renders with field-level errors and the submitted values.
4. For an `other`-typed page, the form shows the public `pages/<slug>` URL through which the
   page is reachable once active.

### Flow 7: Bulk Enable / Disable / Delete Pages
1. The admin selects rows and clicks Enable, Disable, or Delete; a confirmation dialog (with
   action-specific copy) gates the destructive ones.
2. If any selected page is a default (landing/offline/thank-you) page and the action is not
   Enable, the entire batch is rejected with the in-use warning.
3. Otherwise each selected page is processed; in-use pages refuse disable/delete and are
   skipped; the result is reported as full-success / partial-count / failure.

---

## Edge Cases & Error Scenarios

### EC-033.1: Empty Result Set
**Scenario**: The active filter matches no records.
**Behavior**: The caption shows "No logs found!", the table body is empty, and no page
links, selection toolbar, or delete control are rendered.

### EC-033.2: Invalid / Future Date Span
**Scenario**: Start date is in the future, or start is after a non-zero end date.
**Behavior**: The span is discarded, both bounds reset to "no restriction", and the message
"Entered date span is invalid. Selection ignored." is surfaced (BS-033.4).

### EC-033.3: Short / Unparseable Date Field
**Scenario**: A date field shorter than 8 characters is supplied.
**Behavior**: That bound is treated as absent (no date restriction from it); the field is
simply ignored rather than erroring.

### EC-033.4: Unrecognized Type Value
**Scenario**: A `type` parameter other than `error` / `warning` / `debug` (case-insensitive)
is supplied.
**Behavior**: No type restriction is applied and the scope reverts to "All logs".

### EC-033.5: Delete With No Selection
**Scenario**: The bulk form is submitted with no rows checked.
**Behavior**: The action is rejected with "You must select at least one log to delete"; no
records are deleted; previously-checked rows (if any survived) are restored on redisplay.

### EC-033.6: Partial Deletion
**Scenario**: Some selected ids no longer exist (already purged/deleted) when delete runs.
**Behavior**: The number actually deleted is compared to the number selected; if fewer, a
"N of M selected logs deleted" warning is shown instead of the full-success message.

### EC-033.7: Zero-Effect Deletion
**Scenario**: The delete query affects no rows and no prior error was set.
**Behavior**: The error "Unable to delete selected logs" is shown.

### EC-033.8: Unknown Command / Action on POST
**Scenario**: A POST arrives with an unrecognized `do` command, or `do=mass_process` with
an action other than `delete`.
**Behavior**: An error is surfaced ("Unknown command/action" for an unknown command;
"Unknown action - get technical help" for an unknown action under mass_process); no records
are deleted.

### EC-033.9: Log Detail Lookup Miss
**Scenario**: `/content/log/<id>` is called with a missing, non-numeric, or non-existent id.
**Behavior**: The error fragment "Error: Unknown or invalid log ID" is returned rather than
a record popover.

### EC-033.10: Grace Period Unset at Purge Time
**Scenario**: The cron purge sweep runs with no (or non-numeric, or `0`/"Never")
`log_graceperiod`.
**Behavior**: The sweep is a no-op and returns; no records are deleted (BS-033.6).

### EC-033.11: Duplicate Page Name
**Scenario**: A create/update submits a page name already used by another page.
**Behavior**: The save is aborted with the field error "Name already exists"; the form
re-renders with the submitted values (FS-033.14).

### EC-033.12: Attempt to Disable/Delete an In-Use Page
**Scenario**: An administrator tries to disable or delete a page that has linked help topics
or is a configured default page — via the entity guard or the bulk action.
**Behavior**: The single-page operation refuses (no write); in bulk, a default-page selection
is rejected up front ("...is in-use and CANNOT be disabled/deleted."), and any other in-use
page is skipped, yielding a partial-count or "Unable to..." result (FS-033.15, FS-033.16,
BS-033.9).

### EC-033.13: Edit Request for an Unknown Page Id
**Scenario**: `pages.php?id=<n>` is requested with an id that resolves to no page.
**Behavior**: The error "Unknown or invalid page" is set; an `update` POST against that id
yields "Invalid or unknown page" and nothing is changed.

### EC-033.14: Unknown Command / Action on Pages POST
**Scenario**: A POST to `pages.php` arrives with an unrecognized `do` command, or
`do=mass_process` with an action other than enable/disable/delete.
**Behavior**: "Unknown action/command" (unknown command) or "Unknown action - get technical
help." (unknown bulk action) is surfaced; no pages are changed.

### EC-033.15: Inactive or Non-Other Page at Public Slug Route
**Scenario**: The public `pages/<slug>` servlet matches a page that is inactive or whose type
is not `other`.
**Behavior**: A 404 "Page Not Found" is returned; only active `other` pages render (BS-033.10).

---

## Dependencies

| Dependency | Specification | Relationship |
|------------|---------------|--------------|
| Admin auth gate (`admin.inc.php`, admin-only check, CSRF) | FS-002 / FS-001 | Restricts the log viewer and AJAX endpoints to administrators; supplies CSRF token for the delete POST |
| Write-side logging (`osTicket::log()` family, severity mapping, log-level gating) | FS-003 | Produces the records this spec views/purges; owns the three-level severity model |
| System settings (`log_level`, `log_graceperiod`, page size, lock time, date format, upload limits) | FS-032 | Provides the configuration values this domain consumes |
| `syslog` table schema & enum sets | FS-091 | Canonical data model for log records (referenced, not restated) |
| Shared pagination (`Pagenate`, `PAGE_LIMIT`) and list/table UI | FS-090 | Drives the paginated results table and page links |
| Cron scheduler (`PurgeLogs` task and run cadence) | FS-043 | Invokes the auto-purge sweep on each scheduled run |
| Staff AJAX dispatcher / router (mounts `/content/...` and `/config/...`) | FS-043 | Routes the content/config AJAX endpoints to their handlers |
| Email templates & variable substitution (`VariableReplacer`) | FS-040 | Consumes the ticket-variable reference card; owns the actual substitution behavior |
| Public client portal upload config | FS-010 / FS-011 | Consumes the companion `/config/client` projection in the same handler |
| Display/formatting helpers (HTML-escaping, day-date-time formatting, sanitize, slugify) | FS-003 | Formats titles, dates, log body text; slugifies page names for `other`-page URLs |
| `page` table schema & page type enum | FS-091 | Canonical data model for content pages (referenced, not restated) |
| Default-page bindings (`landing_page_id` / `offline_page_id` / `thank-you_page_id`) | FS-032 | Bind pages to landing/offline/thank-you slots; consumed here for in-use protection |
| Landing-page render (`index.php`) | FS-010 | Renders the configured `landing` page body to the public root |
| Offline-banner render (`offline.php`) | FS-001 | Renders the configured `offline` page body in offline mode |
| Thank-you-page render after submission (`open.php`) | FS-011 | Renders the configured `thank-you` page body post-submission |
| Public custom-page servlet (`pages/index.php`) | FS-010 | Serves active `other` pages by slug; help topics bind a `thank-you` page (FS-030) |
| Help topics ↔ page reference | FS-030 | Help topics may reference a page (`topic.page_id`); drives the in-use count |

---

## Known Limitations

### KL-033.1: Site Pages Feature Constraints (Single Body, Fixed Type Set, No Versioning)
**Limitation**: The Site Pages content-management feature IS present in this 1.7 snapshot
(`scp/pages.php`, `class.page.php`, `ost_page` table, `pages.inc.php`/`page.inc.php` views —
see FS-033.11–.16). Its constraints are: (a) a page has a single rich-text body with no
revision history or draft/publish workflow — saving overwrites; (b) the type set is a fixed
closed enum (`landing` / `offline` / `thank-you` / `other`) with no operator-defined types;
(c) ticket-variable substitution is supported only in `thank-you` page bodies; (d) only one
page per default slot (landing/offline/thank-you) can be bound at a time via FS-032 settings;
(e) public free-standing URLs exist only for active `other` pages, addressed by name-derived
slug.
**Why It Exists**: The feature is intentionally minimal — a small editable-copy store for a
handful of well-known consumption sites rather than a general CMS.
**Impact**: Operators get editable landing/offline/thank-you/custom copy, but no page
versioning, scheduling, or per-page access control; mistaken overwrites are not recoverable.

> **Correction note**: An earlier draft of this spec wrongly asserted that no Site Pages
> feature existed in 1.7. That was incorrect — the feature is fully implemented. This KL now
> documents its real constraints instead.

### KL-033.2: Log Detail Popover Calls a Mismatched Accessor (`getIP()` vs `getIp()`)
**Limitation**: The content AJAX log handler references the record's IP via `getIP()`, but
the record loader defines the accessor as `getIp()` (different casing). On a case-sensitive
runtime this would fail to return the IP (or error), leaving the IP blank in the popover,
even though the IP is present in the record and shown correctly in the list column.
**Why It Exists**: An inconsistent method-name casing between the loader and its caller.
**Impact**: The "IP Address" line in the single-log popover may render empty; the list-table
IP column is unaffected (it reads the raw field directly). A production fix is a one-line
casing correction.

### KL-033.3: No Server-Side Validation Hardening on Log Filter/Sort Inputs
**Limitation**: Filter, sort, and pagination parameters are accepted from the request with
only allow-list checks for sort key and order direction; free-text date strings are parsed
loosely (any ≥8-char string is parsed). There is no rate-limiting on the viewer.
**Impact**: Malformed dates degrade gracefully (treated as absent / invalid-span rejected),
but the viewer leans on the allow-list rather than strict input typing. (SQL parameterization
for ids/types/dates is performed via the DB input helpers.)

### KL-033.4: Activity-Log Purge Not Implemented
**Limitation**: The auto-purge sweep contains a `// TODO: Activity logs` marker and only
purges the system log. There is no activity-log retention/purge in this version.
**Impact**: Only the `syslog` store is aged out by the grace-period sweep; any other
activity-style logs (e.g., ticket-event history) are not covered by this purge.

### KL-033.6: Log Type Filter Uses Raw Request Casing in the DB Match
**Limitation**: When a recognized type is chosen, the viewer assigns the **raw** request
value (e.g. lowercase `error` / `warning` / `debug` as submitted) into the `log_type=...` SQL
predicate, while stored records use capitalized `Error` / `Warning` / `Debug`. The dropdown's
own option values are capitalized (`Error`/`Warning`/`Debug`), so a normal dropdown submission
sends the capitalized value and matches correctly; but a manually-crafted lowercase `type`
query parameter would produce a predicate (`log_type='error'`) that matches no rows under a
case-sensitive collation. Separately, the dropdown's "selected" re-highlight compares the
(lowercased) `$type` against the capitalized option labels, so the previously-chosen type may
not appear re-selected after submit even though the filter is applied.
**Impact**: Filtering via the UI dropdown works; filtering via a hand-built lowercase URL may
silently return nothing on case-sensitive databases, and the active type may not visibly
re-highlight in the dropdown after a filter submit.

### KL-033.7: Page List "Disabled" Partial-Count Uses an Unset Variable
**Limitation**: In the bulk **disable** path the partial-success message
"N of M selected pages disabled" interpolates a variable (`$num`) that is not set on that
branch (the disable loop counts into a different variable, `$i`), so the displayed count may
be blank/zero rather than the true number disabled. The delete path correctly uses `$i`.
**Impact**: Cosmetic — a partial disable still disables the eligible pages and reports a
warning, but the count shown in the warning may be inaccurate. Full-success and failure
messages are unaffected.

### KL-033.5: Config/Content AJAX Handlers Mix Admin and Client Concerns
**Limitation**: The `ConfigAjaxAPI` handler defines both a staff (`scp`) and a client
projection; the `ContentAjaxAPI` handler defines both the admin-only log popover and a
generically-useful ticket-variable card. They share one file/handler each rather than being
split by audience.
**Impact**: Cross-references are needed at dedupe time (FS-010/011 for the client config
projection; FS-040 for the ticket-variable card consumer). Documented here so the
admin-vs-non-admin split is explicit. The log popover specifically is admin-gated; the
ticket-variable card and the client config projection are reachable by their respective
non-admin consumers.

---

## Future Considerations

- Extend the Site Pages feature with page versioning, draft/publish workflow, and
  operator-defined types (currently single-body, fixed type set — KL-033.1).
- Correct the `getIP()`/`getIp()` accessor mismatch so the log popover shows the IP
  (KL-033.2).
- Add an admin "purge now" control and a per-type retention horizon (currently a single
  global month-based grace period, cron-only — FS-033.7).
- Implement activity-log retention/purge to complement the system-log sweep (KL-033.4).
- Add export of the filtered log view (CSV) alongside the existing full-DB export path
  (FS-090).
- Split the config/content AJAX handlers by audience (admin vs client) for clearer
  authorization boundaries (KL-033.5).
