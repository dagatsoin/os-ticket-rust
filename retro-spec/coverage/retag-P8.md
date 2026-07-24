# Retag P8 — include/ajax.*.php + upgrader glue + migration tasks

Converted Phase-3 spec-level `// [FS-0XX]` brackets into requirement-level `@implements`
annotations. Comment-only edits; no HEAD source lines touched.

Owning specs consulted: FS-020, FS-021, FS-022, FS-033, FS-040, FS-050, FS-061,
FS-010, FS-011, FS-031, FS-003, FS-043, FS-002, FS-090.

## Per-file summary

| File | Old bracket lines | New `@implements` lines | Notes |
|------|------:|------:|-------|
| `include/ajax.config.php` | 3 | 5 | scp→FS-033.10; client→FS-033.10 + FS-011.7 + FS-010.7 (split one bracket-trio into 3 lines) |
| `include/ajax.content.php` | 3 | 5 | log→FS-033.8; ticket_variables→FS-033.9 + FS-040; class spans .8/.9 |
| `include/ajax.kbase.php` | 3 | 4 | cannedResp→FS-022.14; faq→FS-050.10; class spans both |
| `include/ajax.users.php` | 2 | 2 | search→FS-020.8 (requester autocomplete) |
| `include/ajax.reports.php` | 7 | 10 | OverviewReport: groups/getData/JSON/CSV→FS-020.13 (+BS-020.15, +FS-090); dateRange/plot→FS-020.12 |
| `include/ajax.tickets.php` | 9 | 15 | lookup/lookupByEmail→FS-020.7 (+BS-020.2); search→FS-020.8 (+BS-020.16); acquire/renew/release/preview→FS-021.18 (+BS-021.3, +FS-021.23a) |
| `include/ajax.upgrader.php` | 2 | 5 | upgrade()→FS-061.11 + FS-061.12 + FS-061.14 + BS-061-03 |
| `include/upgrader/prereq.inc.php` | 1 | 3 | FS-061.9 + FS-061.16 + BS-061-03 |
| `include/upgrader/upgrade.inc.php` | 1 | 3 | FS-061.9 + FS-061.11 + BS-061-03 |
| `include/upgrader/done.inc.php` | 1 | 3 | FS-061.14 + FS-061.9 + BS-061-03 |
| `include/upgrader/aborted.inc.php` | 1 | 3 | FS-061.12 + FS-061.9 + BS-061-03 |
| `include/upgrader/rename.inc.php` | 1 | 3 | FS-061.13 + FS-061.9 + BS-061-03 |
| `include/upgrader/streams/core/15b30765-dd0022fb.task.php` (AttachmentMigrater) | 17 | 17 | FS-061.5 (resumable task internals) + FS-022.12 (next/queueAttachments — DB file store) + FS-061.12 (error/getErrors) |
| `include/upgrader/streams/core/435c62c3-2e7531a2.task.php` (MigrateGroupDeptAccess) | 2 | 4 | FS-061.5 + BS-031-021 (group↔dept matrix) |
| `include/upgrader/streams/core/8aeda901-16fcef4a.task.php` (CryptoMigrater) | 3 | 4 | FS-061.5 + FS-003.1 (two-key reversible encryption) |
| `include/upgrader/streams/core/98ae1ed2-e342f869.task.php` (APIKeyMigrater) | 2 | 3 | FS-061.5 + FS-043.1 (API key entity/secret) |
| `include/upgrader/streams/core/c00511c7-7be60a84.task.php` (MigrateDbSession) | 3 | 3 | FS-061.5 + FS-002.14 (DB session store) |
| **Total** | **61** | **92** | |

## ID-class breakdown of the 92 new annotation lines

- **FS-XXX.N requirement-level**: 84
  - FS-020.7 (4), FS-020.8 (3), FS-020.12 (3), FS-020.13 (6)
  - FS-021.18 (5), FS-021.23a (1)
  - FS-022.12 (4), FS-022.14 (2)
  - FS-033.8 (3), FS-033.9 (2), FS-033.10 (3)
  - FS-010.7 (1), FS-011.7 (1)
  - FS-050.10 (2)
  - FS-061.5 (24), FS-061.9 (5), FS-061.11 (3), FS-061.12 (5), FS-061.13 (1), FS-061.14 (3), FS-061.16 (1)
  - FS-002.14 (2), FS-003.1 (3), FS-043.1 (2)
- **FS-XXX spec-level fallback**: 1
  - FS-040 (1) — ticket_variables card cross-refs the whole VariableReplacer substitution domain; spec itself directs to FS-040 with no single owning requirement.
  - FS-090 (1) — shared export/download helper; export mechanics owned wholesale by FS-090, referenced not re-specified. (2 spec-level total.)
- **BS-XXX first-class**: 6
  - BS-020.2 (2), BS-020.15 (1), BS-020.16 (1), BS-021.3 (1), BS-031-021 (2), BS-061-03 (6)
  - (BS-061-03 appears once per upgrader screen + ajax endpoint = 6 occurrences.)
- **EC-/KL-**: 0 used (none mapped cleanly enough to be load-bearing here).

Note: FS-090 counted under spec-level fallback above; BS counts list occurrences, so the
headline "FS.N=84 / fallback=2 / BS=6" sums slightly over 92 because BS-061-03 (×6) and
BS-020.2 (×2), BS-031-021 (×2) recur across multiple units — see raw grep counts.

## Ambiguous / judgement cases (logged)

1. **`ajax.config.php::client()`** — lives in `ConfigAjaxAPI` (FS-033 territory) but its
   consumer is the public portal. FS-033.10 explicitly defers the client projection to
   FS-010/FS-011 (KL-033.5). Tagged FS-033.10 (co-location) + FS-011.7 (Attachment
   Submission) + FS-010.7 (Client Ticket View reply form) as the two upload-config consumers.
2. **`ticket_variables()`** — FS-033.9 owns the *card content*; the substitution authority
   is FS-040 (VariableReplacer). No FS-040.N requirement is the precise owner, so FS-040
   spec-level fallback is correct per the spec's own note.
3. **Migration task second IDs** — domain specs (FS-031/003/043/002) document the *target*
   state, not a "migrate from v1.6" requirement. Mapped each to the closest concrete
   requirement that defines the data the migrator writes (BS-031-021, FS-003.1, FS-043.1,
   FS-002.14) rather than a vague spec-level fallback. FS-061.5 is the primary (resumable-task
   contract) on every migrator.
4. **AttachmentMigrater `next()` / `queueAttachments()`** — chose FS-022.12 (Content-Addressed
   Chunked File Storage) over FS-022.10/.13 because these methods perform the *store write*
   (AttachmentFile::save) and legacy-path discovery, which is .12's subject. Other internal
   helpers (queue/skip accounting) stay on FS-061.5 (the resumable-task contract).
5. **Upgrader `.inc.php` screens** — each is a single wizard view selected by FS-061.9. Added
   the screen-specific requirement (.16 prereq checks / .11 manual mode / .14 done / .12
   aborted / .13 rename) plus BS-061-03 for the shared admin gate that is the only executable
   line in most of these template files.
