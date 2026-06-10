# P5 Coverage Report — include/class.* chunk B (config + filter + faq..http)

Partition: **P5** — 7 files. Phase-3 COVERAGE tagging pass.
Date: 2026-06-10

Files: `class.config.php`, `class.faq.php`, `class.file.php`, `class.filter.php`,
`class.format.php`, `class.group.php`, `class.http.php`.

## Summary counts

| Metric | Count |
|--------|------:|
| Files examined | 7 |
| Units examined (classes + methods + top-level fns) | ~165 |
| Units tagged | ~120 |
| Units uncovered (no spec) | 0 |
| Trivia-skipped (trivial 1-line accessors covered by class-level tag) | ~45 |

All non-trivial units mapped to at least one existing spec. No unit required an
uncovered row. The ~45 "trivia-skipped" are one-line getters/setters
(`getId`, `getName`, `getHash`, `getNotes`, `isActive`, `getInfo`,
`getHashtable`, `reload`, the long run of `OsticketConfig` boolean config
accessors, etc.). They carry no independent behavior beyond returning a stored
field / config key and are subsumed by the tagged class header (and, for config
keys, by FS-091's config-key catalog). No separate tag inserted to avoid noise.

## Spec mapping applied (by file)

| File | Primary spec(s) | Notes |
|------|-----------------|-------|
| class.config.php | FS-032 (settings tabs / update*Settings), FS-001 ($cfg singleton, online/offline), FS-091 (config keys + Config base accessors) | FS-061 on getVersion/getSchemaSignature/getDBVersion (upgrade); FS-003 on getDBTZoffset / date-format accessors; FS-050 on isKnowledgebaseEnabled; FS-040 on updateEmailsSettings; FS-033 on updatePagesSettings |
| class.faq.php | FS-050 (FAQ CRUD/publish/topics) | FS-022 on attachment add/remove/upload/delete helpers |
| class.file.php | FS-022 (chunked DB file store, download, deleteOrphans) | FS-032 on uploadLogo/allLogos (logo upload); FS-011 on format() restrict validation |
| class.filter.php | FS-042 (filter rule engine, banlist, routing) | FS-041 on isAutoResponse/isAutoBounce (inbound pipeline); FS-003 on endsWith helper |
| class.format.php | FS-003 (format/sanitize/date infra) | FS-021 on display/clickableurls; FS-041 on mimedecode/decodeRfc5987; FS-050 on slugify |
| class.group.php | FS-031 (permission groups, members, dept access) | FS-002 on getDepartments/updateDeptAccess/save (authorization model) |
| class.http.php | FS-003 (HTTP response/redirect/download primitives) | FS-090 also on download() |

## Notable observations (not uncovered, but worth flagging)

1. **`AttachmentFile::uploadLogo` aspect-ratio rule** (class.file.php:~202) —
   rejects images whose width/height ratio is below `$aspect_ratio=3`
   ("Image is too square") and requires the GD extension. This is a concrete
   image-validation business rule that lives under FS-032 (Pages settings logo
   upload) but is not called out as its own BS in that spec's logo handling.
   Suggest verifying FS-032 captures the 3:1 minimum-aspect / GD-required logo
   constraint as an explicit BS/EC.
   > **ADDRESSED (round 2, 2026-06-10)**: FS-032 now carries **BS-032.17** (logo
   > must be a sub-3:1 GIF/JPEG/PNG; "too square" / "invalid image file type"
   > messages), **EC-032.15** (GD-absent ⇒ validation skipped), **KL-032.13**, and
   > a logo-validation bullet in FS-032.6. (The 3:1 threshold + GD gate are in the
   > image helper specified by FS-022.)

2. **`OsticketConfig` legacy fallbacks** (class.config.php:~149 constructor,
   getSchemaSignature, getDBVersion) — three distinct pre-1.7 / pre-namespaced
   config read paths (single-row `id=1` fetch, namespaced schema_signature,
   md5(DBVersion) for 1.6). Covered by FS-061 (upgrader) + FS-001, but the
   "config table shape evolution across versions" is subtle; confirm FS-061 /
   FS-091 document the id=1 single-row legacy layout.

3. **`TicketFilter::quickList` backwards SQL pre-filter** (class.filter.php:~756)
   — the heavily-commented "determine if the rules *might* apply" negative-logic
   short-list query (LOCATE-based, includes `dn_contain`/`not_equal` and
   no-email/name/subject filters). Functionally critical performance path for
   inbound matching; covered by FS-042 but its exact short-listing semantics are
   intricate enough that FS-042 should be checked for an EC describing the
   "quick scan may over-include, matches() is authoritative" contract.
   > **ADDRESSED (round 2, 2026-06-10)**: FS-042 now carries **EC-042-18**
   > documenting the by-design over-inclusion of `quickList` (false positives
   > accepted; `getAllActive()` fallback when no email) and that `Filter::matches()`
   > is the sole authoritative match decision.

## Verification

All edits are additive `// [FS-0XX]` comment lines inserted directly above the
covered unit. No existing source lines modified, moved, or reformatted
(confirmable via `git diff` — comment-line insertions only).
