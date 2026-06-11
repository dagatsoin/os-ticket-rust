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

- add(infra): an `SmtpMailer` implementing the M1 `Mailer` port (TS-M1-A4b), configured from `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM`.
- update(wiring): at startup, if `SMTP_HOST` is set → bind `SmtpMailer`; else bind the M1 stub mailer (dev mailbox preserved).
- add(infra): a **Mailpit** service in docker-compose — SMTP **3704** / web UI **3705** (staging +10 → 3714/3715); document in CLAUDE.md + port registry.
- text-only send (KL-040.1): body decoded to plain text, single text MIME part.

## Regressions

- With `SMTP_HOST` unset, every M1 mail path (stub mailbox) must behave exactly as before. Re-run any M1 sweep that reads `/api/dev/mailbox`.

## Acceptance Tests

### AC-1: D3 — with SMTP_HOST set, a send is delivered to Mailpit. [API-ONLY]
- Backend `SMTP_HOST=localhost SMTP_PORT=3704`; trigger a send → the message appears in Mailpit (`GET http://localhost:3705/api/v1/messages`).
- Status: [ ]

### AC-2: D3 — with SMTP_HOST unset, a send is recorded in the stub dev mailbox, not delivered. [API-ONLY]
- Backend with no `SMTP_HOST`; trigger a send → recorded in `GET /api/dev/mailbox`, nothing in Mailpit.
- Status: [ ]

### AC-3: FS-040.12 — the delivered message is plain-text with the configured From. [API-ONLY]
- Inspect the Mailpit message → text/plain body, `From` = `SMTP_FROM`.
- Status: [ ]

## Test Infrastructure

- **Mailpit** container, SMTP **3704** / web **3705** (staging 3714/3715). Compose service `mailpit`.

## Dependencies

- TS-M1-A4b (mailer port + stub + dev mailbox).
