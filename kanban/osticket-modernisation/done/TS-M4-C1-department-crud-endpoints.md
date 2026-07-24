# TS-M4-C1 — add(route): FS-030.5/.6/.7 department CRUD + delete re-home + default protection + group-access sync

- **ID**: TS-M4-C1
- **Type**: Technical Story
- **Parent**: US-M4-C1
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

Backend for department management: list, create/edit with required email/template + optional
SLA/manager + group-access sync, default-department protections, delete re-homing, and bulk actions.

## Impact

- add(route): `GET /api/staff/admin/departments`, `POST`, `PUT /:id`, bulk endpoint — admin-gated.
- add(route): **`GET /api/staff/admin/departments/form-options`** → `{email_accounts:[{id,email,name}],
  template_groups:[{id,name}], sla:[{id,name}], staff:[{id,name}], groups:[{id,name}]}` — the source
  for the dept form's Email/Template/SLA/manager/group-access selects. The `email_accounts` and
  `template_groups` reads are **NET-NEW and REQUIRED** (no prior endpoint exposes them); `sla`/`staff`/
  `groups` may reuse EPIC-M4-D/B reads but MUST be surfaced through this one options payload.
- add(service): validation — BS-030-01 (name required, ≥4 chars, unique), BS-030-02/03
  (`email_id` + `tpl_id` both REQUIRED → 422 "Email/Template selection required"; the two-field form
  surfaces the distinct "Email selection required" / "Template selection required" messages per field),
  BS-030-04 (default not private), BS-030-08 (group-access full-replace).
- add(service): delete — BS-030-06 (refuse if any home staff `staff.dept_id==id` remain),
  BS-030-07 (re-home moves **only `ticket.dept_id` + `help_topic.dept_id` → `default_dept_id`**, NOT
  staff; drop this dept's `group_dept_access` rows), BS-030-05 (default excluded from delete/disable).
  Clarifies BS-030-06↔07: the guard refuses while home staff exist; re-home moves tickets + topics only.
- add(service): group-access full-replace on save (BS-030-08) — **inverted sync**:
  `DELETE FROM group_dept_access WHERE dept_id=:id` then insert one row per submitted `group_id`.
- BS-030-29 id-mismatch guard; KL-030-11 no-change guard MODERNISED (a no-op re-save reports success).
- add(dev): **`POST /api/dev/reset-departments`** (consolidation gap — the US/TS test infra reference it but no TS declared it; add it here for clean-state setup, mirroring reset-teams/reset-help-topics).

## Regressions

- M3 transfer/visibility unaffected; the seeded Support default department stays protected.

## Acceptance Tests

### AC-1: BS-030-02/03 — email + template required. [API-ONLY]
- Setup: `cargo run -p tools --bin seed`; admin session cookie (`POST /api/staff/login` as `admin`/`Admin123!`).
- Request: `POST /api/staff/admin/departments` with a valid name but no email → Expect 422 `{error.message}` "Email selection required".
- Request: `POST /api/staff/admin/departments` with email set but no template → Expect 422 "Template selection required".
- Status: [x]

### AC-2: BS-030-04/05 — default department cannot be private/deleted. [API-ONLY]
- Setup: admin cookie; discover the default department id via `GET /api/staff/admin/departments`.
- Request: `PUT /api/staff/admin/departments/:defaultId` with `ispublic=false` → Expect 422 "System default department cannot be private".
- Request: `DELETE /api/staff/admin/departments/:defaultId` → Expect refusal (422/403), department still present.
- Status: [x]

### AC-3: BS-030-06/07 — delete refused with home staff; else re-homes tickets + topics only. [API-ONLY]
- Setup: admin cookie; create a non-default dept via POST; seed a staff homed to it (`staff.dept_id==id`), a ticket homed to it and a help topic homed to it (`seed-staff` with its dept_id, `seed-ticket` + `seed-topic` with its dept id).
- Request: `DELETE /api/staff/admin/departments/:id` while home staff remain → Expect refusal (BS-030-06 guard: refuses while any `staff.dept_id==id` remain).
- Action: move the staff to another department (`PUT` staff or re-seed); then `DELETE /api/staff/admin/departments/:id` → Expect 200; the re-home moves **only** `ticket.dept_id` and `help_topic.dept_id` to `default_dept_id` (staff are NOT re-homed — they were the delete precondition). `GET` on the moved ticket/topic shows them re-homed to the default dept; the dept's `group_dept_access` rows are gone (BS-030-07).
- Status: [x]

### AC-4: BS-030-08 — group-access full-replace sync. [API-ONLY]
- Setup: admin cookie; a department id with an initial group-access set.
- Request: `PUT /api/staff/admin/departments/:id` with a different `groups` array → Expect 200; `GET /api/staff/admin/departments/:id` returns exactly the submitted set (manager/primary group retained).
- Status: [x]

### AC-5: KL-030-11 — no-change re-save reports success (modernised). [API-ONLY]
- Setup: admin cookie; an existing department fetched via GET.
- Request: `PUT /api/staff/admin/departments/:id` re-submitting the identical payload (no field changes) → Expect 200 success, NOT an "Unable to update" error (KL-030-11 modernised).
- Status: [x]

### AC-6: form-options endpoint surfaces all select sources (incl. NET-NEW email/template reads). [API-ONLY]
- Setup: `cargo run -p tools --bin seed`; admin session cookie. Ensure the seeded `support@osticket.local` email account and "osTicket Default" template group exist (M4-D1 rows).
- Request: `GET /api/staff/admin/departments/form-options` → Expect 200 with keys `email_accounts`, `template_groups`, `sla`, `staff`, `groups`; `email_accounts` includes the seeded account (`{id,email,name}`) and `template_groups` includes "osTicket Default" (`{id,name}`) — proving the net-new reads resolve.
- Status: [x]

## Test Infrastructure

- Admin account; dev `reset-departments`. `.sqlx` cache updated. Requires M4-D1 rows (email_account + template_group) + SLA plans. The net-new `email_accounts`/`template_groups` reads are exposed via `GET /api/staff/admin/departments/form-options`.

## Dependencies

- **EPIC-M4-PREP**, **TS-M4-A0**, **EPIC-M4-B (staff/groups)**, **EPIC-M4-D (SLA)**.
