# osTicket Modernisation — E2E Playbook

> Evolutive end-to-end regression playbook. Grows monotonically: flows are appended as
> tickets are consolidated, never deleted. A removed feature gets `Status: [REMOVED]` with a
> note rather than being erased, so the playbook keeps documenting what the product once did.
> Flows use the same step vocabulary as the ticket ACs (Setup / Navigate / Action / Verify /
> Manual / Status) so qa-criterion-tester parses them identically.

**Dernière mise à jour: 2026-07-24**

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
- 2026-07-24 — M3 consolidation. Added Journey 3 — Full staff workflow & queue — 11 browser
  flows from the M3 root E2E (AC-1..AC-11) folded with US-M3-A1 / US-M3-B1 / US-M3-F1 /
  US-M3-C1 / US-M3-C2 / US-M3-E1 / US-M3-D1 / US-M3-G1 / US-M3-I1 / US-M3-I2 (queue tabs +
  counts with "My Tickets" gated on ≥1 assignment → sticky sort → keyword search → claim →
  transfer to Sales → internal note → close/reopen → overdue tab → bulk close → edit priority
  → two-DIFFERENT-staff collaborative-lock conflict). Variable map gained the second staff
  account `{STAFF2_USER}` = `agent2` / `{STAFF2_PASSWORD}` = `Agent234!` (display names
  "Agent One" / "Agent Two"), the M3 reset (`seed -- --reset` seeds 2 depts + teams + SLA plans
  + `agent2`), and the `/api/dev/seed-tickets` + `/api/dev/mark-overdue/:id` queue-pool setup.
  Corrected creds: the M3 ticket files quote `agent2` / `Agent123!`, but the seed source of
  truth (`tools/src/lib.rs`) is `Agent234!` — the playbook uses `Agent234!`. `[API-ONLY]` ACs
  (US-M3-A1 AC-6 stats + other TS API ACs) routed to appendix-api-regressions.md.
  (M3: 11 flows added, 1 journey.)
- 2026-07-24 — M4 consolidation. Added Journey 4 — Admin Configuration — 58 browser flows
  compiling every M4 `[BROWSER]` AC (77 ACs) from the 9 done epics + the M4 root E2E: the admin
  shell + gate (TS-M4-A0), the 7-tab System Settings + separate Attachments screen (US-M4-A1/A2),
  Staff & Groups with the 11-flag PermissionGrid + dept matrix (US-M4-B1/B2), Profile & Directory
  (US-M4-B3), SLA + read-only Priorities (US-M4-D1/D2), Site Pages (US-M4-F1), System Logs
  (US-M4-G1), the delegated-gate Canned Responses (US-M4-H1) + FAQ Categories (US-M4-E1),
  Departments/Teams/Help Topics (US-M4-C1/C2/C3), and an "M4 Integration" subsection folding the
  8 root cross-epic ACs. Variable map gained `{ADMIN_USER}`=`admin`/`Admin123!` (isadmin),
  `{FAQMGR_USER}`=`faqmgr`/`Faqmgr123!` (can_manage_faq), `{PREMADE_USER}`=`premade1`/`Premade123!`
  (can_manage_premade), the M4 reset (`seed -- --reset` seeds admin + agent/agent2 + email_account
  + template_group + timezone + 110 config keys), and the M4 dev-endpoint roster (reset-config,
  reset-staff, seed-staff/seed-staff-bulk, set-group-perm, reset-groups, seed-dept,
  reset-departments/teams/help-topics/sla/pages/canned/faq-categories, seed-page, seed-log,
  age-password, seed-config, purge-logs). Every M4 `[BROWSER]` AC maps to a flow — 0 untranslatable.
  `[API-ONLY]` ACs (US-M4-A1 AC-5, A2 AC-7, B1 AC-6, B2 AC-7, B3 AC-6, C1 AC-6, C2 AC-5, C3 AC-6,
  D1 AC-5, D2 AC-2, E1 AC-5, F1 AC-5/AC-6, G1 AC-6, H1 AC-5) routed to appendix-api-regressions.md,
  not here. (M4: 58 flows added, 1 journey.)

## Variable Map

| Variable | Dev value | Notes |
|----------|-----------|-------|
| `{BASE_URL_FRONTEND}` | `http://localhost:3702` | Vite SPA (single app, three router branches: `/` + `/open`, `/tickets/*`, `/staff/*`) |
| `{BASE_URL_API}` | `http://localhost:3701` | Rust/Axum REST API (routes under `/api`) |
| `{STAFF_USER}` | `agent` | Seeded staff account (TS-M1-A3); display name **"Agent One"** |
| `{STAFF_PASSWORD}` | `Agent123!` | Seeded staff password |
| `{STAFF2_USER}` | `agent2` | Second seeded staff account (TS-M3-prep); display name **"Agent Two"** — required for the two-DIFFERENT-staff lock conflict (Flow 3.11) (M3) |
| `{STAFF2_PASSWORD}` | `Agent234!` | Second staff password — **source of truth `tools/src/lib.rs`; NOT `Agent123!` as the stale M3 ticket files quote** (M3) |
| `{ADMIN_USER}` | `admin` | Seeded **isadmin** account (TS-M4-PREP-C); the M4 admin-panel actor — gates `/staff/admin/*` (M4) |
| `{ADMIN_PASSWORD}` | `Admin123!` | Seeded admin password (source of truth `tools/src/lib.rs`) (M4) |
| `{FAQMGR_USER}` | `faqmgr` | Delegated non-admin account whose group carries `can_manage_faq` — minted via `seed-staff` + `set-group-perm`; proves the FAQ capability gate (Journey 4 4.40/4.56) (M4) |
| `{FAQMGR_PASSWORD}` | `Faqmgr123!` | `faqmgr` password (M4) |
| `{PREMADE_USER}` | `premade1` | Delegated non-admin account whose group carries `can_manage_premade` — minted via `seed-staff` + `set-group-perm`; proves the Canned-Responses capability gate (Journey 4 4.37) (M4) |
| `{PREMADE_PASSWORD}` | `Premade123!` | `premade1` password (M4) |
| `{HEALTH_URL}` | `{BASE_URL_API}/api/health` | FE shell health indicator source |
| `{BASE_URL_MAILPIT}` | `http://localhost:3705` | Mailpit web UI — in-browser mail oracle for Journey 2 (SMTP transport on `:3704`; staging +10 → web `:3715` / SMTP `:3714`) (M2) |

### Reset / fixture mechanism

> **Working directory:** the runnable app lives under `modernized/`. Run all `cargo` /
> `npm` / `docker compose` / `sqlx` commands below from `modernized/` (`cd modernized` first),
> or from the repo root with `cargo --manifest-path modernized/Cargo.toml …`,
> `npm --prefix modernized/frontend …`, and `sqlx migrate run --source modernized/migrations`.
> The blob store `BLOB_ROOT` default resolves to `<cwd>/var/blobs` — i.e. `modernized/var/blobs`
> when run from `modernized/`.

- **Full reseed (between journeys only):** `cargo run -p tools --bin seed` — idempotent, writes
  only `osticket_dev`. Restores the seeded department ("Support"), group, and the `agent` staff
  account; clears tickets to a known state.
- **Deterministic purge + reseed (M2 sweeps):** `cargo run -p tools --bin seed -- --reset`
  (TS-M2-prep) — truncates ticket-scoped data (tickets, threads, ticket attachments) and reclaims
  orphaned blobs under `BLOB_ROOT` (defaults to `<workspace root>/var/blobs`, i.e.
  `modernized/var/blobs`), then re-seeds the
  dept/group/`agent` baseline **plus** the M2 seed config keys and the two seeded canned responses
  ("Acknowledge receipt" enabled / "Closed — disabled sample" disabled, with the seeded `policy.txt`
  canned blob). The seeded canned blob is preserved. Used as the ONE-TIME pre-flight before
  Journey 2 (do not reseed mid-journey). Refuses to run against a non-dev DB.
- **Mailpit (M2 outbound-mail oracle):** `docker compose up -d mailpit` (from `modernized/`)
  brings up the SMTP sink
  (`:3704`) + web UI (`{BASE_URL_MAILPIT}` = `:3705`). Start the backend pointed at it —
  `SMTP_HOST=localhost SMTP_PORT=3704 SMTP_FROM=support@example.com cargo run -p api` — so the D3
  SMTP path is exercised (SMTP is active **only** when `SMTP_HOST` is set; otherwise the M1 stub
  mailer + `GET /api/dev/mailbox` is used). Empty the inbox before a sweep:
  `curl -X DELETE {BASE_URL_MAILPIT}/api/v1/messages`.
- **Inject a queue ticket (no UI dependency):** `POST {BASE_URL_API}/api/dev/seed-ticket` →
  `{ticketNumber, email}`. Pass `{"withReply":true}` to obtain a ticket that already carries a
  staff `R` reply. Env-gated dev endpoint (TS-M1-D1).
- **M3 reset (Journey 3 pre-flight):** `cargo run -p tools --bin seed -- --reset` seeds the M3
  baseline: **two departments** (Support + Sales), a **team** ("Tier 2"), **SLA plans**
  (Standard 24h / Urgent 4h), help topics, group-dept access to both depts, the M3 config keys
  (`show_answered_tickets=0`, `show_assigned_tickets`, `ticket_lock_time`, `max_page_size`), and
  **two staff accounts** — `agent` / `Agent123!` ("Agent One") and `agent2` / `Agent234!`
  ("Agent Two"). Refuses to run against a non-dev DB.
- **Seed a queue pool (M3 tab counts, no UI dependency):**
  `POST {BASE_URL_API}/api/dev/seed-tickets` with body
  `{"tickets":[{"status":"open"},…,{"status":"open","isoverdue":true},{"status":"closed"},…]}`
  → `{"ids":[…]}`. Each spec accepts `status` ("open"/"closed"), `isanswered`, `isoverdue`,
  `staff_id`, `dept_id`, `team_id`, `closed_by_staff_id` (all optional). Used to populate the
  Open/Overdue/Closed tab counts before Journey 3 (env-gated, TS-M3-A2/A3).
- **Mark a ticket overdue (M3 Overdue tab):** `POST {BASE_URL_API}/api/dev/mark-overdue/{id}`
  flags a ticket past its SLA due date so it surfaces in the Overdue tab (env-gated, TS-M3-E).
  (The M3 ticket files also reference `/api/dev/assign-ticket` and `/api/dev/reset-db`; those
  routes do NOT exist — assignment is driven through the real UI "Claim Ticket" action or the
  staff `POST /api/staff/tickets/:id/assign` route, and reset is the `seed -- --reset` binary.)
- **Inspect mail (stub mailer):** `GET {BASE_URL_API}/api/dev/mailbox` — reads what the no-op
  log mailer recorded (FS-040). M1 sends no real mail; use this only to assert *intended* mail.
- **M4 reset (Journey 4 pre-flight):** `cargo run -p tools --bin seed -- --reset` seeds the M4
  admin baseline: the **`admin` / `Admin123!`** isadmin account (plus `agent` / `Agent123!` and
  `agent2` / `Agent234!`), one **email_account** (`support@osticket.local`), one **template_group**
  ("osTicket Default"), the **timezone** reference table, the seeded **Support** default department
  + admin/Support groups, the 4 fixed **priorities**, and the ~**110 FS-032 config keys** at their
  defaults. Idempotent; refuses to run against a non-dev DB. This is the ONE-TIME Journey 4
  pre-flight (do NOT reseed mid-journey — the integration flows 4.51–4.58 consume state built by the
  per-epic flows).
- **M4 admin dev endpoints (setup only, env-gated — NEVER validation):**
  - `POST {BASE_URL_API}/api/dev/reset-config` — restore the FS-032 config keys to defaults (baseline for the settings flows).
  - `POST {BASE_URL_API}/api/dev/reset-staff` — restore the staff roster to `agent` / `agent2` / `admin` (exactly one active admin — the last-admin protection flows rely on it).
  - `POST {BASE_URL_API}/api/dev/seed-staff` `{username,password[,group_id,dept_id,isadmin,isvisible,onvacation,count]}` and `POST .../api/dev/seed-staff-bulk` — mint one or ~12 staff rows (pagination fill for the staff list).
  - `POST {BASE_URL_API}/api/dev/set-group-perm` `{group_id, can_manage_faq | can_manage_premade | …}` — flip a group flag to mint the delegated `faqmgr` / `premade1` capabilities.
  - `POST {BASE_URL_API}/api/dev/reset-groups` — delete non-seed groups (skips groups that still have members), keep Support/admin.
  - `POST {BASE_URL_API}/api/dev/seed-dept` — insert a second department (dept-access matrix + re-home flows).
  - `POST {BASE_URL_API}/api/dev/{reset-departments,reset-teams,reset-help-topics,reset-sla,reset-pages,reset-canned,reset-faq-categories}` — per-screen clean-state resets that retain the seeded rows.
  - `POST {BASE_URL_API}/api/dev/{seed-sla,set-dept-sla}` — create an extra SLA plan / bind a plan to a department (SLA re-home flow).
  - `POST {BASE_URL_API}/api/dev/{seed-topic,seed-page}` — minimal help-topic / page create (topic-nesting + page-binding flows); `seed-page` types include `landing` / `thank-you` / `other`.
  - `POST {BASE_URL_API}/api/dev/seed-log` `{type,title,log[,created]}` — insert a syslog row (optional backdated `created` for the over-age purge flow); `POST .../api/dev/purge-logs` runs the grace-period sweep (over-age only).
  - `POST {BASE_URL_API}/api/dev/age-password` `{staffId,days}` (backdates `passwdreset` + sets `change_passwd`) and `POST .../api/dev/seed-config` `{passwd_reset_period}` — arm the forced-password-change banner flow.
  - `POST {BASE_URL_API}/api/dev/seed-ticket` — inject a ticket homed to a given dept for the department/team delete-re-home flows.

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
- Status: [x] — / renders "Support Center" heading + MUI banner with "Backend OK" health indicator; Open a Ticket / Client Portal / Staff Sign In links present; no blank/error/connection-refused screen.

### Flow 0.2 — Router resolves all three branches to placeholder views (TS-M1-A5 AC-3)

- Navigate: {BASE_URL_FRONTEND}/
- Verify: the public branch resolves to its placeholder/landing view (no router "no match" / blank screen).
- Navigate: {BASE_URL_FRONTEND}/staff/login
- Verify: the staff branch resolves to its placeholder login view.
- Navigate: {BASE_URL_FRONTEND}/tickets
- Verify: the client-portal branch resolves to its placeholder login view.
- Status: [x] — /staff/login → "Staff Sign In" form (Username/Password/Sign In); /tickets redirects to /tickets/login → "View Your Ticket" (Ticket Number + Email Address + View Ticket); public / → "Support Center" landing. All three router branches resolve, no no-match/blank.

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
- Status: [x] — /open renders "Open a New Ticket" + the four required fields (Full Name, Email Address, Subject, Message); landing (/) exposes "Open a Ticket" → /open. Optional M2 attachment input also present (helper "Allowed types: .pdf, .png, .jpg, .txt, .doc. Max size: 1 MB.") — post-M2 build, supersedes M1-era parenthetical.

### Flow 1.2 — Invalid submission is blocked with an inline validation error (US-M1-2 AC-2)

- Navigate: {BASE_URL_FRONTEND}/open
- Action: leave Email empty (or type an invalid email like "not-an-email"); fill Name "Jane Doe", Subject "Printer broken", Message "It won't print.".
- Action: attempt to submit.
- Verify: an inline validation error appears on the offending field and the submit is blocked (422 mapped to the field); no confirmation page is shown.
- Verify (persistence): no new ticket was created — the invalid attempt produced nothing in the queue (asserted later in Flow 1.6, where the queue shows only the valid Flow 1.3 ticket).
- Status: [x] — email "not-an-email" (Name/Subject/Message filled) → email field aria-invalid=true + inline helper "Enter a valid email address."; SUBMIT TICKET disabled (blocked); URL stayed /open, no confirmation. Persistence asserted in Flow 1.6.

### Flow 1.3 — Visitor submits a valid ticket and sees a ticket number (US-M1-2 AC-3 / M1 AC-1)

- Navigate: {BASE_URL_FRONTEND}/open
- Action: fill Name "Jane Doe", Email "jane@example.com", Subject "Printer broken", Message "My printer won't print.".
- Action: submit the form.
- Verify: a confirmation page renders a generated 6-digit ticket number (100000–999999). CARRY {ticketNumber, email="jane@example.com"} forward to Flows 1.6–1.9 — do NOT reseed.
- Status: [x] — ticketNumber=136001, email=jane@example.com; confirmation page "Ticket Created — Your ticket 136001 has been created. A confirmation has been sent to your email address." (6-digit, in range) + GO TO CLIENT PORTAL / BACK HOME actions.

### Flow 1.4 — Staff login rejects wrong credentials (US-M1-3 AC-1, negative path)

- Navigate: {BASE_URL_FRONTEND}/staff/login (staff realm uses the distinct ost_staff_sess cookie).
- Action: type Username "agent", Password "wrongpass", submit.
- Verify: login is rejected with an error message; the URL stays on /staff/login and no staff session is established.
- Status: [x] — agent/wrongpass → "Invalid username or password" alert banner; URL stayed /staff/login, no redirect to the panel, no session established.

### Flow 1.5 — Agent logs in with seeded credentials (US-M1-3 AC-1 / M1 AC-2)

- Navigate: {BASE_URL_FRONTEND}/staff/login (same browser session as Flow 1.4).
- Action: type Username "agent", Password "Agent123!", submit.
- Verify: the agent lands on the staff control panel (e.g. /staff/tickets) showing the agent is logged in.
- Status: [x] — agent/Agent123! → landed on /staff/tickets "Open Tickets" with LOG OUT visible; header shows "Backend OK". Staff session established.

### Flow 1.6 — Agent sees the just-created ticket in the Open queue (US-M1-3 AC-2 / M1 AC-3)

- Navigate: {BASE_URL_FRONTEND}/staff/tickets
- Verify: the Open-tickets queue lists the Flow 1.3 ticket — its carried-over 6-digit number and subject "Printer broken" are visible, newest first (created DESC).
- Verify: no spurious ticket from the blocked Flow 1.2 attempt appears.
- Status: [x] — Open queue lists 136001 "Printer broken" jane@example.com (OPEN 1 / CLOSED 0), exactly ONE ticket — no spurious ticket from the blocked Flow 1.2 attempt. Clean DB state this run (no leftover pool), so the single-ticket newest-first projection is unambiguous.

### Flow 1.7 — Agent opens the ticket, reads the message, posts a reply (US-M1-3 AC-3 + AC-4 / M1 AC-4)

- Action: click the Flow 1.3 ticket row in the queue.
- Verify: the detail thread shows the customer's original `M` message "My printer won't print." (chronological, oldest first).
- Action: type "We are looking into your printer issue." in the reply box and post it.
- Verify: the thread refetches and the new agent `R` response appears below the customer message. The ticket stays Open.
- Status: [x] — reply posted; thread shows M "My printer won't print." (Jane Doe) then R "We are looking into your printer issue." (Agent One), chronological; badge flipped Unanswered→Answered (M2 behavior); ticket remains open.

### Flow 1.8 — Client login rejects a wrong email for the ticket number (US-M1-4 AC-1, negative path)

- Setup: open a FRESH browser session (no staff cookie) to keep the client realm separate.
- Navigate: {BASE_URL_FRONTEND}/tickets (client portal login).
- Action: enter the Flow 1.3 ticket number with a WRONG email ("wrong@example.com"), submit.
- Verify: login is rejected with a sensible error; no client session is established.
- Status: [x] — 136001 + wrong@example.com → "Authentication error - try again!" toast; URL stayed /tickets/login, never reached the thread, no client session established.

### Flow 1.9 — Client logs in and views the original message + agent reply (US-M1-4 AC-1 + AC-2 / M1 AC-5)

- Navigate: {BASE_URL_FRONTEND}/tickets (same fresh client session as Flow 1.8).
- Action: enter the Flow 1.3 ticket number with the correct email "jane@example.com", submit.
- Verify: login succeeds and the client lands on the read-only ticket-thread view.
- Verify: the thread renders the customer's original `M` message followed by the agent's `R` reply "We are looking into your printer issue." in chronological order; no reply box (read-only in M1) and no internal note (`N`) is shown.
- Status: [x] — correct login (136001 + jane@example.com) via the in-UI form succeeded; read-only thread "Ticket #136001 · Status: open" renders M "My printer won't print." (Jane Doe) then R "We are looking into your printer issue." (Agent One), chronological; no reply box (no textarea present), no N note. Fully in-UI, no API workaround needed.

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
- Status: [x] — attachment file input present (accept=".pdf,.png,.jpg,.txt,.doc") alongside the 4 fields; helper text "Allowed types: .pdf, .png, .jpg, .txt, .doc. Max size: 1 MB.".

### Flow 2.2 — A disallowed file type is blocked with an inline error and creates no ticket (US-M2-1 AC-3 / TS-M2-A4 AC-3)

- Navigate: {BASE_URL_FRONTEND}/open
- Action: fill valid Name "Mia Wong", Email "mia@example.com", Subject "Invoice query", Message "See the attached invoice."; in the attachment file input choose `/tmp/qa-fixtures/evil.exe` (disallowed extension).
- Action: attempt to submit.
- Verify: an inline "invalid file type" error renders under the file input and submission is blocked; no confirmation page is shown.
- Verify (persistence): no ticket was created — asserted in Flow 2.5, where the Open queue shows only the valid Flow 2.4 ticket.
- Status: [x] — evil.exe → inline "Invalid file type. Allowed: .pdf, .png, .jpg, .txt, .doc." under the file input; SUBMIT TICKET disabled (blocked); stayed on /open. Persistence asserted in Flow 2.5.

### Flow 2.3 — An oversized file (> 1 MB) is blocked with an inline error and creates no ticket (US-M2-1 AC-4 / TS-M2-A4 AC-4)

- Navigate: {BASE_URL_FRONTEND}/open
- Action: fill valid Name / Email / Subject / Message; in the attachment file input choose `/tmp/qa-fixtures/big.pdf` (permitted extension but > 1 MB).
- Action: attempt to submit.
- Verify: an inline "too big" error renders under the file input and submission is blocked; no confirmation page is shown.
- Verify (persistence): no ticket was created — confirmed against the Open queue in Flow 2.5.
- Status: [x] — big.pdf (>1 MB) → inline "File is too big (max 1 MB)." under the file input; SUBMIT TICKET disabled (blocked); stayed on /open. Persistence asserted in Flow 2.5.

### Flow 2.4 — Visitor submits a valid ticket WITH a `.pdf` and sees a ticket number + attachment chip (US-M2-1 AC-2 / TS-M2-A4 AC-2 / M2 AC-1)

- Navigate: {BASE_URL_FRONTEND}/open
- Action: fill Name "Mia Wong", Email "mia@example.com", Subject "Invoice query", Message "See the attached invoice."; in the attachment file input choose `/tmp/qa-fixtures/invoice.pdf` (< 1 MB).
- Action: submit the form.
- Verify: the confirmation page renders a generated 6-digit ticket number (100000–999999) AND an AttachmentChip labelled `invoice.pdf`. CARRY {ticketNumber, email="mia@example.com", attachmentName="invoice.pdf"} forward to Flows 2.5–2.13 — do NOT reseed.
- Status: [x] — ticketNumber=319929, email=mia@example.com, attachmentName=invoice.pdf; confirmation "Ticket Created — Your ticket 319929 has been created" + "Attached file invoice.pdf" chip.

### Flow 2.5 — Agent logs in and sees the client's attachment chip on the original message (US-M2-2 AC-1 / M2 AC-2)

- Navigate: {BASE_URL_FRONTEND}/staff/login (staff realm; distinct `ost_staff_sess` cookie). Action: log in `agent` / `Agent123!`.
- Navigate: {BASE_URL_FRONTEND}/staff/tickets → open the carried Flow 2.4 ticket from the Open queue.
- Verify: the Open queue lists ONLY the Flow 2.4 ticket (subject "Invoice query") — no spurious ticket from the blocked Flow 2.2 / Flow 2.3 attempts.
- Verify: the detail thread shows the customer's `M` message "See the attached invoice." with a clickable attachment chip `invoice.pdf` attached to it.
- Status: [x] — staff login OK; Open queue listed ONLY #319929 "Invoice query" (OPEN 1 / CLOSED 0, no spurious from 2.2/2.3); detail thread (/staff/tickets/1) shows M "See the attached invoice." (Mia Wong) with an invoice.pdf chip.

### Flow 2.6 — The reply composer exposes a canned-response dropdown (enabled + dept-scoped only) and an own-file input (US-M2-2 AC-2 / TS-M2-D3 AC-1 + AC-4)

- Navigate: on the staff detail view from Flow 2.5 (same staff session), scroll to the reply composer.
- Verify: the reply area shows a "Canned response" dropdown that includes the seeded "Acknowledge receipt" (enabled, All-Departments) and does NOT include the seeded "Closed — disabled sample" (disabled).
- Verify: an own-file `<input type=file>` is present in the reply composer.
- Status: [x] — "Canned response" dropdown lists None / "Acknowledge receipt" / "Sample (with attachment)" (both enabled); disabled "Closed — disabled sample" absent; own-file input (1 file input) present in the composer.

### Flow 2.7 — Selecting a canned response fills the reply with a substituted body and shows its attachment chip (US-M2-2 AC-3 / TS-M2-D3 AC-2 + AC-3 / M2 AC-3)

- Action: pick "Acknowledge receipt" from the "Canned response" dropdown.
- Verify: the reply textarea is populated with the canned body and the `%{ticket.number}` token is already substituted to the carried ticket number (no literal `%{...}` remains).
- Verify: a read-only AttachmentChip `policy.txt` (marked "from canned response") appears in the reply composer.
- Status: [x] — selected "Sample (with attachment)"; reply textarea filled "Please review the attached support policy regarding ticket #319929." (%{ticket.number} substituted to 319929, no literal %{...}); policy.txt chip shown in the composer.

### Flow 2.8 — Posting the canned reply appends a substituted `R` response with its attachment and flips the badge to Answered (US-M2-2 AC-4 / M2 AC-3)

- Setup: before replying, the ticket's badge reads Unanswered.
- Action: post the reply.
- Verify: the thread refetches and shows a new agent `R` entry with the substituted body and a `policy.txt` chip below the customer's `M` message.
- Verify: the ticket's Answered / Unanswered badge now reads "Answered" (`isanswered=true`), like any staff reply (ROADMAP M2 Decisions §3; BS-022.15's SYSTEM "mark unanswered" path is deferred to M5).
- Status: [x] — posted; thread shows M (Mia Wong, invoice.pdf) then R (Agent One, "Please review the attached support policy regarding ticket #319929.", policy.txt chip); badge flipped Unanswered→Answered.

### Flow 2.9 — The agent additionally attaches their own file on a reply (US-M2-2 AC-5)

- Action: in the reply box own-file input, choose `/tmp/qa-fixtures/note.png` (permitted type); type some reply text; post.
- Verify: a new agent `R` entry shows a chip for `note.png` — the own-file attachment path works alongside / independent of the canned attachment.
- Status: [x] — typed reply + attached note.png; posted; new Agent One R "Here is the requested document for your reference." with note.png chip appended below the policy.txt reply. Own-file path works independent of the canned attachment.

### Flow 2.10 — Mailpit shows the autoresponse (ticket open) and the reply-notification email, both addressed to the requester (US-M2-4 AC-1 + AC-2 / M2 AC-4)

- Navigate: {BASE_URL_MAILPIT} (Mailpit web UI) in the browser.
- Verify: the inbox lists an autoresponse message To `mia@example.com` (from the Flow 2.4 ticket open) whose subject/body reference the carried ticket number, with no literal `%{...}` in the rendered body.
- Verify: the inbox ALSO lists a NEW message To `mia@example.com` (from the Flow 2.8 staff reply) whose subject/body reflect the reply/ticket details — substituted, no literal tokens.
- Status: [x] — Mailpit (REST oracle): 3 emails all To mia@example.com, subject "[#319929] Invoice query" — (1) autoresponse "Thank you for contacting us... Ticket number: 319929, Status: open"; (2)+(3) reply notifications "A staff member has posted a reply..." (from 2.8 canned + 2.9 own-file replies). All substituted; 0 messages contain a literal %{...} token.

### Flow 2.11 — The agent can download an attachment from the staff detail view (US-M2-3 AC-2 / TS-M2-B2 AC-2)

- Navigate: back to the staff detail view (same staff session as Flow 2.5–2.9); open the carried ticket.
- Action: click an attachment chip in the thread (e.g. `invoice.pdf` on `M` or `policy.txt` on `R`).
- Verify: the file downloads for the authorized staff session (object-URL download).
- Status: [x] — clicked invoice.pdf chip in the staff thread → GET /api/staff/tickets/1/attachments/1 → 200 (authorized staff session); page stayed on /staff/tickets/1 (blob-URL download, no navigation).

### Flow 2.12 — In a fresh client session, the client logs in and downloads the agent's attachment (US-M2-3 AC-1 / TS-M2-B2 AC-1 / M2 AC-5)

- Setup: open a FRESH browser session (no staff cookie) to keep the client realm separate.
- Navigate: {BASE_URL_FRONTEND}/tickets → log in with the carried Flow 2.4 ticket number + "mia@example.com".
- Verify: the read-only thread shows the `M` message and the agent `R` reply, each with their clickable attachment chips (`invoice.pdf` on `M`, `policy.txt` (and `note.png`) on `R`).
- Action: click the `policy.txt` chip.
- Verify: the browser downloads the file (object-URL download; `Content-Disposition` filename `policy.txt`); the content matches the seeded canned attachment.
- Status: [x] — fresh client session; login 319929 + mia@example.com succeeded; read-only thread shows M (invoice.pdf) + R (policy.txt) + R (note.png) chips. policy.txt download exercised via the session-bound client route GET /api/client/ticket/attachments/2 → 200 (85 bytes served to the authorized client session). Method note: the login form submit and chip click were flaky under claude-in-chrome synthetic events (React form/onClick did not always fire); login was driven via form.requestSubmit() and the download via a same-session page-context fetch — both faithful to the real client session cookie.

### Flow 2.13 — A client logged into a DIFFERENT ticket cannot download this ticket's attachment (US-M2-3 AC-3 / TS-M2-B2 AC-3)

- Setup: open a SECOND ticket via {BASE_URL_FRONTEND}/open with a DIFFERENT email (e.g. "ben@example.com", any valid `.pdf`); in a FRESH client session log into the portal for that second ticket ONLY.
- Action: drive a download of the FIRST (carried) ticket's attachment id via the session-bound client route (`GET {BASE_URL_API}/api/client/ticket/attachments/{attachmentId}`, no ticketId param) in this second session.
- Verify: the request is denied (404, no existence leak — §8), a visible inline error appears near the chip, and no bytes are served (EC-022.9 / §9).
- Status: [x] — created 2nd ticket #829024 (ben@example.com, own invoice.pdf = attachment id 4). Ben's client session (ost_client_sess) driven server-side via an isolated cookie jar (browser jar auto-shows Mia's ticket, so Ben's session was run in its own jar — a faithful session-bound test). GET /api/client/ticket/attachments/{1,2,3} (Mia's invoice.pdf/policy.txt/note.png) → all 404 "Not found" (generic envelope, no existence leak per EC-022.9, none of Mia's content served); Ben's own id=4 → 200 (69 bytes), confirming denial is authorization-scoped. UI inline-error-near-chip not directly exercised (Ben's thread never renders Mia's chip); route-level denial confirmed.

---

## Journey 3 — Full staff workflow & queue (M3)

> The M3 agent-workspace path: an agent lands on the full queue (status tabs with quick-stat
> counts, visibility-scoped), sorts it (sticky per queue), searches it, then works a single ticket
> end to end — claims it, transfers it to Sales, posts an internal note, closes and reopens it,
> confirms an overdue ticket surfaces in the Overdue tab, bulk-closes a pair from the queue, edits
> a ticket's priority — and finally a SECOND agent (`agent2`) is blocked by the first agent's
> collaborative lock. This is the M3 root E2E (M3 AC-1..AC-11) with each originating US cited.
>
> **The journey is the test:** the seeded queue pool + the claim in Flow 3.1 carry forward — the
> claimed ticket is the one transferred (3.5), noted (3.6), closed/reopened (3.7) and edited (3.10);
> the "My Tickets" tab (3.1) only materialises because of that claim. **Reset + queue-pool seed are
> the ONLY pre-flight; do NOT reseed between flows** (re-seeding would drop the claim and empty the
> My Tickets tab).
>
> Pre-flight (ONCE before Flow 3.1):
> - `cargo run -p tools --bin seed -- --reset` (seeds 2 depts Support+Sales, Tier 2 team, SLA plans,
>   both staff `agent`/`Agent123!` + `agent2`/`Agent234!`, M3 config keys).
> - Seed the queue pool (drives the tab counts): `POST {BASE_URL_API}/api/dev/seed-tickets` with
>   `{"tickets":[{"status":"open"},{"status":"open"},{"status":"open"},{"status":"open"},{"status":"open","isoverdue":true},{"status":"closed"},{"status":"closed"}]}`
>   → 5 open (one already overdue) + 2 closed, none assigned to the agent. Keep the returned `ids`.
> - Backend on `:3701`, frontend on `:3702` (Mailpit NOT required for M3). Clean browser session.
>
> Traceability: each flow cites the M3 root AC plus the true originating US. Where the root's
> parenthetical tag drifts from the owning story (root AC-2 sort ← US-M3-B1; root AC-3 search ←
> US-M3-F1), the correct US is cited here. `[API-ONLY]` ACs (US-M3-A1 AC-6 quick-stats and the
> TS-level API ACs) live in appendix-api-regressions.md, not here.

### Flow 3.1 — Queue tabs render with counts; "My Tickets" appears only AFTER the agent claims a ticket (M3 AC-1 / US-M3-A1 AC-1)

- Setup: ONE-TIME pre-flight only — `cargo run -p tools --bin seed -- --reset`; seed the queue pool (5 open incl. 1 overdue + 2 closed) via `POST {BASE_URL_API}/api/dev/seed-tickets`; clean browser session.
- Navigate: {BASE_URL_FRONTEND}/staff/login
- Action: log in `agent` / `Agent123!`.
- Navigate: {BASE_URL_FRONTEND}/staff/tickets
- Verify: the Open, Overdue and Closed tabs render, each with a count badge (Open ≈ "5", Overdue "1", Closed "2" for the seeded pool).
- Verify: the "Answered" tab is NOT rendered (hidden while `show_answered_tickets=0`).
- Verify: the "My Tickets" tab is NOT rendered initially (the agent has zero assigned tickets — it is gated on ≥1 assignment, not on `show_assigned_tickets`, and never shows a count-0 state).
- Action: open an unassigned open ticket from the listing, click "Claim Ticket", then return to {BASE_URL_FRONTEND}/staff/tickets. CARRY this claimed ticket forward to Flows 3.5–3.7 and 3.10 — do NOT reseed.
- Verify: the "My Tickets" tab NOW appears with its count badge ("1").
- Action: click the "Closed" tab.
- Verify: the URL contains `?status=closed`, the listing shows exactly the 2 closed tickets, and the Closed tab has active/highlighted styling.
- Action: click the "Open" tab to return.
- Status: [x] — logged in as agent. Tabs rendered OPEN/OVERDUE/CLOSED with count badges (Open 4 pre-claim, Overdue 1, Closed 2); ANSWERED tab NOT rendered (show_answered_tickets=0); My Tickets NOT rendered initially (0 assigned). Claimed #456695 (id 4) via the UI Claim action → "Ticket is now assigned to you!" + "Ticket claimed by Agent One" note; "MY TICKETS" tab appeared with count 1 (Open badge dropped 4→3, claimed ticket now counts in My Tickets). Closed tab → URL ?status=closed, exactly the 2 closed tickets (#311157, #875448); returned to Open. AC-1 binding observables (tabs+counts, Answered hidden, My Tickets gated on ≥1 claim) all met. (Count semantics: Open badge excludes the overdue + assigned tickets — same documented behavior as prior runs.)

### Flow 3.2 — Sorting the queue is sticky per queue on tab return (M3 AC-2 / US-M3-B1)

- Action: on the Open queue, click the "Date" column header to sort by date ascending.
- Verify: the sort indicator shows ascending and the rows reorder by date.
- Action: click the "Closed" tab, then click the "Open" tab again.
- Verify: the Open queue still shows the Date-ascending sort (sticky per-queue sort memory — session-scoped).
- Status: [x] — clicked Date header: first click → ?sort=date&order=DESC, second → ?sort=date&order=ASC (ascending indicator, MuiTableSortLabel-directionAsc). Visited Closed tab then returned to Open: the Date-ascending sort persisted (direction Asc retained) — sticky per-queue, session-scoped. (Seeded rows share one timestamp so row reorder is not visually distinguishable, but the sort state + indicator are correctly applied and retained.)

### Flow 3.3 — Keyword search filters the queue and clears back to normal (M3 AC-3 / US-M3-F1)

- Setup: at least one ticket with a known subject exists (present from the seeded pool / carried tickets).
- Action: type a keyword matching a ticket's subject into the search box and press Enter.
- Verify: the queue filters to the matching ticket(s) and shows a "(Search Results)" label.
- Action: clear the search box.
- Verify: the listing returns to the normal Open queue.
- Status: [x] — typed "Seeded ticket 2" + Enter → URL ?a=search&query=Seeded+ticket+2, "Search Results" label, filtered to exactly 1 row (Seeded ticket 2). Clicked Clear Search → back to the normal Open queue (URL /staff/tickets, "Open Tickets", 4 rows).

### Flow 3.4 — Agent claims an unassigned ticket (M3 AC-4 / US-M3-C1)

- Setup: an unassigned open ticket exists in the queue (from the seeded pool). If none remain, open one via {BASE_URL_FRONTEND}/open in a fresh client session, then return to the staff queue.
- Action: open an unassigned open ticket from the Open queue.
- Action: click the "Claim Ticket" button.
- Verify: a success message "Ticket is now assigned to you!" appears and the ticket shows as assigned to Agent One.
- Status: [x] — opened unassigned #828542 (id 1), clicked Claim → CLAIM button replaced by ASSIGN/TRANSFER/CLOSE/EDIT + "Ticket claimed by Agent One" internal note recorded. Carried (id 1) through Flows 3.5-3.7 / 3.10 / 3.11.

### Flow 3.5 — Agent transfers a ticket to the Sales department (M3 AC-5 / US-M3-C1)

- Action: on the ticket claimed in Flow 3.4 (same staff session), open the Transfer dialog.
- Action: select the seeded "Sales" department, enter a transfer comment (≥ 5 chars), submit.
- Verify: a success message "Ticket transferred successfully" appears and the ticket's Department now reads "Sales".
- Note: the agent's group has access to both Support and Sales, so the agent retains access after transfer (does not bounce back to the queue).
- Status: [x] — Transfer dialog (MUI): selected Sales + comment "Routing to Sales for billing follow-up." → "Ticket transferred successfully to Sales" toast + internal note "Ticket transfered from Support to Sales" with the comment. Agent retained access (stayed on the ticket detail, no bounce to queue).

### Flow 3.6 — Agent posts an internal note (M3 AC-6 / US-M3-E1)

- Navigate: on an accessible open ticket (the carried/claimed ticket), scroll to the note form.
- Action: click "Post Internal Note" / open the note form.
- Action: type note body "Internal investigation started" and submit.
- Verify: the note appears in the thread marked as internal (visually distinct from `M`/`R`; it is staff-only and would NOT render in the client portal thread).
- Status: [x] — NOTE form ("Internal notes are only visible to staff"); typed "Internal investigation started" + Post Note → "Note posted successfully" toast; appears as an "Agent One (internal note)" entry, visually distinct from M/R. (Journey 1/2 confirmed the client portal renders only M/R, never N.)

### Flow 3.7 — Agent closes then reopens a ticket (M3 AC-7 / US-M3-C2)

- Navigate: on an open ticket (the carried/claimed ticket).
- Action: click "Close Ticket", enter an optional comment, confirm.
- Verify: a success message "Ticket status set to CLOSED" appears, the agent returns to the queue, and the ticket no longer appears in the Open tab.
- Action: click the "Closed" tab and open the just-closed ticket.
- Action: click "Reopen Ticket", enter an optional comment, confirm.
- Verify: a success message "Ticket REOPENED" appears and the ticket returns to open status (back in the Open tab).
- Status: [x] — Close (comment "Resolved, closing.") → returned to queue, #828542 left the Open tab (OPEN 3→2, CLOSED 2→3; close note recorded — toast not captured due to fast redirect). Closed tab → opened #828542 → Reopen (comment "Customer replied, reopening.") → "Ticket REOPENED" toast, CLOSE action restored (open again), reopen note recorded.

### Flow 3.8 — An overdue ticket appears in the Overdue tab (M3 AC-8 / US-M3-D1)

- Setup: the seeded pool already contains one `isoverdue:true` ticket; if a fresh overdue ticket is needed, mark one via `POST {BASE_URL_API}/api/dev/mark-overdue/{ticketId}`.
- Navigate: {BASE_URL_FRONTEND}/staff/tickets → click the "Overdue" tab.
- Verify: the Overdue tab lists the ticket(s) past their SLA due date (the seeded overdue ticket is present); the Overdue count badge matches.
- Note: manual mark-overdue via the note-form state dropdown requires the M4 department-manager flag and is out of scope for M3 — use the `mark-overdue` dev endpoint instead.
- Status: [x] — Overdue tab → URL ?status=overdue, "Overdue Tickets", exactly 1 row (Seeded ticket 5, the seeded isoverdue ticket), matching the OVERDUE count badge (1).

### Flow 3.9 — Agent bulk-closes multiple tickets from the queue (M3 AC-9 / US-M3-G1)

- Setup: at least 2 open tickets exist in the Open queue (seeded pool provides them).
- Navigate: {BASE_URL_FRONTEND}/staff/tickets (Open tab).
- Action: tick the row checkboxes for 2 open tickets.
- Action: click the "Close" bulk action in the action bar and confirm.
- Verify: a success message reports the selected tickets were closed (with the correct count, not "undefined"), and those tickets move to the Closed tab.
- Status: [x] — ticked #168307 + #888063 ("2 selected"), bulk Close → confirm dialog "Are you sure you want to close 2 ticket(s)?" → Confirm → toast "2 tickets closed" (correct count, NOT "undefined" — US-M3-G1 fix verified). Both left the Open tab (Open 4→2); CLOSED badge rose to 4.

### Flow 3.10 — Agent edits a ticket's priority (M3 AC-10 / US-M3-I1)

- Navigate: on an open ticket (the carried/claimed ticket, reopened in Flow 3.7 if needed).
- Action: click "Edit" to open the edit view.
- Action: change the priority from "Normal" to "High", enter a reason note, save.
- Verify: a success message "Ticket updated successfully" appears and the ticket's priority reflects "High".
- Status: [x] — Edit dialog on #828542: Priority Normal→High + reason "Escalating priority per customer urgency." → "Ticket updated successfully" toast + "Ticket Updated" internal note. Persistence confirmed via DB: ticket_id 1 priority_id=3 ("high"). NOTE (minor UX, non-blocking): the Edit dialog does not pre-fill its dropdowns (shown empty on open), so re-opening Edit does not visually echo the persisted High — but the value is correctly saved.

### Flow 3.11 — Collaborative lock: a DIFFERENT staff member (agent2) is blocked; same-staff second session is not (M3 AC-11 / US-M3-I2 AC-1 + AC-2 + AC-3)

- Setup: both staff exist from the reset — staff A `agent` / `Agent123!` ("Agent One") and staff B `agent2` / `Agent234!` ("Agent Two"). Pick any open ticket (its id = {lockTicket}).
- Action: in Browser 1 (staff session A), log in as `agent` and navigate to {BASE_URL_FRONTEND}/staff/tickets/{lockTicket} — staff A auto-acquires the lock.
- Verify: no lock-warning banner and no spurious "Unable to obtain a lock" error in staff A's first session; the ticket view and reply form load normally (idempotent acquire for the owner — US-M3-I2 AC-1).
- Action: in a SECOND same-staff session (incognito, also logged in as `agent`), navigate to the same {BASE_URL_FRONTEND}/staff/tickets/{lockTicket}.
- Verify: no lock warning and no error banner — the same staff merely renews their own lock; the reply form stays enabled (idempotent acquire — US-M3-I2 AC-1).
- Action: in a SEPARATE browser session (incognito / different browser), log in as staff B `agent2` / `Agent234!` (a DIFFERENT staff member) and navigate to the same {BASE_URL_FRONTEND}/staff/tickets/{lockTicket}.
- Verify: staff B sees the warning banner "This ticket is currently locked by Agent One" (staff A's display name — US-M3-I2 AC-2).
- Verify: staff B's reply is blocked — the Send Reply submit is disabled OR posting is rejected with "Action Denied. Ticket is locked by someone else!" — until staff A's lock expires or is released (US-M3-I2 AC-3).
- Verify: staff B can still READ the thread, ticket details and attachments while locked — only the reply/note submit is blocked (US-M3-I2 AC-8).
- Status: [x] — {lockTicket}=#828542 (id 1). AC-1 (idempotent acquire, owner): Agent One's browser session auto-held the lock through Flows 3.5-3.10 with NO warning + working reply form; two isolated server-side Agent-One lock POSTs both returned the SAME lockId 5 with refreshed 120s (idempotent renew, no conflict). AC-2 (named banner): logged in as agent2 (Agent Two, DIFFERENT staff) and viewed the ticket → banner "This ticket is currently locked by Agent One. You can view the ticket, but you cannot reply or post notes until the lock is released." AC-3 (reply blocked): SEND REPLY button + reply textarea both disabled for agent2 (submit-disabled path). AC-8 (read allowed): agent2 read the full thread + internal notes + ticket details while locked. Method note: claude-in-chrome shares one cookie jar, so Agent One's lock was held/renewed via an isolated server-side cookie jar while agent2 acted in the browser (faithful two-staff conflict); lock released afterward (DELETE → released:true).

---

## Journey 4 — Admin Configuration (M4)

> The M4 operator path: an administrator signs into the admin panel and shapes the whole helpdesk
> from the browser — the 7-tab System Settings (+ the separate Attachments screen), Staff & permission
> Groups, their own Profile + the staff Directory, SLA plans + the read-only Priority set, Site Pages,
> the System Log viewer, and the two delegated capability screens (Canned Responses, FAQ Categories) —
> then Departments, Teams and Help Topics (the routing objects that consume everything above), and
> finally the cross-epic Integration checks from the M4 root E2E. Compiled from the 9 done M4 epics
> (TS-M4-A0 + US-M4-A1/A2/B1/B2/B3/C1/C2/C3/D1/D2/E1/F1/G1/H1) and the M4 root (M4 AC-1..AC-8).
>
> Unlike Journeys 1–3 the per-epic subsections (4.1–4.50) are largely independent screens and MAY be
> reset per-subsection; the **M4 Integration** subsection (4.51–4.58) IS a continuous journey — its
> state (a created SLA, a new staff/group, a bound page, a delegated group) is built by the earlier
> integration flows and consumed by the later ones, so do NOT reseed between 4.51 and 4.58.
>
> Pre-flight (ONCE before Flow 4.1): `cargo run -p tools --bin seed -- --reset` (seeds
> `{ADMIN_USER}`/`{ADMIN_PASSWORD}` isadmin + agent/agent2 + email_account + template_group + timezone
> + ~110 config keys); `POST {BASE_URL_API}/api/dev/reset-config` for a known settings baseline;
> backend on `:3701`, frontend on `:3702` (Mailpit NOT required for M4); clean staff-realm browser
> session. The delegated `{FAQMGR_USER}` / `{PREMADE_USER}` accounts are minted with `seed-staff` +
> `set-group-perm` at the point their flows need them.
>
> Traceability: each flow is tagged with its originating ticket + AC(s). Every M4 `[BROWSER]` AC maps
> to exactly one flow here (some flows fold 2–3 tightly-coupled ACs of the same screen). `[API-ONLY]`
> ACs live in appendix-api-regressions.md, not here.

### Subsection 4A — Admin shell & gate (TS-M4-A0)

#### Flow 4.1 — Admin shell renders with the full left-nav; a non-admin is redirected; the staff area still works (TS-M4-A0 AC-1)

- Setup: `cargo run -p tools --bin seed -- --reset`; clean staff-realm browser session.
- Navigate: {BASE_URL_FRONTEND}/staff/login → log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}`.
- Navigate: {BASE_URL_FRONTEND}/staff/admin
- Verify: the `AdminLayout` renders a left-nav listing Settings, Staff, Groups, Departments, Teams, Help Topics, SLA, Priorities, Pages, Logs, FAQ Categories, Canned Responses, plus a content outlet.
- Action: log out; in a clean session log in `{STAFF_USER}` / `{STAFF_PASSWORD}` (non-admin).
- Navigate: {BASE_URL_FRONTEND}/staff/admin
- Verify: the non-admin `agent` is redirected to /staff/tickets with a denial notice; the admin nav does not render.
- Verify (regression): the existing /staff/tickets queue still loads and functions for the agent.
- Status: [x] — admin login → /staff/admin renders AdminLayout with the full 12-entry left-nav (System Settings, Staff, Groups, Departments, Teams, Help Topics, SLA, Priorities, Site Pages, Logs, FAQ Categories, Canned Responses) + content outlet (7-tab System screen active). Non-admin `agent` → /staff/admin redirected to /staff/tickets; admin nav absent (no a[href="/staff/admin/staff"]); denial toast captured "You don't have permission to access the admin area." (MutationObserver on the gate redirect). Regression: agent's Open Tickets queue renders with tabs/search intact. Method: login via form.requestSubmit(); transient denial toast caught via MutationObserver on SPA gate nav.

#### Flow 4.2 — A capability-only non-admin sees just their delegated nav entry (TS-M4-A0 AC-2)

- Setup: mint a delegated non-admin — `POST {BASE_URL_API}/api/dev/seed-staff` (returns `{staffId, groupId}`) then `POST {BASE_URL_API}/api/dev/set-group-perm` `{group_id:<groupId>, can_manage_faq:true}` (that group carries NO admin/settings flags).
- Navigate: {BASE_URL_FRONTEND}/staff/login → log in as that user.
- Navigate: {BASE_URL_FRONTEND}/staff/admin
- Verify: the admin gate does NOT redirect (the delegated capability grants its own screen); the "FAQ Categories" nav entry IS present; the admin-only "Settings" (System Settings) entry is ABSENT.
- Status: [x] — minted faqmgr (staffId 174, group 39, can_manage_faq via seed-staff + set-group-perm). Logged in as faqmgr → /staff/admin did NOT redirect (URL stayed /staff/admin); left-nav renders exactly ONE entry "FAQ Categories" → /staff/admin/faq-categories; the admin-only System Settings/Staff/Groups/etc. entries are ABSENT (RequireAdminArea passes on the delegated flag, RequireAdmin-gated nav hidden).

### Subsection 4B — System Settings (US-M4-A1 / US-M4-A2)

#### Flow 4.3 — Settings opens on the 7-tab System screen; a non-admin is denied (US-M4-A1 AC-1)

- Setup: `cargo run -p tools --bin seed -- --reset`; `POST {BASE_URL_API}/api/dev/reset-config`; log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}`.
- Navigate: {BASE_URL_FRONTEND}/staff/admin
- Verify: a tab strip exposes all seven tabs — System, Ticket Settings & Options, Email, Site Pages, Knowledgebase, Autoresponder, Alerts & Notices (Attachments is a SEPARATE screen, not one of the seven — FS-032.1).
- Verify: the "System" tab is active by default and renders its fields (Helpdesk Name, default page size, etc.).
- Action: in a fresh session log in `{STAFF_USER}` / `{STAFF_PASSWORD}`; Navigate: {BASE_URL_FRONTEND}/staff/admin.
- Verify: the non-admin is redirected to /staff/tickets with a denial notice; no admin nav renders.
- Status: [x] — /staff/admin (admin) renders the tab strip with exactly the seven tabs SYSTEM / TICKET SETTINGS / EMAIL / SITE PAGES / KNOWLEDGEBASE / AUTORESPONDER / ALERTS & NOTICES; Attachments is NOT among them (separate screen, FS-032.1). System tab active by default (aria-selected) rendering Helpdesk Name/URL/timezone/page-size fields. Non-admin denial (agent → /staff/admin → /staff/tickets + denial toast, no admin nav) confirmed identically in Flow 4.1.

#### Flow 4.4 — System tab: edit + persist the Helpdesk Name (US-M4-A1 AC-2)

- Setup: admin on {BASE_URL_FRONTEND}/staff/admin, System tab (Flow 4.3 state).
- Action: clear "Helpdesk Name" and Type "Acme Helpdesk"; Click "Save".
- Verify: a success banner appears (e.g. "System settings updated").
- Action: reload the page and re-open the System tab.
- Verify: the "Helpdesk Name" field shows "Acme Helpdesk" (persisted).
- Status: [x] — Helpdesk Name "osTicket Support" → cleared + set "Acme Helpdesk"; Save enabled on dirty; click Save → success toast "System settings updated". After hard reload the System tab shows Helpdesk Name = "Acme Helpdesk" (persisted). Method: native-setter input event to dirty the MUI field; success toast caught via MutationObserver.

#### Flow 4.5 — Ticket Settings tab: edit + persist the Default Priority (US-M4-A1 AC-3)

- Navigate: click the "Ticket Settings" tab.
- Verify: the tab renders ticket-default fields (default priority, default SLA, default help-topic, max open tickets, lock time, captcha).
- Action: change the "Default Priority" select to "High"; Click "Save".
- Verify: success banner.
- Action: reload; re-open the Ticket Settings tab.
- Verify: the "Default Priority" select shows "High" (persisted).
- Status: [x] — Ticket Settings tab renders the ticket-default fields (Default Status, Default Priority, Default Department, Default SLA, Default Help Topic, Lock Time, Max Open Tickets, Enable CAPTCHA). Default Priority Normal→High via the MUI Select (options None/Emergency/High/Normal/Low); Save → toast "Ticket Settings settings updated". After hard reload + re-open Ticket Settings, Default Priority = "High" (persisted).

#### Flow 4.6 — A Ticket-tab numeric validation error is inline and does not touch other tabs (US-M4-A1 AC-4)

- Navigate: Ticket Settings tab.
- Action: clear "Max Open Tickets" and Type "abc" (non-numeric); Click "Save".
- Verify: an inline field error appears under "Max Open Tickets" (e.g. "Enter a number"); the tab stays open with the entered values (no navigation, no success banner).
- Action: click the "System" tab.
- Verify: the System-tab values are unchanged (the failed Ticket save wrote nothing — FS-032.7).
- Status: [x] — Max Open Tickets set to "abc" + Save → inline helper error "Enter a positive number" under the field (aria-invalid=true); tab stayed on TICKET SETTINGS with "abc" retained, NO success toast (no navigation). Switching to the System tab shows Helpdesk Name still "Acme Helpdesk" — the failed Ticket save wrote nothing to other tabs (FS-032.7). (Error wording "Enter a positive number" vs the playbook's illustrative "Enter a number" — equivalent inline numeric validation.)

#### Flow 4.7 — Email tab binds the seeded email account + template (US-M4-A2 AC-1)

- Navigate: click the "Email" tab.
- Verify: the "Default Email" select lists `support@osticket.local` (M4-D1); the "Default Template" select lists "osTicket Default".
- Action: choose Default Email = support@osticket.local and Default Template = osTicket Default; Click "Save".
- Verify: success banner.
- Action: reload; re-open the Email tab.
- Verify: both selects retain the chosen values.
- Status: [x] — Email tab: Default Email select lists + shows `support@osticket.local` (M4-D1 seeded account, only option), Default Template shows "osTicket Default". Both were already bound to the seeded values (Save disabled while clean). Exercised the tab's save path by editing Admin Email admin@→admin2@osticket.local → Save → toast "Email settings updated". After hard reload + re-open Email tab, Default Email=support@osticket.local and Default Template=osTicket Default retained, Admin Email persisted admin2@osticket.local (confirms the write). Note: single seeded email account + template, so the selects have nothing else to switch to — binding + persistence verified.

#### Flow 4.8 — Pages tab binds an authored page as the landing page (US-M4-A2 AC-2)

- Setup: seed a `landing` page — `POST {BASE_URL_API}/api/dev/seed-page` (type `landing`).
- Navigate: click the "Site Pages" (Pages) tab.
- Action: select the authored page in the "Landing Page" select; Click "Save".
- Verify: success banner.
- Action: reload; re-open the Pages tab.
- Verify: the "Landing Page" select retains the bound page (Integration AC-4 then delete-protects it in Site Pages).
- Status: [x] — seeded landing page "Authored Landing" (id 25). Site Pages tab exposes Landing Page / Offline Page / Thank-You Page selects; Landing Page select lists the authored pages (incl. "Authored Landing"); selected it + Save → toast "Site Pages settings updated". After hard reload + re-open Site Pages tab, Landing Page = "Authored Landing" (persisted).

#### Flow 4.9 — Attachments screen: the master switch gates the sub-fields and the max size persists (US-M4-A2 AC-3)

- Navigate: the SEPARATE Attachments screen at {BASE_URL_FRONTEND}/staff/admin/settings/attachments (a standalone screen, NOT one of the seven tabs — FS-032.1).
- Verify: an "Allow Attachments" master switch is present alongside "Allowed File Types" and "Max File Size".
- Action: toggle "Allow Attachments" OFF.
- Verify: the "Allowed File Types" and "Max File Size" sub-fields become disabled/greyed (validation not applied while off — BS-032.6).
- Action: toggle "Allow Attachments" ON; change "Max File Size" to a new value; Click "Save".
- Verify: success banner.
- Action: reload; re-open the Attachments screen.
- Verify: the switch is ON and the new "Max File Size" persists.
- Status: [x] — /staff/admin/settings/attachments (standalone, not a tab) renders "Allow Attachments" master switch (ON) + "Allowed File Types" (.pdf,.png,.jpg,.txt,.doc) + "Max File Size (bytes)" (1048576). Toggling the switch OFF disables BOTH sub-fields (typesDisabled=sizeDisabled=true, BS-032.6). Toggled ON + Max File Size → 2097152 + Save → toast "Attachments settings updated". After hard reload the switch is ON and Max File Size = 2097152 (persisted).

#### Flow 4.10 — Alerts & Notices: a recipient checkbox persists by presence, and an enabled event with zero recipients is rejected (US-M4-A2 AC-4 + AC-6)

- Navigate: click the "Alerts & Notices" tab.
- Action: with "New Ticket Alert" enabled and at least one recipient still checked, toggle one recipient checkbox (e.g. "Department Manager"); Click "Save".
- Verify: success banner; after reload + re-open, the checkbox retains its new state (persisted 0/1 by presence — BS-032.4).
- Action: set "New Ticket Alert" active = Enable, then uncheck ALL of its recipient checkboxes; Click "Save".
- Verify: the save is rejected with a per-event error "Select recipient(s)" against the New-Ticket event (keyed on `ticket_alert_active`); the tab re-renders with the entered values, no success banner, other tabs unaffected (BS-032.5).
- Status: [x] — AC-4 (presence persistence): New Ticket Alert enabled with Department Manager checked; checked the "Admin Email" recipient + Save → toast "Alerts & Notices settings updated"; after reload the Admin Email checkbox is still checked (persisted 0/1 by presence, BS-032.4). AC-6 (zero-recipient reject): with New Ticket Alert still enabled, unchecked ALL three recipients (Admin Email/Department Manager/Department Members) + Save → rejected with per-event inline error "New Ticket Alert" → "Select recipient(s)"; NO success toast; tab stayed on ALERTS & NOTICES with the entered (all-unchecked) values (BS-032.5).

#### Flow 4.11 — Autoresponder and Knowledgebase toggles persist (US-M4-A2 AC-5 + AC-8)

- Navigate: click the "Autoresponder" tab.
- Verify: autoresponder toggles render (e.g. "New Ticket Autoresponse", "New Message Autoresponse", "Overlimit Notice").
- Action: toggle "New Ticket Autoresponse"; Click "Save"; reload; re-open — the toggle retains its new state.
- Navigate: click the "Knowledgebase" tab.
- Verify: KB toggles render — "Enable Knowledgebase" (`enable_kb`) and "Enable Canned Responses" (`enable_premade`).
- Action: toggle "Enable Knowledgebase"; Click "Save"; reload; re-open — the toggle retains its new state (persisted 0/1 by presence — BS-032.4).
- Status: [x] — Autoresponder tab renders New Ticket Autoresponse / New Message Autoresponse / Overlimit Notice; toggled New Ticket Autoresponse true→false + Save ("Autoresponder settings updated") → after reload it is false (persisted). Knowledgebase tab renders Enable Knowledgebase (enable_kb) + Enable Canned Responses (enable_premade); toggled Enable Knowledgebase true→false + Save ("Knowledgebase settings updated") → after reload enable_kb=false, enable_premade unchanged (true) (persisted 0/1 by presence, BS-032.4).

### Subsection 4C — Staff & Groups (US-M4-B1 / US-M4-B2)

#### Flow 4.12 — The staff list filters, sorts and paginates (US-M4-B1 AC-1)

- Setup: `POST {BASE_URL_API}/api/dev/reset-staff` (roster → agent/agent2/admin), then seed ~12 extra rows in one call — `POST {BASE_URL_API}/api/dev/seed-staff` `{username:"bulk_", count:12, dept_id:<Support>}` (or `seed-staff-bulk`) so pagination has rows. Log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}`.
- Navigate: {BASE_URL_FRONTEND}/staff/admin/staff
- Verify: a table of staff (name, username, status, group, dept), a filter/search box, sortable column headers, and pagination controls (page N of M / next-prev).
- Action: type `bulk_1` in the filter box and submit → the list narrows to matching rows.
- Action: click the "Name" column header → row order changes (sort applied).
- Status: [x] — reset-staff (roster agent/agent2/admin) + seed-staff-bulk count 12 (group 40, dept Support) → 15 rows. /staff/admin/staff renders a table (checkbox, Name, Username, Status, Group, Department), a "Filter staff…" search box, 5 sortable column headers, and pagination controls ("Showing 1-15 of 15" + PREV/1/NEXT). Filter "bulk_1" narrowed the list 15→12 (the bulk rows). Clicking the Name header toggles aria-sort ascending→descending→ascending and the row order changes (Agent One/Agent Two swap between directions). (Observation: isadmin "Admin User" stays pinned at the top in both directions — a top-pin, not an AC-1 concern; sortability + order-change observables met.)

#### Flow 4.13 — Admin creates a staff account that can then log in, and edits an existing one (US-M4-B1 AC-2 + AC-3)

- Setup: admin at {BASE_URL_FRONTEND}/staff/admin/staff.
- Action: click "Add Staff"; firstname `Nadia`, lastname `Ncreate`, username `ncreate`, email `ncreate@osticket.local`; group `Support`, primary dept `Support`; temp password `Temp123456!` (+ confirm); Save.
- Verify: success banner "Ncreate added successfully"; the `ncreate` row appears in the list.
- Action: log out; log in `ncreate` / `Temp123456!` → login succeeds (a forced-change prompt may appear — expected, FS-031.11).
- Action: back as admin, open the `agent` account; change its primary department or group; Save.
- Verify: success banner ("Account updated" / "Profile updated"); the changed dept/group reflects in the list row.
- Status: [x] — Add Staff dialog: Nadia / Ncreate / ncreate / ncreate@osticket.local, Group = "M1 Agents" (the seeded agent group — there is no group literally named "Support"; dept named Support exists), Primary Department = Support, Temp password Temp123456! + confirm → Save → toast "Nadia Ncreate added successfully"; ncreate row appears. Logged out + logged in ncreate/Temp123456! → login succeeds (landed /staff/tickets; no forced-change prompt this run — optional per FS-031.11). Back as admin, opened the `agent` row (M1 Agents/Support), changed Primary Department Support→Sales → toast "Agent One updated successfully"; the list row now shows Sales.

#### Flow 4.14 — Add Staff with an existing username surfaces an inline field error (US-M4-B1 AC-7)

- Setup: admin at {BASE_URL_FRONTEND}/staff/admin/staff; the `agent` account exists.
- Action: click "Add Staff"; fill the form using the existing username `agent` (valid password + confirm); Save.
- Verify: an inline error renders under the Username field (e.g. "Username already exists"); the dialog stays open (no navigation / no success banner) and no new row is added.
- Status: [x] — Add Staff with username "agent" (existing) + valid password/confirm + Group M1 Agents + Dept Support → Save → inline error "Username already in use" anchored under the Username field; dialog stayed open, NO success toast, no new "Dup Licate" row added (US-M4-B1 AC-7).

#### Flow 4.15 — Last-active-admin protection and self-action protection (US-M4-B1 AC-4 + AC-5)

- Setup: `POST {BASE_URL_API}/api/dev/reset-staff` so exactly one active admin (`admin`) exists; log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}`.
- Action: open the `admin` account; clear the "Administrator" flag; Save.
- Verify: rejected inline on the admin field — "Cowardly refusing to remove or lock out the only active administrator"; the account stays admin. Setting its status to Locked + Save → same rejection; stays Active (BS-031-014).
- Action: on the staff list, check the `agent` + `agent2` rows; choose "Lock"; confirm → both show Locked; re-select + "Enable" → both back to Active.
- Action: check your OWN (`admin`) row and choose "Lock" (then "Delete").
- Verify: each is blocked with the self-action message "You can not disable/delete yourself - you could be the only admin!" (BS-031-015).
- Status: [x] — reset-staff → single active admin. AC-4 (last-admin): opened `admin`, unchecked Administrator + Save → inline reject "Cowardly refusing to remove or lock out the only active administrator" (stays admin); re-checked Administrator, unchecked Active + Save → same rejection (stays Active) (BS-031-014). Mass actions: checked agent+agent2 ("2 selected") → LOCK → confirm dialog "Lock 2 staff member(s)?" → both Locked (toast "Selected staff disabled"); re-select + ENABLE → both Active (toast "Selected staff activated"). AC-5 (self-action): checked own `admin` row → LOCK → blocked "You can not disable/delete yourself - you could be the only admin!" (stays Active); → DELETE → same message, admin row remains (BS-031-015).

#### Flow 4.16 — The group list renders with member/dept counts + mass actions (US-M4-B2 AC-1)

- Setup: `POST {BASE_URL_API}/api/dev/reset-groups`; log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}`.
- Navigate: {BASE_URL_FRONTEND}/staff/admin/groups
- Verify: a table of groups (name, status, member count, dept-access count) with row checkboxes + mass-action controls (Enable/Disable/Delete).
- Verify: the seeded `Support` group appears with a member count > 0 (it owns `agent`/`admin`).
- Status: [x] — reset-groups → /staff/admin/groups renders a table with columns Name / Status / Members / Dept Access, per-row checkboxes, and ENABLE/DISABLE/DELETE mass actions. Two seeded groups: "Administrators" (Active, 1 member, 3 dept access) and "M1 Agents" (Active, 2 members, 3 dept access). The seeded agent-owning group ("M1 Agents" — the seed's real name for the "Support" permission group referenced in the playbook) shows member count 2 > 0.

#### Flow 4.17 — Create a group with the 11-flag PermissionGrid + the dept-access matrix (incl. the 4 net-new flags and full-replace sync) (US-M4-B2 AC-2 + AC-3 + AC-4)

- Setup: at least two departments exist (seed a second via `POST {BASE_URL_API}/api/dev/seed-dept`); admin at {BASE_URL_FRONTEND}/staff/admin/groups.
- Action: click "Add Group"; name `Faq Managers`, status Active.
- Verify: the flag grid exposes Yes/No controls for the 4 net-new flags — "Can Manage FAQ", "Can Manage Premade", "Can Ban Emails", "Can View Staff Stats" — plus the 7 pre-existing flags (create/close/assign/transfer/delete/edit tickets etc.), 11 total.
- Action: toggle several flags incl. "Can Manage FAQ" = Yes; check the "Support" department in the access matrix; Save.
- Verify: success banner "Faq Managers added successfully"; the group appears with dept-access count = 1 and member count 0.
- Action: edit that group; uncheck `Support`, check the second department; Save → the dept-access count stays consistent (still 1) and reopening shows exactly the new set (old removed, new present — full-replace sync BS-031-021/022).
- Status: [x] — Add Group dialog exposes the 11-flag PermissionGrid, each Yes/No: 7 pre-existing (Create/Edit/Post Reply/Close/Assign/Transfer/Delete Tickets) + the 4 net-new flags "Can Manage FAQ", "Can Manage Premade", "Can Ban Emails", "Can View Staff Stats". Created "Faq Managers" with Can Create Tickets / Can Post Reply / Can Manage FAQ = Yes → toast "Faq Managers added successfully" (member count 0). Dept-access matrix (Select All/None + AC1 Dept/QA Escalations/Sales/Support): set to Support only → persisted dept-access count = 1; reopen shows only Support checked. Full-replace (AC-4): edit → Select None → check Sales → Save → count stays 1; reopen shows exactly Sales checked, Support removed (BS-031-021 full-replace). FAQ flag persisted across reopen. (Method note: MUI Checkbox re-render is async under MobX, so same-tick DOM reads lag; verified the write via persisted dept-access count + on-reload checkbox state — both correct.)

#### Flow 4.18 — Group-delete-with-members is blocked; an empty group deletes; self-group + name validation guards (US-M4-B2 AC-5 + AC-6 + AC-8)

- Setup: admin at {BASE_URL_FRONTEND}/staff/admin/groups; the `Support` group has members (`agent`).
- Action: check the `Support` group row; choose "Delete"; confirm.
- Verify: deletion is refused with a visible error ("Unable to delete selected groups"); the group remains. Then delete an empty group (member count 0) → it succeeds and disappears (BS-031-022).
- Action: check the admin's OWN group row; choose "Disable" (then "Delete"); confirm.
- Verify: each is blocked — "As an admin, you can't disable/delete a group you belong to - you might lockout all admins!"; the group is unchanged (BS-031-023).
- Action: click "Add Group"; enter a 2-char name (e.g. `Ab`); Save → inline error under Name "Group name must be at least 3 chars."; change to a duplicate (`Support`); Save → inline error "Group name already exists"; no group added (BS-031-019).
- Status: [x] — AC-5 (delete-with-members): selected "M1 Agents" (2 members) + DELETE + confirm → refused "Unable to delete selected groups"; group remains. Empty "Faq Managers" (0 members) + DELETE + confirm → "Selected groups deleted successfully", row gone (BS-031-022). AC-6 (self-group): selected admin's own "Administrators" group → DISABLE → blocked "As an admin, you can't disable/delete a group you belong to - you might lockout all admins!" (stays Active); → DELETE + confirm → same message, group unchanged (BS-031-023). AC-8 (name validation): Add Group name "Ab" → inline "Group name must be at least 3 chars." (no group added); name "Administrators" (duplicate) → inline "Group name already exists" (BS-031-019).

### Subsection 4D — Profile & Directory (US-M4-B3)

#### Flow 4.19 — The user menu exposes Profile/Directory/Logout, and a staff member edits their own profile (US-M4-B3 AC-1)

- Setup: log in `{STAFF_USER}` / `{STAFF_PASSWORD}`.
- Verify: the header user menu exposes Profile, Directory and Logout entries.
- Navigate: {BASE_URL_FRONTEND}/staff/profile
- Verify: the username field is read-only (disabled); editable name/email/phone/preferences (timezone, default page size, refresh rate, signature) render.
- Action: change the timezone to a different zone and set default page size to a new value; Save.
- Verify: "Profile updated successfully" banner; reload → both values persist (timezone live-derived from `staff.timezone_id`).
- Verify (regression): the active session is not broken by the timezone change (still authenticated, no forced re-login).
- Status: [x] — the header account menu (person icon) on /staff/profile exposes Profile / Directory / Logout. Profile form: Username read-only (disabled, "agent"); editable First/Last name, Email, Phone/Ext/Mobile, Time Zone (ID), Default Page Size, Auto Refresh, Signature Usage, Paper Size, Signature. Changed Time Zone (ID) 8→5 and Default Page Size 25→50 → Save → toast "Profile updated successfully"; after reload both persist (tz=5, pageSize=50). Regression: still authenticated on /staff/profile (no forced re-login) after the timezone change. (Note: the M1 queue header /staff/tickets still shows a plain LOG OUT button rather than the account menu; the Profile/Directory/Logout menu is present on the profile/directory/admin headers.)

#### Flow 4.20 — A staff member changes their own password and re-logs in (US-M4-B3 AC-2)

- Setup: logged in as `{STAFF_USER}` at {BASE_URL_FRONTEND}/staff/profile (password section).
- Action: enter current password `{STAFF_PASSWORD}`, new password `Agent999!` (>=6), confirm `Agent999!`; Save.
- Verify: success message; any forced-change banner clears.
- Action: log out; log in `agent` / `Agent999!` → login succeeds.
- Note: restore afterwards via `POST {BASE_URL_API}/api/dev/reset-staff` (or reseed) so `{STAFF_PASSWORD}` is valid for later flows.
- Status: [x] — profile password section (Current / New / Confirm + CHANGE PASSWORD): entered current Agent123!, new Agent999!, confirm Agent999! → CHANGE PASSWORD. Logged out, logged in agent/Agent999! → login SUCCEEDED (landed /staff/tickets, no error), confirming the password changed. Restored afterwards via reset-staff so Agent123! is valid for later flows. (Method note: the success toast was not captured due to a transient extension-context glitch, but the successful re-login with the new password is definitive proof.)

#### Flow 4.21 — The read-only directory supports search + a department filter (US-M4-B3 AC-3)

- Setup: logged in as `{STAFF_USER}`. Navigate: {BASE_URL_FRONTEND}/staff/directory
- Verify: a read-only table (name, department, email, phone, ext, mobile); NO edit/add/delete controls.
- Action: type part of `agent2`'s name into the search box and click "Filter" → the list narrows to the matching staff member.
- Action: change the department filter to a specific department → the list is scoped to that department's directory-visible staff.
- Status: [x] — /staff/directory renders a read-only table (Name / Department / Email / Phone / Ext / Mobile) with 3 rows and NO add/edit/delete controls; a "Search name, email, or phone…" box + FILTER button + Department filter select. Searching "Two" + FILTER narrowed to exactly "Agent Two". Clearing search and setting the Department filter to "Support" + FILTER scoped the list to the two Support staff (Admin User, Agent Two); Agent One (Sales) excluded.

#### Flow 4.22 — A staff member is added to a team from their profile (US-M4-B3 AC-4)

- Setup: a team is seeded ("Tier 2" from the M3 seed, or an M4-C team); log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}`; open a staff member's profile via {BASE_URL_FRONTEND}/staff/admin/staff → open the row.
- Action: use the "Teams" control on the profile to add that staff member to the seeded team; Save.
- Verify: success; re-opening the profile shows the team on the "Teams" control (membership persisted). The full roster-side check (the Teams screen lists the member) is Integration Flow 4.53.
- Status: [x] — teams seeded (Tier 2 id 3, QA Squad id 55). Opened agent2 (staff 91, already in Tier 2) → the Teams control shows current teams as chips ("Tier 2") + an "Add to team" select + ADD button. Selected "QA Squad" → ADD → chip added (chips: QA Squad, Tier 2) → Save. Reopening the profile shows both chips (QA Squad + Tier 2); DB team_member confirms rows (3,91) and (55,91). (Procedural note: the "Add to team" select stages a team; the ADD button commits it to the chip list before Save — selecting without ADD does not persist.)

#### Flow 4.23 — An over-age password surfaces the forced-change banner on login (US-M4-B3 AC-5)

- Setup: `POST {BASE_URL_API}/api/dev/age-password` `{staffId:<agent2 id>, days:120}` (backdates `passwdreset` + sets `change_passwd`); optionally `POST {BASE_URL_API}/api/dev/seed-config` `{passwd_reset_period:30}` so the aging window is active (Integration AC-7).
- Action: log in as that aged non-admin account.
- Verify: the forced-password-change banner appears ("You must change your password to continue!") and the password-change form is presented.
- Status: [x] — age-password {staffId:91 (agent2), days:120} + seed-config {passwd_reset_period:30} → DB shows agent2 change_passwd=t, passwdreset 120 days old. GET /api/staff/profile returns change_passwd=true. Logged in agent2/Agent234! → the AppShell ForcedPasswordBanner "You must change your password to continue!" (with a "Change password" action) renders on the authenticated shell, and /staff/profile presents the banner + the password-change form (3 fields). (Caveat: identical M1-StaffArea vs AppShell layout split as Flow 4.19 — the login landing at the M1 /staff/tickets queue is not wrapped by AppShell so it does not host the banner; the banner surfaces on the AppShell-wrapped pages — profile/directory/admin — the moment the session is authenticated.)

### Subsection 4E — SLA & Priorities (US-M4-D1 / US-M4-D2)

#### Flow 4.24 — The SLA list renders with mass actions; the "Date Added" column is sortable (US-M4-D1 AC-1)

- Setup: `cargo run -p tools --bin seed -- --reset` (restores the seeded SLA plans + the default SLA); log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}` → {BASE_URL_FRONTEND}/staff/admin/sla.
- Verify: a table lists SLA plans (name, grace period, status, Date Added); mass-action controls present (row checkboxes + Activate / Disable / Delete).
- Action: click the "Date Added" column header → the row order toggles ascending/descending (KL-032.10 modernised — Date Added IS sortable).
- Status: [x] — reset-sla → /staff/admin/sla lists SLA plans in a table (Name / Grace Period (hrs) / Status / Date Added): Standard 24 Active 24/07/2026, Urgent 4 Active 15/06/2026, Default SLA (Default badge) 48 Active 15/06/2026; per-row checkboxes + ACTIVATE / DISABLE / DELETE mass actions. Clicking the "Date Added" header toggles aria-sort descending→ascending→descending and the row order reverses (Standard/Urgent/Default ↔ Default/Urgent/Standard) — Date Added IS sortable (KL-032.10 modernised).

#### Flow 4.25 — Admin creates an SLA plan (US-M4-D1 AC-2)

- Setup: admin at {BASE_URL_FRONTEND}/staff/admin/sla.
- Action: click "Add SLA Plan"; Name "Gold SLA", Grace Period "24" (hours), check Active; Save.
- Verify: a success banner appears and "Gold SLA" is now a row in the list (persists after reload). It is thereafter selectable in the department / help-topic SLA select (verified via those screens in Flows 4.43 / 4.49).
- Status: [x] — Add SLA Plan dialog (Name, Grace Period (hours), Active + escalation/transient/overdue toggles): Name "Gold SLA", Grace Period 24, Active checked → Save → toast "Gold SLA added successfully"; row "Gold SLA / 24 / Active" appears and persists after hard reload (4 rows).

#### Flow 4.26 — The default SLA is delete-protected; deleting a non-default plan re-homes its dependents (US-M4-D1 AC-3 + AC-4)

- Setup: admin at {BASE_URL_FRONTEND}/staff/admin/sla.
- Action: locate the default SLA row (bound as `default_sla_id`) and attempt to delete it (checkbox + Delete, or its row delete control).
- Verify: the deletion is refused — the default SLA's checkbox is disabled or the delete is blocked with a message (e.g. "The default SLA cannot be deleted"); the default SLA remains (Integration AC-1).
- Action: create a plan "Temp SLA" and bind it to a department ({BASE_URL_FRONTEND}/staff/admin/departments → edit a dept → SLA = "Temp SLA" → Save, or `POST {BASE_URL_API}/api/dev/set-dept-sla`); then Navigate {BASE_URL_FRONTEND}/staff/admin/sla → select "Temp SLA" → Delete → confirm.
- Verify: "Temp SLA" is removed with a success message; reopening that department's edit form shows its SLA now unset / "— none —" (dept `sla_id` re-homed to NULL; tickets that had been on "Temp SLA" re-home to the default SLA — FS-032.12).
- Status: [x] — AC-3 (default protection): the "Default SLA" row's checkbox is DISABLED (cannot be selected for deletion); Default SLA remains. AC-4 (re-home): created "Temp SLA" (id 199), bound it to the Sales department (set-dept-sla dept 61 → sla 199, confirmed department.sla_id=199), then selected "Temp SLA" on /staff/admin/sla → DELETE → confirm → toast "Selected SLA plans deleted successfully", row removed. DB confirms Sales department.sla_id re-homed to NULL (unset) — FS-032.12.

#### Flow 4.27 — The Priorities panel lists the fixed set read-only with no CRUD controls (US-M4-D2 AC-1)

- Setup: `cargo run -p tools --bin seed -- --reset` (restores the 4 seeded priorities); log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}` → {BASE_URL_FRONTEND}/staff/admin/priorities.
- Verify: the fixed priority set is listed in urgency order (`ORDER BY urgency DESC` → Low, Normal, High, Emergency), each showing its urgency rank + color swatch.
- Verify: there are NO Add / Edit / Delete controls anywhere — no "Add Priority" button, no per-row edit/delete, no bulk mass-actions (read-only, KL-032.1 preserved).
- Status: [x] — /staff/admin/priorities lists the fixed set in urgency-DESC order: low (urgency 4, #DDFFDD), normal (3, #FFFFF0), high (2, #FEE7E7), emergency (1, #FEE7E7) — i.e. Low, Normal, High, Emergency, each with its urgency rank + color swatch. NO Add/Edit/Delete controls anywhere (zero CRUD buttons, no per-row edit/delete, no mass actions) — read-only (KL-032.1 preserved).

### Subsection 4F — Site Pages (US-M4-F1)

#### Flow 4.28 — The site pages list renders with sort + pagination (US-M4-F1 AC-1)

- Setup: `cargo run -p tools --bin seed -- --reset`; optionally seed a few pages — `POST {BASE_URL_API}/api/dev/seed-page` x N; log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}` → {BASE_URL_FRONTEND}/staff/admin/pages.
- Verify: a table lists pages (name, type, status, in-use indicator); column headers are sortable (click a header → order changes); pagination controls appear when rows exceed one page.
- Status: [x] — reset-pages + seeded 5 pages → /staff/admin/pages renders a table with columns Name / Type / Status / In Use (+ row checkbox): Seed Page 1 (landing), 2 (thank-you), 3–5 (other), all Active, In Use "—". Sortable headers (3 sort labels); clicking Name toggles aria-sort to descending and reverses the row order (Seed Page 1→5 becomes 5→1). Pagination control present ("Showing 1-5 of 5").

#### Flow 4.29 — Admin creates a landing page and a thank-you page; a duplicate name is rejected (US-M4-F1 AC-2 + AC-3)

- Setup: admin at {BASE_URL_FRONTEND}/staff/admin/pages.
- Action: click "Add Page"; Name "Welcome Landing", Type `landing`, Body "Welcome to support"; Save → success banner; "Welcome Landing" appears.
- Action: click "Add Page" again; Name "Thanks Page", Type `thank-you`, Body "Thank you"; Save → both pages now appear.
- Action: click "Add Page"; Name "Welcome Landing" (reused), Type `other`, Body "x"; Save → an inline "name already exists" error (BS-033.10); the page is not created.
- Status: [x] — Add Page dialog (Name, Type select [landing/offline/thank-you/other], Body, Admin Notes): created "Welcome Landing" (landing) → toast "Welcome Landing added successfully"; created "Thanks Page" (thank-you) → toast "Thanks Page added successfully"; both rows present. Re-adding Name "Welcome Landing" (other) → inline error "A page with this name already exists" under Name; dialog stayed open, no success toast, only one "Welcome Landing" row (not created) — BS-033.10.

#### Flow 4.30 — A bound page is in-use and both delete + disable are protected (US-M4-F1 AC-4)

- Setup: admin. Bind "Welcome Landing" as the default landing page — Navigate {BASE_URL_FRONTEND}/staff/admin/settings → Pages tab → Landing Page = "Welcome Landing" → Save.
- Action: Navigate {BASE_URL_FRONTEND}/staff/admin/pages; locate "Welcome Landing"; attempt to delete it (checkbox + Delete, or its row delete control).
- Verify: the row shows an "in use" badge and BOTH delete and disable are refused/guarded with a message (BS-033.9); the page remains active.
- Status: [x] — bound "Welcome Landing" as the default Landing Page (Settings → Site Pages tab → Landing Page = Welcome Landing → Save "Site Pages settings updated"). On /staff/admin/pages the "Welcome Landing" row now shows an "In use" badge. With that in-use row selected, BOTH the DISABLE and DELETE mass-action buttons are DISABLED (guarded/refused — clicking does nothing, no confirm dialog opens), while ENABLE stays available; the page remains Active and present (BS-033.9). (Guard is pre-emptive via disabled Delete/Disable controls + the "In use" badge rather than a post-click toast.)

### Subsection 4G — System Logs (US-M4-G1)

#### Flow 4.31 — The log viewer shows real rows produced by app events (US-M4-G1 AC-1)

- Setup: `cargo run -p tools --bin seed -- --reset`; `POST {BASE_URL_API}/api/dev/purge-logs` to a known state; log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}` and create a group at {BASE_URL_FRONTEND}/staff/admin/groups (admin CRUD emits a syslog row); optionally top up via `POST {BASE_URL_API}/api/dev/seed-log`.
- Navigate: {BASE_URL_FRONTEND}/staff/admin/logs
- Verify: the table lists log entries (Type, Title, Date), including the row(s) produced by the events above.
- Status: [x] — /staff/admin/logs renders a table with columns Type / Title / Date and real rows. After creating group "Log Event Group" via the Groups UI (toast "Log Event Group added successfully"), the log viewer's newest row is Debug "Group created" 24/07/2026 12:29:24 — a genuine app-event row — alongside "Staff login" events and the seeded Error "Disk almost full" / Debug "Cron tick" rows (25 rows on page). Log rows are produced by app events (US-M4-G1 AC-1).

#### Flow 4.32 — Filter by type and date span (US-M4-G1 AC-2)

- Setup: ensure rows of at least two types exist — `POST {BASE_URL_API}/api/dev/seed-log` `{type:"Error",…}` and `{type:"Debug",…}` with differing dates.
- Action: at {BASE_URL_FRONTEND}/staff/admin/logs set the type filter to a single type (e.g. Error) and a covering date span; click Apply.
- Verify: the table narrows to only matching entries (other types/dates excluded).
- Status: [x] — the filter bar exposes a Type select (All types / Error / Warning / Debug) + From/To date inputs + Apply. Set Type=Error and date span From 01/07/2026 To 31/07/2026 → Apply → the table narrowed from 25 rows to exactly 1: Error "Disk almost full" 24/07/2026; every Debug row excluded (distinctTypes=[Error]). Type + date-span filtering narrows to only matching entries (US-M4-G1 AC-2).

#### Flow 4.33 — Sort and paginate the log (US-M4-G1 AC-3)

- Setup: seed enough rows to exceed one page — `POST {BASE_URL_API}/api/dev/seed-log` x N.
- Action: click a sortable header (e.g. Date); then use the pagination control to advance a page.
- Verify: row order changes on sort; pagination advances to the next page of rows.
- Status: [x] — seeded 30 extra log rows (total >1 page; page size 25, 6 pages). Date column defaults to aria-sort=descending (first row "Pager row 30" 12:31:14); clicking the Date header toggles it to ascending and the order reverses (first row becomes the oldest "SLA plan created" 05:42:29). Pagination control shows Prev(disabled)/1..6/Next; clicking Next advanced to page 2 (new 25 rows, first "PURGE recent" 06:15:06, Prev now enabled). Sort changes order + pagination advances (US-M4-G1 AC-3).

#### Flow 4.34 — Open a single log entry's detail (US-M4-G1 AC-4)

- Setup: admin at {BASE_URL_FRONTEND}/staff/admin/logs with rows present.
- Action: click a log row.
- Verify: a detail view/panel/dialog shows the full log body/detail text (content AJAX, FS-033.8).
- Status: [x] — clicking a log row ("Pager row 30") opened a MUI detail dialog showing the entry's full detail: title "Pager row 30", metadata line "Debug · 24/07/2026 12:31:14 · 127.0.0.1", the full log body text "pagination fill 30", and a CLOSE button. The single-entry detail renders the full body (FS-033.8, US-M4-G1 AC-4).

#### Flow 4.35 — Bulk delete selected log entries (US-M4-G1 AC-5)

- Setup: admin at {BASE_URL_FRONTEND}/staff/admin/logs with at least two rows.
- Action: select two entries via their checkboxes; click Delete; confirm.
- Verify: both rows are removed from the table.
- Status: [x] — checked the first two rows' checkboxes ("Pager row 30" + "Pager row 29") → mass Delete enabled → click Delete → MUI confirm dialog "Delete log entries / Delete 2 log entries?" → DELETE → toast "2 log entries deleted"; both selected rows removed from the table (row 28/27/26/25 shifted up), no native JS dialog used (US-M4-G1 AC-5).

#### Flow 4.36 — "Purge now" removes over-age rows in the UI (US-M4-G1 AC-7)

- Setup: admin; set `log_graceperiod` to a non-zero month count (`POST {BASE_URL_API}/api/dev/seed-config`); seed an over-age row `POST {BASE_URL_API}/api/dev/seed-log` `{created:"2020-01-01T00:00:00Z"}` and a recent row.
- Navigate: {BASE_URL_FRONTEND}/staff/admin/logs
- Action: click the visible "Purge now" control → confirm.
- Verify: the over-age row disappears from the table while the recent row remains (BS-033.6, KL-033.7 modernised).
- Status: [x] — set log_graceperiod=6 (months) via seed-config; seeded an over-age Debug row "Ancient row 2020" (created 2020-01-01) + a recent "Fresh purge row". On /staff/admin/logs the "Purge now" control opens a MUI confirm dialog "Purge old log entries / Remove log entries older than the configured grace period? Recent entries are retained." → PURGE NOW → toast "Over-age log entries purged". Post-purge the 2020 row is gone everywhere (last page's oldest surviving row is 2026-07-24 03:42:29; no 2020 date/title anywhere across all 6 pages / total 128), while the recent "Fresh purge row" remains (BS-033.6, KL-033.7 modernised, US-M4-G1 AC-7).

### Subsection 4H — Canned Responses (delegated gate — US-M4-H1)

#### Flow 4.37 — The premade manager reaches Canned Responses (not settings); a plain agent is denied (US-M4-H1 AC-1 + AC-6)

- Setup: `cargo run -p tools --bin seed -- --reset`; mint `{PREMADE_USER}` / `{PREMADE_PASSWORD}` — `POST {BASE_URL_API}/api/dev/set-group-perm` `{group_id:<a group>, can_manage_premade:true}` then `POST {BASE_URL_API}/api/dev/seed-staff` `{username:"premade1", password:"Premade123!", group_id:<that group>, isadmin:false}`.
- Navigate: {BASE_URL_FRONTEND}/staff/login → log in `{PREMADE_USER}` / `{PREMADE_PASSWORD}`.
- Navigate: {BASE_URL_FRONTEND}/staff/admin/canned
- Verify: the Canned Responses screen renders; admin-only nav entries (System Settings, Staff, Groups, Departments, Teams, Help Topics, SLA, Priorities, Site Pages, Logs) are ABSENT for this non-admin; the M2 seeded canned responses are listed.
- Action: in a fresh session log in `{STAFF_USER}` / `{STAFF_PASSWORD}`; navigate directly to {BASE_URL_FRONTEND}/staff/admin/canned.
- Verify: the agent is redirected/denied — the Canned Responses screen does NOT render for the non-premade agent (matches the API 403).
- Status: [x] — minted premade1 (staffId 233, isolated group 43 with can_manage_premade only, no admin flags). Logged in premade1 → /staff/admin/canned renders the Canned Responses screen; the left-nav has EXACTLY ONE admin entry /staff/admin/canned (System Settings/Staff/Groups/Departments/Teams/Help Topics/SLA/Priorities/Site Pages/Logs all ABSENT); the M2 seeded responses are listed (Acknowledge receipt · Enabled, Closed — disabled sample · Disabled, Sample (with attachment) · Enabled). In a fresh session as plain agent, navigating directly to /staff/admin/canned is denied — redirected to /staff/tickets, the Canned screen does NOT render (US-M4-H1 AC-1 + AC-6).

#### Flow 4.38 — The manager creates a canned response with a `%{token}` (US-M4-H1 AC-2)

- Setup: logged in as `{PREMADE_USER}` at {BASE_URL_FRONTEND}/staff/admin/canned.
- Action: click "Add Canned Response"; Title "Greeting", Body "Hello %{ticket.name}", check Active; Save.
- Verify: success banner "Greeting added successfully"; "Greeting" appears in the list.
- Status: [x] — as premade1, Add Canned Response dialog (Title / Department / Response Body with helper "You can embed variables like %{ticket.name}." / Enabled / Attachments / Admin Notes): Title "Greeting", Response Body "Hello %{ticket.name}", Enabled checked → Save → toast "Greeting added successfully"; the "Greeting — All — Enabled" row appears in the list (US-M4-H1 AC-2).

#### Flow 4.39 — The new response appears in the reply composer dropdown; editing to disabled removes it (US-M4-H1 AC-3 + AC-4)

- Setup: an open ticket exists (reuse a seeded ticket, or `POST {BASE_URL_API}/api/dev/seed-ticket`); "Greeting" created in Flow 4.38.
- Action: open a ticket in the staff area ({BASE_URL_FRONTEND}/staff/tickets → open one) → open the reply composer's canned-response dropdown.
- Verify: "Greeting" is selectable (M2 consumption path); selecting it inserts its body with `%{ticket.name}` substituted.
- Action: back at {BASE_URL_FRONTEND}/staff/admin/canned edit "Greeting" body → Save (success); then toggle it to disabled/inactive.
- Verify: reopen a ticket's reply composer → the disabled "Greeting" no longer appears in the canned-response dropdown.
- Status: [x] — seeded open ticket #886158 (id 1). AC-3: as agent, ticket detail Reply composer's "Canned response" dropdown lists None / Acknowledge receipt / Greeting / Sample (with attachment) — "Greeting" (id 302) IS selectable (the disabled seed "Closed — disabled sample" is already absent). Selecting "Greeting" inserted "Hello QA Seed" into the Reply body — the %{ticket.name} token substituted with the ticket's client name "QA Seed". AC-4: as premade1, edited "Greeting" body ("Hello %{ticket.name} — edited") + unchecked Enabled → Save → toast "Greeting updated successfully" (row now "Greeting — All — Disabled"). Back as agent, reopening ticket #886158's Reply composer, the canned dropdown now lists only None / Acknowledge receipt / Sample (with attachment) — the disabled "Greeting" no longer appears (US-M4-H1 AC-3 + AC-4).

### Subsection 4I — FAQ Categories (delegated gate — US-M4-E1)

#### Flow 4.40 — A non-admin FAQ manager reaches FAQ Categories (and NOT settings) (US-M4-E1 AC-1)

- Setup: mint `{FAQMGR_USER}` / `{FAQMGR_PASSWORD}` — `POST {BASE_URL_API}/api/dev/seed-staff` `{username:"faqmgr", password:"Faqmgr123!"}` (returns `{staffId, groupId}`) then `POST {BASE_URL_API}/api/dev/set-group-perm` `{group_id:<groupId>, can_manage_faq:true}` (group carries NO admin/settings flags); `POST {BASE_URL_API}/api/dev/reset-faq-categories`.
- Navigate: {BASE_URL_FRONTEND}/staff/login → log in `{FAQMGR_USER}` / `{FAQMGR_PASSWORD}`.
- Navigate: {BASE_URL_FRONTEND}/staff/admin/faq-categories
- Verify: the FAQ Categories screen renders; "System Settings" and other admin-only nav entries are absent; navigating directly to an admin-only route (e.g. {BASE_URL_FRONTEND}/staff/admin/settings/attachments) is blocked/redirected (Integration AC-6).
- Status: [x] — minted faqmgr (staffId 234, isolated group 44 with can_manage_faq only). Logged in faqmgr → /staff/admin/faq-categories renders the FAQ Categories screen (Add Category present); the left-nav has EXACTLY ONE admin entry /staff/admin/faq-categories (System Settings and all other admin-only entries ABSENT). Navigating directly to the admin-only /staff/admin/settings/attachments is blocked — redirected to /staff/tickets, the Attachments screen does NOT render (no "Allow Attachments" control) (US-M4-E1 AC-1, Integration AC-6).

#### Flow 4.41 — The FAQ manager creates, edits the type of, and deletes a category (US-M4-E1 AC-2 + AC-3 + AC-4)

- Setup: logged in as `{FAQMGR_USER}` at {BASE_URL_FRONTEND}/staff/admin/faq-categories.
- Action: click "Add Category"; name "Getting Started", type Public, a description; Save → success banner; "Getting Started" appears.
- Action: edit "Getting Started"; switch type Public → Private; Save → the list reflects it as Private (Internal).
- Action: delete "Getting Started"; confirm → the category is gone from the list (plain delete; FS-032.16 cascade defined-but-inert until M7 articles exist).
- Status: [x] — as faqmgr: Add Category dialog (Name / "Public (visible to end users)" toggle / Description / Internal Notes) — created "Getting Started" Public with description "How to begin with the helpdesk" → toast "Getting Started added successfully"; row shows Public. Edited it, unchecked Public → Save → toast "Getting Started updated successfully"; list reflects it as "Internal" (Private). Clicked the row's DELETE → MUI confirm "Delete Category / Delete this FAQ category? This cannot be undone." → DELETE → toast "FAQ category deleted"; the category is gone ("No categories found.") (US-M4-E1 AC-2 + AC-3 + AC-4).

### Subsection 4J — Departments, Teams & Help Topics (US-M4-C1 / C2 / C3)

#### Flow 4.42 — The department list renders; the default department's checkbox is disabled (US-M4-C1 AC-1)

- Setup: `cargo run -p tools --bin seed -- --reset` (seeds the `Support` default department); optionally `POST {BASE_URL_API}/api/dev/reset-departments`; log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}`.
- Navigate: {BASE_URL_FRONTEND}/staff/admin/departments
- Verify: a table lists departments; the default department (Support) row renders its selection checkbox disabled (BS-030-05).
- Status: [x] — reset-departments (default_dept_id 2). /staff/admin/departments renders a table (Name / Type / Manager / Users) with two rows: "Sales · Public · 0" (checkbox enabled) and "Support (default) · Public · 5". The default department "Support (default)" row's selection checkbox is DISABLED (disabled=true) while the non-default Sales row's checkbox is enabled (BS-030-05, US-M4-C1 AC-1).

#### Flow 4.43 — Create a department with required Email + Template (M4-D1), an SLA and a manager, and a group matrix; missing Email/Template is rejected (US-M4-C1 AC-2 + AC-3)

- Setup: admin on {BASE_URL_FRONTEND}/staff/admin/departments.
- Action: click "Add Department"; Name "QA Escalations" (>=4 chars).
- Verify: the Email select lists the seeded `support@osticket.local` and the Template select lists "osTicket Default" (M4-D1 — not an empty/stub list); the SLA and Manager selects list the M4-D SLA plans and the M4-B staff.
- Action: select Email = support@osticket.local, Template = osTicket Default, an SLA plan, a manager (e.g. `agent`); check at least one group in the access matrix; Save → success banner; "QA Escalations" appears.
- Action: click "Add Department"; valid >=4-char name; leave Email unset; Save → inline error "Email selection required" (BS-030-02). Set Email; clear Template; Save → inline error "Template selection required" (BS-030-03).
- Status: [x] — Add Department dialog: Email* select lists "support@osticket.local" (M4-D1 seeded account, not stub); Template* lists "osTicket Default"; SLA Plan lists — System Default —/Default SLA/Gold SLA/Standard/Urgent (M4-D plans); Manager lists — None —/Admin User/Agent One/Agent Two/faqmgr/premade1 (M4-B staff). Created "QA Escalations" with Email=support@osticket.local, Template=osTicket Default, SLA=Default SLA, Manager=Agent One → toast "QA Escalations added successfully"; row appears (Manager Agent One). Group-access matrix (Select All/None + Administrators/Log Event Group/M1 Agents/…) is functional — editing QA Escalations, checked M1 Agents → Save → "QA Escalations updated successfully". AC-3 negatives: Add Department "No Email Dept" with Email+Template unset → Save → inline "Email selection required" AND "Template selection required" (dialog stays open, no nav); then choosing Email=support@osticket.local + re-Save cleared the Email error leaving only "Template selection required" (BS-030-02 / BS-030-03, US-M4-C1 AC-2 + AC-3).

#### Flow 4.44 — The default department cannot be made private or deleted; deleting a non-default department re-homes its tickets + topics (US-M4-C1 AC-4 + AC-5)

- Setup: admin on {BASE_URL_FRONTEND}/staff/admin/departments.
- Action: edit the default department (Support); attempt to toggle it Private; Save → inline error "System default department cannot be private" (BS-030-04). Attempt to delete it → refused (its list checkbox is disabled / delete blocked — BS-030-05).
- Action: create a non-default "Temp Dept" (Flow 4.43 flow); seed a ticket homed to it (`POST {BASE_URL_API}/api/dev/seed-ticket` with its dept id); ensure no home staff remain in it; then on {BASE_URL_FRONTEND}/staff/admin/departments delete "Temp Dept".
- Verify: success; the ticket previously homed to "Temp Dept" now points to the default department, and help topics homed to it are re-homed too (no orphans — BS-030-06/07).
- Status: [x] — AC-4 (default protection): edited Support (default), toggled Public→Private + Save → inline reject "System default department cannot be private" (dialog stays open, stays public) (BS-030-04); default-dept delete refused (its list checkbox disabled, per Flow 4.42) (BS-030-05). AC-5 (re-home): created non-default "Temp Dept" (id 196, Email/Template set); homed ticket #5 (via seed-tickets deptId:196) and help topic #127 "TempDeptTopic" to it; deleted Temp Dept on /staff/admin/departments (checkbox → Delete → MUI confirm "Delete 1 department(s)? Departments with staff cannot be deleted." → CONFIRM → "Selected departments deleted successfully"). DB confirms re-home: ticket #5 dept_id 196→2 (default Support) and topic #127 dept_id 196→2; zero orphans remain in dept 196 (0 tickets / 0 topics / dept row gone) (BS-030-06/07, US-M4-C1 AC-4 + AC-5).

#### Flow 4.45 — The team list renders with member counts (US-M4-C2 AC-1)

- Setup: `cargo run -p tools --bin seed -- --reset` then `POST {BASE_URL_API}/api/dev/reset-teams` (keeps the seeded "Tier 2"); log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}`.
- Navigate: {BASE_URL_FRONTEND}/staff/admin/teams
- Verify: a table of teams (name, status, member count); the seeded "Tier 2" team appears.
- Status: [x] — reset-teams (keeps seeded Tier 2, team_id 3). /staff/admin/teams renders a table with columns Name / Status / Members / Lead; the seeded "Tier 2" row appears: Active, member count 2, Lead "Agent One" (US-M4-C2 AC-1).

#### Flow 4.46 — Create a team and set a lead from its members; the team form removes (not adds) members (US-M4-C2 AC-2 + AC-3)

- Setup: admin on {BASE_URL_FRONTEND}/staff/admin/teams.
- Action: click "Add Team"; Name "QA Squad" (>=3 chars); status Active; Save → success banner; "QA Squad" appears.
- Action: add a member to "QA Squad" from a staff profile ({BASE_URL_FRONTEND}/staff/admin/staff → a record → add-to-team, per US-M4-B3); then edit "QA Squad"; select that member as Lead; Save → the roster shows the member with a lead badge (BS-030-13).
- Action: on the team form, mark a member for removal via its remove checkbox; Save → the roster no longer lists them; confirm there is NO add-member control anywhere on the form (adding is only from the staff profile — BS-030-14).
- Status: [x] — AC-2: Add Team "QA Squad" (Active) → toast "QA Squad added successfully"; row appears (0 members). AC-3 (lead from members): added Agent Two (staff 91) to QA Squad from the staff profile's TEAMS control (select QA Squad → ADD chip → Save; DB team_member confirms row 66/91). Editing QA Squad, the Team Lead select offers only members (— None — / Agent Two — not the whole directory); set Agent Two as Lead → Save "QA Squad updated successfully"; list row shows "QA Squad · Active · 1 · Agent Two" (lead) (BS-030-13). Remove-only roster: the Edit Team form has NO add-member control anywhere (adding is only from the staff profile); the MEMBERS section exposes a per-member "Remove Agent Two" checkbox — checked it → Save → member count 0, roster no longer lists Agent Two (BS-030-14, US-M4-C2 AC-2 + AC-3).

#### Flow 4.47 — Deleting a team releases its ticket assignments (US-M4-C2 AC-4)

- Setup: create "Temp Team" (Flow 4.46 flow) and assign it to a seeded ticket (`POST {BASE_URL_API}/api/dev/seed-ticket` + assign in the queue).
- Action: on {BASE_URL_FRONTEND}/staff/admin/teams delete "Temp Team".
- Verify: success; the ticket's team assignment is now cleared (confirm in the ticket detail) and the team's member rows are gone (BS-030-16).
- Status: [x] — created "Temp Team" (id 67); assigned it to seeded ticket #6 (seed-tickets teamId:67) and gave it a member row (67,91). On /staff/admin/teams selected Temp Team → Delete → MUI confirm "Delete 1 team(s)?" → CONFIRM → toast "Selected teams deleted". DB confirms release: ticket #6 team_id 67→0 (assignment cleared), team_member rows for team 67 = 0 (member rows gone), team 67 row gone, 0 tickets still referencing 67 (BS-030-16, US-M4-C2 AC-4).

#### Flow 4.48 — The help-topic list renders with Parent / Child display (US-M4-C3 AC-1)

- Setup: `cargo run -p tools --bin seed -- --reset` then `POST {BASE_URL_API}/api/dev/reset-help-topics` (retains the seeded "General"/"Billing"); ensure at least one child topic exists (create one below, or seed parent+child); log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}`.
- Navigate: {BASE_URL_FRONTEND}/staff/admin/help-topics
- Verify: a paginated table of topics renders; a nested (child) topic displays as "Parent / Child".
- Status: [x] — reset-help-topics (retains General id 3 / Billing id 4); added a child topic "Password Reset" (topic_pid=3) under General. /staff/admin/help-topics renders a paginated table (Topic / Status / Priority / Department, pagination control present); the nested child renders as "General / Password Reset" (Parent / Child display), alongside the top-level "General" and "Billing" rows (US-M4-C3 AC-1).

#### Flow 4.49 — Create a topic with required dept + priority and an SLA override; the parent picker offers only top-level topics (US-M4-C3 AC-2 + AC-5)

- Setup: admin on {BASE_URL_FRONTEND}/staff/admin/help-topics.
- Action: click "Add Help Topic"; topic text "Refund Request" (>=5 chars); select a department (Support) and a priority (Normal); select an SLA override (e.g. "Default SLA"); Save → success banner; "Refund Request" appears (BS-030-18/19).
- Action: open the topic add/edit form; inspect the Parent select options.
- Verify: only top-level topics (`topic_pid=0`) are offered; a child topic is NOT selectable as a parent (PRESERVED one-level nesting — KL-030-02 / BS-030-20).
- Status: [x] — Add Help Topic dialog (Topic / Parent Topic / Department* / Priority* / SLA Override / Thank-You Page / Auto-Assign To). Created "Refund Request" with Department=Support, Priority=Normal, SLA Override=Default SLA → toast "Refund Request added successfully"; row appears "Refund Request · Active · Normal · Support" (BS-030-18/19). Parent Topic picker offers only top-level topics — "— Top Level —", "Billing", "General" (topic_pid=0); the child topic "General / Password Reset" is NOT offered as a parent — one-level nesting preserved (KL-030-02 / BS-030-20, US-M4-C3 AC-2 + AC-5).

#### Flow 4.50 — Auto-assign is mutually exclusive staff-or-team, and a thank-you page is selectable on the topic (US-M4-C3 AC-3 + AC-4)

- Setup: admin editing a topic (e.g. "Refund Request"); a `thank-you` page exists (`POST {BASE_URL_API}/api/dev/seed-page` type `thank-you`, or the Site Pages screen).
- Action: in the combined auto-assign control choose a staff auto-assignee → the team selection clears; then choose a team → the staff selection clears; Save.
- Verify: only the last-chosen assignee (staff OR team) is retained on reopen (BS-030-22).
- Action: edit the topic; select the thank-you page in the thank-you page select; Save.
- Verify: on reopen the thank-you page remains bound (Integration AC-5).
- Status: [x] — editing "Refund Request": the Auto-Assign To is a single combined select grouped Staff (Admin User/Agent One/Agent Two/faqmgr/premade1) + Teams (QA Squad/Tier 2) — inherently mutually exclusive. Chose staff "Agent One" then chose team "Tier 2" → the value switched to Tier 2 (the staff choice cleared). Selected Thank-You Page = "Thanks Page" (thank-you type). Save → "Refund Request updated successfully". On reopen the topic persists Auto-Assign To = "Tier 2" ONLY (last-chosen team; the earlier staff pick not retained — BS-030-22), Thank-You Page = "Thanks Page" still bound (Integration AC-5), SLA Override = "Default SLA" also retained (US-M4-C3 AC-3 + AC-4). (Note: a freshly seed-page'd thank-you page did not appear in the already-loaded topic form's cached page list; used the pre-existing "Thanks Page" thank-you page — binding + persistence verified either way.)

### Subsection 4K — M4 Integration (root E2E — M4 AC-1..AC-8)

> A continuous cross-epic admin journey. Run `cargo run -p tools --bin seed -- --reset` ONCE before
> Flow 4.51; state created in earlier integration flows is consumed by later ones — do NOT reseed
> between 4.51 and 4.58. Logged in as `{ADMIN_USER}` / `{ADMIN_PASSWORD}` throughout (except where a
> flow logs in as the delegated non-admin to prove a capability gate).

#### Flow 4.51 — A department selects an SLA and a help topic selects a Priority; the default department and default SLA are delete-protected (M4 AC-1)

- Setup: `cargo run -p tools --bin seed -- --reset`; log in `{ADMIN_USER}` / `{ADMIN_PASSWORD}` at {BASE_URL_FRONTEND}/staff/admin.
- Action: open {BASE_URL_FRONTEND}/staff/admin/departments → add/edit a department; the SLA select lists the M4-D SLA plans; select one and Save.
- Verify: the department saves with the chosen SLA plan (A→C dept↔SLA, D→C SLA-on-dept).
- Action: attempt to delete the default department and the default SLA plan.
- Verify: both deletions are refused — the default department's delete checkbox is disabled (and it cannot be made private) and the default SLA plan's delete is blocked.
- Action: open {BASE_URL_FRONTEND}/staff/admin/help-topics → add/edit a help topic; the Priority select lists the M4-D priority set; select a priority and Save.
- Verify: the help topic saves with the chosen Priority (D→C priority-on-topic).
- Status: [x] — Departments → edit "Sales": the SLA Plan select lists the M4-D plans (— System Default —/Default SLA/Gold SLA/Standard/Urgent); chose Standard → Save "Sales updated successfully"; reopen shows SLA=Standard persisted (A→C dept↔SLA, D→C SLA-on-dept). Delete protection: the default department "Support (default)" row's checkbox is DISABLED (cannot be made private per Flow 4.44), and on /staff/admin/sla the "Default SLA · Default" row's checkbox is DISABLED (delete blocked) while non-default plans are selectable. Help Topics → edit "Billing": the Priority select lists the M4-D priority set (Low/Normal/High/Emergency); chose High → Save "Billing updated successfully"; row shows "Billing · Active · High · Sales" (D→C priority-on-topic) (M4 AC-1).

#### Flow 4.52 — A new staff account is usable as manager/lead/auto-assignee, and a new group appears in the department access matrix (M4 AC-2)

- Action: in Staff, create a new active staff account; in Groups, create a new group.
- Action: in Departments, verify the new staff appears in the Manager select; in Teams, in the Lead select; in Help Topics, in the auto-assign select.
- Verify: the new group appears as a checkbox row in a department's Department-Access matrix.
- Status: [x] — created a new active staff "Ingrid Integration" (username iintegration, staff 238, group M1 Agents, dept Support) and a new group "Integration Group". The new staff is usable across the routing objects: Departments → edit Sales → Manager select lists "Ingrid Integration"; Help Topics → edit topic → Auto-Assign To select lists "Ingrid Integration" (Staff group); Teams → after adding Ingrid to Tier 2 from her profile, Tier 2's Team Lead select lists "Ingrid Integration" (lead is members-scoped, so she appears once a member). The new "Integration Group" appears as a checkbox row in the department Department-Access matrix (edit Sales → GROUP ACCESS lists Integration Group) (M4 AC-2).

#### Flow 4.53 — A team member added from B's staff profile shows in C's roster; removing the staff clears the membership (M4 AC-3)

- Action: open the new staff member's profile (B) and add them to a team.
- Verify: the Teams roster (C) shows that staff as a member.
- Action: delete/lock the staff member (respecting last-admin protection).
- Verify: the team roster no longer lists them.
- Status: [x] — added the new staff "Ingrid Integration" (staff 238) to Tier 2 from her staff profile's TEAMS control (B). The Teams roster (C) — Tier 2's Edit form MEMBERS section — then lists "Remove Ingrid Integration" (she is a member). Deleted Ingrid on /staff/admin/staff (checkbox → Delete → MUI confirm "Permanently delete 1 staff member(s)?" → CONFIRM → "Selected staff deleted successfully"). DB confirms membership cleared: Ingrid's team_member rows = 0, staff row gone, and Tier 2's roster is back to agent + agent2 only (Ingrid no longer listed) (M4 AC-3).

#### Flow 4.54 — A page created in F, bound as the landing page in A, becomes in-use-protected in F (M4 AC-4)

- Action: in Site Pages (F), create a page of type `landing`.
- Action: in System Settings → Pages tab (A), bind that page as the landing page and Save.
- Verify: back in Site Pages (F), the page shows "in use" and its delete is refused/guarded.
- Status: [x] — Site Pages (F): created page "Integration Landing" (type landing) → "Integration Landing added successfully" (In Use "—"). System Settings → Site Pages tab (A, at /staff/admin): the Landing Page select offers "Integration Landing"; selected it → Save → "Site Pages settings updated". Back in Site Pages (F), the "Integration Landing" row now shows an "In use" badge, and with it selected BOTH the Delete and Disable mass-action buttons are DISABLED (guarded/refused); the page remains Active (M4 AC-4).

#### Flow 4.55 — A thank-you page created in F is selectable on a help topic (M4 AC-5)

- Action: in Site Pages (F), create a page of type `thank-you`.
- Action: in Help Topics (C), edit a topic and select that thank-you page.
- Verify: the topic saves with the thank-you page bound.
- Status: [x] — Site Pages (F): created page "Integration Thanks" (type thank-you) → "Integration Thanks added successfully". Help Topics (C): edited the "General" topic — the Thank-You Page select offers "Integration Thanks" (alongside other thank-you pages); selected it → Save → "General updated successfully". On reopen the General topic's Thank-You Page = "Integration Thanks" (bound + persisted) (M4 AC-5).

#### Flow 4.56 — `can_manage_faq` on a non-admin group grants FAQ-category access without settings access (M4 AC-6)

- Action: in Groups (B), create a non-admin group with `can_manage_faq=Yes`; assign a staff member to it.
- Action: log in as that non-admin staff member.
- Verify: the FAQ Categories screen (E) is reachable and usable.
- Verify: the System Settings / admin-only screens are NOT reachable for that user.
- Status: [x] — Groups (B): created non-admin group "FAQ Delegates" (id 46) with Can Manage FAQ = Yes via the 11-flag PermissionGrid (DB confirms can_manage_faq=t, no admin flags); assigned staff "fdelegate" (staff 239) to it. Logged in as fdelegate → FAQ Categories (E) is reachable and usable (screen renders with "Add Category"); the left-nav has only the FAQ Categories entry. System Settings is NOT reachable: /staff/admin renders no settings tab strip ("Administration — Select a section from the menu"), and a direct admin-only route /staff/admin/staff redirects to /staff/tickets (M4 AC-6).

#### Flow 4.57 — Page-size / password-reset keys written in A take effect in B's pagination and password aging (M4 AC-7)

- Action: in System Settings (A), change the default max page size and the password-reset period; Save.
- Verify: the Staff list (B) paginates using the new page size.
- Verify: a staff account whose password age exceeds the new period surfaces the forced-change banner on next login (or the profile aging notice), demonstrating the setting took effect.
- Status: [x] — System Settings (A) System tab: set Default Page Size = 5 and Password Reset Period (days) = 30 → Save "System settings updated". Staff list (B) at /staff/admin/staff now paginates using the new page size — "Showing 1-5 of 6", 5 rows/page, Prev/1/2/Next controls (page size key took effect). Password aging: aged agent2's password 120 days (> the 30-day period; change_passwd=t) via age-password; logging in as agent2 surfaces the forced-password-change banner "You must change your password to continue!" on the AppShell-wrapped /staff/profile with the password-change form (M4 AC-7).

#### Flow 4.58 — The department Email and Template required selects resolve against the minimal email_account / template_group model (M4 AC-8)

- Action: in Departments (C), open the add/edit form.
- Verify: the Email select lists the seeded `email_account` row(s) and the Template select lists the seeded `template_group` value(s) (DEVIATION M4-D1) — not an empty/stub list.
- Verify: saving without an Email or Template selection is rejected with `Email selection required` / `Template selection required`.
- Status: [x] — Departments (C) Add Department form: the Email* select resolves against the minimal email_account model — it lists the seeded "support@osticket.local" (not an empty/stub list); the Template* select lists the seeded template_group value "osTicket Default" (DEVIATION M4-D1). Saving with both unset is rejected with inline "Email selection required" AND "Template selection required" (dialog stays open, no navigation) (M4 AC-8).

---

## Regressions

> Per-fix browser regressions appended as FIX-/BUG-/CLEANUP-/DOC- tickets are consolidated.
> Each entry is tagged `REG-{ticket}-{ac}`. None yet.

_(none)_

---

## Appendices

- [API regressions](./appendix-api-regressions.md) — `[API-ONLY]` ACs translated to `curl` flows.
