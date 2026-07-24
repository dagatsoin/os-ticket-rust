# FS-040 Gap Report — Email Accounts, Templates & Outbound Mail

Phase-2 GAP-CLOSE pass against source slice: `scp/emails.php`, `scp/templates.php`,
`include/class.email.php`, `include/class.template.php`, `include/class.mailer.php`,
`include/class.variable.php`, `include/class.config.php` (referenced).
View partials (`include/staff/email*.inc.php`, `template*.inc.php`, `tpl.inc.php`,
`header/footer.inc.php`) and `scp/emailtest.php` are **absent** from this snapshot.

Legend — type: MISSING | INCORRECT | IMPRECISE.

| # | Id(s) | Type | Sev | What the code does | What the spec said | Fix applied |
|---|-------|------|-----|--------------------|--------------------|-------------|
| 1 | FS-040.10 / BS-040.13 | INCORRECT | High | `EmailTemplate::fromInitialData` loads `I18N_DIR."$lang/templates/$name.yaml"` via `YamlDataParser::load` and requires `subject` + `body` keys; missing either raises `InitialDataError`. | Said packaged default is a file "whose first part is the subject and remainder the body". | Rewrote FS-040.10 + BS-040.13 to describe the structured `.yaml` `subject`/`body` contract and the `InitialDataError` path; added Note disclaiming the first-line/remainder reading. |
| 2 | KL-040.5 | INCORRECT/MISSING | High | The entire `include/staff/` view tree is absent — not just `tpl.inc.php`. All FS-040 UI presentation details are inferred, not verifiable. | KL-040.5 scoped the gap to `tpl.inc.php` only, implying the list/form/manage views were observed. | Broadened KL-040.5 to cover all missing staff partials and explicitly flag every UI acceptance criterion as inferred design intent pending view source. |
| 3 | BS-040.4 / FS-040.4 | INCORRECT/IMPRECISE | High | `Email::delete()` returns 0 (no-op) when id == default email **OR** id == alert email. Bulk loop only skips the default; an alert-email account selected for delete is silently left in place. | Mentioned only the default email as undeletable; the alert-email guard was buried as a confusing parenthetical ("excluded from the alert-email default check"). | Renamed BS-040.4 to cover both, documented the two-layer guard and the silent no-delete of a selected alert-email account; added a bullet to FS-040.4. |
| 4 | BS-040.27 / FS-040.12 (EC-040.13) | MISSING | Med | `Email::isSMTPEnabled()` = `smtp_active && (!auth || password)`. SMTP-active + auth-required + no stored password ⇒ NOT enabled ⇒ skipped in mailer precedence. | BS-040.21 precedence omitted the credential gate. | Added BS-040.27 + a definition note in BS-040.21 + EC-040.13. |
| 5 | BS-040.26 / FS-040.3 | MISSING | Med | Every successful save resets `mail_errors=0`, `mail_lastfetch=NULL`; coerces blank numeric fields to 0; defaults blank protocol to `POP`; persists checkbox flags by presence; maps post-fetch choice to `mail_delete`/archive columns. | None of these persistence side-effects / coercions were documented. | Added BS-040.26 and three side-effect bullets to FS-040.3. |
| 6 | BS-040.25 / FS-040.3 | MISSING | Low | `save()` raises "Internal error. Get technical help." when editing and `$id != $vars['id']` (hidden-id tamper/stale guard). | Not documented. | Added validation step 0 to FS-040.3 + BS-040.25. |
| 7 | BS-040.28 / FS-040.12 | MISSING | Med | Mailer line-ending = `MAIL_EOL` constant if defined-string; else forced `\n` when Suhosin patch present AND non-SMTP; else MIME default. | Line-ending / Suhosin / MAIL_EOL config interaction undocumented. | Added FS-040.12 bullet + BS-040.28. |
| 8 | BS-040.29 / FS-040.12 | IMPRECISE | Low | SMTP connections created with `'persist'=>true` and cached per host:port:username; failure evicts the key. | FS-040.12 mentioned caching+reuse but not the persistent-session flag nor explicit eviction-on-failure as a rule. | Added BS-040.29 making the reuse/eviction contract explicit. |
| 9 | FS-040.12 / EC-040.14 | MISSING | Low | Unresolvable stored-file id, or missing/unreadable file path, is silently skipped; message still sends. | Attachment-source-failure behavior not stated. | Added "silently skipped" clause to FS-040.12 + EC-040.14. |
| 10 | KL-040.8 | MISSING | Low | `staff.pwreset` catalog entry has `'default'=>'templates/staff.pwreset.txt'`, but resolver always loads `<lang>/templates/<code>.yaml`; the `.txt` pointer is inert. | Data Requirements catalog listed the `.txt` default as if authoritative. | Annotated catalog row + added KL-040.8. |
| 11 | KL-040.9 | MISSING | Low | `EmailTemplate::save` create branch writes the code-name-required error into mistyped `$errprs` instead of `$errors`, so the guard never fires. | Not documented. | Added KL-040.9. |
| 12 | KL-040.10 | MISSING | Low | `EmailTemplateGroup::getTemplates` caches on mistyped `$this->_tempates`, so the cache never hits and the message list re-queries each call. | Not documented. | Added KL-040.10. |
| 13 | FS-040.11 / BS-040.14 | IMPRECISE | Low | Token regex `[A-Za-z_][\w._]+` requires ≥1 trailing char ⇒ single-char token names are not recognized. | "begins with letter/underscore and may contain word chars/dots/underscores" implied 1+ total. | Tightened FS-040.11 wording + BS-040.14 note about single-char tokens. |
| 14 | BS-040.15 | IMPRECISE | Low | `getVar` traversal also supports a generic `getVar(<segment>)` accessor fallback when no `get<Capitalized>` accessor exists; `false` ⇒ empty. | Only the `get<Capitalized>` convention was described. | Expanded BS-040.15 to document the generic `getVar` fallback and its return semantics. |
| 15 | BS-040.17 | IMPRECISE | Low | `_parse` resolves each distinct token once (dedup), drops `false` (unresolved) from the replace map; resolution order = memoized var → object → root-scalar override → unresolved. | "Unknown left unchanged" was correct but resolution order + dedup were unstated. | Added resolution-order & dedup notes to BS-040.17. |
| 16 | BS-040.24 | IMPRECISE | Low | Update path decrypts current password into `cpasswd`; effective password = submitted-if-nonempty else decrypted current; re-encrypt only on non-empty submit. | Stated the outcome but not the decrypt-then-merge mechanism that also feeds live connection checks. | Added mechanism note to BS-040.24. |

## Summary

- **Gaps found / fixed:** 16 / 16.
- **By type:** INCORRECT 3 · MISSING 7 · IMPRECISE 6. (#2 spans MISSING+INCORRECT, counted under INCORRECT.)
- **By severity:** High 3 · Med 4 · Low 9.

### Most significant
1. **Packaged-default format was wrong (#1)** — spec described a first-line/remainder text file; code uses a structured `.yaml` with mandatory `subject`/`body` keys and an `InitialDataError` failure mode. Materially changes how the fallback content is authored.
2. **View tree entirely absent (#2)** — all UI acceptance criteria were presented as observed but no `include/staff/` partial exists in this snapshot; reclassified as inferred to avoid asserting unverifiable behavior.
3. **Alert email is also undeletable (#3)** — `Email::delete()` guards both default and alert email; a selected alert-email account is silently NOT deleted while only the default's checkbox is disabled — a real protection the spec misattributed.

### New rule counts for FS-040
- New BS rules: **5** (BS-040.25 … BS-040.29).
- New EC: **3** (EC-040.12, EC-040.13, EC-040.14).
- New KL: **3** (KL-040.8, KL-040.9, KL-040.10).
- New FR sub-steps: validation step 0 added to FS-040.3 (no new FS-040.N top-level FR).
- Rewrites/corrections (no new id): FS-040.4, FS-040.10, FS-040.11, FS-040.12, BS-040.4, BS-040.13, BS-040.14, BS-040.15, BS-040.17, BS-040.21, BS-040.24, KL-040.5, Template Type Catalog row.
