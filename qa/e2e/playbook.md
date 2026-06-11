# osTicket Modernisation — E2E Playbook

> Evolutive end-to-end regression playbook. Grows monotonically: flows are appended as
> tickets are consolidated, never deleted. A removed feature gets `Status: [REMOVED]` with a
> note rather than being erased, so the playbook keeps documenting what the product once did.
> Flows use the same step vocabulary as the ticket ACs (Setup / Navigate / Action / Verify /
> Manual / Status) so qa-criterion-tester parses them identically.

**Dernière mise à jour: 2026-06-11**

## Changelog

- 2026-06-11 — Initialized playbook (one-time backfill). Added 11 browser flows from M1
  consolidation: M1 root round-trip journey + US-M1-2 / US-M1-3 / US-M1-4 negative/render
  flows + TS-M1-A5 scaffold flows. (M1: 11 flows added across 2 journeys.)
- 2026-06-11 — M2 consolidation. Added Journey 2 — Attachments, canned replies & email —
  13 browser flows from the M2 root E2E (AC-1..AC-5) folded with US-M2-1 / US-M2-2 / US-M2-3 /
  US-M2-4 and TS-M2-A4 / TS-M2-B2 / TS-M2-D3 (open+attach with validation negatives → staff
  chip + canned dropdown + post → Mailpit autoresponse + reply notice → client download +
  cross-session denial). Variable map gained `{BASE_URL_MAILPIT}`, the Mailpit startup, the
  `--reset` seed flag, and the QA fixtures block. `[API-ONLY]` ACs (US-M2-1 AC-5, US-M2-3 AC-4,
  US-M2-4 AC-3/AC-4) routed to appendix-api-regressions.md. (M2: 13 flows added, 1 journey.)

## Variable Map

| Variable | Dev value | Notes |
|----------|-----------|-------|
| `{BASE_URL_FRONTEND}` | `http://localhost:3702` | Vite SPA (single app, three router branches: `/` + `/open`, `/tickets/*`, `/staff/*`) |
| `{BASE_URL_API}` | `http://localhost:3701` | Rust/Axum REST API (routes under `/api`) |
| `{STAFF_USER}` | `agent` | Seeded staff account (TS-M1-A3) |
| `{STAFF_PASSWORD}` | `Agent123!` | Seeded staff password |
| `{HEALTH_URL}` | `{BASE_URL_API}/api/health` | FE shell health indicator source |
| `{BASE_URL_MAILPIT}` | `http://localhost:3705` | Mailpit web UI — in-browser mail oracle for Journey 2 (SMTP transport on `:3704`; staging +10 → web `:3715` / SMTP `:3714`) (M2) |

### Reset / fixture mechanism

- **Full reseed (between journeys only):** `cargo run -p tools --bin seed` — idempotent, writes
  only `osticket_dev`. Restores the seeded department ("Support"), group, and the `agent` staff
  account; clears tickets to a known state.
- **Deterministic purge + reseed (M2 sweeps):** `cargo run -p tools --bin seed -- --reset`
  (TS-M2-prep) — truncates ticket-scoped data (tickets, threads, ticket attachments) and reclaims
  orphaned blobs under `BLOB_ROOT` (defaults to `<workspace root>/var/blobs`), then re-seeds the
  dept/group/`agent` baseline **plus** the M2 seed config keys and the two seeded canned responses
  ("Acknowledge receipt" enabled / "Closed — disabled sample" disabled, with the seeded `policy.txt`
  canned blob). The seeded canned blob is preserved. Used as the ONE-TIME pre-flight before
  Journey 2 (do not reseed mid-journey). Refuses to run against a non-dev DB.
- **Mailpit (M2 outbound-mail oracle):** `docker compose up -d mailpit` brings up the SMTP sink
  (`:3704`) + web UI (`{BASE_URL_MAILPIT}` = `:3705`). Start the backend pointed at it —
  `SMTP_HOST=localhost SMTP_PORT=3704 SMTP_FROM=support@example.com cargo run -p api` — so the D3
  SMTP path is exercised (SMTP is active **only** when `SMTP_HOST` is set; otherwise the M1 stub
  mailer + `GET /api/dev/mailbox` is used). Empty the inbox before a sweep:
  `curl -X DELETE {BASE_URL_MAILPIT}/api/v1/messages`.
- **Inject a queue ticket (no UI dependency):** `POST {BASE_URL_API}/api/dev/seed-ticket` →
  `{ticketNumber, email}`. Pass `{"withReply":true}` to obtain a ticket that already carries a
  staff `R` reply. Env-gated dev endpoint (TS-M1-D1).
- **Inspect mail (stub mailer):** `GET {BASE_URL_API}/api/dev/mailbox` — reads what the no-op
  log mailer recorded (FS-040). M1 sends no real mail; use this only to assert *intended* mail.

### QA fixture files (M2 — create once before a Journey 2 sweep)

Journey 2's upload/attachment flows reference four files under `/tmp/qa-fixtures/`. Create them
once (idempotent) before the sweep — copied here verbatim from US-M2-1 / US-M2-2 Test Infrastructure
so the playbook is self-contained:

```sh
mkdir -p /tmp/qa-fixtures
# valid PDF, < 1 MB — the client's "invoice" upload (M2 AC-1)
printf '%%PDF-1.4\n1 0 obj<</Type/Catalog>>endobj\ntrailer<</Root 1 0 R>>\n%%%%EOF\n' > /tmp/qa-fixtures/invoice.pdf
# disallowed extension — rejected-type negative (US-M2-1 AC-3)
printf 'MZ\220\000\003' > /tmp/qa-fixtures/evil.exe
# permitted extension but > 1 MB — oversized negative (US-M2-1 AC-4)
{ printf '%%PDF-1.4\n'; head -c 1048577 /dev/zero | tr '\0' A; } > /tmp/qa-fixtures/big.pdf
# valid PNG — the agent's own-file reply attachment (US-M2-2 AC-5)
printf '\211PNG\r\n\032\n\000\000\000\rIHDR\000\000\000\001\000\000\000\001\010\006\000\000\000\037\025\304\211' > /tmp/qa-fixtures/note.png
```

| Fixture | Role |
|---------|------|
| `/tmp/qa-fixtures/invoice.pdf` | valid, < 1 MB — client's `M` attachment (Flow 2.4) |
| `/tmp/qa-fixtures/evil.exe` | disallowed extension — Flow 2.2 negative |
| `/tmp/qa-fixtures/big.pdf` | permitted type, > 1 MB — Flow 2.3 oversized negative |
| `/tmp/qa-fixtures/note.png` | valid PNG — agent's own-file reply attachment (Flow 2.9) |

Note: `policy.txt` (the canned-response attachment that flows through Journey 2) is NOT a `/tmp`
fixture — it is the seeded canned blob produced by `seed -- --reset`, so no manual creation is needed.

### Realm cookie discipline

- Staff realm uses the `ost_staff_sess` cookie; client realm uses a separate ticket-scoped
  session cookie. **Never share a browser session across realms** — open a fresh session when a
  flow crosses from staff to client (and vice-versa). The CSRF token is the realm XSRF cookie,
  injected as `X-CSRFToken` by the shared `apiClient` on mutating requests.

---

## Pre-flight (run once before each journey)

- Verify services are up: backend on `:3701` (`GET {HEALTH_URL}` → ok), frontend on `:3702`.
- Reseed to a known DB state: `cargo run -p tools --bin seed`.
- Use a clean browser session (no carried-over realm cookies) unless a flow explicitly says to
  reuse the session from the previous step in the same journey.

---

## Journey 0 — Frontend scaffold & routing (M1 foundation)

> Smoke checks that the SPA shell and router are alive before any functional journey. These do
> not depend on backend data (API may be mocked/unavailable). Reseed not required for this
> journey; only the dev server (`npm run dev`, port 3702) must be running.

### Flow 0.1 — App shell renders at the frontend root (TS-M1-A5 AC-2)

- Setup: start the dev server — `npm run dev` (port 3702); the API may be mocked or unavailable for this check.
- Navigate: {BASE_URL_FRONTEND}/
- Verify: the React app shell renders (MUI layout visible) — not a blank page, a build-error overlay, or a connection-refused screen.
- Status: [x]

### Flow 0.2 — Router resolves all three branches to placeholder views (TS-M1-A5 AC-3)

- Navigate: {BASE_URL_FRONTEND}/
- Verify: the public branch resolves to its placeholder/landing view (no router "no match" / blank screen).
- Navigate: {BASE_URL_FRONTEND}/staff/login
- Verify: the staff branch resolves to its placeholder login view.
- Navigate: {BASE_URL_FRONTEND}/tickets
- Verify: the client-portal branch resolves to its placeholder login view.
- Status: [x]

---

## Journey 1 — Ticket round-trip (M1)

> The irreducible helpdesk loop: a visitor opens a ticket, an agent picks it up and replies, the
> customer reads the reply. This is the M1 root E2E (M1 AC-1..AC-5) plus the per-US render and
> negative-path flows folded in at the point they belong. **The journey is the test:** state
> created in Flow 1.3 (the ticket number + email) carries forward through Flows 1.6–1.9. Reseed
> is the ONLY pre-flight; do NOT reseed between flows in this journey.
>
> Traceability: Flows 1.3 / 1.5 / 1.6 / 1.7 / 1.9 each cover a US AC and the matching M1 root
> step — both ticket IDs are cited; the flow is kept once (root-journey canonical), not duplicated.

### Flow 1.1 — Open-a-ticket page renders with the four required fields (US-M1-2 AC-1)

- Setup: reseed dev DB to a known state — `cargo run -p tools --bin seed`. Use a clean browser session.
- Navigate: {BASE_URL_FRONTEND}/open
- Verify: the page renders an "Open a New Ticket" heading and four fields — Name, Email, Subject, Message (no help-topic / CAPTCHA / attachment fields in M1).
- Navigate: {BASE_URL_FRONTEND}/
- Verify: the public landing exposes a link/route to /open.
- Status: [x]

### Flow 1.2 — Invalid submission is blocked with an inline validation error (US-M1-2 AC-2)

- Navigate: {BASE_URL_FRONTEND}/open
- Action: leave Email empty (or type an invalid email like "not-an-email"); fill Name "Jane Doe", Subject "Printer broken", Message "It won't print.".
- Action: attempt to submit.
- Verify: an inline validation error appears on the offending field and the submit is blocked (422 mapped to the field); no confirmation page is shown.
- Verify (persistence): no new ticket was created — the invalid attempt produced nothing in the queue (asserted later in Flow 1.6, where the queue shows only the valid Flow 1.3 ticket).
- Status: [x]

### Flow 1.3 — Visitor submits a valid ticket and sees a ticket number (US-M1-2 AC-3 / M1 AC-1)

- Navigate: {BASE_URL_FRONTEND}/open
- Action: fill Name "Jane Doe", Email "jane@example.com", Subject "Printer broken", Message "My printer won't print.".
- Action: submit the form.
- Verify: a confirmation page renders a generated 6-digit ticket number (100000–999999). CARRY {ticketNumber, email="jane@example.com"} forward to Flows 1.6–1.9 — do NOT reseed.
- Status: [x]

### Flow 1.4 — Staff login rejects wrong credentials (US-M1-3 AC-1, negative path)

- Navigate: {BASE_URL_FRONTEND}/staff/login (staff realm uses the distinct ost_staff_sess cookie).
- Action: type Username "agent", Password "wrongpass", submit.
- Verify: login is rejected with an error message; the URL stays on /staff/login and no staff session is established.
- Status: [x]

### Flow 1.5 — Agent logs in with seeded credentials (US-M1-3 AC-1 / M1 AC-2)

- Navigate: {BASE_URL_FRONTEND}/staff/login (same browser session as Flow 1.4).
- Action: type Username "agent", Password "Agent123!", submit.
- Verify: the agent lands on the staff control panel (e.g. /staff/tickets) showing the agent is logged in.
- Status: [x]

### Flow 1.6 — Agent sees the just-created ticket in the Open queue (US-M1-3 AC-2 / M1 AC-3)

- Navigate: {BASE_URL_FRONTEND}/staff/tickets
- Verify: the Open-tickets queue lists the Flow 1.3 ticket — its carried-over 6-digit number and subject "Printer broken" are visible, newest first (created DESC).
- Verify: no spurious ticket from the blocked Flow 1.2 attempt appears.
- Status: [x]

### Flow 1.7 — Agent opens the ticket, reads the message, posts a reply (US-M1-3 AC-3 + AC-4 / M1 AC-4)

- Action: click the Flow 1.3 ticket row in the queue.
- Verify: the detail thread shows the customer's original `M` message "My printer won't print." (chronological, oldest first).
- Action: type "We are looking into your printer issue." in the reply box and post it.
- Verify: the thread refetches and the new agent `R` response appears below the customer message. The ticket stays Open.
- Status: [x]

### Flow 1.8 — Client login rejects a wrong email for the ticket number (US-M1-4 AC-1, negative path)

- Setup: open a FRESH browser session (no staff cookie) to keep the client realm separate.
- Navigate: {BASE_URL_FRONTEND}/tickets (client portal login).
- Action: enter the Flow 1.3 ticket number with a WRONG email ("wrong@example.com"), submit.
- Verify: login is rejected with a sensible error; no client session is established.
- Status: [x]

### Flow 1.9 — Client logs in and views the original message + agent reply (US-M1-4 AC-1 + AC-2 / M1 AC-5)

- Navigate: {BASE_URL_FRONTEND}/tickets (same fresh client session as Flow 1.8).
- Action: enter the Flow 1.3 ticket number with the correct email "jane@example.com", submit.
- Verify: login succeeds and the client lands on the read-only ticket-thread view.
- Verify: the thread renders the customer's original `M` message followed by the agent's `R` reply "We are looking into your printer issue." in chronological order; no reply box (read-only in M1) and no internal note (`N`) is shown.
- Status: [x]

---

## Journey 2 — Attachments, canned replies & email (M2)

> The M2 demo path: a visitor opens a ticket WITH an attachment (and we prove wrong-type / oversized
> files are rejected first), an agent picks it up, sees the chip, answers with a canned response that
> fills the box with a substituted body + carries its own file, real email lands in the customer's
> Mailpit inbox (autoresponse + reply notice), and the customer downloads the agent's attachment from
> the portal — while a different customer is denied. This is the M2 root E2E (M2 AC-1..AC-5) with the
> per-US render / negative-path / security flows folded in at the point they belong.
>
> **The journey is the test:** the ticket created in Flow 2.4 (number + email + `invoice.pdf` on `M`)
> carries forward through Flows 2.5–2.13 — the agent's canned `R` reply (`policy.txt`) and own-file
> reply (`note.png`) accumulate on it, the same ticket's mail appears in Mailpit, and the same ticket
> is downloaded from the client portal. **Reset is the ONLY pre-flight; do NOT reseed between flows.**
>
> Pre-flight (ONCE before Flow 2.1): `docker compose up -d mailpit`; create the QA fixtures (see
> "QA fixture files" in the Variable Map); `cargo run -p tools --bin seed -- --reset`; start the
> backend with `SMTP_HOST=localhost SMTP_PORT=3704 SMTP_FROM=support@example.com`; empty Mailpit
> (`curl -X DELETE {BASE_URL_MAILPIT}/api/v1/messages`); clean browser session.
>
> Traceability: every flow is tagged with its originating ticket IDs. Where a US AC and the M2 root
> step (or a child TS AC) cover the same observable, the flow is kept ONCE and cites all the ids;
> it is not duplicated. `[API-ONLY]` ACs (US-M2-1 AC-5, US-M2-3 AC-4, US-M2-4 AC-3/AC-4, plus
> TS-M2-prep) live in appendix-api-regressions.md, not here.

### Flow 2.1 — The /open form shows a file input with helper text when attachments are enabled (US-M2-1 AC-1 / TS-M2-A4 AC-1)

- Setup: ONE-TIME pre-flight only — `docker compose up -d mailpit`; QA fixtures created; `cargo run -p tools --bin seed -- --reset`; backend started with `SMTP_HOST=localhost SMTP_PORT=3704 SMTP_FROM=support@example.com`; Mailpit emptied; clean browser session.
- Navigate: {BASE_URL_FRONTEND}/open
- Verify: alongside Name / Email / Subject / Message there is an attachment file input with helper text naming the allowed types (`.pdf,.png,.jpg,.txt,.doc`) and the 1 MB max.
- Status: [ ]

### Flow 2.2 — A disallowed file type is blocked with an inline error and creates no ticket (US-M2-1 AC-3 / TS-M2-A4 AC-3)

- Navigate: {BASE_URL_FRONTEND}/open
- Action: fill valid Name "Mia Wong", Email "mia@example.com", Subject "Invoice query", Message "See the attached invoice."; in the attachment file input choose `/tmp/qa-fixtures/evil.exe` (disallowed extension).
- Action: attempt to submit.
- Verify: an inline "invalid file type" error renders under the file input and submission is blocked; no confirmation page is shown.
- Verify (persistence): no ticket was created — asserted in Flow 2.5, where the Open queue shows only the valid Flow 2.4 ticket.
- Status: [ ]

### Flow 2.3 — An oversized file (> 1 MB) is blocked with an inline error and creates no ticket (US-M2-1 AC-4 / TS-M2-A4 AC-4)

- Navigate: {BASE_URL_FRONTEND}/open
- Action: fill valid Name / Email / Subject / Message; in the attachment file input choose `/tmp/qa-fixtures/big.pdf` (permitted extension but > 1 MB).
- Action: attempt to submit.
- Verify: an inline "too big" error renders under the file input and submission is blocked; no confirmation page is shown.
- Verify (persistence): no ticket was created — confirmed against the Open queue in Flow 2.5.
- Status: [ ]

### Flow 2.4 — Visitor submits a valid ticket WITH a `.pdf` and sees a ticket number + attachment chip (US-M2-1 AC-2 / TS-M2-A4 AC-2 / M2 AC-1)

- Navigate: {BASE_URL_FRONTEND}/open
- Action: fill Name "Mia Wong", Email "mia@example.com", Subject "Invoice query", Message "See the attached invoice."; in the attachment file input choose `/tmp/qa-fixtures/invoice.pdf` (< 1 MB).
- Action: submit the form.
- Verify: the confirmation page renders a generated 6-digit ticket number (100000–999999) AND an AttachmentChip labelled `invoice.pdf`. CARRY {ticketNumber, email="mia@example.com", attachmentName="invoice.pdf"} forward to Flows 2.5–2.13 — do NOT reseed.
- Status: [ ]

### Flow 2.5 — Agent logs in and sees the client's attachment chip on the original message (US-M2-2 AC-1 / M2 AC-2)

- Navigate: {BASE_URL_FRONTEND}/staff/login (staff realm; distinct `ost_staff_sess` cookie). Action: log in `agent` / `Agent123!`.
- Navigate: {BASE_URL_FRONTEND}/staff/tickets → open the carried Flow 2.4 ticket from the Open queue.
- Verify: the Open queue lists ONLY the Flow 2.4 ticket (subject "Invoice query") — no spurious ticket from the blocked Flow 2.2 / Flow 2.3 attempts.
- Verify: the detail thread shows the customer's `M` message "See the attached invoice." with a clickable attachment chip `invoice.pdf` attached to it.
- Status: [ ]

### Flow 2.6 — The reply composer exposes a canned-response dropdown (enabled + dept-scoped only) and an own-file input (US-M2-2 AC-2 / TS-M2-D3 AC-1 + AC-4)

- Navigate: on the staff detail view from Flow 2.5 (same staff session), scroll to the reply composer.
- Verify: the reply area shows a "Canned response" dropdown that includes the seeded "Acknowledge receipt" (enabled, All-Departments) and does NOT include the seeded "Closed — disabled sample" (disabled).
- Verify: an own-file `<input type=file>` is present in the reply composer.
- Status: [ ]

### Flow 2.7 — Selecting a canned response fills the reply with a substituted body and shows its attachment chip (US-M2-2 AC-3 / TS-M2-D3 AC-2 + AC-3 / M2 AC-3)

- Action: pick "Acknowledge receipt" from the "Canned response" dropdown.
- Verify: the reply textarea is populated with the canned body and the `%{ticket.number}` token is already substituted to the carried ticket number (no literal `%{...}` remains).
- Verify: a read-only AttachmentChip `policy.txt` (marked "from canned response") appears in the reply composer.
- Status: [ ]

### Flow 2.8 — Posting the canned reply appends a substituted `R` response with its attachment and flips the badge to Answered (US-M2-2 AC-4 / M2 AC-3)

- Setup: before replying, the ticket's badge reads Unanswered.
- Action: post the reply.
- Verify: the thread refetches and shows a new agent `R` entry with the substituted body and a `policy.txt` chip below the customer's `M` message.
- Verify: the ticket's Answered / Unanswered badge now reads "Answered" (`isanswered=true`), like any staff reply (ROADMAP M2 Decisions §3; BS-022.15's SYSTEM "mark unanswered" path is deferred to M5).
- Status: [ ]

### Flow 2.9 — The agent additionally attaches their own file on a reply (US-M2-2 AC-5)

- Action: in the reply box own-file input, choose `/tmp/qa-fixtures/note.png` (permitted type); type some reply text; post.
- Verify: a new agent `R` entry shows a chip for `note.png` — the own-file attachment path works alongside / independent of the canned attachment.
- Status: [ ]

### Flow 2.10 — Mailpit shows the autoresponse (ticket open) and the reply-notification email, both addressed to the requester (US-M2-4 AC-1 + AC-2 / M2 AC-4)

- Navigate: {BASE_URL_MAILPIT} (Mailpit web UI) in the browser.
- Verify: the inbox lists an autoresponse message To `mia@example.com` (from the Flow 2.4 ticket open) whose subject/body reference the carried ticket number, with no literal `%{...}` in the rendered body.
- Verify: the inbox ALSO lists a NEW message To `mia@example.com` (from the Flow 2.8 staff reply) whose subject/body reflect the reply/ticket details — substituted, no literal tokens.
- Status: [ ]

### Flow 2.11 — The agent can download an attachment from the staff detail view (US-M2-3 AC-2 / TS-M2-B2 AC-2)

- Navigate: back to the staff detail view (same staff session as Flow 2.5–2.9); open the carried ticket.
- Action: click an attachment chip in the thread (e.g. `invoice.pdf` on `M` or `policy.txt` on `R`).
- Verify: the file downloads for the authorized staff session (object-URL download).
- Status: [ ]

### Flow 2.12 — In a fresh client session, the client logs in and downloads the agent's attachment (US-M2-3 AC-1 / TS-M2-B2 AC-1 / M2 AC-5)

- Setup: open a FRESH browser session (no staff cookie) to keep the client realm separate.
- Navigate: {BASE_URL_FRONTEND}/tickets → log in with the carried Flow 2.4 ticket number + "mia@example.com".
- Verify: the read-only thread shows the `M` message and the agent `R` reply, each with their clickable attachment chips (`invoice.pdf` on `M`, `policy.txt` (and `note.png`) on `R`).
- Action: click the `policy.txt` chip.
- Verify: the browser downloads the file (object-URL download; `Content-Disposition` filename `policy.txt`); the content matches the seeded canned attachment.
- Status: [ ]

### Flow 2.13 — A client logged into a DIFFERENT ticket cannot download this ticket's attachment (US-M2-3 AC-3 / TS-M2-B2 AC-3)

- Setup: open a SECOND ticket via {BASE_URL_FRONTEND}/open with a DIFFERENT email (e.g. "ben@example.com", any valid `.pdf`); in a FRESH client session log into the portal for that second ticket ONLY.
- Action: drive a download of the FIRST (carried) ticket's attachment id via the session-bound client route (`GET {BASE_URL_API}/api/client/ticket/attachments/{attachmentId}`, no ticketId param) in this second session.
- Verify: the request is denied (404, no existence leak — §8), a visible inline error appears near the chip, and no bytes are served (EC-022.9 / §9).
- Status: [ ]

---

## Regressions

> Per-fix browser regressions appended as FIX-/BUG-/CLEANUP-/DOC- tickets are consolidated.
> Each entry is tagged `REG-{ticket}-{ac}`. None yet.

_(none)_

---

## Appendices

- [API regressions](./appendix-api-regressions.md) — `[API-ONLY]` ACs translated to `curl` flows.
