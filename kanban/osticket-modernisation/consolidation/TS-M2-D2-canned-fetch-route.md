# TS-M2-D2 — add(route): FS-022.14 canned-response fetch (substituted body + attachments)

- **ID**: TS-M2-D2
- **Type**: Technical Story
- **Parent**: US-M2-2
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

The route the reply-box dropdown calls: list the offerable canned responses for a ticket, and fetch
one with its body **variable-substituted** (Epic C) for that ticket plus its attachment list.

## Impact

- add(route): `GET /api/staff/tickets/{id}/canned` — lists **enabled** responses scoped to the ticket's department (or dept 0/all) (BS-022.1, BS-022.2): `id`, `title`.
- add(route): `GET /api/staff/tickets/{id}/canned/{cannedId}` — returns the body run through the substitution engine for this ticket (FS-022.14) + the attachment list (`id`, `name`, `size`).
- A disabled or out-of-scope canned id returns 404 (BS-022.2).

## Regressions

- New read routes; no write path touched.

## Acceptance Tests

### AC-1: BS-022.1/.2 — the list returns only enabled, dept-scoped responses. [API-ONLY]
- GET the list for a seeded ticket → "Acknowledge receipt" present; the disabled sample absent.
- Status: [ ]

### AC-2: FS-022.14 — fetching a response returns its body with %{ticket.number} substituted. [API-ONLY]
- GET the canned detail for a ticket → body has the ticket's number, no literal `%{...}`.
- Status: [ ]

### AC-3: FS-022.14 — the response includes its attachment list. [API-ONLY]
- GET the detail → `files` includes `policy.txt` with id/name/size.
- Status: [ ]

### AC-4: BS-022.2 — fetching a disabled canned id returns 404. [API-ONLY]
- GET the detail for the disabled sample → 404.
- Status: [ ]

## Dependencies

- TS-M2-D1 (schema + seed), TS-M2-C1 (substitution engine), TS-M1-C1 (staff session).
