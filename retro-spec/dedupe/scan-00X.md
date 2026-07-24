# DEDUPE Scan — Band FS-00X (FS-001 bootstrap, FS-002 staff auth/sessions, FS-003 infra)

Phase-4 DEDUPE scan. Perspective: FS-001 / FS-002 / FS-003. Scan-only; no spec modified.

Method: read the three band specs in full, then grep+read all 19 neighbour specs
for content that **restates** (rather than cross-references) material canonically
belonging to this band, and the inverse (band content that belongs elsewhere).
Each finding classified **VERBATIM** (near-identical prose/literals → must fix) or
**ACCEPTABLE** (one-line summary carrying a cross-ref → keep).

---

## Findings

| ID | Content item | Recommended canonical owner + why | Specs restating it (section refs) | Class | Fix recommendation |
|----|--------------|-----------------------------------|-----------------------------------|-------|--------------------|
| D00X-1 | **Nav model construction** — exact staff 3-tab set (`dashboard`/`tickets`/`kbase`), admin 5-tab set, per-tab submenus, hrefs (`tickets.php?status=assigned`, `tickets.php?a=open`), `<panel>.<tab>` submenu keying, FAQs also matching `faq.php`, permission/count gating. | **FS-090** (Shared UI, Navigation & Data Export) — its overview explicitly claims ownership of "the menu construction layer that produces the staff/admin/client navigation" and it carries the full active-state model (FS-090.5–.7). Construction belongs with rendering. | FS-001 §FS-001.13 (full re-spec of all three nav builders), FS-001 Data Requirements "Navigation Model" table, FS-001 §EC-015 (set-missing-tab → false); FS-090 §FS-090.1–.5 | **VERBATIM** | Trim FS-001.13 to a 2–3 line summary ("each realm gate builds a realm-appropriate nav model — public `UserNav`, staff `StaffNav`, admin `AdminNav`; construction + active-state owned by FS-090") + cross-ref. Keep in FS-001 only the *gate-ordering* fact (which gate builds which, and that admin overwrites staff nav). Drop the exact tab/href/submenu enumeration and the Data-Req nav table detail; fold EC-015 into FS-090 (already present as FS-090.5). |
| D00X-2 | **CSRF ownership** — Phase-1/2 flagged seam. Token name `__CSRFToken__`, `X-CSRFToken` header, lazy mint + rotation, derivation from session id + random bytes + secret salt, POST-field-before-header order, link-token = hash(CSRF+salt+session id), failure logged with alert suppressed. | **FS-001** §FS-001.11 — CSRF holder is created at *system startup* by the bootstrap singleton; it is request-lifecycle infrastructure shared by both realms. FS-001.11 is already the most complete statement. | FS-001 §FS-001.11 + §BS-009 + §EC-007 (canonical); FS-002 §FS-002.7 + §BS-002-01/-02 + §FS-002.1 (token mechanics restated: derivation "session id, 16 random bytes, secret salt", rotation, header/field, 400 message); FS-010 §FS-010.3/.5 + §BS-010.6 (rotation restated) | **MIXED** — FS-002.7 is VERBATIM-level on derivation/rotation; FS-010 is ACCEPTABLE (summaries w/ xref to BS-010.6 which is client-login-specific). | Designate **FS-001.11 canonical** for the generic token mechanism. Trim FS-002.7 to: the staff-realm *consequences* (400 "Valid CSRF Token Required", meta tag for AJAX, login-page rotation) + "token mechanism owned by FS-001.11". Keep FS-002's login-rotation rule (BS-002-02) as it is an auth-specific behavior, but cross-ref the derivation to FS-001 rather than re-stating "16 random bytes + secret salt". FS-010 needs no change. |
| D00X-3 | **`UserSession` shared base** — Phase-1/2 flagged seam (FS-002 vs FS-010). Session validation token shape `"<hash>:<time>:<md5(ip)>"` with `hash = md5(time + session secret + account/user id)`; recompute-and-compare validation; idle-timeout via embedded time; sliding refresh; optional IP-binding. | **FS-002** §FS-002.11 — the `UserSession` primitive "physically lives in the staff session classes" (FS-002 scope note line 17 already claims it as shared infrastructure documented here). | FS-002 §FS-002.11 + §BS-002-12 (canonical staff form); FS-010 §FS-010.6 (token shape + hash formula restated **verbatim**, differing only in id=email and IP-binding disabled) | **VERBATIM** (token shape + hash formula identical) | Keep FS-002.11 canonical for the `UserSession` token shape/validation/refresh. Trim FS-010.6 to state only the *client deltas*: id = lowercased email, key = ticket#, IP-binding disabled, uses `client_session_timeout`; replace the duplicated `"<hash>:<time>:<md5(ip)>"` / `hash = md5(...)` lines with "shared token shape owned by FS-002.11 (UserSession)". FS-010.6 already has the right xref note (line 102) — just remove the restated formula. |
| D00X-4 | **Three-level log collapse (Error/Warning/Debug) + write-side mapping** — priorities collapsed to 3 stored levels, threshold suppression unless forced, title sanitize+strip / message sanitize before insert. | Split ownership: **FS-091** §FS-091.8 owns the *enum values* (`log_type` ∈ {Debug,Warning,Error}) and config defaults; **FS-001** §FS-001.12 owns the *write-side collapse + alerting + purge* logic. | FS-001 §FS-001.12 + §BS-012 + §KL-007 (write-side, canonical); FS-003 §FS-003.24 "Cross-reference (write path)" + §BS-015 + §KL-007 (restates the 1=Error/2=Warning/3=Debug priority map); FS-033 §BS-033.2 + §FS-033.2 (viewer mirrors the 3 levels); FS-091 §FS-091.8 (enum) | **MIXED** — FS-003.24 restates the priority→level map (borderline VERBATIM); FS-033/FS-091 are ACCEPTABLE (one already an explicit "Note" deferral; the other is the enum owner). | FS-003.24 already frames its block as a "Cross-reference (write path)" — tighten it to drop the literal priority-bucket enumeration (`emergency/alert/critical/error → Error` …) and point to FS-001.12 + FS-091.8 for the mapping. FS-003 §BS-015 duplicates FS-001 §BS-012 and FS-091 §FS-091.8 — recommend collapsing BS-015 to "Three-level logging (owned by FS-091 enum / FS-001 write-side)". FS-033/FS-091 keep-as-is (summary / enum owner). |
| D00X-5 | **Validators vs account-policy** — Phase-1/2 flagged seam. Username rule (≥2 chars, Unicode letters+`._-`, exact error strings "At least two (2) characters"/"Username contains invalid characters"); phone rule (strip `()-.+ ` → numeric 7–16). | **FS-003** §FS-003.9 + §BS-013 (username), §BS-010 (phone) — these are the standalone format validators in the infrastructure layer; every caller invokes them. | FS-003 §FS-003.9/§BS-013/§BS-010 (canonical); FS-002 §BS-002-22 (username rule + error strings restated); FS-031 §BS-031-017 (username rule + error strings restated **verbatim**) + §BS-031-018 (phone 7–16 restated verbatim) | **VERBATIM** (error strings + 7–16 length reproduced in FS-031; FS-002.22 restates too) | Designate **FS-003.9 canonical** for the format-shape rules + error strings. FS-031 BS-031-017/-018 should cite FS-003 for the literal pattern/error strings and keep only the *admin-save consequence* (rejected on save, stored tag-stripped). FS-002 BS-002-22 should cross-ref FS-003 §BS-013 for the character class/error strings and keep only the uniqueness/system-email-collision rule (which IS auth-domain-specific). |

---

## Inverse check — band content that belongs elsewhere (already correctly deferred)

These were verified as **already cross-referenced, not owned** in the band specs — no fix needed, noted for completeness:

- **Config write handlers** (`update*Settings`) — FS-001 §FS-001.9 overview + Data-Req note + §KL-008 already defer to **FS-032**. Clean.
- **`purgeLogs` detail** — FS-001 §FS-001.12 carries a one-line summary + "(Invoked by cron — FS-043)"; full owner is **FS-033** §FS-033.7. Acceptable summary.
- **`sql_mode=''` reset** — FS-001 §EC-021 explicitly states "Helper contract and consequences owned by FS-003.29 / BS-028". Clean adjudication; FS-003.29/BS-028/EC-023/KL-015 is the single detailed owner. **Keep-as-is.**
- **`getLinkToken`** — FS-003 §FS-003.14 correctly cross-refs FS-001 for the link-token source. Clean.
- **`db_connect`/`db_query`/`db_version` bootstrap subset** — FS-003 §overview + §FS-003.27/.29 + Dependencies explicitly tag these FS-001/FS-060. Clean.
- **Upgrade-pending / schema-signature mechanics** — FS-001 §FS-001.7 + §BS-008 defer stream detail to **FS-061**; FS-061 §BS-061-19 keeps CSRF as a short xref summary. Clean both directions.

---

## Counts

- **Total findings: 5** (plus 6 inverse-checks confirmed clean).
- **VERBATIM (must-fix): 3** fully (D00X-1 nav, D00X-3 UserSession, D00X-5 validators) + **2 mixed** with a verbatim sub-part (D00X-2 CSRF derivation in FS-002.7; D00X-4 log-map in FS-003.24/BS-015).
- **ACCEPTABLE summaries (keep): the FS-010/FS-011/FS-043/FS-061/FS-091 CSRF & log one-liners**, all carrying cross-refs.

## Five most significant duplications (ranked)

1. **D00X-1 Navigation construction** — FS-001.13 fully re-specs the StaffNav/AdminNav/UserNav tab/href/submenu structure that FS-090.1–.5 also fully owns. Largest verbatim surface. **Canonical owner: FS-090** (construction + render together); FS-001 keeps only gate-ordering.
2. **D00X-3 UserSession token** — FS-010.6 reproduces FS-002.11's exact token shape and hash formula. **Canonical owner: FS-002.11**; FS-010 keeps client deltas only.
3. **D00X-2 CSRF token mechanics** — FS-002.7 restates the bootstrap-owned derivation/rotation. **Canonical owner: FS-001.11**; FS-002 keeps staff-realm consequences + login rotation rule.
4. **D00X-5 Validators vs account-policy** — FS-031 (and FS-002.22) reproduce FS-003's username/phone literals and error strings verbatim. **Canonical owner: FS-003.9**; consumers cite it and keep domain consequences.
5. **D00X-4 Three-level log model** — FS-003.24/BS-015 restates the priority→level map owned by FS-001.12 write-side + FS-091.8 enum. **Canonical owners: FS-091.8 (enum) + FS-001.12 (write-side)**; FS-003 tightens to a cross-ref.
