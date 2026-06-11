# M2 — MILESTONE – Attachments, Canned Responses & Email Basics

- **ID**: M2
- **Type**: Milestone (root, stub)
- **Status**: Backlog stub — not yet broken down.

## Spec References

- FS-022 (canned responses + content-addressed file store / attachments)
- FS-040 (outbound mail — real autoresponse/alert emails, replacing the M1 stub mailer)

## Description

Builds on M1's round-trip: tickets and replies can carry **attachments** (the chunked,
content-addressed DB file store), agents get a **canned-response** library with variable
substitution, and the stub mailer is replaced by **real outbound mail basics** (autoresponse
on ticket open, reply notifications) using the FS-040 template/token grammar.

## Acceptance Criteria (to be defined as a browser-only E2E during consolidation)

- Status: [ ] (placeholder) Client opens a ticket with an attachment; agent replies using a canned response; client receives/views the reply and can download the attachment.

## Notes

- Not refined. Requires spec-writer dependency-graph confirmation before consolidation.
