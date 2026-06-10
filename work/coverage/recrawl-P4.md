# Recrawl Report — P4 (verification round)

Partition: **P4** (`include/class.*` chunk A: ajax→export)
Mode: independent, adversarial fresh-eyes re-crawl. Did NOT trust prior `@implements` tags or the prior `P4-uncovered.md` (which claimed zero findings).
Date: 2026-06-10

Files walked (17): `class.ajax.php`, `class.api.php`, `class.attachment.php`,
`class.banlist.php`, `class.canned.php`, `class.captcha.php`, `class.category.php`,
`class.charset.php`, `class.client.php`, `class.cron.php`, `class.crypto.php`,
`class.csrf.php`, `class.dept.php`, `class.dispatcher.php`, `class.email.php`,
`class.error.php`, `class.export.php`.

## Summary counts

| Metric | Count |
|--------|------:|
| Files examined | 17 |
| Units walked (classes + methods + top-level fns) | ~210 |
| COVERED (tag present + verified) | ~209 |
| UNCOVERED (untagged / undocumented behavior in tagged unit) | 1 |
| MIS-TAGGED (wrong id) | 0 |
| Spot-checks opened against spec (≥30%) | FS-003 (BS-001–007, BS-018/019, FS-003.17/.23), FS-010.3/.4/.5/.8/.11 + BS-010.2, FS-042.8/.11/.12 + BS-042-13/24, FS-040.13 + BS-040.24, FS-043.3/.4/.14 + BS-438, FS-090.27 + BS-090.14, FS-041.5.3 — **all verified correct** |

Verdict: near-DRY. Every class/method/branch maps to a real, correctly-cited
spec id, with **one** genuine undocumented-behavior gap (below). The three
"notable observations" in the prior `P4-uncovered.md` (crypto library ladder,
`data:` attachment grammar, backup file framing) are in fact all already pinned
in spec text (FS-003.4/BS-002, FS-043.4/KL-438, FS-090.27/BS-090.14) and are
NOT true gaps — I confirmed each against the spec.

## Findings (UNCOVERED / MIS-TAGGED)

| File:line | Unit | Behavior | Why it's a gap | Suggested spec | New id? |
|-----------|------|----------|----------------|----------------|---------|
| `include/class.client.php:141-150` | `Client::getLastTicketIdByEmail($email)` (and its consumer `Client::lookupByEmail`, :158-160) | Resolves a client from an email **alone** by selecting one ticket number with `SELECT ticketID ... WHERE email=? ORDER BY created LIMIT 1`. Tagged `@implements FS-010.8` with comment "most-recent ticket # for an email". | The query orders **ascending** with no `DESC`, so it returns the **oldest** ticket, not the most recent — the method name ("Last") and the tag comment ("most-recent") both contradict the actual behavior. More importantly, FS-010.8 documents only the email-scoped *list* + `getClientStats`; it never specifies the `lookupByEmail` / `getLastTicketIdByEmail` email-only client-resolution path or which ticket it picks. This is undocumented behavior inside a tagged unit (and a latent name/comment-vs-code defect of the kind FS-010 already catalogues as KL-010.x). | FS-010 (add to FS-010.8, or a short FR/BS for email-only client resolution) + a KL note that the selector is ordered oldest-first despite the "Last" naming | Yes — new KL id (e.g. KL-010.x) for the oldest-vs-newest selector defect; FS-010.8 acceptance criterion extension (no new top-level FR needed) |

## Notes on units that looked suspicious but are correctly covered

- `class.client.php::login` strike/lockout + "log every other failed attempt as
  warning" + excessive-attempts error → FS-010.5 + BS-010.1/010.2: verified.
- `class.api.php` `ApiXmlDataParser`/`ApiJsonDataParser::fixup` (`data:` URL +
  `charset=` hint that is computed but never stored) → already pinned as KL-438
  on the JSON parser tag; correct.
- `class.crypto.php` three-backend ladder + `$tag$` envelope + subkey/keyHash →
  FS-003.4/.3/.5 + BS-001–005; OpenSSL-IV-from-openssl_random vs Crypto::random
  distinction is in FS-003.5. Correct.
- `class.export.php` `DatabaseExporter` 34-table list, `Signal::send('export.tables')`
  hook, `\x1e` separator, signed header, `die()` on zero-column table →
  FS-090.27 + BS-090.14 + EC-090.10. Correct.
- `class.dispatcher.php` nested sub-dispatcher / `_method` emulation / 400 / 500
  not-callable / unmounted GET/DELETE factories → FS-043.3 + EC-433/446 + KL-440
  + BS-438. Correct.
- `class.canned.php::getFilters()` reads an undeclared `$this->_filters` property
  — code smell only, behavior (filter-name list) is correctly tagged BS-022.6;
  not a coverage finding.

## Round 2 closure (2026-06-10)

The P4 finding is **CLOSED**.

- **`include/class.client.php:141-160` — `getLastTicketIdByEmail` / `lookupByEmail` oldest-vs-newest selector defect:** FS-010.8 acceptance criteria extended with the email-only client-resolution path (`Client::lookupByEmail` → `getLastTicketIdByEmail`, `ORDER BY created LIMIT 1`) and the new **KL-010.11** (Email-Only Resolver Returns The OLDEST Ticket Despite Its "Last" Name) records the missing-`DESC` defect. `class.client.php` `getLastTicketIdByEmail` tag note rewritten: the misleading "most-recent ticket #" comment replaced with `@implements FS-010.8` (email-only resolver) + `@implements KL-010.11` (oldest-ticket defect).
