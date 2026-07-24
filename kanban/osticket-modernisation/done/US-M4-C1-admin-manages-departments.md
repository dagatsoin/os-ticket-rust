# US-M4-C1 — Admin manages departments

- **ID**: US-M4-C1
- **Type**: User Story
- **Parent**: EPIC-M4-C
- **Labels**: User Story, M4
- **Scope**: medium

## Spec References

- FS-030.3 (department list view), FS-030.4 (add/edit form), FS-030.5 (create & update)
- FS-030.6 (deletion & re-homing), FS-030.7 (bulk actions)
- BS-030-01 (name required/min/unique), BS-030-02 (email required), BS-030-03 (template required)
- BS-030-04/05 (default department protections), BS-030-06/07 (delete guard + re-home)
- BS-030-08 (allowed-group access full-replace sync), BS-030-09 (auto-response overrides)
- KL-030-11 (no-change update guard — modernised)

## Context

Epic: EPIC-M4-C — Departments, Teams & Help Topics. Departments are the ownership/routing anchor.
This story delivers their list and add/edit form, with the required Email/Template selects
(resolving against M4-D1 rows), the SLA/priority/manager selects (from EPIC-M4-D/B), the group-access
matrix, and the default-department protections and delete re-homing.

## Description

As an administrator, I can list departments and open a form to create or edit one: name, public/
private, required **Email** and **Template** selection, optional SLA and manager, the group-access
checkbox matrix, and auto-response overrides. Deleting a department re-homes its tickets and help
topics to the default department (its home staff must be moved away first). The default department
cannot be made private, deleted, or disabled.

## Impact

- Frontend (department list + add/edit form + bulk actions)
- Backend (department CRUD + delete re-home + default protection + group-access sync)
- Database (`department`, `group_dept_access`, `email_account`, `template_group`, `sla_plan`, `staff`, `ticket`, `help_topic`)
- Browser (desktop)

## Business Rules

- BS-030-01: name required, >= 4 chars, unique.
- BS-030-02/03: an outbound email account and a template value are required.
- BS-030-04/05: the default department stays public and cannot be deleted/disabled (disabled checkbox).
- BS-030-06/07: delete refused while any home staff (`staff.dept_id==id`) remain; once none remain, delete re-homes **tickets and help topics only** (`ticket.dept_id` + `help_topic.dept_id` → default); staff are NOT re-homed (they were the delete precondition).
- BS-030-08: saving replaces the group-access set.

## Regressions

- M3 ticket visibility/transfer keeps working; the seeded Support/Sales departments remain valid.
- Re-homing must not orphan tickets/topics/staff.

## Acceptance Criteria

### AC-1: The department list renders; the default department's checkbox is disabled. [BROWSER]
- Setup: `cargo run -p tools --bin seed` (idempotent dev-reset — seeds the `Support` default department); optionally `POST /api/dev/reset-departments` for a clean non-default set.
- Setup: log in as admin (`admin` / `Admin123!`) at http://localhost:3702/staff/login.
- Navigate: http://localhost:3702/staff/admin/departments
- Verify: a table lists departments; the default department (Support) row renders its selection checkbox disabled (BS-030-05).
- Status: [x]

### AC-2: Admin creates a department with required Email + Template (M4-D1) and an SLA. [BROWSER]
- Setup: admin on /staff/admin/departments (from AC-1).
- Action: click "Add Department".
- Action: type "QA Escalations" (>=4 chars) into Name.
- Action: select the seeded email account (`support@osticket.local`) in the Email select; select "osTicket Default" in the Template select.
- Action: select an SLA plan (e.g. "Default SLA") and a manager (a seeded staff, e.g. `agent`).
- Action: check at least one group in the group-access matrix; click Save.
- Verify: a success banner appears; "QA Escalations" now appears in the department table.
- Status: [x]

### AC-3: Missing Email or Template is rejected. [BROWSER]
- Setup: admin on /staff/admin/departments; click "Add Department"; type a valid >=4-char name.
- Action: leave the Email select unset; click Save.
- Verify: inline error "Email selection required" (BS-030-02).
- Action: set Email; clear the Template select; click Save.
- Verify: inline error "Template selection required" (BS-030-03).
- Status: [x]

### AC-4: The default department cannot be made private or deleted. [BROWSER]
- Setup: admin on /staff/admin/departments.
- Action: edit the default department (Support); attempt to toggle it to Private; click Save.
- Verify: inline error "System default department cannot be private" (BS-030-04).
- Action: attempt to delete the default department (its list checkbox is disabled / the delete action is blocked).
- Verify: deletion is refused (BS-030-05).
- Status: [x]

### AC-5: Deleting a department re-homes its tickets + topics to the default. [BROWSER]
- Setup: create a non-default department "Temp Dept" (AC-2 flow); seed a ticket homed to it (`POST /api/dev/seed-ticket` with its dept id), and ensure no home staff remain in it (move any away first — BS-030-06 guard refuses while home staff exist).
- Action: on /staff/admin/departments, delete "Temp Dept".
- Verify: success message; the ticket previously homed to "Temp Dept" now points to the default department (confirm in the ticket queue or via `GET /api/staff/admin/departments`); help topics homed to it are likewise re-homed; no orphaned tickets/topics (BS-030-07 — staff are not re-homed, they were moved as the precondition).
- Status: [x]

### AC-6: Group-access is a full-replace sync. [API-ONLY]
- Setup: admin session cookie (login via `POST /api/staff/login` as `admin`/`Admin123!`); a department id with an existing group-access set (from AC-2).
- Request: `PUT /api/staff/admin/departments/:id` with a changed `groups` array.
- Headers: admin session cookie.
- Expect: 200; a follow-up `GET /api/staff/admin/departments/:id` returns exactly the submitted group set (the manager/primary group retained) — BS-030-08.
- Status: [x]

## Checklist (children)

- [ ] TS-M4-C1 — Backend: department CRUD + delete re-home + default protection + group-access sync
- [ ] TS-M4-C2 — Frontend: department list + add/edit form UI

## Test Infrastructure

- Admin account (`admin`/`Admin123!`). Requires seeded email_account/template_group (M4-D1), SLA plans (EPIC-M4-D), staff (EPIC-M4-B). `seed-dept` and `seed-ticket` dev endpoints already exist. Form selects (email/template/sla/manager/groups) are served by `GET /api/staff/admin/departments/form-options` (TS-M4-C1) — the `email_accounts`/`template_groups` reads are net-new there.
- Dev endpoint **`POST /api/dev/reset-departments`** for clean state — **now declared as a deliverable of TS-M4-C1** (`add(dev)`), mirroring reset-teams/reset-help-topics/reset-faq-categories. Consolidation gap resolved.

## Dependencies

- **EPIC-M4-PREP**: department additive columns, email_account/template_group.
- **EPIC-M4-A (shell)**, **EPIC-M4-B (staff/groups)**, **EPIC-M4-D (SLA)**.
