# M2 — MILESTONE – Attachments, Canned Responses & Email Basics

- **ID**: M2
- **Type**: Milestone (root ticket — owns its own E2E ACs)
- **Parent**: — (top of hierarchy)
- **Labels**: Milestone
- **Column**: derived from children (`min(children.columns)`); promoted to `qa` only when its own
  browser-only E2E ACs (below) all pass.

## Spec References

- FS-022 (canned responses + content-addressed file store + attachment download/validation)
- FS-040 (variable-substitution grammar + outbound mail composition/wrappers/templates)
- FS-011 (public submission — attachment hook on create)
- FS-021 (staff reply — attachment hook + canned consumption on reply)

## Description

Builds directly on M1's round-trip. After M2, tickets and replies can carry **attachments**
(a real filesystem blob store keyed by SHA-256), agents reply faster with a **canned-response
library** whose bodies are personalised by a **`%{token}` variable-substitution engine**, and the
M1 stub mailer is replaced by **real outbound email basics** — an autoresponse when a ticket is
opened and a notification when staff reply — delivered over SMTP and observable in **Mailpit**.

Business value: this is the first milestone where the product touches the customer *outside* the
web portal (real email lands in their inbox) and where attachments — the single most-requested
helpdesk affordance — work end to end. It turns the M1 toy loop into something demonstrable as a
real support desk.

Scope deliberately excludes (deferred): canned-response **CRUD UI** (M4 admin), inline image
display, multi-attachment download-all, FAQ/logo file types, the chunked-DB store, inbound email
fetch/pipe/parse (M5), email filters/banlist (M5), per-account/template-set admin UI (M4/M5),
and HTML email. The two seed canned responses and the single seeded template set stand in for the
admin CRUD that arrives later.

## Deviations from legacy (pinned for M2 — see ROADMAP.md → M2)

- **D1 — Filesystem blob store, not chunked-DB.** Legacy FS-022.12 stores file bytes in a chunked
  DB table keyed by a time-salted hash (no real dedup, KL-022.4). M2 instead writes blobs to the
  filesystem under `var/blobs/ab/cd/<sha256>` keyed by the **content SHA-256**, giving **real
  byte-level dedup**. `attachment_file` carries a `storage_key` pointer; no chunk table.
- **D2 — Session-on-parent-ticket download auth, not session-bound MD5 hash.** Legacy FS-022.10 /
  BS-022.8 gate downloads on a per-session `md5(file_id+session_id+file_hash)` access hash. M2
  preserves the *observable behaviour* of BS-022.8 (a requester may only download an attachment
  whose parent ticket they can access) but enforces it with a plain **session check on the parent
  ticket** instead of the MD5 hash scheme (KL-022.5 obsoleted).
- **D3 — Env-driven single SMTP transport, not per-account/template-set DB config.** Legacy FS-040
  selects a sending account/transport from DB rows (BS-040.21). M2 ships **one** transport from env
  (`SMTP_HOST/PORT/USER/PASS/FROM`); **SMTP is active when `SMTP_HOST` is set**, otherwise the M1
  stub mailer + `GET /api/dev/mailbox` is kept. The packaged-default templates (FS-040.10) and the
  single seeded template set stand in for the template-set admin UI (deferred to M5).

## Acceptance Criteria (root E2E — browser-only journey, no [API-ONLY])

> One continuous browser journey covering every descendant US, validated by qa-criterion-tester
> against the running frontend once all children reach `qa`. The journey runs WITHOUT re-seeding
> between steps — state created in AC-1 flows through to AC-5. Pre-flight: **purge + reseed**
> (`cargo run -p tools --bin seed -- --reset`, TS-M2-prep) run ONCE before AC-1, and **Mailpit is
> running** (SMTP :3704 / web UI :3705) with `SMTP_HOST` pointing the backend at it. Each AC step is
> tagged with the originating ticket; on a step failure, that originating child is moved back to
> inProgress.

### AC-1 (US-M2-1): Client opens a ticket WITH a `.pdf` attachment and sees it confirmed. [BROWSER]
- Setup: ONE-TIME pre-flight only — `cargo run -p tools --bin seed -- --reset` (purge tickets + reseed baseline); Mailpit up; clean browser session.
- Navigate: http://localhost:3702/open
- Action: fill Name "Mia Wong", Email "mia@example.com", Subject "Invoice query", Message "See the attached invoice.".
- Action: in the **attachment file input**, choose a small `.pdf` (e.g. `invoice.pdf`, < 1 MB).
- Action: submit the form.
- Verify: the confirmation page shows a generated 6-digit ticket number AND an **attachment chip** naming `invoice.pdf`. CARRY {ticketNumber, email="mia@example.com", attachmentName="invoice.pdf"} forward — do not reseed.
- Status: [ ]

### AC-2 (US-M2-2): Agent logs in, opens the ticket, sees the client's attachment chip on the original message. [BROWSER]
- Navigate: http://localhost:3702/staff/login (staff realm; distinct ost_staff_sess cookie). Action: log in agent / Agent123!.
- Navigate: open the carried ticket from the Open queue (http://localhost:3702/staff/tickets).
- Verify: the detail thread shows the customer's `M` message "See the attached invoice." with a clickable **attachment chip** `invoice.pdf` attached to it.
- Status: [ ]

### AC-3 (US-M2-2): Agent replies USING a canned response, which fills the box with a substituted body + carries its attachment. [BROWSER]
- Action: in the reply box, open the **"Canned response" dropdown** and pick "Acknowledge receipt".
- Verify: the reply textarea is populated with the canned body and the **`%{ticket.number}` token is already substituted** to the carried ticket number (no literal `%{...}` remains); a chip shows the canned response's carried `.txt` attachment (e.g. `policy.txt`).
- Action: post the reply.
- Verify: the reply appears in the thread as an agent `R` response with the substituted text and the `policy.txt` chip.
- Status: [ ]

### AC-4 (US-M2-4): Client RECEIVES the reply notification email — visible in the Mailpit web UI. [BROWSER]
- Navigate: http://localhost:3705 (Mailpit web UI) in the browser.
- Verify: the Mailpit inbox lists a message **To `mia@example.com`** whose subject/body reference the carried ticket number; the body is the substituted notification text (no literal `%{...}`). (An autoresponse email from AC-1's ticket open is ALSO present, To the same address.)
- Status: [ ]

### AC-5 (US-M2-3): In a fresh client session, the client logs into the portal and DOWNLOADS the agent's attachment. [BROWSER]
- Setup: FRESH browser session (no staff cookie) to keep the client realm separate.
- Navigate: http://localhost:3702/tickets → log in with the carried ticket number + "mia@example.com".
- Verify: the read-only thread shows the original `M` message and the agent `R` reply, each with their attachment chips (`invoice.pdf` on `M`, `policy.txt` on `R`).
- Action: click the `policy.txt` chip.
- Verify: the file downloads (browser receives bytes with a Content-Disposition filename `policy.txt`); the content matches the seeded canned attachment.
- Status: [ ]

## Children (column derived: min of these)

- [ ] EPIC-M2-A — Attachment storage & upload
- [ ] EPIC-M2-B — Authorized attachment download
- [ ] EPIC-M2-C — Variable substitution engine
- [ ] EPIC-M2-D — Canned response consumption
- [ ] EPIC-M2-E — Real outbound email basics
- [ ] TS-M2-prep — Dev reset/purge endpoint (test infra; enables clean E2E sweeps)

## Dependencies

- M1 (DONE) — shared ticket create-and-append core, staff/client realms, reply path, dev mailbox.
- Epic dependency order: **A → B**; **C** standalone; **D** needs **A + C**; **E** needs **C**.
  TS-M2-prep is independent test infra (do first).

## Test Infrastructure

- **Mailpit** container: SMTP **:3704**, web UI **:3705** (staging +10 → 3714/3715). Backend points
  at it via `SMTP_HOST=localhost SMTP_PORT=3704` so D3's SMTP path is exercised; AC-4 reads the
  Mailpit web UI in-browser.
- **TS-M2-prep** dev reset/purge so each E2E sweep starts from a known, empty-tickets baseline.
- Seeded canned responses + seeded `.txt` canned attachment (Epic D seed) are the AC-3 fixture; a
  small `.pdf` test file is the AC-1 fixture.
