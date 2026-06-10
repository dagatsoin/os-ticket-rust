# osTicket 1.7 — Spec Reverse-Engineering Project

## What this repo is

A snapshot of **osTicket 1.7**, an open-source PHP web helpdesk / support-ticket system
(~290 PHP files / ~59,600 lines). It provides a public client portal (open/view tickets),
a staff control panel (ticket workflow), an admin configuration area, an email pipeline
(POP3/IMAP fetch + mail piping + outbound mail), an external API + cron, and a knowledge
base / FAQ, all backed by MySQL (`ost_`-prefixed tables).

## What we are doing here

Reverse-engineering **technology-agnostic functional specifications** from the existing
source — documentation only. This mirrors the method used on `/Users/warfog/dev/crm_pascal`.

- **Functional specifications** live in [`specs/`](./specs/) (FS-XXX docs with embedded
  BS-XXX business rules, EC-XXX edge cases, KL-XXX known limitations). Start with
  [`specs/CLAUDE.md`](./specs/CLAUDE.md) for the index and conventions.
- **Process artifacts** (the plan, trackers, gap/coverage/dedup reports) live in
  [`work/`](./work/). The resume point and full method is
  [`work/SPEC_REVERSE_ENGINEER_PLAN.md`](./work/SPEC_REVERSE_ENGINEER_PLAN.md).

## Hard rule

**Report spec only.** Never modify any PHP / source file. The only permitted source edit
is the insertion of **additive `@implements` comment tags** during Phase 3 (coverage tagging),
which never alter executable code. Tags use the canonical requirement-level form
`@implements FS-XXX.N: <Title> — <note>` — one id per line, with BS/EC/KL first-class
(normalized from the original bracketed `[FS-XXX]` form in Round 3).

## Status

**ALL 4 PHASES COMPLETE — RUN COMPLETE (2026-06-10).** Phase 0 (scouting),
**Phase 1 (GENERATE)**, **Phase 2 (GAP-CLOSE)**, **Phase 3 (COVERAGE)** and
**Phase 4 (DEDUPE)** complete. **22 specs** (21 from Phase 1 + FS-092 added in
Phase 3 round-2 for the `setup/cli/**` tooling), all **Final (Phase 4 de-duplicated)**.

- **Final corpus (heading-counted, post gap-close + dedupe):** 22 specs · 13,426 lines ·
  365 FR · 413 BS · 353 EC · 221 KL.
- **Phase 1 baseline:** 318 FR · 341 BS · 261 EC · 155 KL (before gap-close additions).
- **Phase 2:** 249 gaps found / 249 fixed / 0 open (MISSING 120 · IMPRECISE 84 ·
  INCORRECT 45), which added/expanded FR/BS/EC/KL across all specs.
- **Phase 3:** additive `@implements` tagging, 2 rounds, dry — declaration-level coverage
  **100% of in-scope units, DRY after round 2** (244 in-scope files; **229 modified /
  2293 additive comment insertions / 2189 `@implements` ids (FS 1709 + BS 399 + EC 44 +
  KL 37); 0 bracket tags remain**; 229/229 files `php -l` pass under php 5.6). Tags use
  the requirement-level form `@implements FS-XXX.N: <Title> — <note>` (one id per line,
  BS/EC/KL first-class), normalized from bracketed `[FS-XXX]` in Round 3.
  Out of scope: 46 vendored third-party files, 11 self-test harness files, 2 bare
  redirect stubs. See `work/UNCOVERED_CODE_REPORT.md`.
- **Phase 4:** 107 raw findings → 38 distinct → 24-item worklist across 14 specs; the
  SLA-precedence contradiction resolved; adversarial verification **CLEAN after 4
  post-verification fixes**. See `work/DUPLICATE-ANALYSIS-REPORT.md` +
  `work/dedupe/ADVERSARIAL-VERIFICATION.md`.

See the convergence summary (`work/FINAL_CONVERGENCE_REPORT.md`), the plan
(`work/SPEC_REVERSE_ENGINEER_PLAN.md`), the tracker
(`work/REVERSE_ENGINEER_PROGRESS.md`), the consolidated gap report
(`work/GAP_REPORT.md`), and the coverage report
(`work/UNCOVERED_CODE_REPORT.md`).
