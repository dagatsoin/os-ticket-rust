# P6 Coverage Report — include/class.* chunk C (json→sla)

Partition: **P6** (20 files). Phase-3 COVERAGE tagging run, 2026-06-10.
Repo: `/Users/warfog/dev/osTicket-1.7`

Method: every class/function/method/significant top-level unit walked. Additive
`// [FS-0XX]` tags inserted above covered units (comment-only, idempotent). Trivial
getters/setters, accessor one-liners, and thin library-override shims left untagged
(covered transitively by their owning class/method tag).

---

## Uncovered / Notable non-trivial units

| File:line | Unit | Behavior (1-line) | FS/BS guess | Suggested spec |
|-----------|------|-------------------|-------------|----------------|
| include/class.pop3.php:1-3 | (whole file) | Dead stub — comment-only file kept to clear an old `class.pop3.php`; "No longer used... will be deleted". No code. | n/a (dead code / KL) | FS-041 (note as KL — legacy POP3 handler removed; live POP3 path is IMAP-ext driven in MailFetcher) |
| include/class.knowledgebase.php:18 (whole class) | `Knowledgebase` | Tagged FS-050 but is **dead/legacy** code: constructor + all CRUD read/write `CANNED_TABLE`/`canned_id`, never instantiated by any entry script. Documented as KL-050.5. | FS-050 KL-050.5 | FS-050 (already documents it as dead code — consider explicit dead-code BS in FS-022 too since it shadows the canned table) |
| include/class.pdf.php:105 | `Cell()` override | Thin FPDF `Cell` wrapper that UTF-8-transcodes text before delegating to parent; rendering detail of the ticket PDF. | FS-021 (transitive) | FS-021 (left untagged; covered by `_print`/class tag — flagged only as a rendering shim) |
| include/class.pdf.php:109 | `WriteText()` | MultiCell text-body writer helper for PDF; rendering detail. | FS-021 (transitive) | FS-021 (left untagged; covered by class tag) |
| include/class.mailparse.php:439 | `EmailDataParser::err()` / `getError()` / `lastError()` | Trivial error-state accessors on the pipe/HTTP intake parser. | FS-041 (transitive) | FS-041 (left untagged as trivia; `parse()`/class tagged) |

No genuinely **undocumented** non-trivial behavior was found in P6. Every substantive
class and method maps cleanly onto an existing spec. The two real "uncovered" items are
*dead code* (`class.pop3.php`, and the legacy `Knowledgebase` class which is already
called out as KL-050.5).

---

## Spec mapping summary (per file)

| File | Primary spec(s) |
|------|-----------------|
| class.json.php | FS-043 (decode/API intake), FS-090 (encode/export) |
| class.knowledgebase.php | FS-050 (KL-050.5 dead class on canned table) |
| class.lock.php | FS-021 (ticket lock), FS-043 (cron cleanup) |
| class.log.php | FS-033 (syslog model) |
| class.mailer.php | FS-040 (outbound mail) |
| class.mailfetch.php | FS-041 (IMAP/POP3 fetch), FS-043 (run via cron) |
| class.mailparse.php | FS-041 (mime parse + EmailDataParser) |
| class.migrater.php | FS-061 (migration), FS-001 (getUpgradeStreams used by bootstrap) |
| class.nav.php | FS-090 (nav assembly), FS-001 (realm nav model), FS-050 (KB link) |
| class.osticket.php | FS-001 (bootstrap/singleton), FS-003 (log/util/signal), FS-002 (CSRF/link token), FS-022 (file-type), FS-040 (template vars), FS-033 (log/purge), FS-043 (cli) |
| class.ostsession.php | FS-002 (DB session), FS-001 (start), FS-020 (online users) |
| class.page.php | FS-032 (site pages), FS-011 (thank-you pages) |
| class.pagenate.php | FS-090 (pagination) |
| class.pdf.php | FS-021 (ticket PDF print) |
| class.pop3.php | dead stub (no coverage) |
| class.priority.php | FS-032 (priorities), FS-091 (reference data), FS-011 (public priorities) |
| class.setup.php | FS-060 (installer wizard), FS-061 (load_sql for upgrader) |
| class.signal.php | FS-003 (pub/sub signal model) |
| class.sla.php | FS-032 (SLA plans + SlaConfig) |
| class.staff.php | FS-031 (admin staff CRUD/profile), FS-002 (auth/login/reset), FS-020/FS-021 (stats/availability lists) |

---

## Counts

| Metric | Count |
|--------|------:|
| Files examined | 20 |
| Files tagged | 19 (class.pop3.php is a dead stub — nothing to tag) |
| Units examined (classes + non-trivial methods/functions) | ~178 |
| Units tagged | ~150 |
| Units uncovered (genuinely undocumented) | 0 |
| Dead-code units (noted, not tagged or tagged-as-KL) | 2 (class.pop3.php file; Knowledgebase class = KL-050.5) |
| Trivia skipped (getters/setters/accessor one-liners/lib shims) | ~26 |
