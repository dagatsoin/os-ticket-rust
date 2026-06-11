# TS-M2-E2 — add(domain): FS-040.13 auto-reply/notice wrappers + FS-040.10 packaged templates

- **ID**: TS-M2-E2
- **Type**: Technical Story
- **Parent**: EPIC-M2-E
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

The message-class send wrappers (anti-loop headers) and the two packaged-default templates M2 needs,
rendered through the Epic-C substitution engine. The anti-loop headers are set via `lettre`'s
custom-header support (ROADMAP M2 Decisions §4); Mailpit-dependent assertions skip-pass when
`MAILPIT_URL` is unset (§5).

## Impact

- add(domain): `send_autoreply(...)` applying the **autoreply** option set — `Precedence: auto_reply`, `X-Autoreply: yes`, `X-Auto-Response-Suppress: DR, RN, OOF, AutoReply`, `Auto-Submitted: auto-replied` (BS-040.22).
- add(domain): `send_notice(...)` applying the **notice** option set — `X-Auto-Response-Suppress: OOF, AutoReply`, `Auto-Submitted: auto-generated` (BS-040.22).
- add(templates): packaged-default templates (FS-040.10) for **new-ticket autoresponse** (FS-011.12) and **staff-reply notification** (FS-021.3) — subject + body with `%{...}` tokens, resolved through TS-M2-C1.

## Regressions

- None — wrappers + templates are additive; wiring to events is TS-M2-E3.

## Acceptance Tests

> Setup: `docker compose up -d mailpit`; tests target Mailpit via `MAILPIT_URL` and skip-pass when unset (§5).
> Run: `MAILPIT_URL=http://localhost:3705 cargo test -p ost_core send_wrappers`. Empty the inbox before
> each header case (`curl -X DELETE http://localhost:3705/api/v1/messages`); read headers via
> `curl -s http://localhost:3705/api/v1/message/{ID}/headers`.

### AC-1: BS-040.22 — an auto-reply send carries the autoreply anti-loop headers. [API-ONLY]
- Run: `send_autoreply(...)` via the SMTP mailer to Mailpit.
- Verify: the captured message carries `Precedence: auto_reply`, `X-Autoreply: yes`, `X-Auto-Response-Suppress: DR, RN, OOF, AutoReply`, and `Auto-Submitted: auto-replied`.
- Status: [ ]

### AC-2: BS-040.22 — a notice send carries the notice anti-loop headers. [API-ONLY]
- Run: `send_notice(...)` via the SMTP mailer to Mailpit.
- Verify: the captured message carries `X-Auto-Response-Suppress: OOF, AutoReply` and `Auto-Submitted: auto-generated`.
- Status: [ ]

### AC-3: FS-040.10/.11 — the autoresponse + notification templates render with tokens substituted. [API-ONLY]
- Run: render both packaged-default templates for a seeded ticket context (`cargo test -p ost_core send_wrappers::templates`).
- Verify: each template's subject + body contain the ticket's number and no literal `%{...}` remains.
- Status: [ ]

## Dependencies

- TS-M2-E1 (mailer + Mailpit), TS-M2-C1 (substitution engine).
