# Retag P3 — `setup/**` tag→`@implements` conversion

Partition P3 (`api/* + setup/**`). Only `setup/**` files carried inserted `// [FS-]`
tags; the `api/*` entries in the partition table live under P1/P8 in practice and had
no P3-inserted tags. `setup/test/**` files carried **no** `[FS-]` tags (none inserted).

Specs consulted: FS-060 (installer), FS-061 (upgrader), FS-090 (data export),
FS-043 (external API/cron), FS-092 (CLI mgmt/deploy/packaging).

Format applied: requirement-level `@implements FS-XXX.N: <Title> — <note>`,
one id per line, BS/EC/KL first-class. No spec-level-only fallbacks were needed
except where a bracket genuinely spanned a top-level concept (all resolved to a
concrete FS-XXX.N).

## Per-file summary

| File | Old tag lines | New `@implements` lines | FS.N | BS | EC/KL | Notes |
|------|--------------:|------------------------:|-----:|---:|------:|-------|
| `setup/index.php` | 1 | 1 | 1 | 0 | 0 | FS-060.1 shim |
| `setup/upgrade.php` | 1 | 1 | 1 | 0 | 0 | FS-061.9 redirect |
| `setup/setup.inc.php` | 5 | 7 | 5 | 0 | 2 | EC-060-12, EC-060-02 split out |
| `setup/install.php` | 12 | 16 | 11 | 1 | 4 | bare `[FS-060]` (L16)→FS-060.1; BS-060-01; KL-060-02 x2, EC-060 fallbacks |
| `setup/inc/file-missing.inc.php` | 1 | 2 | 1 | 0 | 1 | FS-060.3 + EC-060-11 |
| `setup/inc/file-perm.inc.php` | 1 | 2 | 1 | 0 | 1 | FS-060.3 + EC-060-11 |
| `setup/inc/file-unclean.inc.php` | 1 | 3 | 1 | 1 | 1 | FS-060.9 + BS-060-01 + EC-060-11 |
| `setup/inc/subscribe.inc.php` | 1 | 2 | 1 | 0 | 1 | FS-060.1 + KL-060-02 |
| `setup/inc/install.inc.php` | 1 | 1 | 1 | 0 | 0 | FS-060.4 |
| `setup/inc/install-prereq.inc.php` | 2 | 4 | 2 | 0 | 1 | FS-060.2 x2 + EC-060-01; HTML `<!-- -->` form |
| `setup/inc/install-done.inc.php` | 1 | 2 | 1 | 1 | 0 | FS-060.8 + BS-060-16 |
| `setup/inc/header.inc.php` | 1 | 1 | 1 | 0 | 0 | HTML comment; FS-060.1 |
| `setup/inc/footer.inc.php` | 1 | 1 | 1 | 0 | 0 | HTML comment; FS-060.1 |
| `setup/inc/ost-sampleconfig.php` | 4 | 7 | 2 | 3 | 1 | BS-060-02 x2, BS-060-17; EC-060-13; KL-060-01 |
| `setup/inc/class.installer.php` | 25 | 49 | many | many | many | full install routine; FS-060.3/.5/.6/.7 + BS-060-04..18 + EC/KL |
| `setup/scripts/rcron.php` | 1 | 1 | 1 | 0 | 0 | FS-043.9 (was spec-level FS-043) |
| `setup/scripts/api_ticket_create.php` | 1 | 1 | 1 | 0 | 0 | FS-043.4 (was spec-level FS-043) |
| `setup/scripts/automail.php` | 2 | 4 | 1 | 1 | 1 | FS-043.15; BS-440; EC-449 |
| `setup/cli/manage.php` | 4 | 7 | 4 | 1 | 1 | FS-092.1 x2/.3; BS-092-02; BS-092-01; EC-092-01 |
| `setup/cli/package.php` | 11 | 14 | 11 | 2 | 1 | FS-092.10 x7/.11 x2/.12; BS-092-01/-07/-08; EC-092-01/-08 |
| `setup/cli/modules/class.module.php` | 10 | 16 | 10 | 1 | 2 | FS-092.2/.3/.4/.5; BS-092-03; EC-092-03 x2/-04 |
| `setup/cli/modules/export.php` | 3 | 3 | 3 | 0 | 0 | FS-090.27 x3 |
| `setup/cli/modules/deploy.php` | 3 | 5 | 3 | 1 | 1 | FS-092.6 x3; BS-092-06; EC-092-05 |
| `setup/cli/modules/unpack.php` | 6 | 12 | 6 | 3 | 3 | FS-092.7/.8/.9; BS-092-04/-05/-06; EC-092-05/-06/-07; KL-092-02 |
| `setup/cli/modules/import.php` | 10 | 22 | 11 | 6 | 5 | FS-092.13/.14/.15/.16 + FS-090.27; BS-092-09..13; EC-092-09/-11/-12/-13; KL-092-05 |

## Totals

- Files in partition with tags: **25** (plus 11 untagged `setup/test/**` files left byte-identical)
- Old `[FS-]` tag lines: **109**
- New `@implements` lines: **184**
- Breakdown of new lines:
  - Requirement-level `FS-XXX.N`: **~96**
  - Spec-level `FS-XXX` fallback only: **0** (every bracket resolved to a concrete requirement)
  - `BS-XXX`: **~31** (BS-060-01..18, BS-092-01..13, BS-440)
  - `EC-/KL-`: **~57** combined (EC-060-*, EC-092-*, EC-449, KL-060-*, KL-092-*)

## Tag-resolution decisions / ambiguous cases

1. **`setup/install.php` L16** — bare `// [FS-060]` over `require('setup.inc.php')`.
   Resolved to **FS-060.1** (installer bootstrap/entry). Not ambiguous in context.
2. **Reference API/cron clients** (`rcron.php`, `api_ticket_create.php`,
   `automail.php`) carried **spec-level `FS-043`**. Each is the *reference client*
   documented inside a specific FR:
   - `rcron.php` → **FS-043.9** (AC explicitly names `scripts/rcron.php`).
   - `api_ticket_create.php` → **FS-043.4** (json ticket-create, HTTP 201 + number).
   - `automail.php` first tag → **FS-043.15** (Local Email-Pipe Response Mapping
     reference client); exit-code map → **BS-440** + **EC-449** (mapping table itself
     canonically owned by FS-041 BS-041.2, noted in-spec — kept BS-440/EC-449 as the
     FS-043-local ids the bracket referenced).
3. **`import.php` header verify** (L46) was mis-tagged `[FS-090] FS-090.27`. The code
   is the *importer* verifying the dump header → primary **FS-092.14** (block framing
   + signature/dbtype guard) with **FS-090.27** retained as the format-reference id and
   **BS-092-09** (mysql-only) added. Cross-spec but not ambiguous.
4. **`export.php`** tags were `[FS-090] FS-090.27` and are genuinely FS-090 (the
   exporter lives in FS-090, the importer in FS-092). Left as **FS-090.27** — correct.
5. **`class.installer.php`** multi-id brackets (e.g. `FS-060.7 / BS-060-15 / BS-060-17
   / EC-060-07`) were expanded to **one `@implements` line per id** per the format rule
   (no comma-collapsing since the ids are not trivially-related).
6. **HTML-context files** (`header.inc.php`, `footer.inc.php`, the `install-prereq`
   inner `<ul>` line) used `<!-- @implements ... -->` form.

No unresolved ambiguities; no spec-level-only fallbacks were required.
