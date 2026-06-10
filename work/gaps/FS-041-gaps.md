# FS-041 Gap Report — Inbound Email Pipeline (Fetch, Pipe & Parse)

Phase-2 GAP-CLOSE. Source slice re-read with fresh eyes: `api/pipe.php`, `include/api.tickets.php`,
`include/class.mailparse.php` (Mail_Parse + EmailDataParser + ApiEmailDataParser), `include/class.mailfetch.php`,
`include/class.thread.php` (postEmail / lookupByEmailHeaders / logEmailHeaders / create),
`include/class.filter.php` (isAutoResponse / isAutoBounce), `include/class.ticket.php` (create / postMessage),
`include/class.api.php` (getRequest / ApiEmailDataParser), `setup/scripts/automail.php`.

Report spec only — no source file modified. Only `specs/FS-041-...md` + this file edited.

| id | type | severity | what the code does | what the spec said | fix applied |
|----|------|----------|--------------------|--------------------|-------------|
| G1 | INCORRECT | High | `PipeApiController::process` returns outcome **416** ("Request failed - retry again!") when no ticket; 416 maps to exit code **65 (data error)** per the exit table, NOT a deferral/retry code. | FS-041.1 said the no-ticket case maps to "the deferral exit code" so the MTA retries. | Corrected FS-041.1 to state outcome 416 → exit 65 (data error), noted the message/exit-code mismatch; added EC-041.14. |
| G2 | MISSING | High | HTTP/pipe channel `processEmail` calls `lookupByEmailHeaders($data)` **without** the `&$seen` out-parameter; only the polled `MailFetcher::createTicket` passes/honors `$seen`. A seen-but-rejected (thread-zero) email is dropped on polled fetch but re-attempts creation on HTTP/pipe. | FS-041.6 step 4 / BS-041.9 presented seen-suppression as universal across all channels. | Rewrote FS-041.6 steps 4–5 to scope seen-suppression to the polled channel; added BS-041.9.1, EC-041.15, and a note in FS-041.2. |
| G3 | MISSING | High | `ApiEmailDataParser::fixup` (HTTP/pipe only) forces source=Email; empty body → **subject** (not `-`) then `-`; empty subject → `[No Subject]`; empty emailId → **system default mail-account id**; strips priorityId when email-priority off. | Spec described empty-body → `-` unconditionally (BS-041.16) and emailId fallback as "fetching account" without the HTTP/pipe default-account distinction; no fixup FR. | Added FS-041.5.3 (post-parse fixup); corrected FS-041.5 emailId resolution + BS-041.16 cross-ref; updated data-requirements `emailId`/`source` rows. |
| G4 | INCORRECT | Med | Polled-path `emailId` resolution scans To/Cc/**Bcc** then falls back to fetching account; HTTP/pipe scans To(+Delivered-To)/Cc (**no Bcc**) then falls back to system default. The two fallbacks differ. | FS-041.5 lumped "To/Delivered-To/Cc/Bcc" and gave a single fetching-account fallback. | Split FS-041.5 emailId bullet by channel with correct recipient sets and fallbacks; updated EC-041.13. |
| G5 | IMPRECISE | Med | Polled bounce-drop is evaluated **only on the create-failure branch** (after `Ticket::create` returns no ticket and not a 403). A bounce that threads or successfully creates is NOT dropped. HTTP/pipe has no bounce-drop branch at all. | BS-041.13 implied any non-threading bounce is dropped pre-create across channels. | Rewrote BS-041.13 with the create-failure-branch timing and HTTP/pipe absence. |
| G6 | IMPRECISE | Med | Polled `createTicket` ban-checks the sender **before** thread lookup (drops even threaded replies from banned senders); HTTP/pipe only ban-checks inside `Ticket::create` (banned-sender reply still appends). | FS-041.7 banlist bullet implied a single uniform ban behavior. | Added channel ban-timing distinction to FS-041.7; added EC-041.16. |
| G7 | IMPRECISE | Low | Explicit-`ticketId` path requires both `Ticket::lookup` to resolve AND `postMessage` to succeed; otherwise it **falls through** to header lookup (no error). Post is always a Message (skips BS-041.8 classification). | FS-041.6 step 1 implied an unconditional post+return. | Clarified FS-041.6 step 1 (fall-through + always-Message semantics). |
| G8 | MISSING | Low | Multi-valued `Message-Id` → last non-empty (`array_pop(array_filter())`); polled re-reads raw header for message-id before synthetic fallback. Thread posts without a mid get `<24rand@<10hex of md5(baseUrl)>>`; email-info row written only when mid non-empty. | No mention of multi-valued mid handling or the thread-post mid backfill format. | Added BS-041.17 + BS-041.18. |
| G9 | MISSING | Low | `MailFetcher::open` disables GSSAPI/NTLM authenticators, sets a 20s imap_timeout, reuses connection on successful ping; close uses CL_EXPUNGE plus an explicit pre-close expunge. | Spec mentioned inbox-only and expunge but not login hardening, timeout, or connection reuse. | Added BS-041.19. |
| G10 | IMPRECISE | Low | Body part-gathering (generic parser) only recurses into sub-parts with **no** Content-Disposition; polled path skips parts carrying a filename parameter. | FS-041.5 body bullet did not state attachment-disposition parts are excluded from body concatenation. | Added a body-part-skip bullet to FS-041.5. |
| G11 | IMPRECISE | Low | `X-Priority` is extracted by a **raw substring scan** of the literal header block (strip non-digits to next newline), first occurrence; decorated values reduce to digits. | BS-041.10 gave only the numeric mapping, implying structured header parsing. | Added the substring-scan extraction detail to BS-041.10. |

## Summary

- **Gaps found / fixed: 11 / 11** — MISSING ×4 (G2,G3,G8,G9), INCORRECT ×2 (G1,G4), IMPRECISE ×5 (G5,G6,G7,G10,G11).
- Severity: High ×3, Med ×3, Low ×5.

## Most significant

1. **G1 (exit-code mismatch)** — the documented "retry/defer" outcome for an empty pipe result actually emits a *permanent data-error* exit code (65); an MTA acting on the spec would expect a deferral and could lose mail to an unexpected bounce.
2. **G2 (seen-flag channel asymmetry)** — idempotent suppression of already-rejected mail is guaranteed only for polled fetch; the HTTP/pipe channel re-runs creation, so the spec's universal "seen → silently accepted" claim was wrong for that channel.
3. **G3/G4 (fixup + emailId fallback)** — the HTTP/pipe channel's `fixup` substitutes empty body with the *subject* and empty target with the *system default* mail account (not `-` and not the fetching account); these channel-specific defaults were entirely undocumented and materially change routing/content of API-ingested mail.

## New rule counts for FS-041

- New FRs: **1** (FS-041.5.3).
- New BS rules: **4** (BS-041.9.1, BS-041.17, BS-041.18, BS-041.19).
- New ECs: **3** (EC-041.14, EC-041.15, EC-041.16).
- Plus in-place corrections to FS-041.1, FS-041.2, FS-041.5, FS-041.6, FS-041.7, BS-041.10, BS-041.13, BS-041.16 (cross-ref), EC-041.13, and Data Requirements.
