# Recrawl P7 — include/class.* chunk D (ticket + thread + tail)

Verification round, fresh-eyes adversarial re-crawl. READ-ONLY on PHP/specs.
Partition files: class.ticket.php (1748), class.thread.php (743), class.team.php (207),
class.template.php (388), class.topic.php (191), class.upgrader.php (366),
class.usersession.php (144), class.validator.php (164), class.variable.php (99),
class.xml.php (83), class.yaml.php (34).

## Result: DRY (zero UNCOVERED / MIS-TAGGED / undocumented-branch findings)

## Summary counts

| File | Tagged units | Cross-spec cites verified | Findings |
|------|-------------:|---------------------------|---------:|
| class.ticket.php | ~40 method/branch tags | FS-021.1-23, BS-021.2-23, FS-010.3/4/7/8, BS-010.1/3/4, FS-011.9/12/13, BS-011.4, FS-040.11, FS-041.6/8/9, BS-041.17, FS-042.7/8, FS-043.4, FS-022.14, BS-022.15, FS-020.11, BS-432 | 0 |
| class.thread.php | ~22 tags | FS-021.22, BS-021.10/16, FS-040.11, FS-041.5.2/6/7/8, BS-041.7/8/9/12/18, FS-022.10/12/13, BS-022.8/10, FS-043.5 | 0 |
| class.team.php | 6 tags | FS-030.8/10/11 | 0 |
| class.template.php | ~14 tags | FS-040.6/7/8/9/10, BS-040.7/9/10/11/12/13 | 0 |
| class.topic.php | 5 tags | FS-030.13/15/16, FS-011.4 | 0 |
| class.upgrader.php | ~16 tags | FS-061.2/3/4/5/6/7/8/12/14/15 | 0 |
| class.usersession.php | 7 tags | FS-002.11, FS-010.6, BS-010.11 | 0 |
| class.validator.php | 5 tags | FS-003.7/8/9 | 0 |
| class.variable.php | 7 tags | FS-040.11, BS-040.14/15/17/18 | 0 |
| class.xml.php | 4 tags | FS-043.4, EC-435 | 0 |
| class.yaml.php | 3 tags | BS-040.13, FS-003.23 | 0 |

Every cited FS/BS/EC id was opened in its target spec and confirmed to exist with a
title matching the tag description. FS-003 headings use `### FS-003.N — ` (em-dash);
EC-435 is a bullet in FS-043 (not a `###` heading) — both verified present.

## Chokepoint multi-line tag blocks spot-checked (>30%, all big ones)

- `Ticket::create` (line 2009, 8-tag block FS-011.9/FS-021.20/FS-041.9/FS-043.4/FS-042.7/FS-042.8/BS-021.19) — VERIFIED. Walked all intra-method branches: ban pre-screen (2022), max-open pre-check excl. staff (2030, BS-021.19), per-origin field switch incl. the `staff`→`api` fall-through requiring `source` (2055-2061; covered functionally by FS-021.20 "a source (Phone/Email/Other)"), filter RejectedException 403 (2098), topic vs email-account routing map (2128/2146), random-vs-sequential ext id (2185, BS-021.23), post-create autorespond suppression chain — system-email loop / isAutoResponse / mailer-daemon+postmaster / canned-reply (2218-2238, BS-021.20/FS-021.16), onOpenLimit at-ceiling (2249, BS-011.4), created event (2256). All branches map to cited rules.
- `Ticket::open` (2265, FS-021.20/BS-021.15) — VERIFIED. source whitelist, issue-required, canAssign gate, response+close-on-response, note-vs-activity branch, alertuser notice email path all covered by FS-021.20.
- `onNewTicket` (762, FS-011.12/FS-021.16/FS-041.9) — VERIFIED. autoresp gate + admin/dept-member(unassigned)/manager alert fan-out, dedup+available+%{recipient} — covered FS-021.16/BS-021.12.
- `onMessage` (892, FS-021.5/FS-041.8) — VERIFIED. unanswer+lastmessage, auto-assign-to-last-respondent-else-unassign (897-904, FS-021.16 new-message auto-assignment; code "closing staff" comment is inert—only last-respondent implemented), autorespond-gated reopen (909, BS-021.5 auto-response-exempt), system-email autoresp suppression (913).
- `postNote` (1625, FS-021.4/FS-041.8) — VERIFIED. poster resolution, non-critical state change, alert recipients with poster-exclusion + closed-ticket access re-check (1701-1703, BS-021.12) all covered.
- `transfer` / `onAssign` / `onOverdue` / `postMessage` / `postReply` / `postCannedReply` recipient loops — all VERIFIED against FS-021.16 + BS-021.12.
- `ThreadEntry::postEmail` (570, FS-041.8/BS-041.8) — VERIFIED. owner→Message / staff→Note / system-email→ignore / other→attributed-Message table matches BS-041.8 + EC-021.17; mid-dedupe early-return (585) matches EC-021.19.
- `ThreadEntry::lookupByEmailHeaders` (733, FS-041.6/BS-041.7) — VERIFIED. mid → References-reverse-walk → subject #number (sender-scoped) precedence matches BS-041.7.

## Known quirks confirmed already documented (not findings)

- `setState` misspelled `unassined` (706) → FS-021.9 + EC-021.18 + KL-021.9.
- `clearOverdue` does NOT annul the overdue event (1186 comment) → KL-021.4.
- `reopen` forces isanswered=0 (746) → KL-021.8.
- `staff_id` overloaded as assignee+closed-by (close 730 / assignToTeam 1311) → KL-021.3 + BS-021.4.
- `create()` dead "strip previous body" comment in Thread::create (804-809) — comment only, no behavior; not a coverage gap.

## Notes

- No `@implements` tag was found citing a non-existent id, and no tag's cited title
  contradicted its target spec section.
- No undocumented side-effect branch was found inside any tagged unit on the long
  Ticket/Thread methods. The earlier authoring round's multi-line tag blocks on the
  chokepoint methods (create/open/onNewTicket/onMessage/postNote/postEmail) already
  enumerate every alert/guard/edge branch via the FS-021.16 + BS-021.12 + BS-021.20
  umbrella rules plus the per-action FS-021.N rules.
