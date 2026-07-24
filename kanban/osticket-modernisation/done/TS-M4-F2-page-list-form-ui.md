# TS-M4-F2 — add(frontend): FS-033.11/.12/.13 site pages list + create/edit form UI

- **ID**: TS-M4-F2
- **Type**: Technical Story
- **Parent**: US-M4-F1
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

The site-pages screen inside the admin shell, wired to TS-M4-F1.

## Impact

- add(frontend): `PageListPage` — table (name, type, status, in-use badge), sortable headers,
  pagination, bulk actions; in-use pages show a protected/disabled delete.
- add(frontend): `PageFormDialog` — name, type select, body (textarea), active flag; inline 422 + success.

## Regressions

- None; additive.

## Acceptance Tests

### AC-1: FS-033.11/.12 — page list renders with sort/pagination + in-use badge. [BROWSER]
- Setup: reseed; seed several pages (`POST /api/dev/seed-page` x N).
- Navigate: http://localhost:3702/staff/login → log in `admin`/`Admin123!` → /staff/admin/pages.
- Verify: table (name, type, status, in-use badge column) with sortable headers (click a header → order changes) and pagination controls.
- Status: [x]

### AC-2: FS-033.13 — create landing + thank-you pages. [BROWSER]
- Setup: continue as admin at /staff/admin/pages.
- Action: Add Page → Name "Landing A", Type `landing`, Body "x", Save. Add Page → Name "Thanks A", Type `thank-you`, Body "y", Save.
- Verify: both appear in the list with the correct type.
- Status: [x]

### AC-3: BS-033.10 — duplicate name inline error. [BROWSER]
- Setup: continue as admin; "Landing A" exists (AC-2).
- Action: Add Page → Name "Landing A" (reused) → Save.
- Verify: inline "already exists" error; not created.
- Status: [x]

### AC-4: BS-033.9 — bound page shows in-use + guarded delete. [BROWSER]
- Setup: bind "Landing A" as default landing page via /staff/admin/settings → Pages tab (EPIC-M4-A).
- Action: /staff/admin/pages → attempt to delete "Landing A" (and to disable it).
- Verify: an in-use badge is shown and BOTH the delete and disable are blocked with a message.
- Status: [x]

## Test Infrastructure

- Vitest + RTL + MSW mocking `/api/staff/admin/pages*`. Admin account.

## Dependencies

- **TS-M4-A0**: shell. **TS-M4-F1**: page endpoints. **EPIC-M4-A**: binding (for the in-use AC).
