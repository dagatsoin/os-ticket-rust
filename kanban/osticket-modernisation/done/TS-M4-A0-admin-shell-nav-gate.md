# TS-M4-A0 — add(frontend): FS-032.1 admin panel shell + nav + admin-gate routing

- **ID**: TS-M4-A0
- **Type**: Technical Story
- **Parent**: US-M4-A1
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

Every M4 admin screen mounts inside a shared admin-panel shell. This TS builds it once: the
`/staff/admin/*` route branch, the admin left-nav (Settings, Staff, Groups, Departments, Teams, Help
Topics, SLA, Priorities, Pages, Logs, FAQ Categories, Canned Responses), and the client-side admin
gate that redirects non-admins. The per-screen bodies are added by their own TS.

## Impact

- add(frontend): `/staff/admin` route branch + `AdminLayout` (nav + content outlet), reusing the M1
  MobX staff auth store + apiClient.
- add(frontend): `RequireAdmin` route guard — redirects non-admins to the staff dashboard with a denial
  notice; `RequireCapability` guard — gates delegated screens (FAQ categories, Canned responses) on a
  specific capability flag instead of `isadmin` (consumed by EPIC-M4-E/H).
- add(frontend): a shared admin snackbar for success / denial notices, reused by every M4 screen.
- add(frontend): AuthStore `profileLoaded` tri-state (loading / loaded / error) plus `isAdmin` and
  `can(flag)` consumers, hydrated from `GET /api/staff/me` so guards don't flash before the profile resolves.
- add(frontend): nav items are capability-aware (hide entries the current staff cannot use).
- consumes: `GET /api/staff/me` (isadmin + capability flags) — **owned and served by TS-M4-A1**, not
  built here (this TS is frontend-only).

## Regressions

- The existing `/staff/tickets` area and its auth store are unchanged; this adds a sibling branch.

## Acceptance Tests

### AC-1: FS-032.1 — an admin sees the admin nav; a non-admin is denied; the staff area still works. [BROWSER]
- Setup: `cargo run -p tools --bin seed` (seeds `admin`/`Admin123!` isadmin + `agent`/`Agent123!`).
- Navigate: http://localhost:3702/staff/login → log in as `admin` / `Admin123!`.
- Navigate: http://localhost:3702/staff/admin
- Verify: the `AdminLayout` renders with a left-nav listing Settings, Staff, Groups, Departments, Teams, Help Topics, SLA, Priorities, Pages, Logs, FAQ Categories, Canned Responses, and a content outlet.
- Action: log out; log in as `agent` / `Agent123!`.
- Navigate: http://localhost:3702/staff/admin
- Verify: the non-admin `agent` is redirected to `/staff/tickets` with a denial notice; the admin nav does not render.
- Verify (regression): the existing `/staff/tickets` area still loads and functions for the agent.
- Status: [x]

### AC-2: Delegated nav items appear for a non-admin with the matching capability flag. [BROWSER]
- Setup: seed a non-admin staff whose group has `can_manage_faq=Yes` — via `POST /api/dev/seed-staff` + `POST /api/dev/set-group-perm` (existing dev endpoints) or the M4-PREP capability seed.
- Navigate: http://localhost:3702/staff/login → log in as that user.
- Navigate: http://localhost:3702/staff/admin
- Verify: the admin gate does NOT redirect (the delegated capability grants access to its own screen); the "FAQ Categories" nav entry is present; the admin-only "System Settings" (Settings) entry is absent.
- Status: [x]

## Test Infrastructure

- Admin account via seed (`admin` / `Admin123!`) and the existing `agent` account.
- Component tests via Vitest + RTL + MSW (mock `/api/staff/me`).

## Dependencies

- **M1 (done)**: staff auth store, apiClient, router.
- **TS-M4-A1**: provides `GET /api/staff/me` (isadmin + the four capability flags) that this shell consumes.
- **EPIC-M4-PREP**: group capability-flag columns (surfaced by TS-M4-A1).
