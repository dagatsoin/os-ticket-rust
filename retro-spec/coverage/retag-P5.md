# Retag P5 — include/class.* chunk B (config/filter/faq..http)

Conversion of Phase-3 spec-level `// [FS-0XX]` coverage tags to requirement-level
`@implements` annotations. Comment-only edits; no HEAD source lines touched.

Partition files (7): class.config.php, class.faq.php, class.file.php,
class.filter.php, class.format.php, class.group.php, class.http.php

## Summary

| File | Old tag lines | New @implements lines | FS.N | FS fallback | BS | EC/KL |
|------|--------------:|----------------------:|-----:|------------:|---:|------:|
| class.config.php | 22 | 37 | 23 | 9 (FS-091/FS-001/FS-061 spanning) | 5 | 0 |
| class.faq.php    | 25 | 31 | 23 | 0 | 7 | 1 (EC-050.12) |
| class.file.php   | 27 | 36 | 23 | 0 | 13 | 0 |
| class.filter.php | 47 | 66 | 47 | 0 | 18 | 1 (EC-042-18) |
| class.format.php | 25 | 29 | 29 | 0 | 0 | 0 |
| class.group.php  | 13 | 17 | 5 | 0 | 12 | 0 |
| class.http.php   | 5  | 5  | 5 | 0 | 0 | 0 |
| **Total** | **164** | **221** | **155** | **9** | **55** | **2** |

(Counts above are per-annotation-line; some code units carry multiple
`@implements` lines — one id per line — so new-line totals exceed old tag-line
counts. FS.N = requirement-level FS-XXX.N; FS fallback = bare spec-level FS-XXX;
BS = first-class business-rule ids; EC/KL = edge-case / known-limitation ids.)

## Per-file notes

### class.filter.php (FS-042)
- Three classes (Filter, FilterRule, TicketFilter) + RejectedException + endsWith.
- `matches()` → FS-042.3 + FS-042.4 + BS-042-26 (email_id scope guard).
- `apply()` (TicketFilter) → FS-042.7 + BS-042-02 (reject absolute) + BS-042-03 (stop-on-match).
- `save_rules()` → FS-042.3 + BS-042-08/09/11/24 (rule cap, mass-replace, zero-rule banlist bypass).
- `isBanned()` → FS-042.8 + BS-042-16 + KL-042.8 (equal-as-contains mis-code).
- `quickList()` → FS-042.7 + EC-042-18 (deliberate over-inclusion; matches() authoritative).
- `isAutoResponse()`/`isAutoBounce()` → FS-042.9 + BS-042-19 (start-anchored markers). HEAD
  carried adjacent `[FS-041]` (consumer pipeline); FS-042 is the canonical owner so the
  marker vocabulary is tagged to FS-042.9, FS-041 noted in prose.
- `endsWith()` HEAD carried `[FS-042] [FS-003]`; it backs the `ends` operator → tagged FS-042.4
  (generic-helper FS-003 dropped to keep the load-bearing operator id).

### class.faq.php (FS-050)
- FAQ + (Category consumed, not defined here).
- `isPublished()` → BS-050.1 (published AND public). `countPublishedFAQs()` → BS-050.2.
- `save()` → FS-050.17 + BS-050.4 (question uniqueness) + EC-050.12 (id-tamper guard).
- `find*ByQuestion()` → BS-050.4. `lookup()` → FS-050.3 (strict, vs Category::lookup hollow).
- Attachment methods HEAD-tagged `[FS-050] [FS-022]`: storage is FS-022, but the FAQ↔file
  association is FS-050.9 → tagged FS-050.9 with "(storage owned by FS-022)" note.

### class.file.php (FS-022)
- AttachmentFile + AttachmentChunkedData.
- Storage core → FS-022.12; delivery (display/download/makeCacheable) → FS-022.11 + BS-022.12 (304).
- `format()` HEAD `[FS-022] [FS-011]` → FS-022.13 + BS-022.13 (ext-only) + BS-022.14 (default-deny).
- `uploadLogo()`/`allLogos()` HEAD `[FS-022] [FS-032]` → FS-022.15 (logo store; ft='L' exempt).
- Reference-count / orphan reclamation → BS-022.10 + BS-022.11 (ticket-type-only sweep).

### class.group.php (FS-031)
- Group entity. Mostly business rules: BS-031-019 (name/min-len), BS-031-020 (perm flag set),
  BS-031-021 (group↔dept matrix), BS-031-022 (zero-member delete), BS-031-024 (disabled exemption),
  BS-031-034 (list counts). HEAD `[FS-002]` adjacents (access enforcement) noted in prose only —
  Group owns the permission/access *data*, FS-002 enforces it.
- Form CRUD entry points → FS-031.8.

### class.http.php (FS-003)
- All four Http helpers → FS-003.22. `download()` HEAD `[FS-003] [FS-090]` kept on FS-003.22
  with "(used by FS-090 export)" note.

### class.format.php (FS-003)
- Pure requirement-level: FS-003.10 (safe_html/sanitize), FS-003.11 (encode/decode/striptags),
  FS-003.12 (display/truncate/wrap/stripEmptyLines), FS-003.13 (strip_slashes),
  FS-003.14 (clickableurls), FS-003.15 (file_size/phone/slugify/elapsedTime/array_implode),
  FS-003.16 (mimedecode/decodeRfc5987), FS-003.19 (date helpers).
- HEAD adjacents `[FS-041]` (mimedecode/decodeRfc5987), `[FS-021]` (display/clickableurls),
  `[FS-050]` (slugify) were consumer references → resolved to the FS-003 owning requirement.

### class.config.php (FS-001/FS-032/FS-091/FS-061/FS-050/FS-003)
- Config base class is the (namespace,key) store — no single dedicated FS-091.N exists for the
  persistence accessors, so generic accessors fall back to **spec-level FS-091** (Config entity
  #32, BS-091.7 installed-defaults, BS-091.11 auto-updating `updated`). This is the only file
  with genuine spec-level fallback.
- Persistence mutators (`set`/`create`/`update`/`updateAll`) → FS-032.7 (settings persistence).
- Per-tab `update*Settings()` → FS-032.2 (dispatch) + FS-032.3/.4/.5/.6 (each tab).
- `isOnline()`/online-offline → FS-001 (bootstrap online flag). `isKnowledgebaseEnabled()` → BS-050.2.
- `getVersion()`/`getSchemaSignature()`/`getDBVersion()` → FS-061 (upgrader).
- `getDBTZoffset()`/`observeDaylightSaving()` → FS-003.19 (time conversion).

## Ambiguous cases (resolved): 2

1. **class.config.php generic accessors (get/exists/persist/getNamespace/lastModified)** —
   no requirement-level FS-091.N covers the runtime config-store accessors (FS-091's numbered
   requirements are reference-data/enum sets; the Config entity is described in the data-model
   table as entity #32). Resolved to **spec-level FS-091** + the two governing business rules
   (BS-091.7, BS-091.11). This is a genuine whole-spec-spanning case, not laziness.

2. **class.filter.php `endsWith()` + class.faq.php attachment methods + class.format.php
   consumer-tagged helpers** — HEAD double-bracket tags mixed an owner spec with a consumer
   spec (FS-041/FS-021/FS-050/FS-022). Resolved by tagging the **owning** requirement and
   demoting the consumer reference to a prose note in the annotation, per the granularity rule
   ("prefer the main requirement id").

No unresolved ambiguities remain; every code unit maps to a concrete FS-XXX.N or first-class
BS/EC id except the documented FS-091 whole-spec fallback in class.config.php.
