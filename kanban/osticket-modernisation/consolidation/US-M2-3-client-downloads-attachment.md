# US-M2-3 — Attachments – Client downloads the attachment

- **ID**: US-M2-3
- **Type**: User Story
- **Parent**: EPIC-M2-B
- **Labels**: User Story, M2
- **Scope**: small

## Spec References

- FS-022 (FS-022.10 authorized download, FS-022.11 download delivery; BS-022.8 parent-ticket access;
  EC-022.7 bad id, EC-022.8 cross-session, EC-022.9 unauthorized ticket)

## Context

Epic: EPIC-M2-B — Authorized attachment download.
The closing step of the M2 demo (root AC-5): the customer retrieves the file the agent sent. Also
covers the security boundary — one customer must not reach another's attachments.

## Description

As a customer viewing my ticket in the portal, I can click an attachment to download it, and I can
only download attachments that belong to a ticket I'm logged into.

## Impact

- Frontend (clickable attachment chips → download links in the client portal; staff detail too)
- Backend (client + staff download routes streaming the blob with a Content-Disposition filename; parent-ticket session gate)
- Browser (desktop)

## Business Rules

- BS-022.8 (behaviour preserved via D2): a requester may download an attachment **only** if their session can access the parent ticket. The client route is **session-bound with no ticketId param** — `GET /api/client/ticket/attachments/{attachmentId}` (ROADMAP M2 Decisions §8).
- FS-022.11: the download streams the blob with `Content-Disposition` filename and the stored MIME type (octet-stream fallback). Chips download via **fetch-with-credentials → blob → object-URL** so a denial surfaces a visible inline error (§9).
- EC-022.7 / EC-022.8 / EC-022.9: bad/unknown id, cross-session replay, and unauthorized-ticket requests are all denied with **no bytes served (404, no existence leak §8)**.

## Regressions

- The client portal read-only thread (US-M1-4, done) must still render messages/replies unchanged; chips are additive.

## Acceptance Criteria

### AC-1: In the client portal, attachment chips on the thread are clickable and download the file. [BROWSER]
- Setup: a ticket exists with an `M` attachment (`invoice.pdf`) and an agent `R` reply attachment (`policy.txt`) — use the root demo ticket or seed one.
- Navigate: http://localhost:3702/tickets → log in with that ticket number + its email.
- Verify: the read-only thread shows the `M` and `R` entries each with their **clickable attachment chip**.
- Action: click the `policy.txt` chip.
- Verify: the browser downloads the file (bytes received, `Content-Disposition` filename `policy.txt`); content matches the seeded canned attachment.
- Status: [ ]

### AC-2: The agent can download the same attachments from the staff detail view. [BROWSER]
- Navigate: log in agent / Agent123!; open the ticket.
- Action: click an attachment chip on the thread.
- Verify: the file downloads for the authorized staff session.
- Status: [ ]

### AC-3: A client logged into a DIFFERENT ticket cannot download this ticket's attachment. [BROWSER]
- Setup: open a SECOND ticket (different email); log into the client portal for that second ticket only.
- Action: attempt the first ticket's attachment id via the session-bound client route in that second session.
- Verify: the request is **denied (404, no existence leak §8)**, a **visible inline error** is shown (§9), and no bytes are served (EC-022.9).
- Status: [ ]

### AC-4: A bad / unknown attachment id is rejected. [API-ONLY]
- Request: as an authenticated client of a ticket, request a download for a non-existent attachment id (and an attachment id belonging to another ticket).
- Expect: **404** with the shared error envelope; no bytes (no existence leak §8). (EC-022.7 / EC-022.8 cross-session replay covered by the session-bound route, D2.)
- Status: [ ]

## Checklist (children)

- [ ] TS-M2-B1 — Client + staff download routes (stream blob, Content-Disposition, parent-ticket auth) — shared with Epic B
- [ ] TS-M2-B2 — Clickable attachment chips → download links (client portal + staff detail)

## Test Infrastructure

- Reuses the root demo ticket (invoice.pdf on M, policy.txt on R) or a purpose-seeded ticket with both attachment kinds. A second ticket (different email) is the cross-session negative fixture.

## Dependencies

- EPIC-M2-A (blob store + `ticket_attachment`). M1 client + staff realms/session.
