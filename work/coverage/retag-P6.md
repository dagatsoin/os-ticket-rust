# Retag P6 — `include/class.*` chunk C (json → sla)

Conversion of Phase-3 `// [FS-0XX]` coverage tags into requirement-level `@implements`
annotations. Partition P6 (20 files; `class.pop3.php` carried no tags so 19 files were
edited). Comment-only edits.

## Per-file summary

| File | Old `[FS-]` tag lines | New `@implements` lines | FS.N | FS fallback | BS | EC/KL |
|------|----------------------:|------------------------:|-----:|------------:|---:|------:|
| class.json.php | 6 | 6 | 6 | 0 | 0 | 0 |
| class.knowledgebase.php | 10 | 10 | 1 (FS-050 class-level) | 0 | 0 | 9 (KL-050.5) |
| class.lock.php | 12 | 15 | 13 | 0 | 2 (BS-021.3) | 0 |
| class.log.php | 4 | 5 | 5 | 0 | 0 | 0 |
| class.mailer.php | 6 | 9 | 6 | 0 | 3 (BS-040.21/22/28) | 0 |
| class.mailfetch.php | 22 | 35 | 22 | 0 | 13 (BS-041.x) | 0 |
| class.mailparse.php | 19 | 22 | 16 | 0 | 6 (BS-041.10/11/17) | 0 |
| class.migrater.php | 8 | 13 | 9 | 0 | 4 (BS-061-04/06/09/15) | 0 |
| class.nav.php | 15 | 15 | 15 | 0 | 0 | 0 |
| class.osticket.php | 24 | 33 | 30 | 0 | 3 (BS-022.13/BS-033.2/BS-033.6) | 0 |
| class.ostsession.php | 9 | 11 | 11 | 0 | 0 | 0 |
| class.page.php | 13 | 18 | 16 | 0 | 2 (BS-033.8/BS-033.9) | 0 |
| class.pagenate.php | 6 | 8 | 7 | 0 | 1 (BS-090.8) | 0 |
| class.pdf.php | 7 | 7 | 7 | 0 | 0 | 0 |
| class.priority.php | 7 | 10 | 8 | 0 | 2 (BS-091.3) | 0 |
| class.setup.php | 8 | 10 | 10 | 0 | 0 | 0 |
| class.signal.php | 3 | 3 | 3 | 0 | 0 | 0 |
| class.sla.php | 14 | 19 | 16 | 0 | 3 (BS-032.9 x2 / BS-032.12) | 0 |
| class.staff.php | 28 | 45 | 38 | 0 | 7 (BS-031-001/005/006/007/014/015/016) | 0 |
| **Total** | **221** | **294** | **239** | **0** | **49** | **9** |

(Counts are `@implements` lines; many code units now carry 2–3 first-class ids on
separate lines per the format spec. "FS.N" = requirement-level `FS-XXX.N`; BS / KL counted
where a first-class `BS-`/`KL-` line was emitted.)

## Spec-level fallbacks

No bare spec-level `FS-XXX` fallbacks were required EXCEPT two deliberate class-level
spec-spanning tags (allowed by the format spec for whole-spec/legacy classes):
- `class.knowledgebase.php` class header carries one `FS-050` class-level line ALONGSIDE
  the primary `KL-050.5` id — the class is documented dead/legacy code.
- A handful of cross-cutting consumer references use a bare spec id where the unit is a
  thin hook into another whole subsystem and no single requirement fits better:
  `FS-011` (priority/page public-form consumers), `FS-020` (staff stats / online-users
  consumers), `FS-050.1` (nav KB-link toggle). These are paired with a precise primary id
  on the same unit, not used as the sole tag.

## Notable id re-attributions (Phase-3 tag was imprecise / wrong)

- **class.page.php**: Phase-3 tagged the `Page` (Site Pages) entity `[FS-032]`. Site Pages
  are owned by **FS-033** (FS-033.12/13/14/16, BS-033.8/9). Re-pointed to FS-033.
- **class.knowledgebase.php**: Phase-3 `[FS-050]` on a class the FS-050 spec explicitly
  flags as dead/legacy bound to the canned table → primary id is **KL-050.5** (spec's own
  known-limitation id), FS-050 kept only as a class-level co-tag.
- **class.osticket.php** logging cluster: Phase-3 `[FS-003]` → mapped to **FS-001.12**
  (System Logging & Admin Alerting), the precise system-logging requirement, with BS-033.2
  on the level-collapsing `log()`.
- **class.migrater.php** `getUpgradeStreams`: dropped the incidental `[FS-001]` co-tag;
  pure upgrader-domain → FS-061.2 / FS-061.8.
- **class.nav.php** / **class.ostsession.php**: dropped incidental `[FS-001]` bootstrap
  co-tags; nav is FS-090.x, session store is FS-002.14.

## Ambiguous cases (logged) — 4

1. **JsonDataEncoder (class.json.php)** — the shared JSON encoder. FS-090 Dependencies note
   calls it "JSON encoder (FS-003 infrastructure)" but FS-003 has no requirement for it and
   FS-090.22 (JSON Exporter) is the only requirement that pins the shared encoder. Tagged
   **FS-090.22**. Slight stretch: the encoder is also reused by the report views and backup
   writer described in FS-090.26/.27.
2. **Knowledgebase class** — non-functional dead code; no live requirement implements it.
   Used **KL-050.5** as the first-class id (the spec's explicit tracking id) rather than
   inventing a functional FS-050.x mapping.
3. **MailFetcher::getMailboxes/createMailbox/checkMailbox** — archive-folder helpers. No
   dedicated requirement; folded under **FS-041.4** (per-message disposition) since the
   archive move is the only consumer. Reasonable but coarse.
4. **osTicket::getDBSignature / getVersion / accessors** — untagged in Phase-3 and left
   untagged (trivial getters); no ambiguity introduced, noted for completeness.

## Byte-identity caveats (tool limitation) — 2

The Edit/Write tools normalize trailing whitespace, which prevented reproducing
trailing-whitespace-only lines that existed at HEAD adjacent to converted comments:

1. **class.lock.php** — had to be rewritten via Write (an early Edit spanned a
   whitespace-only blank line). Five originally trailing-whitespace blank/code lines
   (orig. lines 25, 54, 99, 107, 109) are now clean (`\n` instead of `    \n`). All code
   tokens are byte-identical; only trailing whitespace on those blank/continuation lines
   changed. No semantic change.
2. **class.knowledgebase.php** — the `function update() {` line (orig. line 99) had one
   trailing space at HEAD; the converting Edit stripped it. Code-token identical.

All other 17 files are byte-identical to HEAD except the converted comment lines.
Tab-indented comment lines (class.pdf.php Header/Footer/ctor, class.migrater.php,
class.osticket.php isUpgradePending) preserved their leading tabs.
