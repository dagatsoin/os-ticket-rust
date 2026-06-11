# TS-M2-E3 — update(core): FS-011.12 autoresponse on create + FS-021.3 notification on reply

- **ID**: TS-M2-E3
- **Type**: Technical Story
- **Parent**: US-M2-4
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Wires the E2 wrappers/templates into the two ticket events so the customer actually receives mail.

**Always-send in M2 (ROADMAP M2 Decisions §12):** the autoresponse + reply notification are **always
sent** in M2; per-department auto-response flags are honored in M5. **helpdesk_url (§6):** the wiring
reads the seeded `helpdesk_url` config key and feeds it to the substitution engine as the `%{url}`
base. Mailpit-dependent assertions skip-pass when `MAILPIT_URL` is unset (§5).

## Impact

- update(core): on public ticket create, after commit, **always send the new-ticket autoresponse** (auto-reply wrapper) to the requester (FS-011.12), rendered via Epic C with `%{url}` = the `helpdesk_url` config key (§6, §12).
- update(core): on staff reply, after commit, **always send the reply notification** (notice wrapper) to the requester (FS-021.3), rendered via Epic C (§12).
- Sends go through whichever transport E1 selected (SMTP when `SMTP_HOST` set; stub mailbox otherwise).

## Regressions

- Sends happen **only after DB commit** (M1 convention from TS-M1-C3 — no mail for a failed write). With `SMTP_HOST` unset, the recorded-intent dev-mailbox behaviour matches M1.

## Acceptance Tests

> Setup (AC-1..3): `docker compose up -d mailpit`; `cargo run -p tools --bin seed -- --reset`; backend with
> `SMTP_HOST=localhost SMTP_PORT=3704 SMTP_FROM=support@example.com`; empty Mailpit
> (`curl -X DELETE http://localhost:3705/api/v1/messages`). Mailpit assertions skip-pass when `MAILPIT_URL`
> is unset (§5). Staff login to `/tmp/qa-staff.jar`.

### AC-1: FS-011.12 — opening a ticket delivers an autoresponse to the requester (Mailpit). [API-ONLY]
- Request: `curl -s -X POST http://localhost:3701/api/tickets -H 'Content-Type: application/json' -d '{"name":"Mia","email":"mia@example.com","subject":"Hi","message":"hello"}'`; then `curl -s http://localhost:3705/api/v1/messages | jq '.messages[] | {to:.To,subject:.Subject}'`.
- Verify: a message **To mia@example.com** is present; its subject/body reference the new ticket number and contain no literal `%{...}`.
- Status: [x]

### AC-2: FS-021.3 — an agent reply delivers a notification to the requester (Mailpit). [API-ONLY]
- Request: `curl -s -b /tmp/qa-staff.jar -X POST http://localhost:3701/api/staff/tickets/{id}/reply -H 'Content-Type: application/json' -d '{"body":"on it"}'`; then re-list Mailpit messages.
- Verify: a NEW message **To mia@example.com** appears whose body reflects the reply/ticket details, substituted (no literal tokens).
- Status: [x]

### AC-3: no mail is sent when the underlying write fails. [API-ONLY]
- Request: force a failing create — `curl -i -s -X POST .../api/tickets -H 'Content-Type: application/json' -d '{"name":"","email":"bad","subject":"","message":""}'` (validation error).
- Verify: HTTP 422; `curl -s http://localhost:3705/api/v1/messages | jq '.total'` is unchanged (no message); the dev mailbox is unchanged too (sends fire only after a successful commit).
- Status: [x]

### AC-4: with SMTP_HOST unset both events record intent in the dev mailbox (M1 fallback). [API-ONLY]
- Setup: restart the backend with **no `SMTP_HOST`** (`unset SMTP_HOST; cargo run -p api`); empty Mailpit; `cargo run -p tools --bin seed -- --reset`.
- Request: create a ticket (JSON) and post a staff reply; then `curl -s http://localhost:3701/api/dev/mailbox`.
- Verify: BOTH the autoresponse and the notification are recorded in the dev mailbox JSON; `curl -s http://localhost:3705/api/v1/messages | jq '.total'` is 0 (nothing delivered to Mailpit).
- Status: [x]

## Dependencies

- TS-M2-E1 (mailer), TS-M2-E2 (wrappers + templates), TS-M2-C1 (substitution), TS-M1-B2/C3 (create/reply paths).
