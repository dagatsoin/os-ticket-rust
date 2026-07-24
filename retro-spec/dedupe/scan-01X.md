# DEDUPE Scan — Band FS-01X (FS-010 Client Portal, FS-011 Web Submission)

Phase-4 dedupe scan. Perspective: client portal (login/lookup/view, captcha generation,
client session) and web ticket submission (open.php form, web validation, captcha
consumption, max-open-tickets web angle, thank-you/number-masking).

**Method**: Read FS-010 + FS-011 fully; skimmed FS-021, FS-041, FS-043, FS-042, FS-022,
FS-032, FS-002, FS-091, FS-001 for restatements either direction across the known seams.

**Headline**: The band is unusually well cross-referenced already. Almost every overlap is
an *acceptable per-source summary* with an explicit cross-ref, not verbatim duplication. The
two genuine trim candidates are (a) attachment type/size validation strings restated in
FS-011 that canonically belong to FS-022, and (b) the captcha image-generation mechanics
restated in FS-011.5 that belong to FS-010.11. The Ticket::create shared-pipeline seam is
already resolved cleanly (FS-011 is de-facto canonical home; others carry per-source deltas).

---

## Findings

| ID | Content item | Recommended canonical owner + why | Restating specs (refs) | Verdict / Fix |
|----|--------------|-----------------------------------|------------------------|---------------|
| D01X-1 | Attachment type/size **validation rules + verbatim error strings** ("Invalid file type for {name}", "File {name} ({size}) is too big. Maximum of {max} allowed") | **FS-022** — it canonically owns attachment validation against `allowed_filetypes`/`max_file_size` (FS-022 §lines 292-303, EC) and already names FS-032 as config owner. FS-011 should reference the validation contract, not restate it. | FS-011.7 + BS-011.12 + EC-011.5 (lines 112-116, 445-448); echoed FS-041.5.2/BS-041.11; FS-043.5/BS-437 | **VERBATIM** (error strings identical). Fix: in FS-011 keep the *web display/processing gate* (BS-011.12 is genuinely FS-011's — it's the open.php asymmetry) but **trim the type/size error-string + rule restatement to a cross-ref** to FS-022. FS-041/FS-043 already frame their copies as channel-specific soft-fail deltas — keep (acceptable). |
| D01X-2 | Captcha **image-generation mechanics** (5-char challenge, rendered onto background image, expected answer stored in session as uppercased digest) | **FS-010.11** — owns `captcha.php` generation + session-hash side effect (the band itself assigns it there; FS-010 §171-182). FS-011 only needs the *consumption* contract. | FS-011.5 AC bullet (line 87): "The challenge image is served by a separate image endpoint (`captcha.php`) that generates a 5-character challenge, renders it onto a background image, and stores the expected answer..." | **ACCEPTABLE-LEANING-VERBATIM**. FS-011.5 already cross-refs FS-010 for generation in its closing note. Fix: **trim the generation sentence to a cross-ref** ("the challenge image is served by `captcha.php` — generation owned by FS-010.11") and keep only the consumption rule (empty→"Enter text shown on the image"; mismatch→"Invalid - try again!"; case-insensitive digest compare). Consumption is correctly FS-011's. |
| D01X-3 | `enable_captcha` **config setting + GD-requirement save validation** | **FS-032** owns the admin setting & its save-time GD/PNG validation (FS-032 §206, 243, EC-032.4). FS-001/FS-091 list it in config-key catalogs only. | FS-010.11 + KL-011.1 reference the GD dependency; FS-032 owns it | **ACCEPTABLE** (3-way clean split: FS-032 = setting+save-validation, FS-010 = generation/GD-runtime-guard, FS-011 = consumption/"silently disabled" KL). No fix — this is correct separation, listed to document the seam is resolved. |
| D01X-4 | **Ticket::create shared pipeline** (trim fields → ban pre-screen → field validation → filter pipeline → routing → reference gen → post initial message → SLA → assign → autoresponse/alerts) | **FS-011** is the de-facto canonical home (Overview line 7 explicitly: "the shared validation and routing core is documented here from the web perspective"). FS-021/041/043 each carry only their per-source delta and cross-ref FS-011. | FS-011.9 (canonical web view); FS-021.20+BS-021.19 (staff delta: origin='staff', no-autorespond, max-open bypass); FS-041.9 (email delta: emailId routing, loop guards); FS-043.4/14 (api delta: format parse, 201 response) | **ACCEPTABLE** — no merge needed. Each source-spec restates only the *origin-specific* behavior and references the shared core. Recommend a one-line note added to FS-011 Overview making "FS-011 = canonical create home" explicit for downstream readers (Phase-1 also suggested FS-011 over FS-091; concur — FS-091 owns only data-model/reference gen, which it already does at FS-091.2). |
| D01X-5 | **Max-open-tickets throttle** (up-front block "You've reached the maximum open tickets allowed." + reach-the-ceiling over-limit notice + "excludes staff origin") | **FS-011** for the *value-bearing behavioral rules* (BS-011.4, BS-011.10, FS-011.13) — it is the public/web angle where the throttle is the primary control. Cap *value* default → FS-091; banlist/filter rejection engine → FS-042. | FS-011.13+BS-011.4+BS-011.10 (canonical); FS-021 BS-021.19 (staff-bypass restated, cross-refs FS-011); FS-041.7+EC-041.9 (email angle, cross-refs FS-011: "see FS-011 / FS-091 for the cap value and behavior") | **ACCEPTABLE**. FS-021 and FS-041 both restate the *existence + their-angle* of the cap and cross-ref FS-011 for the canonical behavior. No verbatim block duplication. No fix — seam resolved. |
| D01X-6 | **Banlist / filter "Ticket denied. Error #403"** rejection + "ban pre-screen requires present-and-valid email" gate | **FS-042** owns the engine + the canonical message/log (FS-042 §114, 148-149). FS-011 restates the user-facing message + the valid-email gating from the web handler's POV. | FS-011.13 + BS-011.8 + BS-011.9 (web POV); FS-042 §148-149 (engine, cross-channel) | **ACCEPTABLE**. Overlap is the user-facing literal "Ticket denied. Error #403" and the "valid-email-required-for-pre-screen" rule (FS-011 BS-011.9 vs FS-042 line 149). Both frame their own angle and FS-011 cross-refs FS-042 for the engine. Minor: the BS-011.9 valid-email-gate rule is arguably FS-042's (it's the engine's pre-screen condition). **Optional trim**: soften FS-011 BS-011.9 to cross-ref FS-042's pre-screen condition; low priority (web handler genuinely re-checks). |
| D01X-7 | **Client brute-force strike/lockout** parallel to staff lockout | **FS-010.5/BS-010.12** owns the *client* realm; **FS-002.2/BS-002-03** owns the *staff* realm. Shared `UserSession` primitive only. | FS-010.5 + KL-010.8 (client, incl. dead-code defect); FS-002.2 + KL-002-01 (staff) | **ACCEPTABLE** (intentional parallelism). Structurally similar text but each owns a distinct realm with distinct config keys + the client side documents a snapshot-specific dead-code defect FS-002 does not share. FS-002 explicitly cross-refs FS-010. No fix. |
| D01X-8 | **Reply-POST seam**: FS-010 documents the client reply *form contract* + reopen-on-reply expectation; FS-021 owns `postMessage` internals | **FS-010.7** = form contract only; **FS-021.5/.22** = postMessage/thread internals; the absent `tickets.php` controller logically hosts the handler (KL-010.1). | FS-010.7 (form, with explicit note delegating handler to FS-021/FS-041); FS-021.5 (postMessage system path) | **ACCEPTABLE**. FS-010.7's closing note already delegates the handler. No verbatim overlap of the posting logic. No fix — seam resolved. |
| D01X-9 | **Autoresponse loop-prevention triggers** (system-own-address / auto-response headers / mailer-daemon@/postmaster@ suppress autoresponse) | **FS-041** for the email-header detection mechanics (BS-041.12); the suppression *applies uniformly in the shared create routine* so FS-011/FS-021 each note it. | FS-011.12 final bullet (notes "primarily matter for email source — cross-ref FS-041"); FS-021 BS-021.20; FS-041 BS-041.12 (canonical detection) | **ACCEPTABLE**. FS-011.12 explicitly flags these as email-source-relevant and cross-refs FS-041. The header-heuristic detail lives canonically in FS-041 BS-041.12. No fix. |
| D01X-10 | **External ticket reference generation** (random 6-digit unique / sequential-id, regenerate-on-collision) | **FS-091.2** owns the canonical generation rule + `EXT_TICKET_ID_LEN`=6. FS-011.9 + FS-021.20 + BS-021.23 restate the scheme. | FS-011.9 + EC-011.8 + KL-011.2 (web, with backfill-failure KL); FS-021 BS-021.23; FS-091.2 (canonical) | **ACCEPTABLE**. FS-011 + FS-021 both cross-ref FS-091 for "reference semantics canonical to FS-091" and add only source-specific deltas (FS-011's KL-011.2 backfill-failure is genuinely FS-011's observation). No fix. |

---

## Summary

**Total findings: 10** — **Verbatim/leaning-verbatim duplication: 2** (D01X-1, D01X-2);
**Acceptable summaries / resolved seams: 8** (D01X-3 through D01X-10).

**5 most significant (with recommended owners):**

1. **D01X-1 — Attachment validation error strings** → owner **FS-022**. Only true verbatim
   duplication in the band. FS-011.7 restates "Invalid file type for {name}" / "File {name}
   ({size}) is too big..." word-for-word. Trim FS-011 to a cross-ref (keep BS-011.12's open.php
   display-vs-processing asymmetry, which is genuinely FS-011's).

2. **D01X-2 — Captcha image-generation mechanics** → owner **FS-010.11**. FS-011.5 restates
   the 5-char/background-image/uppercased-digest generation. Trim to a cross-ref; keep only the
   consumption contract (the matching rule + error strings), which is correctly FS-011's.

3. **D01X-4 — Ticket::create shared pipeline** → canonical home **FS-011** (already de-facto;
   concur with Phase-1's FS-011-over-FS-091 suggestion). No merge needed — per-source deltas in
   FS-021/041/043 are clean. Recommend one explicit "canonical create home" line in FS-011
   Overview for downstream clarity.

4. **D01X-5 — Max-open-tickets throttle** → owner **FS-011** for behavioral rules (cap *value*
   → FS-091). Seam already resolved: FS-021 (staff bypass) and FS-041 (email angle) restate only
   their angle and cross-ref FS-011. No fix.

5. **D01X-6 — Banlist "Ticket denied. Error #403" + valid-email pre-screen gate** → engine owner
   **FS-042**. Mostly acceptable; one optional low-priority trim: FS-011 BS-011.9's
   "valid-email-required-for-pre-screen" rule arguably belongs to FS-042's engine condition.

**Net recommendation**: 2 small trims (D01X-1, D01X-2) + 1 clarifying line (D01X-4). The band's
cross-referencing discipline is high; no merges or canonical-home reassignments required.
