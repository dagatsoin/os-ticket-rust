# EPIC-M1-A — EPIC – Foundation & Workspace Setup

- **ID**: EPIC-M1-A
- **Type**: Epic
- **Parent**: M1
- **Labels**: Epic, M1
- **Column**: derived from children

## Spec References

- FS-091 (logical data model — subset)
- FS-003 (crypto / validation / formatting infrastructure)
- FS-001 (bootstrap / config / session / CSRF / realm gates)
- FS-002 (staff auth primitives — password hashing, DB sessions)

## Context

Milestone: M1 — First Ticket Round-Trip.
Before any user-facing flow can be built, the modernised stack needs a skeleton: a Cargo
workspace, an Axum HTTP server with health + middleware, a PostgreSQL schema (the M1 table
subset), seed fixtures (one dept/group/staff), and a React/Vite app shell that can talk to
the API. This epic delivers no end-user feature on its own; it is the platform every US in
EPIC-M1-B builds on.

## Description

Stand up the runnable foundation: backend workspace + server skeleton, database migrations
for the M1 schema subset, seed data, shared infrastructure (validation, password hashing,
session/CSRF, realm gating, a stubbed mailer port), and the frontend scaffold with routing
and an API client.

## Business value

Provides the demonstrable platform (a server that boots, a DB that migrates, a UI that loads)
that unblocks all feature work and reduces integration risk by validating the toolchain early.

## Acceptance Criteria

> No ACs — intermediate parent; verification owned by TS-M1-A5 (app shell renders) and TS-M1-A1 (health endpoint / FE→BE→DB connectivity). Column derived from children.

## Children (column derived: min of these)

- [ ] TS-M1-A1 — Cargo workspace + Axum server skeleton + health endpoint
- [ ] TS-M1-A2 — PostgreSQL migrations for the M1 schema subset (SQLx)
- [ ] TS-M1-A3 — Seed fixtures (dept + group + staff) + reference enums
- [ ] TS-M1-A4a — Validation, sanitization & password hashing utils (pure functions)
- [ ] TS-M1-A4b — Sessions, CSRF, realm gates & stub mailer (+ `GET /api/dev/mailbox`)
- [ ] TS-M1-A5 — React + Vite + TS app scaffold (routing, MUI theme, MobX store, API client)

## Dependencies

- TS-M1-A2 depends on TS-M1-A1 (workspace exists). TS-M1-A3 depends on TS-M1-A2 (schema exists)
  and on TS-M1-A4a (argon2id hashing for the seeded password).
- TS-M1-A4a and TS-M1-A4b both depend on TS-M1-A1; TS-M1-A4b pairs with TS-M1-A2 for the
  `session` table. (Split from the original TS-M1-A4, which was over-scoped.) TS-M1-A4a unblocks
  B1/B2 (validation/sanitize) and A3 (hashing); TS-M1-A4b unblocks C1/C3/D1 (sessions, CSRF,
  realm gates, permission gate, stub mailer + `GET /api/dev/mailbox`).
- TS-M1-A5 is independent and can run in parallel.
