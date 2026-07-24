# TS-M2-E1 — add(infra): FS-040.12 SMTP mailer behind the mailer port + Mailpit

- **ID**: TS-M2-E1
- **Type**: Technical Story
- **Parent**: EPIC-M2-E
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Replaces the M1 stub mailer with a real SMTP transport implementing the existing mailer port, plus
the Mailpit test container. **DEVIATION D3 (pinned):** a single env-driven transport; **SMTP active
when `SMTP_HOST` is set**, otherwise the M1 stub mailer + `GET /api/dev/mailbox` is retained.

## Impact

- add(infra): an `SmtpMailer` implementing the M1 `Mailer` port (TS-M1-A4b), built on **`lettre`** (tokio1 + rustls, custom-header support — ROADMAP M2 Decisions §4), configured from `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM`.
- update(wiring): at startup, if `SMTP_HOST` is set → bind `SmtpMailer`; else bind the M1 stub mailer (dev mailbox preserved).
- add(infra): a **`docker-compose.yml` at the repo root** (ROADMAP M2 Decisions §5) with a `mailpit` service (image `axllent/mailpit`, ports `3704:1025` + `3705:8025`); document `docker compose up -d mailpit` in CLAUDE.md Quick Start + the port registry.
- add(dev-dep): **`reqwest` as a DEV-dependency** for asserting on delivered messages via the Mailpit API (§4).
- test gating: Mailpit-dependent cargo tests are **skip-pass when `MAILPIT_URL` is unset** (same pattern as `TEST_DATABASE_URL`, §5).
- text-only send (KL-040.1): body decoded to plain text, single text MIME part.

## Regressions

- With `SMTP_HOST` unset, every M1 mail path (stub mailbox) must behave exactly as before. Re-run any M1 sweep that reads `/api/dev/mailbox`.

## Acceptance Tests

> Setup: `docker compose up -d mailpit`; the integration tests target the running Mailpit via
> `MAILPIT_URL=http://localhost:3705` and **skip-pass when it is unset** (§5). Run:
> `MAILPIT_URL=http://localhost:3705 cargo test -p api smtp_mailer`. Empty the inbox before each case
> with `curl -X DELETE http://localhost:3705/api/v1/messages`.

### AC-1: D3 — with SMTP_HOST set, a send is delivered to Mailpit. [API-ONLY]
- Setup: bind the `SmtpMailer` with `SMTP_HOST=localhost SMTP_PORT=3704 SMTP_FROM=support@example.com`; empty Mailpit.
- Request: trigger a send through the mailer, then `curl -s http://localhost:3705/api/v1/messages | jq '.total'`.
- Verify: `.total` is 1 — the message is delivered to Mailpit.
- Status: [x]

### AC-2: D3 — with SMTP_HOST unset, a send is recorded in the stub dev mailbox, not delivered. [API-ONLY]
- Setup: bind the mailer with **no `SMTP_HOST`** (stub path); empty Mailpit.
- Request: trigger a send, then `curl -s http://localhost:3701/api/dev/mailbox` and `curl -s http://localhost:3705/api/v1/messages | jq '.total'`.
- Verify: the send is recorded in the dev mailbox JSON; Mailpit `.total` is 0 (nothing delivered).
- Status: [x]

### AC-3: FS-040.12 — the delivered message is plain-text with the configured From. [API-ONLY]
- Setup: AC-1's SMTP config; one delivered message.
- Request: `curl -s http://localhost:3705/api/v1/messages | jq '.messages[0].ID'` then `curl -s http://localhost:3705/api/v1/message/{ID}`.
- Verify: the message has a `text/plain` part (no HTML part, KL-040.1), and `From` equals `SMTP_FROM` (`support@example.com`).
- Status: [x]

## Test Infrastructure

- **Mailpit** via the repo-root `docker-compose.yml` (§5), SMTP **3704** / web **3705** (staging 3714/3715), service `mailpit`, started with `docker compose up -d mailpit`. Mailpit-dependent tests skip-pass when `MAILPIT_URL` is unset.

## Dependencies

- TS-M1-A4b (mailer port + stub + dev mailbox).
