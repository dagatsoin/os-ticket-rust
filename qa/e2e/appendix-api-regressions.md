# Appendix — API Regressions (`[API-ONLY]`)

> `[API-ONLY]` ACs that are validated via the REST API rather than the browser. These are NOT
> part of the main browser journeys (qa-criterion-tester runs the playbook against the UI).
> They are kept here so the contract-level checks aren't lost. Variables (`{BASE_URL_API}` etc.)
> are defined in the main playbook's Variable Map.

**Dernière mise à jour: 2026-06-11**

## M1

### API-REG-US-M1-2-AC-4 — Created ticket persists (status Open, seeded dept, `M` thread entry)

- Request: `curl -s -X POST {BASE_URL_API}/api/tickets -H 'Content-Type: application/json' -d '{"name":"Jane Doe","email":"jane@example.com","subject":"Printer broken","message":"My printer won'\''t print."}'` → capture the returned ticket number.
- Request: `curl -s -c /tmp/c.txt -X POST {BASE_URL_API}/api/staff/login -H 'Content-Type: application/json' -d '{"username":"agent","password":"Agent123!"}'`
- Request: `curl -s -b /tmp/c.txt "{BASE_URL_API}/api/staff/tickets?status=open"` → find the ticket id; then `curl -s -b /tmp/c.txt {BASE_URL_API}/api/staff/tickets/{id}`.
- Expect: 201 on create; the detail shows status `open`, the seeded "Support" department, and a first thread entry of type `M` containing the submitted body.
- Status: [ ]

### API-REG-US-M1-3-AC-5 — Unauthenticated staff route is rejected

- Request: `curl -s -o /dev/null -w "%{http_code}" "{BASE_URL_API}/api/staff/tickets?status=open"` (no session cookie).
- Expect: 401 with the shared error envelope; no ticket data returned.
- (Browser variant lives in the main playbook only implicitly; this appendix covers the API contract.)
- Status: [ ]

### API-REG-US-M1-4-AC-3 — Client session is ticket-scoped (cannot read another ticket)

- Setup: seed two tickets — `curl -s -X POST {BASE_URL_API}/api/dev/seed-ticket` twice → ticketA {numberA, emailA}, ticketB {numberB, emailB}.
- Request: `curl -s -c /tmp/cli.txt -X POST {BASE_URL_API}/api/client/login -H 'Content-Type: application/json' -d '{"ticketNumber":"<numberA>","email":"<emailA>"}'`.
- Request: `curl -s -b /tmp/cli.txt {BASE_URL_API}/api/client/ticket`.
- Expect: returns ONLY ticketA's thread (`M`+`R`, never `N`); there is no route to read ticketB; reaching a staff route with the client cookie is denied (401/403).
- Status: [ ]

### API-REG-TS-M1-A5-AC-1 — Frontend build + Vitest suite pass

- Request: in `frontend/`, run `npm run build` then `npm test`.
- Expect: both succeed (bundle emitted; Vitest suite green).
- Status: [ ]

### API-REG-TS-M1-A5-AC-4 — apiClient injects `X-CSRFToken` and redirects on 401 (TDD)

- Request: `npm test` — apiClient unit test (MSW-mocked) asserts a mutating request carries `X-CSRFToken` from the realm XSRF cookie, and that a 401 triggers a redirect to the matching realm login.
- Expect: both assertions pass.
- Status: [ ]

### API-REG-TS-M1-A5-AC-5 — apiClient parses the shared JSON error envelope

- Request: `npm test` — apiClient unit test feeds `{ "error": { "message": "...", "fields": { "email": "..." } } }`.
- Expect: parsed into top-level + per-field errors.
- Status: [ ]

### API-REG-TS-M1-A5-AC-6 — Shell health indicator reflects `/api/health`

- Request: `npm test` — component test (MSW harness) mocks `/api/health` ok then down.
- Expect: the shell indicator reflects "backend OK" / "down" accordingly.
- Status: [ ]
