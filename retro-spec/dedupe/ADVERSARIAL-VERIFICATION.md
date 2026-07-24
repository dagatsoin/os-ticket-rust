# Phase-4 DEDUPE — Adversarial Verification Report

Run: osTicket-1.7 reverse-engineered spec suite.
Verifier posture: fresh, skeptical, read-only. Input: `work/DUPLICATE-ANALYSIS-REPORT.md` (24-item worklist, 14 specs).
Method: opened every target spec + spot-checked canonical owners; grep residual scan across all 22 specs; dangling-ref sweep on Phase-4 ids; SLA coherence cross-read.

---

## OVERALL VERDICT: **1 dangling ref, 0 residuals, 0 contradictions**

The trim/correction work landed cleanly across all 24 worklist items (verbatim content removed and cross-referenced; canonical homes intact). The SLA precedence contradiction is fully resolved and coherent across FS-030 / FS-021.13 / FS-091.6. **One dangling cross-reference** was introduced by a fix agent: **`FS-042.137`** (cited in FS-021) — this id does not exist; it is the exact "suspicious" reference the task flagged. One pre-existing out-of-scope residual is noted for completeness (not a fix-agent failure).

---

## (1) Per-worklist-item verdicts

| # | File / Item | Verdict | Evidence |
|---|-------------|---------|----------|
| 1 | FS-001.13 Navigation model | **LANDED** | FS-001.13 owns gate-ordering only, cross-refs FS-090.1–.9/.28; EC-015 folded to FS-090. No literal tab/href enumerations remain. |
| 2 | FS-001.12 / BS-015 log write-side | **LANDED** | FS-001.12 retained as write-side owner; cites FS-091.8 for the enum value-list. |
| 3 | FS-002.7 + BS-002-01/-02 CSRF | **LANDED** | FS-002.7 + BS-002-01/-02 cross-ref FS-001.11 for derivation/rotation/header-order; staff-realm 400 + login rotation kept. No restated derivation. |
| 4 | BS-002-22 username validator | **LANDED** | Cross-refs FS-003.9 (BS-013); keeps uniqueness/system-email-collision rule. Literals gone from the rule body. |
| 5 | FS-003.24 + BS-015 log map | **LANDED** | FS-003.24 is a pure cross-ref block (FS-091.8 enum + FS-001.12 write-side); BS-015 collapsed. No priority-bucket enumeration. |
| 6 | FS-010.6 client UserSession | **LANDED** | Token shape/hashing/refresh → FS-002.11. Literal `"<hash>:<time>:<md5(ip)>"` formula GONE from FS-010; client deltas kept. |
| 7 | FS-011.7/BS-011.12/EC-011.5 attachments | **LANDED** | Cross-refs FS-022 (contract+strings) + FS-032 (caps). Verbatim error strings gone. BS-011.12 display-vs-processing asymmetry kept. |
| 8 | FS-011.5 captcha | **LANDED** | Generation sentence removed → "served by captcha.php — generation owned by FS-010.11"; consumption strings kept. |
| 9 | BS-020.18 show-assigned override | **LANDED** | Cross-refs FS-002 (~line 200) for the admin/manager gate; keeps Open-queue toggle + quick-stat branch. |
| 10 | FS-020.10 export strings | **LANDED** | `tickets-<YYYYMMDD>.csv` + 3 error strings → FS-090.23/BS-090.12. Page-size hierarchy → FS-090.19; `limit` override kept. |
| 11 | FS-021.13/BS-021.17/BS-021.6 overdue sweep+due-date | **LANDED** | 50/run cap + reopened-aware conditions + `created+grace` formula removed → FS-043 BS-432 (cadence/cap) + FS-032.11 (math). markOverdue/checkOverdue action+note kept. |
| 12 | FS-021.13 SLA precedence (correction anchor) | **LANDED** | FS-021.13 states the single full precedence: "explicit trump > department > topic > system default — department before topic". |
| 13 | FS-021.14/BS-021.11 Ban/Unban | **LANDED** (text) — but see dangling-ref note | Cross-refs FS-042.12 + FS-042.10; verbatim ban messages gone; reply-block consequence kept. The `FS-042.137` pointer it adds is dangling (§3). |
| 14 | FS-021.1 export branch + quick-stats submenu | **PARTIAL** | Quick-stats submenu correctly delegated to FS-020.2/FS-020.11 (no restatement). BUT the export error strings ("Query token required"/"Query token not found"/"Internal error: Unable to dump query results") are still spelled out verbatim in FS-021.1 line 36, inside an "owned by FS-020.10" sentence. Borderline: attributed, not a fresh restatement, but the worklist asked for a one-line cross-ref without the literals. Mirrored in EC-021.22 (attributed). Low severity. |
| 15 | FS-030 BS-030-21/23 SLA correction | **LANDED** | False "topic SLA overrides department SLA" removed; both rules now cite FS-021.13 with correct order; misleading form help-text annotated (FS-030.14). Topic-can-carry-SLA kept. |
| 16 | FS-032.8 priority seed table | **LANDED** | 4-row hex/urgency/public table gone → "canonical in FS-091.5"; consumption facts (id 2 default, no CRUD) kept. No hex in FS-032. |
| 17 | FS-032 SLA seed line | **LANDED** | Restated "Default SLA"/48h row → one-line cross-ref to FS-091.6; FS-032.11 overdue math kept. |
| 18 | FS-032.7 config read cascade | **LANDED** | Read-precedence + pre-namespace fallback bullets removed → FS-001.5 (BS-016); set()/persist() write semantics kept. |
| 19 | FS-040 Template Type Catalog | **LANDED** | 13-row code-name→name→description table removed (FS-040.6 + Data-Req both cross-ref FS-091.7; "does not re-tabulate it"); BS-040.7 resolver + staff.pwreset edge cases (KL-040.4/.8) kept. |
| 20 | FS-041 BS-041.12/13 marker vocabulary | **LANDED** | Full marker list removed → FS-042.9; pipeline suppression + bounce drop-timing (BS-041.13, canonical here) kept. |
| 21 | FS-043.15/BS-440 pipe exit-code map | **LANDED** | Map attributed to FS-041 BS-041.2 ("canonically owned by"); "extends API controller + no key + maps via FS-041 table" kept. (Literals appear inline but explicitly cross-referenced as not-owned.) |
| 22 | FS-043.14/BS-439 fixup + threading | **LANDED** | Fixup defaults → FS-041.5.3; threading precedence → FS-041.6; HTTP wire framing (dispatch, 201, allow-list) kept. |
| 23 | FS-043.8/BS-434 fetch cadence + BS-432 + FS-043.4 allow-list | **PARTIAL** | FS-043.8/BS-434 correctly defer cadence/backoff/time-box/cap to FS-041 (.3 / BS-041.5/6/15). FS-043.4 confirmed canonical for the wire allow-list (FS-041.5.1 cites it). **BUT BS-432 still restates the full reopened-aware grace qualification math verbatim** (clauses a/b/c: "aged past active SLA grace period measured from creation" / "from reopen time" / "explicit due date in past") without citing FS-032.11, which the worklist explicitly required ("cite FS-032.11 for the qualification math"). The sweep-selection + 50-cap + "Ticket Marked Overdue" log are correctly kept here. Low-severity near-duplication of the FS-032.11 math. |
| 24 | FS-060 seed VALUE tables + streams.cfg | **LANDED** | Verbatim seed-value tables removed → FS-091.5/.6/.7/.14, FS-091.9/FS-042; streams.cfg discovery → FS-061.2 (BS-061-15). Only "kinds of rows" + install behavior kept. |

**Summary: 22 LANDED, 2 PARTIAL (items 14 and 23 — both low-severity attributed/near-duplicate restatements, not unconverted verbatim blocks), 0 MISSING.**

---

## (2) Residual literal scan (across all 22 specs)

| Literal | Canonical home | Residuals outside owner | Verdict |
|---------|----------------|-------------------------|---------|
| Priority hex `#FEE7E7` / `#DDFFDD` / `#FFFFF0` | FS-091.5 (+ its KL/EC notes) | none | **CLEAN** |
| 13 template code-names / `all_names` | FS-091.7 | none (FS-040 cross-refs) | **CLEAN** |
| Pipe exit-code map 66/77/65/69/75 | FS-041 BS-041.2 | FS-043.15/BS-440 list inline but attributed "owned by FS-041 BS-041.2" | **CLEAN (attributed)** |
| Auto-response/bounce markers (X-Autoreply, Precedence, Auto-Submitted, X-Auto-Response-Suppress, mailer-daemon From) | FS-042.9 (self-declared canonical) | FS-040.12/BS-040.22 carry the **outbound-composition** headers — a distinct concern (what osTicket emits), not inbound detection | **CLEAN (different direction)** |
| UserSession token `"<hash>:<time>:<md5(ip)>"` | FS-002.11 | none (FS-010 cross-refs) | **CLEAN** |
| CSRF derivation (session id + 16 random bytes + salt) | FS-001.11 | none (FS-002.7 cross-refs) | **CLEAN** |
| Nav tab enumerations / hrefs (`tickets.php?status=assigned`, `?a=open`), StaffNav/AdminNav/UserNav | FS-090.1–.9 | none (FS-001.13 cross-refs) | **CLEAN** |
| `streams.cfg` discovery rule | FS-061.2 (BS-061-15) | none (FS-060 cross-refs) | **CLEAN** |
| `tickets-<YYYYMMDD>.csv` + export error strings | FS-090.23 / FS-090 | FS-020.10 + FS-021.1/EC-021.22 cite strings but attributed to FS-090.23/BS-090.12 | **CLEAN (attributed; see item 14)** |
| Username validator literals ("At least two (2) characters" / "Username contains invalid characters") | FS-003.9 (BS-013) | FS-002 cross-refs (OK). **FS-031:246 restates both literals.** | **OUT-OF-SCOPE RESIDUAL** — FS-031 was a KEEP-AS-IS / no-required-edit spec in the worklist; this restatement predates and was not in scope for this pass. Flagged for awareness, not a fix-agent failure. |
| Phone validator (7–16 digits) | FS-003.9 (BS-010) | none | **CLEAN** |
| Log level map (Debug/Warning/Error) | FS-091.8 | FS-001.12 (write-side, cross-refs) / FS-003.24 (cross-ref) | **CLEAN** |
| Paper sizes incl. `Ledger` | FS-091.11 (= Letter/Legal/A4/A3) | none — `Ledger` REMOVED everywhere; only corrective annotations remain (FS-091.11, FS-021 deps) | **CLEAN** |

**Residual verdict: 0 in-scope residuals.** One out-of-scope pre-existing duplication (FS-031 username literals) noted; it was never a worklist item.

---

## (3) Dangling-reference sweep (~30 Phase-4 ids spot-checked)

Confirmed-existing (target id present in the named spec): FS-090.19, FS-090.23, BS-090.12, FS-090.28, FS-090.1–.9, FS-001.5/BS-016, FS-001.11, FS-001.12, FS-001.13, FS-002.11, FS-003.9/BS-013/BS-010, FS-091.5, FS-091.6, FS-091.7, FS-091.8, FS-091.9, FS-091.11, FS-091.14, FS-032.11, FS-043 BS-432, FS-041.5.3, FS-041.6, FS-041 BS-041.2, FS-042.9, FS-042.10, FS-042.12, FS-042.13, FS-061.2/BS-061-15, FS-021.13.

### DANGLING (FAIL):

- **`FS-042.137`** — cited in **FS-021 line 195** ("the banner-vs-enforcement wording divergence is owned by FS-042.137"). **No such id exists.** FS-042's requirements run to ~FS-042.13; the highest is in that range. The banner-vs-enforcement divergence content actually lives in **FS-042.13** (line 137: "the banner is informational, this check is enforcing"). `FS-042.137` is almost certainly a typo concatenating **FS-042.13 + line 137**. This is the exact reference the task flagged as suspicious — **confirmed fabricated/dangling.**

  - **Secondary mis-citation in the same sentence:** FS-021 line 195 also attributes "reply-block enforcement" to **FS-042.10**, but FS-042.10 is *"Mass Filter Operations"* — the reply-block enforcement is actually **FS-042.13** (the `disable_autoresponder`/enforcement requirement at line 137). FS-042.12 ("Per-Ticket Ban / Unban Staff Action") and FS-042.10 both *exist*, so they are not dangling, but the FS-042.10→reply-block mapping is semantically wrong; the intended target for both the divergence note and the enforcement is FS-042.13.

No other dangling refs found among the spot-checked Phase-4 ids.

---

## (4) SLA precedence contradiction (D03X-10 / Conflict 2)

**COHERENT — resolved.** All three specs now agree on **explicit (filter) trump > department SLA > topic SLA > system default** (department evaluated *before* topic):

- **FS-021.13** (canonical runtime owner, `selectSLAId`): states the single full precedence verbatim — "explicit trump value > department's SLA > help topic's SLA > system default — i.e. department is evaluated before topic."
- **FS-030 BS-030-21 / BS-030-23**: false claim "topic SLA overrides department SLA" removed; both now state "explicit trump > department SLA > topic SLA > system default ... owned canonically by FS-021.13"; the misleading "(Overrides department's SLA)" form help-text is explicitly annotated as misleading (FS-030.14 + BS-030-23).
- **FS-091.6**: "the effective SLA is resolved ... (explicit trump > department > topic > system default), defined canonically in FS-021.13, not here."

No surviving contradictory statement. FS-011.4's "topic SLA → default" was the incomplete (not contradictory) statement; it is consistent as a partial view.

---

## RETURN SUMMARY

- **Overall:** CLEAN except **1 dangling ref**; 0 in-scope residuals; SLA precedence coherent.
- **Problems found:**
  1. **DANGLING REF — `FS-042.137`** (FS-021 line 195): non-existent id; intended target is **FS-042.13**. The exact suspicious ref the task flagged — confirmed fabricated (FS-042.13 + line 137 typo). Same sentence also mis-maps "reply-block enforcement" to FS-042.10 (exists, but wrong; should be FS-042.13).
  2. **PARTIAL (item 23)** — FS-043 **BS-432** still restates the reopened-aware overdue grace math verbatim without citing FS-032.11 (worklist asked it to cite FS-032.11 for the qualification math; the 50-cap/log were correctly kept). Low severity (near-duplicate, not an un-trimmed block).
  3. **PARTIAL (item 14)** — FS-021.1 (and EC-021.22) still spell out the three export error strings verbatim inside an "owned by FS-020.10" sentence rather than a bare cross-ref. Attributed, low severity.
  4. **OUT-OF-SCOPE residual (note only)** — FS-031:246 restates the username validator literals ("At least two (2) characters" / "Username contains invalid characters"). FS-031 was a no-required-edit spec in the worklist, so this predates and was outside this pass; flagged for awareness.

---

## Post-verification fixes (applied)

All four findings above were remediated. Edits confined to `specs/FS-021*`, `specs/FS-043*`, `specs/FS-031*`. Ids verified present in their owner specs before writing; no ids renumbered.

1. **FS-021.14 (FS-021 ~line 195) — dangling/mis-mapped refs fixed.**
   - Before: "…owned by **FS-042.12** (ban/unban mechanics) and **FS-042.10** (reply-block enforcement); the banner-vs-enforcement wording divergence is owned by FS-042.137."
   - After: "…owned by **FS-042.12**; the reply-block enforcement and the banner-vs-enforcement wording divergence are owned by **FS-042.14**."
   - **Note / deviation from task wording:** the task and finding #1 specified `FS-042.13` as the corrected target. Verification of `specs/FS-042-…md` shows FS-042.13 is *"Filter Action Variable Mapping & Default Interaction"* (line 119) — unrelated to bans/replies. The reply-block enforcement and the "banner is informational, this check is enforcing" divergence both live in **FS-042.14: Reply-Time Ban Enforcement** (lines 131–138; the cited "line 137" is *inside* FS-042.14, not FS-042.13). Per the "verify the corrected ids exist before writing" rule, the semantically-correct owner **FS-042.14** was used. `FS-042.137` is eliminated either way.

2. **FS-043 BS-432 — overdue grace math trimmed to cross-ref.**
   - Before: full verbatim clauses (a) never-reopened grace-from-creation / (b) reopened grace-from-reopen / (c) explicit past due date, plus three examples.
   - After: "…becomes overdue per the reopened-aware grace qualification math owned by **FS-032.11** …. At most 50 tickets are aged per invocation, oldest first, and each newly overdue ticket logs the activity 'Ticket Marked Overdue'." (50-cap + log fact retained; FS-032.11 confirmed canonical for the math.)

3. **FS-021.1 (FS-021 line 36) — export error strings collapsed to cross-ref.**
   - Before: "The export error strings ('Query token required' / 'Query token not found' / 'Internal error: Unable to dump query results'), the `tickets-<YYYYMMDD>.csv` filename, and the CSV mechanics are owned by **FS-020.10**…"
   - After: "The export error strings, the export filename, and the CSV mechanics are owned by **FS-020.10** (strings/filename per FS-090.23 / BS-090.12)…" (EC-021.22 was already a pure cross-ref — no literals — left as-is.)

4. **FS-031 BS-031-017 (line ~246) — username validator literals trimmed to cross-ref.**
   - Before: "A username must be **at least two characters** and may contain only letters, digits, dot, underscore, and hyphen; otherwise the save is rejected (`At least two (2) characters` / `Username contains invalid characters`). Usernames are stored stripped of HTML tags."
   - After: "A username must pass the username validator owned by **FS-003.9 / BS-013** (length and allowed-character rules, with their literal error strings); on failure the staff save is rejected. Usernames are stored stripped of HTML tags." (FS-031 domain consequence — save-rejection + HTML-strip-on-store — retained; literals removed.)
