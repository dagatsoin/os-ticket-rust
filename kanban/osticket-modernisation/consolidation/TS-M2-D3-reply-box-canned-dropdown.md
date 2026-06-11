# TS-M2-D3 — add(ui): FS-022.14 reply-box canned-response dropdown + own-file input

- **ID**: TS-M2-D3
- **Type**: Technical Story
- **Parent**: US-M2-2
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

The staff reply-box UI: a canned-response dropdown that fills the textarea with the substituted body
and shows the carried attachment chips, plus an own-file input.

## Impact

- update(ui): staff reply box gains a **"Canned response" dropdown** populated from the D2 list route (enabled + dept-scoped only).
- update(ui): selecting a response calls the D2 detail route and **fills the reply textarea** with the substituted body; renders the carried attachments as **AttachmentChips** (read-only, marked "from canned response").
- update(ui): add an own-file `<input type=file>` to the reply box (validated/submitted via the D4 reply route).

## Regressions

- The plain M1 reply (no canned, no file) must still post (TS-M1-C4).

## Acceptance Tests

### AC-1: BS-022.2 — the dropdown lists only enabled, dept-scoped responses. [BROWSER]
- Open a ticket as staff → the dropdown shows "Acknowledge receipt" and NOT the disabled sample.
- Status: [ ]

### AC-2: FS-022.14 — selecting a response fills the textarea with the substituted body. [BROWSER]
- Pick "Acknowledge receipt" → textarea shows the body with `%{ticket.number}` already substituted (no literal token).
- Status: [ ]

### AC-3: selecting a response shows its attachment as a chip. [BROWSER]
- After selection → a `policy.txt` chip appears in the reply composer.
- Status: [ ]

### AC-4: the reply box has an own-file input. [BROWSER]
- A file input is present in the reply composer (wired to D4).
- Status: [ ]

## Dependencies

- TS-M2-D2 (canned fetch route), TS-M2-A5 (AttachmentChip), TS-M1-C4 (staff reply UI).
