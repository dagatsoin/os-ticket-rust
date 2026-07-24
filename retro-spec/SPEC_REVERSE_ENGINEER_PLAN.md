# Spec Reverse-Engineering Plan — osTicket-1.7 (RESUME FROM HERE)

> Phase 0 (scouting) completed 2026-06-10. This file is the `/clear`-resilient resume point.
> Method replicates the worked crm_pascal run (`/Users/warfog/dev/crm_pascal/work/SPEC_REVERSE_ENGINEER_PLAN.md`),
> scaled up for a ~290-PHP-file / ~59,600-line multi-file PHP helpdesk (≈7× crm_pascal).
> If context is lost, a fresh session resumes from §3 (the verified domain map) + the trackers in `work/` and `specs/`.

---

## 0. Decisions already made (don't re-ask)

1. **Reverse-engineer functional specs only** from the existing osTicket 1.7 source. Output is documentation, not code.
2. **Worker agent = `spec-writer`** on every working agent spawn (mirrors the crm_pascal run).
3. **Technology-agnostic specs** — describe WHAT the system does / WHAT rules govern it / WHAT data it needs / HOW users interact. Exact literals, field names, enum values, table/column names, and config keys ARE extracted (they are behavioral facts), but no prescription of language/framework/DB/UI-library beyond documenting what the code already does.
4. **Files as memory** — every phase writes a tracker/report so state survives `/clear` or restart.
5. **One FS spec per domain → one Phase-1 agent per spec** → no write conflicts in Phase 1.

---

## M. Method fidelity — the 4-phase loop

| Original manual move | Automated equivalent (this run) |
|---|---|
| `/clear` each pass | Fresh subagent context per task — every agent spawn starts empty. |
| `/iterate` (re-enter spec-writer) | `agentType: 'spec-writer'` on every working agent → FS-XXX/BS-XXX/EC-XXX/KL-XXX format, technology-agnostic, DRY, acceptance-criteria style. |
| Re-paste canonical prompt | Each phase issues its canonical prompt deterministically. |
| Re-anchor on report file | Agents read/write `specs/*.md` + `work/*` trackers from disk. |
| Human decides "keep going?" | Loop-until-dry against `work/UNCOVERED_CODE_REPORT.md` (Phase 3). |
| Preserve main context | Thin orchestrator: agents return compact structured summaries; bulk detail goes to files. |

**Standing scope guard (every agent prompt):** `report spec only` — fix the SPECS, NEVER modify any PHP/source file. The ONLY allowed source edit is **additive coverage comment tags in Phase 3** (idempotent; never alter executable code, only insert comment lines). Tags use the canonical requirement-level `@implements FS-XXX.N: Title — detail` form (one id per line, BS/EC/KL first-class); the original bracketed `[FS-XXX]` form was normalized to `@implements` in Round 3.

---

## 1. Source & target

| | |
|---|---|
| **Source** | `/Users/warfog/dev/osTicket-1.7` — osTicket 1.7 PHP helpdesk, ~290 PHP files / ~59,600 lines |
| **Architecture** | Multi-file procedural-PHP entry scripts + `include/class.*.php` domain classes + `include/staff/*.inc.php` & `include/client/*.inc.php` view templates; MySQL persistence via global `db_query()` helpers; `ost_` table prefix (configurable `TABLE_PREFIX`). |
| **Target** | `/Users/warfog/dev/osTicket-1.7/specs/` (functional specs) + `/Users/warfog/dev/osTicket-1.7/work/` (process artifacts) |
| **Spec format** | `FS-XXX` functional requirements (`FS-XXX.N`), `BS-XXX` business rules, `EC-XXX` edge cases, `KL-XXX` known limitations — technology-agnostic, literals extracted. |
| **Format template (REUSE)** | Hand each agent `/Users/warfog/dev/crm_pascal/specs/FS-001-navigation-app-shell.md` as a worked example + `/Users/warfog/dev/crm_pascal/specs/CLAUDE.md` for conventions. |

### Spec conventions
- `FS-XXX`: Functional Spec. `BS-XXX`: Business Rule. `EC-XXX`: edge case. `KL-XXX`: known limitation. (All but FS embedded within FS docs.)
- File: `FS-XXX-short-description.md`. Structure: **Overview → Functional Requirements → Business Rules → Data Requirements → User Flows/Interactions → Edge Cases → Dependencies → Known Limitations → Future Considerations**.
- Maintain `specs/CLAUDE.md` index + `work/REVERSE_ENGINEER_PROGRESS.md` tracker.

---

## 2. KEY ARCHITECTURAL FINDINGS (Phase-1 agents MUST read this)

1. **Persistence = MySQL.** All state lives in MySQL accessed through global procedural helpers (`db_query`, `db_fetch_array`, `db_input`, `db_result`, `db_assoc_array`, etc., in `include/mysql.php`). Tables use a configurable prefix (default `ost_`, the `%TABLE_PREFIX%` placeholder in the install SQL). ~35 tables — full schema in `setup/inc/streams/core/install-mysql.sql`.
2. **Bootstrap chain.** Every page boots through `main.inc.php` (root) which instantiates the global `$ost` (`osTicket` singleton in `class.osticket.php`) and `$cfg` (`OsticketConfig` in `class.config.php`). Staff pages additionally include `scp/staff.inc.php` (auth gate, CSRF, nav); client pages include `client.inc.php`; API requests include `api/api.inc.php`.
3. **Two session realms.** `class.usersession.php` defines `ClientSession extends Client` (keyed on ticket ID + email, stored in `$_SESSION['_client']`) and `StaffSession extends Staff` (keyed on `staff_id` in `$_SESSION['_staff']`). Sessions persist in the `ost_session` DB table via `class.ostsession.php`. There is no registered "user account" model — clients authenticate per-ticket (ticket number + email/access-link), staff have real accounts.
4. **Authorization model.** Staff belong to a **Group** (`ost_groups`, role permission flags: can create/edit/close/assign/transfer/delete tickets, ban emails, manage premium features, etc.) AND a **Department** (`ost_department`), with **group↔department access** rows (`ost_group_dept_access`). `isadmin` flag on staff unlocks the admin control panel. **Teams** (`ost_team` + `ost_team_member`) are cross-dept assignment groups. Department access + assignment drive ticket visibility.
5. **CSRF + crypto.** `class.csrf.php` issues per-session tokens enforced on every POST (`$ost->checkCSRF…`); `class.crypto.php` (OpenSSL / phpseclib backends) + `PasswordHash.php` (portable bcrypt/phpass) handle secrets/passwords.
6. **Email pipeline is central.** Inbound: POP3/IMAP fetch (`class.mailfetch.php` + `class.pop3.php`) on cron, and pipe (`api/pipe.php` → `include/api.tickets.php`). Parsing via `class.mailparse.php` (Mail_Parse / EmailDataParser) + `class.charset.php`. Outbound: `class.mailer.php`. Email accounts in `ost_email`; templates in `ost_email_template(_group)` (`class.template.php`); autoresponses, alerts, banlist (`class.banlist.php`), and **ticket filters** (`class.filter.php`, rule-based inbound routing/rejection) all hang off this pipeline.
7. **API + cron.** `class.api.php` (API/ApiController + XML/JSON/Email data parsers) with API-key auth (`ost_api_key`, IP-bound, `can_create_tickets`/`can_exec_cron` flags). `api/http.php` routes via `class.dispatcher.php` (UrlMatcher). Cron via `api/cron.php` + `class.cron.php` (fetches mail, runs SLA overdue sweep, purges logs). `scp/autocron.php` is the web-triggered fallback cron.
8. **Ticket is the spine** (`class.ticket.php`, ~1,748 lines — the largest class). Owns creation (web/email/phone/api), the **thread** (`class.thread.php`: Message / Response / Note `ThreadEntry` subclasses in `ost_ticket_thread`), status (open/closed + overdue/answered flags), assignment (staff or team), transfer (department), SLA + due-date, priority, help-topic, locking (`class.lock.php`, `ost_ticket_lock`), events log (`ost_ticket_event`), and email-info threading (`ost_ticket_email_info`).
9. **AJAX subsystem.** `class.ajax.php` (AjaxController extends ApiController) + `class.dispatcher.php` route a family of `include/ajax.*.php` handlers (tickets, users, kbase, reports, config, content, upgrader). Public client AJAX via root `ajax.php`; staff AJAX via `scp/ajax.php`.
10. **Files/attachments are chunked + de-duplicated.** `class.file.php` (`AttachmentFile`, `AttachmentChunkedData`) stores file bytes in `ost_file` + `ost_file_chunk` keyed by content hash; ticket/canned/faq attachment join tables reference them. Avatars/inline images served via `image.php`/`attachment.php`.
11. **Templated/i18n-light.** No gettext; user-facing strings are inline. Email templates are DB-stored with `%{variable}` token substitution (`class.variable.php` VariableReplacer). Output via raw PHP `.inc.php` view partials, navigation assembled by `class.nav.php` (StaffNav/AdminNav/UserNav).
12. **Setup & upgrade are streamed.** `setup/` installer (`class.installer.php`, `class.setup.php` SetupWizard) runs the install SQL; the **upgrader** (`class.upgrader.php`, `class.migrater.php`) applies signed migration "streams" (`include/upgrader/streams/core/*.task.php`, hash-chained filenames) to bring older schemas current. `class.config.php` persists all runtime settings in `ost_config`.
13. **Signals/hooks.** `class.signal.php` provides a lightweight publish/subscribe used to decouple side-effects (e.g. ticket events). Worth a short shared-infra section.

> **Surprising / noteworthy:** client "login" is not account-based (ticket# + email or emailed access link); cron can be web-triggered (`autocron`); the upgrader uses cryptographic hash-chained task filenames; files are content-addressed and chunked in the DB rather than the filesystem.

---

## 3. VERIFIED Phase 0 map — domain → FS spec → source files (the Phase-1 work-list)

Numbering bands:
- **FS-00X** — shared shell, bootstrap, session/auth, CSRF/crypto
- **FS-01X** — public client portal
- **FS-02X** — staff control panel: ticket workflow + dashboard/search
- **FS-03X** — admin configuration
- **FS-04X** — email pipeline, filters, API & cron
- **FS-05X** — knowledge base / FAQ
- **FS-06X** — setup / installer / upgrader
- **FS-09X** — shared components & reference data

Line counts are approximate (entry scripts are small; the weight is in `include/class.*.php` + the matching `include/staff|client/*.inc.php` views).

| FS | Spec | Primary source files (entry + class + views) |
|----|------|----------------------------------------------|
| **FS-001** | App bootstrap & shared request lifecycle | `main.inc.php`*, `client.inc.php`, `scp/staff.inc.php`, `scp/admin.inc.php`, `include/class.osticket.php` (~315), `include/class.config.php` (OsticketConfig, ~837), `include/class.nav.php` (StaffNav/AdminNav/UserNav), `offline.php`, `index.php` |
| **FS-002** | Staff authentication, sessions & access control | `scp/login.php`, `scp/logout.php`, `scp/pwreset.php`, `include/staff/login.tpl.php`, `include/staff/pwreset*.php`, `include/class.staff.php` (~643), `include/class.group.php`, `include/class.usersession.php` (StaffSession), `include/class.ostsession.php`, `include/class.csrf.php`, `include/class.passwd.php`, `include/PasswordHash.php` |
| **FS-003** | Cryptography, validation & formatting infrastructure | `include/class.crypto.php`, `include/class.validator.php`, `include/class.format.php`, `include/class.charset.php`, `include/class.misc.php`, `include/class.signal.php`, `include/class.http.php`, `include/class.error.php`, `include/class.log.php` (`ost_syslog`), `include/class.timezone.php` (`ost_timezone`) |
| **FS-010** | Public client portal — ticket lookup, view & login | `index.php`, `view.php`, `login.php`, `account.php` (if present), `include/class.client.php`, `include/class.usersession.php` (ClientSession), `include/client/{header,footer,login,view,tickets}.inc.php`, `captcha.php`, `include/class.captcha.php` |
| **FS-011** | Public ticket submission (web form) | `open.php`, `include/client/open.inc.php`, ticket-create path in `include/class.ticket.php`, help-topic/priority lookups, attachment handling on submit |
| **FS-020** | Staff ticket queue, dashboard & search | `scp/index.php`, `scp/tickets.php`, `scp/dashboard.php`, `include/staff/tickets.inc.php`, `include/staff/index.php`, `include/class.pagenate.php`, dashboard/reports overlap with `include/ajax.reports.php` |
| **FS-021** | Staff ticket view & workflow (reply/note/assign/transfer/close/reopen/lock) | `scp/tickets.php`, `include/staff/ticket-view.inc.php`, `include/staff/ticket-edit.inc.php`, `include/staff/ticket-open.inc.php`, `include/class.ticket.php` (~1748), `include/class.thread.php`, `include/class.lock.php`, `include/class.pdf.php`, `include/ajax.tickets.php` |
| **FS-022** | Canned responses & ticket attachments | `scp/canned.php`, `include/staff/cannedresponse(s).inc.php`, `include/class.canned.php`, `include/class.attachment.php`, `include/class.file.php`, `attachment.php`, `scp/attachment.php`, `scp/file.php`, `image.php`, `scp/image.php` |
| **FS-030** | Admin — departments, teams & help topics | `scp/departments.php`, `scp/teams.php`, `scp/helptopics.php`, `include/staff/department(s).inc.php`, `include/staff/team(s).inc.php`, `include/staff/helptopic(s).inc.php`, `include/class.dept.php`, `include/class.team.php`, `include/class.topic.php` |
| **FS-031** | Admin — staff, groups & directory | `scp/staff.php`, `scp/groups.php`, `scp/directory.php`, `scp/profile.php`, `include/staff/staff(members).inc.php`, `include/staff/group(s).inc.php`, `include/staff/directory.inc.php`, `include/staff/profile.inc.php`, `include/class.staff.php`, `include/class.group.php` |
| **FS-032** | Admin — system settings, SLA, priorities & categories | `scp/settings.php`, `scp/slas.php`, `scp/categories.php`, `include/staff/settings-*.inc.php` (system/tickets/emails/alerts/autoresp/kb/pages), `include/staff/slaplan(s).inc.php`, `include/staff/categor*.inc.php`, `include/class.sla.php`, `include/class.priority.php`, `include/class.category.php`, `include/class.config.php` |
| **FS-033** | Admin — logs, pages & content | `scp/logs.php`, `scp/pages.php`, `include/staff/syslogs.inc.php`, `include/staff/page(s).inc.php`, `include/class.log.php`, `include/class.page.php`, `include/ajax.config.php`, `include/ajax.content.php` |
| **FS-040** | Email accounts, templates & outbound mail | `scp/emails.php`, `scp/templates.php`, `scp/emailtest.php`, `include/staff/email(s).inc.php`, `include/staff/template(s).inc.php`, `include/class.email.php` (`ost_email`), `include/class.template.php`, `include/class.mailer.php`, `include/class.variable.php` |
| **FS-041** | Inbound email pipeline — fetch, pipe & parse | `api/pipe.php`, `include/api.tickets.php`, `include/class.mailfetch.php` (~537), `include/class.pop3.php`, `include/class.mailparse.php`, `setup/scripts/automail.php`, mail-to-ticket path in `class.ticket.php` |
| **FS-042** | Ticket filters & banlist (inbound routing) | `scp/filters.php`, `scp/banlist.php`, `include/staff/filter(s).inc.php`, `include/staff/banlist.inc.php`, `include/staff/banrule.inc.php`, `include/class.filter.php`, `include/class.banlist.php` |
| **FS-043** | External API & cron scheduler | `api/http.php`, `api/index.php`, `api/api.inc.php`, `api/cron.php`, `include/api.cron.php`, `include/class.api.php`, `include/class.ajax.php`, `include/class.dispatcher.php`, `include/class.cron.php`, `scp/apikeys.php`, `scp/autocron.php`, `include/staff/apikey(s).inc.php` |
| **FS-050** | Knowledge base & FAQ (public + staff management) | `kb/index.php`, `kb/kb.inc.php`, `scp/kb.php`, `scp/faq.php`, `include/client/{faq,faq-category,knowledgebase}.inc.php`, `include/staff/faq*.inc.php`, `include/class.faq.php`, `include/class.knowledgebase.php`, `include/class.category.php`, `include/ajax.kbase.php`, `pages/` |
| **FS-060** | Installer & setup wizard | `setup/index.php`, `setup/install.php`, `setup/inc/class.installer.php`, `include/class.setup.php` (SetupWizard), `setup/inc/*.inc.php`, `setup/inc/ost-sampleconfig.php`, `setup/inc/streams/core/install-mysql.sql` |
| **FS-061** | Upgrader & database migration streams | `setup/upgrade.php`, `scp/upgrade.php`, `include/class.upgrader.php`, `include/class.migrater.php`, `include/upgrader/*.inc.php`, `include/upgrader/streams/core/*.task.php`, `include/ajax.upgrader.php` |
| **FS-090** | Shared UI, navigation & data export | `include/class.nav.php`, `include/staff/{header,footer}.inc.php`, `include/client/{header,footer}.inc.php`, `include/class.pagenate.php`, `include/class.export.php` (CSV/JSON/DB exporters), `include/ajax.reports.php`, `include/staff/tpl.inc.php` |
| **FS-091** | Reference data, enums & data model | `setup/inc/streams/core/install-mysql.sql` (35-table schema), config defaults in `class.config.php`, enum sets across `class.ticket.php`/`class.staff.php`, `include/class.priority.php`, `include/class.sla.php`, ticket-source/status/ref-type enums |

> Total: **21 specs** across 9 bands (00X×3, 01X×2, 02X×3, 03X×4, 04X×4, 05X×1, 06X×2, 09X×2 — FS-091 is the second 09X spec). Overlaps (e.g. ticket-create shared by FS-011/FS-021/FS-041; config keys shared by FS-001/FS-032/FS-091; AJAX handlers spanning FS-020/FS-043) are EXPECTED and resolved in **Phase 4 dedupe** via canonical cross-references.

### Core data model (35 tables, `ost_` prefix — from `install-mysql.sql`)

- **Ticket spine:** `ticket`, `ticket_thread` (Message/Response/Note via `thread_type` M/R/N), `ticket_attachment`, `ticket_lock`, `ticket_email_info`, `ticket_event`, `ticket_priority`.
- **Routing/SLA:** `department`, `team` + `team_member`, `help_topic`, `sla`, `filter` + `filter_rule`.
- **Staff/auth:** `staff`, `groups` + `group_dept_access`, `session`, `syslog`.
- **Email:** `email`, `email_template_group` + `email_template`, plus `banlist` (in filter pipeline).
- **Content:** `faq`, `faq_category`, `faq_attachment`, `faq_topic`, `canned_response`, `canned_attachment`, `page`.
- **Files:** `file`, `file_chunk` (content-addressed chunked storage).
- **Platform:** `config`, `api_key`, `timezone`.

Key relationships: `ticket.staff_id`/`team_id` (assignment) · `ticket.dept_id` (→ `department`, transfer target) · `ticket.topic_id` (→ `help_topic` → default dept/sla/priority) · `ticket.sla_id`/`priority_id` · `staff.group_id`+`dept_id` with `group_dept_access` gating visibility · `ticket_thread.ticket_id` (the conversation) · attachments → `file`/`file_chunk` by content hash.

---

## 4. The Workflow (4 phases — chain all; files as memory)

```
Phase 1 GENERATE  — 1 spec-writer agent per domain (§3, 21 agents). Each reads its file slice +
                    crm_pascal FS-001 worked example + crm_pascal specs/CLAUDE.md conventions; writes
                    specs/FS-XXX-*.md. Then write specs/CLAUDE.md index + work/REVERSE_ENGINEER_PROGRESS.md.

Phase 2 GAP-CLOSE — per spec: find gaps vs implementation (report spec only) → fix in spec →
                    append to work/GAP_REPORT.md.

Phase 3 COVERAGE  — spec-writer agents tag the PHP with @implements ADDITIVE comment tags (idempotent,
                    comment-only, never touch executable code) → produce work/UNCOVERED_CODE_REPORT.md →
                    LOOP-UNTIL-DRY: keep generating specs for uncovered scope until the report is empty
                    (2 consecutive empty/plateau rounds). Coverage asymptotes ~95-98% — accept plateau.

Phase 4 DEDUPE    — cross-file duplicate scan → work/DUPLICATE-ANALYSIS-REPORT.md → resolve to canonical
                    cross-references; adversarial verify CLEAN, no dangling FS-ids.
```

### Hard constraints (baked into every agent prompt)
- **`report spec only`** — never modify any PHP/source file except additive `@implements` comment tags in Phase 3.
- **Technology-agnostic** specs — exact literals/field/table/enum/config values OK; no tech-stack prescriptions beyond documenting observed behavior.
- **Files as memory** — every phase writes a report/tracker so state survives `/clear`.
- **Idempotent tagging** — `@implements` tags are checked-before-inserted; re-running never duplicates.
- **Convergence-driven** — Phase 3 loops until `UNCOVERED_CODE_REPORT.md` is empty/plateaus, not a fixed pass count.

---

## 5. Progress log

- [x] **Phase 0** scouting — DONE (verified map in §3; architecture in §2; band scheme set).
- [x] **Phase 1** GENERATE — DONE (2026-06-10): 21 specs drafted, all Draft (Phase 1). Totals: 318 FRs · 341 BS · 261 ECs · 155 KLs. (Plan estimated 23; executed work-list = 21.)
- [x] **Phase 2** GAP-CLOSE — DONE (2026-06-10): 249 gaps / 249 fixed / 0 open (MISSING 120 · IMPRECISE 84 · INCORRECT 45) across all 21 specs. Detail in `work/gaps/FS-*-gaps.md`; consolidated in `work/GAP_REPORT.md`.
- [x] **Phase 3** COVERAGE — DONE (2026-06-10): 2 rounds, dry. Additive `@implements` tags across 244 in-scope files (P1..P10); declaration-level coverage **100% of in-scope units, DRY after round 2**; FS-092 written in round-2 to absorb the `setup/cli/**` tooling, `db_*` helpers absorbed into FS-003/FS-001. **229 modified files / 2293 additive comment insertions / 2189 `@implements` id lines (FS 1709 + BS 399 + EC 44 + KL 37); 0 bracket tags remain** (TAG_VERIFICATION.md Round 3), all 229 files `php -l` pass under php 5.6 (229/229). Out of scope: 46 vendored + 11 test-harness + 2 redirect stubs. See `work/UNCOVERED_CODE_REPORT.md`.
- [x] **Tag format normalization → @implements requirement-level (2026-06-10):** bracketed `[FS-XXX]` → canonical `@implements FS-XXX.N: Title — detail` (one id per line, BS/EC/KL first-class) per the spec-writer methodology ruling; executed across 10 partitions (logs in `work/coverage/retag-P*.md`). Final: **229 files / 2293 insertions / 0 deletions; 2189 `@implements` ids (FS 1709 + BS 399 + EC 44 + KL 37); 0 bracket tags remain; php 5.6 lint 229/229; byte-identical otherwise** (TAG_VERIFICATION.md Round 3).
- [x] **Phase 4** DEDUPE — DONE (2026-06-10): 7 band scans → 107 raw findings → 38 distinct → 24-item fix worklist applied across 14 specs (13 edit files + FS-091 residuals). 1 factual contradiction resolved (SLA selection precedence: FS-021.13 canonical, FS-030 corrected, FS-091.6 narrowed). Adversarial verification **CLEAN after 4 post-verification fixes** (the flagged `FS-042.137` dangling ref → FS-042.14; 2 low-severity attributed restatements trimmed; 1 out-of-scope FS-031 residual remediated). See `work/DUPLICATE-ANALYSIS-REPORT.md` + `work/dedupe/ADVERSARIAL-VERIFICATION.md`.
- [x] **Final** convergence report — DONE (2026-06-10): `work/FINAL_CONVERGENCE_REPORT.md`. Final corpus (heading-counted): **22 specs / 13,426 lines / 365 FR · 413 BS · 353 EC · 221 KL**. **RUN COMPLETE.**
- [x] **Re-crawl verification round — 14 findings closed, DRY at declaration level, 2026-06-10.** Totals now **365 FR · 415 BS · 354 EC · 224 KL / 13,469 lines** (see `work/UNCOVERED_CODE_REPORT.md` §6 + `work/coverage/TAG_VERIFICATION.md` Round 4).

---

## 6. Immediate next actions on resume

1. (Phase 0 done — skip.)
2. `specs/` and `work/` already exist with skeletons (`specs/CLAUDE.md`, `work/REVERSE_ENGINEER_PROGRESS.md`).
3. Launch **Phase 1 GENERATE** — 21 `spec-writer` agents, one per FS in §3. Each gets: its source-file slice, the crm_pascal `FS-001` worked example, crm_pascal `specs/CLAUDE.md` conventions, §2 architecture findings, and the standing scope guard.
4. **Chain 2 → 3 → 4.** Write trackers/reports as you go; update §5 checkboxes after each phase. Leave a final convergence report on disk.

### Risks / ambiguities to surface before Phase 1
- **FS-091 vs per-domain enums:** the data model/enum reference spec will overlap heavily with ticket/staff/email specs. Plan is to make FS-091 canonical and cross-reference; confirm that's the desired split (vs. embedding enums in each domain spec).
- **Premium KB vs FAQ:** osTicket 1.7 has both a public FAQ (`class.faq.php`) and a staff-side knowledgebase (`class.knowledgebase.php`); both folded into FS-050. Confirm one spec is acceptable rather than splitting public/staff KB.
- **`account.php` / some root scripts** may or may not exist in this 1.7 snapshot — Phase-1 agents should treat missing files in their slice as "not present" rather than erroring.
- **Spec count (executed = 21)** sits inside the requested 18–28 band; if the user wants fewer, FS-030↔FS-031 (dept/team/topic ↔ staff/group) and FS-040↔FS-041 (mail accounts ↔ inbound pipeline) are the natural merge candidates.

_This plan is the resume point. Keep it current._
