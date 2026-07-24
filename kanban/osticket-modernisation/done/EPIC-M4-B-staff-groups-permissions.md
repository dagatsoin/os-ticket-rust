# EPIC-M4-B — EPIC – Staff, Groups & Permissions

- **ID**: EPIC-M4-B
- **Type**: Epic
- **Parent**: M4
- **Labels**: Epic, M4
- **Column**: derived from children

## Spec References

- FS-031.1–.5 (staff management gate, list/filter/sort/paginate, create/edit, mass actions, validation)
- FS-031.6–.8 (group management gate, list/mass actions, create/edit with flag set + dept access)
- FS-031.9 (staff directory browse & search)
- FS-031.10–.13 (own profile view/edit, forced-change & vacation notices, password aging, password change)
- BS-031-020 (canonical 11-flag permission set incl. net-new `can_manage_faq`, `can_manage_premade`, `can_ban_emails`, `can_view_staff_stats`)
- BS-031-014 (last-administrator protection), BS-031-015 (self-action protection)
- BS-030-14 (team members added from the staff profile, not the team form)

## Context

Milestone: M4 — Admin Configuration. Authorization backbone. M1 seeded a single group + staff row;
this epic makes staff and groups fully CRUD-able and exposes the directory and own-profile screens.
Its outputs feed EPIC-M4-C (managers, team leads, topic auto-assignees, dept-access groups) and
EPIC-M4-E (the `can_manage_faq` gate) — see Integration ACs 2, 3, 6.

## Description

- **Staff**: list with filter/sort/pagination; create/edit (identity, group, primary dept, admin flag,
  status, directory-visible, vacation, signature, prefs); mass enable/lock/delete; last-admin &
  self-action protections; add-to-team from the profile (BS-030-14).
- **Groups**: list + mass actions; create/edit form with the full 11-flag radio set and the
  department-access checkbox matrix (full-replace sync).
- **Directory**: read-only searchable/filterable directory of directory-visible staff.
- **Own profile**: each staff member edits their own record + password; forced-change/vacation
  notices; password aging drives forced change at login (Integration AC-7). KL-031-002 modernised.

## Business value

Operators provision agents, shape authorization via groups, and each agent self-serves profile and
password. This removes the seed-only account model and is the prerequisite for delegated,
non-admin capabilities (FAQ, canned responses).

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by leaf tickets. Column derived from children.

## Children (column derived: min of these)

- [ ] [US-M4-B1](US-M4-B1-admin-manages-staff-accounts.md) — Admin manages staff accounts
  - [ ] [TS-M4-B1](TS-M4-B1-staff-crud-endpoints.md) — Backend: staff CRUD + validation + last-admin/self protection + add-to-team
  - [ ] [TS-M4-B2](TS-M4-B2-staff-list-form-ui.md) — Frontend: staff list + create/edit form + mass actions UI
- [ ] [US-M4-B2](US-M4-B2-admin-manages-permission-groups.md) — Admin manages permission groups
  - [ ] [TS-M4-B3](TS-M4-B3-group-crud-endpoints.md) — Backend: group CRUD + 11-flag set + dept-access sync
  - [ ] [TS-M4-B4](TS-M4-B4-group-list-form-ui.md) — Frontend: group list + create/edit form (flag radios + dept matrix)
- [ ] [US-M4-B3](US-M4-B3-staff-profile-and-directory.md) — Staff views/edits own profile and browses the directory
  - [ ] [TS-M4-B5](TS-M4-B5-profile-directory-endpoints.md) — Backend: own-profile GET/PUT + password change + directory endpoint
  - [ ] [TS-M4-B6](TS-M4-B6-profile-directory-ui.md) — Frontend: own-profile + directory UI (+ add-to-team control)

## Consolidation notes (cross-cutting ownership)

- **TS-M4-B1** owns: additive `staff.mobile` column (PREP missed it), app-level case-insensitive
  `email` uniqueness (excluding self) → 422, and the queue `page_limit`→`max_page_size` read fix.
- **TS-M4-B3** owns: `POST /api/dev/reset-groups` (delete non-seed groups, refuse/skip groups with
  members) and the extended `seed-staff` (`groupId`/`deptId`/`isvisible`/`isadmin`/`onvacation`/`count`,
  or `seed-staff-bulk`) used by the list-pagination/filter and directory-visibility ACs.
- **TS-M4-B5** owns: live-derived timezone (from `staff.timezone_id`, no stored offset) and the
  `age-password` `set_change_passwd` flag. Password-change is scoped to the **current-password path**;
  the reset-token path is **DEFERRED** (no reset-token store exists in M1).
- **TS-M4-B6** owns the non-admin shell plumbing (directory/profile are NOT admin-gated):
  a `RequireStaff` guard, a staff user menu (Profile / Directory / Logout), and the
  forced-password-change-at-login banner.
- **TS-M4-B4** builds the reusable **`PermissionGrid`** (full 11-flag BS-031-020 set, 7 existing + 4 new)
  and **`DeptAccessMatrix`** components — reused by EPIC-M4-C.
- **Cross-epic sequencing:** US-M4-B3 AC-4 / TS-M4-B6 AC-5 (add-to-team verified against the roster)
  are **sequenced-after-EPIC-M4-C** — stub-verified in B (membership persists), full roster
  verification at the M4 Integration AC.

## Dependencies

- **EPIC-M4-PREP**: staff/groups additive columns, timezone table.
- **EPIC-M4-A (shell)**: admin screens mount inside TS-M4-A0.
- Blocks: EPIC-M4-C (managers/leads/auto-assign/dept-access groups), EPIC-M4-E (`can_manage_faq` gate).
