# US-M4-H1 — Premade manager manages canned responses

- **ID**: US-M4-H1
- **Type**: User Story
- **Parent**: EPIC-M4-H
- **Labels**: User Story, M4
- **Scope**: medium

## Spec References

- FS-022 (canned responses — the admin CRUD surface deferred out of M2)
- BS-031-020 (`can_manage_premade` flag gates this surface)

## Context

Epic: EPIC-M4-H — Canned Response CRUD UI. M2 built the `canned_response` / `canned_attachment` model
and the consumption path but deferred the CRUD UI to M4. This story delivers the admin surface, gated
by `can_manage_premade` (a delegated, non-admin capability like FAQ categories). The two M2 seeded
responses become editable rows.

## Description

As a staff member with the "Can Manage Premade" permission, I can list canned responses, create or
edit one (title, optional department scope, body with `%{token}` variables, attachments, active
flag), and disable or delete one. The variable tokens reuse the M2 catalog.

## Impact

- Frontend (canned-response list + create/edit form, reachable via the premade capability gate)
- Backend (canned-response CRUD + `can_manage_premade` gate)
- Database (`canned_response`, `canned_attachment`)
- Browser (desktop)

## Business Rules

- Access is gated by `can_manage_premade` (admins included), NOT the admin gate.
- The body may embed `%{token}` variables from the M2 catalog (substituted at consumption time, M2).
- Attachments reuse the M2 blob store + `canned_attachment` link.

## Regressions

- The M2 consumption path (reply-composer dropdown) keeps working; editing a response updates what the
  dropdown offers. The two M2 seeded responses remain valid and editable.

## Acceptance Criteria

### AC-1: A premade manager reaches the Canned Responses screen (and NOT settings). [BROWSER]
- Setup: reseed. Mint the delegated account `premade1`/`Premade123!` with a group carrying `can_manage_premade` — either the one-call `POST http://localhost:3701/api/dev/set-group-perm {group_id, can_manage_premade:true}` (extended by TS-M4-H1) or the M4-B admin group POST `{name:"Premade Managers", can_manage_premade:true, can_post_reply:true}`; then `POST http://localhost:3701/api/dev/seed-staff {username:"premade1", password:"Premade123!", group_id:<that group id>, isadmin:false}`.
- Navigate: http://localhost:3702/staff/login → log in `premade1` / `Premade123!` → the admin/staff panel.
- Verify: the Canned Responses screen (/staff/admin/canned) renders and is reachable; admin-only nav entries (System Settings, Staff, SLA, Pages, Logs) are ABSENT for this non-admin.
- Verify: the two M2 seeded canned responses are listed.
- Status: [x] — VERIFIED in-browser: premade1/Premade123! (non-admin, group carries can_manage_premade+can_manage_faq) logs in, /staff/admin/canned renders. Sidebar shows ONLY delegated entries (Canned Responses + FAQ Categories); all admin-only entries (System Settings, Staff, Groups, Departments, Teams, Help Topics, SLA, Priorities, Site Pages, Logs) are ABSENT. Admin also reaches the screen. The M2 seeded responses are listed (NOTE: the seed now ships 3 — "Acknowledge receipt", "Closed — disabled sample", "Sample (with attachment)" — not 2).

### AC-2: The manager creates a canned response with a variable token. [BROWSER]
- Setup: continue logged in as `premade1` at /staff/admin/canned.
- Action: click "Add Canned Response"; Title "Greeting", Body "Hello %{ticket.name}", check Active; Save.
- Verify: success banner; "Greeting" appears in the list.
- Status: [x] — VERIFIED in-browser as premade1: Add Canned Response → Title "Greeting", Department "Support" (from the dept-options dropdown), Body "Hello %{ticket.name}", Enabled → Save → green "Greeting added successfully" banner; "Greeting" row appears (Support, Enabled).

### AC-3: The new response is available in the reply composer dropdown. [BROWSER]
- Setup: an open ticket must exist (reuse M1/M3 seeded ticket, or `POST /api/dev/seed-ticket`). "Greeting" created in AC-2.
- Action: open a ticket in the staff area (/staff/tickets → open one) → open the reply composer's canned-response dropdown.
- Verify: "Greeting" is selectable (M2 consumption path); selecting it inserts its body.
- Status: [x] — VERIFIED in-browser: opened seeded ticket #953069, Reply composer → Canned response dropdown lists "Greeting" (disabled "Closed — disabled sample" correctly absent); selecting "Greeting" inserts its body into the Reply field as "Hello QA Seed" (the %{ticket.name} token substituted with the ticket's name — M2 consumption + substitution).

### AC-4: The manager edits and disables a response. [BROWSER]
- Setup: continue as `premade1`; "Greeting" exists.
- Action: at /staff/admin/canned edit "Greeting" body → Save (success). Then toggle it to disabled/inactive.
- Verify: reopen a ticket's reply composer → the disabled "Greeting" no longer appears in the canned-response dropdown.
- Status: [x] — VERIFIED in-browser as premade1: edited "Greeting" body (appended "— updated by premade1") and unchecked Enabled → Save → "Greeting updated successfully" banner, row Status = Disabled. Reopened ticket #953069's reply composer (as agent) → the Canned response dropdown no longer lists "Greeting" (only None / Acknowledge receipt / Sample) — the disabled response left the M2 consumption dropdown.

### AC-5: A staff member WITHOUT the flag is denied. [API-ONLY]
- Setup: an `agent` session (POST /api/staff/login agent/Agent123! — the seeded agent lacks can_manage_premade).
- Request: GET http://localhost:3701/api/staff/canned-responses as `agent` → 403.
- Status: [x] — VERIFIED live: GET /api/staff/canned-responses as agent (no can_manage_premade) → 403.

### AC-6: A plain agent is denied the Canned Responses screen in the browser. [BROWSER]
- Setup: reseed (the seeded `agent` lacks `can_manage_premade`).
- Navigate: http://localhost:3702/staff/login → log in `agent` / `Agent123!` → attempt to open /staff/admin/canned directly.
- Verify: the agent is redirected/denied — the Canned Responses management screen does NOT render for this non-premade agent (a not-authorised redirect or a denied state), matching the API-only 403 (negative delegated gate).
- Status: [x] — VERIFIED in-browser: logged in as agent/Agent123! (authenticated, landed on /staff/tickets with no admin nav), then navigated directly to /staff/admin/canned → redirected to /staff/tickets; the Canned Responses screen does NOT render for the non-premade agent (matches the API 403).

## Checklist (children)

- [ ] TS-M4-H1 — Backend: canned-response CRUD + `can_manage_premade` gate
- [ ] TS-M4-H2 — Frontend: canned-response list + create/edit form UI

## Test Infrastructure

- Admin (`admin`/`Admin123!`) + `agent` (`agent`/`Agent123!`) accounts + the delegated `premade1`/`Premade123!` (group carries `can_manage_premade`). Reuses the M2 canned-response seed + blob store + `drain_multipart`.
- Delegated account setup: grant the flag via the **extended one-call `POST /api/dev/set-group-perm {group_id, can_manage_premade:true, can_manage_faq:true}`** (built by TS-M4-H1) or the M4-B admin group POST; then `POST /api/dev/seed-staff {group_id}` for `premade1`.
- `POST /api/dev/reset-canned` is **built by TS-M4-H1** (restores the two M2 seeded responses) — use it for cheap reset between CRUD runs; reseed remains a fallback.

## Dependencies

- **M2 (done)**: `canned_response` / `canned_attachment` model + `%{token}` catalog + reply composer.
- **EPIC-M4-B**: `can_manage_premade` flag + a carrying group.
- **EPIC-M4-A (shell)**: reachable via the premade capability gate.
