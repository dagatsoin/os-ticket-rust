# TS-M3-F1 — add(route): FS-020.7 basic search (number/email/deep-text)

- **ID**: TS-M3-F1
- **Type**: Technical Story
- **Parent**: US-M3-F1
- **Labels**: Technical Story, M3
- **Scope**: small

## Context

Implements basic keyword search: a `query` parameter triggers search by ticket number
(numeric), requester email (contains @), or deep text search (subject, name, thread body).
Minimum 3 characters required.

## Impact

- update(route): `GET /api/staff/tickets?a=search&query={keyword}`:
  - If `query` < 3 chars: return 400 with error "Search term must be more than 3 chars".
  - Keyword resolution (BS-020.7):
    - Purely numeric: `ticketID LIKE '{query}%'` (prefix match).
    - Contains `@` and valid email: `email = '{query}'` (exact match).
    - Otherwise: deep search `LIKE '%{query}%'` across email, name, subject, thread body.
  - Response: filtered ticket listing with `search_results: true` flag.
- update(service): `ticket_service::search(query, staff_visibility)`:
  - Joins thread_entry table for deep search.
  - Returns DISTINCT tickets (avoid duplicates from thread joins).

## Regressions

- Regular listing without `a=search` must continue to work.
- Visibility scoping still applies to search results.

## Acceptance Tests

### AC-1: BS-020.7 — Numeric query matches ticket number prefix. [API-ONLY]
- Setup: POST /api/dev/reset-tickets
- Setup: POST /api/dev/seed-tickets with `{"ticketId":"123456"}`
- Setup: POST /api/dev/seed-tickets with `{"ticketId":"123789"}`
- Setup: POST /api/dev/seed-tickets with `{"ticketId":"999999"}`
- Request: GET http://localhost:3701/api/staff/tickets?a=search&query=1234
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, results include ticket #123456
- Expect: Results exclude #123789 (prefix is 1237), #999999
- Status: [x]

### AC-2: BS-020.7 — Email query matches requester email exactly. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"email":"alice@example.com"}`
- Setup: POST /api/dev/seed-tickets with `{"email":"alice-other@example.com"}`
- Request: GET http://localhost:3701/api/staff/tickets?a=search&query=alice@example.com
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, only ticket from alice@example.com returned
- Expect: Ticket from alice-other@example.com NOT returned
- Status: [x]

### AC-3: BS-020.7 — Free text query triggers deep search. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"subject":"Network outage","body":"router failure occurred"}`
- Request: GET http://localhost:3701/api/staff/tickets?a=search&query=router%20failure
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, ticket with "Network outage" subject returned
- Status: [x]

### AC-4: BS-020.6 — Query < 3 chars returns 400. [API-ONLY]
- Request: GET http://localhost:3701/api/staff/tickets?a=search&query=ab
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 400, error.message = "Search term must be more than 3 chars"
- Status: [x]

### AC-5: BS-020.6 — Empty query with a=search returns 400. [API-ONLY]
- Request: GET http://localhost:3701/api/staff/tickets?a=search&query=
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 400, error.message = "Search term must be more than 3 chars"
- Status: [x]

### AC-6: Deep search joins thread but returns distinct tickets. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"subject":"Password issue"}` -> ticket_id
- Setup: POST /api/staff/tickets/{ticket_id}/reply with `{"body":"password reset step 1"}`
- Setup: POST /api/staff/tickets/{ticket_id}/reply with `{"body":"password reset step 2"}`
- Setup: POST /api/staff/tickets/{ticket_id}/reply with `{"body":"password reset complete"}`
- Request: GET http://localhost:3701/api/staff/tickets?a=search&query=password%20reset
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, ticket appears exactly once (not 3 times)
- Status: [x]

### AC-7: Search respects visibility scoping. [API-ONLY]
- Setup: POST /api/dev/seed-tickets with `{"dept":"External","ticketId":"888888"}` (agent has no access)
- Request: GET http://localhost:3701/api/staff/tickets?a=search&query=888888
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, ticket NOT returned (visibility enforced)
- Status: [x]

### AC-8: Response includes search_results flag. [API-ONLY]
- Setup: POST /api/dev/seed-tickets
- Request: GET http://localhost:3701/api/staff/tickets?a=search&query=test
- Headers: Authorization: Bearer {{staff_token}}
- Expect: 200, response has `search_results: true`
- Status: [x]

## Test Infrastructure

- Uses seeded staff credentials `agent` / `Agent123!` (TS-M1-A3).
- Dev endpoints needed:
  - **`POST /api/dev/seed-tickets`** — accepts `ticketId`, `email`, `subject`, `body`, `dept` params
  - **`POST /api/dev/reset-tickets`** — clears all tickets for clean state

## Dependencies

- **TS-M3-A2**: visibility scoping.
- **M1 done**: ticket listing route, thread_entry table.
