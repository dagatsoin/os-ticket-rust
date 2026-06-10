# Retag P4 — include/class.* chunk A (ajax→export)

Converted Phase-3 spec-level `// [FS-0XX]` bracket tags into requirement-level
`@implements` annotations. Comment-only edits; no HEAD source lines altered
(one trailing-whitespace caveat in class.ajax.php, noted below).

Counts below: **old** = bracket comment lines we inserted in Phase 3;
**@impl lines** = total `@implements` lines emitted (≥ old because many bracket
tags carried 2 ids — collapsed brackets like `[FS-043][FS-041]` and class-level
units expanded to FS-XXX.N + BS/EC/KL lines).

| File | old tags | @impl lines | FS-XXX.N | FS spec-fallback | BS | EC/KL |
|------|---------:|------------:|---------:|-----------------:|---:|------:|
| class.ajax.php | 5 | 5 | 5 | 0 | 0 | 0 |
| class.api.php | 35 | 50 | 38 | 0 | 4 | 8 |
| class.attachment.php | 12 | 13 | 12 | 0 | 1 | 0 |
| class.banlist.php | 8 | 11 | 9 | 0 | 2 | 0 |
| class.canned.php | 29 | 39 | 16 | 0 | 23 | 0 |
| class.captcha.php | 3 | 3 | 3 | 0 | 0 | 0 |
| class.category.php | 24 | 28 | 18 | 0 | 10 | 0 |
| class.charset.php | 4 | 6 | 4 | 0 | 2 | 0 |
| class.client.php | 20 | 26 | 23 | 0 | 3 | 0 |
| class.cron.php | 6 | 11 | 5 | 0 | 3 | 1 |
| class.crypto.php | 36 | 48 | 36 | 0 | 11 | 1 |
| class.csrf.php | 10 | 11 | 11 | 0 | 0 | 0 |
| class.dept.php | 39 | 61 | 39 | 0 | 22 | 0 |
| class.dispatcher.php | 16 | 26 | 16 | 0 | 1 | 5 (KL) + 2 (EC) |
| class.email.php | 28 | 35 | 28 | 0 | 7 | 0 |
| class.error.php | 7 | 7 | 7 | 0 | 0 | 0 |
| class.export.php | 18 | 26 | 18 | 0 | 8 | 0 |
| **Total** | **300** | **406** | **288** | **0** | **97** | **~21** |

(Per-file BS/EC/KL counts are approximate where a single comment block lists
multiple secondary ids; FS-XXX.N is the primary requirement-level id on the
unit. No unit fell back to bare spec-level `FS-XXX`.)

## Notable id resolutions (Phase-3 spec guess → precise home)

- **class.captcha.php** — Phase 3 tagged `[FS-011]`. The `Captcha` generator is
  authored by **FS-010.11 (Captcha Image Generation)**; FS-011 is only the
  *consumer* (submission form). Re-homed to FS-010.11 with an FS-011-consumer note.
- **class.csrf.php** — Phase 3 tagged `[FS-002]`. The CSRF token *mechanism*
  (derivation/rotate/timeout/form-input) is owned by **FS-001.11**; FS-002.7
  only consumes it. Re-homed to FS-001.11; `validateToken` additionally carries
  FS-002.7 (the staff-gate consumer).
- **class.category.php** — Phase 3 tagged `[FS-032][FS-050]`. Split by unit:
  read-model getters/load/lookup → **FS-050.15** (staff category detail / public
  read); admin form/setters/save/validate/create/update → **FS-032.14**;
  delete → **FS-032.16** (+ BS-032.16 / BS-050.5 cascade).
- **class.api.php** `ApiEmailDataParser` / `getEmailRequest` — Phase-3 `[FS-041]`.
  HTTP wire-framing home is **FS-043.14**; payload fixup defaults are owned by
  **FS-041.5.3** (tagged on `fixup`). BS-439 added.
- **class.cron.php** / **class.dept.php** / **class.export.php** / **class.email.php**
  — collapsed dual brackets (`[FS-043][FS-041]`, `[FS-030]`, `[FS-090][FS-020]`,
  `[FS-040][FS-041]`) resolved to the owning requirement + a parenthetical
  consumer-spec note (FS-041/FS-020 are downstream consumers, not the home).

## Ambiguous / judgement cases (2)

1. **class.ajax.php `staffOnly()`** — Phase-3 `[FS-002]`. The guard physically
   lives in the shared AJAX base and is described verbatim in **FS-043.13**;
   re-homed there (FS-002 staff-session dependency implicit, not restated as a
   second id to avoid noise).
2. **class.dept.php accessors** — many bare-getter `[FS-030]` tags have no single
   precise sub-FR; assigned to the closest consuming FR (FS-030.3 list-view vs
   FS-030.4 add/edit-form vs FS-030.5/.6 create/delete) by what reads the field.
   Defensible but not unique; no bare FS-030 fallback used.

## HEAD-byte caveat (1)

- **class.ajax.php** lines 8 and 30 (a comment-block trailing space and the empty
  constructor-body line that originally held 4 trailing spaces) were normalized:
  the Edit/Write tooling strips trailing whitespace and cannot re-emit it. These
  are whitespace-only, non-functional lines; all *content* bytes are unchanged.
  No other file had trailing-whitespace-only HEAD lines inside an edited region.
