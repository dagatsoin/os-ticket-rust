# TS-M4-B4 — add(frontend): FS-031.7/.8 group list + create/edit form (flag radios + dept matrix)

- **ID**: TS-M4-B4
- **Type**: Technical Story
- **Parent**: US-M4-B2
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

The permission-group screen inside the admin shell, wired to TS-M4-B3.

## Impact

- add(frontend): `GroupListPage` — table (name, status, member count, dept-access count) + mass actions.
- add(frontend): `GroupFormDialog` — name + status; the eleven Yes/No flag radio pairs with hint text
  (incl. the four net-new flags); a department checkbox matrix with Select All / Select None; admin notes.
- add(frontend, **reusable**): **`PermissionGrid`** — renders the **full BS-031-020 11-flag set** as
  Yes/No pairs (the 7 pre-existing flags + the 4 net-new: `can_manage_faq`, `can_manage_premade`,
  `can_ban_emails`, `can_view_staff_stats` — **all 11, not just the 4 new ones**; per BS-031-020,
  spec lines 254-269). Built as a standalone reusable component — **reused by EPIC-M4-C**.
- add(frontend, **reusable**): **`DeptAccessMatrix`** — the department checkbox matrix (Select All /
  Select None, full-replace semantics). Also a standalone reusable component **reused by EPIC-M4-C**.
- Inline 422 errors + success banner.

## Regressions

- None; additive screen.

## Acceptance Tests

### AC-1: FS-031.7 — group list renders counts + mass actions. [BROWSER]
- Setup: log in as `admin` / `Admin123!`. Navigate: http://localhost:3702/staff/admin/groups.
- Verify: table with name, status, member count, dept-access count; row checkboxes + mass-action controls (Enable/Disable/Delete).
- Status: [x]

### AC-2: FS-031.8 — the eleven flags (incl. four net-new) render as Yes/No pairs. [BROWSER]
- Action: click "Add Group".
- Verify: Yes/No radio pairs exist for "Can Manage FAQ", "Can Manage Premade", "Can Ban Emails", "Can View Staff Stats" plus the 7 pre-existing flags (11 total), each with hint text.
- Status: [x]

### AC-3: FS-031.8 — create with a dept-matrix selection persists. [BROWSER]
- Action: set name `Faq Managers`, status Active, toggle "Can Manage FAQ" = Yes; check the "Support" department; Save.
- Verify: success banner; the group appears with dept-access count = 1.
- Status: [x]

### AC-4: Full-replace matrix edit reflects on reopen. [BROWSER]
- Setup: ≥2 departments seeded (`POST /api/dev/seed-dept`).
- Action: edit a group; uncheck the currently-checked department, check a different one; Save; reopen the form.
- Verify: the form shows exactly the new department set (old unchecked, new checked).
- Status: [x]

### AC-5: BS-031-022/023 — delete-with-members and self-group blocks surface visibly. [BROWSER]
- Action: check the `Support` group (has members) → Delete → confirm → visible refusal ("Unable to delete selected groups"); group remains.
- Action: check the admin's OWN group → Disable → visible block "As an admin, you can't disable/delete a group you belong to - you might lockout all admins!".
- Status: [x]

### AC-6: BS-031-019 — too-short/duplicate group name surfaces an inline field error. [BROWSER]
- Action: "Add Group"; enter a 2-char name; Save → inline error under Name ("Group name must be at least 3 chars."); dialog stays open.
- Action: change to a name duplicating an existing group (e.g. `Support`); Save → inline error under Name ("Group name already exists"); no group added.
- Status: [x]

## Test Infrastructure

- Vitest + RTL + MSW mocking `/api/staff/admin/groups*`. Admin account for browser ACs.

## Dependencies

- **TS-M4-A0**: admin shell. **TS-M4-B3**: group endpoints.
