# TS-M4-B2 — add(frontend): FS-031.2/.3/.4 staff list + create/edit form + mass actions UI

- **ID**: TS-M4-B2
- **Type**: Technical Story
- **Parent**: US-M4-B1
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

The staff management screen inside the admin shell, wired to TS-M4-B1's endpoints.

## Impact

- add(frontend): `StaffListPage` — table (name, username, status, group, dept), search/filter box,
  sortable headers, pagination, row checkboxes + mass-action bar (Enable/Lock/Delete with confirm).
- add(frontend): `StaffFormDialog` — create/edit fields (identity, group select, primary dept select,
  admin flag, status, directory-visible, vacation, signature, preferences, temp/new password), inline
  422 field errors, success banner.

## Regressions

- Reuses the shared apiClient + error-envelope parsing; no change to `/staff/tickets`.

## Acceptance Tests

### AC-1: FS-031.2 — list renders with filter/sort/pagination. [BROWSER]
- Setup: `POST /api/dev/reset-staff` + seed ~12 accounts via `POST /api/dev/seed-staff`; log in as `admin` / `Admin123!`.
- Navigate: http://localhost:3702/staff/admin/staff.
- Verify: table (name, username, status, group, dept) + filter/search box + sortable headers + pagination controls.
- Action: filter by `bulk_1`; verify the list narrows. Click the Name header; verify re-sort.
- Status: [x]

### AC-2: FS-031.3 — create form submits and shows the new row. [BROWSER]
- Setup: logged in as `admin` at /staff/admin/staff.
- Action: click "Add Staff"; fill firstname/lastname/username `ncreate`/email, pick group `Support` + primary dept `Support`, set temp password + confirm; Save.
- Verify: success banner "<name> added successfully"; the `ncreate` row appears in the list.
- Status: [x]

### AC-3: FS-031.4 — mass lock with confirm + self-action block surfaced. [BROWSER]
- Setup: logged in as `admin`; `agent` + `agent2` active.
- Action: check the `agent` and `agent2` rows → Lock → confirm dialog → both show Locked.
- Action: check your OWN (`admin`) row → Lock.
- Verify: blocked with the self-action message "You can not disable/delete yourself - you could be the only admin!".
- Status: [x]

### AC-4: BS-031-014 — last-admin error surfaced inline. [BROWSER]
- Setup: `POST /api/dev/reset-staff` (sole active admin `admin`); log in as `admin`.
- Action: open the `admin` account; clear the "Administrator" flag; Save.
- Verify: inline error on the admin field "Cowardly refusing to remove or lock out the only active administrator"; account stays admin. (Also try Locked → same inline error.)
- Status: [x]

### AC-5: BS-031-001 — duplicate-username 422 surfaces as an inline field error. [BROWSER]
- Setup: log in as `admin`; the `agent` account exists.
- Action: "Add Staff"; use the existing username `agent` (valid password + confirm); Save.
- Verify: inline field error under Username ("Username already exists"); dialog stays open, no navigation, no new row.
- Status: [x]

## Test Infrastructure

- Vitest + RTL + MSW mocking `/api/staff/admin/staff*`. Admin account for browser ACs.

## Dependencies

- **TS-M4-A0**: admin shell. **TS-M4-B1**: staff endpoints.
