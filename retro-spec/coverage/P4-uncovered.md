# P4 Coverage Report — Uncovered Units

Partition: **P4** (`include/class.*` chunk A: ajax→export)
Run: Phase-3 COVERAGE tagging, osTicket-1.7
Date: 2026-06-10

Files in partition (17):
`class.ajax.php`, `class.api.php`, `class.attachment.php`, `class.banlist.php`,
`class.canned.php`, `class.captcha.php`, `class.category.php`, `class.charset.php`,
`class.client.php`, `class.cron.php`, `class.crypto.php`, `class.csrf.php`,
`class.dept.php`, `class.dispatcher.php`, `class.email.php`, `class.error.php`,
`class.export.php`.

## Uncovered non-trivial units

| File:line | Unit | Behavior | FS/BS guess | Suggested spec |
|-----------|------|----------|-------------|----------------|
| (none)    | —    | All non-trivial units in this partition mapped to an existing spec. | — | — |

No non-trivial unit was left untagged. Every class/method/function maps to a
documented spec (see Spec mapping below). Constructors that merely call `load()`,
pure getters over `$this->ht[...]`, and `reload()/lookup()` wrappers were tagged
under the same FS as their owning class since the spec documents the entity and
its lifecycle/accessor surface.

## Notable observations (covered, but worth flagging)

1. **`class.crypto.php` — pluggable crypto algorithm dispatch (CryptoMcrypt /
   CryptoOpenSSL / CryptoPHPSecLib + `$N$` tag self-description / auto-upgrade).**
   FS-003 describes encryption/random at a functional level (FS-003.x, BS for
   encrypt/decrypt) but does NOT enumerate the three-library auto-selection
   ladder, the `$tag$base64` envelope grammar, or the subkey/keyHash derivation.
   These are documented behavioral details that FS-003 could call out explicitly
   (suggest adding BS rules for: library preference order OpenSSL→Mcrypt→PHPSecLib;
   `$N$` crypto-tag envelope; per-record subkey mixing). Tagged FS-003 throughout.

2. **`class.api.php` parser fixups (`ApiXmlDataParser`/`ApiJsonDataParser`
   ::fixup).** The `data:` URI attachment decoding (base64 + `charset=` hint →
   UTF-8 via Charset::utf8) and the XML `phone/ext` → `phone_ext` flattening are
   non-obvious normalization rules straddling FS-043 (API request shape) and
   FS-041 (email intake). Tagged [FS-043]/[FS-041]; FS-043 could add an EC/BS for
   the inline `data:` attachment grammar.

3. **`class.export.php` — `DatabaseExporter` full-backup stream.** The fixed
   34-table export order, the `Signal::send('export.tables')` plugin hook, the
   `\x1e` record separator, and the backup header (signature/version/salt/streams)
   are a concrete backup-file format. FS-090 covers CSV/JSON/DB export at a
   functional level but does not pin the record-separator framing or the exact
   table set — candidate for an explicit BS in FS-090 (and cross-ref FS-091 for
   the table list) / FS-061 for the embedded upgrade-stream manifest.

## Summary counts

| Metric | Count |
|--------|------:|
| Files examined | 17 |
| Units examined (classes + methods + top-level fns) | ~205 |
| Units tagged | ~205 |
| Units uncovered (untagged, non-trivial) | 0 |
| Trivia skipped (file header blocks, `?>`, bare `var` decls, define() consts) | n/a (not unit-level) |

## Spec mapping (per file)

| File | Primary FS | Secondary |
|------|-----------|-----------|
| class.ajax.php | FS-043 (Ajax/API base) | FS-090 (json encode), FS-002 (staffOnly) |
| class.api.php | FS-043 (API key + dispatcher controller) | FS-041 (email parser) |
| class.attachment.php | FS-022 | — |
| class.banlist.php | FS-042 | — |
| class.canned.php | FS-022 | — |
| class.captcha.php | FS-011 | — |
| class.category.php | FS-032 (admin FAQ-category CRUD) | FS-050 (read/visibility) |
| class.charset.php | FS-003 | — |
| class.client.php | FS-010 | — |
| class.cron.php | FS-043 | FS-041/FS-021/FS-022/FS-033 (per job) |
| class.crypto.php | FS-003 | — |
| class.csrf.php | FS-002 | — |
| class.dept.php | FS-030 | — |
| class.dispatcher.php | FS-043 | — |
| class.email.php | FS-040 | FS-041 (getMailAccountInfo) |
| class.error.php | FS-003 | FS-060 (InitialDataError) |
| class.export.php | FS-090 | FS-020 (dumpTickets), FS-091 (table set) |
