# M6 — MILESTONE – External API & Cron

- **ID**: M6
- **Type**: Milestone (root, stub)
- **Status**: Backlog stub — not yet broken down.

## Spec References

- FS-043 (IP-bound API-key management, HTTP API dispatcher: ticket-create + remote cron routes, scheduled-maintenance cron subsystem)

## Description

Exposes the system to external integrations: IP-bound API-key management, the HTTP API
dispatcher (ticket-create as a fourth adapter over the shared core, plus remote cron trigger),
and the scheduled-maintenance cron subsystem (mail fetch, overdue checks, etc.).

## Acceptance Criteria (to be defined as a browser-only E2E during consolidation)

- Status: [ ] (placeholder) Admin issues an API key bound to an IP; an API call from that IP creates a ticket visible in the queue; the cron route triggers scheduled maintenance.

## Notes

- Not refined. Requires spec-writer dependency-graph confirmation before consolidation.
- API ticket-create reuses the M1 ticket core (TS-M1-B1) as a fourth channel adapter.
