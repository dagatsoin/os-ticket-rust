# EPIC-M4-G — EPIC – System Logs

- **ID**: EPIC-M4-G
- **Type**: Epic
- **Parent**: M4
- **Labels**: Epic, M4
- **Column**: derived from children

## Spec References

- FS-033.1 (system log viewer entry point & access gate)
- FS-033.2 (log filtering — type and date span)
- FS-033.3 (log results table)
- FS-033.4 (log sorting)
- FS-033.5 (log pagination)
- FS-033.6 (bulk manual deletion of log entries)
- FS-033.7 (automatic log purge — grace-period sweep)
- FS-033.8 (single log record detail — content AJAX `/content/log/<id>`)
- BS-033.6 (auto-purge is grace-period-gated and cron-driven)
- FS-003 (logging infrastructure — MINIMAL subset only, DEVIATION M4-D2)

## Context

Milestone: M4 — Admin Configuration. The log viewer needs real rows, so this epic also builds a
**minimal write-side logging facility** (DEVIATION M4-D2) — an `osTicket::log()`-equivalent that
records `syslog` rows on notable admin/system events (staff login, admin CRUD, mailer errors). Full
FS-003 logging is out of scope. The purge sweep **function** lands here, invoked by a dev/manual
trigger; the cron **trigger** is a noted M6 forward-dependency.

## Description

- **Write-side logging** (M4-D2): a thin `log(type, title, detail)` writer that inserts `syslog`
  rows; wired onto a handful of notable events so the viewer has real data.
- **Log viewer**: list filtered by type (Error/Warning/Debug) and date span; sortable columns
  (KL-032.10-style "not sortable" bugs avoided); pagination; single-record detail via content AJAX;
  bulk manual deletion.
- **Purge sweep function**: grace-period-gated deletion (`log_graceperiod` months), exposed via a
  dev/manual trigger endpoint for now (cron trigger deferred to M6). KL-033.7 modernised.

## Business value

Operators get audit visibility into system events and can prune old entries. The minimal write-side
facility means the viewer is populated by the running app rather than mocked.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M4-G1](US-M4-G1-admin-views-system-log.md) — Admin views, filters, sorts and prunes the system log
  - [ ] [TS-M4-G1](TS-M4-G1-write-side-logging-facility.md) — Backend: minimal write-side logging facility (M4-D2) + event wiring
  - [ ] [TS-M4-G2](TS-M4-G2-log-viewer-endpoints-purge.md) — Backend: log viewer endpoints (list/filter/sort/paginate/detail/bulk-delete) + purge sweep fn
  - [ ] [TS-M4-G3](TS-M4-G3-log-viewer-ui.md) — Frontend: system log viewer UI

## Dependencies

- **EPIC-M4-PREP**: `syslog` table + `log_graceperiod` config key.
- **EPIC-M4-A (shell)**: admin screens mount inside TS-M4-A0.
- **Forward (M6)**: cron scheduler invokes the purge sweep function (FS-043).
