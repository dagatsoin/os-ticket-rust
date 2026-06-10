# FS-041: Inbound Email Pipeline — Fetch, Pipe & Parse

## Overview

The Inbound Email Pipeline is the subsystem that converts incoming email into tickets (new tickets) or thread entries (replies appended to existing tickets). It is the email-side counterpart to the public web submission path (FS-011) and the staff-side ticket workflow (FS-021), and it shares the single canonical ticket-creation routine (`Ticket::create`) and thread-append routines (`Ticket::postMessage` / `Ticket::postNote`) with those specs.

The pipeline has **three intake channels**, all of which converge on the same downstream parse → threading-detection → ticket-create-or-append flow:

1. **Polled fetch (IMAP/POP3)** — for each configured, active mail account the system periodically logs into the remote mailbox over IMAP or POP3, reads new messages, and processes them. Triggered on a schedule by the cron subsystem (FS-043). Implemented around the platform IMAP capability.
2. **Local pipe** — a Mail Transfer Agent (MTA) on the same host pipes a raw RFC-822 message into the pipeline over standard input (`pipe.php`). No API key is required because intake is local-only. The pipeline communicates its outcome back to the MTA via process exit codes, so the MTA can defer/retry/bounce appropriately.
3. **Remote HTTP email post** — a raw RFC-822 message is posted to the email API endpoint (`api/tickets.email`). This requires a valid API key with the create-tickets privilege (key/IP authorization owned by FS-043).

Channels 2 and 3 parse the raw message into a normalized data structure with a generic RFC-822 MIME parser (`Mail_Parse` / `EmailDataParser`). Channel 1 reads message parts directly from the IMAP connection (`MailFetcher`) and produces the same normalized structure. From that point on the three channels are identical: detect whether the message threads onto an existing ticket, and either append to that ticket's thread or create a new ticket.

**This specification owns**: the three intake channels; polling eligibility/frequency/error-backoff rules; per-poll fetch limits and post-fetch disposition (mark-seen, archive-move, delete); MIME parsing rules (sender/subject/body extraction, charset transcoding, attachment extraction); threading-detection mechanics (message-id de-duplication, In-Reply-To / References chain matching, subject ticket-number matching); loop, bounce, and auto-response protection at the email layer; and the email-specific behavior of the shared ticket-create/append routines.

**This specification references** (does not restate): ticket filters and the banlist routing/rejection engine (FS-042); mail-account configuration records and outbound mail/templates (FS-040); the cron scheduler and the HTTP API key authorization (FS-043); the canonical ticket/thread/attachment data model, enum sets, and table/column names (FS-091); attachment de-duplicated chunked storage (FS-022); and global configuration keys (FS-032 / FS-091).

---

## Functional Requirements

### FS-041.1: Local Pipe Intake

**Description**: The system shall accept a raw RFC-822 email piped into it over standard input on the local host and convert it into a ticket or thread entry, reporting the outcome to the calling MTA via a process exit code.

**Acceptance Criteria**:
- The pipe entry point reserves a working memory budget of `256M` before processing, to accommodate emails carrying attachments.
- Intake is rejected unless the process is invoked in a command-line (local) context; a non-local invocation is refused with the message instructing the caller to use the HTTP email endpoint instead. No API key is required for the local pipe.
- The raw message is read from standard input, parsed (FS-041.5), and routed through the shared threading-and-create flow (FS-041.6).
- On success the process reports the created/updated ticket's external number and exits with the success exit code.
- The pipe maps internal outcome codes to MTA-meaningful process exit codes per BS-041.2, so the MTA can decide to accept, defer-and-retry, or bounce.
- If the flow returns no ticket, the pipe emits internal outcome code **416** with the message `Request failed - retry again!`. Per the exit-code map (BS-041.2) outcome 416 yields the **data-error exit code (65)**, not a deferral/retry code; the literal message text nonetheless reads "retry again". (See EC-041.14 — the no-ticket outcome is signaled to the MTA as a data error, so a strictly conforming MTA may bounce rather than defer.)
- The pipe entry point runs the dedicated pipe controller (`PipeApiController::process`), a subclass of the shared email API controller whose only override is the exit-code response mechanism; it reuses the identical `processEmail` flow as the HTTP channel.

### FS-041.2: Remote HTTP Email Intake

**Description**: The system shall accept a raw RFC-822 email posted to the email API endpoint and convert it into a ticket or thread entry, returning an HTTP status.

**Acceptance Criteria**:
- The request requires a valid API key authorized to create tickets; an unauthorized or non-create-privileged key is refused with an unauthorized response (key/IP authorization is owned by FS-043).
- The raw posted body is read, parsed by the email data parser (FS-041.5), and structurally validated against the permitted field set (FS-041.5.1) before processing. The email data parser additionally applies a **post-parse fixup** (FS-041.5.3) that the polled path does not: it forces `source = Email`, substitutes an empty body, an empty subject, and an empty target mail-account id, and strips a parsed priority when email-priority is disabled.
- The request is routed through the same threading-and-create flow (FS-041.6) as the local pipe — but note the HTTP/pipe channel does **not** carry the "seen" out-parameter (BS-041.9.1): a previously-seen-but-rejected message-id is re-evaluated and can attempt creation again on this channel, whereas the polled channel suppresses it.
- On success the endpoint returns a created response carrying the ticket's external number.
- On failure the endpoint returns the appropriate HTTP status (e.g. validation error, denied/rejected, server error) per the shared API responder.

### FS-041.3: Polled Mailbox Fetch (IMAP/POP3)

**Description**: The system shall, on a schedule, log into each configured active mail account over IMAP or POP3, read new messages from the inbox, convert each into a ticket or thread entry, and then dispose of the processed message.

**Acceptance Criteria**:
- Polling runs only when the global email-polling setting is enabled; otherwise the fetch routine returns immediately.
- Polling requires the platform IMAP capability; if that capability is unavailable the routine logs a warning and returns without fetching (this guards against the capability being disabled after mail accounts were configured).
- Only mail accounts that are active and **due** (per the frequency/back-off rules in BS-041.5) are polled; accounts are processed oldest-last-fetch-first.
- For each due account the system opens a connection. On a successful connection it records a successful-fetch timestamp and resets that account's consecutive-error counter to zero, then fetches messages.
- On a failed connection the system increments that account's consecutive-error counter and records a last-error timestamp; after the consecutive-error threshold (BS-041.6) it sends an administrator alert and applies a delayed back-off before retrying that account.
- The default fetch folder is the inbox.
- Within a single account fetch, messages are processed and disposed per FS-041.4. The number processed in one poll is bounded by the account's max-fetch limit (BS-041.4).
- The overall polling pass is time-boxed: it stops launching new account fetches once it has consumed approximately 80% of the configured maximum execution time (default 300 seconds when unbounded), to avoid being killed mid-run.

### FS-041.4: Per-Message Processing & Disposition Within a Poll

**Description**: For each message read from a polled mailbox, the system shall attempt to create/append the ticket and then, only on success, mark, archive, or delete the source message.

**Acceptance Criteria**:
- Messages are processed in reverse order within the mailbox.
- For each message the system reads header info, extracts body and attachments, and runs the shared threading-and-create flow (FS-041.6).
- **On successful processing** of a message the system:
  1. Marks the message as Seen on the server.
  2. If an archive folder is configured, moves the message to that archive folder.
  3. If no archive move occurred (or none configured) **and** the account is configured to delete fetched mail, deletes the message.
- A successful disposition increments the processed counter and resets the consecutive-error counter for the current poll.
- A failed processing increments the consecutive-error counter (messages are left in place — neither marked, archived, nor deleted — so a later poll can retry).
- The per-account fetch loop stops early when either the processed count reaches the account's max-fetch limit, **or** the consecutive-error count exceeds 80% of the max-fetch limit (a runaway-error circuit breaker).
- If total errors in the account exceed total successes, the system logs a warning advising a manual inbox check.
- At the end of the account fetch, the mailbox is expunged (committing the deletions) and the connection closed.

### FS-041.5: MIME Parsing & Normalization

**Description**: The system shall parse a raw or fetched email into a normalized data structure containing sender identity, target mail account, subject, body, threading headers, priority, and attachments.

**Acceptance Criteria**:
- **Decode failure**: a message that cannot be decoded into a valid MIME structure (or that yields no usable headers) is reported as a parse failure; for the HTTP/pipe channels this surfaces a parse-error response and the message is not turned into a ticket.
- **Sender (From)**: the originating address and personal name are taken from the `From` list. When multiple `From` addresses exist, the first **valid** email address is preferred. If the sender has no personal name, the email address is used as the name. (For polled fetch, sender is read from the message header info; name/subject are RFC-2047 MIME-decoded.)
- **Target mail account (emailId)**: the system resolves which configured osTicket mail account the message was addressed to by scanning recipient addresses and matching against known mail-account addresses. This disambiguates inboxes that aggregate multiple osTicket aliases. The recipient set scanned differs by channel:
  - **HTTP/pipe channel** (generic MIME parser): the `To` list **merged with** any `Delivered-To` addresses is scanned first; if no match, the `Cc` list is scanned. `Bcc` is **not** scanned on this channel. If none match, the parser leaves `emailId` empty and the post-parse fixup (FS-041.5.3) substitutes the configured **system default mail-account id**.
  - **Polled fetch channel**: `To`, then `Cc`, then `Bcc` (from the server's parsed header info) are scanned; if none match, the **fetching account's own id** is used as the fallback (not the system default).
- **Subject**: extracted and MIME-decoded to the working charset. An empty subject is substituted with the literal `[No Subject]`.
- **Body**: the preferred body is the `text/plain` part; if absent, the `text/html` part is used. A plain-text body is HTML-escaped. An HTML body has selected block/line-break tags converted to newlines and is then passed through HTML balancing/sanitization (unsafe tags neutralized). After extraction, empty lines are stripped. An email consisting only of attachments (empty body) has its body substituted with a single `-`.
- **Body part selection skips attachment-disposition parts**: when recursing a multipart structure to gather the body, the system descends only into sub-parts that carry **no** `Content-Disposition` (i.e. inline non-attachment parts); parts explicitly marked as a disposition are not concatenated into the body text. (Generic-parser path; the polled path matches the part by MIME type and skips parts that carry a filename parameter.)
- **Charset**: body and header text are transcoded to the working charset (default `UTF-8`) using each part's declared charset when present; header values are RFC-2047 decoded, with a best-effort detection fallback for un-encoded non-ASCII headers.
- **Message-Id (mid)**: the `Message-Id` header is captured. If absent or unreadable, a synthetic id of the form `<{md5 of the raw headers}@local>` is generated so every message has a stable identity.
- **Threading headers**: the `In-Reply-To` and `References` header values are captured verbatim for chain matching (FS-041.6).
- **Reply-To**: when present, the reply-to address and name are captured.
- **Priority**: when use-email-priority is enabled, the `X-Priority` header is mapped to an internal priority (BS-041.10).
- **Attachments**: when email attachments are allowed, attachment parts are extracted per FS-041.5.2.
- **Raw headers**: the full raw header block is retained on the normalized record for downstream loop/bounce/auto-response detection (FS-041.7) and for persistence on the thread entry (FS-041.6).

### FS-041.5.1: Permitted Email-Request Field Set (HTTP/pipe channel)

**Description**: For the HTTP/pipe email channel, the system shall validate the parsed request against a fixed permitted-field set and reject unexpected fields.

**Acceptance Criteria**:
- The wire-level permitted-field allow-list (the base API field set plus the `email`-format extension) is owned canonically by **FS-043.4** (it is an API-endpoint validation concern); this spec consumes that allow-list to validate the parsed email request and states only the pipeline-relevant extension fields below.
- The base permitted fields are: `alert`, `autorespond`, `source`, `topicId`, `name`, `email`, `subject`, `phone`, `phone_ext`, `attachments` (each with `name`, `type`, `data`, `encoding`), `message`, `ip`, `priorityId`.
- For the `email` format the permitted set is additionally extended with: `header`, `mid`, `emailId`, `ticketId`, `reply-to`, `reply-to-name`, `in-reply-to`, `references`.
- Any field outside the permitted set causes a validation error and the request is refused.
- Attachment validation is "soft-fail": an invalid file type, a poorly-encoded base64 payload, or an oversize attachment sets a per-attachment error string but does not abort the whole request; the offending attachment is skipped downstream while the ticket/message is still created. (See BS-041.11.)

### FS-041.5.2: Attachment Extraction

**Description**: The system shall extract attachments and inline parts from the message, including inline images that lack a filename.

**Acceptance Criteria**:
- A message part is treated as an attachment when it has a `Content-Disposition` of `attachment` or `inline`, **or** when its content type is `image/*` or `application/*`, **or** (polled path) when it carries a filename in its content-type/disposition parameters.
- The filename is resolved in preference order: disposition `filename`, then RFC-5987/6266 `filename*` (decoded), then content-type `name`, then `name*`.
- **Inline images without a filename** (as sent by some clients/servers, e.g. Lotus Notes/Domino): when a part is an image with a content-id but no filename, a random filename of the form `image-{4 random chars}.{subtype}` is generated so the image is still captured.
- Each extracted attachment record carries at least a name and a MIME type; the body bytes are fetched and decoded (base64 / quoted-printable / 8-bit / binary) per the part's transfer encoding. For polled fetch, attachment bodies are fetched on demand at save time (not eagerly) to conserve memory.
- Extracted attachments are subject to file-type allow-listing and the maximum-file-size cap (BS-041.11); rejected attachments are logged as a system note on the ticket rather than aborting the message.

### FS-041.5.3: Post-Parse Fixup (HTTP/pipe email channel only)

**Description**: After the generic MIME parser produces the normalized record, the HTTP/pipe email channel applies a fixup pass that the polled-fetch channel does not, normalizing channel-specific defaults before validation/processing.

**Acceptance Criteria** (`ApiEmailDataParser::fixup`):
- The `source` field is forced to `Email`.
- **Empty body substitution differs from the polled path**: an empty/missing body is substituted with the **subject text** when a subject is present, falling back to a single `-` only when the subject is also empty. (The polled path substitutes `-` unconditionally; see BS-041.16.)
- An empty/missing subject is substituted with `[No Subject]`.
- An empty/missing resolved target mail-account id is substituted with the configured **system default mail-account id** (FS-041.5; differs from the polled path's fetching-account fallback).
- When use-email-priority is disabled, any parsed `priorityId` is **removed** from the record (the polled path simply does not populate it when disabled).
- If the parser returned a falsy/empty record (decode failure), the fixup is a no-op and the empty record propagates as a parse failure (FS-041.5 / EC-041.1).

### FS-041.6: Threading Detection & Create-or-Append Flow

**Description**: The system shall determine whether an incoming email belongs to an existing ticket thread and, if so, append it; otherwise it shall create a new ticket. It shall never process the same email message twice.

**Acceptance Criteria** — the flow is evaluated in this exact order:
1. **Explicit ticket id (HTTP/pipe only)**: if the request carries a `ticketId` that resolves to an existing ticket **and** posting the message to that ticket's thread succeeds (returns a message-id), that ticket is returned. If the `ticketId` does not resolve, or the post does not succeed, the flow falls through to step 2 (it does **not** short-circuit to an error). The explicit-ticket-id post uses the shared `postMessage` routine with source `Email`, bypassing the sender-identity append-type classification of BS-041.8 (it is always posted as a Message).
2. **Header/subject thread lookup** (`lookupByEmailHeaders`, BS-041.7): attempt to locate an existing thread entry by, in order:
   a. **Exact message-id match** — if the incoming `mid` is already recorded against a thread entry, the message has been *seen before*; the seen flag is set.
   b. **Reference-chain match** — scan the `mid`, then `In-Reply-To`, then `References` header values for any `<...@...>` message-ids and look each up (References scanned newest-first / right-to-left); the first matching recorded message-id resolves to its thread entry.
   c. **Subject ticket-number match** (last resort) — if the subject contains a `#`-prefixed ticket-number token matching the pattern `#(optional-letters-or-hyphens)(1–10 digits)` **and** the sender email matches the ticket's owner email, resolve to the last message of that ticket. The sender-email match is mandatory here to prevent third-party message injection via a guessed ticket number in the subject.
3. **Append on match**: if a thread entry was found, append the email to it per BS-041.8 (owner → Message; staff → Note; otherwise → Message with a "Received From:" prefix). A successful append returns the existing ticket.
4. **Seen-but-no-append (polled channel only)**: when the lookup positively set the "seen" flag (step 2a) but resolved to no usable thread entry — e.g. the matching record points at thread zero because the message was previously permanently rejected — the **polled** channel treats the email as already-processed and silently accepts it (reports success so the source can be deleted/moved) without creating a duplicate. The **HTTP/pipe channel does not consult the seen flag** (it does not pass the out-parameter), so on that channel a seen-but-rejected message falls through to step 5 and a fresh creation attempt is made (which the rejection filter/ban will typically deny again). See BS-041.9.1.
5. **Create new ticket**: if no thread match is found (and, on the polled channel, the message-id was not seen), a new ticket is created via the shared `Ticket::create` with origin/source `Email` (FS-041.8 / FS-041.9).

### FS-041.7: Loop, Bounce & Auto-Response Protection

**Description**: The system shall detect and suppress mail loops, delivery bounces, and automated replies so that the help desk does not create runaway tickets or ping-pong autoresponders with other mail systems.

**Acceptance Criteria**:
- **Mail from the system's own addresses**: if the sender address matches any configured osTicket mail account, the email is not processed into a new thread item (it is treated as system-originated and accepted as already-handled) — this breaks self-mail loops.
- **Auto-response detection** (BS-041.12): a message whose headers indicate an automatic response (out-of-office, vacation reply, bulk/list/junk precedence, auto-submitted, various `X-Auto*` headers, or a recognized auto-reply subject prefix) is still threaded/created where applicable, but **no autoresponse is sent back** to it.
- **Bounce detection** (BS-041.13): a message whose headers indicate a delivery failure/bounce (mailer-daemon / null-sender From, or a delivery-failure/undeliverable subject) is recognized as a bounce. When a bounce cannot be threaded onto an existing ticket and would otherwise create a ticket, it is logged as a bounced-email warning and dropped (reported success so the source is removed) rather than opening a new ticket.
- **Mailer-daemon / postmaster senders**: even when a ticket is created, autoresponse is suppressed if the sender localpart is `mailer-daemon@…` or `postmaster@…`.
- **Banlist / reject rules** (FS-042): a banned sender address causes the email to be denied; the denial is logged and the source message is reported as handled (moved/deleted) so it is not retried. A filter-rule rejection likewise denies the email with a logged reason. **Ban-check timing differs by channel**: the **polled** channel checks the sender ban *up front* — before any thread lookup — so a banned sender's email is dropped even when it would otherwise have threaded onto an existing ticket (replies from a banned address are silently discarded as "moved/deleted"). The **HTTP/pipe** channel performs no standalone pre-lookup ban check; the ban is enforced only inside the shared create routine, so a banned sender's reply that threads onto an existing ticket on the HTTP/pipe channel is still appended.
- **Permanent rejection memo**: when a new-ticket attempt is rejected (denial / error code 403), the system records the rejected message-id against a zero thread so the same email is never re-evaluated on a subsequent poll.
- **Open-ticket cap (loop control)**: when the configured maximum-open-tickets-per-requester limit is set and reached for the sender (non-staff origins), a further new-ticket email is denied with a logged warning (see FS-011 / FS-091 for the cap value and behavior).

### FS-041.8: Email-Sourced Thread Append Semantics

**Description**: When an email threads onto an existing ticket, the system shall classify the append as a Message, a Note, or a Message-with-attribution based on who sent it.

**Acceptance Criteria** (`ThreadEntry::postEmail`):
- If the appending thread entry has no parent ticket, the append fails (cannot continue a conversation without a ticket).
- **Duplicate guard**: if the email's message-id equals the message-id already recorded on the target thread entry, the append is treated as already-done and reported successful (no duplicate post).
- **Sender = ticket owner** (sender email matches the ticket's email): the email is posted as a **Message** (thread type M).
- **Sender = a staff member** (sender email matches a staff account): the email is posted as an internal **Note** (thread type N), attributed to that staff member.
- **Sender = one of the system's own mail accounts**: the email is not posted (loop control — see FS-041.7).
- **Sender = anyone else** (collaborator / third party): the email is posted as a **Message** with the body prefixed `Received From: {email}` so the actual originator is recorded.
- On append, the incoming message-id and raw headers are persisted to the ticket email-info record (FS-041.6 dedupe data) so future replies in the chain can be matched.
- **Quoted-reply stripping**: on an emailed reply posted as a Message, when strip-quoted-reply is enabled and the configured reply separator marker is found in the body, everything from the separator onward is removed (keeping only the new content above it), provided the remaining text is non-empty.

### FS-041.9: Email-Sourced New-Ticket Creation Semantics

**Description**: When an email does not thread onto any existing ticket, the system shall create a new ticket via the shared creation routine using the email-origin rules.

**Acceptance Criteria** (email-specific behavior within `Ticket::create`):
- Required fields for email origin are name, email, subject, message, and a resolved target mail-account id (`emailId`). A missing/unknown target mail account is a validation error.
- Routing defaults are derived from the target mail account when no help topic is supplied: the ticket's department and (if not otherwise set) priority come from the mail account's configuration; the source is fixed to `Email`.
- When a help topic is present (e.g. supplied via the HTTP channel), topic-derived department/priority/SLA/auto-assignment take precedence per the shared create rules (FS-011).
- Ticket filters are applied to the email's create arguments before insertion (routing, canned response, auto-assignment, rejection) — owned by FS-042.
- The initial subject is used as the first message's title; the message body becomes the first thread Message.
- After creation, autoresponse is conditionally suppressed (FS-041.7: self-address, auto-response headers, mailer-daemon/postmaster sender, department/topic autoresponse settings, or a canned auto-reply having been sent).
- Email attachments (when allowed) are imported onto the new ticket's first message; type/size-rejected attachments are logged as system notes rather than aborting creation.

### FS-041.10: Outcome Signaling to the MTA (pipe channel)

**Description**: The local pipe shall translate processing outcomes into process exit codes that instruct the calling MTA how to proceed.

**Acceptance Criteria**: outcomes map to exit codes per BS-041.2 (success → accept; validation → permanent fail; permission/denied → permanent fail; unsupported/parse → data error; service-unavailable → temp-fail/defer; unknown/server error → temp-fail/retry). The pipe does not emit an HTTP body; the exit code is the entire response to the MTA.

---

## Business Rules

> Enum sets, table/column names, and global default values referenced below are owned canonically by **FS-091**; this section states only the email-pipeline-specific behavioral rules and the literal thresholds the pipeline itself enforces.

### BS-041.1: Three Intake Channels, One Downstream Flow
**Rule**: Local pipe, remote HTTP email post, and polled IMAP/POP3 fetch all normalize to the same data structure and run the identical threading-detection → create-or-append flow. The only differences are the transport, the parser used to obtain the normalized structure, and the outcome-signaling mechanism (exit code vs HTTP status vs server-side disposition).
**Rationale**: One canonical flow guarantees that an emailed reply threads the same way regardless of how the mail reached the system.

### BS-041.2: Pipe Exit-Code Mapping
**Rule**: The local pipe maps internal outcome codes to MTA exit codes as follows:

| Internal outcome | Exit code | MTA meaning |
|---|---|---|
| 201 success | 0 | Accepted |
| 400 validation | 66 | Permanent failure (cannot deliver) |
| 401 / 403 permission/denied | 77 | Permission denied (permanent) |
| 415 / 416 / 417 / 501 | 65 | Data / format error |
| 503 service unavailable | 69 | Service unavailable (defer) |
| 500 / default (unknown) | 75 | Temporary failure — retry later |

**Rationale**: Postfix-style exit codes let the MTA decide whether to accept, bounce, or defer-and-retry without an HTTP channel.

### BS-041.3: Local-Only Pipe
**Rule**: The pipe channel only accepts local (CLI) invocation and requires no API key; remote senders must use the HTTP email endpoint (which requires an authorized API key).
**Rationale**: A local MTA handoff is implicitly trusted; a remote post must be authenticated.

### BS-041.4: Per-Poll Fetch Limit
**Rule**: Each mail account has a max-fetch-per-poll limit. When the account's configured value is missing or non-numeric, the default limit is **20** messages per poll. A poll stops processing an account once the processed count reaches this limit.
**Rationale**: Bounds the work per poll so a large backlog is drained gradually rather than in one long-running pass.

### BS-041.5: Polling Eligibility & Frequency
**Rule**: An account is polled only when it is active **and** either it has never been fetched or the elapsed time since its last fetch exceeds its configured fetch-frequency (in minutes) **and** it is not currently in error back-off. Accounts are polled in order of oldest last-fetch first.
**Rationale**: Honors each account's configured poll interval and spreads work fairly across accounts.

### BS-041.6: Connection Error Back-Off
**Rule**: The polling routine tolerates up to **5** consecutive connection errors per account; once an account reaches 5 consecutive errors it enters a delayed-retry state and is skipped until **10** minutes have elapsed since its last error. On reaching the threshold an administrator alert email is sent describing the account, host, and last error. A successful connection resets the account's consecutive-error counter to zero.
**Rationale**: Prevents hammering an unreachable/mis-credentialed mailbox every poll while still alerting the operator.

### BS-041.7: Thread-Match Precedence
**Rule**: Existing-thread lookup is attempted strictly in this precedence: (1) exact recorded message-id (also sets the "seen" flag); (2) message-ids found in the `mid` / `In-Reply-To` / `References` headers, with `References` scanned newest-first; (3) a `#`-ticket-number in the subject matching `#([\p{L}-]+)?([0-9]{1,10})`, gated on the sender email matching the ticket owner. The first match wins.
**Rationale**: Header-based threading (RFC-2822 / RFC-1036) is reliable; subject-number matching is a fragile last resort and is therefore additionally gated on sender identity to prevent injection.

### BS-041.8: Append Type By Sender Identity

| Incoming sender | Result |
|---|---|
| Matches ticket owner email | Posted as **Message** (M) |
| Matches a staff account email | Posted as internal **Note** (N) |
| Matches a system mail-account address | **Not posted** (loop control) |
| Anyone else | Posted as **Message** (M), body prefixed `Received From: {email}` |

**Rule**: The sender's identity determines whether an emailed reply becomes a public Message or an internal Note, and guards against the system replying to itself.
**Rationale**: Staff replying by email should produce internal notes; the ticket owner produces public messages; third parties are captured with attribution.

### BS-041.9: Idempotent Processing (Message-Id De-duplication)
**Rule**: Every processed email's message-id (real or synthetic `<md5@local>`) is recorded against a thread entry. A subsequently re-fetched email whose message-id is already recorded is recognized as already-processed and is accepted as success **without** creating a duplicate ticket or thread item. Permanently-rejected emails record their message-id against a zero thread so they are never re-evaluated.
**Rationale**: Polled mailboxes may re-present undeleted messages; idempotency by message-id prevents duplicate tickets on re-poll.

### BS-041.9.1: "Seen" Flag Is Consulted Only on the Polled Channel
**Rule**: The thread-lookup routine exposes a by-reference "seen" out-parameter that it sets to true whenever the incoming message-id exactly matches a recorded `email_mid` — even when that record points at thread zero (a previously permanently-rejected email) and therefore resolves to no usable thread entry. The **polled** fetch path passes and honors this flag, so a seen-but-unresolvable message is treated as already-processed and dropped. The **HTTP/pipe** path calls the same lookup **without** the out-parameter, so it cannot distinguish "never seen" from "seen but rejected"; a seen-but-rejected message on that channel falls through to a fresh create attempt (typically re-denied by the ban/filter that rejected it originally).
**Rationale**: Documents an observed cross-channel asymmetry: idempotent suppression of rejected mail is guaranteed only for polled fetch; the HTTP/pipe channel relies on the create-time rejection re-firing.

### BS-041.10: Email Priority Mapping
**Rule**: When use-email-priority is enabled, the `X-Priority` header maps to an internal priority: value `> 4` → priority 1; value `3–4` → priority 2; value `1–2` → priority 3; absent/non-numeric → 0 (unset, default applies). The `X-Priority` value is extracted by a **raw substring scan** of the literal header block (find `X-Priority:`, read to the next newline, strip all non-digits) rather than by structured header parsing; consequently any non-digit decoration (e.g. `3 (Normal)`) is reduced to its digits and a header value that contains no digits maps to 0. The priority is mapped from the **first** `X-Priority:` occurrence found in the block.
**Rationale**: Honors sender-asserted urgency where configured, while defaulting safely. Documenting the substring-scan extraction explains why malformed/decorated priority headers still map deterministically.

### BS-041.11: Attachment Acceptance Limits
**Rule**: Email attachments are imported only when email attachments are globally allowed. Each attachment must pass the file-type allow-list and must not exceed the configured maximum file size; a poorly-encoded base64 payload is rejected. A rejected attachment is logged as a system note on the ticket and skipped — it does not abort ticket/message creation (soft-fail).
**Rationale**: Keeps a single bad attachment from blocking an otherwise valid email; size/type limits owned by FS-032/FS-091.

### BS-041.12: Auto-Response Suppression Triggers
**Rule**: An emailed message recognized as an automatic response (out-of-office / vacation reply, bulk/list/junk precedence, auto-submitted, `X-Auto*` suppression headers, recognized auto-reply subject prefixes, notification-relay headers) must NOT receive an autoresponse. **The canonical auto-response/bounce marker vocabulary (the literal header names, value prefixes, and subject markers) is owned by FS-042.9; this spec consumes that list at the pipeline layer.** In addition to the FS-042.9 markers, a message that is a bounce (BS-041.13), a sender of `mailer-daemon@…` or `postmaster@…`, or a sender matching a system mail account also suppresses autoresponse.
**Rationale**: Prevents the help desk from auto-replying to vacation responders, lists, and daemons (which would create mail loops). The marker list is centralized in FS-042.9 to avoid divergence; this rule owns only the pipeline-side suppression behavior.

### BS-041.13: Bounce Detection & Handling
**Rule**: Bounce recognition (null-sender / mailer-daemon `From` markers and delivery-failure / undeliverable `Subject` markers) uses the **canonical bounce marker vocabulary owned by FS-042.9**; this spec does not restate the literal marker strings. Bounces also count as auto-responses for suppression (BS-041.12).
**Bounce-drop timing (polled channel)** — *canonical here; FS-041 owns the pipeline drop behavior*: the bounce-to-drop short-circuit is **not** a pre-create gate — it is evaluated only *after* the shared create routine has been attempted and **failed to return a ticket**. Concretely, on the polled path, if `Ticket::create` returns no ticket and the failure was not a permanent (403) rejection, the headers are tested for a bounce; only then is the message logged as a bounced-email warning and reported as handled (dropped). A bounce that nonetheless threads onto an existing ticket, or one that passes filters and successfully creates a ticket, is **not** dropped by this rule (its autoresponse is still suppressed via the auto-response rule). On the HTTP/pipe channel there is no equivalent bounce-drop branch; a create failure there returns an error status to the caller.
**Rationale**: Bounce-backs must not spawn new tickets nor trigger further outbound mail; the drop is positioned on the create-failure branch so it does not interfere with legitimate threaded bounces.

### BS-041.14: Post-Fetch Disposition Order
**Rule**: A successfully-processed polled message is, in order: (1) marked Seen; (2) moved to the archive folder if one is configured; (3) deleted only if it was not archived and the account is configured to delete fetched mail. Deletions are committed by an expunge at the end of the account fetch. Failed messages are left untouched for a later retry.
**Rationale**: Archive-then-delete precedence preserves a copy when archiving is enabled; leaving failures in place enables retry/idempotent reprocessing.

### BS-041.15: Runaway-Error Circuit Breaker & Time-Box
**Rule**: A per-account fetch stops early when processed count reaches max-fetch, or when consecutive errors exceed 80% of max-fetch. The overall poll stops launching new account fetches at ~80% of the maximum execution time (default budget 300s when unbounded). Excess errors (errors > successes) trigger a "manually check the inbox" warning.
**Rationale**: Bounds runaway failures and avoids the process being killed mid-fetch (which could leave mail half-processed).

### BS-041.16: Empty Subject & Empty Body Substitution
**Rule**: An empty/missing subject is replaced with `[No Subject]`; an email with no body text (e.g. attachments only) has its body replaced with a single `-`; a missing sender name is replaced with the sender email address.
**Rationale**: Downstream ticket creation requires non-empty name/subject/message fields.

### BS-041.17: Multi-Valued Message-Id Resolution
**Rule**: When the `Message-Id` header appears more than once (yielding an array of values), the system retains the **last non-empty** value (after filtering out empties). When the header is absent or unreadable, a synthetic id `<{md5 of the raw header block}@local>` is generated. On the polled path, when the server-supplied message-id is empty the system first re-reads the raw header block for a `message-id` entry (again taking the last non-empty when multi-valued) before falling back to the synthetic hash.
**Rationale**: Some automated senders emit duplicate Message-Id headers; choosing a single deterministic value keeps de-duplication stable.

### BS-041.18: Thread-Post Message-Id Backfill
**Rule**: Every thread post (message/note/response) must carry a message-id for the email-info de-duplication record. When a post is created without one (e.g. a staff-side post that did not originate from email), a synthetic id `<{24 random chars}@{last 10 hex of the md5 of the system base URL}>` is generated and recorded. An email-info row is written only when a non-empty message-id is present.
**Rationale**: Guarantees that every thread entry has a stable identity so future emailed replies can thread onto it.

### BS-041.19: Connection Reuse, Login Hardening & Operation Timeout
**Rule**: A poll reuses an already-open mailbox connection when a ping succeeds; otherwise it (re)opens. On open, Kerberos (GSSAPI) and NTLM authenticators are explicitly disabled on platforms that support that option, and an operation timeout of **20 seconds** is requested for open/read operations. The connection is always opened against the default inbox folder. At account-fetch end the mailbox is closed with an expunge so pending deletions are committed; an explicit expunge is also issued before close.
**Rationale**: Avoids redundant logins within a pass, sidesteps brittle SSO authenticators, and bounds per-operation blocking so one slow mailbox cannot stall the whole pass.

---

## Data Requirements

The pipeline produces and consumes a **normalized email record** (a transient in-memory hashtable, not a stored entity). Persistence of the resulting ticket/thread/attachment/email-info rows is owned canonically by **FS-091**; this section lists only the fields the pipeline populates and the de-duplication data it persists.

### Normalized Email Record (transient)

| Field | Meaning | Source |
|---|---|---|
| `email` | Originating sender address (lowercased) | `From` (first valid) / header info |
| `name` | Sender personal name (MIME-decoded; falls back to `email`) | `From` personal / comment |
| `subject` | MIME-decoded subject (falls back to `[No Subject]`) | `Subject` |
| `message` | Extracted, sanitized, empty-line-stripped body (falls back to `-`) | preferred `text/plain` else `text/html` |
| `emailId` | Resolved target osTicket mail-account id. Fallback differs by channel: polled → fetching account id; HTTP/pipe → configured system default mail-account id (FS-041.5.3) | polled: `To`/`Cc`/`Bcc`; HTTP/pipe: `To`(+`Delivered-To`)/`Cc` |
| `source` | Forced to `Email` on HTTP/pipe channel by fixup (FS-041.5.3) | constant |
| `mid` | Message-Id (synthetic `<md5@local>` if absent) | `Message-Id` |
| `in-reply-to` | Parent message-id(s) for chain matching | `In-Reply-To` |
| `references` | Ancestor message-id list for chain matching | `References` |
| `reply-to`, `reply-to-name` | Reply-To address/name when present | `Reply-To` |
| `priorityId` | Mapped internal priority (when use-email-priority on) | `X-Priority` (BS-041.10) |
| `header` | Full raw header block (for loop/bounce/auto-response detection and persistence) | raw message |
| `ticketId` | Explicit existing ticket id (HTTP/pipe email field only) | request field |
| `attachments[]` | `{name, type, data/encoding, [index], [error]}` per part | MIME parts (when allowed) |

### Persisted De-duplication Data (owned by FS-091; referenced here)

- **Ticket email-info record** — stores, per thread entry, the email message-id and the raw headers. This is the table consulted by the thread-match lookup (BS-041.7 / BS-041.9). A permanently-rejected email records its message-id against a zero thread to block re-processing.

### Mail-Account Settings Consumed (owned by FS-040; referenced here)

Host, port, protocol (`pop`→`pop3` / `imap`, lowercased), encryption (SSL → `/ssl`; connections always use `/novalidate-cert`), username, password, max-fetch-per-poll (default 20), fetch-frequency (minutes), delete-after-fetch flag, archive-folder, active flag, plus the per-account error bookkeeping (consecutive errors, last-error time, last-fetch time) used by BS-041.5/BS-041.6.

### Global Configuration Keys Consumed (owned by FS-032 / FS-091; referenced here)

Email-polling-enabled, allow-email-attachments, allow-API-attachments, use-email-priority, maximum-file-size, file-type allow-list, strip-quoted-reply + reply-separator marker, maximum-open-tickets-per-requester, default department/priority/SLA/template/email, and the per-department/per-topic autoresponse-on-new-ticket / on-new-message settings consulted during suppression.

---

## User Flows / Interactions

This pipeline is **machine-driven**; "users" here are external mail senders, the MTA, and the cron scheduler. There is no interactive UI in this spec (mail-account configuration UI is FS-040; cron/API-key UI is FS-043).

### Flow 1: New Ticket From an Inbound Email (polled)
1. Cron triggers a polling pass; the account is due and active (BS-041.5).
2. The system connects, resets the error counter, and reads new inbox messages in reverse order.
3. For a message, header info is read; the sender is not banned, not a system address, not a bounce.
4. No message-id match, no reference-chain match, no subject ticket-number match → no existing thread.
5. A new ticket is created (origin `Email`); department/priority derive from the target mail account; the body becomes the first Message; allowed attachments are imported.
6. Autoresponse is sent unless suppressed (BS-041.12); staff alerts fire per config.
7. The message is marked Seen, archived (if configured) or deleted (if configured), and the mailbox expunged.

### Flow 2: Emailed Reply Threads Onto an Existing Ticket
1. An inbound email carries `In-Reply-To`/`References` pointing at a prior osTicket message (or a `#NUMBER` in the subject from the ticket owner).
2. Thread lookup matches an existing thread entry (BS-041.7).
3. The append type is chosen by sender identity (BS-041.8): owner → Message, staff → Note, third party → attributed Message.
4. Quoted-reply text below the reply separator is stripped if enabled; the new content is posted; the new message-id/headers are recorded for future matching.
5. New-message alerts fire per config; no new ticket is created.

### Flow 3: Local MTA Pipe
1. The MTA pipes a raw message to the pipe entry point over stdin.
2. The message is parsed and run through the same threading-and-create flow.
3. The pipe exits 0 on success (ticket number reported) or a deferral/permanent code per BS-041.2; the MTA acts on the exit code.

### Flow 4: Auto-Reply / Bounce Arrives
1. An out-of-office auto-reply (or a mailer-daemon bounce) is fetched.
2. If it threads onto a ticket it is appended, but **no autoresponse** is returned (BS-041.12).
3. If it would create a new ticket and is a bounce, it is logged and dropped (BS-041.13); if it is from a system address it is accepted-as-handled without posting (FS-041.7), breaking the loop.

---

## Edge Cases & Error Scenarios

### EC-041.1: Undeliverable / Un-parseable Message
A message that cannot be decoded into a valid MIME structure (or yields no usable headers) is a parse failure: HTTP/pipe channels return a parse-error response (and the corresponding pipe exit code); the message is not turned into a ticket. For polled fetch, header-info read failure causes the message to be skipped (counts as an error, left in place for retry).

### EC-041.2: Missing Message-Id
If a message has no readable `Message-Id`, a synthetic `<{md5 of headers}@local>` id is generated so the message still has a stable identity for de-duplication and threading.

### EC-041.3: Re-fetched (Undeleted) Message
A message re-presented on a later poll whose message-id is already recorded is recognized as already-processed and accepted as success without creating a duplicate (BS-041.9). This is the normal case when delete-after-fetch is off and no archive folder is set.

### EC-041.4: Subject Ticket-Number From a Stranger
A `#NUMBER` in the subject only threads when the sender email matches the ticket owner. An email from a different address carrying a guessed ticket number does NOT thread onto that ticket (anti-injection, BS-041.7); it instead creates a new ticket.

### EC-041.5: Email Sent From One of osTicket's Own Addresses
The message is treated as system-originated: it is not posted as a thread item and is accepted as already-handled, breaking the self-mail loop (FS-041.7 / BS-041.8).

### EC-041.6: Attachments-Only Email
An email with no textual body but with attachments has its body substituted with `-` so the required message field is satisfied; the attachments are imported normally (BS-041.16, FS-041.5.2).

### EC-041.7: Oversize / Disallowed Attachment
A too-large or disallowed-type attachment (or poorly-encoded base64) is soft-failed: the error is logged as a system note on the ticket and the attachment is skipped, but the ticket/message is still created (BS-041.11).

### EC-041.8: Banned Sender / Filter Rejection
A banned sender or a filter-rule rejection denies the email; the denial is logged, the source is reported as handled (so it is not retried), and the rejected message-id is recorded against a zero thread to block future re-evaluation (FS-041.7, FS-042).

### EC-041.9: Maximum Open Tickets Reached
A non-staff sender at the configured open-ticket cap has further new-ticket emails denied with a logged warning (loop control); existing-ticket replies still thread normally (FS-041.7).

### EC-041.10: Mail Server Unreachable
Repeated connection failures back off after 5 consecutive errors (10-minute delay) and alert the administrator; the account's mail is left untouched and retried later (BS-041.6).

### EC-041.11: Poll Time-Box / Runaway Errors
A long backlog is bounded by max-fetch per account, the per-account consecutive-error circuit breaker (>80% of max-fetch), and the overall ~80%-of-max-execution-time stop; remaining mail is processed on subsequent polls (BS-041.15).

### EC-041.12: HTML-Only Body
When no `text/plain` part exists, the `text/html` part is converted (selected tags → newlines) and sanitized (unsafe tags neutralized) before becoming the message body (FS-041.5).

### EC-041.13: Aggregated Inbox With Multiple osTicket Aliases
A single fetched inbox receiving mail for several osTicket addresses resolves each message's target account by scanning recipients against known accounts. The polled path scans To/Cc/Bcc and falls back to the fetching account; the HTTP/pipe path scans To(+Delivered-To)/Cc (not Bcc) and falls back to the configured **system default** mail account (FS-041.5, FS-041.5.3).

### EC-041.14: Pipe Returns No Ticket
When the local-pipe flow completes without producing a ticket (no thread match and creation did not return a ticket), the pipe emits internal outcome 416 with the text `Request failed - retry again!`, which the exit-code map (BS-041.2) translates to exit code **65 (data error)** — *not* a temp-fail/retry code. The literal message implies a retry, but a strictly-conforming MTA treating 65 as permanent may bounce. (Most real no-ticket cases — bans, filter rejection, bounces — are intercepted earlier and reported as success, so this path is reached mainly on unexpected create failure.)

### EC-041.15: Previously-Rejected Email Re-Posted via HTTP/Pipe
A message-id that was permanently rejected on a prior poll is recorded against thread zero. If the *same* email is later posted through the HTTP/pipe channel, that channel does not consult the "seen" flag, so it does not recognize the message as already-rejected and re-runs the create flow; the email is re-denied only if the original ban/filter rejection still applies (BS-041.9.1). The polled channel, by contrast, would silently drop it as already-processed.

### EC-041.16: Banned Sender Reply via HTTP/Pipe vs Polled
A reply from a banned sender that threads onto an existing ticket is **dropped** when fetched on the polled channel (which ban-checks before lookup) but is **appended** when posted via the HTTP/pipe channel (which only ban-checks inside new-ticket creation, not on thread append) — see BS-041.13/FS-041.7 ban-timing note.

---

## Dependencies

| Dependency | Specification | Relationship |
|---|---|---|
| Ticket filters & banlist (`isBanned`, filter `apply`, `RejectedException`) | FS-042 | Inbound routing, rejection, and ban checks invoked during create/append |
| Mail-account configuration records & outbound mail/templates | FS-040 | Provides the polled-account credentials/settings and the autoresponse/alert outbound side |
| Cron scheduler & HTTP API-key authorization | FS-043 | Triggers polling passes; authorizes the remote HTTP email post |
| Shared `Ticket::create` / `postMessage` / `postNote` / thread model | FS-011, FS-021, FS-091 | The canonical create/append routines and the ticket/thread/email-info/attachment tables |
| Attachment de-duplicated chunked storage (`AttachmentFile`) | FS-022 | Where extracted attachment bytes are persisted |
| Charset transcoding, format/sanitize helpers (`Charset`, `Format`, `Validator`) | FS-003 | MIME charset conversion, HTML sanitization, email validation |
| Reference data, enums, table/column names, config defaults | FS-091 | Ticket source/thread-type enums, priority values, table names, config keys |
| Web ticket submission (shared create path, open-ticket cap) | FS-011 | Shares `Ticket::create`; cap behavior defined there |

---

## Known Limitations

### KL-041.1: IMAP/POP3 Fetch Requires the Platform IMAP Capability
Polled fetch depends on the platform IMAP capability being present; if it is disabled after mail accounts were configured, polling silently no-ops (with a warning) and inbound email via fetch stops. The pipe and HTTP channels are unaffected.

### KL-041.2: Single Fetch Folder (Inbox Only)
The polled fetcher reads only the default inbox folder; there is no per-account option to fetch from a different folder or label (noted as a TODO in the source). Archiving moves processed mail to a configured folder but reading is inbox-only.

### KL-041.3: Subject-Number Threading Is Fragile
Thread matching by `#NUMBER` in the subject is a last resort and is gated on sender-email match. If a legitimate reply changes the From address (e.g. forwarded from a different mailbox) and lacks header threading, it will open a new ticket rather than thread.

### KL-041.4: Synthetic Message-Id Is Header-Hash Based
When `Message-Id` is missing, the synthetic id is the md5 of the raw headers. Two distinct emails with byte-identical headers (rare, but possible for some automated senders) could collide and be treated as duplicates.

### KL-041.5: Auto-Response/Bounce Detection Is Heuristic
Loop/bounce/auto-response suppression relies on a fixed set of header/subject heuristics (BS-041.12 / BS-041.13). Non-conforming auto-responders or bounce formats may slip through (creating a ticket and possibly an autoresponse) or, conversely, a legitimate message with a matching subject prefix could be mis-suppressed. Matches are anchored to the start of the header value to reduce false positives.

### KL-041.6: Connection Always Disables Certificate Validation
Polled connections always include `/novalidate-cert`, so TLS server certificates are not validated even when SSL is selected. This trades strict transport security for tolerance of self-signed/mis-matched certs on mail servers.

### KL-041.7: No Server-Side Size Cap Before Download (pipe/HTTP)
The pipe reserves a fixed 256M memory budget and relies on per-attachment size checks after parsing; an extremely large piped message could still pressure memory before the per-attachment cap is applied. Polled fetch mitigates this by fetching attachment bodies on demand.

---

## Future Considerations

- Configurable source folder/label per account (beyond inbox-only).
- Stronger transport security (optional certificate validation) for fetched accounts.
- Richer, standards-based auto-submitted/bounce detection (e.g. full RFC-3464 DSN parsing) to reduce heuristic misses.
- A pre-download size cap / streaming parse for very large piped messages.
- De-duplication keyed on more than header-hash for messages lacking a Message-Id.
- Native protocol fetch support independent of the platform IMAP capability (a standalone POP3/IMAP client; note the planned `class.pop3.php` is not present in this snapshot).
