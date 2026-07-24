# TS-M4-G1 — add(core): DEVIATION M4-D2 minimal write-side logging facility + event wiring

- **ID**: TS-M4-G1
- **Type**: Technical Story
- **Parent**: US-M4-G1
- **Labels**: Technical Story, M4
- **Scope**: small

## Context

**DEVIATION M4-D2 (RISK-3).** The FS-033 log viewer needs real data. This TS builds a minimal
`osTicket::log()`-equivalent that records `syslog` rows on notable admin/system events. Full FS-003
logging is NOT in scope — only a thin writer + a handful of call sites.

## Impact

- add(core): `ost_core::log(pool, type: LogType, title, detail, ip)` writing a `syslog` row —
  **best-effort** (never fails/propagates to the caller), gated by the `log_level` config threshold
  (Error/Warning/Debug). `ip` is the captured client IP (empty-string default acceptable).
- wire call sites — emit a log row on: **login success + login failure** (Warning/Debug), **admin
  CRUD create + delete** (department/group/staff/SLA/page → Debug), and **mailer send failure**
  (Error). Capture the client IP from Axum `ConnectInfo` / the `X-Forwarded-For` header (empty default ok).
- **extend** `POST /api/dev/seed-log` to accept an **optional `created` timestamp** (backdated rows for
  the grace-period purge tests); when omitted it stamps `now()`.

## Regressions

- Logging is best-effort and must never fail the operation it observes (a log-write error is swallowed).

## Acceptance Tests

### AC-1: M4-D2 — log() writes a syslog row. [API-ONLY]
- Setup: reseed; `POST /api/dev/purge-logs` to zero the table; admin session.
- Action: trigger an admin CRUD event — POST /api/staff/admin/groups (create a group).
- Request: `docker exec backend-db-1 psql -U postgres -d osticket_dev -c "SELECT count(*) FROM syslog"` → count increased by ≥1; the new row has the expected type/title.
- Status: [x] — VERIFIED live: POST /api/staff/admin/groups (200) → syslog count 22→23; newest row = `Debug | Group created`.

### AC-2: M4-D2 — log_level threshold suppresses below-threshold entries. [API-ONLY]
- Setup: admin session; set `log_level=Error` (POST /api/dev/seed-config {log_level:"Error"} or the settings endpoint).
- Action: trigger a Debug-level event → no new syslog row. Trigger an Error-level event (e.g. a mailer send failure path) → a row is written.
- Status: [x] — VERIFIED live: with log_level=Error a Debug admin-CRUD (group create, http 200) left syslog count unchanged (26→26, suppressed); after restoring log_level=Debug the same event wrote a row (26→27). Threshold gating confirmed; also covered by integration test `threshold_suppresses_below_level` (passes).

### AC-3: A failing log write does not fail the underlying operation. [API-ONLY]
- Setup: unit/integration test that forces the syslog INSERT to error (best-effort writer).
- Verify: the observed admin CRUD operation still returns 200/201 success; the log-write error is swallowed (logging is non-fatal).
- Status: [x] — VERIFIED: integration test `log_write_is_best_effort` passes (forces the syslog INSERT to error, asserts the operation still succeeds with no propagation). Code inspection confirms `ost_core::log` swallows every error path with `tracing::warn`.

### AC-4: dev seed-log inserts a dated (backdated) row. [API-ONLY]
- Request: POST /api/dev/seed-log {type:"Debug", title:"old", created:"2020-01-01T00:00:00Z"}.
- Expect: 201; the row is present stamped with that `created` date. Omitting `created` stamps `now()`.
- **INFRA NOTE**: extending `seed-log` with the optional `created` field is **owned by this TS**. Without it, the M4-G purge-sweep ACs (US-M4-G1 AC-6, TS-M4-G2 AC-5) cannot seed an over-age row.
- Status: [x] — VERIFIED live: POST /api/dev/seed-log {type:Debug,title:"old backdated",created:"2020-01-01T00:00:00Z"} → 201; row present stamped 2020-01-01 00:00:00+00. Omitting `created` stamps now() (confirmed in AC-1/AC-5 rows).

### AC-5: login success AND login failure both emit a syslog row. [API-ONLY]
- Setup: reseed; `POST /api/dev/purge-logs`.
- Action: POST /api/staff/login with a wrong password (failure), then with `admin`/`Admin123!` (success).
- Verify: `SELECT count(*) FROM syslog` shows rows for both the failed and the successful login attempt (subject to the `log_level` threshold).
- Status: [x] — VERIFIED live: wrong-password login (401) then valid login (200) → syslog +2; newest rows = `Warning | Staff login failed` and `Debug | Staff login`. Also covered by integration test `login_success_and_failure_logged` (passes).

## Test Infrastructure

- `.sqlx` cache updated; DB-backed tests read `TEST_DATABASE_URL`. Dev `seed-log`.

## Dependencies

- **EPIC-M4-PREP**: `syslog` table + `log_level` config key.
