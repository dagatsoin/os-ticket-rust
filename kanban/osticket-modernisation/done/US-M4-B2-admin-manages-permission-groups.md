# US-M4-B2 — Admin manages permission groups

- **ID**: US-M4-B2
- **Type**: User Story
- **Parent**: EPIC-M4-B
- **Labels**: User Story, M4
- **Scope**: medium

## Spec References

- FS-031.6 (group management access gate)
- FS-031.7 (group list, sort & mass actions)
- FS-031.8 (create / edit a group)
- BS-031-019 (group identity & minimum name length)
- BS-031-020 (canonical 11 permission-flag set incl. net-new `can_manage_faq`, `can_manage_premade`, `can_ban_emails`, `can_view_staff_stats`)
- BS-031-021/022 (group department-access set is a full-replace sync)
- BS-031-023 (group self-action protection)

## Context

Epic: EPIC-M4-B — Staff, Groups & Permissions. Groups carry the permission flags and department
access shared by their members. This story delivers the group list and the create/edit form with
the full flag set and the department-access checkbox matrix.

## Description

As an administrator, I can list permission groups, and create or edit one: set its name and status,
toggle each of the eleven permission flags (Yes/No), and check the departments its members may
access. Saving fully reconciles the department-access set. The four net-new flags
(`can_manage_faq`, `can_manage_premade`, `can_ban_emails`, `can_view_staff_stats`) are present and
drive the delegated capabilities used elsewhere in M4.

## Impact

- Frontend (group list + create/edit form with flag radios + dept matrix + mass actions)
- Backend (group CRUD + flag set + dept-access sync)
- Database (`groups`, `group_dept_access`, `department`)
- Browser (desktop)

## Business Rules

- BS-031-019: group name required, >= 3 chars, unique.
- BS-031-020: the eleven flags apply to all members.
- BS-031-021/022: saving replaces the department-access set (insert checked, remove unchecked).
- BS-031-023: an admin cannot mass-act on their own group.

## Regressions

- The M1/M3 `agent` group keeps its existing flags; new flags default off and don't alter M3 behavior.
- Department-access changes propagate to member effective access (BS-031-005) used by FS-020/021.

## Acceptance Criteria

### AC-1: The group list renders with member/department counts and mass actions. [BROWSER]
- Setup: `POST /api/dev/reset-groups` (see Test Infrastructure — **flagged missing**); log in as `admin` / `Admin123!`.
- Navigate: http://localhost:3702/staff/admin/groups.
- Verify: a table of groups with columns name, status, member count, dept-access count; row checkboxes + mass-action controls (Enable/Disable/Delete).
- Verify: the seeded `Support` group appears with a member count > 0 (it owns `agent`/`admin`).
- Status: [x]

### AC-2: Admin creates a group with the full 11-flag set and a department matrix. [BROWSER]
- Setup: logged in as `admin` at /staff/admin/groups.
- Action: click "Add Group"; set name `Faq Managers`, status Active; toggle several permission flags including "Can Manage FAQ" = Yes; check the "Support" department; Save.
- Verify: success banner "Faq Managers added successfully"; the group appears in the list with dept-access count = 1 and member count 0.
- Status: [x]

### AC-3: The four net-new flags are present on the form. [BROWSER]
- Action: open the create form ("Add Group") or edit an existing group.
- Verify: Yes/No controls exist for "Can Manage FAQ", "Can Manage Premade", "Can Ban Emails", "Can View Staff Stats" — plus the 7 pre-existing flags (create/close/assign/transfer/delete/edit/ban tickets etc.), 11 total.
- Status: [x]

### AC-4: Editing the department matrix is a full-replace sync (BS-031-021/022). [BROWSER]
- Setup: at least two departments seeded (e.g. `Support` + a second via `POST /api/dev/seed-dept`); a group with `Support` checked.
- Action: edit that group; uncheck `Support`, check the second department; Save.
- Verify: the dept-access count stays consistent with the new set (still 1); reopening the form shows exactly the new set (old removed, new present).
- Status: [x]

### AC-5: Group deletion is refused while the group has members (BS-031-022). [BROWSER]
- Setup: log in as `admin`; the seeded `Support` group has members (`agent`).
- Navigate: /staff/admin/groups.
- Action: check the `Support` group row; choose "Delete"; confirm the dialog.
- Verify: deletion is refused with a visible error ("Unable to delete selected groups" / the group remains in the list unchanged). Then delete an empty group (member count 0) → it succeeds and disappears.
- Status: [x]

### AC-6: Self-group protection blocks mass-acting on your own group (BS-031-023). [BROWSER]
- Setup: log in as `admin` (a member of the seeded full-capability admin group).
- Action: check the admin's OWN group row; choose "Disable" (then repeat with "Delete"); confirm.
- Verify: each is blocked with the visible message "As an admin, you can't disable/delete a group you belong to - you might lockout all admins!"; the group is unchanged.
- Status: [x]

### AC-7: Group name validation. [API-ONLY]
- Setup: authenticate as `admin` (session cookie + CSRF).
- Request: `POST /api/staff/admin/groups` with a 2-char name → 422 "Group name must be at least 3 chars." (BS-031-019).
- Request: `POST /api/staff/admin/groups` duplicating an existing group name → 422 "Group name already exists".
- Status: [x]

### AC-8: Add Group with a too-short or duplicate name surfaces an inline field error. [BROWSER]
- Setup: logged in as `admin` / `Admin123!` at /staff/admin/groups.
- Action: click "Add Group"; enter a 2-char name (e.g. `Ab`); Save.
- Verify: an inline error renders under the Name field ("Group name must be at least 3 chars."); the
  dialog stays open (no navigation / no success banner).
- Action: change the name to duplicate an existing group (e.g. `Support`); Save.
- Verify: an inline error renders under the Name field ("Group name already exists"); no group added.
- Status: [x]

## Checklist (children)

- [ ] TS-M4-B3 — Backend: group CRUD + 11-flag set + dept-access sync
- [ ] TS-M4-B4 — Frontend: group list + create/edit form (flag radios + dept matrix)

## Test Infrastructure

- Admin account (`admin` / `Admin123!`) — confirmed seeded.
- `POST /api/dev/reset-groups` — **delivered by TS-M4-B3** (was missing from `backend/api/src/lib.rs`): deletes non-seed groups and **refuses/skips any group that still has members**, keeping the `Support`/admin groups, so the group-list/mass-action ACs start clean.
- `POST /api/dev/set-group-perm` — confirmed; can toggle 7 flags (does NOT yet cover the 4 net-new flags `can_manage_faq`/`can_manage_premade`/`can_ban_emails`/`can_view_staff_stats`). Net-new flag persistence is exercised through the group form/API under test (TS-M4-B3 AC-3), so no dev-endpoint change is required for coverage.
- `POST /api/dev/seed-dept` — confirmed; used for the second department in AC-4.

## Dependencies

- **EPIC-M4-PREP**: groups net-new flag columns.
- **EPIC-M4-A (shell)**: groups screen mounts in the admin shell.
- Produces the `can_manage_faq` / `can_manage_premade` groups consumed by EPIC-M4-E / EPIC-M4-H.
