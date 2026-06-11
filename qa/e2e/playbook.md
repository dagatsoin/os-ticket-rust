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

## Variable Map

| Variable | Dev value | Notes |
|----------|-----------|-------|
| `{BASE_URL_FRONTEND}` | `http://localhost:3702` | Vite SPA (single app, three router branches: `/` + `/open`, `/tickets/*`, `/staff/*`) |
| `{BASE_URL_API}` | `http://localhost:3701` | Rust/Axum REST API (routes under `/api`) |
| `{STAFF_USER}` | `agent` | Seeded staff account (TS-M1-A3) |
| `{STAFF_PASSWORD}` | `Agent123!` | Seeded staff password |
| `{HEALTH_URL}` | `{BASE_URL_API}/api/health` | FE shell health indicator source |

### Reset / fixture mechanism

- **Full reseed (between journeys only):** `cargo run -p tools --bin seed` — idempotent, writes
  only `osticket_dev`. Restores the seeded department ("Support"), group, and the `agent` staff
  account; clears tickets to a known state.
- **Inject a queue ticket (no UI dependency):** `POST {BASE_URL_API}/api/dev/seed-ticket` →
  `{ticketNumber, email}`. Pass `{"withReply":true}` to obtain a ticket that already carries a
  staff `R` reply. Env-gated dev endpoint (TS-M1-D1).
- **Inspect mail (stub mailer):** `GET {BASE_URL_API}/api/dev/mailbox` — reads what the no-op
  log mailer recorded (FS-040). M1 sends no real mail; use this only to assert *intended* mail.

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

## Regressions

> Per-fix browser regressions appended as FIX-/BUG-/CLEANUP-/DOC- tickets are consolidated.
> Each entry is tagged `REG-{ticket}-{ac}`. None yet.

_(none)_

---

## Appendices

- [API regressions](./appendix-api-regressions.md) — `[API-ONLY]` ACs translated to `curl` flows.
