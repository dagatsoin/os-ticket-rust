# Recrawl P3 — Independent Verification (`setup/**`)

Partition: **P3** (`api/* + setup/**` per table; the `api/*.php` entrypoints actually
live in P1/P8, so P3 = `setup/**` exclusively — 36 files). Read-only adversarial
re-crawl, fresh eyes, did not trust existing `@implements` tags.

Prepared: 2026-06-10.
Specs cross-referenced: FS-060 (installer), FS-061 (upgrader), FS-090 (data export),
FS-043 (external API/cron reference clients), FS-092 (CLI mgmt/deploy/packaging).

## Result: **DRY** — zero UNCOVERED / MIS-TAGGED findings.

Every behavior-bearing unit in P3 carries an `@implements` tag whose cited id was
spot-verified to (a) exist as a first-class section in the named spec and (b) match
the code's actual behavior. Spot-check rate ~70% (well above the 30% floor): every
tag in `class.installer.php`, `install.php`, `import.php`, `package.php`, `manage.php`,
`class.module.php`, `deploy.php`, `unpack.php`, `export.php`, all 3 `scripts/*`, all
`inc/*` partials, `setup.inc.php`, and `inc/ost-sampleconfig.php` was opened against
its spec.

## Summary counts

| Metric | Count |
|--------|------:|
| Files in partition | 36 |
| Files walked | 36 |
| Files carrying `@implements` (191 tag lines across 25 files) | 25 |
| `setup/test/**` (test-harness, intentionally untagged) | 11 |
| Units COVERED (tag present + cited id verified) | ~95 |
| Units UNCOVERED | 0 |
| Units MIS-TAGGED | 0 |
| Units TRIVIA (structural getters / one-line shims / dup helpers) | ~10 |

## Spec-id verification (spot-checks, all PASS)

- FS-060.1/.2/.3/.4/.5/.6/.7/.8/.9 — all present (FS-060 lines 37–286).
- BS-060-01..18 (the subset used: 01,03,04–18) — all present.
- EC-060-01/-02/-05/-07/-10/-11/-12/-13/-14, KL-060-01/-02/-04/-05/-09/-11 — present.
- FS-092.1–.16 — all present; BS-092-01..13 — all present; EC-092-01/03/04/05/06/07/08/09/11/12/13, KL-092-02/05 — present.
- FS-090.27 (exporter, consumed by importer) — present (FS-090 line 339).
- FS-043.4/.9/.15 — present; BS-440 + EC-449 first-class in FS-043 (lines 497, 624).
- FS-061.9 — present (FS-061 line 112).

## `setup/test/**` classification — VERIFIED still test-harness

Walked all 11 files (`run-tests.php`, `tests/class.test.php`, `stubs.php`, and the
8 `test.*.php`). Confirmed: pure regression/lint harness (assert fixtures, `php -l`
runner, whitespace/short-open/undefined-method/uninitialized-var linters, signal
publisher checker, crypto/validation round-trip tests). **No file in `setup/test/**`
ships user-facing behavior.** `test.crypto.php` and `test.validation.php` merely
*exercise* FS-003's Crypto/Validator (which live in P4/P6), they do not implement it.
The only production consumer of this tree is `package.php`'s release-gate
`run_tests()` call, already covered by FS-092.10 / BS-092-07. Classification holds.

## Non-blocking observations (NOT findings — recorded for completeness)

1. `import.php:194 truncate_table()` is defined but never invoked anywhere in the
   importer (run → import_table → create_table/create_indexes; truncate is dead code).
   Its tag `FS-092.16` is the *correct* requirement id, so this is not a mis-tag —
   just a dead method. No action needed.
2. `import.php` reads `$this->getOption('prime-time')` but `prime-time` is never
   declared in the Importer `$options`, so the live-execution branch is unreachable
   and the sink always writes SQL to stdout. This is exactly what BS-092-13 documents
   ("default sink is SQL to stdout … live only when a prime-time mode is in effect"),
   so the tag is accurate.
3. `unpack.php:77 exclude()` and `class.module.php` getters/`_run`/abstract `run` are
   untagged structural plumbing (TRIVIA); behavior-bearing siblings (`unpackage`,
   `parseArgs`, `parseOptions`) are tagged. Correct to leave untagged.
4. Installer methods `check_prereq/check_php/check_mysql/load_sql_file/getMySQLVersion/
   getVersionVerbose/getErrors` are inherited from `SetupWizard` in
   `include/class.setup.php` (P6), not defined in P3 — correctly outside P3 coverage.

No UNCOVERED or MIS-TAGGED rows to report.
