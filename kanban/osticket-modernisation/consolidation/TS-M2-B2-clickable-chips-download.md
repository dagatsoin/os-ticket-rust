# TS-M2-B2 — add(ui): FS-022.10 clickable attachment chips → download (client portal + staff detail)

- **ID**: TS-M2-B2
- **Type**: Technical Story
- **Parent**: US-M2-3
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Turns the Epic-A AttachmentChip (TS-M2-A5) into a download trigger wired to the B1 routes, in both
the client portal thread and the staff detail thread.

## Impact

- update(ui): AttachmentChip becomes clickable — clicking calls the realm-appropriate B1 download route and saves the file (browser download), passing the realm session cookie.
- update(ui): client portal uses the client route; staff detail uses the staff route.

## Regressions

- Threads without attachments render unchanged (US-M1-4 / staff detail).

## Acceptance Tests

### AC-1: clicking a chip in the client portal downloads the file. [BROWSER]
- Client portal thread → click `policy.txt` chip → browser downloads `policy.txt`, bytes match.
- Status: [ ]

### AC-2: clicking a chip in the staff detail view downloads the file. [BROWSER]
- Staff detail thread → click a chip → file downloads for the staff session.
- Status: [ ]

### AC-3: a client logged into a different ticket cannot reach this ticket's download. [BROWSER]
- In a second-ticket client session, attempt the first ticket's download → denied, no file (mirrors EC-022.9).
- Status: [ ]

## Test Infrastructure

- Reuses the demo ticket (invoice.pdf on M, policy.txt on R) + a second ticket for the cross-session negative.

## Dependencies

- TS-M2-B1 (download routes), TS-M2-A5 (AttachmentChip + thread rendering).
