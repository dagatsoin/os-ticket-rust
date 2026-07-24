# P1 — Uncovered Units Report

Partition: **P1** (root entry scripts + top-level includes)
Run: Phase-3 COVERAGE tagging, osTicket-1.7
Prepared: 2026-06-10

Scope: 34 files. COMMENT-ONLY additive `[FS-0XX]` tags inserted above documented
units; undocumented non-trivial units recorded below.

> **Round-2 update (2026-06-10)**: the procedural `db_*` helper API and the
> Timezone field accessors below are now SPEC'D and TAGGED. They were absorbed
> into **FS-003** as a new "Procedural data-access helper layer" — new units
> FS-003.26–FS-003.30, rules BS-026–BS-029, edge cases EC-021–EC-023, and
> limitations KL-015/KL-016; the connect-time `sql_mode=''` side-effect also got
> EC-021 in FS-001. The Timezone getName/getDesc dead-read was ALREADY documented
> (FS-003.25 / KL-013) — tagged in place, no new id. Every closed row is marked
> **COVERED** below.

---

## Uncovered units

| File:line | Unit | Behavior (1-line) | FS/BS guess | Suggested owning spec | Status |
|-----------|------|-------------------|-------------|------------------------|--------|
| include/mysql.php:184 / mysqli.php:235 | `db_input()` | Coerce/escape a scalar or array for safe SQL embedding (numeric passthrough else real-escape). | FS (infra) | FS-091 (data-access helper layer) — currently only the *table model* is spec'd, not the escaping helper API | **COVERED — FS-003.26 / BS-026 / EC-021 (R2)** |
| include/mysql.php:176 / mysqli.php:226 | `db_real_escape()` | Driver-level string escape with optional quoting; foundation of all SQL-injection defense. | FS (infra) | FS-091 or FS-003 (input-safety) — escaping primitive is undocumented as a unit | **COVERED — FS-003.26 / BS-026 / KL-016 (R2)** |
| include/mysql.php:163 / mysqli.php:213 | `db_output()` | Reverse magic-quotes on result rows (stripslashes when get_magic_quotes_runtime on). | KL (infra) | FS-091 — legacy magic-quotes compensation undocumented | **COVERED — FS-003.28 / BS-027 (R2)** |
| include/mysql.php:115-161 / mysqli.php:158-211 | `db_result/db_fetch_array/db_fetch_row/db_fetch_field/db_assoc_array/db_num_rows/db_affected_rows/db_data_seek/db_data_reset/db_insert_id/db_free_result` | Thin result-set accessor wrappers over the native driver. | FS (infra) | FS-091 — the result-access helper API is the unspec'd procedural surface | **COVERED — FS-003.28 / EC-022 (R2)** |
| include/mysql.php:100 / mysqli.php:143 | `db_squery()` | "Smart" sprintf-style parameterized query (`?` placeholders, per-arg escape). | FS (infra) | FS-091 — parameterized-query helper undocumented | **COVERED — FS-003.27 (R2)** |
| include/mysql.php:111 / mysqli.php:154 | `db_count()` | Convenience scalar-count query wrapper. | FS (infra) | FS-091 | **COVERED — FS-003.27 (R2)** |
| include/mysql.php:62-79 / mysqli.php:94-112 | `db_timezone/db_get_variable/db_set_variable/db_select_database` | Read/write MySQL session variables + select DB (sql_mode reset, time_zone read). | FS (infra) | FS-001 (bootstrap sets sql_mode; select_database in connect seq) — session-var helpers themselves unspec'd | **COVERED — FS-003.29 / BS-028 / EC-023 / KL-015; FS-001 EC-021 (R2)** |
| include/mysql.php:46 / mysqli.php:78 | `db_close()` | Close the active DB connection. | FS (infra) | FS-091 | **COVERED — FS-003.29 (R2)** |
| include/mysql.php:194-208 / mysqli.php:250-263 | `db_error/db_connect_error/db_errno/db_field_type` | Driver error/metadata accessors used by the logging path. | FS (infra) | FS-091 / FS-001 | **COVERED — FS-003.28 (R2)** |
| include/class.timezone.php:42 | `Timezone::reload()` | Re-run load() against the current id. | FS (infra) | FS-091 (trivial-ish reload of the documented load) | **COVERED — FS-003 (read-model, FS-003.25 family); tagged (R2)** |
| include/class.timezone.php:46-60 | `getId/getOffset/getName/getDesc` | Plain field accessors over the loaded timezone row. | FS (infra) | FS-091 (accessor boilerplate; getName reads `$this->info` which is never populated — latent bug) | **COVERED — ALREADY in FS-003.25 / KL-013 (getName/getDesc dead-read); tagged in place, no new id (R2)** |
| api/index.php:2, include/index.php:2 | bare `header('Location: ../')` | Directory-index redirect guard to prevent listing. | KL (trivia) | FS-001 (security hardening) — too trivial to tag | Unchanged — trivia, intentionally untagged (FS-001 security hardening covers the intent) |

## Notes on borderline calls

- **`db_connect` / `db_query` / `db_version` / `db_create_database`** WERE tagged
  (FS-001 owns the bootstrap connection + DB-error logging path; FS-060 owns the
  installer's version check + database creation). The *rest* of the procedural
  `db_*` library (escaping, fetch, output, session-var helpers) is genuinely
  un-spec'd at the unit level — FS-091 documents the table model and names
  "procedural data-access helpers" as the access mechanism but does not specify
  the helper function contracts. This is the single largest coverage gap in P1.
- **`Misc::*`** all tagged FS-003 (FS-003 explicitly enumerates: random
  alphanumeric/numeric code gen, timezone/GMT/DB time conversions, current-URL
  reconstruction, time-of-day dropdown).
- **`Timezone` load/lookup/getOffsetById** tagged FS-091/FS-003; the pure
  field-accessor methods left untagged as boilerplate.

## Round-2 closure note (2026-06-10)

- The single largest P1 gap — the procedural `db_*` helper library — is closed.
  FS-003 gained a "Procedural data-access helper layer" subsection:
  **FS-003.26** (`db_input`/`db_real_escape` SQL-injection defense + per-driver
  numeric pattern), **FS-003.27** (`db_query`/`db_squery`/`db_count`),
  **FS-003.28** (result-set accessors + `db_output` magic-quotes reversal +
  error/metadata accessors), **FS-003.29** (connection lifecycle + session-var
  read/write + the connect-time `sql_mode=''` side-effect), **FS-003.30** (dual
  driver back-ends, one contract). Supporting rules **BS-026–BS-029**, edge cases
  **EC-021–EC-023**, limitations **KL-015–KL-016**. The connect-time `sql_mode`
  reset also got **FS-001 EC-021** (the connect routine is bootstrap-owned).
- The Timezone `getName`/`getDesc` dead-read was NOT a new finding — it was
  already **FS-003.25 / KL-013**. Tagged in place (`// [FS-003] … KL-013`); no new
  id created (per instruction to skip duplicates).
- `include/mysql.php` and `include/mysqli.php`: **25 `// [FS-003]` tags each (50
  total)** inserted above every previously-untagged helper; the four already-tagged
  bootstrap/installer helpers (`db_connect`/`db_query`/`db_version`/
  `db_create_database`) were left untouched.
- `include/class.timezone.php`: 5 `// [FS-003]` tags inserted (`reload`, `getId`,
  `getOffset`, `getName` w/ KL-013 note, `getDesc`).

## Tooling artifact (disclosure)

- `include/class.timezone.php` (round-2): inserting the accessor tags, the Edit
  tool stripped trailing whitespace from three pre-existing lines — `function
  getId() {` (a trailing space after `{`), the `return $this->ht['offset'];`
  line (trailing spaces), and one all-whitespace blank line between methods. No
  code tokens changed; PHP ignores trailing/blank-line whitespace, so these are
  non-behavioral. Flag for the git-diff verifier: same class of whitespace-only
  artifact disclosed for `attachment.php` below.
- `attachment.php`: while inserting the `[FS-022][FS-010]` tag, the Edit/Write
  tools stripped trailing whitespace from several pre-existing continuation lines
  inside the two `if(...)` guard blocks (lines ~21-22, 30-32) and one header
  blank line. No code tokens were changed; the deltas are whitespace-only on
  otherwise-unmodified lines plus the one added tag line. Flag for the git-diff
  verifier: these whitespace-only lines in `attachment.php` are not behavioral
  edits.

---

## Summary counts

- Files processed: **34** (2 are bare `header('Location: ../')` redirect stubs)
- Units examined: **~96** (entry-script sections, inc-file gates, class methods, db helper functions)
- Units tagged: **~62** (round 1) **+ 55 (round 2: 50 db_* helpers + 5 Timezone accessors) = ~117**
- Units uncovered (recorded above): **~30 → 1** after round 2 (only the bare index-redirect stubs remain intentionally untagged)
- Trivia skipped (bare requires, index redirect stubs): **~2**

> **Round-2 result**: all ~29 db_*/Timezone-accessor rows are now COVERED + tagged.
> The only remaining "uncovered" item is the deliberately-skipped directory-index
> redirect stub (FS-001 security-hardening intent; too trivial to tag).

### Most notable uncovered findings

1. ~~**The entire procedural `db_*` helper API (escaping, fetch, output, parameterized query) is unspec'd at the unit level**~~ — **RESOLVED (R2)**: absorbed into FS-003.26–FS-003.30 (+ BS-026–BS-029 / EC-021–EC-023 / KL-015–KL-016); both `include/mysql.php` and `include/mysqli.php` fully tagged.
2. ~~**`Timezone::getName()` reads `$this->info['timezone']` but the class only ever populates `$this->ht`**~~ — **ALREADY DOCUMENTED**: FS-003.25 / KL-013. Tagged in place in round 2 (no new id — duplicate, skipped per instruction).
3. ~~**`db_get_variable`/`db_set_variable` … `sql_mode=''` reset on every connect**~~ — **RESOLVED (R2)**: FS-003.29 / BS-028 / EC-023 / KL-015, plus **FS-001 EC-021** for the bootstrap-side side-effect.
