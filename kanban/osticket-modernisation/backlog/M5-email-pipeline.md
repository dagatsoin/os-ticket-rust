# M5 — MILESTONE – Email Pipeline

- **ID**: M5
- **Type**: Milestone (root, stub)
- **Status**: Backlog stub — not yet broken down.

## Spec References

- FS-041 (inbound email-to-ticket pipeline: IMAP/POP3 poll, MTA pipe, HTTP email-post → parse → thread → create/append)
- FS-042 (ticket filters / banlist / inbound routing rule engine)
- FS-040 (email accounts + templates — full, beyond M2's basics)

## Description

Adds email as a first-class ticket channel: inbound mail (IMAP/POP3 polling, MTA pipe, HTTP
email-post) converges on the **shared ticket create-and-append core** built in M1, with the
ordered ticket-filter rule engine and ban list governing rejection/mutation/routing of
inbound messages. Email becomes a thin adapter over the same core as the web form and staff UI.

## Acceptance Criteria (to be defined as a browser-only E2E during consolidation)

- Status: [ ] (placeholder) An inbound email creates a ticket visible in the staff queue; a reply email threads onto the existing ticket; a banned sender is rejected.

## Notes

- Not refined. Requires spec-writer dependency-graph confirmation before consolidation.
- Reuses the M1 ticket core (TS-M1-B1) as the email-channel adapter target.
