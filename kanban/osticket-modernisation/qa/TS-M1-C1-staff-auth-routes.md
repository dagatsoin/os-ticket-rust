# TS-M1-C1 — add(route): BS-002 staff auth routes (login/logout/session) + permission gate

- **ID**: TS-M1-C1
- **Type**: Technical Story
- **Parent**: US-M1-3
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

The staff-realm authentication adapter over the FS-002 infra from TS-M1-A4b. Issues a staff
session on valid credentials and enforces the "can reply" permission via the generic permission
gate on protected routes.

**M1 scope notes:**
- The login **identifier is `username` only** in M1. Email-as-identifier (FS-002.9) is
  **deferred**.
- **Account lockout (FS-002.2) is explicitly OUT of M1 scope** (note for QA — do not test for it).
- Cookie attributes follow the auth decision: `ost_staff_sess`, `HttpOnly`, `SameSite=Lax`,
  `Secure` in non-dev; CSRF re-seeds `XSRF-TOKEN-STAFF` on login (ROADMAP.md → Decisions → 1/2).

## Impact

- add(route): `POST /api/staff/login` (**username** + password → `ost_staff_sess` cookie +
  `XSRF-TOKEN-STAFF` seed, session-id regenerated on login), `POST /api/staff/logout`,
  `GET /api/staff/me`.
- add(route): `GET /api/staff/me` returns **`{ id, username, name, deptId }`** for the
  authenticated staff session.
- update(core): staff-realm middleware gating `/api/staff/**` routes; permission check via the
  generic named-permission gate (`can_post_reply`).
- add(infra): password verify against the seeded argon2id hash (TS-M1-A4a hashing).

## Regressions

- Shared auth middleware; affects all staff routes (TS-M1-C2/C3).

## Acceptance Tests

### AC-1: BS-002 — valid credentials establish a staff session (ost_staff_sess) and GET /api/staff/me returns { id, username, name, deptId }. [API-ONLY]
- Setup: API running, migrations + seed applied (`agent` exists).
- Request: `curl -s -i -c /tmp/c.txt -X POST http://localhost:3701/api/staff/login -H 'Content-Type: application/json' -d '{"username":"agent","password":"Agent123!"}'` — confirm an `ost_staff_sess` (HttpOnly) cookie and an `XSRF-TOKEN-STAFF` cookie are set.
- Request: `curl -s -b /tmp/c.txt http://localhost:3701/api/staff/me`.
- Expect: 200 with `{ id, username: "agent", name, deptId }`.
- Status: [ ]

### AC-2: BS-002 — invalid credentials are rejected (401), no session issued. [API-ONLY]
- Request: `curl -s -i -X POST http://localhost:3701/api/staff/login -H 'Content-Type: application/json' -d '{"username":"agent","password":"wrong"}'`.
- Expect: 401, shared error envelope, no `ost_staff_sess` cookie set.
- Status: [ ]

### AC-3: BS-002 — a staff-only route without a session is rejected (401). [API-ONLY]
- Request: `curl -s -o /dev/null -w "%{http_code}" http://localhost:3701/api/staff/me` (no cookie).
- Expect: 401.
- Status: [ ]

## Dependencies

- TS-M1-A4b (session/CSRF/realm gate/permission gate), TS-M1-A4a (argon2id verify),
  TS-M1-A3 (seeded staff).
