# US-M2-2 — Canned – Staff replies with attachment + canned response

- **ID**: US-M2-2
- **Type**: User Story
- **Parent**: EPIC-M2-D
- **Labels**: User Story, M2
- **Scope**: medium

## Spec References

- FS-022 (FS-022.14 canned consumption; BS-022.1 dept scope, BS-022.2 enabled-only, BS-022.15 unanswered)
- FS-040 (FS-040.11 substitution — via Epic C)
- FS-021 (FS-021.3 staff reply; FS-021.16 reply attachment)

## Context

Epic: EPIC-M2-D — Canned response consumption (also exercises Epic A's reply attachment hook).
The agent half of the M2 demo: an agent answers the ticket fast using a templated canned reply that
auto-fills a personalised body and carries a file. Produces the `R` reply + its attachment that the
client later downloads (root AC-5) and that triggers the notification email (root AC-4).

## Description

As an agent replying to a ticket, I can pick a canned response from a dropdown; the reply box fills
with the canned body already personalised to this ticket (e.g. its ticket number) and shows the
canned response's attachment ready to send. I can also attach my own file. Posting sends the reply
with its attachment(s) and re-flags the ticket as awaiting customer response.

## Impact

- Frontend (staff reply box: **canned-response dropdown** + **own-file input**; substituted body fills textarea; attachment chips)
- Backend (canned-fetch route returning substituted body + attachment list; reply route accepts canned id + own upload, binds attachments, marks unanswered)
- Database (`canned_response`, `canned_attachment`, `ticket_attachment`)
- Browser (desktop)

## Business Rules

- BS-022.1: only the ticket's department (or "All Departments") canned responses are offered (dept-scoped).
- BS-022.2: only **enabled** canned responses are offered; a disabled one is never listed.
- FS-022.14: selecting a canned response returns its body with `%{...}` substituted for this ticket and its attachments pre-attached.
- §3 (isanswered, ROADMAP M2 Decisions): **ANY staff reply — plain or canned-assisted — marks the ticket Answered (`isanswered=true`)**, like an M1 reply. Before any staff reply the ticket is Unanswered. BS-022.15's "mark unanswered" is the filter-driven SYSTEM auto-reply path → **DEFERRED to M5** (documented deviation). The staff UI shows an Answered / Unanswered badge on detail + queue. (Author divergence: stays the posting agent, not "SYSTEM (Canned Reply)" — pinned in EPIC-M2-D.)

## Regressions

- The M1 plain staff reply (US-M1-3, no canned, no attachment) must still post and appear. Verify after wiring the dropdown/attachment paths.

## Acceptance Criteria

### AC-1: The agent sees a clickable attachment chip on the client's original message. [BROWSER]
- Setup: `cargo run -p tools --bin seed -- --reset`; fixtures present (see Test Infrastructure); open a ticket via http://localhost:3702/open with `/tmp/qa-fixtures/invoice.pdf` (or use the carried root-AC-1 ticket).
- Navigate: log in agent / Agent123!; open that ticket.
- Verify: the customer's `M` message shows an **attachment chip** for the uploaded file.
- Status: [ ]

### AC-2: The reply box exposes a "Canned response" dropdown listing only enabled, dept-scoped responses. [BROWSER]
- Navigate: on the staff detail view from AC-1, scroll to the reply composer (same staff session).
- Verify: the reply area shows a **"Canned response" dropdown** that includes the seeded "Acknowledge receipt" (enabled, All-Departments) and does NOT include the seeded "Closed — disabled sample".
- Status: [ ]

### AC-3: Selecting a canned response fills the reply with a substituted body and shows its attachment chip. [BROWSER]
- Action: pick "Acknowledge receipt" from the dropdown.
- Verify: the reply textarea is populated with the canned body and the **`%{ticket.number}` token is substituted** to this ticket's number (no literal `%{...}`); a chip shows the canned response's seeded `policy.txt` attachment.
- Status: [ ]

### AC-4: Posting the canned reply appends an agent response carrying the substituted text + attachment, and the ticket shows Answered. [BROWSER]
- Setup: before replying, the ticket's badge reads **Unanswered**.
- Action: post the reply.
- Verify: the thread shows a new agent `R` entry with the substituted body and a `policy.txt` chip; the ticket's **Answered / Unanswered badge now reads "Answered"** (`isanswered=true`), like any staff reply (ROADMAP M2 Decisions §3). (BS-022.15's "mark unanswered" SYSTEM path is deferred to M5.)
- Status: [ ]

### AC-5: The agent can additionally attach their own file on a reply. [BROWSER]
- Action: in the reply box own-file input, choose `/tmp/qa-fixtures/note.png` (permitted type), type some text, post.
- Verify: the new `R` entry shows a chip for `note.png` (the own-file attachment path works alongside / independent of canned attachments).
- Status: [ ]

## Checklist (children)

- [ ] TS-M2-D2 — Canned-response fetch route (substituted body + attachment list; enabled + dept-scoped)
- [ ] TS-M2-D3 — Reply-box canned-response dropdown + own-file input UI
- [ ] TS-M2-D4 — Reply route: accept canned id + own upload, bind attachments, mark unanswered
- [ ] TS-M2-A5 — Staff reply attachment hook + thread chips (shared with Epic A)

## Test Infrastructure

- Seeded canned responses from TS-M2-D1: "Acknowledge receipt" (enabled, All-Departments, body with `%{ticket.number}`, carries `policy.txt`) + "Closed — disabled sample" (disabled; negative case for AC-2).
- Fixtures (create once; see US-M2-1 Test Infrastructure for the full block): `/tmp/qa-fixtures/invoice.pdf` (AC-1 client upload) and `/tmp/qa-fixtures/note.png` (AC-5 own-file path):
  ```sh
  mkdir -p /tmp/qa-fixtures
  printf '\211PNG\r\n\032\n\000\000\000\rIHDR\000\000\000\001\000\000\000\001\010\006\000\000\000\037\025\304\211' > /tmp/qa-fixtures/note.png
  ```

## Dependencies

- TS-M2-D1 (canned schema + seed), TS-M2-C1 (substitution engine), TS-M2-A5 (reply attachment hook). M1 staff reply route.
