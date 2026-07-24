# Security Audit Log

Open security findings raised during pre-QA review. Entries are appended, never
overwritten. When a finding is fixed, update its **Status** to `RESOLVED` with a date.

---

## EPIC-M4-A / EPIC-M4-B — 2026-07-24

**Status:** RESOLVED (2026-07-24)
**Compliance:** none declared (general OWASP + auth/authz correctness)
**Branch:** 2-modernisation

### Findings

- [WARN-1] `backend/api/src/config.rs:29-45` + `backend/api/src/dev.rs` (all handlers) —
  Dev endpoints fail **open** by default. `AppEnv` defaults to `Development`
  (`DEFAULT_APP_ENV = "development"`, and `from_str_lossy` treats anything not exactly
  `"production"` as Development). The dev endpoints have **no authentication** and include
  destructive / privilege-granting operations: `reset-tickets` (`DELETE FROM ticket`),
  `reset-staff`, `reset-groups`, `purge-logs`, and `seed-staff` (which accepts
  `isadmin: true` with an arbitrary password). If `APP_ENV` is unset or misspelled in a
  production deploy, all of these become reachable and unauthenticated. The task brief
  referenced an `ALLOWED_RESET_DBS` guard for destructive endpoints; no such guard exists —
  gating is solely the single `APP_ENV` variable.
- [WARN-2] `backend/api/src/admin_staff.rs:752-767` (`mass_staff` lock/delete) — the
  last-active-administrator guard reads the remaining-admin count on the **pool**, outside
  the transaction that performs the lock/delete (the delete opens its own separate
  transaction at line 818; lock/enable run as standalone statements). This leaves a TOCTOU
  window: two concurrent mass actions targeting different admins could each pass the count
  check and collectively remove the last active admin. The single-record `update_staff` path
  does this correctly (`SELECT ... FOR UPDATE` + in-transaction count, lines 519-552).

### Recommendation

- WARN-1: Make dev endpoints fail **closed** — default `AppEnv` to `Production` when `APP_ENV`
  is unset, and/or gate the destructive/admin-seeding endpoints behind an explicit secondary
  opt-in (e.g. an `ALLOWED_RESET_DBS` allowlist checked against the connected database name),
  so a missing env var can never expose them.
- WARN-2: Perform the remaining-active-admin count and the lock/delete mutation inside one
  transaction, taking `SELECT ... FOR UPDATE` locks on the affected admin rows (mirroring the
  `update_staff` pattern) to close the TOCTOU window.

### Resolution — 2026-07-24

- **[WARN-1] RESOLVED** — `backend/api/src/config.rs`: `DEFAULT_APP_ENV` is now `"production"`
  and `AppEnv::from_str_lossy` fails **safe** — only the explicit values `development` / `test`
  enable dev endpoints; unset/empty/misspelled ⇒ `Production`, so a missing `APP_ENV` no longer
  exposes the unauthenticated dev surface. Dev/QA set `APP_ENV=development` explicitly (already in
  `backend/.env`). Unit tests: `config::tests::app_env_fails_safe_to_production` +
  `dev_endpoints_enabled_only_in_development`.
- **[WARN-2] RESOLVED** — `backend/api/src/admin_staff.rs` (`mass_staff`): the last-active-admin
  count now runs INSIDE the lock/delete transaction, after `SELECT ... FOR UPDATE` on the affected
  active-admin rows (mirroring `update_staff`); `delete_staff_ops` runs on that same transaction.
  Concurrent mass actions targeting the same admin serialize on the row lock, closing the TOCTOU
  window. Integration test: `mass_lock_preserves_last_active_admin_in_txn`.

---

## EPIC-M4-D / EPIC-M4-F / EPIC-M4-G / EPIC-M4-H — 2026-07-24

**Status:** RESOLVED (2026-07-24)
**Compliance:** none declared (general OWASP + auth/authz correctness)
**Branch:** 2-modernisation

### Findings

- [WARN-3] `backend/api/src/auth/routes.rs:90-102` + `backend/core/src/log.rs:114-144` —
  Unbounded, attacker-triggerable audit-log growth (disk-fill DoS). Every failed staff
  login writes a `Warning` `syslog` row (`Unknown username` / `Inactive account` /
  `Bad password`), and the `staff_login` endpoint is unauthenticated with no rate limiting.
  The best-effort `log()` writer has no volume cap, and the only retention mechanism
  (`purge_logs`, `BS-033.6`) is a grace period expressed in **months** and runs only when
  manually triggered (cron lands in M6). An anonymous attacker can therefore flood the
  `syslog` table with one row per login attempt and grow it without bound between purges.
  The default `log_level` is Debug (3), so `Warning` rows persist by default. The stored
  values themselves are safe (bound params; the attacker-controlled username is stored, not
  interpolated, and no password is ever logged), so this is availability-only, not injection
  or secret exposure.

### Recommendation

- WARN-3: Rate-limit the login endpoint (or throttle/deduplicate failed-login syslog writes),
  and/or add a row-count ceiling on the `syslog` table with oldest-row eviction so
  best-effort logging cannot be driven to fill disk between grace-period purges. At minimum,
  document the exposure and ensure the M6 cron purge runs frequently with a sane grace period.

### Resolution — 2026-07-24

- **[WARN-3] RESOLVED** — `backend/core/src/log.rs` + `backend/api/src/auth/routes.rs`: added a
  per-IP fixed-window throttle with coalescing (`note_failed_login` / `failed_login_decision`,
  `FAILED_LOGIN_MAX_PER_WINDOW=10` writes per `FAILED_LOGIN_WINDOW_SECS=60`). `staff_login` routes
  all three failed-login paths through `log_failed_login`, which writes at most the per-window cap
  of `Warning` rows per client IP; attempts over the cap are dropped but counted and reported on the
  next written row (`(+N suppressed attempt(s))`), so brute force can no longer grow `syslog`
  without bound while genuine failed-login visibility is preserved. The throttle map self-prunes
  stale keys (bounded memory under a distributed flood). Unit tests:
  `failed_login_throttle_caps_and_coalesces` + `note_failed_login_bounds_burst`.

### Informational (not blocking)

- `backend/api/src/pages.rs:543-566` (`public_page`, unauthenticated) correctly restricts
  serving to `type='other' AND isactive=true` rows, matches the slug in Rust (no SQL slug
  function, constant parameter-free query — no injection), returns a plain 404 for any
  miss (no existence oracle, no id-based access, no path traversal). It returns the
  admin-authored `body` HTML verbatim in JSON. That body is a stored-HTML surface, but it is
  **admin-authored only** (create/edit are admin-gated), so it is a trusted-author CMS field
  rather than a user-injection XSS vector. The React client that renders it must still treat
  it as HTML content deliberately (sanitise or scope) — flagged for the frontend renderer,
  not a backend defect.
- `backend/api/src/dev.rs:958-959` (`seed_search_ticket`, pre-existing M3 dev endpoint,
  outside this M4 change set) interpolates the caller-supplied `status` string into an
  `UPDATE ticket ... SET status = '{}'` statement without escaping — a genuine SQL-injection
  sink. It is dev-only (404 when `dev_endpoints_enabled()` is false), so it inherits the
  WARN-1 fail-open caveat; it is not part of the D/F/G/H surface reviewed here but is
  recorded because it was observed. Recommend migrating it to a bound parameter.

### Informational (not blocking; pre-existing, outside the M4 change set)

- `backend/api/src/staff.rs` `execute_search_query` (M3 search, ~lines 895-927) builds SQL by
  string interpolation with manual `escape_sql_string` quote-doubling for dates / keyword /
  assignee rather than bound parameters. It is not introduced by M4 and quote-doubling blocks
  the classic break-out, but migrating these to bound parameters is recommended for defence in
  depth.

---

## EPIC-M4-C / EPIC-M4-E — 2026-07-24

**Status:** RESOLVED (2026-07-24)
**Compliance:** none declared (general OWASP + auth/authz correctness)
**Branch:** 2-modernisation

### Findings

- [WARN-4] `backend/api/src/help_topics.rs:93-95,111` (`list_topics`) — the `page` query
  parameter is floored (`page.max(1)`) but has **no upper bound**, while `page_size` is
  correctly clamped to `1..=200`. `offset = (page - 1) * page_size` is then interpolated
  into `LIMIT {page_size} OFFSET {offset}`. Both values are integers (no SQL-injection
  vector), but an admin passing a very large `page` (e.g. `i64::MAX`) overflows the
  `i64` multiplication: a debug build panics and a release build wraps to a negative
  `OFFSET`, which Postgres rejects — either way the request returns a 500. This is
  admin-gated and self-inflicted (no privilege escalation, no data exposure, no
  injection), so it is a low-severity robustness / defence-in-depth item, not an
  exploitable vulnerability.

### Recommendation

- WARN-4: Clamp `page` to a sane maximum (or use checked/saturating arithmetic for the
  offset computation) so a hostile/oversized page number cannot overflow into a 500,
  mirroring the existing `page_size` clamp.

### Resolution — 2026-07-24

- **[WARN-4a] RESOLVED** — `backend/api/src/help_topics.rs` (`list_topics`): the new `clamp_page`
  helper clamps `page` to `1..=MAX_PAGE` (1,000,000) as well as `page_size` to `1..=200`, and
  computes the offset with `saturating_mul`, so an oversized `page` (e.g. `i64::MAX`) can no longer
  overflow into a 500. Unit test: `clamp_page_bounds_and_never_overflows`.
- **[WARN-4b] RESOLVED** — `backend/api/src/help_topics.rs` (`update_topic`): giving a parent to a
  topic that already HAS children is now rejected with a 422 on `parentId` (new `has_children`
  guard), preserving strict one-level nesting (BS-030-20 / KL-030-02) — no more `C → A → B`
  three-level chains. Integration test: `cannot_nest_a_topic_that_already_has_children`.

### Informational (not blocking)

- `backend/api/src/help_topics.rs:424-483` (one-level nesting, BS-030-20) — create/edit
  validate that a *supplied parent* is itself top-level, and `update_topic` forbids a
  topic being its own parent. However, editing a topic that already **has children** to
  give it a parent is not blocked, which can produce a three-level chain
  (`C → A → B`). This is a functional/spec-integrity gap for QA, not a security defect
  (no authz bypass, no injection, no data exposure) — recorded for the functional review,
  not as a security finding.

---
