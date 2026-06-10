# Coverage Re-crawl — Partition P6 (include/class.* chunk C: json → sla)

Verification round. Fresh-eyes, adversarial. Existing `@implements` tags NOT trusted; cited ids
independently verified against `specs/` (spot-checked well above 30% — see below).

## Scope walked (20 files)

class.json.php, class.knowledgebase.php, class.lock.php, class.log.php, class.mailer.php,
class.mailfetch.php, class.mailparse.php, class.migrater.php, class.nav.php, class.osticket.php,
class.ostsession.php, class.page.php, class.pagenate.php, class.pdf.php, class.pop3.php,
class.priority.php, class.setup.php, class.signal.php, class.sla.php, class.staff.php

## Summary counts

| Classification | Count (behavioral blocks) |
|----------------|---------------------------|
| COVERED (tag present, cited id verified correct) | ~150 |
| UNCOVERED (real behavior, no tag) | 0 |
| MIS-TAGGED (tag present, wrong id) | 0 |
| TRIVIA (pure getters/setters/accessors, no tag needed) | ~120 |
| SOFT / borderline (noted, not a hard finding) | 1 |

**Result: effectively DRY.** Zero hard UNCOVERED or MIS-TAGGED findings.

## Spot-check of cited ids (all resolved correct in the named spec)

- KL-050.5 → FS-050 (knowledgebase dead/legacy class) ✓
- FS-090.22 / .15 / .16 / .17 / .18 / .29 / .32 / .1 / .2 / .3 / .4 / .5 / .6 → FS-090 ✓
- FS-043.4 (JSON request body parser) → FS-043 ✓ (cross-ref; see note below)
- FS-021.18 / BS-021.3 (TicketLock); FS-021.17 (Ticket2PDF) → FS-021 ✓
- FS-033.8 / .3 / .7 / .16 / .13 / .12 / .14; BS-033.2 / .6 / .8 / .9 → FS-033 ✓
- FS-040.12 / .13 / .11; BS-040.21 / .22 / .28 → FS-040 ✓
- FS-061.3 / .5 / .15 / .2 / .8 / .1; BS-061-04 / -06 / -09 / -15 → FS-061 ✓
- FS-001.5 / .7 / .11 / .12 / .16 / .3 → FS-001 ✓
- FS-002.14 / .5 / .9 / .12 / .15 / .16 / .17 / .1 / .2 / .3 / .4 / .6 / .10 / .7 → FS-002 ✓
- FS-091.5 / BS-091.3; FS-032.8 / .9 / .10 / .11 / .12 / .4; BS-032.9 / .12 → FS-091 / FS-032 ✓
- FS-003.21 (Signal pub/sub) → FS-003 ✓
- FS-041.3 / .4 / .5 / .5.1 / .5.2 / .6 / .7; BS-041.4 / .5 / .6 / .7 / .10 / .11 / .13 / .14 / .15 / .17 / .19 → FS-041 ✓
- FS-031.3 / .4 / .5 / .10 / .12 / .13; BS-031-001 / -005 / -006 / -007 / -014 / -015 / -016 / -032 → FS-031 ✓
- FS-022.13 / BS-022.13 (file-type allow-list) → FS-022 ✓ (cross-ref; see note)
- FS-060.1 / .2 / .7 (SetupWizard) → FS-060 ✓
- FS-050.1 (KB enable toggle, nav KB-link gate) → FS-050 ✓

## Findings

### UNCOVERED / MIS-TAGGED
None.

### Soft note (not a hard finding — no action required, recorded for the lead's judgment)

| File:line | Unit | Behavior | Observation | Spec | New id? |
|-----------|------|----------|-------------|------|---------|
| include/class.osticket.php:142 | `osTicket::isFileTypeAllowed($file, $mimeType='')` | Core-object extension allow-list check (`.*` wildcard; extension last 3–4 chars; in-array against configured allowed types). Consumed by the inbound-email path (class.mailfetch.php:577 `$ost->isFileTypeAllowed($file)`) and other upload paths. | Tagged `FS-022.13 / BS-022.13`. Those ids live in FS-022 and describe the **shared Format/Validation upload helper** (`Format::checkUploads`-style), a *different* implementation. The osTicket-core method enforces the **same rule** (BS-022.13: extension-only matching), so the tag is a defensible rule cross-reference rather than a wrong id. Borderline: a purist might want a distinct FS-001-scoped id for the core method since it is the email pipeline's gate, but the rule citation is accurate and not misleading. Left as-is. | FS-022 (or optionally a thin FS-001.x mirror) | No |

## Adversarial negatives explicitly checked and cleared

- **class.mailparse.php `EmailDataParser::parse` (385–459)**: confirmed it does NOT contain the
  FS-041.5.3 "post-parse fixup" (force source=Email / empty subject / empty body / system-default
  mail-account). That fixup lives outside P6 (api.tickets.php path). mailparse tags
  (FS-041.5 / .5.1 / .5.2) are therefore correctly scoped — no missing-tag here.
- **class.osticket.php getLinkToken / validateLinkToken (131–138)**: tagged FS-002.10 (Logout);
  FS-002.10 explicitly covers the session-derived single-use link token. Correct, not a mis-tag
  despite the helpers being generically reusable.
- **class.osticket.php error/warning/notice/header/title accessors (172–231)**: pure state
  accessors — TRIVIA, correctly untagged.
- **class.staff.php capability getters (`can*`, `is*`, dept/group/team getters, 246–399)**: the
  derived-capability ones (`canManageTickets`, `isAvailable`, `canAccessDept`, `getDepartments`,
  `getTeams`, `getManagedDepartments`) carry FS-002.15/.16/.17 + BS-031-005/-006/-007 tags;
  the remaining flat `$this->ht[...]` passthroughs are TRIVIA, correctly untagged.
- **class.knowledgebase.php getters/setters (38–65)**: dead-code passthroughs; the class-level +
  DB-access blocks carry KL-050.5. Correct.
- **class.pop3.php**: stub file ("No longer used"), zero behavior — correctly untagged.
- **class.lock.php cleanup (166) / class.osticket.php purgeLogs (338)**: both correctly carry the
  FS-043.7 cron-inventory cross-tag in addition to their primary spec.

## Conclusion

P6 is fully covered. Every behavioral unit carries a tag, every spot-checked cited id resolves to
the correct rule in the correct spec, and the only item worth surfacing is one defensible
rule-cross-reference (osTicket::isFileTypeAllowed → FS-022.13/BS-022.13) that is accurate as written.
No tags to add, none to correct.
