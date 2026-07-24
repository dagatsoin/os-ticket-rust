# US-M4-C2 — Admin manages teams

- **ID**: US-M4-C2
- **Type**: User Story
- **Parent**: EPIC-M4-C
- **Labels**: User Story, M4
- **Scope**: small

## Spec References

- FS-030.8 (team list view), FS-030.9 (add/edit form), FS-030.10 (create/update & member removal)
- FS-030.11 (deletion & ticket release), FS-030.12 (bulk actions)
- BS-030-12 (name required/min/unique), BS-030-13 (lead must be a member)
- BS-030-14 (members added from staff profiles, not the team form)
- BS-030-15 (disabled teams unavailable), BS-030-16 (deletion releases associations)
- KL-030-12 ("Last Updated" sort — modernised)

## Context

Epic: EPIC-M4-C — Departments, Teams & Help Topics. Teams group agents for assignment. This story
delivers the team list and add/edit form; members are added from staff profiles (EPIC-M4-B), so the
team form only removes members and sets the lead.

## Description

As an administrator, I can list teams and create or edit one: name, status, assignment-alerts
override, and the team lead (chosen from current members). The team form lets me remove members but
not add them (that's done from the staff profile). Deleting a team releases its ticket assignments.

## Impact

- Frontend (team list + add/edit form + bulk actions)
- Backend (team CRUD + lead/member-removal + delete release)
- Database (`team`, `team_member`, `staff`, `ticket`)
- Browser (desktop)

## Business Rules

- BS-030-12: name required, >= 3 chars, unique.
- BS-030-13: lead chosen from current members; removing the lead resets it.
- BS-030-14: adding members is done on the staff profile, not here.
- BS-030-16: deletion removes member rows and clears team assignment on owned tickets AND on any help topic auto-assigned to the team (help_topic.team_id is a RESTRICT FK — cleared to avoid a delete FK-violation).

## Regressions

- The M3 seeded "Tier 2" team keeps working; assignment/visibility unaffected.

## Acceptance Criteria

### AC-1: The team list renders with member counts. [BROWSER]
- Setup: `cargo run -p tools --bin seed` then `POST /api/dev/reset-teams` for clean state (keeps the seeded "Tier 2").
- Setup: log in as admin (`admin` / `Admin123!`) at http://localhost:3702/staff/login.
- Navigate: http://localhost:3702/staff/admin/teams
- Verify: a table of teams shows name, status, and member count; the seeded "Tier 2" team appears.
- Status: [x]

### AC-2: Admin creates a team and sets a lead from its members. [BROWSER]
- Setup: admin on /staff/admin/teams (from AC-1).
- Action: click "Add Team"; type "QA Squad" (>=3 chars); set status Active; click Save.
- Verify: success banner; "QA Squad" appears in the list.
- Action: add a member to "QA Squad" from a staff profile (open /staff/admin/staff → a staff record → add-to-team, per EPIC-M4-B); then edit "QA Squad"; select that member as Lead; click Save.
- Verify: success banner; the roster shows the member and the lead badge on them (BS-030-13).
- Status: [x]

### AC-3: The team form removes a member (does not add). [BROWSER]
- Setup: admin editing a team with >=1 member (e.g. "QA Squad" from AC-2, or "Tier 2").
- Action: mark a member for removal via its remove checkbox; click Save.
- Verify: the roster no longer lists that member; confirm there is NO add-member control anywhere on the team form (adding is only done from the staff profile — BS-030-14).
- Status: [x]

### AC-4: Deleting a team releases its ticket assignments. [BROWSER]
- Setup: a team assigned to a ticket — create "Temp Team" (AC-2 flow), then assign it to a seeded ticket (`POST /api/dev/seed-ticket` + assign in the queue, or seed with team assignment).
- Action: on /staff/admin/teams, delete "Temp Team".
- Verify: success; the ticket's team assignment is now cleared (confirm in the ticket detail) and the team's member rows are gone (BS-030-16).
- Status: [x]

### AC-5: Name validation. [API-ONLY]
- Setup: admin session cookie (`POST /api/staff/login` as `admin`/`Admin123!`).
- Request: `POST /api/staff/admin/teams` with a 2-char name → 422 "Team name must be at least 3 chars." (BS-030-12).
- Request: `POST /api/staff/admin/teams` with an existing team name (e.g. "Tier 2") → 422 "Team name already exists".
- Status: [x]

## Checklist (children)

- [ ] TS-M4-C3 — Backend: team CRUD + lead/member-removal + delete release
- [ ] TS-M4-C4 — Frontend: team list + add/edit form UI

## Test Infrastructure

- Admin account (`admin`/`Admin123!`). Members added via the EPIC-M4-B add-to-team control on the staff profile (Integration AC-3).
- Dev **`POST /api/dev/reset-teams`** — declared as a deliverable of TS-M4-C3 (`add(dev)`); provided by the dev iteration, no consolidation gap.

## Dependencies

- **EPIC-M4-PREP**, **EPIC-M4-A (shell)**, **EPIC-M4-B** (staff for members/lead — Integration AC-3).
