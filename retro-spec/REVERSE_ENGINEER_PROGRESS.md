# osTicket-1.7 Reverse-Engineering — Pass Tracker

Source: `/Users/warfog/dev/osTicket-1.7` (osTicket 1.7 PHP helpdesk, ~290 PHP files / ~59,600 lines).
Plan: `work/SPEC_REVERSE_ENGINEER_PLAN.md`. Conventions: `specs/CLAUDE.md`.

## Phase Status

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 0 | Scouting / domain map | DONE |
| Phase 1 | GENERATE (one spec-writer per domain) | DONE |
| Phase 2 | GAP-CLOSE | DONE — 249 gaps found / 249 fixed / 0 open (MISSING 120 · IMPRECISE 84 · INCORRECT 45), 2026-06-10. See `work/GAP_REPORT.md`. |
| Phase 3 | COVERAGE (additive `@implements` tags + loop-until-dry) | DONE (2026-06-10) — 2 rounds, dry. Declaration-level coverage **100% of in-scope units, DRY after round 2**. 244 in-scope files tagged across P1..P10 (0 uncovered after R2); **229 modified files / 2293 additive comment insertions / 2189 `@implements` id lines (FS 1709 + BS 399 + EC 44 + KL 37); 0 bracket tags remain** (TAG_VERIFICATION.md Round 3; supersedes the interim Round-2 1549 bracketed-tag figure). Round-2 closed: CLI/packaging/import → new **FS-092**; `db_*` helpers + Timezone accessors → FS-003/FS-001. Out of scope: 46 vendored, 11 test-harness, 2 redirect stubs. See `work/UNCOVERED_CODE_REPORT.md`. |
| — | Tag format normalization → `@implements` requirement-level | DONE (2026-06-10) — bracketed `[FS-XXX]` → canonical `@implements FS-XXX.N: Title — detail` (one id per line, BS/EC/KL first-class) per spec-writer methodology ruling; executed across 10 partitions (`work/coverage/retag-P*.md`). Final: 229 files / 2293 insertions / 0 deletions; 2189 `@implements` ids (FS 1709 + BS 399 + EC 44 + KL 37); 0 bracket tags remain; php 5.6 lint 229/229; byte-identical otherwise. |
| Phase 4 | DEDUPE | DONE (2026-06-10) — 107 raw findings → 38 distinct → 24-item worklist applied across 14 specs (13 edit files + FS-091 residuals); 1 factual contradiction (SLA precedence) resolved; adversarial verification CLEAN after 4 post-verification fixes (incl. the `FS-042.137` dangling ref → FS-042.14). See `work/DUPLICATE-ANALYSIS-REPORT.md` + `work/dedupe/ADVERSARIAL-VERIFICATION.md`. |
| Final | Convergence report | DONE (2026-06-10) — `work/FINAL_CONVERGENCE_REPORT.md`. Final corpus: **22 specs / 13,426 lines / 365 FR · 413 BS · 353 EC · 221 KL** (heading-counted, post gap-close + dedupe). **RUN COMPLETE.** |
| — | Re-crawl verification round | Re-crawl verification round — 14 findings closed, DRY at declaration level, 2026-06-10. Totals now **365 FR · 415 BS · 354 EC · 224 KL / 13,469 lines** (new ids BS-010.14/.15, EC-010.13, KL-010.10/.11, KL-040.11; KL-040.5 corrected; 8 retags). See `work/UNCOVERED_CODE_REPORT.md` §6 + `work/coverage/TAG_VERIFICATION.md` Round 4. |

## Spec Tracker

| ID | Spec | Band | Status | Gaps (M/INC/IMP) |
|----|------|------|--------|------------------|
| FS-001 | App bootstrap & shared request lifecycle | 00X shell | Gap-closed | 9 (3/1/5) |
| FS-002 | Staff authentication, sessions & access control | 00X shell | Gap-closed | 17 (8/6/3) |
| FS-003 | Cryptography, validation & formatting infrastructure | 00X shell | Gap-closed | 19 (10/3/6) |
| FS-010 | Public client portal — ticket lookup, view & login | 01X client | Gap-closed | 13 (6/3/4) |
| FS-011 | Public ticket submission (web form) | 01X client | Gap-closed | 9 (3/3/3) |
| FS-020 | Staff ticket queue, dashboard & search | 02X staff | Gap-closed | 8 (2/2/4) |
| FS-021 | Staff ticket view & workflow | 02X staff | Gap-closed | 18 (12/2/4) |
| FS-022 | Canned responses & ticket attachments | 02X staff | Gap-closed | 13 (6/4/3) |
| FS-030 | Admin — departments, teams & help topics | 03X admin | Gap-closed | 8 (4/2/2) |
| FS-031 | Admin — staff, groups & directory | 03X admin | Gap-closed | 12 (9/1/2) |
| FS-032 | Admin — system settings, SLA, priorities & categories | 03X admin | Gap-closed | 11 (4/2/5) |
| FS-033 | Admin — logs, pages & content | 03X admin | Gap-closed | 9 (5/1/3) |
| FS-040 | Email accounts, templates & outbound mail | 04X email/api | Gap-closed | 16 (7/3/6) |
| FS-041 | Inbound email pipeline — fetch, pipe & parse | 04X email/api | Gap-closed | 11 (4/2/5) |
| FS-042 | Ticket filters & banlist (inbound routing) | 04X email/api | Gap-closed | 16 (11/1/4) |
| FS-043 | External API & cron scheduler | 04X email/api | Gap-closed | 15 (8/3/4) |
| FS-050 | Knowledge base & FAQ (public + staff management) | 05X kb | Gap-closed | 10 (7/1/2) |
| FS-060 | Installer & setup wizard | 06X setup | Gap-closed | 11 (6/1/4) |
| FS-061 | Upgrader & database migration streams | 06X setup | Gap-closed | 10 (4/1/5) |
| FS-090 | Shared UI, navigation & data export | 09X shared | Gap-closed | 10 (5/1/4) |
| FS-091 | Reference data, enums & data model | 09X shared | Gap-closed | 4 (1/1/2) |
| FS-092 | CLI management, deployment & packaging tooling | 09X shared | Coverage-tagged (P3 R2) + Dedupe-clean (Phase 4) | n/a (new in Phase 3) |

**Total: 22 specs (21 from Phase 1 + FS-092 from Phase 3 round-2) — all Gap-closed (Phase 2)
+ Coverage-tagged (Phase 3) + De-duplicated (Phase 4). Phase 2: 249 gaps found / 249 fixed / 0 open
(MISSING 120 · IMPRECISE 84 · INCORRECT 45). Phase 3: 100% in-scope declaration coverage, DRY,
229 files / 2293 additive `@implements` tags (2189 ids: FS 1709 + BS 399 + EC 44 + KL 37; 0 bracket
tags remain). Phase 4: 24-item dedupe worklist across 14 specs, adversarial
verification CLEAN. RUN COMPLETE (2026-06-10).**

**Final corpus (heading-counted, post gap-close + dedupe): 22 specs / 13,426 lines /
365 FR · 413 BS · 353 EC · 221 KL.** See `work/FINAL_CONVERGENCE_REPORT.md`.

**Phase 1 summary (2026-06-10):** 21 of 21 specs drafted (one spec-writer per domain, no write conflicts). Totals across all specs: **318 FRs · 341 BS rules · 261 ECs · 155 KLs**. The Phase-0 plan estimated 23 specs; the executed work-list produced 21 (FS-00X×3, FS-01X×2, FS-02X×3, FS-03X×4, FS-04X×4, FS-05X×1, FS-06X×2, FS-09X×2). Per-spec FR/BS/EC/KL counts are recorded in `specs/CLAUDE.md`.

## Process Artifacts (created as phases run)

| File | Phase | Status |
|------|-------|--------|
| `work/SPEC_REVERSE_ENGINEER_PLAN.md` | 0 | Created |
| `work/REVERSE_ENGINEER_PROGRESS.md` | 0 | Created (this file) |
| `specs/CLAUDE.md` | 0 | Created (skeleton) |
| `work/GAP_REPORT.md` | 2 | Created (249 gaps / 249 fixed, 2026-06-10) |
| `work/coverage/PARTITIONS.md` | 3 | Created (10 partitions, 244 in-scope files, 2026-06-10) |
| `work/backups/BACKUP_RUNBOOK.md` | 3 | Created (backup PENDING shell exec) |
| `work/UNCOVERED_CODE_REPORT.md` | 3 | Created (2026-06-10 — 100% in-scope coverage, DRY after R2) |
| `work/coverage/P{1..10}-uncovered.md` | 3 | Created (per-partition tagging reports) |
| `work/coverage/TAG_VERIFICATION.md` | 3 | Created (Round 3 final: 229 files / 2293 additive insertions / 2189 `@implements` ids; 0 bracket tags remain; 229/229 `php -l` pass) |
| `work/coverage/retag-P{1..10}.md` | 3 | Created (per-partition `@implements` conversion logs, Round 3) |
| `work/DUPLICATE-ANALYSIS-REPORT.md` | 4 | Created (107 raw → 38 distinct → 24-item worklist, 14 specs) |
| `work/dedupe/ADVERSARIAL-VERIFICATION.md` | 4 | Created (CLEAN after 4 post-verification fixes) |
| `work/FINAL_CONVERGENCE_REPORT.md` | Final | Created (run complete, 2026-06-10) |

---

## Phase 3 — COVERAGE preparation (2026-06-10)

Prep complete (analysis + partition map + exclusion classification). One step deferred to a
shell-capable runner. **No PHP file and no spec was modified during prep** (HARD CONSTRAINTS honored).

### Done
- **Partition map** `work/coverage/PARTITIONS.md`: every in-scope `.php` file assigned to exactly
  one of **10 partitions** (P1..P10), balanced by non-empty-line proxy (~1.8k–5.8k lines each;
  the huge `class.ticket.php` 1,748 + `class.thread.php` 743 isolated in P7).
- **Third-party / vendored classification**: **46** bundled files EXCLUDED from coverage —
  4 misc libs (`include/JSON.php`, `Spyc.php`, `PasswordHash.php`, `htmLawed.php`),
  13 under `include/fpdf/**`, 29 under `include/pear/**` (PEAR/phpseclib/Mail/SASL/Net/Math).
  osTicket-authored thin wrappers (`class.json.php`, `class.yaml.php`, `class.crypto.php`,
  `class.pdf.php`, mailer/mailparse/mailfetch, `mysql.php`, `mysqli.php`, `setup/test/**`)
  kept **IN scope**.
- **Inventory**: 290 total `.php` files = **244 in-scope** + 46 excluded (reconciles exactly).

### Deferred (prep agent had no Bash/shell tool)
- **Backup tarball NOT yet created.** Exact commands in `work/backups/BACKUP_RUNBOOK.md`;
  target `work/backups/pre-tag-php-YYYYMMDD.tar.gz` (expect 290 files inside).
- **git status baseline** not captured (same limitation) — command in the runbook. Repo IS a
  git repository; baseline cleanliness lets `git diff` later prove comment-only tagging.

### Counts
| Metric | Value |
|--------|------:|
| Total `.php` files | 290 |
| Excluded (vendored) | 46 |
| In-scope | 244 |
| Partitions | 10 (P1..P10) |
| In-scope lines (non-empty proxy) | ~30,456 |

### Next (Phase-3 proper, NOT done here)
1. Execute the backup runbook + capture git baseline.
2. Additively tag in-scope PHP with `@implements` comments, partition by partition.
3. Produce `work/UNCOVERED_CODE_REPORT.md`.
