# TS-M4-C3 — add(route): FS-030.10/.11/.12 team CRUD + lead/member-removal + delete release

- **ID**: TS-M4-C3
- **Type**: Technical Story
- **Parent**: US-M4-C2
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

Backend for team management: list, create/edit (lead from members, member removal only), delete
releasing associations, bulk actions.

## Impact

- add(route): `GET /api/staff/admin/teams`, `POST`, `PUT /:id`, bulk endpoint — admin-gated.
- add(service): BS-030-12 name validation; BS-030-13 lead-must-be-a-current-member (removing the lead
  resets `lead_id=NULL`); BS-030-16 delete releases associations —
  `UPDATE ticket SET team_id=0 WHERE team_id=:id` **AND `UPDATE help_topic SET team_id=NULL WHERE
  team_id=:id`** (help_topic.team_id is a **RESTRICT FK** — the delete FK-violates unless the topic
  rows are cleared first), then remove `team_member` rows.
- Member ADD is NOT here — the team form only removes members (BS-030-14; add is owned by TS-M4-B1's
  existing `POST /api/staff/admin/staff/:id/teams`). KL-030-12 MODERNISED: `updated` added to the
  allowed sort keys so "Last Updated" sorts correctly.
- add(dev): `POST /api/dev/reset-teams`.

## Regressions

- The M3 "Tier 2" team + its membership stay intact.

## Acceptance Tests

### AC-1: BS-030-12 — name validation. [API-ONLY]
- Setup: `cargo run -p tools --bin seed`; admin session cookie (`POST /api/staff/login` as `admin`/`Admin123!`).
- Request: `POST /api/staff/admin/teams` with a 2-char name → Expect 422 "Team name must be at least 3 chars.".
- Request: `POST /api/staff/admin/teams` with an existing name (e.g. "Tier 2") → Expect 422 "Team name already exists".
- Status: [x]

### AC-2: BS-030-13 — lead must be a current member; removing the lead resets it. [API-ONLY]
- Setup: admin cookie; a team with a member set as lead (add a member via `POST /api/staff/admin/staff/:id/teams` from TS-M4-B1, then set lead).
- Request: `PUT /api/staff/admin/teams/:id` with `lead_id` = a non-member staff id → Expect rejection (422).
- Request: `PUT /api/staff/admin/teams/:id` marking the current lead for removal → Expect 200 with `lead_id` reset to NULL on the returned/GET row.
- Status: [x]

### AC-3: BS-030-16 — deletion releases ticket AND help-topic associations. [API-ONLY]
- Setup: admin cookie; a team assigned to a ticket (`seed-ticket` + assign the team) AND referenced by a help topic (`seed-topic` with the team as auto-assignee, exercising the help_topic.team_id RESTRICT FK).
- Request: `DELETE /api/staff/admin/teams/:id` → Expect 200 (no FK violation); the team's `team_member` rows are gone, the ticket's team assignment is cleared (`team_id=0`), and the help topic's `team_id` is NULL (verify via `GET` on the ticket + topic).
- Status: [x]

### AC-4: BS-030-14 — the team form exposes no add-member operation. [API-ONLY]
- Setup: admin cookie.
- Request: `PUT /api/staff/admin/teams/:id` — Expect the accepted payload has no add-member field (an add-member field is ignored/rejected); adding members is only via TS-M4-B1's `POST /api/staff/admin/staff/:id/teams`.
- Status: [x]

## Test Infrastructure

- Admin account; dev `reset-teams`. `.sqlx` cache updated.

## Dependencies

- **EPIC-M4-PREP**, **TS-M4-A0**, **TS-M4-B1** (member add source).
