# TS-M1-B2 — add(route): BS-011 public create-ticket route + validation

- **ID**: TS-M1-B2
- **Type**: Technical Story
- **Parent**: US-M1-2
- **Labels**: Technical Story, M1
- **Scope**: small

## Context

The web-form adapter over the ticket service core (TS-M1-B1). Validates the public submission
and calls `create_ticket`. No CAPTCHA/banlist/throttle in M1.

**M1 deviations (deliberate):**
- **`topicId` is NOT required** in M1. FS-011.8 marks the help topic required; that requirement
  is **deferred to M4** (Admin Configuration, where help topics are managed).
- **No CSRF** on this route — it is a public, unauthenticated endpoint (ROADMAP.md →
  Decisions → 2 exempts `POST /api/tickets`).

## Impact

- add(route): `POST /api/tickets` (public, unauthenticated) accepting name, email, subject, message.
- update(core): wires FS-003 validation (required fields + email) and sanitization before calling create_ticket.
- add(route): returns the generated ticket number on success; structured field errors on validation failure.

## Regressions

- None new; depends on the shared core (TS-M1-B1).

## Acceptance Tests

### AC-1: BS-011 — POST with valid payload returns 201 + ticket number and persists the ticket. [API-ONLY]
- Setup: API running on 3701, migrations + seed applied.
- Request: `curl -s -w "\n%{http_code}\n" -X POST http://localhost:3701/api/tickets -H 'Content-Type: application/json' -d '{"name":"Jane Doe","email":"jane@example.com","subject":"Printer broken","message":"It won'\''t print."}'`.
- Expect: 201 with a 6-digit ticket number in the body; the ticket is persisted (visible via the staff detail route).
- Status: [ ]

### AC-2: BS-011 — POST missing a required field or with an invalid email returns 422 (shared envelope, field errors) and creates no ticket. [API-ONLY]
- Request: `curl -s -w "\n%{http_code}\n" -X POST http://localhost:3701/api/tickets -H 'Content-Type: application/json' -d '{"name":"Jane","email":"not-an-email","subject":"","message":""}'`.
- Expect: 422 with `{ "error": { "message": ..., "fields": { "email": ..., "subject": ..., "message": ... } } }`; no ticket created.
- Status: [ ]

### AC-3: It should sanitize name/subject/message (HTML) before persist. [API-ONLY]
- Request: POST with `"message":"<script>alert(1)</script>hello"` (and similar in name/subject).
- Verify: read the created ticket back via the staff detail route — the persisted body has the disallowed HTML stripped/escaped (ammonia), safe text preserved.
- Status: [ ]

### AC-4: The route accepts a payload without topicId (not required in M1). [API-ONLY]
- Request: POST a valid body that omits `topicId` entirely.
- Expect: 201 (topicId deferred to M4, not required in M1).
- Status: [ ]

### AC-5: The route requires no CSRF token (public unauthenticated endpoint). [API-ONLY]
- Request: POST a valid body with NO `X-CSRFToken` header and NO session cookie.
- Expect: 201 (POST /api/tickets is CSRF-exempt per ROADMAP Decisions §2).
- Status: [ ]

## Dependencies

- TS-M1-B1 (service core), TS-M1-A4a (validation/sanitize).
