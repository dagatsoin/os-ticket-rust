# EPIC-M2-E — EPIC – Real outbound email basics

- **ID**: EPIC-M2-E
- **Type**: Epic
- **Parent**: M2
- **Labels**: Epic, M2
- **Column**: derived from children (`min(children.columns)`)

## Spec References

- FS-040 (FS-040.12 composition, FS-040.13 send wrappers, FS-040.10 packaged-default templates,
  BS-040.22 anti-loop headers, FS-040.11 substitution via Epic C)
- FS-011 (FS-011.12 new-ticket autoresponse)
- FS-021 (FS-021.3 staff-reply notification)

## Context

Milestone: M2. Needs Epic C (templates' `%{...}` bodies are substituted by the engine).
Replaces the M1 stub mailer with a real transport so the customer is reached *outside* the portal.

## Description

Implements a **real SMTP mailer behind the existing M1 mailer port** (DEVIATION D3 — single
env-driven transport `SMTP_HOST/PORT/USER/PASS/FROM`; **SMTP active when `SMTP_HOST` is set**,
otherwise the M1 stub mailer + `GET /api/dev/mailbox` is retained). Adds the **auto-reply** and
**notice** send wrappers with **anti-loop headers** (FS-040.13, BS-040.22), and ships
**packaged-default templates** (FS-040.10) for the **new-ticket autoresponse** (FS-011.12) and the
**staff-reply notification** (FS-021.3). All bodies are rendered through Epic C and sent **text-only**
(KL-040.1). Test infrastructure: a **Mailpit** container (SMTP **:3704**, web UI **:3705**) the
backend points at so emails are observable in-browser.

Business value: the first time the product reaches the customer in their own inbox — an
autoresponse confirming their ticket and a notification when an agent replies. This is the
"real email" half of the M2 promise and what makes M2 root AC-4 (client receives the reply email in
Mailpit) green.

## Acceptance Criteria

- (Epic — no ACs. Validated through its children's ACs and the M2 root E2E AC-4.)

## Checklist (children)

- [ ] TS-M2-E1 — SMTP mailer behind the mailer port (env-driven; stub kept when SMTP_HOST unset) + Mailpit infra
- [ ] TS-M2-E2 — auto-reply + notice wrappers (anti-loop headers) + packaged-default templates
- [ ] US-M2-4 — Client receives the reply by email (child: TS-M2-E3 wire autoresponse on create + notification on reply)

## Dependencies

- EPIC-M2-C (substitution engine). M1 mailer port + `GET /api/dev/mailbox`.
