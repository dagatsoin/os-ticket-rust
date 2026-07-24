# US-M4-B3 — Staff views/edits own profile and browses the directory

- **ID**: US-M4-B3
- **Type**: User Story
- **Parent**: EPIC-M4-B
- **Labels**: User Story, M4
- **Scope**: medium

## Spec References

- FS-031.9 (staff directory browse & search)
- FS-031.10 (own profile view & edit)
- FS-031.11 (forced-password-change and vacation notices on profile)
- FS-031.12 (periodic password-reset aging — forced change on login)
- FS-031.13 (own-profile password change section)
- BS-031-030 (directory search matching), BS-031-009 (directory visibility flag)
- BS-030-14 (team members are added from the staff profile)

## Context

Epic: EPIC-M4-B — Staff, Groups & Permissions. Two non-admin-gated staff screens: each agent edits
their own profile and password, and any staff member can browse the directory of directory-visible
colleagues. The profile also hosts the "add to team" control (BS-030-14), feeding EPIC-M4-C's roster
(Integration AC-3), and surfaces the password-aging notice (Integration AC-7).

## Description

As a staff member, I can open my own profile, edit my name/email/phone/preferences (timezone, page
size, refresh rate, signature) and change my password; if my password is due for rotation I see a
forced-change banner. I can also open a searchable directory of directory-visible staff and filter it
by department. From my (or another staff member's) profile an admin can add them to a team.

## Impact

- Frontend (own-profile form + password section + directory list/search)
- Backend (profile GET/PUT + password change + directory endpoint + add-to-team)
- Database (`staff`, `timezone`, `team_member`, `department`)
- Browser (desktop)

## Business Rules

- BS-031-030: numeric term → phone/ext/mobile; `@`-term → email exact; else email/last/first substring.
- BS-031-009: only directory-visible staff are listed.
- BS-031-013: password change (current-password path) clears forced-change and records the reset
  timestamp. (Reset-token path is DEFERRED — no reset-token store exists in M1.)
- BS-030-14: team membership is added from the staff profile, not the team form.

## Regressions

- Changing one's own timezone must not break the active session. (Timezone is live-derived from
  `staff.timezone_id` per request — no stored session offset — so the oracle is that the new timezone
  is shown after save + reload, not a session-offset refresh.)
- Username stays read-only on the profile.

## Acceptance Criteria

### AC-1: A staff member edits their own profile and it persists. [BROWSER]
- Setup: log in as `agent` / `Agent123!`. Navigate: http://localhost:3702/staff/profile.
- Verify: the username field is read-only (disabled); editable name/email/phone/preferences (timezone, default page size, refresh rate, signature) render.
- Action: change the timezone to a different zone and set default page size to a new value; Save.
- Verify: "Profile updated successfully" banner; reload the page and confirm both values persist
  (the profile shows the new timezone after save + reload — timezone is live-derived from
  `staff.timezone_id`, so no session-offset refresh is asserted).
- Verify (regression): the active session is not broken by the timezone change (page still authenticated; no forced re-login).
- Status: [x]

### AC-2: A staff member changes their own password. [BROWSER]
- Setup: logged in as `agent` at /staff/profile (password section).
- Action: enter current password `Agent123!`, new password `Agent999!` (>=6), confirm `Agent999!`; Save.
- Verify: success message; any forced-change banner clears.
- Action: log out; log in as `agent` / `Agent999!`.
- Verify: login succeeds. (Reset via seed/reset-staff afterwards to restore `Agent123!`.)
- Status: [x]

### AC-3: The directory lists directory-visible staff and supports search + dept filter. [BROWSER]
- Setup: logged in as `agent`. Navigate: http://localhost:3702/staff/directory.
- Verify: a read-only table with columns name, department, email, phone, ext, mobile; NO edit/add/delete controls.
- Action: type a colleague's name (e.g. part of `agent2`'s name) into the search box and click "Filter".
- Verify: the list narrows to the matching staff member.
- Action: change the department filter to a specific department.
- Verify: the list is scoped to that department's directory-visible staff.
- Status: [x]

### AC-4: A staff member is added to a team from their profile (feeds Teams roster). [BROWSER]
- **Sequenced-after-EPIC-M4-C** — the team roster screen belongs to M4-C. In B, **stub-verify** that
  the add-to-team control posts and the membership persists (re-open the profile → the team is listed).
  The **full** roster-screen verification (the M4-C team screen lists the member) is deferred to the
  **M4 Integration AC** once M4-C exists.
- Setup: a team is seeded (M4-C seed or M3 "Tier 2" team); log in as `admin` / `Admin123!`; open a staff member's profile via /staff/admin/staff → open the row.
- Action: use the "Teams" control on the profile to add that staff member to the seeded team; Save.
- Verify (B stub): success; re-opening the profile shows the team on the "Teams" control (membership persisted).
- Verify (full, at M4 Integration): the team's roster (EPIC-M4-C team screen) now lists that staff member.
- Status: [x]

### AC-5: Password-aging notice — an over-age password surfaces the forced-change banner. [BROWSER]
- Setup: `POST /api/dev/age-password` {staffId: <agent2's id>, days: 120} (backdates `passwdreset` and sets `change_passwd`); optionally `POST /api/dev/seed-config` {passwd_reset_period: 30} so the aging window is active (Integration AC-7).
- Action: log in as that aged non-admin account.
- Verify: the forced-password-change banner appears ("You must change your password to continue!") and the password-change form is presented.
- Status: [x]

### AC-6: Directory search matching rules. [API-ONLY]
- Setup: authenticate as `agent` (session cookie).
- Request: `GET /api/staff/directory?q=<numeric>` → matches on phone/ext/mobile.
- Request: `GET /api/staff/directory?q=<email>` → matches email exactly.
- Request: `GET /api/staff/directory?q=<name-substring>` → matches email OR lastname OR firstname (substring).
- Status: [x]

## Checklist (children)

- [ ] TS-M4-B5 — Backend: own-profile GET/PUT + password change + directory endpoint
- [ ] TS-M4-B6 — Frontend: own-profile + directory UI (+ add-to-team control)

## Test Infrastructure

- `agent` / `Agent123!` for the profile/directory ACs; `admin` / `Admin123!` for the add-to-team AC — both confirmed seeded.
- Dev endpoint **`POST /api/dev/age-password`** — confirmed; backdates `passwdreset` by N days AND sets `change_passwd = true`. Covers AC-5 and TS-M4-B6 AC-3 directly. **Note:** because it also sets `change_passwd` itself, it forces the banner regardless of `passwd_reset_period`; TS-M4-B5 AC-6 (which verifies the *login-time* aging computation) additionally needs `passwd_reset_period > 0` via `POST /api/dev/seed-config` and should assert the admin-exempt branch separately.
- **Gap flagged (non-blocking):** to restore `agent`'s password after AC-2 mutates it, use `POST /api/dev/reset-staff` (re-seeds documented logins) or re-run the seed — there is no dedicated password-reset dev endpoint.
- Requires a seeded team (M3 "Tier 2" or an M4-C team) for AC-4. Confirm a team exists in the seed before running AC-4; if the M4-C team seed is not yet present this AC is BLOCKED on that dependency, not on this ticket.

## Dependencies

- **EPIC-M4-PREP**: staff preference columns + timezone table.
- **EPIC-M4-A (shell)**: profile/directory reachable from the staff nav (non-admin gated).
- **EPIC-M4-C**: team roster verification for AC-4 (Integration AC-3).
