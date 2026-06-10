# Phase-4 DEDUPE Scan — Band FS-09X (FS-090 Shared UI/Nav/Export, FS-091 Reference Data/Enums/Data Model)

Scope: scan-only. Canonical specs read in full: FS-090, FS-091. All 20 sibling specs skimmed for
restatement of FS-090/FS-091-owned content (enum literals, seeded rows, config defaults, table
schemas, pagination/export/nav mechanics) and the inverse (content in FS-090/FS-091 better owned by
a domain spec).

Legend — fix: **trim-to-crossref** = replace restated literals with a one-line FS-091.N / FS-090.N
reference; **merge** = consolidate two near-identical tables into one canonical home; **keep** =
acceptable one-line summary or genuinely domain-owned behavior, leave as is.

VERBATIM = full literal table / value-list / structure restated. ACCEPTABLE = one-line summary,
in-context behavioral use of a literal, or an already-present cross-reference.

---

## Findings

| ID | Content item | Canonical owner (why) | Restating spec(s) + refs | Verbatim? | Fix |
|----|--------------|-----------------------|--------------------------|-----------|-----|
| D09X-1 | **Seeded priority set** (4 rows: low/normal/high/emergency with `#DDFFDD`/`#FFFFF0`/`#FEE7E7`/`#FEE7E7`, urgency 4/3/2/1, public flags) | **FS-091** (Seeded Priorities table, FS-091.5 / BS-091.3) — canonical seeded-reference home | FS-032:375-389 (full id/tag/desc/color/urgency/public table); FS-060:472-475 (full inline list) | VERBATIM (both) | trim-to-crossref. FS-032 keeps the "no admin CRUD, read-only, default id=2" behavior + a one-line "seed set per FS-091.5"; FS-060 keeps "installer seeds 4 priorities (FS-091 Seeded Priorities)". Drop the color/urgency columns from both. |
| D09X-2 | **Full seeded reference-row block** (Default SLA grace 48; Support/Billing depts; Support/Billing topics; osTicket Default Template + 12 code-names; 3 groups + flags; Cartesian group↔dept; Level I Support team; SYSTEM BAN LIST; 2 canned; osTicket.txt file; 30 timezones; 3 pages) | **FS-091** "Seeded Reference Rows" — single canonical seed catalog | FS-060:466-509 (near-complete restatement of every seeded entity with values) | VERBATIM | trim-to-crossref/merge. FS-060 legitimately owns the seeding ACT (BS-060-13/14 — what the installer writes vs SQL seed) but should reference "the seed rows enumerated in FS-091 Seeded Reference Rows" instead of re-listing each value. Already does this for the config keys (FS-060:508 "Full key list owned by FS-091/FS-032") — apply the same pattern to SLA/depts/topics/templates/groups/team/banlist/canned/file/tz/pages. |
| D09X-3 | **Navigation model: client links + staff tabs/submenus + admin tabs/submenus** (targets, drop-only flags, permission/config conditions, active-state model "mark active by index/href, missing tab returns false") | **FS-090** (FS-090.1–FS-090.9, FS-090.28 — owns nav assembly + render + active-state model) | FS-001:167-181 (FS-001.13 re-enumerates all three structures with targets, submenu items, conditions, and the active-state contract) | VERBATIM | trim-to-crossref. Seam: FS-001's gates *construct* `StaffNav`/`AdminNav`/`UserNav` (legit to say so), but the literal tab/submenu enumerations duplicate FS-090.1–.9/.28. Trim FS-001.13 to "builds the realm nav model defined in FS-090.1–FS-090.9 (client FS-090.4/.28); active-section keys home/kb/new/tickets/status". Keep the *construction-timing* facts (which gate builds which nav, default section). |
| D09X-4 | **Email template type catalog** (13 code-names → display name → description, incl. staff.pwreset notes) | Contested: **FS-040** is the template DOMAIN owner (token catalog, resolution, seeding); **FS-091.7** also claims the code-name "set". The descriptions are identical prose. | FS-040:343-358 AND FS-091:90-106 carry the *same* 13-row table verbatim | VERBATIM (mutual) | merge/trim-to-crossref. Pick one home. Recommend FS-040 owns the catalog+descriptions (it owns tokens/resolution/seeding and already notes staff.pwreset edge cases KL-040.4/.8); FS-091.7 trims to "code-name set owned by FS-040; 12 seeded into the default group, staff.pwreset lazy-loaded" + keeps only the seeded-subject table (FS-091:453-466, which is genuinely seed data). |
| D09X-5 | **Filter match-field / operator / target enums + `(filter_id,what,how,val)` uniqueness** (name/email/subject/body/header; equal/not_equal/contains/dn_contain/starts/ends; Any/Web/Email/API; origin→target map) | **FS-091** (FS-091.9 / BS-091.2 / Enumerations table) for the storage value-lists; **FS-042** owns operator *semantics* + admin picker *labels* | FS-042:45-56, 96-97, 220, 228, 249 (full storage value lists + uniqueness + origin map) | PARTLY VERBATIM | trim-to-crossref for the bare storage value-lists ("`what`∈{...}, `how`∈{...}" → "per FS-091.9"); **keep** the operator semantics (FS-042.3/BS-042-06), the admin-picker label mappings (name→"Name" etc., arguably UI-domain), and the ban-pre-screen scoping. Origin→target map (FS-042:96/220) duplicates FS-091.3 — trim to crossref. |
| D09X-6 | **Ticket export flow specifics** (token error strings "Query token required"/"Query token not found"/"Internal error: Unable to dump query results"; `search_<md5>` session key; `tickets-<YYYYMMDD>.csv` filename; LIMIT-strip) | **FS-090** (FS-090.23 / BS-090.12 — ticket-queue CSV export flow) | FS-020:182-190, 343-350, 421-495 (FS-020.10/BS-020.12 restate the token strings, session key, filename) | PARTLY VERBATIM | keep mostly. FS-020 is the queue domain and consistently cross-references FS-090 ("see FS-090 for export mechanics", lines 190, 536). The exact error strings + filename are duplicated — minor; recommend trim FS-020.10 to reference FS-090.23 for the literal strings/filename, keep the queue-side trigger ("Export link only when ≥1 ticket"). |
| D09X-7 | **Page-size resolution hierarchy + `25` default** (per-request `limit` > per-staff `max_page_size` > system `max_page_size` > 25 / `DEFAULT_PAGE_LIMIT`) | **FS-090** (FS-090.19 / BS-090.7); constant `DEFAULT_PAGE_LIMIT` derived in **FS-001:37** | FS-020:121-124 (full hierarchy restated); FS-001:37,109,125 (derives the 25 fallback + sets PAGE_LIMIT) | PARTLY VERBATIM | keep/trim. FS-001 legitimately owns the *constant derivation* (DEFAULT_PAGE_LIMIT=25 at bootstrap) — keep. FS-020 restates the full precedence chain that FS-090.19/BS-090.7 owns — trim to "PAGE_LIMIT resolved per FS-090.19" + keep the queue-only `limit` override note (which is the queue's own contribution). |
| D09X-8 | **Log severity enum** (Debug/Warning/Error) | **FS-091.8** (storage value-list); **FS-003** owns runtime logging semantics | FS-033:92-126, 454-456 (viewer filter offers Error/Warning/Debug); FS-003 (log-level gating) | PARTLY VERBATIM | keep. FS-033 is the log-viewer domain (filter labels, scope titles) and FS-003 owns log-level gating; the 3-value list is used in-context. Recommend a one-line crossref to FS-091.8 in FS-033's data section; otherwise acceptable. |
| D09X-9 | **Page type enum** (landing/offline/thank-you/other) | **FS-091.10** (storage value-list); **FS-033** owns the page editor; **FS-032** owns the `*_page_id` binding config | FS-033:21-22, 378 (full 4-value list, required-one-of validation) | PARTLY VERBATIM | keep/trim. FS-033 is the page-editor domain; value list used in-context with a crossref to FS-032 for binding (FS-033:54-55). Add a one-line FS-091.10 crossref for the enum; otherwise acceptable. |
| D09X-10 | **Group permission-flag set** (11 flags: can_create/edit/delete/close/assign/transfer_tickets, can_ban_emails, can_manage_premade, can_manage_faq, can_view_staff_stats, can_post_ticket_reply) | Split: **FS-091** entity #16 (logical model) + Seeded Groups; **FS-031** owns the group admin form; **FS-002** owns enforcement | FS-031:255-269, 308 (full flag table); FS-091:357 + Seeded Groups:445 | PARTLY VERBATIM | keep. Clean three-way boundary already stated (FS-031:255 "Enforcement owned by FS-002"; FS-091 owns logical model). The flag-list table appears in both FS-031 and FS-091 but each frames it differently (admin form vs data model). Acceptable; optionally FS-091 entity #16 could crossref FS-031 for the form labels. |
| D09X-11 | **Mail protocol / encryption enums + fetch defaults** (POP/IMAP, NONE/SSL, fetchfreq 5, fetchmax 30, smtp defaults) | **FS-091.12** (value-lists + installed defaults) | FS-040:37,62,308,333 (restates POP/IMAP, NONE/SSL, defaults 5/30); FS-040:485 already crossrefs FS-091 | ACCEPTABLE | keep. FS-040 is the email-account domain owner and already cross-references FS-091 at line 485. In-context behavioral use; no action needed. |
| D09X-12 | **Thread entry types (M/R/N) + ticket event states (created/closed/reopened/assigned/transferred/overdue) + ticket status/source** | **FS-091** (FS-091.3, FS-091.4, FS-091.13) | FS-021 throughout (M/R/N at :60,79,92,306,309; event states at :334; source `Email` at :282); FS-042:249 | ACCEPTABLE | keep. FS-021 is the ticket-workflow domain owner; single-char codes and state strings are used in-context to define behavior (logEvent, thread model). Not a literal-table restatement. No action. |
| D09X-13 | **Staff signature mode (none/mine/dept) + paper sizes** | **FS-091.11** | FS-031:219 (none/mine/dept); FS-021:238 (paper sizes) | ACCEPTABLE + **DIVERGENCE** | keep, but flag: FS-021:238 lists paper sizes as "Letter, Legal, A4, A3" — **missing `Ledger`** vs FS-091.11's 5-value set {Letter,Legal,Ledger,A4,A3}. Either FS-021 is documenting an actual UI subset or it is a drift error. Recommend verify against source; if subset is intentional, note it; else correct FS-021 to crossref FS-091.11. |
| D09X-14 | **`max_file_size`=1048576 / file chunk / content-addressed storage** | **FS-091** (FS-091.14, config table, BS-091.7) | FS-022, FS-040, FS-041, FS-060:505 (1048576 inline) | ACCEPTABLE | keep. The default value 1048576 appears as an installed-config fact in FS-060's "~90 default rows" summary and is used in-context by consumers; FS-091 is the catalog. One-line crossref already implied (FS-060:508). No action. |
| D09X-15 | **Full-DB backup block format** (record-separated `\x1e` JSON blocks, signature/version/prefix/salt/streams metadata, 34-table list) | **FS-090.27 / BS-090.14** (produces it); **FS-091.15** owns the table inventory | FS-061 (consumes backup; references restore) | ACCEPTABLE | keep. FS-061 correctly *consumes/references* the backup without restating the block format or table list (verified: no `\x1e`/signature/table-list restatement in FS-061). Good boundary — FS-090's Dependencies already names FS-061 as the consumer. No action. |

---

## Inverse seam checks (content in FS-090/FS-091 that may belong to a domain spec)

- **Dashboard/report data semantics** (Opened/Assigned/Overdue/Closed/Reopened, Service/Response
  Time, group keys dept/topic/staff): correctly **NOT** in FS-090 — FS-090.26 owns only the CSV/JSON
  *export mechanic* and explicitly cross-references FS-020 for the data. FS-020 owns the semantics.
  Seam is clean. No action.
- **Nav construction timing** (which gate builds which nav): legitimately split — FS-001 owns the
  bootstrap/gate that constructs the model; FS-090 owns the structure + render. The duplication is
  the *literal structure*, handled by D09X-3, not the seam itself.
- **Config-key semantics vs catalog**: FS-032 owns settings editing semantics; FS-091 owns the
  installed-default catalog. Boundary largely respected (FS-040/FS-060 defer to FS-091 for defaults).
  The one overlap is the seeded-priority and SLA *values* (D09X-1, D09X-2) which FS-032 restates.

---

## Summary

- **Total findings: 15** — **6 VERBATIM** (D09X-1,2,3,4,5,6 — though 5 & 6 are partly verbatim),
  **~6 PARTLY VERBATIM** (D09X-5,6,7,8,9,10), **~4 ACCEPTABLE** (D09X-11,12,14,15) + 1 divergence note (D09X-13).
  Cleanly-counted: **4 fully-verbatim restatements** (D09X-1,2,3,4) requiring trim/merge;
  **6 partial** restatements (mostly trim-to-crossref); **5 acceptable** in-context uses.

### 5 most significant (recommended owners)

1. **D09X-2 — FS-060 re-lists the entire seeded reference catalog** → owner **FS-091** (Seeded
   Reference Rows). FS-060 should reference, not restate (it already does this for config keys).
   Highest-volume duplication; values can drift between installer-act spec and reference catalog.
2. **D09X-3 — FS-001.13 re-enumerates all three nav structures** → owner **FS-090** (FS-090.1–.9/.28).
   FS-001 keeps construction-timing; trim the literal tab/submenu/active-state enumerations.
3. **D09X-1 — Seeded priority set duplicated in FS-032 AND FS-060** → owner **FS-091** (FS-091.5).
   Both restate colors/urgency verbatim; trim to crossref, keep domain-specific behavior (no-CRUD note).
4. **D09X-4 — Email template type catalog duplicated verbatim in FS-040 AND FS-091.7** → recommend
   owner **FS-040** (template domain: tokens, resolution, seeding); FS-091.7 trims to a crossref +
   keeps only genuine seed data (seeded-subjects table).
5. **D09X-5 — Filter enums (what/how/target) + origin→target map restated in FS-042** → owner
   **FS-091.9 / FS-091.3** for the bare value-lists; **keep** FS-042's operator semantics + picker
   labels. Trim the storage value-lists and the duplicated origin→target map to crossrefs.
