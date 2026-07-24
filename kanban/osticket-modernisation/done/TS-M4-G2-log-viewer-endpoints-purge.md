# TS-M4-G2 — add(route): FS-033.2–.8 log viewer endpoints + purge sweep function

- **ID**: TS-M4-G2
- **Type**: Technical Story
- **Parent**: US-M4-G1
- **Labels**: Technical Story, M4
- **Scope**: medium

## Context

Backend for the log viewer: list with filter/sort/pagination, single-record detail, bulk deletion,
and the grace-period purge sweep function (exposed via a dev/manual trigger; cron trigger is M6).

## Impact

- add(route): `GET /api/staff/admin/logs` (filter `type` + date range `from`/`to`; sort; paginate;
  **newest-first default order**) — admin-gated.
- add(route): `GET /api/staff/admin/logs/:id` (single-record detail, FS-033.8 content AJAX).
- add(route): `POST /api/staff/admin/logs/delete` (bulk manual deletion, FS-033.6).
- add(service): a **real** `purge_logs()` grace-period sweep — deletes rows older than
  `log_graceperiod` **months** (BS-033.6); **no-op** when `log_graceperiod` is unset / zero /
  non-numeric. KL-033.7 modernised.
- **repoint** `POST /api/dev/purge-logs` to call the grace-period sweep (it currently TRUNCATEs the
  whole table — that behaviour is replaced by the over-age sweep; keep a separate reset path only if a
  full clear is still needed for setup). Cron trigger deferred to M6.

## Regressions

- None; read + delete only over `syslog`.

## Acceptance Tests

### AC-1: FS-033.2 — filter by type + date span. [API-ONLY]
- Setup: admin session; purge; seed rows of ≥2 types with differing dates (`POST /api/dev/seed-log`).
- Request: GET http://localhost:3701/api/staff/admin/logs?type=Error&from=<d1>&to=<d2> → only Error rows within the span.
- Status: [x] — VERIFIED live: GET /api/staff/admin/logs?type=Error&from=2024-03-01&to=2024-03-31 → exactly the 2 Error rows in that span (G2 err A/B), Warning row outside the span and other types excluded. Non-admin `agent` GET → 403.

### AC-2: FS-033.4/.5 — sort + paginate. [API-ONLY]
- Setup: admin session; seed enough rows to exceed one page.
- Request: GET /api/staff/admin/logs?sort=created&page=2 → rows ordered by created and paged (page 2 differs from page 1). Default order (no `sort`) is **newest-first**.
- Status: [x] — VERIFIED live: sort=created&order=asc returns rows in created-ascending order; pagination page=1 (25 ids) vs page=2 (5 ids) are disjoint sets. Default (no sort) = newest-first by `id DESC` (deliberate design per logs.rs; with organically-inserted rows id-order == recency).

### AC-3: FS-033.8 — single record detail. [API-ONLY]
- Setup: admin session; a known seeded row id.
- Request: GET /api/staff/admin/logs/{id} → 200 with the full body/detail; GET an unknown id → 404.
- Status: [x] — VERIFIED live: GET /api/staff/admin/logs/31 → 200 with full `log` body ("errbody2") + type/title/created/ip; GET /api/staff/admin/logs/99999999 → 404 "Log entry not found".

### AC-4: FS-033.6 — bulk deletion. [API-ONLY]
- Setup: admin session; two known seeded row ids.
- Request: POST /api/staff/admin/logs/delete {ids:[id1,id2]} → 200; a subsequent GET no longer lists them.
- Status: [x] — VERIFIED live: POST /api/staff/admin/logs/delete {ids:[31,30]} → 200 {affected:2}; subsequent filtered GET returns 0 remaining rows for that span.

### AC-5: BS-033.6 — purge sweep deletes over-age rows only. [API-ONLY]
- Setup: admin session; set `log_graceperiod` to a non-zero month count (e.g. 12); seed an over-age row (`seed-log {created:"2020-01-01..."}`) and a recent row.
- Request: invoke the grace-period sweep via `POST /api/dev/purge-logs` → the over-age row is gone, the recent row is kept.
- Verify: with `log_graceperiod` unset / zero / non-numeric, the sweep is a **no-op** (nothing deleted).
- **INFRA NOTE**: owned by this TS: (a) implement `purge_logs()` as a grace-period-gated, months-based delete with the unset/zero/non-numeric no-op guard; (b) **repoint** `/api/dev/purge-logs` at it (it currently TRUNCATEs everything). Depends on TS-M4-G1 extending `seed-log` with `created` (to seed the over-age row).
- Status: [x] — VERIFIED live: with log_graceperiod=12 → POST /api/dev/purge-logs deleted over-age rows (2020 row gone), recent row kept. No-op guard confirmed for log_graceperiod=0, "" (empty), and "abc" (non-numeric) — over-age row retained (deleted:0) in each case. Also covered by integration test `purge_sweep_over_age_only` (passes).

## Test Infrastructure

- Admin account; dev `seed-log`, `purge-logs`. `.sqlx` cache updated.

## Dependencies

- **EPIC-M4-PREP**: `syslog` + `log_graceperiod`. **TS-M4-G1**: rows to view. **TS-M4-A0**: admin gate.
- **Forward (M6)**: the cron scheduler calls `purge_logs()`.
