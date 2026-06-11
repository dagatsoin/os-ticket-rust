# EPIC-M2-A — EPIC – Attachment storage & upload

- **ID**: EPIC-M2-A
- **Type**: Epic
- **Parent**: M2
- **Labels**: Epic, M2
- **Column**: derived from children (`min(children.columns)`)

## Spec References

- FS-022 (FS-022.12 content-addressed file store, FS-022.13 upload validation, FS-022.4 binding)
- FS-011 (FS-011.7 / EC-011.5 attachment hook on public create)
- FS-021 (FS-021.3 / FS-021.16 attachment hook on staff reply)

## Context

Milestone: M2 — Attachments, Canned Responses & Email Basics.
Attachments are the foundational layer the rest of M2 stands on: downloads (Epic B) and canned
attachments (Epic D) both consume the blob store this epic builds. First epic in M2; nothing else
ships without it.

## Description

Builds the **filesystem blob store keyed by SHA-256** (DEVIATION D1 — `var/blobs/ab/cd/<hash>`,
real byte-level dedup, no chunked-DB table) plus the two tables that reference it
(`attachment_file`, `ticket_attachment`), the **upload validation** (extension allow-list + max
size), and the **upload hooks** on both write channels (public ticket create, staff reply). The
frontend gets a file input on `/open` and the staff reply box, inline validation errors, and
**attachment chips** rendered on thread entries.

Business value: gives every ticket and reply the ability to carry files — the single most-requested
helpdesk affordance — with content dedup keeping storage lean. It is the prerequisite for the whole
M2 demo (a client cannot download in B, nor a canned attachment ride along in D, without it).

## Acceptance Criteria

- (Epic — no ACs. Validated through its children's ACs and the M2 root E2E AC-1/AC-2/AC-5.)

## Checklist (children)

- [ ] TS-M2-A0 — apiClient FormData/multipart support + multipart MSW handlers (frontend enabler; blocks A4, D3)
- [ ] TS-M2-A1 — Blob store + attachment_file / ticket_attachment schema (SHA-256 filesystem store)
- [ ] TS-M2-A2 — Upload validation (extension allow-list + max size) + attachment config keys
- [ ] US-M2-1 — Client attaches a file when opening a ticket (children: TS-M2-A3 create hook, TS-M2-A4 /open file input)
- [ ] TS-M2-A5 — Staff reply attachment hook + reply-box file input + thread chips

## Dependencies

- M1 (shared ticket core, public create + staff reply routes). No other M2 epic.
