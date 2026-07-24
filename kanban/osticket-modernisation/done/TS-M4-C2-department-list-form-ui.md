# TS-M4-C2 — add(frontend): FS-030.3/.4 department list + add/edit form UI

- **ID**: TS-M4-C2
- **Type**: Technical Story
- **Parent**: US-M4-C1
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

The department screen inside the admin shell, wired to TS-M4-C1. The Email/Template selects resolve
against M4-D1 rows; SLA/manager selects from EPIC-M4-D/B; the group-access matrix from EPIC-M4-B.

## Impact

- add(frontend): `DepartmentListPage` — table; the default row's checkbox disabled (BS-030-05); bulk actions.
- add(frontend): `DepartmentFormDialog` — name, public/private, required Email + Template selects,
  optional SLA + manager selects, group-access checkbox matrix, auto-response override toggles;
  inline 422 errors + success banner. All selects source their options from
  `GET /api/staff/admin/departments/form-options` (`email_accounts`/`template_groups`/`sla`/`staff`/`groups`).
- refactor(frontend): **generalize `DeptAccessMatrix` → a reusable `AccessMatrix`** (generic
  `items` / `testIdPrefix` / `emptyLabel` props), or add a `GroupAccessMatrix` variant, so the dept
  form's group-access checkbox matrix reuses the shared component (also reused by C4/C6 rosters).
- add(store): **`DeptAdminStore`** (MobX, TDD) — list/form-options/create/update/delete/group-sync.

## Regressions

- None; additive screen.

## Acceptance Tests

### AC-1: FS-030.3 — list renders; default checkbox disabled. [BROWSER]
- Setup: `cargo run -p tools --bin seed`; log in as admin (`admin`/`Admin123!`) at http://localhost:3702/staff/login.
- Navigate: http://localhost:3702/staff/admin/departments
- Verify: the department table renders; the default (Support) row's selection checkbox is disabled.
- Status: [x]

### AC-2: FS-030.4 — form shows Email/Template (M4-D1) + SLA selects and saves. [BROWSER]
- Setup: admin on /staff/admin/departments.
- Action: click "Add Department".
- Verify: the Email select lists `support@osticket.local`; the Template select lists "osTicket Default"; the SLA select lists the seeded plans (all sourced from `GET /api/staff/admin/departments/form-options`).
- Action: type a >=4-char name; select Email + Template + an SLA; click Save.
- Verify: success banner; the new department appears in the table.
- Status: [x]

### AC-3: BS-030-02/03 — missing Email/Template surfaces inline errors. [BROWSER]
- Setup: admin on /staff/admin/departments; click "Add Department"; type a valid name.
- Action: click Save without selecting Email → inline "Email selection required".
- Action: select Email, leave Template unset; click Save → inline "Template selection required".
- Status: [x]

### AC-4: BS-030-04 — setting the default department private is rejected inline. [BROWSER]
- Setup: admin on /staff/admin/departments.
- Action: edit the default (Support) department; toggle Private; click Save.
- Verify: inline error "System default department cannot be private".
- Status: [x]

## Test Infrastructure

- Vitest + RTL + MSW mocking department CRUD + `GET /api/staff/admin/departments/form-options`. Admin account.

## Dependencies

- **TS-M4-A0**: shell. **TS-M4-C1**: department endpoints + form-options. **EPIC-M4-D/B**: SLA/manager selects.
- **Router**: adds `/staff/admin/departments` to the shared admin `<Route>` block in `router.tsx` —
  coordinate the single coherent router edit with C4/C6 + E2 (all C/E screens touch the same block).
