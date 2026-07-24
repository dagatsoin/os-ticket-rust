# Phase-4 DEDUPE — Consolidated Duplicate-Analysis Report

Run: osTicket-1.7 reverse-engineered spec suite.
Source: 7 scan reports in `work/dedupe/` (scan-00X, -01X, -02X, -03X, -04X, -05X-06X-092, -09X).
This report **adjudicates** and **merges** the scanner findings and produces a per-spec FIX
WORKLIST. No spec files were modified by this pass — this report drives the fix-agents.

---

## (a) Executive summary

| Metric | Count |
|--------|-------|
| Raw findings across the 7 scans (incl. inverse checks) | 107 |
| Distinct content items after merging cross-scanner overlaps | 38 |
| Items requiring an edit (trim-to-crossref / correct / merge) | 17 |
| — of which VERBATIM literal-table/string duplications | 13 |
| — of which NEAR-VERBATIM (same formula/message, light paraphrase) | 3 |
| — of which factual CONTRADICTIONS (not mere restatement) | 1 (D03X-10) |
| Items adjudicated **KEEP-AS-IS** (acceptable summary + cross-ref / intentional parallel / clean seam) | 21 |
| Cross-scanner ownership CONFLICTS adjudicated | 2 (template catalog; SLA precedence) |
| Spec files needing edits | 13 |
| Total discrete fix items in the worklist | 24 |
| Spec files needing NO changes | 9 |

**Merge highlights** (same item flagged by multiple scanners — collapsed to one canonical decision):

- **Priority seed table**: flagged by 03X (D03X-12), 05X/060 (D05X-1), 09X (D09X-1) — unanimous: owner **FS-091.5**.
- **SLA seed ("Default SLA"/48h)**: 03X (D03X-13), 05X/060 (D05X-2), 09X (D09X-2) — unanimous: owner **FS-091.6**.
- **Navigation model**: 00X (D00X-1), 09X (D09X-3) — unanimous: owner **FS-090** (construction + render together).
- **Email-template seed code-names (12)**: 05X/060 (D05X-3), 09X (D09X-2 sub-row) — owner **FS-091.7** seed list.
- **Overdue sweep math** (50/run + reopened-aware grace): 02X (D02X-1, D02X-4), 03X (D03X-11), 04X (D04X-12) — split owners **FS-032.11** (math) + **FS-043 BS-432** (cadence/cap); **FS-021.13** keeps the action note only.
- **Template *type catalog*** (13 code-names + descriptions): 04X (D04X-1 → FS-091.7) vs 09X (D09X-4 → FS-040) — **CONFLICT, adjudicated below**.

---

## Adjudication of cross-scanner conflicts

### Conflict 1 — Email template type catalog (13 code-names → display name → description)

**D04X-1 recommends FS-091.7. D09X-4 recommends FS-040.**

**Decision: FS-091.7 owns the code-name → display-name → description catalog table. FS-040 keeps
the resolver behavior and cites FS-091.7.**

Rationale: The governing heuristic applied throughout this run is *FS-091 owns enum / value / seed
DATA tables; domain specs own BEHAVIOR*. The 13-row catalog is a stored application constant
(`all_names`) — a static value-list, the textbook case for the reference-data spec. D09X-4's
counter-argument (FS-040 owns tokens, resolution, seeding, and the `staff.pwreset` edge cases) is
real but argues for FS-040 owning **behavior over the catalog**, not the catalog literals
themselves. We split exactly on that line: the *table of names/descriptions* is data (FS-091.7);
the *resolution/fallback/lazy-load of `staff.pwreset`* is behavior (FS-040, retained). This keeps
the adjudication consistent with the priority seed, SLA seed, filter-enum, and log-enum decisions
in this same run, all of which sent the literal value-list to FS-091 and left semantics with the
domain. Choosing FS-040 here would make the template catalog the lone exception and reintroduce the
drift risk (catalog values diverging between two homes) that this entire pass exists to remove.

Consequence: **FS-040 is trimmed** (drop the re-tabulated 13 rows, cite FS-091.7). **FS-091.7 is
unchanged** and remains canonical. The seeded-subjects table already in FS-091 stays in FS-091.

### Conflict 2 — SLA selection precedence (factual CONTRADICTION, D03X-10)

This is **not a duplication** — it is a factual error. FS-030 BS-030-21/23 asserts *"topic SLA
overrides the department SLA."* The runtime code (`selectSLAId`, owned by **FS-021.13**) resolves
**explicit trump > department SLA > topic SLA > system default** — i.e. department is evaluated
**before** topic, the opposite of FS-030's claim. FS-011.4 states only "topic SLA → default" with
no dept layer (incomplete, not contradictory).

**Decision: FS-021.13 is the canonical runtime owner and is CORRECT. FS-030 must be corrected to
match the code. FS-091.6's pointer must be narrowed to FS-021.13 only.** (See FS-030 and FS-091
entries in the worklist.) D02X-5 also touched SLA precedence but only as a "where is the full list"
note — subsumed here; FS-021.13 remains the single full statement.

### Minor cross-scanner alignments (no conflict, recorded for consistency)

- **Filter operator/criterion enums**: D04X-4 and D09X-5 agree — storage enum value-lists + the
  origin→target map → **FS-091.9 / FS-091.3**; operator *semantics* + admin picker *labels* stay in
  **FS-042.3**. Applied as a single trim.
- **Auto-response/bounce marker vocabulary**: D04X-5 → **FS-042.9** owns the marker list; FS-041
  BS-041.12/13 trim to cite it, keep pipeline drop-timing behavior. (Suppression-*trigger* statements
  D01X-9 / D04X-6 are KEPT — each spec owns its own send/detection slice.)

---

## (b) Canonical-ownership decision table

| Content item | Canonical owner | Rationale (one-liner) |
|--------------|-----------------|-----------------------|
| Seeded priority table (4 rows, hex colors, urgency, public) | **FS-091.5** | Seed DATA value-list; reference-data spec owns it. |
| Seeded SLA ("Default SLA", grace 48h, flags) | **FS-091.6** | Seed DATA row. |
| Email-template **type catalog** (13 code-names + descriptions) | **FS-091.7** | Static value-list / `all_names` constant — DATA, not behavior (Conflict 1). |
| Email-template seed code-names (12 in default group) | **FS-091.7** | Seed DATA list. |
| Email-template resolver / fallback / `staff.pwreset` lazy-load | **FS-040** | Resolution BEHAVIOR (retained alongside the FS-091.7 catalog). |
| Filter operator/criterion **storage enums** + origin→target map | **FS-091.9 / FS-091.3** | Storage value-lists = DATA. |
| Filter operator **semantics** + admin picker **labels** | **FS-042.3** | Matching BEHAVIOR + UI labels. |
| Navigation model (staff/admin/client tabs, hrefs, submenus, active-state) | **FS-090** (.1–.9/.28) | Construction + render belong together. |
| Nav construction *timing* (which gate builds which nav) | **FS-001.13** | Bootstrap/gate ordering is genuinely FS-001's. |
| CSRF token mechanism (derivation, rotation, header/field) | **FS-001.11** | Request-lifecycle infra minted by bootstrap singleton. |
| `UserSession` token shape + validation + refresh | **FS-002.11** | Shared session primitive physically lives in staff session classes. |
| Username / phone format validators + error strings | **FS-003.9** (BS-013 / BS-010) | Standalone infra format validators; every caller invokes them. |
| Three-level log enum (Debug/Warning/Error) | **FS-091.8** (enum) + **FS-001.12** (write-side collapse) | Enum value-list = DATA; write/alert/purge = behavior. |
| SLA→due-date derivation + reopened-aware overdue math | **FS-032.11** | Config/plan-owned SLA computation. |
| Overdue sweep cadence + 50/run cap + job inventory | **FS-043 BS-432** | Cron job inventory + cadence owner. |
| `markOverdue`/`checkOverdue` action + activity note | **FS-021.13** | The per-ticket mutation action. |
| **SLA selection precedence** (`selectSLAId`) | **FS-021.13** | Canonical runtime resolver; dept-before-topic (Conflict 2 — FS-030 is wrong). |
| Per-ticket Ban/Unban action + banlist messages | **FS-042.12** | Banlist storage + per-ticket ban action owner. |
| Attachment type/size validation rules + error strings | **FS-022** | Validates against `allowed_filetypes`/`max_file_size`. |
| Captcha image-generation mechanics | **FS-010.11** | Owns `captcha.php` generation + session-hash side effect. |
| Captcha consumption (compare + error strings) | **FS-011.5** | open.php consumption is FS-011's. |
| Auto-response/bounce marker vocabulary | **FS-042.9** | Self-declared canonical; markers live on filter subsystem. |
| Pipe exit-code map (0/65/66/69/75/77) | **FS-041 BS-041.2** | Parse/pipe mechanics owner. |
| Email-format reply-detection + fixup defaults + threading precedence | **FS-041.5.3 / FS-041.6** | Parse/threading owner. |
| HTTP-email permitted field allow-list | **FS-043.4** | API endpoint owner; allow-list is an API-validation concern. |
| Mail-fetch cadence / backoff (5-err/10-min/80%-box/max_fetch) | **FS-041** (.3 / BS-041.5/6/15) | Fetch execution mechanics. |
| Department effective-access union model | **FS-002.17** | Access-resolution enforcement owner. |
| Config read-precedence cascade + pre-namespace fallback | **FS-001.5 (BS-016)** | Bootstrap/runtime read object. |
| `streams.cfg` discovery rule | **FS-061.2 (BS-061-15)** | Stream subsystem owner. |
| Seed VALUE catalog (depts/topics/templates/groups/team/banlist/canned/file/tz/pages) | **FS-091** (Seeded Reference Rows) | Single canonical seed catalog. |

---

## (c) Per-spec FIX WORKLIST

Each item is self-contained and drives one fix-agent per file. Format: **what to remove → what
cross-ref to put in its place → what domain content to KEEP**.

### FILE: `FS-001` (bootstrap)

1. **§FS-001.13 Navigation model** (merge of D00X-1 + D09X-3).
   - REMOVE: the literal enumeration of the staff 3-tab set, admin 5-tab set, per-tab submenus,
     hrefs (`tickets.php?status=assigned`, `tickets.php?a=open`), `<panel>.<tab>` submenu keying,
     FAQs-also-matching-`faq.php`, and the active-state model; also the Data-Requirements
     "Navigation Model" table detail; fold §EC-015 (set-missing-tab → false) into FS-090.
   - REPLACE WITH: "Each realm gate builds a realm-appropriate nav model — public `UserNav`, staff
     `StaffNav`, admin `AdminNav`; structure, render, and active-state owned by FS-090.1–FS-090.9/.28
     (client FS-090.4/.28)."
   - KEEP: the gate-ordering facts only — which gate builds which nav, that admin overwrites staff
     nav, and the default active section.

2. **§FS-001.12 / §BS-015 cross-ref** (D00X-4) — *no removal of FS-001's own write-side logic.*
   - KEEP: FS-001.12 remains canonical for the write-side collapse + alerting + purge.
   - ACTION: confirm FS-001.12 cites FS-091.8 for the `log_type` enum value-list. (Edit is on the
     FS-003 side; listed here only to note FS-001.12 is the retained write-side owner.)

> Note: CSRF (FS-001.11), config cascade (FS-001.5/BS-016), and `UserSession`-is-FS-002 are all
> *canonical here or correctly elsewhere* — FS-001 needs **no** change for them; the trims are on the
> restating specs (FS-002, FS-032). Only item 1 above is an edit to FS-001.

### FILE: `FS-002` (staff auth/sessions)

3. **§FS-002.7 + BS-002-01/-02 CSRF** (D00X-2).
   - REMOVE: the restated token derivation ("session id, 16 random bytes, secret salt"), rotation
     mechanics, and header/field ordering prose.
   - REPLACE WITH: "Token mechanism (derivation, rotation, header/field order) owned by FS-001.11."
   - KEEP: staff-realm consequences — 400 "Valid CSRF Token Required", AJAX meta-tag, and the
     login-page rotation rule (BS-002-02 is auth-specific behavior; keep it, cross-ref the derivation).

4. **§BS-002-22 username validator** (D00X-5).
   - REMOVE: the restated username character-class rule + literal error strings ("At least two (2)
     characters" / "Username contains invalid characters").
   - REPLACE WITH: cross-ref to FS-003.9/BS-013 for the pattern + error strings.
   - KEEP: the uniqueness / system-email-collision rule (auth-domain-specific).

> §FS-002.11 `UserSession` is CANONICAL — keep as is (FS-010 trims to it).

### FILE: `FS-003` (infrastructure)

5. **§FS-003.24 + §BS-015 log map** (D00X-4).
   - REMOVE: the literal priority-bucket enumeration (`emergency/alert/critical/error → Error` …)
     and the duplicated 1=Error/2=Warning/3=Debug map; collapse BS-015.
   - REPLACE WITH: "Three-level logging — enum value-list owned by FS-091.8; write-side
     collapse/alerting/purge owned by FS-001.12." (Keep the existing "Cross-reference (write path)"
     framing.)
   - KEEP: nothing extra needed; FS-003.24 is purely a cross-reference block.

> §FS-003.9 (BS-013 username, BS-010 phone) is CANONICAL — keep as is.

### FILE: `FS-010` (client portal)

6. **§FS-010.6 client UserSession** (D00X-3).
   - REMOVE: the duplicated token shape `"<hash>:<time>:<md5(ip)>"` and `hash = md5(time + session
     secret + …)` formula lines (the existing xref note at ~line 102 can stay).
   - REPLACE WITH: "Shared token shape/validation/refresh owned by FS-002.11 (UserSession)."
   - KEEP: the client deltas only — id = lowercased email, key = ticket#, IP-binding disabled, uses
     `client_session_timeout`.

> FS-010.11 captcha generation is CANONICAL — keep (FS-011 trims to it).

### FILE: `FS-011` (web submission)

7. **§FS-011.7 + BS-011.12 + EC-011.5 attachment validation** (merge of D01X-1 + D02X-11).
   - REMOVE: the restated type/size rules and verbatim error strings ("Invalid file type for
     {name}", "File {name} ({size}) is too big. Maximum of {max} allowed").
   - REPLACE WITH: cross-ref to FS-022 for the attachment-validation contract (+ FS-032 for the cap
     config values).
   - KEEP: BS-011.12's open.php display-vs-processing asymmetry (genuinely FS-011's web gate).

8. **§FS-011.5 captcha** (D01X-2).
   - REMOVE: the generation sentence (5-char challenge / rendered onto background image / uppercased
     digest stored in session).
   - REPLACE WITH: "The challenge image is served by `captcha.php` — generation owned by FS-010.11."
   - KEEP: the consumption contract — empty → "Enter text shown on the image"; mismatch → "Invalid -
     try again!"; case-insensitive digest compare.

> Optional/low-priority (D01X-6): FS-011 BS-011.9 valid-email-pre-screen gate could cross-ref
> FS-042's engine condition. Not included as a required item; web handler legitimately re-checks.

### FILE: `FS-020` (queue/dashboard)

9. **§BS-020.18 show-assigned override** (D02X-8).
   - REMOVE: the restated admin-or-manager gate.
   - REPLACE WITH: cross-ref to FS-002 (line ~200) for the access/flag-honouring primitive.
   - KEEP: how the override feeds the Open-queue toggle + the quick-stat branch (BS-020.17 —
     genuinely FS-020's).

10. **§FS-020.10 / BS-020.12 export strings** (D09X-6) — *low-priority trim.*
    - REMOVE: the literal export error strings ("Query token required" / "Query token not found" /
      "Internal error: Unable to dump query results") and the `tickets-<YYYYMMDD>.csv` filename.
    - REPLACE WITH: cross-ref to FS-090.23 / BS-090.12 for the literal strings + filename.
    - KEEP: the queue-side trigger ("Export link only when ≥1 ticket") + session query-token storage.

> Page-size hierarchy (D09X-7): trim FS-020's full precedence chain to "PAGE_LIMIT resolved per
> FS-090.19", keep the queue-only `limit` override note. (Add as part of item 10's file pass.)

### FILE: `FS-021` (ticket view/workflow)

11. **§FS-021.13 + BS-021.17 + BS-021.6 overdue sweep + due-date** (merge of D02X-1 + D02X-4 +
    D03X-11 + D04X-12).
    - REMOVE: the "50 open not-yet-overdue per run, oldest first" cap and the three reopened-aware
      sweep conditions; the `created + SLA grace_period` due-date formula.
    - REPLACE WITH: cross-ref to FS-032.11 (due-date/grace math) + FS-043 BS-432 (sweep cadence/cap).
    - KEEP: the `markOverdue`/`checkOverdue` action + activity-note text + idempotency, and the
      per-ticket "operator may override via explicit due date" workflow framing.

12. **§FS-021.13 SLA precedence — CORRECTION** (D03X-10, Conflict 2).
    - ACTION: FS-021.13 is CORRECT and remains canonical (explicit trump > department > topic >
      system default). No change to FS-021 text beyond ensuring it is unambiguously stated as the
      single full precedence statement. (The correction lands in FS-030 + FS-091.6.)

13. **§FS-021.14 + BS-021.11 Ban/Unban action** (merge of D02X-2 + D02X-3).
    - REMOVE: the restated ban/unban messages ("Email already in banlist" / "Email (<addr>) added
      to banlist" / "Email removed from banlist" / "Email is not in the banlist") and the
      unbannable-only-if-explicit-entry rule and the enforcing-check detail.
    - REPLACE WITH: cross-ref to FS-042.12 (ban/unban mechanics) + FS-042.10 (reply-block
      enforcement) + let FS-042.137 own the banner-vs-enforcement divergence note.
    - KEEP: only the workflow consequence — the reply form is blocked while banned.

14. **§FS-021.1 export branch + quick-stats submenu** (D02X-14 + D02X-15).
    - REMOVE: the duplicated export error-string set and the submenu/quick-stat paragraph
      (incl. the ">10 assigned/overdue" warning text).
    - REPLACE WITH: one-line cross-ref to FS-020.10 (export) and FS-020.2/FS-020.11 (submenu +
      quick stats).
    - KEEP: nothing extra — these live in FS-021 only because of the shared `scp/tickets.php` entry
      script.

> Paper-sizes divergence (D09X-13): FS-021:238 lists "Letter, Legal, A4, A3" — **missing `Ledger`**
> vs FS-091.11's 5-value set. VERIFY against source: if the UI subset is intentional, annotate it;
> otherwise correct FS-021 to the full set / cross-ref FS-091.11. (Flagged, verification required.)

### FILE: `FS-030` (departments/teams/help-topics)

15. **§BS-030-21 / BS-030-23 SLA precedence — FACTUAL CORRECTION** (D03X-10, Conflict 2).
    - REMOVE / CORRECT: the false claim "topic SLA overrides the department SLA." This contradicts
      the code.
    - REPLACE WITH: "A help topic MAY carry an SLA override; the effective resolution order
      (explicit trump > department SLA > topic SLA > system default) is owned by FS-021.13."
    - KEEP: the topic-side definition that a topic *can* supply an SLA (just not its precedence claim).

> Help-topic routing precedence (D03X-9) is ACCEPTABLE — keep. Team-membership-add split (D03X-15)
> is the documented split — keep.

### FILE: `FS-032` (settings/SLA/priorities)

16. **§FS-032.8 priority seed table** (merge of D03X-12 + D05X-1 + D09X-1).
    - REMOVE: the literal 4-row hex/urgency/public table.
    - REPLACE WITH: "(seed values canonical in FS-091.5)".
    - KEEP: the consumption facts — Default Priority control reads them, id 2 default, no CRUD screen.

17. **§Data-Req "SLA Plan" seed line** (merge of D03X-13 + D05X-2).
    - REMOVE: the restated "Default SLA"/48h/flags row.
    - REPLACE WITH: cross-ref to FS-091.6 Seeded SLA.
    - KEEP: FS-032.11's own overdue-computation semantics (genuinely FS-032's — not the seed row).

18. **§FS-032.7 config read cascade** (D03X-14).
    - REMOVE: the read-resolution-order bullets + the pre-namespace legacy single-row fallback bullet.
    - REPLACE WITH: "Reads resolve per the runtime config object (FS-001.5 / BS-016); this spec owns
      `set()`/`persist()` write semantics."
    - KEEP: the write / no-op / timestamp rules (genuinely FS-032's).

### FILE: `FS-040` (email accounts/templates/outbound)

19. **§"Template Type Catalog" Data-Req table (BS-040.7)** (Conflict 1 — D04X-1 over D09X-4).
    - REMOVE: the re-tabulated 13-row code-name → display-name → description table.
    - REPLACE WITH: cross-ref to FS-091.7 for the catalog rows.
    - KEEP: the behavioral resolver rules — BS-040.7 / BS-040.13 fallback resolution and the
      `staff.pwreset` lazy-load / edge cases (KL-040.4/.8). Behavior stays; only the table moves.

### FILE: `FS-041` (inbound pipeline)

20. **§BS-041.12 / BS-041.13 auto-response/bounce marker vocabulary** (D04X-5).
    - REMOVE: the full verbatim marker list (Auto-Submitted, Precedence bulk/junk/list,
      X-Auto-Response-Suppress, mailer-daemon From, delivery-failure subjects, etc.).
    - REPLACE WITH: cross-ref to FS-042.9 for the marker vocabulary.
    - KEEP: the pipeline-side behavior — suppress autoresponse / drop bounce on create-failure, and
      the BS-041.13 drop-timing prose (genuinely FS-041's).

### FILE: `FS-043` (API/cron)

21. **§FS-043.15 / BS-440 pipe exit-code map** (D04X-7, the named D-check).
    - REMOVE: the full 6-row exit-code table (201→0 / 400→66 / 401-403→77 / 415-417/501→65 / 503→69
      / 500→75).
    - REPLACE WITH: cite FS-041 BS-041.2 for the table.
    - KEEP: only "the pipe controller extends the API controller + requires no key + maps via the
      FS-041 table".

22. **§FS-043.14 / BS-439 email-format fixup defaults + threading** (D04X-8).
    - REMOVE: the restated fixup defaults (force source=Email, subject/message/emailId defaults,
      strip priority) and the ticketId→header→new-ticket threading precedence.
    - REPLACE WITH: cross-ref to FS-041.5.3 (fixup) + FS-041.6 (threading order).
    - KEEP: the HTTP wire framing only — format dispatch, 201 response, field allow-list.

23. **§FS-043.8 / BS-434 mail-fetch cadence/backoff** (D04X-10) + **§FS-043.4 field allow-list**
    (D04X-9) + **BS-432 overdue qualification** (D04X-12).
    - REMOVE: the re-derived 5-error / 10-min backoff / 80%-time-box / max_fetch / oldest-first
      rules (cite FS-041); for BS-432, the grace-from-creation/reopen qualification math (cite
      FS-032.11).
    - REPLACE WITH: "cron invokes mail-fetch as job #1 on its cadence; eligibility/backoff/time-box
      owned by FS-041 (.3 / BS-041.5/6/15)"; BS-432 keeps the sweep-selection + 50-cap + "Ticket
      Marked Overdue" log and cites FS-032.11 for the qualification math.
    - KEEP: FS-043.4 owns the **wire** field allow-list (canonical) — have FS-041.5.1 cite *it*
      instead (no change to FS-043.4 here beyond confirming it is the cited owner). FS-043 keeps the
      cron job inventory (FS-043.7) as canonical.

> Note: D04X-9 designates FS-043.4 as canonical for the allow-list, so the *trim is on FS-041.5.1*
> (cite FS-043.4). Captured under FS-041 file pass if a separate fix-agent runs it; otherwise
> bundle into item 20's FS-041 pass. Listed here for traceability.

### FILE: `FS-060` (installer)

24. **§Data-Req seed VALUE tables** (merge of D05X-1/-2/-3/-4/-6/-7 + D06X-4 + D09X-2).
    - REMOVE the verbatim seed literals and the streams.cfg discovery algorithm:
      - Priorities (4) full tag/color/urgency/public table → cross-ref FS-091.5.
      - SLA (1) "Default SLA"/grace=48 → cross-ref FS-091.6.
      - Email-template group + 12 code-names → cross-ref FS-091.7.
      - Groups (3) + permission-flag matrix → cross-ref FS-091 Seeded Groups.
      - Ban-list filter (execorder 99, sample `test@example.com`) → cross-ref FS-091.9 / FS-042.
      - Sample file `osTicket.txt` (25 bytes, "Canned attachments rock!") → cross-ref FS-091.14.
      - `streams.cfg` discovery rule (one stream/line, `#` comments, require `.sig`+dir, default
        `core`) → cross-ref FS-061.2 (BS-061-15).
    - REPLACE WITH: pointers to FS-091 "Seeded Reference Rows" (and FS-061.2 for streams), mirroring
      the pattern FS-060 already uses for config keys ("Full key list owned by FS-091/FS-032").
    - KEEP: the install-time BEHAVIOR — binding-by-lookup (e.g. `default_priority_id` wiring, Eastern
      timezone `offset=-5.0` lookup for the admin row, ~90 config-default illustration handful),
      admin-row creation, config-rewrite order, prefix-collision guard, and the install-time
      signature-record fact. Group↔dept Cartesian (D05X-5) and timezone list (D05X-8) stay as
      1-line behavioral summaries with a `(FS-091)` cross-ref.

---

## (d) KEPT-AS-IS list (acceptable summary / intentional parallel / clean seam)

| Item | Scan refs | Why kept |
|------|-----------|----------|
| Ticket::create shared pipeline (FS-011 canonical home, per-source deltas in 021/041/043) | D01X-4 | Each source restates only origin-specific behavior + cross-refs FS-011. Optional 1-line "canonical create home" note in FS-011 overview. |
| Max-open-tickets throttle (behavior FS-011, cap value FS-091, engine FS-042) | D01X-5 | Seam resolved; consumers cross-ref. |
| Banlist "Ticket denied. Error #403" + valid-email pre-screen | D01X-6, D02X-3 | FS-042 owns engine/message; FS-011 frames web POV with cross-ref. |
| Client vs staff brute-force lockout parallelism | D01X-7 | Intentional parallel; distinct realms/config keys; client documents a unique dead-code defect. |
| Reply-POST seam (FS-010 form contract / FS-021 postMessage) | D01X-8 | FS-010 delegates handler; no logic overlap. |
| Autoresponse loop-prevention triggers (FS-041 detection; FS-011/FS-021 note their angle) | D01X-9, D04X-6 | Each owns its send/detection slice; cross-refs present. |
| External ticket reference generation (FS-091.2) | D01X-10 | Consumers cross-ref FS-091; add only source-specific deltas. |
| `mass_process` FS-020↔FS-021 seam | D02X-6 | Model clean bidirectional split. |
| `checkStaffAccess` per-ticket vs listing predicate | D02X-7 | Distinct surfaces; both defer to FS-002/FS-031. |
| `%{...}` substitution into canned/templates → FS-040.11 grammar | D02X-9, D04X-2, D04X-3, D03X-18 | Consumers carry ">owned by FS-040" pointers; FS-033.9 owns only the static reference-card content. |
| Content-addressed file store + orphan reclamation (FS-022) | D02X-10 | Consumers cross-ref; minor FS-043 ref-count predicate optional-trim, low priority. |
| Attachment download hash forms (FS-022.10/.12) | D02X-12, D050-5 | FS-050.9 summary w/ cross-ref; optional low-priority trim of the md5 recipe. |
| Dashboard CSV/JSON export (FS-090.26 mechanic / FS-020.13 data) | D02X-13 | Exemplary split. |
| `postMessage` thread entry-type + reopen-on-message | D02X-16, D04X-15 | FS-021 owns model; FS-041 owns trigger/lookup. |
| Canned-reply "leaves unanswered" semantics (FS-022.14) | D02X-17 | Boundaries acceptable; optional low-priority trims. |
| Group permission-flag set (FS-091 model / FS-031 form / FS-002 enforce) | D03X-1, D09X-10 | Clean three-way boundary, each frames differently. |
| "Can manage tickets" derived predicate (FS-002) | D03X-2 | FS-031 points to FS-002. |
| Last-active-admin protection message | D03X-4 | Both legitimately need it (FS-031 surfaces, FS-002 enforces); keep message, optional condition-mechanics trim in FS-031. |
| Staff-deletion side effects | D03X-5 | FS-031's CRUD context legitimately documents delete guards. |
| Group↔dept reconcile + group-delete-requires-zero-members | D03X-6 | FS-002 one-liner cross-ref. |
| Group-disabled admin exemption / availability | D03X-7 | Acceptable. |
| canManageFAQ gate on category screens | D03X-8 | FS-032 notes "owned by FS-031". |
| Help-topic routing precedence | D03X-9 | Each consumer cross-refs FS-030/FS-011. |
| Site-page in-use protection / page bindings | D03X-16, D03X-17, D09X-9 | FS-033 owns entity; FS-032 owns bindings; clean. |
| Client `/config/client` upload projection | D03X-19 | Flagged out-of-scope, cross-refs FS-010/011. |
| Attachment-policy config keys (FS-032.4 settings / FS-022 enforce) | D03X-20, D02X-11 | FS-032 cross-refs FS-022.13. |
| Department effective-access union model (FS-002.17) | D03X-3 | NEAR-VERBATIM; FS-031 may trim to form-effect + "(enforced per FS-002.17)" — OPTIONAL low-risk, not a required item. |
| Cron job inventory (FS-043.7 owns; bodies owned by domain specs) | D04X-11 | Intended seam, all cross-ref correctly. |
| SYSTEM BAN LIST reserved-filter seed (FS-091 seed / FS-042 behavior) | D04X-13, D05X-6 | Legitimate split. |
| Mail-account fetch settings consumed (FS-040) | D04X-14, D09X-11 | Clean consumer references w/ attribution. |
| X-Forwarded-For / is_cli / get_path_info (FS-001) | D04X-16 | FS-043 attributes to FS-001. |
| Schema-signature 3-tier fallback (FS-001 gate / FS-061 detect / FS-060 record) | D06X-1, D06X-6 | Clean triangulation, each cross-refs. |
| Prereq contract {php:4.3, mysql:4.4} quirk | D06X-2 | Genuinely shared; both cross-ref. |
| SQL-loader mechanics | D06X-3 | FS-061 1-line summary w/ cross-ref. |
| Install/upgrade welcome-ticket synthetic identity | D06X-5 | Two distinct seeded tickets; parallel recipes. |
| Backup dump block format (FS-090.27 produce / FS-092 consume / FS-061 ref) | D09X-1(05X), D09X-15 | FS-092/FS-061 cede format, document consume side. |
| `display_errors` / INCLUDE_DIR / ROOT_DIR packager transforms | D09X-2(05X), D09X-3(05X) | FS-092 cedes, documents local transform only. |
| `Category::lookup` hollow-object quirk | D050-1 | Both consumers (FS-032 form, FS-050 router) legit; cross-ref. |
| FAQ category delete cascade | D050-2 | FS-050 adds orphan-`faq_topic` quirk FS-032 lacks. |
| cannedResp AJAX co-location under `kb` | D050-3 | FS-050 defers to FS-022. |
| Help-topic `ispublic` KB gating | D050-4 | FS-050 references read-only. |
| Log severity enum in viewer (FS-033) | D09X-8 | In-context viewer labels; add 1-line FS-091.8 cross-ref optionally. |
| Page type enum in editor (FS-033) | D09X-9 | In-context editor use. |
| Thread codes M/R/N + event states in FS-021 | D09X-12 | In-context behavioral definition, not a table restatement. |
| `max_file_size`=1048576 inline uses | D09X-14 | In-context config fact; FS-091 catalog owner. |

---

## (e) Spec files needing NO changes

These specs carry only acceptable summaries / clean seams / canonical content that stays put:

1. **FS-022** — canonical attachment-validation & file-store owner; consumers trim *to* it. No edit.
2. **FS-031** — group/staff CRUD; flag table and access model kept (D03X-3 trim is OPTIONAL/low-risk,
   not a required item). No required edit.
3. **FS-033** — log viewer / pages / variable reference-card; all kept (optional 1-line FS-091.8 /
   FS-091.10 cross-refs only). No required edit.
4. **FS-042** — canonical ban/marker/filter-semantics owner; others trim *to* it. No edit.
5. **FS-050** — KB/FAQ; cleanly defers everything; only optional low-priority hash-recipe trim
   (D050-5). No required edit.
6. **FS-061** — upgrader; clean seams (3-tier fallback, prereq, SQL-loader, streams owner). No edit.
7. **FS-090** — canonical nav/export/backup owner; others trim *to* it. No edit.
8. **FS-091** — canonical reference-data/seed/enum owner; **unchanged** (it WINS the template-catalog
   conflict at FS-091.7 and the priority/SLA seed and filter-enum decisions). No edit.
9. **FS-092** — exemplary; every cross-spec concern ceded with a cross-ref. No edit.

> (Specs **with** required edits, for contrast: FS-001, FS-002, FS-003, FS-010, FS-011, FS-020,
> FS-021, FS-030, FS-032, FS-040, FS-041, FS-043, FS-060 = 13 files touched. FS-003/FS-010 each
> carry a single small cross-ref trim.)

### Authoritative file count

The **authoritative list of files that must be edited** is:

`FS-001, FS-002, FS-003, FS-010, FS-011, FS-020, FS-021, FS-030, FS-032, FS-040, FS-041, FS-043, FS-060`
— **13 files**, 24 fix items (items 1–24 above; items 2 and 12 are confirmations/anchors, all
others are concrete trims/corrections).

Files needing **NO** change: `FS-022, FS-031, FS-033, FS-042, FS-050, FS-061, FS-090, FS-091,
FS-092` — **9 files** (plus any band-spec not enumerated here, e.g. none remain).
</content>
</invoke>
