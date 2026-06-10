# FS-091 Gap Report — Reference Data, Enums & Data Model

Phase-2 GAP-CLOSE pass over `specs/FS-091-reference-data-enums-data-model.md`.
Sources re-read with fresh eyes: `setup/inc/streams/core/install-mysql.sql` (full schema + seeds),
`include/class.config.php`, `include/class.ticket.php` (genExtRandID / sequential path),
`include/class.misc.php` (randNumber), `include/class.template.php` (all_names), `include/class.filter.php`
(getSupportedMatches / getSupportedMatchTypes / origin2target), `include/class.priority.php`,
`main.inc.php` (constants).

**Scope note:** Spec is overwhelmingly accurate. All enum literal sets, the 13-value email-template
code-name catalog, filter match-fields/operators/targets, origin→target map, ticket source/status
enums, seeded priorities/SLA/departments/help-topics/timezones, and the full installed config-key
catalog (including all alert sub-keys) were verified correct against source and required no change.

## Gaps found / fixed

| ID | Type | Severity | What code/schema does | What spec said | Fix applied |
|----|------|----------|------------------------|----------------|-------------|
| G-091-01 | INCORRECT | High | `install-mysql.sql` contains exactly **34** `CREATE TABLE` statements (verified count; full inventory enumerated). | Repeatedly claimed **35 tables** ("35-Table Logical Model", overview "comprises 35 tables", and a bogus entity #35 that double-counted the `email_template_group`/`email_template` pair to pad to 35). | Corrected all "35" references to **34**; rewrote the overview; renamed the model heading to "The 34-Table Logical Model"; replaced the fabricated entity #35 with an authoritative table-count note; added **FS-091.15** (Installed Table Inventory, exactly 34, with the complete logical-name list) and **KL-091.9** explaining the off-by-one (double-counted template pair). |
| G-091-02 | MISSING | Low | `ticket_email_info` declares `KEY message_id (email_mid)` — index *named* `message_id` but indexing the `email_mid` column; the `message_id` reference column is not separately indexed. | Not mentioned (entity #5 described the reference correctly but omitted the index-naming anomaly). | Added **KL-091.10** documenting the misnamed index (analogue of the `team_id`/`staff_id` anomaly in KL-091.4). |
| G-091-03 | IMPRECISE | Low | Seeded groups `INSERT` omits `can_post_ticket_reply` and `can_view_staff_stats`, so all three groups take column defaults: `can_post_ticket_reply`=1 (all), `can_view_staff_stats`=0 (all, including Admins/Managers). | "Seeded Groups" said Admins/Managers have "all permissions" — overstated, since `can_view_staff_stats` is 0 for every seeded group and `can_post_ticket_reply` is granted only by column default, not by the seed. | Clarified the Seeded Groups paragraph: spelled out that the two unseeded flags fall to defaults (reply=1 all, view-stats=0 all) and that "all permissions" excludes view-staff-stats. |
| G-091-04 | IMPRECISE | Low | `email` table seeds `priority_id` default `2` (=normal); SMTP column defaults `smtp_secure`=1, `smtp_auth`=1, `smtp_active`=0, `smtp_spoofing`=0. | Email Account entity (#20) listed `priority_id` reference with no default; SMTP defaults were only in FS-091.12, not on the entity. | Annotated entity #20 with `priority_id` default `2` (`normal`) and the SMTP boolean defaults for completeness. |

## Verified-correct (no change), spot-checked against source
- Ticket external-number generation: `genExtRandID()` re-rolls on collision; `Misc::randNumber(6)` → `mt_rand("100000","999999")`. Random range 100000–999999 and the re-roll/recursion behavior (FS-091.2, BS-091.4, EC-091.1) confirmed.
- Sequential mode: post-insert `UPDATE ... SET ticketID=ticket_id` with the in-source "RETHING what happens if this fails" comment; on failure the random insert-time id is retained (EC-091.3, KL-091.2) confirmed.
- `EXT_TICKET_ID_LEN`=6, `DEFAULT_PRIORITY_ID`=1 (main.inc.php), `CHUNK_SIZE`=500*1024=512000 (class.file.php) confirmed.
- Filter match fields/operators/targets, picker labels, and origin2target map (`phone`/`staff`→`Web`) confirmed verbatim.
- Email template `all_names` 13-entry catalog (incl. `staff.pwreset` lazy-loaded from `templates/staff.pwreset.txt`, absent from seeded group — KL-091.5) confirmed; 12 seeded templates + subjects confirmed.
- Seeded priorities (4, low/normal/high/emergency with colors + urgency + ispublic; high & emergency share #FEE7E7), seeded SLA (Default SLA, 48h), departments, help topics (crossed sla_id seeds), team, ban-list filter + demo rule, pages (3) + page-id config writeback, timezones (30 rows) confirmed.
- Full installed config-key catalog incl. all alert sub-keys (`*_admin`, `*_dept_manager`, `*_dept_members`, `*_laststaff`, `*_assigned`, `*_staff`, `*_team_lead`, `*_team_members`) confirmed against the install `INSERT`.

## Summary
- Gaps found/fixed: **4** total — INCORRECT ×1, MISSING ×1, IMPRECISE ×2.
- By severity: High ×1, Low ×3.
- New artifacts added to spec: **1 functional requirement** (FS-091.15), **2 known limitations** (KL-091.9, KL-091.10); plus in-place corrections to the overview, the data-model heading, the fabricated entity #35, the Email entity (#20), and the Seeded Groups paragraph.
- No new enums or config keys were missing (existing enum/config inventory verified complete).
