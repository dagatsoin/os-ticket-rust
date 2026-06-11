# US-M2-4 — Email – Client receives the reply by email

- **ID**: US-M2-4
- **Type**: User Story
- **Parent**: EPIC-M2-E
- **Labels**: User Story, M2
- **Scope**: small

## Spec References

- FS-040 (FS-040.12 composition, FS-040.13 wrappers, FS-040.10 packaged-default templates,
  BS-040.22 anti-loop headers, FS-040.11 substitution via Epic C)
- FS-011 (FS-011.12 new-ticket autoresponse)
- FS-021 (FS-021.3 staff-reply notification)

## Context

Epic: EPIC-M2-E — Real outbound email basics.
The "real email" half of the M2 demo (root AC-4): the product reaches the customer in their own
inbox — an autoresponse when they open a ticket and a notification when an agent replies — both
observable in Mailpit.

## Description

As a customer, when I open a ticket I get an email confirming it, and when an agent replies I get an
email notifying me, each personalised with my ticket details — observable in the Mailpit inbox.

## Impact

- Backend (auto-reply send on public create; notice send on staff reply; SMTP transport via env; packaged-default templates rendered through the substitution engine)
- Test infra (Mailpit SMTP :3704 / web UI :3705)
- Browser (Mailpit web UI is the in-browser oracle)

## Business Rules

- FS-011.12: opening a ticket triggers a **new-ticket autoresponse** email to the requester.
- FS-021.3: an agent reply triggers a **notification** email to the requester.
- FS-040.11: subject + body are `%{...}`-substituted for the ticket (no literal tokens in the sent mail).
- BS-040.22: auto-reply/notice mails carry anti-loop headers (`Precedence`, `X-Auto-Response-Suppress`, `Auto-Submitted`).
- D3: SMTP active when `SMTP_HOST` set; otherwise the stub mailer + `GET /api/dev/mailbox` records (does not send).

## Regressions

- When `SMTP_HOST` is unset, behaviour must fall back to the M1 stub mailer + dev mailbox exactly as before (M1 sweeps that read `/api/dev/mailbox` must still pass).

## Acceptance Criteria

### AC-1: Opening a ticket sends an autoresponse email, visible in Mailpit, addressed to the requester. [BROWSER]
- Setup: `cargo run -p tools --bin seed -- --reset`; Mailpit up; backend `SMTP_HOST=localhost SMTP_PORT=3704`.
- Navigate: http://localhost:3702/open → submit a ticket as "mia@example.com" (note the ticket number).
- Navigate: http://localhost:3705 (Mailpit web UI).
- Verify: the inbox lists a message **To mia@example.com** whose subject/body reference the ticket number; **no literal `%{...}`** remains in the rendered body.
- Status: [ ]

### AC-2: An agent reply sends a notification email, visible in Mailpit, addressed to the requester. [BROWSER]
- Navigate: log in agent / Agent123!; open the ticket; post a reply.
- Navigate: http://localhost:3705 (Mailpit).
- Verify: a NEW message **To mia@example.com** appears whose body reflects the reply/ticket details (substituted, no literal tokens).
- Status: [ ]

### AC-3: The sent mail carries anti-loop headers. [API-ONLY]
- Setup: trigger AC-1 (autoresponse) with Mailpit up.
- Request: query the Mailpit API for the captured message headers (e.g. `GET http://localhost:3705/api/v1/messages` then the message detail).
- Expect: the autoresponse carries `Precedence: auto_reply` (or `bulk`), `X-Auto-Response-Suppress`, and `Auto-Submitted` headers (BS-040.22); the notice carries the notice-class headers.
- Status: [ ]

### AC-4: With SMTP_HOST unset, sends fall back to the stub mailer (no Mailpit delivery). [API-ONLY]
- Setup: restart the backend with `SMTP_HOST` unset; open a ticket.
- Request: `GET http://localhost:3701/api/dev/mailbox`.
- Expect: the intended autoresponse is RECORDED in the dev mailbox (M1 behaviour preserved) and NOT delivered to Mailpit.
- Status: [ ]

## Checklist (children)

- [ ] TS-M2-E3 — Wire autoresponse on create + notification on reply (render via Epic C, send via Epic E mailer)

## Test Infrastructure

- **Mailpit** container, SMTP **:3704** / web UI **:3705** (staging +10). Backend env `SMTP_HOST=localhost SMTP_PORT=3704` to exercise D3; Mailpit web UI (AC-1/AC-2) and Mailpit API (AC-3) are the oracles.
- Stub-mailbox `GET /api/dev/mailbox` (M1) is the AC-4 oracle when SMTP is off.

## Dependencies

- TS-M2-E1 (SMTP mailer + Mailpit infra), TS-M2-E2 (wrappers + templates), TS-M2-C1 (substitution). M1 mailer port + create/reply paths.
