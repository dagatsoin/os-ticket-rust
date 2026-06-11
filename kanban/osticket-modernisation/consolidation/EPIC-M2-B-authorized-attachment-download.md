# EPIC-M2-B — EPIC – Authorized attachment download

- **ID**: EPIC-M2-B
- **Type**: Epic
- **Parent**: M2
- **Labels**: Epic, M2
- **Column**: derived from children (`min(children.columns)`)

## Spec References

- FS-022 (FS-022.10 authorized download, FS-022.11 download delivery semantics, BS-022.8 access,
  EC-022.7 / EC-022.8 / EC-022.9 download error scenarios)

## Context

Milestone: M2. Needs Epic A (the blob store + `ticket_attachment` rows it streams).
Closes the loop opened by Epic A: once a file is stored and bound to a ticket, an authorized
requester must be able to get the bytes back out.

## Description

Adds **client and staff download routes** that stream blobs with a `Content-Disposition` filename
(FS-022.10 / FS-022.11). Authorization is a **session check on the parent ticket** (DEVIATION D2 —
BS-022.8 behaviour preserved: you can only download an attachment whose parent ticket your session
can access), covering the EC-022.7 (bad/unknown id), EC-022.8 (cross-session replay), and EC-022.9
(unauthorized ticket) error scenarios. The frontend turns the Epic-A attachment chips into
**clickable download links** in the client portal and the staff detail view.

Business value: completes the user-visible attachment experience — a customer can actually retrieve
the file an agent sent, and vice-versa — while the parent-ticket gate keeps one customer's files
out of another's reach. This epic is what makes M2 root AC-5 (client downloads the attachment)
green.

## Acceptance Criteria

- (Epic — no ACs. Validated through its children's ACs and the M2 root E2E AC-5.)

## Checklist (children)

- [ ] TS-M2-B1 — Client + staff download routes (stream blob, Content-Disposition, parent-ticket auth)
- [ ] US-M2-3 — Client downloads the attachment (child: TS-M2-B2 clickable chips client portal + staff detail)

## Dependencies

- EPIC-M2-A (blob store + `ticket_attachment` rows). M1 realms/session.
