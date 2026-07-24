# US-M4-B1 — Admin manages staff accounts

- **ID**: US-M4-B1
- **Type**: User Story
- **Parent**: EPIC-M4-B
- **Labels**: User Story, M4
- **Scope**: medium

## Spec References

- FS-031.1 (staff management access gate)
- FS-031.2 (staff list, filter, sort & paginate)
- FS-031.3 (create / edit a staff account)
- FS-031.4 (staff mass actions — enable / lock / delete)
- FS-031.5 (staff field validation)
- BS-031-001 (username+email unique), BS-031-013 (password rules)
- BS-031-014 (last-administrator protection), BS-031-015 (self-action protection)
- BS-031-016 (staff deletion side effects)

## Context

Epic: EPIC-M4-B — Staff, Groups & Permissions. M1 seeded a single staff row; this story makes staff
fully manageable from a screen: a filterable/sortable/paginated list, a create/edit form, and mass
enable/lock/delete — with the last-admin and self-action safeguards.

## Description

As an administrator, I can see a paginated list of staff, filter and sort it, and open a form to
create or edit a staff account (name, username, email, group, primary department, admin flag,
status, directory-visible, vacation, signature, preferences, temp/new password). I can select rows
and enable, lock, or delete them in bulk. The system stops me from removing or locking the last
active administrator, and from acting on my own account in a mass action.

## Impact

- Frontend (staff list + filters + create/edit form + mass-action bar)
- Backend (staff CRUD + validation + protections)
- Database (`staff`, `groups`, `department`, `team_member`)
- Browser (desktop)

## Business Rules

- BS-031-001: username and email are each unique.
- BS-031-013: temp password required on create; new password >= 6 chars; confirmation must match.
- BS-031-014: the save cannot remove/lock the only active administrator → error on `isadmin`.
- BS-031-015: an admin cannot include their own account in a mass action.
- BS-031-016: deleting staff reassigns/clears their owned associations.

## Regressions

- The M1/M3 `agent` login must keep working (its group/dept/flags unchanged by the new form).
- The effective-department-access rule (BS-031-005) still governs ticket visibility.

## Acceptance Criteria

### AC-1: The staff list renders with filter, sort and pagination. [BROWSER]
- Setup: `POST /api/dev/reset-staff` (roster → `agent`, `agent2`, `admin`); then seed ~12 extra accounts in one call via the extended `POST /api/dev/seed-staff` {username `bulk_`, count: 12, dept_id: Support} (or `seed-staff-bulk`), both delivered by TS-M4-B3, so pagination has enough rows.
- Navigate: http://localhost:3702/staff/admin/staff (log in first as `admin` / `Admin123!`).
- Verify: a table of staff with columns (name, username, status, group, dept); a filter/search box; sortable column headers; pagination controls (page N of M / next-prev).
- Action: type `bulk_1` in the filter box and submit.
- Verify: the list narrows to matching rows only.
- Action: click the "Name" column header.
- Verify: row order changes (sort applied).
- Status: [x]

### AC-2: Admin creates a new staff account that can then log in. [BROWSER]
- Setup: logged in as `admin` / `Admin123!` at /staff/admin/staff.
- Action: click "Add Staff"; type firstname `Nadia`, lastname `Ncreate`, username `ncreate`, email `ncreate@osticket.local`; pick group `Support` and primary dept `Support`; set temp password `Temp123456!` (+ confirm); Save.
- Verify: success banner "Ncreate added successfully" (or "<name> added successfully"); the `ncreate` row appears in the list.
- Action: log out; log in as `ncreate` / `Temp123456!`.
- Verify: login succeeds (a forced-change prompt may appear — expected, FS-031.11).
- Status: [x]

### AC-3: Admin edits a staff account (e.g., changes group/department). [BROWSER]
- Setup: logged in as `admin`; the `agent` row exists.
- Navigate: /staff/admin/staff; open the `agent` account.
- Action: change its primary department (e.g. to another seeded dept) or its group; Save.
- Verify: success banner "Profile updated" / "Account updated"; the changed dept/group reflects in the list row.
- Status: [x]

### AC-4: Last-administrator protection blocks demoting/locking the only admin (BS-031-014). [BROWSER]
- Setup: `POST /api/dev/reset-staff` so exactly one active admin (`admin`) exists; log in as `admin`.
- Navigate: /staff/admin/staff; open the `admin` account.
- Action: clear the "Administrator" flag; Save.
- Verify: rejected inline on the admin field with "Cowardly refusing to remove or lock out the only active administrator"; the account stays admin.
- Action: instead set the account status to Locked; Save.
- Verify: same rejection; the account stays Active.
- Status: [x]

### AC-5: Mass lock/enable works; self lock/delete is blocked (BS-031-015). [BROWSER]
- Setup: log in as `admin`; `agent` and `agent2` are non-admin active accounts.
- Action: check the `agent` and `agent2` rows; choose "Lock"; confirm the dialog.
- Verify: both rows show Locked. Then check the `agent` rows again and choose "Enable"; confirm.
- Verify: both return to Active.
- Action: check your OWN (`admin`) row and choose "Lock" (then repeat with "Delete").
- Verify: each is blocked with the self-action message "You can not disable/delete yourself - you could be the only admin!". (Because the sole admin is the acting admin, this is also what prevents deleting the last admin — the last-admin count check itself is edit-form-only, KL-031-005.)
- Status: [x]

### AC-6: Create validation — duplicate username and password rules. [API-ONLY]
- Setup: `POST /api/dev/reset-staff`; authenticate as `admin` (session cookie + CSRF).
- Request: `POST /api/staff/admin/staff` with username `agent` (existing) → 422 field error on `username` (unique).
- Request: `POST /api/staff/admin/staff` with an email matching an existing staff email in a different case → 422 field error on `email` (case-insensitive unique, BS-031-001).
- Request: `POST /api/staff/admin/staff` with a 4-char password → 422 "Must be at least 6 characters".
- Request: `POST /api/staff/admin/staff` with password/confirm mismatch → 422 confirmation error.
- Status: [x]

### AC-7: Add Staff with an existing username surfaces an inline field error (no navigation). [BROWSER]
- Setup: logged in as `admin` / `Admin123!` at /staff/admin/staff; the `agent` account exists.
- Action: click "Add Staff"; fill the form using the existing username `agent` (valid password + confirm); Save.
- Verify: an inline field error renders under the Username field (e.g. "Username already exists"); the
  dialog stays open (no navigation / no success banner) and no new row is added to the list.
- Status: [x]

## Checklist (children)

- [ ] TS-M4-B1 — Backend: staff CRUD + validation + last-admin/self protection + add-to-team
- [ ] TS-M4-B2 — Frontend: staff list + create/edit form + mass actions UI

## Test Infrastructure

- Seeded admin login **`admin` / `Admin123!`** (isadmin) — created by the M4-PREP seed. Confirmed present (`tools/src/lib.rs`).
- Dev endpoint **`POST /api/dev/reset-staff`** — confirmed; restores roster to `agent`/`agent2`/`admin` (nullifies staff-referencing FKs first). This leaves exactly one active admin, which AC-4 relies on.
- Dev endpoint **`POST /api/dev/seed-staff`** — extended by **TS-M4-B3** with `count`/`groupId`/`deptId`/`isvisible`/`isadmin`/`onvacation` (or a `seed-staff-bulk`). AC-1 now bulk-fills the list with a single call (gap closed — no ~12× loop).
- **Backend fixes owned by TS-M4-B1** (relevant to this story): additive `staff.mobile` column (used by the create/edit form + directory), app-level case-insensitive `email` uniqueness (AC-6/AC-7), and the queue `page_limit`→`max_page_size` read fix. See TS-M4-B1 Impact.

## Dependencies

- **EPIC-M4-PREP**: staff additive columns, groups flags, timezone.
- **EPIC-M4-A (shell)**: staff screen mounts in the admin shell.
