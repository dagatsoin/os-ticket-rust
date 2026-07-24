# TS-M4-PREP-A — add(schema): FS-091 additive admin columns on department/groups/staff

- **ID**: TS-M4-PREP-A
- **Type**: Technical Story
- **Parent**: EPIC-M4-PREP
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

The M1–M3 schema carries a partial admin data model. The admin CRUD screens (FS-030/031) need
columns that were never required by the runtime path. This TS adds them as an **additive migration**
— no data destruction, existing rows get sensible defaults.

## Impact

- add(schema): `department` columns — `email_id` (FK email_account, M4-D1), `tpl_id` (template_group),
  `sla_id` (FK sla_plan, nullable), `manager_id` (FK staff, nullable), `ispublic`, `group_membership`,
  `ticket_auto_response`, `message_auto_response`, `autoresp_email_id`, `signature`, `updated`.
- add(schema): `groups` columns — the net-new permission flags `can_manage_faq`, `can_manage_premade`,
  `can_ban_emails`, `can_view_staff_stats` (bool, default false), `notes`, `updated` (the 7 pre-existing
  M1/M3 flags remain).
- add(schema): `staff` columns — `isvisible` (directory-visible), `onvacation`, `signature`,
  `default_signature_type`, `default_paper_size`, `max_page_size`, `auto_refresh_rate`, `timezone_id`,
  `daylight_saving`, `change_passwd` (forced-change flag), `passwdreset` (aging timestamp), `notes`.
- Migration is idempotent-safe (`ADD COLUMN IF NOT EXISTS` / guarded) and `SQLX_OFFLINE`-friendly.

## Regressions

- M1/M2/M3 seed + queries must still compile and run; new columns are nullable or defaulted.
- The M1/M3 group flag columns and staff auth columns are untouched.

## Implementation note — columns that ALREADY EXIST (guard, do not re-create)

Verified against the M1–M3 migrations; these are already present and MUST NOT be re-added
(use `ADD COLUMN IF NOT EXISTS` or skip them — a bare `ADD COLUMN` errors on the 2nd apply
and breaks AC-4):
- `department`: `ispublic`, `ticket_auto_response`, `message_auto_response`, `updated`,
  `dept_signature` (this is the existing "signature"), and `sla_id` (added by
  `0006_department_sla_id.sql`). Genuinely net-new here: `email_id`, `tpl_id`, `manager_id`,
  `group_membership`, `autoresp_email_id`.
- `groups`: `notes`, `updated` already exist. Genuinely net-new: `can_manage_faq`,
  `can_manage_premade`, `can_ban_emails`, `can_view_staff_stats`.
- `staff`: `isvisible`, `signature`, `isadmin` already exist (`0001`). Genuinely net-new:
  `onvacation`, `default_signature_type`, `default_paper_size`, `max_page_size`,
  `auto_refresh_rate`, `timezone_id`, `daylight_saving`, `change_passwd`, `passwdreset`, `notes`.

## Acceptance Tests

### AC-1: FS-091 — department admin columns exist with correct types/defaults. [API-ONLY]
- Setup: apply migrations (`cargo run -p api` startup, or `DATABASE_URL=postgres://postgres:pass123@localhost:5432/osticket_dev sqlx migrate run --source migrations`)
- Request: `psql -U postgres -d osticket_dev -c "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name='department' AND column_name IN ('email_id','tpl_id','sla_id','manager_id','group_membership','autoresp_email_id') ORDER BY column_name"`
- Expect: all six rows returned; `email_id`/`tpl_id`/`manager_id`/`autoresp_email_id` are `integer` and `is_nullable=YES` (FK columns are nullable per M4-D1).
- Verify (data present): `psql -U postgres -d osticket_dev -c "SELECT email_id, tpl_id, sla_id, manager_id FROM department LIMIT 1"` succeeds (columns selectable).
- Status: [x]

### AC-2: FS-091 — the four net-new group permission flags exist defaulting false. [API-ONLY]
- Request: `psql -U postgres -d osticket_dev -c "SELECT column_name, column_default FROM information_schema.columns WHERE table_name='groups' AND column_name IN ('can_manage_faq','can_manage_premade','can_ban_emails','can_view_staff_stats') ORDER BY column_name"`
- Expect: four rows; every `column_default` is `false`.
- Verify (default false on the BASE agent group): `psql -U postgres -d osticket_dev -c "SELECT can_manage_faq, can_manage_premade, can_ban_emails, can_view_staff_stats FROM groups WHERE group_name = 'M1 Agents'"` returns all four columns `f` (the base seeded agent group inherits the false default — it was created before these flags existed and is not intentionally privileged).
- Verify (positive check on Administrators): `psql -U postgres -d osticket_dev -c "SELECT can_manage_faq, can_manage_premade, can_ban_emails, can_view_staff_stats FROM groups WHERE group_name = 'Administrators'"` returns all four columns `t` (the admin group seeded by TS-M4-PREP-C intentionally sets all four flags true — confirms the columns are writable and the seed applies them).
- Status: [x]

### AC-3: FS-091 — staff admin/preference columns exist. [API-ONLY]
- Request: `psql -U postgres -d osticket_dev -c "SELECT column_name FROM information_schema.columns WHERE table_name='staff' AND column_name IN ('onvacation','default_signature_type','default_paper_size','max_page_size','auto_refresh_rate','timezone_id','daylight_saving','change_passwd','passwdreset','notes') ORDER BY column_name"`
- Expect: all ten net-new columns returned.
- Verify: `psql -U postgres -d osticket_dev -c "SELECT onvacation, max_page_size, timezone_id, change_passwd, passwdreset FROM staff LIMIT 1"` succeeds.
- Status: [x]

### AC-4: Migration is idempotent — re-applying is a safe no-op. [API-ONLY]
- Setup: run `DATABASE_URL=... sqlx migrate run --source migrations` twice in a row
- Expect: the second run applies 0 new migrations and exits 0 (no "column already exists" error — proves the guarded `ADD COLUMN IF NOT EXISTS` path).
- Verify (schema stable): the `information_schema.columns` row counts from AC-1/AC-2/AC-3 are identical before and after the second run.
- Status: [x]

## Review feedback

- **AC-2**: The schema was always correct — all four flag columns exist with `column_default = false`
  (the primary Expect passed). Only the Verify sub-assertion was over-broad: it aggregated
  `bool_or(...)` across ALL groups, so the intentionally-privileged `Administrators` group (all four
  flags true, seeded by TS-M4-PREP-C for the admin account) legitimately flipped the result to `t`.
  Reworded to scope the default-false check to the base `M1 Agents` group and added a positive check
  that `Administrators` has the flags true. Status reset to `[ ]` for re-test. No code/schema change
  required.

## Test Infrastructure

- DB-backed test reads `TEST_DATABASE_URL` (skip-pass when unset, per project convention).
- No dev endpoint required — pure schema.
- These columns are the backing store for later admin-CRUD epics (M4-B staff/groups edit forms,
  M4-C department edit form); no runtime read/write is asserted here, only shape.

## Dependencies

- **M1/M3 schema (done)**: base `department`/`groups`/`staff` tables.
- **TS-M4-PREP-B**: `email_account`/`template_group`/`timezone` tables must exist for the FK columns
  (order PREP-B before PREP-A, or combine FK creation with PREP-B).
