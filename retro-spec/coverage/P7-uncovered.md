# P7 Coverage Report — `include/class.*` chunk D (ticket/thread + tail)

Partition: **P7** | Prepared: 2026-06-10 | Mode: Phase-3 COVERAGE tagging (additive `// [FS-0XX]` comments only)

Files owned (11):
`class.ticket.php`, `class.thread.php`, `class.team.php`, `class.template.php`,
`class.topic.php`, `class.upgrader.php`, `class.usersession.php`, `class.validator.php`,
`class.variable.php`, `class.xml.php`, `class.yaml.php`.

## Summary Counts

| Metric | Count |
|---|---|
| Files examined | 11 |
| Non-trivial units examined | ~118 |
| Units tagged | 84 |
| Units uncovered (no spec) | 0 |
| Trivial units skipped (accessors/getters/constructors/lookups) | ~34 |

All non-trivial behavioral units in P7 mapped cleanly to an existing FS spec. No uncovered
non-trivial units were found; the spec set (FS-021/041/011/043/042/020/010/030/040/061/002/003/022)
fully covers this partition. Trivial pure-accessor getters (`getId`, `getName`, `getStatus`, …),
boilerplate constructors, and one-line static `lookup()` wrappers were intentionally left untagged
(covered implicitly by their class-level tag and by FS-091's data model).

## Multi-spec hot-spots (tagged with combined ids)

| File:unit | Specs | Behavior |
|---|---|---|
| `class.ticket.php` :: `Ticket::create` | FS-011, FS-021, FS-041, FS-043, FS-042 | The canonical ticket-create spine for all four origins (web/staff/api/email): ban check, max-open cap, validation, filter `apply`, topic/email routing, ext-id gen, initial Message, SLA, auto-assign, autoresponse, alerts. |
| `class.ticket.php` :: `Ticket` (class) | FS-021, FS-011, FS-041, FS-043 | Domain class shared across creation channels + staff workflow. |
| `class.ticket.php` :: `postMessage` | FS-021, FS-041, FS-011 | Requester Message append used by create + inbound-email + web/API. |
| `class.ticket.php` :: `onNewTicket` / `onOpenLimit` | FS-021, FS-011, (FS-041) | New-ticket autoresponse + alerts / max-open-tickets notice. |
| `class.ticket.php` :: `checkOverdue` | FS-021, FS-043 | System overdue sweep invoked by cron. |
| `class.thread.php` :: `ThreadEntry::create` | FS-021, FS-041 | Thread-entry insert + attachments + email-info message-id backfill. |
| `class.thread.php` :: `postEmail` / `lookupByEmailHeaders` | FS-041 | Inbound-email append-by-sender-identity + thread matching. |
| `class.usersession.php` :: `UserSession` + token methods | FS-002, FS-010 | Shared session-token primitive across staff + client realms. |

## Notable spec-boundary findings

1. **`Ticket::create` is the single multi-channel chokepoint (FS-011/021/041/043/042).** Five specs
   describe slices of one ~250-line method (web form, staff phone, API, inbound email, and the
   filter/ban rejection path). Tagged once with all five ids per the multi-id guidance; reviewers
   should treat its origin-`switch` as the seam between those specs.

2. **`class.template.php` / `class.variable.php` / `class.yaml.php` are FS-040 territory living in
   P7's tail.** `EmailTemplateGroup`/`EmailTemplate`, the `%{...}` `VariableReplacer`, and the YAML
   initial-data loader (`YamlDataParser::load`, consumed only by `EmailTemplate::fromInitialData`)
   all belong to FS-040 even though they're physically grouped with ticket/thread code. `YamlParserError`
   is the lone FS-003 unit in that file (Error-class hierarchy).

3. **`Ticket::getStaffStats` (FS-020) and `getClientStats`/`checkClientAccess`/`getAuthToken`
   (FS-010) cross out of FS-021.** The `Ticket` class is not purely staff-workflow: its quick-stats
   aggregate feeds the FS-020 queue submenu, and its client-side accessors (auth token, per-email
   stats, client access gate) feed the FS-010 public portal. These were split off from the FS-021
   class-level tag so the queue/portal specs trace correctly.

## Process note (COMMENT-ONLY caveat)

Tags were inserted as standalone comment lines above each unit. Two incidental trailing-whitespace
normalizations occurred in `class.team.php` (a blank line inside `getTeams()` and the trailing space
after `function create($vars, &$errors) {`) because the editor strips trailing whitespace from
inserted context; no source token, statement, or logic line was modified, moved, reformatted, or
deleted. (Repo is not under git, so `git diff` verification was unavailable; changes were verified by
re-read.)
