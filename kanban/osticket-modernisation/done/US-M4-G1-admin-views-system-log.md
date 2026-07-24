# US-M4-G1 — Admin views, filters, sorts and prunes the system log

- **ID**: US-M4-G1
- **Type**: User Story
- **Parent**: EPIC-M4-G
- **Labels**: User Story, M4
- **Scope**: medium

## Spec References

- FS-033.1 (log viewer entry & gate), FS-033.2 (filter type + date span), FS-033.3 (results table)
- FS-033.4 (sorting), FS-033.5 (pagination), FS-033.6 (bulk manual deletion)
- FS-033.7 (automatic log purge — grace-period sweep), FS-033.8 (single record detail — content AJAX)
- BS-033.6 (auto-purge grace-period-gated, cron-driven), KL-033.7 (modernised)
- FS-003 (logging infra — MINIMAL, DEVIATION M4-D2)

## Context

Epic: EPIC-M4-G — System Logs. The viewer needs real rows, so the epic also builds a minimal
write-side logging facility (M4-D2) that records `syslog` rows on notable events. This story delivers
the admin-facing viewer + the manual purge trigger.

## Description

As an administrator, I can open the System Logs viewer, filter by log type (Error/Warning/Debug) and
date span, sort and page the results, open a single entry's detail, and delete entries in bulk. I can
also trigger a grace-period purge (which, in production, is cron-driven — M6). The log is populated by
the running app via the minimal write-side logging facility.

## Impact

- Frontend (log viewer: filters, table, detail, bulk delete, purge trigger)
- Backend (write-side logging facility + viewer endpoints + purge sweep function)
- Database (`syslog`, `config.log_graceperiod`)
- Browser (desktop)

## Business Rules

- FS-033.2: filter by type + date span; FS-033.4/.5 sortable + paginated.
- FS-033.6: bulk manual deletion of selected entries.
- BS-033.6: auto-purge is grace-period-gated (`log_graceperiod` months); the cron trigger is M6, but
  the sweep function is exposed via a dev/manual trigger now (DEVIATION M4-D2).

## Regressions

- The write-side logging must be lightweight and must not break the events it hooks (login, admin CRUD).

## Acceptance Criteria

### AC-1: The log viewer shows real rows produced by app events. [BROWSER]
- Setup: reseed; purge to a known state (`POST http://localhost:3701/api/dev/purge-logs`); then generate rows via app events — log in `admin`/`Admin123!` and create a group at /staff/admin/groups (admin CRUD emits a syslog row). Optionally top up with `POST /api/dev/seed-log` for varied types/dates.
- Navigate: http://localhost:3702/staff/login → admin session → /staff/admin/logs.
- Verify: the table lists log entries with type, title, and date columns; the rows include the entry(ies) produced by the events above.
- Status: [x] — VERIFIED in-browser at /staff/admin/logs: table renders real rows with Type/Title/Date columns, including app-event rows (Group created, Staff login, Staff login failed, SLA plan created/deleted, Page created/deleted) produced by live admin CRUD + login events.

### AC-2: Filter by type and date span. [BROWSER]
- Setup: ensure rows of at least two types exist — `POST /api/dev/seed-log {type:"Error", ...}` and `{type:"Debug", ...}` with differing dates.
- Action: at /staff/admin/logs set the type filter to a single type (e.g. Error) and a date span covering it; click Apply.
- Verify: the table narrows to only matching entries (other types/dates excluded).
- Status: [x] — VERIFIED in-browser: Type=Warning + Apply → narrowed to the single Warning row "Staff login failed" ("Showing 1-1 of 1"). Adding From=25/07/2026 (after that row's 24/07 timestamp) → "No log entries found", confirming the date-span bound also narrows. (Date filter additionally proven end-to-end at API level in TS-M4-G2 AC-1.)

### AC-3: Sort and paginate. [BROWSER]
- Setup: seed enough rows to exceed one page (`POST /api/dev/seed-log` x N).
- Action: click a sortable header (e.g. Date); then use the pagination control to page through results.
- Verify: row order changes on sort; pagination advances to the next page of rows.
- Status: [x] — VERIFIED in-browser: clicking the Date header toggled order (asc shows oldest "NOOP overage 01/01/2019" first; desc shows newest first). Pagination: "Showing 1-25 of 29" page 1 → NEXT → "Showing 26-29 of 29" page 2 with distinct rows.

### AC-4: Open a single log entry's detail. [BROWSER]
- Setup: continue as admin at /staff/admin/logs with rows present.
- Action: click a log row.
- Verify: a detail view/panel/dialog shows the full log body/detail text (content AJAX, FS-033.8).
- Status: [x] — VERIFIED in-browser: clicking a "Group created" row opened a MUI detail dialog titled "Group created" showing "Debug · 24/07/2026 06:13:27" and the full body "Group 'GateGrp Written' (#27) created by staff #125".

### AC-5: Bulk delete selected entries. [BROWSER]
- Setup: continue as admin at /staff/admin/logs with at least two rows.
- Action: select two entries via their checkboxes; click Delete; confirm.
- Verify: both rows are removed from the table.
- Status: [x] — VERIFIED in-browser: selected 2 rows (checkboxes → "2 selected") → DELETE → MUI confirm "Delete 2 log entries?" → DELETE → total dropped 29→27 and the two selected rows disappeared.

### AC-6: The purge sweep deletes entries older than the grace period. [API-ONLY]
- Setup: admin session; set `log_graceperiod` to a non-zero month count; seed an over-age row `POST /api/dev/seed-log {type:"Debug", created:"2020-01-01"}` and a recent row `POST /api/dev/seed-log {created:"<today>"}`.
- Request: POST http://localhost:3701/api/dev/purge-logs (manual trigger standing in for the M6 cron; now calls the grace-period sweep, not TRUNCATE-all).
- Expect: 200; a subsequent GET /api/staff/admin/logs shows the over-age row gone and the recent row retained (BS-033.6, KL-033.7 modernised). With `log_graceperiod` unset/zero/non-numeric the sweep is a no-op.
- Status: [x] — VERIFIED live: with log_graceperiod=12, POST /api/dev/purge-logs → 200 {deleted:3}; over-age 2020 row gone, recent row kept. No-op confirmed for graceperiod=0, "" (empty), and "abc" (non-numeric) — over-age row retained, deleted:0.

### AC-7: The "Purge now" button removes over-age rows in the UI. [BROWSER]
- Setup: admin session; set `log_graceperiod` to a non-zero month count; seed an over-age row (`POST /api/dev/seed-log {created:"2020-01-01T00:00:00Z"}`) and a recent row.
- Navigate: http://localhost:3702/staff/login → admin session → /staff/admin/logs.
- Action: click the visible "Purge now" control → confirm.
- Verify: the over-age row disappears from the table and the recent row remains.
- Status: [x] — VERIFIED in-browser: with log_graceperiod=12 and a seeded 2020 over-age + a recent row, clicking "Purge now" → MUI confirm "Purge old log entries" → PURGE NOW → the over-age rows (2020 "UI PURGE overage", 2019 "NOOP overage") vanished from the table while "UI PURGE recent" remained (DB confirms overage=0, recent=1).

## Checklist (children)

- [ ] TS-M4-G1 — Backend: minimal write-side logging facility (M4-D2) + event wiring
- [ ] TS-M4-G2 — Backend: log viewer endpoints + purge sweep function
- [ ] TS-M4-G3 — Frontend: system log viewer UI

## Test Infrastructure

- Admin account (`admin`/`Admin123!`). Dev endpoints (both exist from M4-PREP):
  - `POST /api/dev/seed-log {type,title,log[,created]}` — inserts a syslog row (optional backdated `created`, added by TS-M4-G1).
  - `POST /api/dev/purge-logs` — repointed by TS-M4-G2 to invoke the **grace-period sweep** (over-age only); a full-clear reset path is kept separately if setup needs it.
- **INFRA (owned inside this epic — both are in-scope, not external blockers):**
  1. **TS-M4-G1** extends `seed-log` with an **optional `created` timestamp** (backdated rows; stamps `now()` when omitted) so AC-6/AC-7 (and TS-M4-G1 AC-4 / TS-M4-G2 AC-5) can seed an over-age row.
  2. **TS-M4-G2** implements the real grace-period `purge_logs()` sweep (delete over-age only, keep recent; months-based; no-op when `log_graceperiod` is unset/zero/non-numeric) and **repoints `POST /api/dev/purge-logs`** at it (replacing the TRUNCATE-all behaviour).

## Dependencies

- **EPIC-M4-PREP**: `syslog` table + `log_graceperiod` key.
- **EPIC-M4-A (shell)**. Forward (M6): cron invokes the purge sweep function.
