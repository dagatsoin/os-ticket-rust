# osTicket Modernisation Project

## What this repo is

The **osTicket modernisation project**. The repository began as a spec
reverse-engineering effort against a snapshot of **osTicket 1.7** (an open-source PHP web
helpdesk / support-ticket system, ~290 PHP files / ~59,600 lines: a public client portal,
a staff control panel, an admin configuration area, an email pipeline (POP3/IMAP fetch +
mail piping + outbound mail), an external API + cron, and a knowledge base / FAQ, all
backed by MySQL `ost_`-prefixed tables).

That reverse-engineering run is **complete**, and the repo has now entered its
**modernisation phase**: we are building a clean reimplementation on a modern stack, driven
by the specs in [`specs/`](./specs/).

## Current layout

- **`legacy/`** — the original osTicket 1.7 PHP source, archived verbatim and **frozen**.
  The pre-modernisation state is captured by the annotated git tag **`php-1.7-final`**.
- **`specs/`** — technology-agnostic functional specifications (FS-XXX docs with embedded
  BS-XXX business rules, EC-XXX edge cases, KL-XXX known limitations). Start with
  [`specs/CLAUDE.md`](./specs/CLAUDE.md) for the index and conventions.
- **`work/`** — process artifacts from the reverse-engineering run (plan, trackers,
  gap / coverage / dedupe reports). The method is
  [`work/SPEC_REVERSE_ENGINEER_PLAN.md`](./work/SPEC_REVERSE_ENGINEER_PLAN.md).
- **`kanban/`** — modernisation planning (to be created by planning).
- **Rust backend** — a Cargo workspace at the repo root (to be created).
- **React/TypeScript frontend** — (to be created).

## The new implementation

- **Backend**: **Rust** — a Cargo workspace at the repo root (to be created).
- **Frontend**: **React + TypeScript**.
- **Source of truth**: the functional specifications under `specs/` define the behaviour to
  reproduce. The frozen PHP under `legacy/` is the authoritative reference whenever a spec
  is ambiguous.

## Hard rules

- **Never push to any remote.** All git operations for this project stay **local** —
  no `git push`, no `git push --tags`, no remote `gh` operations.
- **The legacy PHP source is frozen.** Never modify any file under `legacy/`. The original
  "report spec only — never modify any PHP / source file" rule still applies in full to
  everything under `legacy/`. The only edits ever made to that source were the additive
  `@implements` comment tags inserted during the reverse-engineering coverage phase; no
  further source edits are permitted.
- **Spec / `@implements` paths are pre-move.** File paths referenced inside `specs/` and in
  the `@implements` tags refer to the original (pre-move) repo-root paths. They are now
  located under `legacy/` — e.g. a spec referencing `include/class.ticket.php` now means
  `legacy/include/class.ticket.php`. Prefix legacy references with `legacy/` when resolving
  them.

## Reverse-engineering run summary (complete)

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
