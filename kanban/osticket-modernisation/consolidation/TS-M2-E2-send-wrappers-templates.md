# TS-M2-E2 — add(domain): FS-040.13 auto-reply/notice wrappers + FS-040.10 packaged templates

- **ID**: TS-M2-E2
- **Type**: Technical Story
- **Parent**: EPIC-M2-E
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

The message-class send wrappers (anti-loop headers) and the two packaged-default templates M2 needs,
rendered through the Epic-C substitution engine.

## Impact

- add(domain): `send_autoreply(...)` applying the **autoreply** option set — `Precedence: auto_reply`, `X-Autoreply: yes`, `X-Auto-Response-Suppress: DR, RN, OOF, AutoReply`, `Auto-Submitted: auto-replied` (BS-040.22).
- add(domain): `send_notice(...)` applying the **notice** option set — `X-Auto-Response-Suppress: OOF, AutoReply`, `Auto-Submitted: auto-generated` (BS-040.22).
- add(templates): packaged-default templates (FS-040.10) for **new-ticket autoresponse** (FS-011.12) and **staff-reply notification** (FS-021.3) — subject + body with `%{...}` tokens, resolved through TS-M2-C1.

## Regressions

- None — wrappers + templates are additive; wiring to events is TS-M2-E3.

## Acceptance Tests

### AC-1: BS-040.22 — an auto-reply send carries the autoreply anti-loop headers. [API-ONLY]
- send_autoreply via SMTP → Mailpit message carries `Precedence: auto_reply`, `X-Autoreply`, `X-Auto-Response-Suppress`, `Auto-Submitted: auto-replied`.
- Status: [ ]

### AC-2: BS-040.22 — a notice send carries the notice anti-loop headers. [API-ONLY]
- send_notice via SMTP → Mailpit message carries `X-Auto-Response-Suppress: OOF, AutoReply`, `Auto-Submitted: auto-generated`.
- Status: [ ]

### AC-3: FS-040.10/.11 — the autoresponse + notification templates render with tokens substituted. [API-ONLY]
- Render each template for a seeded ticket → subject + body contain the ticket number, no literal `%{...}`.
- Status: [ ]

## Dependencies

- TS-M2-E1 (mailer + Mailpit), TS-M2-C1 (substitution engine).
