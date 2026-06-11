# Appendix — API Regressions (`[API-ONLY]`)

> `[API-ONLY]` ACs that are validated via the REST API rather than the browser. These are NOT
> part of the main browser journeys (qa-criterion-tester runs the playbook against the UI).
> They are kept here so the contract-level checks aren't lost. Variables (`{BASE_URL_API}` etc.)
> are defined in the main playbook's Variable Map.

**Dernière mise à jour: 2026-06-11** (M2 API regressions added)

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

## M2

> Variables (`{BASE_URL_API}`, `{BASE_URL_MAILPIT}`) and the QA fixtures block are defined in the
> main playbook's Variable Map. The browser counterparts live in Journey 2.

### API-REG-US-M2-1-AC-5 — Attachment binds to the `M` entry and dedups identical bytes (D1 SHA-256)

- Setup: `cargo run -p tools --bin seed -- --reset`; fixtures present.
- Request: create a ticket via the public multipart route with the SAME `.pdf` bytes, twice (capture both ticket numbers): `curl -s -X POST {BASE_URL_API}/api/tickets -F name=Mia -F email=mia@example.com -F subject=Dup -F message=one -F attachment=@/tmp/qa-fixtures/invoice.pdf` then repeat.
- Request: read each ticket as staff — `curl -c /tmp/qa-staff.jar -X POST {BASE_URL_API}/api/staff/login` (`agent`/`Agent123!`), then `curl -b /tmp/qa-staff.jar {BASE_URL_API}/api/staff/tickets/{id}` for each.
- Expect (DB): `select count(*) from attachment_file` → 1; `select count(*) from ticket_attachment` → 2.
- Expect (disk): `find "${BLOB_ROOT:-var/blobs}" -type f | wc -l` → exactly 1 blob; each ticket's `M` entry lists the file under `attachments` with the right name/size, both pointing at the same `attachment_file` id (one `storage_key` / SHA-256 — D1 dedup).
- Status: [ ]

### API-REG-US-M2-3-AC-4 — Bad / unknown / cross-ticket attachment id is rejected (404, no existence leak)

- Setup: `cargo run -p tools --bin seed -- --reset`; seed ticket-A with both attachments + a ticket-B (different email); authenticate a client session for ticket-A — `curl -c /tmp/qa-client.jar -X POST {BASE_URL_API}/api/client/login` with ticket-A number + email.
- Request: with that session, request a non-existent id — `curl -i -b /tmp/qa-client.jar {BASE_URL_API}/api/client/ticket/attachments/999999`; then request ticket-B's real attachment id via the same ticket-A session.
- Expect: both return 404 with the shared error envelope and an empty/zero-byte body (no existence leak — §8); the 404 is identical whether the id is non-existent or belongs to another ticket (EC-022.7; EC-022.8 cross-session replay is structurally covered by the session-bound no-ticketId route, D2 §8).
- Status: [ ]

### API-REG-US-M2-4-AC-3 — Outbound mail carries anti-loop headers (BS-040.22)

- Setup: `docker compose up -d mailpit`; backend with `SMTP_HOST=localhost SMTP_PORT=3704`; `curl -X DELETE {BASE_URL_MAILPIT}/api/v1/messages`; trigger an autoresponse (create a ticket) AND a notice (post a staff reply) so both are captured.
- Request: list captured messages — `curl -s {BASE_URL_MAILPIT}/api/v1/messages | jq '.messages[].ID'`; fetch each message's headers — `curl -s {BASE_URL_MAILPIT}/api/v1/message/{ID}/headers`.
- Expect: the autoresponse carries `Precedence: auto_reply` (or `bulk`), `X-Auto-Response-Suppress`, and `Auto-Submitted: auto-replied`; the reply-notice carries the notice-class headers (`X-Auto-Response-Suppress: OOF, AutoReply`, `Auto-Submitted: auto-generated`).
- Status: [ ]

### API-REG-US-M2-4-AC-4 — With `SMTP_HOST` unset, sends fall back to the stub mailer (no Mailpit delivery)

- Setup: restart the backend with `SMTP_HOST` unset (`unset SMTP_HOST; cargo run -p api`); `curl -X DELETE {BASE_URL_MAILPIT}/api/v1/messages`; `cargo run -p tools --bin seed -- --reset`.
- Request: create a ticket — `curl -s -X POST {BASE_URL_API}/api/tickets -H 'Content-Type: application/json' -d '{"name":"Mia","email":"mia@example.com","subject":"Stub","message":"hi"}'`; then `curl -s {BASE_URL_API}/api/dev/mailbox`.
- Expect: the intended autoresponse To `mia@example.com` is RECORDED in the dev mailbox JSON (M1 behaviour preserved); `curl -s {BASE_URL_MAILPIT}/api/v1/messages | jq '.total'` → 0 (nothing delivered to Mailpit).
- Status: [ ]

### API-REG-TS-M2-prep-AC-1..4 — `seed -- --reset` purge / preserve / blob-reclaim / non-destructive contract

- AC-1: create a few tickets (some with `-F attachment=@/tmp/qa-fixtures/invoice.pdf`), run `cargo run -p tools --bin seed -- --reset`, then `curl -s -b /tmp/qa-staff.jar '{BASE_URL_API}/api/staff/tickets?status=open'` → empty; `select count(*) from ticket; select count(*) from ticket_attachment;` → both 0.
- AC-2: after reset, staff login (`agent`/`Agent123!`) → 200; `select count(*) from canned_response;` → 2; seeded dept/group rows remain.
- AC-3: `find "${BLOB_ROOT:-var/blobs}" -type f | wc -l` before/after reset → the ticket-only blob is gone, the seeded `policy.txt` canned blob remains (still referenced).
- AC-4: with tickets present, run plain `cargo run -p tools --bin seed` (no flag) → `select count(*) from ticket;` unchanged; dept/group/`agent` reseeded idempotently (no dup rows). Guard: the tool refuses a non-dev target DB.
- Status: [ ]
