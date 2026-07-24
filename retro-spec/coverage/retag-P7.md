# Retag P7 — `include/class.*` chunk D (ticket/thread + tail)

Converted Phase-3 bracketed `// [FS-0XX]` coverage tags to requirement-level
`@implements` annotations across all 11 partition files. Every old bracket tag
was rewritten; no non-comment source line was modified except the whitespace
regression noted at the bottom.

Format applied: `@implements FS-XXX.N: <Requirement Title> — <note>` (em-dash),
one id per line, BS/EC ids first-class.

## Per-file summary

| File | Old `[FS-]` tags | New `@implements` lines | Notes |
|------|-----------------:|------------------------:|-------|
| class.xml.php | 3 | 4 | FS-043.4 (xml-format parser) + EC-435 (malformed body) |
| class.yaml.php | 3 | 3 | BS-040.13 (packaged fallback) + FS-003.23 (error subclass) |
| class.validator.php | 4 | 6 | FS-003.7 / FS-003.8 / FS-003.9 |
| class.variable.php | 5 | 7 | FS-040.11 + BS-040.14/.15/.17/.18 |
| class.usersession.php | 5 | 7 | FS-002.11 + FS-010.6 + BS-010.11 |
| class.team.php | 7 | 7 | FS-030.8/.10/.11 |
| class.topic.php | 4 | 5 | FS-030.13/.15/.16 + FS-011.4 |
| class.template.php | 11 | 16 | FS-040.6/.7/.8/.9/.10 + BS-040.7/.9/.10/.11/.12/.13 |
| class.upgrader.php | 13 | 15 | FS-061.2/.3/.4/.5/.6/.7/.8/.12/.14/.15 |
| class.thread.php | 26 | 36 | FS-021.22 + FS-041.6/.7/.8 + FS-022.10/.12/.13 + FS-043.5 + BS rules |
| class.ticket.php | 51 | 84 | FS-021.* spine + FS-011/FS-041/FS-043/FS-042 + FS-010/FS-020/FS-040 + BS |
| **Total** | **132** | **190** | line growth from multi-line splits of multi-bracket tags |

## Granularity breakdown (new annotation lines)

- **Requirement-level FS-XXX.N**: ~120 lines (the dominant form). Examples:
  FS-021.3/.4/.5/.7/.8/.9/.10/.11/.12/.13/.15/.17/.18/.19/.20/.22/.23,
  FS-011.9/.12/.13/.4, FS-041.6/.7/.8/.9/.5.2, FS-043.4/.5,
  FS-042.7/.8, FS-040.6-.11, FS-030.8/.10/.11/.13/.15/.16, FS-061.2-.15,
  FS-003.7/.8/.9/.23, FS-002.11, FS-010.3/.4/.6/.7/.8, FS-020.11.
- **BS-XXX first-class**: ~52 lines. BS-021.2/.3/.4/.7/.9/.14/.15/.16/.17/.18/.19/.21/.23,
  BS-040.7/.9/.10/.11/.12/.13/.14/.15/.17/.18, BS-041.7/.8/.9/.12/.17/.18,
  BS-010.1/.3/.4/.11, BS-022.8/.10/.15, BS-432.
- **EC- ids**: 1 (EC-435 — Malformed body, on XmlDataParser::parse).
- **Spec-level FS-XXX fallback**: 1 (see ambiguous cases).

## Multi-bracket tag resolution (one line per consuming spec)

- `Ticket::create` `[FS-011][FS-021][FS-041][FS-043][FS-042]` → 7 lines:
  FS-011.9, FS-021.20, FS-041.9, FS-043.4, FS-042.7, FS-042.8, BS-021.19
  (the canonical create chokepoint — one requirement id per consuming spec).
- `Ticket::onNewTicket` → FS-011.12 + FS-021.16 + FS-041.9.
- `Ticket::onMessage` → FS-021.5 + FS-041.8.
- `Ticket::postMessage` → FS-021.5 + FS-041.8.
- `Ticket::postNote` → FS-021.4 + FS-041.8.
- `Ticket::onOpenLimit` → BS-011.4 + FS-011.13.
- `Ticket::getOpenTicketsByEmail` → FS-011.13 + BS-021.19.
- `Ticket::postCannedReply` → FS-022.14 + BS-022.15.
- `Ticket::checkOverdue` → FS-021.13 + BS-021.17 + BS-432.
- `Ticket::assign` → FS-021.7 + FS-021.8.
- `Topic::getHelpTopics` `[FS-030][FS-011]` → FS-030.13 + FS-011.4.
- `ThreadEntry::importAttachments` / `importAttachment` `[FS-022][FS-041]` →
  FS-041.5.2 + FS-043.5 (emailed vs API attachment intake; the `[FS-022]`
  bracket was reinterpreted as the API/email intake requirements, since these
  methods serve the inbound email + API channels, not canned-response binding).
- `Thread::create` `[FS-021][FS-041]` → FS-021.22 + BS-041.18.
- `UserSession` / `sessionToken` / `isvalidSession` `[FS-002][FS-010]` →
  FS-002.11 (+ FS-010.6 on the class, + BS-010.11 on isvalidSession).

## Ambiguous / fallback cases (logged)

1. **`Ticket::setLastMsgId` mutator block** (was `[FS-021]` "ticket mutators
   priority/dept/staff/team/sla/status/state/answered"). These are low-level
   property setters consumed across many FS-021 workflow actions
   (.7/.10/.11/.15). No single requirement owns them, so tagged **spec-level
   fallback `FS-021`** with an explanatory note. This is the only spec-level
   fallback in the partition.

2. **`Thread::deleteAttachments`** — mapped to FS-022.12 + BS-022.10 (orphan
   reference-counted purge). The original `[FS-022]` had no sub-id; chose the
   content-addressed storage requirement that owns orphan reclamation.

3. **`Ticket::getClientStats`** — mapped to FS-010.8 ("My Tickets" list, which
   consumes the open/closed counts). Plausible alt: a portal-landing stat, but
   FS-010.8 is the documented consumer.

4. **`Ticket::getIdByExtId`** — mapped to FS-041.6 (subject-line ticket-number
   threading). It is also used by client lookup, but the comment explicitly
   frames it as subject threading, so FS-041.6 is the precise owner.

## Known deviation (tooling limitation) — class.team.php

The Edit/Write tools in this environment strip trailing whitespace from written
content. Consequences on `include/class.team.php`:

- Line 229 `function create($vars, &$errors) {` lost a single trailing space
  present at HEAD (could not be re-added — both Edit and Write strip it).
- A full-file Write performed to attempt the fix additionally stripped trailing
  whitespace from 8 HEAD-original whitespace-bearing lines
  (orig lines 34, 135, 201, 238, 246, 258, 264, 267 — all blank-indent or
  trailing-space-after-content lines).

Net: class.team.php differs from HEAD only by (a) the intended comment-line
conversions and (b) trailing-whitespace stripping on ~9 lines. No code tokens,
ordering, or line endings were changed; the diff is whitespace-only on those
lines. RECOMMENDATION: if byte-exact trailing whitespace matters, restore
`class.team.php` from HEAD and re-apply the 7 comment edits with a tool that
preserves trailing whitespace, OR run `git diff -w` to confirm the only
substantive change is the comment conversion. All other 10 partition files are
byte-identical to HEAD except their comment lines (verified: no unintended
trailing-whitespace loss on the other files, since those were edited only via
in-place comment-line Edits which preserve sibling lines).
