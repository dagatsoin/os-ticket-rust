# US-M4-D2 — Admin views the read-only priority reference set

- **ID**: US-M4-D2
- **Type**: User Story
- **Parent**: EPIC-M4-D
- **Labels**: User Story, M4
- **Scope**: small

## Spec References

- FS-032.8 (ticket priority reference set)
- BS-032.13 (priorities are a fixed, urgency-ranked set)
- KL-032.1 (no admin UI for priorities — PRESERVED: read-only, no CRUD)

## Context

Epic: EPIC-M4-D — SLA Plans & Priorities. Priorities are a fixed, urgency-ranked set in osTicket 1.7
(no CRUD). This story surfaces them as a **read-only** reference panel so admins can see the scale
that drives ticket urgency, without implying editability.

## Description

As an administrator, I can open a Priorities panel and see the fixed, urgency-ranked priority set
(e.g., Low / Normal / High / Emergency) with their colors/urgency rank, presented read-only — there
are no add/edit/delete controls (KL-032.1 preserved as faithful 1.7 behavior).

## Impact

- Frontend (read-only priorities panel)
- Backend (priority list read endpoint)
- Database (`priority`)
- Browser (desktop)

## Business Rules

- BS-032.13: the priority set is fixed and urgency-ranked; not operator-editable.

## Regressions

- The M3 priority usage (ticket priority, topic default priority) is unchanged.

## Acceptance Criteria

### AC-1: The Priorities panel lists the fixed set read-only. [BROWSER]
- Setup: reseed the dev DB — `cargo run -p tools --bin seed` (restores the 4 seeded priorities).
- Navigate: http://localhost:3702/staff/login → log in `admin` / `Admin123!` → admin shell → /staff/admin/priorities.
- Verify: the fixed priority set is listed in urgency order — `ORDER BY urgency DESC` yields **Low, Normal, High, Emergency** — each showing its urgency rank and color swatch.
- Verify: there are NO Add / Edit / Delete controls anywhere on the panel — no "Add Priority" button, no per-row edit/delete, no bulk mass-actions (read-only, KL-032.1 preserved).
- Status: [x]

### AC-2: The priority read endpoint returns the ranked set. [API-ONLY]
- Setup: obtain an admin session cookie (POST /api/staff/login with admin/Admin123!).
- Request: GET http://localhost:3701/api/staff/admin/priorities (admin session).
- Expect: 200; the fixed priorities returned `ORDER BY urgency DESC` (Low, Normal, High, Emergency) with rank + color; no create/update/delete route exists for priorities.
- Status: [x]

## Checklist (children)

- [ ] TS-M4-D3 — Frontend: read-only priorities reference panel
- [ ] (shared) TS-M4-D1 — priority read endpoint (owned by US-M4-D1's TS-M4-D1)

## Test Infrastructure

- Admin account. Reads the seeded `priority` set (M1/M3).

## Dependencies

- **EPIC-M4-A (shell)**. Priority read endpoint provided by TS-M4-D1.
