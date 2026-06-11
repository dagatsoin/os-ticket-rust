# TS-M2-D3 — add(ui): FS-022.14 reply-box canned-response dropdown + own-file input

- **ID**: TS-M2-D3
- **Type**: Technical Story
- **Parent**: US-M2-2
- **Labels**: Technical Story, M2
- **Scope**: small

## Context

Owns the **ENTIRE staff reply composer UI** (ROADMAP M2 Decisions §10): the canned-response dropdown
that fills the textarea with the substituted body, the read-only **carried** canned-attachment chips,
the own-file input, AND the construction of the **multipart POST that retains the `cannedId`** so the
D4 route re-renders the body server-side and binds the canned attachments by file id (no re-upload).
Submits via the FormData-aware apiClient (TS-M2-A0). Thread-entry chip rendering / download is B2 — a
different surface; this is the composer.

## Impact

- update(ui): the staff reply box gains a **"Canned response" dropdown** populated from the D2 list route (enabled + dept-scoped only).
- update(ui): selecting a response calls the D2 detail route and **fills the reply textarea** with the substituted body; renders the carried attachments as **read-only AttachmentChips** marked "from canned response" (sourced from the D2 `attachments` array, §7). **Note:** reuses A4's AttachmentChip (readOnly + marker) and the `validateAttachment` helper for the own-file input (ROADMAP M2 Decisions §14).
- update(ui): add an own-file `<input type=file>` to the reply box.
- update(ui): submit the reply as `multipart/form-data` via the FormData-aware apiClient (TS-M2-A0), **retaining the selected `cannedId`** plus the own `attachment` part so the D4 route re-renders + binds server-side (§2, §10).

## Regressions

- The plain M1 reply (no canned, no file) must still post (TS-M1-C4).

## Acceptance Tests

> Setup (all): `cargo run -p tools --bin seed -- --reset`; backend on :3701, SPA on :3702; seed/open a
> ticket; log in agent / Agent123! at http://localhost:3702/staff/login and open that ticket's detail.

### AC-1: BS-022.2 — the dropdown lists only enabled, dept-scoped responses. [BROWSER]
- Navigate: the staff ticket detail; scroll to the reply composer.
- Verify: the "Canned response" dropdown shows "Acknowledge receipt" and does NOT show "Closed — disabled sample".
- Status: [x]

### AC-2: FS-022.14 — selecting a response fills the textarea with the substituted body. [BROWSER]
- Action: pick "Acknowledge receipt" from the dropdown.
- Verify: the reply textarea is populated with the canned body and `%{ticket.number}` is already substituted to this ticket's number (no literal `%{...}`).
- Status: [x]

### AC-3: selecting a response shows its attachment as a chip. [BROWSER]
- Action: (after AC-2's selection) inspect the composer.
- Verify: a read-only `policy.txt` AttachmentChip (marked "from canned response") appears in the reply composer.
- Status: [x]

### AC-4: the reply box has an own-file input. [BROWSER]
- Navigate: the reply composer.
- Verify: an own-file `<input type=file>` is present (wired to the D4 multipart POST).
- Status: [x]

## Dependencies

- **TS-M2-A0 (apiClient FormData/multipart support — blocker)**, TS-M2-D2 (canned fetch route), TS-M2-D4 (multipart reply route the composer POSTs to), TS-M1-C4 (staff reply UI). AttachmentChip component from TS-M2-A4.
