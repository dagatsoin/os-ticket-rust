# TS-M2-prep — add(route): dev reset endpoint purging accumulated test tickets

- **ID**: TS-M2-prep-dev-reset
- **Type**: Technical Story (orphan, dev/test infrastructure)
- **Parent**: — (M2 prep; not yet attached to a User Story)
- **Labels**: Technical Story, M2
- **Status**: Backlog stub — not yet refined.
- **Scope**: small

## Context

QA observation from the M1 E2E sweep (root E2E 5/5, playbook 11/11 green, 2026-06-11):
the seed binary (`cargo run -p tools --bin seed`, TS-M1-A3) restores the seeded department,
group, and `agent` staff account, but it **never purges tickets**. As a result the staff
Open-tickets queue accumulates tickets across repeated QA runs (every Flow 1.3 / root AC-1
submission, plus every `POST /api/dev/seed-ticket` injection, leaves a residual ticket).

This does not break the M1 journey today (it asserts the *carried-over* ticket is present,
newest-first), but it makes the queue noisier on each sweep and will eventually undermine
"the queue lists only the expected tickets" style assertions. A clean reset is needed for
deterministic E2E sweeps going forward.

## Impact

- add(route): a **dev-only, env-gated** reset endpoint (e.g. `POST /api/dev/reset`) that purges
  accumulated tickets + thread entries (and dependent rows) back to the seeded baseline — OR
- update(tooling): a **`--purge` / `--reset` flag on the seed binary** that truncates ticket /
  thread tables before re-seeding the dept/group/staff baseline.
- Decide one mechanism during consolidation (endpoint vs. seed flag); both must be **disabled in
  production** (same env gate as `POST /api/dev/seed-ticket`, TS-M1-D1).

## Regressions

- Must not delete the seeded department / group / `agent` account — only ticket-scoped data.
- The existing `cargo run -p tools --bin seed` idempotency contract (writes only `osticket_dev`)
  must be preserved.

## Acceptance Tests

- it should purge all tickets + thread entries so the Open queue returns empty after reset.
- it should preserve the seeded department, group, and `agent` staff account after reset.
- it should be refused (404/403) when the dev env gate is off (production-safe).

## Notes

- Not refined. Attach to M2 consolidation; confirm endpoint-vs-flag mechanism with the team.
- Enables clean repeated E2E sweeps for M2 and beyond.
