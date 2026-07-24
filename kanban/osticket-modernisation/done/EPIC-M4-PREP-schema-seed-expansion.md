# EPIC-M4-PREP — EPIC – Schema & Seed Expansion

- **ID**: EPIC-M4-PREP
- **Type**: Epic
- **Parent**: M4
- **Labels**: Epic, M4
- **Column**: derived from children

## Spec References

- FS-091 (reference data / data model — the authoritative column & table shapes)
- FS-030 / FS-031 / FS-032 / FS-033 (admin columns and net-new tables these epics require)
- BS-030-02/03 (department email/template selects → DEVIATION M4-D1)
- FS-033.7 / BS-033.6 (syslog + purge → DEVIATION M4-D2 supporting schema)

## Context

Milestone: M4 — Admin Configuration. This epic is the **dependency root for every other M4 epic**.
The M1–M3 schema carries a partial admin data model (department, sla_plan, groups,
group_dept_access, team, team_member, help_topic, priority, config, canned_response); M4 adds the
missing admin-CRUD columns, six net-new tables, the SLA transient trump key, and the ~110 seeded
config keys the settings tabs read/write.

## Description

Additive migrations + seed expansion, no behaviour of its own beyond making the data model M4-ready:
- **Missing columns** on `department`, `groups`, `staff` (admin-CRUD fields not needed until now).
- **Net-new tables**: `faq_category`, `page`, `syslog`, `timezone`, `email_account`, `template_group`.
- **SLA transient trump key** column supporting the FS-021.13 explicit-trump precedence slot.
- **~110 core config keys** seeded to FS-032 defaults, plus `default_dept_id`, `default_sla_id`,
  and the `*_page_id` bindings (`landing_page_id`, `offline_page_id`, `thank-you_page_id`).
- **Admin account** `admin`/`Admin123!` (`isadmin=true`) seeded (TS-M4-PREP-C AC-5) — the login every
  M4 admin-panel epic authenticates with; M1 only seeds non-admin `agent`/`agent2`.
- **DEVIATION M4-D1** minimal `email_account` + `template_group` rows so department Email/Template
  selects and the FS-032 Emails tab resolve against real rows (M5/FS-040 extends this model).

## Test infrastructure PREP provides / owes to downstream M4 epics

- **Admin login** `admin`/`Admin123!` (PREP-C AC-5) — Setup credential for all M4 admin `[BROWSER]` E2Es.
- **email_account / template_group / timezone rows** (PREP-D) — options for the M4-C dept selects and
  M4-B profile timezone select.
- **Open gaps for the consuming epics to close** (raised here, not blocking PREP; not schema/seed):
  - `seed_config` upserts preserve existing values, so `seed --reset` does NOT restore admin-mutated
    settings. M4-A settings-tab E2Es need a config-reset path (`--reset` forces FS-032 defaults, or a
    dev `POST /api/dev/reset-config`).
  - Admin-CRUD E2Es on the net-new tables (M4-E FAQ, M4-F pages, M4-H canned) create/edit/delete rows,
    but `--reset` only purges ticket-scoped data. Extend the reset purge set or add dev seed endpoints.
  - `syslog` has no writer yet — M4-G's log viewer needs generated/seeded rows (minimal logging path
    or a dev `POST /api/dev/seed-syslog`) to have anything to display.

## Business value

Unblocks all admin CRUD. Every downstream M4 screen reads from and writes to these tables; without
PREP the department form has no Email/Template options, the settings tabs have no keys to persist,
and the log viewer has no `syslog` table to read.

## Acceptance Criteria

> No ACs — infrastructure epic; verification owned by the leaf TS tickets. Column derived from children.

## Children (column derived: min of these)

- [x] [TS-M4-PREP-A](TS-M4-PREP-A-schema-additive-columns.md) — Migration: additive admin columns on department/groups/staff
- [x] [TS-M4-PREP-B](TS-M4-PREP-B-schema-net-new-tables.md) — Migration: net-new tables + SLA transient trump key
- [x] [TS-M4-PREP-C](TS-M4-PREP-C-seed-config-keys-defaults.md) — Seed: ~110 config keys + default_dept/sla/*_page_id bindings
- [x] [TS-M4-PREP-D](TS-M4-PREP-D-seed-email-template-timezone.md) — Seed: minimal email_account + template_group + timezone (M4-D1)

## Dependencies

- **M1/M2/M3 (done)**: base schema + idempotent seed binary + `--reset` flow.
- Blocks: EPIC-M4-A/B/C/D/E/F/G/H (all consume PREP schema/seed).
