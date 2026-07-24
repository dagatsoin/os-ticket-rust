# TS-M4-B3 — add(route): FS-031.7/.8 group CRUD + 11-flag set + dept-access sync

- **ID**: TS-M4-B3
- **Type**: Technical Story
- **Parent**: US-M4-B2
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

Backend for permission-group management: list + mass actions, create/edit with the canonical
eleven-flag set (BS-031-020) including the four net-new flags, and the department-access full-replace
sync (BS-031-021/022).

## Impact

- add(route): `GET /api/staff/admin/groups` (list with member/dept counts, sort, mass actions) — admin-gated.
- add(route): `POST` / `PUT /api/staff/admin/groups/:id` — name validation (BS-031-019), the eleven
  flags, and department-access reconcile (insert checked, delete unchecked).
- add(route): `POST /api/staff/admin/groups/mass` (enable/disable/delete) — BS-031-023 self-group guard.
- The four net-new flags (`can_manage_faq`, `can_manage_premade`, `can_ban_emails`,
  `can_view_staff_stats`) are read/written and exposed on the session bootstrap for gating.
- add(dev): **build + register `POST /api/dev/reset-groups`** — deletes non-seed groups, and
  **refuses/skips any group that still has members** (keep the `Support` + admin groups); register
  it in `backend/api/src/lib.rs` alongside `reset-staff`/`reset-tickets`/`reset-config`.
- add(dev): **extend `seed-staff`** with optional `groupId`, `deptId`, `isvisible`, `isadmin`,
  `onvacation`, and `count` fields (**OR** add a `POST /api/dev/seed-staff-bulk`) so the staff-list
  pagination/filter ACs (US-M4-B1 AC-1 / TS-M4-B2 AC-1) can seed ~12 rows in one call and so
  TS-M4-B5 AC-5 can seed an `isvisible=false` row directly. This closes the gaps flagged in
  US-M4-B1 / TS-M4-B5 Test Infra (noisy per-row loop; `seed-staff` can't set `isvisible`).

## Regressions

- The `agent` group's existing flags are preserved; new flags default off.
- Department-access changes flow into member effective access (BS-031-005) — no change to that rule.

## Acceptance Tests

### AC-1: FS-031.7 — list returns member/dept counts (admin only). [API-ONLY]
- Setup: authenticate as `admin` / `Admin123!`.
- Request: `GET /api/staff/admin/groups` → 200; each group carries a distinct member count and dept-access count.
- Request: same as `agent` (non-admin) → 403.
- Status: [x]

### AC-2: BS-031-019 — name validation. [API-ONLY]
- Setup: authenticate as `admin`.
- Request: `POST /api/staff/admin/groups` with a 2-char name → 422 "Group name must be at least 3 chars.".
- Request: `POST …/groups` duplicating an existing name → 422 "Group name already exists".
- Status: [x]

### AC-3: BS-031-020 — all eleven flags persist, including the four net-new. [API-ONLY]
- Setup: authenticate as `admin`.
- Request: `POST /api/staff/admin/groups` with `can_manage_faq=1, can_manage_premade=1, can_ban_emails=1, can_view_staff_stats=1` (plus the 7 pre-existing flags set) → 200.
- Request: `GET /api/staff/admin/groups/:id` → all four net-new flags true alongside the seven pre-existing; verify the four are exposed on the session bootstrap for gating.
- Status: [x]

### AC-4: BS-031-021/022 — dept-access full-replace sync. [API-ONLY]
- Setup: authenticate as `admin`; ≥2 departments exist (`POST /api/dev/seed-dept` for a second).
- Request: `PUT /api/staff/admin/groups/:id` with a new dept-access set → 200.
- Request: `GET …/groups/:id` → shows exactly the submitted set (previously-checked removed, newly-checked inserted).
- Status: [x]

### AC-5: BS-031-022 — group with members cannot be deleted. [API-ONLY]
- Setup: authenticate as `admin`; the `Support` group has members (`agent`).
- Request: `POST /api/staff/admin/groups/mass` {action: delete, ids:[<Support group id>]} → refused (group remains; "Unable to delete selected groups" / partial result). Deleting an empty group succeeds and also removes its dept-access rows.
- Status: [x]

### AC-6: BS-031-023 — cannot mass-act on own group. [API-ONLY]
- Setup: authenticate as `admin` (member of the admin group).
- Request: `POST /api/staff/admin/groups/mass` {action: disable, ids:[<admin's own group id>, …]} → rejected with "As an admin, you can't disable/delete a group you belong to - you might lockout all admins!". Repeat with action delete → same block.
- Status: [x]

## Test Infrastructure

- Admin account (`admin` / `Admin123!`) — confirmed seeded.
- **MUST ADD `POST /api/dev/reset-groups`** — this dev endpoint does not yet exist (verified against `backend/api/src/lib.rs`: only `reset-staff`/`reset-tickets`/`reset-config` are registered). This TS delivers it: **delete non-seed groups, and refuse/skip any group that still has members** (keep the `Support` + admin groups) so the group list/mass-action E2E starts clean. Register it in `lib.rs` alongside the other dev routes.
- **MUST EXTEND `seed-staff` (or add `seed-staff-bulk`)** — owned here (see Impact): optional
  `groupId`/`deptId`/`isvisible`/`isadmin`/`onvacation`/`count`. Unblocks US-M4-B1 AC-1 pagination
  (one bulk call instead of a ~12× loop) and TS-M4-B5 AC-5 (`isvisible=false` seed). Update
  `SeedStaffRequest` accordingly.
- `.sqlx` cache updated.

## Dependencies

- **EPIC-M4-PREP**: groups net-new flag columns, group_dept_access.
- **TS-M4-A0**: admin gate + session bootstrap flag exposure.
