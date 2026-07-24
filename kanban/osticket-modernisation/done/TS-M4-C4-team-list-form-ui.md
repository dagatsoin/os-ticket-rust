# TS-M4-C4 — add(frontend): FS-030.8/.9 team list + add/edit form UI

- **ID**: TS-M4-C4
- **Type**: Technical Story
- **Parent**: US-M4-C2
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

The team screen inside the admin shell, wired to TS-M4-C3.

## Impact

- add(frontend): `TeamListPage` — table (name, status, member count) + bulk actions.
- add(frontend): `TeamFormDialog` — name, status, assignment-alert override, lead select (from current
  members only — selecting a member for removal that is the lead resets the lead), a roster list with
  per-member remove checkboxes (NO add control, BS-030-14; adding is on the staff profile).
- add(store): **`TeamAdminStore`** (MobX, TDD) — list/create/update/delete/member-removal/lead.

## Regressions

- None; additive.

## Acceptance Tests

### AC-1: FS-030.8 — team list renders with member counts. [BROWSER]
- Setup: `cargo run -p tools --bin seed`; log in as admin (`admin`/`Admin123!`) at http://localhost:3702/staff/login.
- Navigate: http://localhost:3702/staff/admin/teams
- Verify: the team table renders name, status, member count; the seeded "Tier 2" appears.
- Status: [x]

### AC-2: FS-030.9 — create a team + set lead from members. [BROWSER]
- Setup: admin on /staff/admin/teams.
- Action: click "Add Team"; type a >=3-char name; set Active; click Save.
- Action: add a member to it from a staff profile (/staff/admin/staff → staff record → add-to-team); return to the team; select that member as Lead; click Save.
- Verify: success; the roster shows the member with the lead badge.
- Status: [x]

### AC-3: BS-030-14 — no add-member control on the team form. [BROWSER]
- Setup: admin editing a team on /staff/admin/teams.
- Action: open the team add/edit form.
- Verify: the roster exposes only per-member remove checkboxes; there is NO add-member widget on the team form.
- Status: [x]

## Test Infrastructure

- Vitest + RTL + MSW mocking team endpoints. Admin account.

## Dependencies

- **TS-M4-A0**: shell. **TS-M4-C3**: team endpoints. **EPIC-M4-B**: members via profile.
- **Router**: adds `/staff/admin/teams` to the shared admin `<Route>` block in `router.tsx` —
  coordinate the single coherent router edit with C2/C6 + E2 (all C/E screens touch the same block).
