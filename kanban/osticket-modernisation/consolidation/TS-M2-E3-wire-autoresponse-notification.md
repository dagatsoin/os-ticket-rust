# TS-M2-E3 — update(core): FS-011.12 autoresponse on create + FS-021.3 notification on reply

- **ID**: TS-M2-E3
- **Type**: Technical Story
- **Parent**: US-M2-4
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Wires the E2 wrappers/templates into the two ticket events so the customer actually receives mail.

## Impact

- update(core): on public ticket create, after commit, **send the new-ticket autoresponse** (auto-reply wrapper) to the requester (FS-011.12), rendered via Epic C.
- update(core): on staff reply, after commit, **send the reply notification** (notice wrapper) to the requester (FS-021.3), rendered via Epic C.
- Sends go through whichever transport E1 selected (SMTP when `SMTP_HOST` set; stub mailbox otherwise).

## Regressions

- Sends happen **only after DB commit** (M1 convention from TS-M1-C3 — no mail for a failed write). With `SMTP_HOST` unset, the recorded-intent dev-mailbox behaviour matches M1.

## Acceptance Tests

### AC-1: FS-011.12 — opening a ticket delivers an autoresponse to the requester (Mailpit). [API-ONLY]
- With Mailpit up, create a ticket as mia@example.com → a To-mia autoresponse appears in Mailpit, body substituted.
- Status: [ ]

### AC-2: FS-021.3 — an agent reply delivers a notification to the requester (Mailpit). [API-ONLY]
- Post a staff reply → a To-mia notification appears in Mailpit, body substituted.
- Status: [ ]

### AC-3: no mail is sent when the underlying write fails. [API-ONLY]
- Force a create/reply failure (e.g. invalid payload) → no message in Mailpit / dev mailbox.
- Status: [ ]

### AC-4: with SMTP_HOST unset both events record intent in the dev mailbox (M1 fallback). [API-ONLY]
- Restart with no SMTP_HOST; create + reply → both recorded in `GET /api/dev/mailbox`, nothing in Mailpit.
- Status: [ ]

## Dependencies

- TS-M2-E1 (mailer), TS-M2-E2 (wrappers + templates), TS-M2-C1 (substitution), TS-M1-B2/C3 (create/reply paths).
