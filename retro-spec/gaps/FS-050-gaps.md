# FS-050 Knowledge Base & FAQ — Phase-2 Gap Report

Spec: `specs/FS-050-knowledge-base-faq.md`
Source slice: `kb/index.php`, `kb/kb.inc.php`, `kb/faq.php`, `scp/kb.php`, `scp/faq.php`,
`include/class.faq.php`, `include/class.category.php`, `include/class.knowledgebase.php`,
`include/ajax.kbase.php`, `include/client/{knowledgebase,faq,faq-category}.inc.php`,
`include/staff/{faq,faq-view,faq-categories,faq-category}.inc.php`,
`include/class.config.php` (`isKnowledgebaseEnabled`), `include/class.nav.php` (kbase sub-nav).
(`pages/` does not exist in this snapshot — treated as not present.)

## Summary

| Type | Count |
|------|-------|
| MISSING | 7 |
| INCORRECT | 1 |
| IMPRECISE | 2 |
| **Total** | **10** |

New ids added: 0 FR (existing FRs amended) · 0 BS · **4 EC** (EC-050.10–050.13) · **4 KL** (KL-050.8–050.11).
Plus targeted amendments to FS-050.3, FS-050.5, FS-050.9, FS-050.12, FS-050.17.

## Gaps

| ID | Type | Sev | What the code does | What the spec said | Fix applied |
|----|------|-----|--------------------|--------------------|-------------|
| G1 | INCORRECT | High | `Category::lookup($id)` returns an object for ANY numeric id (test is `is_numeric && new Category(id)`), even when no row exists — no `getId()==$id` check, unlike `FAQ::lookup`. Bad numeric `cid` yields a hollow category (getId()=0, isPublic() falsy). | FS-050.3 said "if lookup fails it records 'Unknown or invalid FAQ category'", implying lookup fails (returns null) on a bad id. | Amended FS-050.3 with the observed-behavior note; added EC-050.10 and KL-050.8 documenting the dead error path and the hollow-object fallthrough. |
| G2 | MISSING | Med | On `do=create` success, `$faq` is reassigned to the new article, so the post-POST view is `faq-view.inc.php` for the new article, not the form. | FS-050.12/13 did not state where a successful create lands. | Amended FS-050.12; added EC-050.11. |
| G3 | MISSING | Med | `FAQ::save` sets its own `err` strings on DB-level failure: "Unable to create FAQ. Internal error" (insert) and "Unable to update FAQ." (update). Entry script only sets its own message when `!$errors['err']`, so the class messages win on DB failure. | Spec listed only the entry-script messages ("Unable to add/update FAQ. Try again!"); the class-level messages were undocumented. | Amended FS-050.12 (both create and update branches). |
| G4 | MISSING | Med | `FAQ::save` rejects with "Internal error. Try again" when `$id && $id != $vars['id']` (hidden-id tamper guard). | FS-050.17 omitted this validation branch. | Added the branch to FS-050.17; added EC-050.12. |
| G5 | MISSING | Low | `uploadAttachments` accepts already-stored numeric file **ids** directly (`is_numeric($file)?$file:AttachmentFile::upload`), skips non-numeric failed uploads, inserts a join row only for numeric file ids, reloads only when ≥1 linked. | FS-050.9 described only fresh uploads. | Amended FS-050.9. |
| G6 | MISSING | Low | `FAQ::save` stores `notes` verbatim — no `safe_html`/striptags; only answer is sanitized, only question is tag-stripped. | FS-050.17 implied uniform sanitization ("answer is stored after safe-HTML sanitization"). | Amended FS-050.17; added KL-050.10. |
| G7 | IMPRECISE | Low | Public search SQL selects only `faq_id, question`; the result template references `$row['ispublished']` for a Published/Internal label that never renders (column not selected). Inert. | FS-050.5 did not mention the dead annotation arg. | Amended FS-050.5; added KL-050.9. |
| G8 | MISSING | Low | `getHelpTopics()`/`getHelpTopicsIds()` memoize and are not invalidated by `updateTopics()`; stale lists possible on the same instance pre-reload. | Not mentioned. | Added KL-050.11. |
| G9 | IMPRECISE | Low | `Category::delete` returns `$num` only inside the `if(affected_rows)` branch; a zero-affect delete returns undefined/null AND skips the cascade FAQ-row DELETE. | BS-050.5 / KL-050.4 described the cascade as occurring; did not note the no-op return/skip. | Added EC-050.13 (cross-refs FS-032 ownership). |
| G10 | MISSING | Low | `save(..., $validation=true)` returns pass/fail without persisting; live create/update never pass it. | Not documented. | Amended FS-050.17 (validation-only path note). |

## Notes
- KL-050.5 (dead `Knowledgebase` class bound to the canned table, malformed `create` SQL missing a closing paren) was already correctly captured in the spec and re-verified against source — no change.
- KL-050.2 (public "Last updated" reads the category timestamp) re-verified — already correct.
- `canManageFAQs()` alias and `can_manage_faq` raw-flag return re-verified — already correct in FS-050.8.
- Report-spec-only constraint honored: no source files modified.
