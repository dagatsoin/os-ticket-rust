# osTicket 1.7 — Functional Specifications

## Directory Structure

```
osTicket-1.7/
  (PHP source tree — the subject of these specs; NEVER modified except
   additive @implements comment tags in Phase 3 — format:
   `@implements FS-XXX.N: <Title> — <note>`, one id per line, BS/EC/KL first-class)

  specs/                                   # ← THIS folder: functional specifications ONLY
    CLAUDE.md                              # This file — specs index / conventions
    FS-0XX-*.md … FS-09X-*.md              # The functional specs (see list below)

  retro-spec/                                    # Process artifacts (NOT specs)
    SPEC_REVERSE_ENGINEER_PLAN.md          # Reverse-engineering plan & resume point
    REVERSE_ENGINEER_PROGRESS.md           # Pass tracker
    GAP_REPORT.md                          # Phase 2 gap-closing report
    UNCOVERED_CODE_REPORT.md               # Phase 3 coverage report
    DUPLICATE-ANALYSIS-REPORT.md           # Phase 4 dedup findings
    FINAL_CONVERGENCE_REPORT.md            # Convergence summary
```

> This `specs/` folder contains only the functional specifications and this index. The reverse-engineering process artifacts live in `../retro-spec/`.

## Specification Numbering Scheme

| Range | Module | Status |
|-------|--------|--------|
| FS-00X | Shared shell, bootstrap, session/auth, crypto/validation | Final (Phase 4 de-duplicated) |
| FS-01X | Public client portal (lookup, view, submission) | Final (Phase 4 de-duplicated) |
| FS-02X | Staff control panel (ticket queue, workflow, canned/attachments) | Final (Phase 4 de-duplicated) |
| FS-03X | Admin configuration (depts/teams/topics, staff/groups, settings/SLA, logs/pages) | Final (Phase 4 de-duplicated) |
| FS-04X | Email pipeline, filters, external API & cron | Final (Phase 4 de-duplicated) |
| FS-05X | Knowledge base / FAQ | Final (Phase 4 de-duplicated) |
| FS-06X | Setup / installer / upgrader | Final (Phase 4 de-duplicated) |
| FS-09X | Shared components, navigation, export, reference data & CLI/packaging tooling | Final (Phase 4 de-duplicated) |

## Specification Naming Convention

- **FS-XXX**: Functional Specification (what the system does)
- **BS-XXX**: Business Rule (embedded within FS documents)
- **EC-XXX**: Edge Case / Error Scenario (embedded within FS documents)
- **KL-XXX**: Known Limitation / quirk (embedded within FS documents)
- File format: `FS-XXX-short-description.md`

## Complete Specification List

22 functional specs: 21 written in Phase 1 (GENERATE) + FS-092 added in Phase 3 round-2 to absorb the CLI/packaging/import tooling. All are **Final (Phase 4 de-duplicated)**. Per-spec counts are FR (`FS-XXX.N` requirements) / BS (business rules) / EC (edge cases) / KL (known limitations).

| ID | File | One-line description | FR | BS | EC | KL | Status |
|----|------|----------------------|----|----|----|----|--------|
| FS-001 | `FS-001-app-bootstrap-request-lifecycle.md` | Master per-request initialization sequence: runtime hardening, `$ost`/`$cfg` singletons, session start, CSRF, and the client/staff/admin realm gates + navigation model. | 15 | 14 | 16 | 8 | Final (Phase 4 de-duplicated) |
| FS-002 | `FS-002-staff-authentication-sessions-access-control.md` | Staff login/logout/password-reset, password hashing, DB-backed sessions, CSRF tokens, and the admin/group/department authorization model. | 17 | 24 | 12 | 8 | Final (Phase 4 de-duplicated) |
| FS-003 | `FS-003-crypto-validation-formatting-infrastructure.md` | Shared utility layer: encryption/random, field validation, HTML/format sanitization, charset transcoding, signal hooks, HTTP/error/log primitives. | 25 | 22 | 15 | 9 | Final (Phase 4 de-duplicated) |
| FS-010 | `FS-010-public-client-portal-lookup-view-login.md` | Account-less client portal: ticket# + email / access-link login, brute-force throttle, client session, ticket-thread view, reply, and "My Tickets" list. | 11 | 11 | 12 | 7 | Final (Phase 4 de-duplicated) |
| FS-011 | `FS-011-public-ticket-submission-web-form.md` | Public "Open a New Ticket" web form: validation, help-topic routing, CAPTCHA/banlist/open-ticket throttle gating, attachments, autoresponse/alert emails. | 13 | 8 | 9 | 5 | Final (Phase 4 de-duplicated) |
| FS-020 | `FS-020-staff-ticket-queue-dashboard-search.md` | Staff ticket queue (tabs, visibility scoping, sort/paginate, mass actions, CSV export), basic/advanced search, and the activity dashboard. | 13 | 15 | 12 | 7 | Final (Phase 4 de-duplicated) |
| FS-021 | `FS-021-staff-ticket-view-workflow.md` | Single-ticket workflow: reply/note, assign/claim/release/transfer, close/reopen, overdue, ban, edit/delete, due-date/SLA, locking, phone-create, PDF print. | 23 | 18 | 17 | 8 | Final (Phase 4 de-duplicated) |
| FS-022 | `FS-022-canned-responses-ticket-attachments.md` | Canned-response library (variable substitution, dept scope, up to 10 attachments) and the content-addressed, chunked DB-backed file store + attachment download. | 14 | 13 | 12 | 6 | Final (Phase 4 de-duplicated) |
| FS-030 | `FS-030-admin-departments-teams-help-topics.md` | Admin CRUD for departments, teams, and help topics — the routing/organization objects that categorize, route, assign, prioritize, and SLA-bind tickets. | 17 | 30 | 14 | 10 | Final (Phase 4 de-duplicated) |
| FS-031 | `FS-031-admin-staff-groups-directory.md` | Admin CRUD for staff accounts and permission groups, the staff directory browse/search, and each staff member's self-service profile editing. | 11 | 27 | 12 | 6 | Final (Phase 4 de-duplicated) |
| FS-032 | `FS-032-admin-system-settings-sla-priorities-categories.md` | Seven-tab system settings (~110 config keys), SLA plans, ticket priorities, and FAQ-category CRUD. | 16 | 16 | 12 | 7 | Final (Phase 4 de-duplicated) |
| FS-033 | `FS-033-admin-logs-pages-content.md` | Admin system-log viewer (filter/sort/paginate `syslog`) plus the read-only content/configuration AJAX endpoints serving the control panel. | 10 | 7 | 10 | 5 | Final (Phase 4 de-duplicated) |
| FS-040 | `FS-040-email-accounts-templates-outbound-mail.md` | Email-account configuration, email-template sets, the `%{token}` variable-substitution grammar, and outbound mail composition/dispatch. | 13 | 25 | 12 | 8 | Final (Phase 4 de-duplicated) |
| FS-041 | `FS-041-inbound-email-pipeline-fetch-pipe-parse.md` | Inbound email-to-ticket pipeline: IMAP/POP3 poll, local MTA pipe, and HTTP email-post intake converging on parse → threading → create/append. | 12 | 16 | 13 | 7 | Final (Phase 4 de-duplicated) |
| FS-042 | `FS-042-ticket-filters-banlist-inbound-routing.md` | Ordered ticket-filter rule engine (reject/mutate inbound tickets across all channels) and the reserved `SYSTEM BAN LIST` filter + ban/unban actions. | 12 | 19 | 12 | 7 | Final (Phase 4 de-duplicated) |
| FS-043 | `FS-043-external-api-cron-scheduler.md` | IP-bound API-key management, the HTTP API dispatcher (ticket-create + remote cron routes), and the scheduled-maintenance cron subsystem. | 13 | 9 | 17 | 8 | Final (Phase 4 de-duplicated) |
| FS-050 | `FS-050-knowledge-base-faq.md` | Public knowledge base (browse/read/search published FAQs) and staff FAQ management (create/publish/attach/associate), gated by article + category visibility flags. | 18 | 7 | 9 | 7 | Final (Phase 4 de-duplicated) |
| FS-060 | `FS-060-installer-setup-wizard.md` | One-time web setup wizard: prerequisite/config checks, DB create + schema-stream load, default-data seeding, admin-account creation, config-file rewrite. | 9 | 18 | 11 | 8 | Final (Phase 4 de-duplicated) |
| FS-061 | `FS-061-upgrader-database-migration-streams.md` | In-app upgrader: hash-chained schema patch streams + resumable batched data-migration tasks, driven by an admin-only AJAX progress loop, fatal-on-error. | 15 | 17 | 14 | 9 | Final (Phase 4 de-duplicated) |
| FS-090 | `FS-090-shared-ui-navigation-data-export.md` | Cross-cutting presentation infra: navigation assembly, page chrome (header/footer/message bars), pagination, and the CSV/JSON/DB export pipeline. | 27 | 14 | 11 | 8 | Final (Phase 4 de-duplicated) |
| FS-091 | `FS-091-reference-data-enums-data-model.md` | Canonical logical data model (34 tables), every enum/reference set, seeded reference rows, and all configuration keys with installed defaults. | 14 | 12 | 10 | 8 | Final (Phase 4 de-duplicated) |
| FS-092 | `FS-092-cli-management-deployment-packaging.md` | Shell-only CLI tooling subsystem (`setup/cli/**`): action-dispatch + argparse framework, deployment/unpack with `INCLUDE_DIR` rewrite, release packager (version stamp + `display_errors` hardening + tar/zip), and the backup import/restore pipeline (counterpart to FS-090.27's exporter). | 16 | 13 | 13 | 7 | Final (Phase 4 de-duplicated) |
| **Totals (authoritative, heading-counted)** | **22 specs** | **13,469 lines** | **365** | **416** | **355** | **225** | **Final (Phase 4 de-duplicated)** |

> **Counting note:** the per-spec FR/BS/EC/KL columns above are the interim Phase-1 + gap-close
> estimates carried forward from earlier passes (they sum to ~334/354/274/162). The **Totals row
> is the authoritative figure** — recomputed by the careful definition-heading method (counting
> `### FS/BS/EC/KL-…:` headings and `**FS/BS/EC/KL-… — …**` bold-bullet definitions, not raw token
> mentions) after the Phase-2 gap-close additions, the Phase-4 dedupe trims/corrections, and
> the 2026-06-10 re-crawl closure round (FS-010 +2 BS/+1 EC/+2 KL, FS-040 +1 KL), and the
> modernisation SMTP transport-security update (FS-040 +1 BS/+1 EC/+1 KL — BS-040.30 / EC-040.15 / KL-040.12):
> **365 FR · 416 BS · 355 EC · 225 KL** across **13,469 lines**. See
> `../retro-spec/FINAL_CONVERGENCE_REPORT.md` for the method and per-spec line counts.

## Source Reference

These specifications are reverse-engineered from:
- **Location**: `/Users/warfog/dev/osTicket-1.7`
- **Type**: osTicket 1.7 — a multi-file procedural-PHP web helpdesk / support-ticket system.
- **Scale**: ~290 PHP files / ~59,600 lines.
- **Persistence**: MySQL, accessed through global procedural helpers (`db_query`, `db_input`, etc.); ~35 tables with a configurable prefix (default `ost_`). Full schema in `setup/inc/streams/core/install-mysql.sql`.
- **Architecture**: small per-feature entry scripts boot through `main.inc.php` (global `$ost` osTicket singleton + `$cfg` config); staff pages gate through `scp/staff.inc.php` (auth/CSRF/nav); domain logic in `include/class.*.php`; views in `include/staff/*.inc.php` + `include/client/*.inc.php`.
- **Two session realms**: per-ticket client sessions (ticket# + email / access link) and account-based staff sessions; both persisted in the `ost_session` table.
- **Authorization**: staff Group permission flags + Department access (+ group↔dept access rows); `isadmin` unlocks the admin panel; Teams provide cross-department assignment.

## Technology Agnosticism

All specifications describe:
- WHAT the system does (functional requirements)
- WHAT rules govern behavior (business rules)
- WHAT data is needed (data requirements — exact table/column/enum/config names extracted as behavioral facts)
- HOW users interact (user flows)

They do NOT prescribe:
- Programming languages or frameworks
- Database technologies or specific SQL implementations
- API/transport implementations
- UI component libraries

(Exact literals — field names, enum values, table names, config keys, status flags — ARE extracted, because they are facts about system behavior, not technology prescriptions.)

## Creating New Specifications

1. Check `../retro-spec/REVERSE_ENGINEER_PROGRESS.md` for the next available FS number and current phase.
2. Create the file in `specs/` following the naming convention.
3. Use the standard structure: Overview → Functional Requirements → Business Rules → Data Requirements → User Interactions → Edge Cases → Dependencies → Known Limitations → Future Considerations.
4. Update the progress tracker in `../retro-spec/`.

## Reverse Engineering Status

**STATUS: ALL 4 PHASES COMPLETE — RUN COMPLETE (2026-06-10).** 22 functional specs (21 from Phase 1 + FS-092 added in Phase 3 round-2), all **Final (Phase 4 de-duplicated)**. Authoritative totals (heading-counted, post gap-close + dedupe): **13,426 lines · 365 FRs · 413 BS rules · 353 ECs · 221 KLs**. Phase 2 (GAP-CLOSE): 249 gaps found / 249 fixed / 0 open. Phase 3 (COVERAGE): declaration-level coverage **100% of in-scope units, DRY after round 2**; **229 files / 2293 additive `@implements` comment tags / 2189 ids (FS 1709 + BS 399 + EC 44 + KL 37); 0 bracket tags remain** (TAG_VERIFICATION.md Round 3) — tags use the requirement-level form `@implements FS-XXX.N: <Title> — <note>`, one id per line, BS/EC/KL first-class (normalized from bracketed `[FS-XXX]` in Round 3) — see `../retro-spec/UNCOVERED_CODE_REPORT.md`. **Phase 4 (DEDUPE): COMPLETE** — 107 raw findings → 38 distinct → 24-item worklist across 14 specs; the single SLA-precedence contradiction resolved; **adversarial verification CLEAN after 4 post-verification fixes** (incl. the `FS-042.137` dangling ref → FS-042.14). Out of scope: 46 vendored third-party files, 11 self-test harness files, 2 bare redirect stubs. See `../retro-spec/FINAL_CONVERGENCE_REPORT.md` for the convergence summary, `../retro-spec/DUPLICATE-ANALYSIS-REPORT.md` + `../retro-spec/dedupe/ADVERSARIAL-VERIFICATION.md` for the dedupe pass, and `../retro-spec/SPEC_REVERSE_ENGINEER_PLAN.md` for the verified domain map and 4-phase workflow.
