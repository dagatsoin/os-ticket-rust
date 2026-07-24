# osTicket-1.7 Spec Reverse-Engineering — Final Convergence Report

Run completed: **2026-06-10**. Source: `/Users/warfog/dev/osTicket-1.7` (osTicket 1.7 PHP
helpdesk, ~290 PHP files / ~59,600 lines). Output: 22 technology-agnostic functional specs in
`specs/` + process artifacts in `work/`. Method: the 4-phase loop (GENERATE → GAP-CLOSE →
COVERAGE → DEDUPE) defined in `work/SPEC_REVERSE_ENGINEER_PLAN.md`, mirroring the crm_pascal run.

> **Standing hard rule honored throughout:** *report spec only* — no PHP/source file was modified
> except **additive `@implements` comment tags** in Phase 3 (comment-only, never altering executable
> code; bracketed `[FS-XXX]` in rounds 1–2, normalized to requirement-level `@implements` in Round 3).
> Source remains byte-identical to HEAD apart from those additive comment lines.

---

## 1. Phase-by-phase summary

### Phase 0 — Scouting / domain map (DONE)
- Produced the verified domain map (`work/SPEC_REVERSE_ENGINEER_PLAN.md` §3): every osTicket
  domain mapped to an FS spec and its primary source files, across 9 numbering bands
  (FS-00X shell, 01X client, 02X staff, 03X admin, 04X email/api, 05X kb, 06X setup, 09X shared).
- Captured 13 key architectural findings (MySQL persistence via procedural `db_*` helpers, the
  `main.inc.php` bootstrap chain, the two session realms, the Group+Department authorization
  model, CSRF/crypto, the central email pipeline, API+cron, the Ticket spine, AJAX subsystem,
  content-addressed chunked file storage, streamed setup/upgrade, signals/hooks).
- Artifacts: `work/SPEC_REVERSE_ENGINEER_PLAN.md`, `work/REVERSE_ENGINEER_PROGRESS.md`,
  `specs/CLAUDE.md` (skeleton).

### Phase 1 — GENERATE (DONE)
- **21 specs** drafted, one `spec-writer` agent per domain (no write conflicts).
- Phase-1 baseline totals: **318 FR · 341 BS · 261 EC · 155 KL**.
- The Phase-0 plan estimated 23 specs; the executed work-list produced 21
  (FS-00X×3, FS-01X×2, FS-02X×3, FS-03X×4, FS-04X×4, FS-05X×1, FS-06X×2, FS-09X×2).

### Phase 2 — GAP-CLOSE (DONE)
- Per-spec fresh-eyes re-read of the assigned source slice vs. the draft spec; gaps fixed in the
  spec only. **249 gaps found / 249 fixed / 0 open** (MISSING 120 · IMPRECISE 84 · INCORRECT 45).
- One internal counting note (FS-003 prose-vs-table) reconciled; the 249 total verified by summing
  per-file rows.
- Artifacts: `work/GAP_REPORT.md` (consolidated) + `work/gaps/FS-*-gaps.md` (21 detail files).

### Phase 3 — COVERAGE (DONE — 2 rounds, dry; tags normalized in Round 3)
- Additive `@implements` comment tagging of in-scope PHP, partitioned P1..P10, loop-until-dry.
- **Declaration-level coverage = 100% of in-scope units; DRY after round 2.** 244 in-scope files
  tagged; 0 genuinely-uncovered units after round 2.
- Round 2 closed the last two clusters: the `setup/cli/**` tooling became a **brand-new spec
  FS-092** (CLI management / deployment / packaging); the procedural `db_*` helpers + Timezone
  accessors were absorbed into FS-003 / FS-001 (cross-referenced, not duplicated).
- **Tag verification (final — TAG_VERIFICATION.md Round 3):**
  **229 modified files / 2293 additive comment insertions / 0 deletions**; **2189 `@implements`
  id lines** (FS 1709 + BS 399 + EC 44 + KL 37); **0 bracket tags remain**. Zero deletions,
  zero modifications, byte-identical otherwise. All 229 files lint clean under `php 5.6-cli`
  (Docker, 229/229).
- Artifacts: `work/UNCOVERED_CODE_REPORT.md`, `work/coverage/PARTITIONS.md`,
  `work/coverage/P{1..10}-uncovered.md`, `work/coverage/TAG_VERIFICATION.md`,
  `work/coverage/retag-P{1..10}.md`.

**Tag format normalization (Round 3).** The user flagged that the bracketed spec-level
`[FS-XXX]` tags were non-conformant with the spec-writer methodology. The adopted ruling is
the requirement-level `@implements FS-XXX.N: Title — detail` form (kyc_pascal style), one id per
line, with BS/EC/KL promoted to first-class tags. The conversion was executed across 10
partitions (per-partition logs in `work/coverage/retag-P*.md`) and verified in
TAG_VERIFICATION.md Round 3: **229 files / 2293 insertions / 0 deletions; 2189 `@implements`
id lines (FS 1709 + BS 399 + EC 44 + KL 37); 0 bracket tags remain; php 5.6 lint 229/229;
byte-identical otherwise.**

### Phase 4 — DEDUPE (DONE)
- 7 per-band duplicate scans (`work/dedupe/scan-*.md`) → **107 raw findings → 38 distinct items**
  after merging cross-scanner overlaps.
- **24-item fix worklist** applied across **14 specs** (13 enumerated edit files + FS-091 residual
  confirmations). 17 items required an edit (13 verbatim-literal duplications, 3 near-verbatim,
  1 factual contradiction); 21 items adjudicated KEEP-AS-IS (clean seams / intentional parallels).
- The single factual **contradiction** (SLA selection precedence) was resolved: the runtime owner
  **FS-021.13** (`selectSLAId`: explicit trump > department SLA > topic SLA > system default —
  department before topic) is canonical and correct; **FS-030** was corrected (its
  "topic SLA overrides department SLA" claim was false) and **FS-091.6** narrowed to point at
  FS-021.13. Coherent across all three specs.
- **Adversarial verification: CLEAN after 4 post-verification fixes.** The skeptical read-only pass
  found 1 dangling cross-reference (`FS-042.137`, a typo concatenating FS-042.13 + line 137; the
  real owner is FS-042.14 "Reply-Time Ban Enforcement") and 3 low-severity partials; all 4 were
  remediated (edits confined to FS-021/FS-043/FS-031; no ids renumbered). 0 in-scope residual
  literal duplications; 0 surviving contradictions.
- Artifacts: `work/DUPLICATE-ANALYSIS-REPORT.md`, `work/dedupe/ADVERSARIAL-VERIFICATION.md`,
  `work/dedupe/scan-*.md`.

---

## 2. Final corpus metrics

Counted by the careful **definition-heading** method (count definition-marker lines —
`### FS/BS/EC/KL-…:` headings and `**FS/BS/EC/KL-… — …**` bold-bullet definitions at line start —
NOT raw in-text token mentions). The corpus uses two definition conventions:
heading-style (`### BS-NNN.N:` / `### BS-NNN:`) and bold-bullet style (`- **BS-NNN-NN — …**` /
`- **BS-NNN — …**`); both were counted, once per definition.

| Metric | Value |
|--------|------:|
| Specs (`specs/FS-*.md`) | **22** |
| Total lines across all 22 specs | **13,426** |
| FR — `FS-XXX.N` functional requirements | **365** |
| BS — business rules | **413** |
| EC — edge cases / error scenarios | **353** |
| KL — known limitations / quirks | **221** |
| **Total embedded definitions (BS+EC+KL)** | **987** |
| **Total addressable spec items (FR+BS+EC+KL)** | **1,352** |

> These are the **current true figures** (post Phase-2 gap-close additions and Phase-4 dedupe
> trims/corrections). They supersede the Phase-1 baseline (318/341/261/155) and the interim
> `specs/CLAUDE.md` per-spec table totals (334/354/274/162), which predate later passes. The
> per-spec CLAUDE.md table was not recomputed line-by-line during the run; this report's
> heading-counted figures are authoritative for the final corpus.

### Per-spec line counts

| Spec | Lines | Spec | Lines | Spec | Lines |
|------|------:|------|------:|------|------:|
| FS-001 | 670 | FS-022 | 718 | FS-042 | 316 |
| FS-002 | 523 | FS-030 | 405 | FS-043 | 705 |
| FS-003 | 540 | FS-031 | 374 | FS-050 | 537 |
| FS-010 | 540 | FS-032 | 1,141 | FS-060 | 720 |
| FS-011 | 567 | FS-033 | 883 | FS-061 | 397 |
| FS-020 | 576 | FS-040 | 530 | FS-090 | 751 |
| FS-021 | 683 | FS-041 | 468 | FS-091 | 663 |
| | | | | FS-092 | 719 |

---

## 3. Notable latent osTicket-1.7 defect catalog

A reverse-engineering side-product: documented defects, dead code, and quirks discovered in the
*actual osTicket 1.7 source* (recorded as KL/EC items in the specs, not corrected — the source was
never modified). These are behavioral facts about the shipped product, several of them
security-relevant.

1. **Client login lockout is dead code (FS-010).** An error is pre-set before the lockout guard, so
   `laststrike` / admin-alert / "Access Denied" never fire on the public client portal. (The
   parallel staff lockout path is unaffected.)
2. **Logout link-token guard is a no-op (FS-002).** A missing `exit` after the redirect means the
   forced-logout-CSRF defence does not actually exist.
3. **Password-reset time window is never enforced (FS-002).** The reset-window check is a dead
   boolean — the expiry it implies is not applied.
4. **MySQL version check is fake (FS-061).** `check_mysql()` is purely
   `extension_loaded('mysql')`; there is **no** MySQL version gate — the advertised "v4.4" is a
   static label, not an enforced minimum.
5. **Directory numeric-search SQL is missing an `OR` (FS-031, KL-031-002).** The phone/ext/mobile
   match clause omits an `OR` between the extension and mobile conditions, so numeric directory
   searches do not reliably match the mobile field.
6. **SLA UI help-text is misleading (FS-030 / FS-021).** The topic SLA select says
   "(Overrides department's SLA)" but the runtime resolver evaluates **department before topic** —
   the help-text states the opposite of actual precedence (the Phase-4 contradiction, now annotated).
7. **`Category::lookup` returns a hollow object for any numeric id (FS-050).** No `getId()==id`
   re-check, so the "Unknown or invalid FAQ category" error path is dead code.
8. **`tickets.php` quick-stats unassigned constraint is mis-scoped (FS-020).** The `staff_id=0`
   (unassigned) constraint is appended only to the `open` UNION branch, not to
   answered/overdue/assigned/closed.
9. **Note-form `state=unassigned` is inert (FS-021).** The switch tests misspell `unassined`, so
   the unassign-on-note action never executes.
10. **Preview lock banner keys off the wrong owner (FS-021).** It never warns about another agent's
    active lock because it compares against the wrong lock owner.
11. **Staff-created tickets bypass the max-open ceiling (FS-021).** The open-ticket throttle is not
    applied on the staff create path.
12. **`isFileTypeAllowed` is default-deny (FS-022).** An empty allow-list rejects *all* uploads;
    only an explicit `.*` allows everything — the opposite of an intuitive "unset = allow".
13. **`kb/file.php` has no session gate (FS-022).** KB file download is reachable without a session.
14. **`send_sys_errors` "always on" is dead (FS-032).** It is actually stored as `0` and never
    triggers — the documented "always send system errors" behavior does not happen.
15. **`equal` ban operator is mis-coded (FS-042).** In the pre-screen it behaves like `contains`,
    not an exact match.
16. **Filter actions apply twice per ticket (FS-042).** The action set is executed twice on one
    inbound ticket.
17. **Empty-pipe outcome emits permanent data-error exit 65 (FS-041).** The intended "retry/defer"
    result is actually a permanent failure code an MTA would mishandle.
18. **Whole Site Pages CMS was undocumented (FS-033, corrected in Phase 2).** `scp/pages.php` +
    `class.page.php` + `ost_page` (landing/offline/thank-you/public-slug pages) — roughly half the
    domain — had been flatly denied by the draft spec; restored.
19. **`ost_email_template` table-count fiction (FS-091, corrected in Phase 2).** The schema has
    exactly **34** `CREATE TABLE`s, not 35 — the draft padded to 35 by double-counting the
    `email_template_group` / `email_template` pair.
20. **Dangling/dead method reads in the Timezone accessor (FS-003).** `Timezone::getName` /
    `getDesc` read the wrong member slot, so the timezone name is permanently unreadable.
21. **`randNumber` ignores its range (FS-003).** It silently ignores `start`/`end` unless `len==0`.
22. **Channel asymmetries across the email pipeline (FS-041/042/043).** Polled-fetch vs. HTTP/pipe
    diverge on seen-flag suppression, emailId fallback, ban timing, and bounce-drop — undocumented
    behavioral splits.

> Additional KL/EC items (no-change-save inconsistencies in FS-030, periodic password-reset aging
> auto-force in FS-031, over-limit admin-alert gating in FS-011, the `ajax`/`getIP` request-header
> casing handling routed through FS-001's `X-Forwarded-For`/`is_cli` utilities, the
> `staff.pwreset` template lazy-load edge cases in FS-040, etc.) are catalogued in full in the
> per-spec KL/EC sections and `work/gaps/FS-*-gaps.md`.

---

## 4. Out-of-scope inventory (intentionally untagged — not coverage gaps)

| Class | Files | Rationale |
|-------|------:|-----------|
| **A — Vendored / third-party libraries** | **46** | Behavior owned by upstream projects, not osTicket specs. 4 misc bundled libs (`include/JSON.php` Services_JSON, `Spyc.php` YAML, `PasswordHash.php` phpass, `htmLawed.php`); 13 under `include/fpdf/**`; 29 under `include/pear/**` (PEAR / phpseclib / Mail / SASL / Net / Math). osTicket-authored thin wrappers over them (`class.json.php`, `class.yaml.php`, `class.crypto.php`, `class.pdf.php`, mailer/mailparse/mailfetch, `mysql.php`, `mysqli.php`) are **IN scope** and tagged. |
| **B — Self-test harness** | **11** | `setup/test/**` (`run-tests.php` + 10 `tests/*.php`) — a regression/lint harness asserting *over* the product, not itself a user/operator-reachable functional capability. |
| **C — Bare redirect stubs** | **2** | `api/index.php` and `include/index.php` — each a single `header('Location: ../')` directory-index guard; security-hardening intent already covered by FS-001; too trivial to carry a unit tag. |

Inventory reconciliation: **290 total `.php` files = 244 in-scope + 46 vendored** (with the 11
test-harness + 2 redirect stubs being in-scope files left intentionally untagged).

---

## 5. Verification guarantees

- **Source byte-identical except additive comment tags.** Every change to the PHP tree is an
  additive comment-line insertion; **2293 added lines, 0 removed, 0 modified** across **229 files**
  (TAG_VERIFICATION.md Round 3, final figures). **2189** added lines carry an `@implements` id
  (FS 1709 + BS 399 + EC 44 + KL 37); **0** bracket `[FS-XXX]` tags remain.
- **git-diff verified.** `git diff --stat` reports `229 files changed, 2293 insertions(+)` with no
  deletion column; the tree is byte-identical to HEAD apart from the additive comment lines. All
  trailing-whitespace regressions disclosed during tagging were repaired back to HEAD bytes.
- **`php -l` pass under era-appropriate runtime.** All 229 modified files lint clean under
  `php:5.6-cli` (Docker; no local PHP) — 229/229, zero repairs in Round 3.
- **Spec integrity.** Phase-4 adversarial verification confirmed 0 dangling cross-references
  remaining (the one `FS-042.137` typo was fixed to FS-042.14), 0 in-scope residual literal
  duplications, and a coherent SLA-precedence resolution across FS-021/FS-030/FS-091.

---

## 6. Final status

**All 4 phases complete (2026-06-10).** The run delivered **22 technology-agnostic functional
specs** (13,426 lines; 365 FR · 413 BS · 353 EC · 221 KL) covering 100% of in-scope osTicket-1.7
declarations, gap-closed against the implementation, deduplicated to canonical cross-references,
and adversarially verified CLEAN — over a source tree that remains byte-identical to HEAD apart
from 2293 additive `@implements` comment tags (2189 spec-id lines: FS 1709 + BS 399 + EC 44 + KL 37).
