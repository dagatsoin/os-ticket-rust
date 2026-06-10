# Recrawl P5 — independent verification round (adversarial, fresh-eyes)

Partition: **P5** — include/class.* chunk B (config/filter/faq..http)
Files (7): `class.config.php`, `class.faq.php`, `class.file.php`, `class.filter.php`,
`class.format.php`, `class.group.php`, `class.http.php`
Date: 2026-06-10. READ-ONLY pass; no PHP/spec edits.

## Method

Walked every class/method/top-level function in all 7 files. Did NOT trust the
existing `@implements` tags or the prior P5 reports. Independently resolved each
cited id against the live spec files and spot-checked far more than 30% of the
distinct ids (FS-042 family, FS-050.{1,2,3,4,7,9,12,17,18}, FS-022.{8-14},
FS-031-{004,019-024,034}, FS-003.{10-16,19,22}, FS-032.{2-7}, FS-091/BS-091.7/.11,
FS-001 online/tz, FS-061 schema-signature). **Every spot-checked id verified
correct** — id exists in the named spec and its prose matches the code behavior.

## Summary counts

| Metric | Count |
|--------|------:|
| Files examined | 7 |
| Units walked (classes + methods + top-level fns) | ~165 |
| Units COVERED (tag present + id verified) | ~119 |
| TRIVIA (1-line accessors / setters subsumed by class tag) | ~45 |
| UNCOVERED | 0 |
| MIS-TAGGED | 1 (borderline / precision-only) |

Spot-check verification: 0 wrong-id / hallucinated-id findings. The heavy,
load-bearing units (`Filter::matches`, `TicketFilter::{quickList,isBanned,apply,
isAutoResponse,isAutoBounce}`, `AttachmentFile::{format,deleteOrphans,uploadLogo,
makeCacheable}`, `OsticketConfig::update*Settings`, `Format::*`) are all tagged
to the correct owning requirement, including the subtle BS/EC/KL ids
(BS-042-26 email-id scope, KL-042.8 equal-as-contains, EC-042-18 quickList
over-inclusion, BS-022.11 ft='T' orphan scope, BS-022.12 304, EC-050.12 id-tamper).

## Findings (UNCOVERED / MIS-TAGGED)

| File:line | Unit | Behavior | Why flagged | Suggested spec | New id? |
|-----------|------|----------|-------------|----------------|---------|
| class.config.php:528 | `OsticketConfig::isCaptchaEnabled()` | Returns true only when `extension_loaded('gd') && function_exists('gd_info') && get('enable_captcha')` — i.e. the runtime GD-capability gate that silently disables CAPTCHA when the image library is absent. | Carries **independent logic** (the GD/`gd_info` capability AND-gate) beyond a bare config read, yet is subsumed under the class-level `FS-032.7` (settings-persistence) tag. The behavior it implements is the runtime "CAPTCHA enabled" predicate owned by **FS-011.5** + **KL-011.1** ("CAPTCHA treated as enabled only when admin setting on AND server image capability present; silently disabled otherwise"), not by the persistence requirement. Precision mis-tag only — not truly uncovered (FS-011.5/KL-011.1 already describe the behavior). | FS-011.5 / KL-011.1 (add a thin `@implements FS-011.5` + `@implements KL-011.1` line above the method; keep it out of the generic config-accessor trivia bucket) | No — existing FS-011.5 + KL-011.1 cover it |

## Non-findings explicitly considered and cleared (adversarial candidates)

- `OsticketConfig::allowOnlineAttachments()` / `allowAttachmentsOnlogin()` /
  `allowEmailAttachments()` (config:726-742) — compose multiple flags with AND
  logic, not bare reads. Cleared: FS-032.4 documents `allow_attachments` as the
  "global master switch" gating the sub-flags (spec lines 220-224), so the
  composition is the documented master-switch behavior. TRIVIA, subsumed.
- `Filter::isSystemBanlist()` (filter:91) — strcasecmp on "SYSTEM BAN LIST".
  Correctly tagged BS-042-13; verified. COVERED.
- `AttachmentFile::format()` restrict branch (file:309) — tagged FS-022.13 +
  BS-022.13 + BS-022.14; verified default-deny / ext-only. COVERED.
- `TicketFilter::origin2target()` mapping phone/staff→Web (filter:1002) — tagged
  FS-042.6; verified. COVERED.
- `CHUNK_SIZE = 500*1024` vs the "256kB chunks" doc comment (file:390-394) and
  `FAQ::attach()` referencing `$this->_attachments` where `getAttachments()`
  populates `$this->attachments` (faq:122 vs 215) — both are latent
  source quirks/bugs, NOT coverage gaps. Out of scope for this pass.

## Verdict

P5 coverage is sound. One precision-only mis-tag (`isCaptchaEnabled` → should
point at FS-011.5/KL-011.1 rather than ride the FS-032.7 class tag). No genuinely
uncovered behavior. No hallucinated or wrong-target ids in the spot-check sample.

## Round 2 closure (2026-06-10)

- **CLOSED** — `class.config.php`:528 `isCaptchaEnabled()` precision mis-tag. Added thin
  `@implements FS-011.5` + `@implements KL-011.1` lines above the method documenting the
  runtime GD-capability AND-gate; the class-level FS-032.7 tag stays. Finding closed.
