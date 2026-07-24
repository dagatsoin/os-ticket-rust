# P3 Coverage Report — api/* + setup/** (installer, CLI, tests, scripts)

Partition: **P3** (36 files). Phase-3 COVERAGE tagging run, osTicket-1.7.
Prepared: 2026-06-10. Relevant specs: FS-060 (installer), FS-061 (upgrader),
FS-090 (data export), FS-043 (external API / cron reference clients), FS-003
(crypto/validation — exercised by the test harness).

> Note: the P3 partition header reads "api/* + setup/**", but the P3 file *list*
> in PARTITIONS.md contains only `setup/**` files; the `api/*.php` entrypoints
> (`api.inc.php`, `cron.php`, `http.php`, `index.php`, `pipe.php`) are assigned to
> **P1**. P3 therefore tagged `setup/**` exclusively.

---

## Summary counts

| Metric | Count |
|--------|------:|
| Files in partition | 36 |
| Files examined | 36 |
| Files with ≥1 spec tag added | 16 |
| Files fully uncovered (recorded below) | 11 (10 test-harness + `setup/index.php` shim tagged) |
| Units examined (classes/functions/methods/significant blocks) | ~95 |
| Units tagged | ~58 |
| Units uncovered / recorded | ~33 |
| Trivia skipped (one-line shims, pure HTML chrome already covered by parent tag) | ~4 |

### Tagged files (spec coverage confirmed)

| File | Specs applied |
|------|---------------|
| `setup/index.php` | FS-060 (shim) |
| `setup/install.php` | FS-060 (full step state machine + dispatcher + view selector) |
| `setup/upgrade.php` | FS-061 (legacy redirect) |
| `setup/setup.inc.php` | FS-060 (bootstrap) |
| `setup/inc/class.installer.php` | FS-060 (config checks + full install routine) |
| `setup/inc/file-missing.inc.php` | FS-060 |
| `setup/inc/file-perm.inc.php` | FS-060 |
| `setup/inc/file-unclean.inc.php` | FS-060 |
| `setup/inc/footer.inc.php` | FS-060 (wizard chrome) |
| `setup/inc/header.inc.php` | FS-060 (wizard chrome) |
| `setup/inc/install-done.inc.php` | FS-060 |
| `setup/inc/install-prereq.inc.php` | FS-060 |
| `setup/inc/install.inc.php` | FS-060 |
| `setup/inc/subscribe.inc.php` | FS-060 (KL-060-02 dead step) |
| `setup/inc/ost-sampleconfig.php` | FS-060 (BS-060-02 self-redirect template) |
| `setup/cli/modules/export.php` | FS-090 (FS-090.27 backup exporter CLI) |
| `setup/scripts/api_ticket_create.php` | FS-043 |
| `setup/scripts/automail.php` | FS-043 |
| `setup/scripts/rcron.php` | FS-043 |
| `setup/cli/modules/import.php` | FS-090 (header verify) + **FS-092** (round-2: full restore/reconstruction pipeline now tagged) |
| `setup/cli/manage.php` | **FS-092** (round-2: CLI dispatcher + aggregated help) |
| `setup/cli/modules/class.module.php` | **FS-092** (round-2: Option/OutputStream/Module argparse framework) |
| `setup/cli/package.php` | **FS-092** (round-2: release packager — staging, version stamp, display_errors hardening, archiving) |
| `setup/cli/modules/deploy.php` | **FS-092** (round-2: continuous deploy) |
| `setup/cli/modules/unpack.php` | **FS-092** (round-2: archive unpack + INCLUDE_DIR rewrite) |

---

## Uncovered units (file:line, unit, behavior, FS/BS guess, suggested spec)

### A. Dev-tooling CLI framework & packaging — **CLOSED (FS-092, round-2)**

All rows below were closed in the round-2 pass by writing
`specs/FS-092-cli-management-deployment-packaging.md` (16 FRs) and tagging the units.

| File:line | Unit | Behavior | Status |
|-----------|------|----------|--------|
| `setup/cli/manage.php:9` | `class Manager` + `run()` | CLI dispatcher: discovers `modules/*.php`, routes an action arg to a registered Module | **COVERED** (FS-092.1, BS-092-01/02) |
| `setup/cli/manage.php:21` | `Manager::showHelp()` | Globs module scripts, prints per-module prologue help | **COVERED** (FS-092.3) |
| `setup/cli/package.php:10` | `get_osticket_root_path()` | Walks up to repo root (finds `main.inc.php`) | **COVERED** (FS-092.10) |
| `setup/cli/package.php:25` | `glob_recursive()` | Recursive glob helper | **COVERED** (FS-092.10) |
| `setup/cli/package.php:36` | `exclude()` | fnmatch exclusion helper | **COVERED** (FS-092.11) |
| `setup/cli/package.php:47` | `package()` | Copies matched files into `stage/upload/...` tree (recursive) | **COVERED** (FS-092.11) |
| `setup/cli/package.php:71-150` | top-level packaging script | Runs tests, builds `stage/`, sed-rewrites THIS_VERSION + display_errors=0, tar/zip release archive | **COVERED** (FS-092.10/11/12, BS-092-07/08; xref FS-060/FS-001) |
| `setup/cli/modules/class.module.php:3` | `class Option` | argparse-style option model (`__construct`, `hasArg`, `handleValue`, `toString`) | **COVERED** (FS-092.4/5) |
| `setup/cli/modules/class.module.php:92` | `class OutputStream` | thin fopen/fwrite wrapper for stdout/stderr | **COVERED** (FS-092.5) |
| `setup/cli/modules/class.module.php:107` | `class Module` | base CLI module: option parsing, help, arg validation, static `register`/`getInstance` registry | **COVERED** (FS-092.2/3/4/5, BS-092-03) |
| `setup/cli/modules/deploy.php:5` | `class Deployment extends Unpacker` | Continuous-deploy: copy repo tree to install path, preserve INCLUDE_DIR, `--setup`/`--dry-run` | **COVERED** (FS-092.6, BS-092-06) |
| `setup/cli/modules/unpack.php:5` | `class Unpacker` | Unpacks an osTicket tarball into an install path; relocates `include/`, rewrites `INCLUDE_DIR` in main.inc.php | **COVERED** (FS-092.7/9, BS-092-04/06) |
| `setup/cli/modules/unpack.php:47` | `change_include_dir()` | Rewrites the `define('INCLUDE_DIR', ...)` line in main.inc.php | **COVERED** (FS-092.8, BS-092-05; xref FS-001) |
| `setup/cli/modules/import.php:62` | `read_block()` | Reads a `\x1e`-separated JSON block from the backup stream | **COVERED** (FS-092.14; format xref FS-090.27) |
| `setup/cli/modules/import.php:85` | `import_table()` | Reads a table block, emits CREATE/INDEX, streams rows | **COVERED** (FS-092.15) |
| `setup/cli/modules/import.php:110` | `create_table()` | Reconstructs `CREATE TABLE` SQL from dumped column metadata | **COVERED** (FS-092.16, BS-092-10) |
| `setup/cli/modules/import.php:144` | `create_indexes()` | Reconstructs `CREATE INDEX` SQL from dumped index metadata | **COVERED** (FS-092.16, BS-092-10, KL-092-05) |
| `setup/cli/modules/import.php:176` | `truncate_table()` | TRUNCATE + DROP INDEX for `--drop`/restore | **COVERED** (FS-092.16) |
| `setup/cli/modules/import.php:189` | `load_row()` | Buffers rows into batched multi-row INSERTs (16 KB flush) | **COVERED** (FS-092.16, BS-092-11/12) |
| `setup/cli/modules/import.php:220` | `run()` | Boots main.inc.php, opens stream, verifies header, loops `import_table` | **COVERED** (FS-092.13) |

> **Round-2 resolution.** Rather than extend FS-090.27, the entire CLI/build subsystem
> (`manage`/`class.module`/`package`/`deploy`/`unpack`) plus the importer/restore side
> was documented in a dedicated new spec **FS-092**. The importer is captured as the
> structural counterpart to FS-090.27's exporter (FS-092.13–092.16), cross-referencing
> the FS-090.27 dump format rather than duplicating it. The release-time inverse of
> FS-060's install-time `display_errors` tuning is captured in FS-092.12 / BS-092-08 /
> KL-092-04. Nothing in Section A remains uncovered.

### B. Self-test harness (`setup/test/**`) — classification: test-harness

These are osTicket's own regression harness. Per run guidance they may legitimately
be uncovered by any functional spec; recorded here rather than force-tagged.

| File | Unit(s) | Behavior | Notes |
|------|---------|----------|-------|
| `setup/test/run-tests.php` | top-level runner, `show_fails()`, ctrl-c handler | Discovers `tests/test.*.php`, runs each Test, aggregates fails, exits with fail count | test-harness |
| `setup/test/tests/class.test.php` | `class Test` (assert/pass/fail/run/getAllScripts/line_number_for_offset) | Base test fixture + script enumeration (excludes 3rd-party paths) | test-harness |
| `setup/test/tests/stubs.php` | `mysqli`, `ReflectionClass` stubs | Empty stub classes for static linting | test-harness |
| `setup/test/tests/test.crypto.php` | `TestCrypto` (Simple/Mcrypt/OpenSSL/PHPSecLib/Random) | Round-trip crypto assertions | test-harness; **exercises FS-003 crypto** (Crypto::encrypt/decrypt/random, CryptoMcrypt/OpenSSL/PHPSecLib) |
| `setup/test/tests/test.validation.php` | `TestValidation::testValidUsernames` | Asserts `Validator::is_username` ascii/unicode behavior | test-harness; **exercises FS-003 validation** (Validator::is_username) |
| `setup/test/tests/test.extra-whitespace.php` | `ExtraWhitespace` | Flags leading-before-`<?php` / trailing-after-final-`?>` whitespace | test-harness (lint) |
| `setup/test/tests/test.shortopentags.php` | `ShortOpenTag` | Flags `<?` short-open tags | test-harness (lint) |
| `setup/test/tests/test.signals.php` | `SignalsTest::testFindSignalPublisher` | Asserts every `Signal::connect` has a matching `Signal::send` | test-harness; touches FS-003 signal infra |
| `setup/test/tests/test.syntax.php` | `SyntaxTest::testCompileErrors` | Runs `php -l` over all scripts | test-harness (lint) |
| `setup/test/tests/test.undefinedmethods.php` | `UndefinedMethods` + `find_function_calls()` | Heuristic check that called methods are defined somewhere | test-harness (lint) |
| `setup/test/tests/test.unitialized.php` | `UnitializedVars` | Runs external `phplint.tcl` for uninitialized-var access | test-harness (lint) |

---

## Notable findings (top 3)

1. **The importer/restore path has NO functional spec.** `setup/cli/modules/import.php`
   reconstructs full `CREATE TABLE` / `CREATE INDEX` / batched `INSERT` SQL from the
   FS-090.27 backup dump, yet FS-090 documents only the *export* side. This is the
   single largest body of unspecced osTicket-authored behavior in P3 (~150 lines of
   real restore logic). Recommend an FS-090.31+ "Backup Import / Restore" requirement.

2. **An entire CLI/build-tooling subsystem is unspecced** (`manage.php`,
   `class.module.php` argparse framework, `package.php` release builder,
   `deploy.php`, `unpack.php`). These perform install-time file placement and rewrite
   `INCLUDE_DIR` in `main.inc.php` — adjacent to FS-060/FS-001 concerns but documented
   by neither. Suggest a dedicated **FS-092 "CLI management, deployment & packaging"**.

3. **`package.php` couples to FS-060 install-bootstrap constants** — it sed-rewrites
   `THIS_VERSION` (FS-060.1) and forces `display_errors=0` in the packaged build (the
   inverse of FS-060.1's install-time `display_errors=1`). This release-time transform
   is a behavioral fact about how the shipped product differs from the dev tree, but no
   spec records it; worth capturing in FS-060's KL section or the proposed FS-092.

---

## Idempotency / safety notes

- All tags are additive comments (`// [FS-0XX]` / `<!-- [FS-0XX] -->`) placed directly
  above the covered unit; no existing logic lines were moved or deleted.
- One cosmetic deviation: in `setup/inc/install-done.inc.php` the original second line
  `?>    ` (4 trailing spaces, mid-file) was emitted as `?>` (no trailing spaces) while
  re-inserting the tag. This is functionally inert and is NOT flagged by the repo's own
  `test.extra-whitespace.php` (which checks only leading-before-`<?php` and
  trailing-after-the-final-`?>`-at-EOF). Likewise `setup/inc/install.inc.php` line 1
  lost a single trailing space after `<?php` (also inert, also unflagged).
