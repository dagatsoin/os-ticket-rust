# TS-M4-G3 — add(frontend): FS-033.1/.3 system log viewer UI

- **ID**: TS-M4-G3
- **Type**: Technical Story
- **Parent**: US-M4-G1
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

The System Logs viewer inside the admin shell, wired to TS-M4-G2.

## Impact

- add(frontend): `LogViewerPage` — a type filter + a date-span control (using **native MUI
  `TextField type="date"`** — no new date-picker dependency), a sortable/paginated results table
  (type, title, date), a detail panel/dialog (content AJAX), row checkboxes + bulk delete, and a
  **visible "Purge now" control** invoking the manual grace-period purge trigger.

## Regressions

- None; additive.

## Acceptance Tests

### AC-1: FS-033.3 — the log table renders real rows. [BROWSER]
- Setup: purge (`POST /api/dev/purge-logs`); seed rows (`POST /api/dev/seed-log` x N) or trigger app events.
- Navigate: http://localhost:3702/staff/login → log in `admin`/`Admin123!` → /staff/admin/logs.
- Verify: the table renders rows with type, title, date columns.
- Status: [x] — VERIFIED in-browser at /staff/admin/logs: table renders real rows with Type/Title/Date columns (Group created, Staff login, SLA plan created, Page created, etc.).

### AC-2: FS-033.2 — filter by type + date. [BROWSER]
- Setup: continue as admin; rows of ≥2 types present.
- Action: set the type filter + a date span; click Apply.
- Verify: the table narrows to matching rows.
- Status: [x] — VERIFIED in-browser: Type=Warning + Apply → narrowed to the single Warning row ("Showing 1-1 of 1"); adding From=25/07/2026 → "No log entries found" (date bound narrows). Native MUI date TextFields present and wired.

### AC-3: FS-033.8 — open an entry's detail. [BROWSER]
- Setup: continue as admin at /staff/admin/logs with rows.
- Action: click a row.
- Verify: a detail panel/dialog shows the full log body (content AJAX).
- Status: [x] — VERIFIED in-browser: clicking a "Group created" row opened a MUI dialog with type/date header and the full body "Group 'GateGrp Written' (#27) created by staff #125" (fetched on click).

### AC-4: FS-033.6 — bulk delete. [BROWSER]
- Setup: continue as admin; ≥2 rows.
- Action: select rows via checkboxes → Delete → confirm.
- Verify: the selected rows are removed from the table.
- Status: [x] — VERIFIED in-browser: selected 2 rows → DELETE → MUI confirm "Delete 2 log entries?" → DELETE → total 29→27, selected rows removed.

### AC-5: BS-033.6 — "Purge now" removes over-age rows, keeps recent. [BROWSER]
- Setup: admin session; set `log_graceperiod` to a non-zero month count; seed an over-age row (`POST /api/dev/seed-log {created:"2020-01-01T00:00:00Z"}`) and a recent row (`POST /api/dev/seed-log` with no `created`).
- Navigate: /staff/admin/logs.
- Action: click the visible "Purge now" control → confirm the prompt.
- Verify: the over-age row is removed from the table and the recent row is retained.
- Status: [x] — VERIFIED in-browser: log_graceperiod=12 + seeded 2020 over-age + recent row → "Purge now" → MUI confirm → PURGE NOW → over-age rows removed from the table, "UI PURGE recent" retained (DB: overage=0, recent=1).

## Test Infrastructure

- Vitest + RTL + MSW mocking `/api/staff/admin/logs*`. Admin account.

## Dependencies

- **TS-M4-A0**: shell. **TS-M4-G2**: log endpoints. **TS-M4-G1**: real rows.
