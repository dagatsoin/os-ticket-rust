# TS-M4-B6 — add(frontend): FS-031.9/.10/.11 own-profile + directory UI (+ add-to-team)

- **ID**: TS-M4-B6
- **Type**: Technical Story
- **Parent**: US-M4-B3
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

The own-profile and directory screens, plus the "add to team" control on a staff profile (BS-030-14),
wired to TS-M4-B5 (profile/directory) and TS-M4-B1 (team membership). **Owns the non-admin shell
plumbing** the directory/profile flows need (a staff guard, a staff user menu with logout, and the
forced-password-change banner) — these are NOT admin-gated, so TS-M4-A0's admin shell does not cover them.

## Impact

- add(frontend): **`RequireStaff` route guard** — authenticated staff of ANY role (not admin-gated),
  wrapping `/staff/directory` and `/staff/profile`. (TS-M4-A0's guard is admin-only; these two screens
  must be reachable by non-admin agents.)
- add(frontend): **staff user menu in `AppShell`** exposing **Profile / Directory / Logout** entries.
  Required by US-M4-B1 AC-2 and US-M4-B3 AC-2 (both need Logout) and US-M4-B3 AC-1 (needs a Profile nav
  entry). Logout clears the session and returns to login.
- add(frontend): **forced-password-change-at-login banner** — `AuthStore` exposes the aged/forced flag
  (from the session bootstrap `change_passwd`), and a global banner renders
  "You must change your password to continue!" with the password-change form presented (FS-031.11).
- add(frontend): `ProfilePage` — read-only username; editable name/email/phone/mobile/preferences
  (timezone, page size, refresh rate, signature); a password-change section (current + new + confirm);
  the forced-change / vacation notice banners (FS-031.11).
- add(frontend): `DirectoryPage` — read-only table (name, dept, email, phone, ext, mobile), a search box
  (submit "Filter"), a department filter, sortable headers, pagination.
- add(frontend): a "Teams" control on the profile (admin editing another staff) that adds/removes team
  membership (posts to TS-M4-B1 endpoints).

## Regressions

- Additive screens; reuses shared apiClient. **AppShell change:** adding the staff user menu
  (Profile / Directory / Logout) touches the shared shell — verify the existing admin nav and the
  `/staff/tickets` layout are unaffected. The new `RequireStaff` guard must not tighten access to
  any already-reachable route.

## Acceptance Tests

### AC-1: FS-031.10 — profile edits persist; username read-only. [BROWSER]
- Setup: log in as `agent` / `Agent123!`. Navigate: http://localhost:3702/staff/profile.
- Verify: username field is read-only (disabled).
- Action: change timezone + default page size; Save.
- Verify: "Profile updated successfully"; reload → values persist; session not broken by the timezone change.
- Status: [x]

### AC-2: FS-031.13 — password change works end to end. [BROWSER]
- Setup: logged in as `agent` at /staff/profile.
- Action: password section → current `Agent123!`, new `Agent999!`, confirm; Save. Log out; log in as `agent` / `Agent999!`.
- Verify: login succeeds. (Restore via `POST /api/dev/reset-staff` afterwards.)
- Status: [x]

### AC-3: FS-031.11 — forced-change banner shown when flagged. [BROWSER]
- Setup: `POST /api/dev/age-password` {staffId: <agent2 id>, days: 120}.
- Action: log in as that aged non-admin account.
- Verify: the forced-password-change banner renders ("You must change your password to continue!").
- Status: [x]

### AC-4: FS-031.9 — directory search + dept filter. [BROWSER]
- Setup: logged in as `agent`. Navigate: http://localhost:3702/staff/directory.
- Verify: read-only table (name, dept, email, phone, ext, mobile); no edit controls.
- Action: filter by a colleague's name → verify the match; change the department filter → verify scoping.
- Status: [x]

### AC-5: BS-030-14 — add-to-team from profile reflects on the team roster. [BROWSER]
- **Sequenced-after-EPIC-M4-C** (roster screen belongs to M4-C). Stub-verify in B (the "Teams" control
  posts and the membership persists via `GET /api/staff/admin/staff/:id/teams`); full roster-screen
  verification happens at the M4 Integration AC once M4-C's team screen exists.
- Setup: a seeded team exists; log in as `admin` / `Admin123!`; open a staff profile via /staff/admin/staff.
- Action: use the "Teams" control to add that staff member to the seeded team; Save.
- Verify (B stub): the membership persists (re-open the profile → the team is listed on the "Teams" control).
- Verify (full, at M4 Integration): the team roster (EPIC-M4-C team screen) lists that staff member.
- Status: [x]

## Test Infrastructure

- Vitest + RTL + MSW mocking profile/directory/team endpoints. `agent` + `admin` accounts.

## Dependencies

- **TS-M4-A0**: base shell/nav (this TS adds the non-admin `RequireStaff` guard + staff user menu on top).
- **TS-M4-B5**: profile/directory endpoints. **TS-M4-B1**: team-membership endpoints.
- **EPIC-M4-C**: team roster for AC-5 full verification (Integration AC-3) — AC-5 stub-verifies in B,
  full-verifies at the M4 Integration AC.
