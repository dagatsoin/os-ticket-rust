# TS-M4-B5 — add(route): FS-031.9/.10/.13 own-profile GET/PUT + password change + directory

- **ID**: TS-M4-B5
- **Type**: Technical Story
- **Parent**: US-M4-B3
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

Backend for the two non-admin staff screens: own-profile read/update + password change, and the
directory search endpoint.

## Impact

- add(route): `GET /api/staff/profile` / `PUT /api/staff/profile` — loads the AUTHENTICATED staff's
  record (ignores submitted id except tamper guard, EC-031-006); username read-only. **Timezone is
  LIVE-DERIVED from `staff.timezone_id` per request** (no stored/session offset): saving the profile
  just persists `timezone_id`, and any subsequent request re-derives the offset from it. There is no
  "refresh the session offset" step — the oracle is that the saved timezone is reflected on next read.
- add(route): `PUT /api/staff/profile/password` — current + new + confirm; on success clear
  forced-change (`change_passwd`) and record the `passwdreset` timestamp (BS-031-013).
  **Scope: current-password path only.** Verified there is **no reset-token store in M1** (only the
  `passwdreset` timestamp + `change_passwd` flag exist — no token table/column). The reset-token
  path (current-password omitted, "cancel reset tokens") is therefore **DEFERRED** until a token
  store lands; this TS does not implement or test it.
- add(route): `GET /api/staff/directory` — lists directory-visible staff (BS-031-009); `q` matching
  per BS-031-030; `did` department filter; sort keys; pagination. Any authenticated staff (not admin-gated).
- add(service): password-aging check (FS-031.12) sets the forced-change flag on login when over age
  (non-admin only).
- add(dev): **extend `POST /api/dev/age-password` with a flag** (e.g. `set_change_passwd: bool`,
  default true) so it can backdate `passwdreset` **without** pre-setting `change_passwd`. Needed so
  AC-6 can prove the *login-time* aging check itself sets the forced-change flag (the current
  endpoint always sets `change_passwd=true`, masking the computation). Owned here.

## Regressions

- The M1 login flow gains the aging check; admins are exempt; existing sessions unaffected.
- Timezone offset is now live-derived from `staff.timezone_id` per request (no stored session
  offset); verify existing timezone-dependent renders still resolve correctly.

## Acceptance Tests

### AC-1: FS-031.10 — profile loads the authenticated staff; username read-only. [API-ONLY]
- Setup: authenticate as `agent` / `Agent123!`.
- Request: `GET /api/staff/profile` → 200 with `agent`'s own record.
- Request: `PUT /api/staff/profile` submitting a changed `username` (and a foreign `id`, EC-031-006) → the username change is ignored/blocked and the record identity is unchanged.
- Status: [x]

### AC-2: FS-031.10 — profile timezone persists (live-derived). [API-ONLY]
- Setup: authenticate as `agent`.
- Request: `PUT /api/staff/profile` with a new `timezone_id` → 200.
- Request: a subsequent `GET /api/staff/profile` → returns the new `timezone_id`. (Timezone is
  live-derived from `staff.timezone_id` per request; there is no stored session offset to refresh —
  the oracle is that the saved timezone is reflected on reload.)
- Status: [x]

### AC-3: BS-031-013 — password change (current-password path) clears forced-change + records reset. [API-ONLY]
- Setup: `POST /api/dev/age-password` {staffId: agent, days: 120} to set forced-change; authenticate as `agent`.
- Request: `PUT /api/staff/profile/password` {current, new (>=6), confirm} valid → 200; forced-change (`change_passwd`) cleared and `passwdreset` recorded.
- Request: `PUT …/password` with a WRONG `current` → 422 current-password error.
- Request: `PUT …/password` with new/confirm mismatch → 422; with a <6-char new → 422.
- Note: the reset-token path ("current omitted, cancel reset tokens") is **DEFERRED** — no reset-token store exists in M1; not tested here.
- Status: [x]

### AC-4: BS-031-030 — directory search matching. [API-ONLY]
- Setup: authenticate as `agent`.
- Request: `GET /api/staff/directory?q=<numeric>` → matches phone/ext/mobile; `?q=<email>` → email exact; `?q=<name-substring>` → email OR lastname OR firstname substring.
- Status: [x]

### AC-5: BS-031-009 — only directory-visible staff listed; did filter counts visible only. [API-ONLY]
- Setup: make one staff row `isvisible=false` (via `PUT /api/staff/admin/staff/:id` as admin — directory-visible toggle; see Test Infrastructure note on seed-staff lacking this flag).
- Request: `GET /api/staff/directory` → that staff is absent; the `did` department-filter counts exclude it.
- Status: [x]

### AC-6: FS-031.12 — over-age non-admin gets forced-change on login; admin exempt. [API-ONLY]
- Setup: `POST /api/dev/seed-config` {passwd_reset_period: 30} so the aging window is active.
- Action: `POST /api/dev/age-password` {staffId: <non-admin>, days: 120, set_change_passwd: false}
  (backdates `passwdreset` **without** pre-setting `change_passwd`), then log in → the login flow's
  aging check itself sets the forced-change flag (`change_passwd` starts false, is true after login).
- Action: repeat with `set_change_passwd: false` for `admin` → the flag stays NOT set (admins exempt).
- Status: [x]

## Test Infrastructure

- `agent` account; dev `age-password` (confirmed) + `seed-config` (confirmed) for the aging window. `.sqlx` cache updated.
- **Owned here:** the `age-password` `set_change_passwd` flag (see Impact / AC-6) so AC-6 can prove the
  login-time aging computation without the endpoint pre-forcing the flag.
- AC-5 (`isvisible=false` seed row): use the extended `seed-staff` `isvisible` flag delivered by
  **TS-M4-B3** (preferred), or fall back to the admin edit endpoint (`PUT /api/staff/admin/staff/:id`)
  under test. No longer a gap once TS-M4-B3 lands.
- **Reset-token store — confirmed absent in M1** (only `passwdreset` timestamp + `change_passwd` flag;
  no token table). Password-change is scoped to the current-password path; the reset-token path is
  DEFERRED (see Impact / AC-3).

## Dependencies

- **EPIC-M4-PREP**: staff preference/aging columns, timezone table.
- **M1**: session realm, argon2id util. (No reset-token flow exists in M1 — reset-token password
  path deferred.)
- **TS-M4-B3**: extended `seed-staff` (`isvisible`) for AC-5.
