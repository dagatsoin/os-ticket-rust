# FS-022 Gap Report — Canned Responses & Ticket Attachments

Phase-2 GAP-CLOSE. Source slice: `scp/canned.php`, `include/class.canned.php`,
`include/staff/cannedresponse(s).inc.php`, `include/class.attachment.php`, `include/class.file.php`,
`attachment.php`, `scp/attachment.php`, `scp/file.php`, `scp/image.php`, `kb/file.php`, plus the
consumption paths `include/ajax.kbase.php::cannedResp` and `include/class.ticket.php::postCannedReply`,
and `include/class.osticket.php::isFileTypeAllowed`.

| ID | Type | Severity | What the code does | What the spec said | Fix applied |
|----|------|----------|--------------------|--------------------|-------------|
| G1 | INCORRECT | Medium | No root-level `image.php` exists in this snapshot; inline staff images are served by `scp/image.php`. `kb/file.php`, `scp/file.php`, `scp/image.php`, `attachment.php`, `scp/attachment.php` are the entry scripts. | Overview & FS-022.10 listed `image.php` (root) as a display entry script. | Overview now enumerates the actual scripts and states no root `image.php` exists. |
| G2 | IMPRECISE | Low | `scp/image.php` validates by comparing the **entire** 64-char hash against `getDownloadHash()`; the other scripts recompute only the last-32 half. | FS-022.10 said `scp/image.php` compares "the full combined hash against the canonical download hash" but conflated it with the two-half check used elsewhere. | FS-022.10 now spells out the differing validation predicate for `scp/image.php`. |
| G3 | MISSING | Medium | `kb/file.php` performs only bootstrap + session-bound hash check — no staff/client session gate; the hash is the sole credential. | Not documented. | Added note in FS-022.10 + new EC-022.13. |
| G4 | INCORRECT | High | `Osticket::isFileTypeAllowed` returns **false** (deny) when the allowed-types config is empty/unset; only `.*` allows all. | FS-022.13 implied an empty allow-list behaved permissively / was unspecified. | Added explicit default-deny clause in FS-022.13, new BS-022.14, EC-022.16. |
| G5 | INCORRECT | Medium | Ext regex `.{3,4}` only matches 3–4-char extensions; 1–2 or 5+ char extensions leave the whole filename as the token, which fails the allow-list ⇒ **rejected**. | KL-022.3 said such extensions "are not normalized" (outcome unstated). | Corrected KL-022.3, added detail to FS-022.13, new EC-022.17. |
| G6 | INCORRECT | Medium | `cannedResp` JSON puts the canned **title** under a field named `ticket` (misnomer); object is `{id, ticket, response, files}`; each file carries `key=md5(file_id+session_id+hash)`. Route is `canned-response/<id>.<json|txt>`; `tid` query param supplies ticket context; 404 "No such premade reply". | FS-022.14 said JSON carries "id, title, body, attachments" (wrong field names) and omitted the route/format/tid mechanics. | Rewrote FS-022.14 JSON acceptance criteria with exact field names, route, format default, and tid behavior. |
| G7 | MISSING | Medium | Bulk enable/disable use a single `UPDATE ... isenabled=N`; already-in-state rows count as not-affected ⇒ partial-count message even when end state is correct. Verb comes from JS-populated hidden `a` field; "Unknown command"/"Unknown action" error paths. | FS-022.6 omitted the affected-rows nuance and the verb-dispatch/error paths. | Added clauses to FS-022.6 + new EC-022.14. |
| G8 | MISSING | Low | `postCannedReply` records poster "SYSTEM (Canned Reply)", substitutes vars, carries attachments by file id, marks ticket unanswered, optionally sends alert/auto-reply via dept/default template+email. | FS-022.14 mentioned only attach + substitute. | Added FS-022.14 clause + new BS-022.15. |
| G9 | MISSING | Low | `uploadLogo` stores `ft='L'` logos via the same store; requires GIF/JPEG/PNG + min aspect ratio (default 3) when GD present ("Invalid image file type" / "Image is too square..."); falls back to plain store when GD absent. `L` files exempt from orphan sweep. | Logo files referenced only obliquely via BS-022.11. | Added FS-022.15. |
| G10 | MISSING | Low | `deleteAttachment` returns `(deleteOrphans() > 0)`, but `deleteOrphans()` always returns `true`, so the helper reports success whenever a binding row was deleted, regardless of whether any file was actually purged. | Not documented. | Added clause to FS-022.5 + new EC-022.15. |
| G11 | MISSING | Low | `sendData()` disables `zlib.output_compression` before streaming so `Content-Length` matches sent bytes; cache headers are emitted before the 304 short-circuit (so a 304 still carries them). | Not documented. | Added clauses to FS-022.11. |
| G12 | IMPRECISE | Low | In-code comment says chunks are "256kB" but the `CHUNK_SIZE` constant is `500*1024`. Chunk writes use `REPLACE INTO` (upsert) on `(file_id, chunk_id)`. File/attachment lookup accept numeric id OR content hash. | FS-022.12 gave 500×1024 (correct) but omitted the misleading comment, the upsert semantics, and the by-hash lookup. | Added notes to FS-022.12. |
| G13 | IMPRECISE | Low | `getCannedResponses(0)` (default, no dept) applies NO dept filter — returns every enabled response. Dept scoping only kicks in for a specific dept id. Ticket-open form uses the unscoped variant; ticket-view reply uses the dept-scoped `responsesByDeptId`. | BS-022.1 examples only described the dept-specific case. | Clarified BS-022.1 examples. |

## Summary

- **Gaps found / fixed: 13** (all fixed in spec).
- By type: **MISSING 6**, **INCORRECT 4**, **IMPRECISE 3**.
- By severity: High 1, Medium 6, Low 6.

### Spec additions
- New FRs: **1** (FS-022.15). FR count 14 → 15.
- New BS: **2** (BS-022.14, BS-022.15). BS count 13 → 15.
- New ECs: **5** (EC-022.13 … EC-022.17). EC count 12 → 17.
- New KLs: **0** (KL-022.3 corrected in place).
- Numerous in-place corrections/clarifications to FS-022.5/.6/.10/.11/.12/.13/.14, BS-022.1, Overview.

### Most significant
1. **G4 (High):** default-deny file-type gate — an empty allow-list rejects all uploads, not "allow
   all"; the spec previously left this ambiguous, a security-relevant behavior.
2. **G6 (Medium):** `cannedResp` JSON contract field names were wrong (`ticket` field actually holds
   the title; `files[].key` access key), plus the route/format/tid mechanics were undocumented.
3. **G3 (Medium):** `kb/file.php` has no staff/client session gate — the session-bound hash is the
   sole credential, an authorization nuance the spec omitted.
