# FS-022: Canned Responses & Ticket Attachments

## Overview

This specification defines two intertwined staff-control-panel capabilities of the osTicket
helpdesk:

1. **Canned responses** (also called "premade replies") — a library of reusable, named reply
   templates that staff can compose, manage, and insert into ticket replies. Each canned response
   carries a title, a response body that supports ticket-variable substitution, optional internal
   notes, an optional department scope, an enabled/disabled state, and up to ten attachments that
   ride along whenever the response is used.
2. **File storage & attachment download** — the content-addressed, database-backed file store
   (`AttachmentFile` / chunked file data) that underpins every attachment in the product (ticket
   attachments, canned-response attachments, FAQ attachments, and uploaded logos), together with
   the hash-keyed download/display entry-point scripts that authorize and serve those bytes to
   staff and clients. The download/display entry scripts present in this 1.7 snapshot are
   `attachment.php` (client), `scp/attachment.php` (staff), `scp/file.php` (staff), `scp/image.php`
   (staff inline display), and `kb/file.php` (knowledge base). There is **no** root-level `image.php`
   entry script in this snapshot; inline staff images are served by `scp/image.php`.

The canned-response **management UI** lives under the staff control panel's knowledge-base area
(`canned.php`), gated behind the "manage premade responses" staff permission. The canned-response
**consumption path** (inserting a response into a reply, with its variables already substituted and
its attachments pre-checked) is owned by the ticket-workflow specification and is referenced here as
a dependency; this spec owns the canned-response data model, CRUD, attachment binding, and the
shared file-storage/download model that the consumption path relies on.

> **Cross-reference**: Exact enum sets, table names, column lists, and config-key defaults are owned
> canonically by **FS-091 (Reference data, enums & data model)**. This spec references those tables
> and config keys by name and documents the literal limits and behaviors observed in the code, but
> does not restate the canonical schema.
>
> **Cross-reference**: Ticket reply composition, the canned-response dropdown on the reply form, and
> the attachment-upload widgets on ticket forms are owned by **FS-021 (Staff ticket view &
> workflow)**. Ticket-variable token definitions and the variable-replacer engine are owned by
> **FS-040 (Email accounts, templates & outbound mail)**.

---

## Functional Requirements

### FS-022.1: Canned Response Library (List View)

**Description**: The system shall present staff with a paginated, sortable list of all canned
responses, each row showing title, status, department scope, last-updated timestamp, and an
attachment indicator.

**Acceptance Criteria**:
- The list page (`cannedresponses.inc.php`, rendered by `canned.php`) is reachable only by a staff
  member with the manage-premade-responses permission (see FS-022.9); other staff are redirected to
  the knowledge-base landing page (`kb.php`).
- Each row displays: a selection checkbox, the response **title** (truncated to 200 characters,
  linking to the edit view), **status** ("Active" or bold "Disabled"), the **department** name (or
  "— All Departments —" when the response has no department scope), and the **last-updated** date/time.
- A row whose canned response has one or more attachments shows a file icon next to the title.
- The list is sortable by **title**, **status**, **department**, or **updated**, ascending or
  descending; the default sort is title ascending. An invalid sort key falls back to title; an
  invalid order falls back to ascending.
- The list is paginated using the global page size (`PAGE_LIMIT`); the page count and per-page
  window are shown in the caption ("N premade responses" or "No premade responses found!").
- A caption/heading reads "Canned Responses" with an "Add New Response" action link.

### FS-022.2: Create Canned Response

**Description**: The system shall let an authorized staff member create a new canned response with a
title, response body, department scope, status, optional internal notes, and optional attachments.

**Acceptance Criteria**:
- The add form (`cannedresponse.inc.php`, opened via `canned.php?a=add`) collects: **Status**
  (Active/Disabled radio, defaulting to Active), **Department** ("— All Departments —" or a specific
  department), **Title**, **Canned Response** body (a textarea, with a link to the supported-variables
  reference), **Canned Attachments** (a file input), and **Internal Notes** (a textarea).
- On submit (`do=create`), the system validates per the canned-response business rules (FS-022.7)
  and, on success, inserts a new canned response with `created` and `updated` timestamps set to the
  current time, then shows "Canned response added successfully".
- After a successful create, any files submitted in the attachments file input are validated and
  uploaded and bound to the new canned response (FS-022.4).
- On validation failure, the form is redisplayed with the submitted values re-populated and the
  per-field error messages shown.

### FS-022.3: Edit / Update Canned Response

**Description**: The system shall let an authorized staff member edit an existing canned response,
including changing its fields and adding or removing attachments.

**Acceptance Criteria**:
- The edit form is opened via `canned.php?id=<id>`; an unknown/invalid id yields "Unknown or invalid
  canned response ID."
- On submit (`do=update`) the system re-validates (FS-022.7) and, on success, updates the response
  fields and sets `updated` to the current time, then shows "Canned response updated successfully".
- **Attachment removal on save**: the edit form lists each existing attachment with a checkbox that
  is checked by default and a caption "Uncheck to delete the attachment on submit". On update, every
  currently-bound attachment whose id is NOT present in the submitted `files[]` keep-list is removed
  from the canned response (FS-022.5).
- **Attachment addition on save**: any newly submitted files are validated and uploaded and bound to
  the canned response (FS-022.4).
- After update, the canned response is reloaded so the redisplayed form reflects the new attachment
  set and field values.

### FS-022.4: Bind Attachments to a Canned Response

**Description**: The system shall accept one or more uploaded files for a canned response, store each
in the shared file store, and create a binding row linking the file to the canned response.

**Acceptance Criteria**:
- Uploaded files are first normalized and basic-validated by the shared file-format helper
  (FS-022.13); the canned-response upload accepts either a fresh upload descriptor or an existing
  numeric file id.
- For each accepted file: if it is a fresh upload it is stored in the file store (FS-022.12), then a
  binding row (`canned_id`, `file_id`) is inserted into the canned-attachment table; if it is already
  a numeric file id, the binding row is inserted directly (no re-upload).
- The function returns the count of successfully bound attachments and reloads the canned response
  when at least one binding was created.
- The number of attachments a canned response may carry is capped (see FS-022.8 / BS-022.4).

### FS-022.5: Remove Attachment(s) from a Canned Response

**Description**: The system shall let a single attachment, or all attachments, be unbound from a
canned response, reclaiming the underlying file storage when it is no longer referenced.

**Acceptance Criteria**:
- **Single removal** deletes the binding row matching this canned response and the given file id,
  then runs orphan reclamation (FS-022.11) so the underlying file is purged if no ticket, canned
  response, or FAQ still references it.
- **Bulk removal** (used when a canned response is deleted) deletes all binding rows for the canned
  response, then runs orphan reclamation.
- Removal of an attachment does not delete the underlying file bytes if another record still
  references the same file (content de-duplication; see BS-022.10).
- Single removal only runs orphan reclamation when the binding-delete actually affected a row (the
  delete matched a real binding). Note that orphan reclamation itself always reports success
  regardless of whether any file was actually purged, so the single-removal helper's boolean result
  reflects "a binding row was deleted," not "a file was purged" (see EC-022.15).

### FS-022.6: Mass-Process Canned Responses (Enable / Disable / Delete)

**Description**: The system shall let staff select multiple canned responses from the list and apply
a bulk enable, disable, or delete action.

**Acceptance Criteria**:
- The bulk action (`do=mass_process`) requires at least one selected response; otherwise it reports
  "You must select at least one canned response".
- **Enable** sets the selected responses' status to enabled; **Disable** sets it to disabled. The
  result message reports full success, or a partial count ("N of M selected canned responses
  enabled/disabled") when fewer rows were affected than selected.
- **Delete** removes each selected canned response (subject to the in-use-by-filter guard, BS-022.6),
  cascading to its attachment bindings and orphan reclamation. The result reports full success, a
  partial count, or failure.
- A delete confirmation dialog warns that deleted items cannot be recovered, "including any
  associated attachments".
- The action verb (enable / disable / delete) is carried in a hidden `a` form field that is populated
  client-side from the Enable/Disable/Delete button the staff member presses; the server dispatches on
  `lower($_POST['a'])`. An unrecognized verb yields "Unknown command"; a POST with an unrecognized
  top-level `do` value yields "Unknown action".
- **Enable** and **Disable** are executed as a single bulk `UPDATE ... WHERE canned_id IN (...)`; the
  "affected rows" count drives the full-vs-partial message. Because the update sets `isenabled` to a
  fixed value, re-enabling already-enabled rows (or re-disabling already-disabled rows) reports those
  rows as **not affected**, producing a partial-count message even though every selected response
  ends in the intended state (see EC-022.14).

### FS-022.7: Canned Response Field Validation

**Description**: The system shall validate canned-response fields on create and update before
persisting.

**Acceptance Criteria**:
- **Title** is required, must be at least 3 characters, and must be unique across canned responses
  (a duplicate title that belongs to a different response is rejected with "Title already exists").
- **Response body** is required ("Response text required").
- On update, if the submitted hidden id does not match the record being edited, an internal-error is
  raised ("Internal error. Try again").
- Title, response body, and notes are stripped of HTML tags before being stored (see BS-022.7 / KL-022.1).
- Department scope defaults to 0 ("All Departments") when not provided.

### FS-022.8: Canned Response Attachment Count Cap

**Description**: The system shall cap the number of attachments per canned response and surface that
cap to the staff member.

**Acceptance Criteria**:
- The form displays the helper text "You can upload up to 10 attachments per canned response."
- The single file-input control is only rendered while the canned response currently has fewer than
  10 attachments; at 10 attachments the add-file control is hidden.
- The cap is a hard-coded value of **10** (there is no admin setting to change it; see KL-022.2).

### FS-022.9: Canned-Response Management Permission

**Description**: The system shall restrict all canned-response management actions to staff who hold
the manage-premade-responses permission.

**Acceptance Criteria**:
- Access to `canned.php` (list, add, edit, and all POST actions) requires `thisstaff` to be present
  and to have the manage-premade-responses permission; failing either condition redirects to
  `kb.php`.
- Every POST action additionally re-checks the permission before processing.
- The permission is derived from the staff member's group permission flag (the "can manage premade"
  flag; canonical flag owned by FS-031 / FS-091).

### FS-022.10: Authorized Attachment Download & Inline Display

**Description**: The system shall serve attachment and file bytes only to authorized requesters, via
hash-keyed entry-point scripts that validate a per-session access hash before streaming the file.

**Acceptance Criteria**:
- **Client ticket-attachment download** (`attachment.php`): requires an authenticated client
  session, a ticket-attachment id, and a matching access hash; the script resolves the attachment →
  file, validates the hash against `md5(file_id + session_id + file_hash)`, confirms the parent
  ticket exists, and confirms the client has access to that ticket before downloading. Any failure
  yields a generic "Unknown attachment!" / "Unknown or invalid attachment" message and no bytes.
- **Staff ticket-attachment download** (`scp/attachment.php`): same as the client path but requires
  an authenticated staff session and validates the staff member's access to the parent ticket;
  failure yields "Access Denied" / "Unknown attachment!".
- **Staff file download by combined hash** (`scp/file.php`): requires a 64-character hash where the
  first 32 characters are the permanent file content hash and the last 32 are
  `md5(file_id + session_id + file_hash)`; both halves must validate or the request is rejected
  ("Unknown or invalid file."). On success the file is downloaded as an attachment.
- **Staff inline image display** (`scp/image.php`): same 64-character combined-hash validation as
  `scp/file.php`, but with one difference in *how* the last 32 characters are validated — instead of
  recomputing only the session-bound half, `scp/image.php` compares the **entire** submitted 64-char
  hash against the file's canonical download hash (`getDownloadHash()` = `lower(file_hash +
  md5(file_id + session_id + file_hash))`). The matched file is then rendered **inline (displayed)**
  rather than forced as a download. (Behaviorally equivalent to the two-half check, but worth noting
  the validation predicate differs from `scp/file.php`, `attachment.php`, and `kb/file.php`.)
- **Knowledge-base file download** (`kb/file.php`): same 64-character combined-hash validation as
  `scp/file.php` (recompute-the-last-32 form), served through the knowledge-base bootstrap context
  (`kb.inc.php`), downloaded as an attachment. **Note**: `kb/file.php` is reachable in the
  knowledge-base context with **no staff or client session re-check beyond the bootstrap and the
  session-bound hash** — the hash itself is the only credential (see EC-022.13).
- The canned-response edit form links each bound attachment to `file.php?h=<combined-hash>` where the
  combined hash is `file_hash + md5(file_id + session_id + file_hash)`.

### FS-022.11: Download vs Display Delivery Semantics

**Description**: The system shall deliver file bytes either as a forced download (with a filename) or
as inline content, with cache-control headers and conditional-request support.

**Acceptance Criteria**:
- Both delivery modes emit cache headers (`Last-Modified`, `ETag` = the file content hash,
  `Cache-Control: private, max-age=<ttl>` with a default TTL of 3600 seconds, `Expires`, `Pragma:
  private`) and honor conditional requests: a matching `If-Modified-Since` or `If-None-Match` (the
  content hash) results in an HTTP 304 Not Modified with no body.
- **Download mode** sets the content type (falling back to `application/octet-stream` when unknown),
  a `Content-Disposition: filename=...` header whose encoding is chosen per requesting user agent
  (legacy IE/Windows, Safari-without-Chrome, and the RFC 5987 `filename*` form for everything else),
  `Content-Transfer-Encoding: binary`, and the content length, then streams the bytes.
- **Display mode** sets the content type (same octet-stream fallback) and the content length, then
  streams the bytes inline (no `Content-Disposition`).
- File bytes are streamed chunk-by-chunk from the chunked store (FS-022.12) without loading the whole
  file into memory. Before streaming, output compression (`zlib.output_compression`) is disabled so
  the emitted `Content-Length` matches the bytes actually sent.
- The cache headers are emitted **before** the conditional-request short-circuit check, so a 304
  response still carries the `Last-Modified`, `ETag`, `Cache-Control`, `Expires`, and `Pragma`
  headers but no body.

### FS-022.12: Content-Addressed Chunked File Storage

**Description**: The system shall store file bytes in the database, split into fixed-size chunks, and
keyed by a content-derived hash, so that the same physical file can be referenced by many records.

**Acceptance Criteria**:
- A stored file record carries: MIME type, byte size, original name, a content hash, a creation
  timestamp, and a file-type marker (see FS-022 data requirements and FS-091).
- File bytes are written into the chunked-data table in fixed-size chunks; the chunk size is a
  defined constant of **500 × 1024 bytes** (≈500 KB), chosen to stay within the database's
  max-allowed-packet limit for large-object writes. (An in-code comment states "256kB" but the actual
  constant is 500 × 1024; the code value governs.)
- Chunk writes use an upsert (`REPLACE INTO`) keyed on `(file_id, chunk_id)`, so re-writing a file's
  data overwrites existing chunks at the same indices rather than appending duplicates.
- A stored file's id can be resolved either by numeric id or by its permanent content hash
  (`getIdByHash`); the file-lookup helper accepts either. The ticket-attachment lookup helper
  likewise accepts either a numeric attachment id or a file content hash (optionally constrained to a
  given ticket id).
- Each chunk row is keyed by `(file_id, chunk_id)` where `chunk_id` is a monotonically increasing
  index starting at 0; reads iterate chunk indices in order.
- The content hash is derived at upload time from the file content combined with the current time, so
  uploads are individually addressable (see BS-022.9 and KL-022.4 regarding de-duplication scope).
- A file's combined download hash is `lower(file_hash + md5(file_id + session_id + file_hash))` —
  the session-bound second half is what the download/display entry scripts validate.

### FS-022.13: Upload Validation (Type & Size)

**Description**: The system shall validate uploaded files against the configured allowed file types
and maximum file size before they are accepted.

**Acceptance Criteria**:
- The shared format/validation helper normalizes the raw multi-file upload structure and skips
  "no file selected" entries silently.
- Each remaining file is checked for a PHP upload error and for being a genuine uploaded file; a bad
  POST yields "Invalid or bad upload POST".
- When restricted validation is requested, each file is additionally checked against:
  - **Allowed file types**: the file's extension (last 3–4 characters after the final dot,
    lower-cased) must appear in the configured comma-separated allowed-types list. The wildcard value
    `.*` allows all types. A disallowed type yields "Invalid file type for <name>". (Type checking is
    extension-based only; MIME type is not enforced — see KL-022.3.)
    - If the allowed-types config is **empty/unset**, the type check returns "not allowed" for
      **every** file (no allow-list ⇒ nothing allowed; not "allow everything"). The wildcard `.*`
      is the only way to allow all types.
    - The extension regex matches only a trailing `.XXX` or `.XXXX` (3 or 4 chars). For a filename
      whose final extension is 1–2 chars or 5+ chars, the regex does not match and the **whole
      filename** is used as the comparison token, which will not be in the allow-list — so such a
      file is **rejected** unless `.*` is configured (see KL-022.3).
  - **Maximum file size**: when a non-zero max-file-size is configured, a file larger than it yields
    "File <name> (<size>) is too big. Maximum of <max> allowed".
- Validated files are passed to the store; the canned-response create/update path applies this
  validation to attachment uploads.

### FS-022.14: Canned Response Consumption (Reference)

**Description**: When a staff member selects a canned response while replying to a ticket, the system
shall return the response body with its ticket variables substituted and shall offer the canned
response's attachments for inclusion in the reply.

**Acceptance Criteria** (behavior owned jointly with FS-021/FS-040; documented here for the
canned-response contract):
- A canned response is only consumable while it is enabled; a disabled or unknown response is not
  returned ("No such premade reply").
- When a ticket context is supplied, the response body is run through the ticket variable-replacer so
  tokens are resolved against that ticket; without a ticket context the raw body is returned.
- The response can be fetched as plain text (`txt`, the default/fallback format) or as a JSON object
  (`json`). The JSON object carries the canned id (`id`), the canned **title** under a field named
  `ticket` (a historical misnomer — it holds the canned title, not a ticket value), the
  (variable-substituted) response body (`response`), and the list of attachments (`files`) — each
  attachment entry holding `id`, `size`, `hash`, `name`, and a per-session access `key` =
  `md5(file_id + session_id + file_hash)`.
- The retrieval entry point is the knowledge-base AJAX route
  `canned-response/<id>.<format>` where `<format>` is `json` or `txt`; an unknown or disabled id
  returns HTTP 404 "No such premade reply".
- The ticket context is supplied via a `tid` query parameter; when present, the ticket is looked up
  and used for variable substitution. (If the `tid` does not resolve to a ticket, the raw,
  unsubstituted body is returned rather than erroring.)
- A canned reply posted to a ticket attaches the canned response's bound attachments to the resulting
  thread entry and marks the response body with variables already substituted.
- When a canned reply is posted, the response body is variable-substituted, the poster is recorded as
  the literal "SYSTEM (Canned Reply)", the bound attachments are carried over by their file ids, and
  the ticket is marked **unanswered** after the reply is created. An alert/auto-reply email may then
  be sent using the department's (or default) template and email account unless alerting is
  suppressed.

### FS-022.15: Logo File Upload (Shared File Store)

**Description**: The system shall accept logo image uploads into the shared file store, marking them
as logo-type files that are exempt from attachment orphan reclamation.

**Acceptance Criteria**:
- A logo upload is stored with the file-type marker **`L`** (logo) rather than `T` (ticket), via the
  same chunked content-addressed store as attachments.
- When the image-processing extension is unavailable, the logo is stored as-is (no validation beyond
  the generic upload check).
- When image processing is available, the logo's image type must be GIF, JPEG, or PNG (otherwise
  "Invalid image file type") and its aspect ratio must meet or exceed a minimum width-to-height ratio
  (default 3); a too-square image is rejected with "Image is too square. Upload a wider image".
- Logo-type (`L`) files are **excluded** from the attachment orphan-reclamation sweep (BS-022.11), so
  they persist independently of ticket/canned/FAQ references.

---

## Business Rules

### BS-022.1: Department Scope Determines Availability
**Rule**: A canned response with a department scope of 0 is available to all departments; a canned
response with a specific department id is available only to that department.
**Rationale**: Lets teams maintain department-specific reply libraries while sharing global templates.
**Examples**:
- A response scoped to "All Departments" appears for every department's tickets.
- When listing responses for a specific department, both that department's responses and the global
  (dept 0) responses are returned unless an "explicit" (department-only) filter is requested.
- When the responses list is requested with **no department** (department id 0 / falsy, the default),
  the department filter is **not** applied at all — every enabled response (regardless of its
  department scope) is returned, ordered by title. (This is the variant used by the new-ticket open
  form's canned dropdown; the ticket-view reply dropdown instead requests responses scoped to the
  ticket's own department via the dept-scoped variant.)

### BS-022.2: Only Enabled Responses Are Offered for Use
**Rule**: Disabled canned responses are excluded from the lists offered during ticket reply and
cannot be consumed, but remain editable in the management UI.
**Rationale**: Allows staff to retire a template without deleting it (and its attachments/history).
**Examples**:
- A response marked "Disabled" still appears in the management list (shown in bold "Disabled") but
  not in the reply dropdown.
- Attempting to fetch a disabled response for a reply returns a not-found result.

### BS-022.3: Title Uniqueness and Minimum Length
**Rule**: Each canned response title must be unique and at least 3 characters long.
**Rationale**: Titles are the human key staff use to find a template; duplicates and stub titles
would make the library ambiguous.
**Examples**:
- Saving a second response titled "Password Reset" fails with "Title already exists".
- A 2-character title fails with "Title is too short. 3 chars minimum".

### BS-022.4: Ten-Attachment Cap per Canned Response
**Rule**: A canned response may carry at most 10 attachments.
**Rationale**: Bounds the payload that rides along with each use of the response.
**Examples**:
- At 10 attachments the add-file control is hidden so no further file can be selected.
- The cap is hard-coded with no admin override.

### BS-022.5: Deletion Cascades to Attachment Bindings
**Rule**: Deleting a canned response removes all of its attachment bindings and runs file-orphan
reclamation.
**Rationale**: Prevents dangling bindings and reclaims storage no longer referenced by anything.
**Examples**:
- Deleting a response with two attachments removes both bindings; each underlying file is purged
  only if no other record references it.

### BS-022.6: A Filter-Referenced Canned Response Cannot Be Deleted
**Rule**: A canned response that is referenced by one or more inbound email filters cannot be deleted.
**Rationale**: Filters may auto-send the canned response; deleting it would break the filter's action.
**Examples**:
- A response used as a filter's auto-reply shows a warning ("Canned response is in use by email
  filter(s): ...") on its edit form and is skipped by the delete action.

### BS-022.7: Canned Content Is Stored Plain-Text (Tags Stripped)
**Rule**: The title, response body, and notes are stripped of HTML tags before storage.
**Rationale**: osTicket 1.7 ticket replies were plain-text; storing markup would not render as
intended and could introduce injection risk.
**Examples**:
- A response body containing `<b>Hello</b>` is stored as "Hello".

### BS-022.8: Download Access Requires a Fresh Session-Bound Hash
**Rule**: Every attachment/file download or inline display requires an access hash that is bound to
the requester's current session id; a hash from another session is invalid.
**Rationale**: Prevents link sharing/replay: a download URL only works inside the session that
generated it, and only for a requester authorized to the parent record.
**Examples**:
- A staff download link copied into another browser session fails validation.
- A client can only download an attachment whose parent ticket they have access to.

### BS-022.9: Files Are Addressed by Content Hash
**Rule**: A stored file is identified by a content-derived hash; entry scripts resolve files by hash
(or numeric id) and the first 32 characters of a download hash is that permanent content hash.
**Rationale**: Enables hash-keyed download URLs and underpins reference counting/orphan reclamation.

### BS-022.10: Shared Files Are Reference-Counted; Bytes Purged Only When Orphaned
**Rule**: The underlying file bytes are deleted only when no ticket attachment, canned-response
attachment, or FAQ attachment references the file; reclamation also deletes orphaned chunk rows.
**Rationale**: One physical file may back many records; bytes must survive until the last reference
is gone.
**Examples**:
- Removing one of two canned responses that reference the same file leaves the file intact.
- Removing the last reference purges the file record and its chunks.

### BS-022.11: Orphan Reclamation Is Scoped to Ticket-Type Files
**Rule**: File-orphan reclamation only purges files marked as ticket-type (`ft = 'T'`); logo-type and
other non-ticket files are exempt from the reference-count sweep.
**Rationale**: Logos and similar assets are managed separately and must not be swept away by the
attachment orphan cleaner.

### BS-022.12: Conditional Requests Short-Circuit With 304
**Rule**: A download/display request whose `If-Modified-Since` matches the file's modified time, or
whose `If-None-Match` matches the file's content hash, returns HTTP 304 with no body.
**Rationale**: Standard HTTP caching to avoid re-streaming unchanged file bytes.

### BS-022.13: Allowed File Types Are Matched by Extension Only
**Rule**: Type restriction compares the file's lower-cased extension (last 3–4 chars after the final
dot) against the configured comma-separated allow-list; `.*` permits all types.
**Rationale**: Simple extension-based gate; MIME content is not inspected (see KL-022.3).

### BS-022.14: An Empty Allow-List Rejects Everything (Default-Deny)
**Rule**: If the allowed-file-types configuration is empty/unset, the type check denies every file;
only the explicit wildcard `.*` allows all types.
**Rationale**: The type gate is default-deny — absence of an allow-list is not interpreted as
"allow all."
**Examples**:
- With no allowed types configured, every restricted attachment upload fails type validation.
- Setting the allow-list to `.*` is the only way to accept arbitrary types.

### BS-022.15: Canned Reply Posts as System and Marks Ticket Unanswered
**Rule**: Posting a canned response as a ticket reply records the poster as "SYSTEM (Canned Reply)",
substitutes ticket variables into the body, carries the canned attachments onto the new thread entry,
and marks the ticket unanswered; an alert/auto-reply email may follow unless suppressed.
**Rationale**: A canned reply is an automated/templated reply; attribution and the unanswered flag
reflect that the customer has not yet been personally responded to.

---

## Data Requirements

> Canonical table/column definitions, enum value sets, and config-key defaults are owned by **FS-091**.
> The following describes the entities functionally as used by this spec.

### Canned Response
A reusable reply template with: a unique id; a title; a response body (plain text, variable-bearing);
internal notes; a department scope (0 = all departments, else a department id); an enabled/disabled
flag; and `created` / `updated` timestamps. Derived/aggregated at read time: attachment count and
filter-reference count.

### Canned Attachment Binding
A join record linking a canned response to a stored file (`canned_id`, `file_id`). Created on
attachment add; deleted on attachment remove or canned-response delete.

### Stored File
A content-addressed file record with: id; MIME type; byte size; original name; content hash; creation
timestamp; and a file-type marker (`ft`) distinguishing ticket-type (`T`) files from logo-type (`L`)
files. Aggregated at read time: number of canned responses and number of tickets referencing it
(used by the in-use check).

### File Chunk
A fixed-size slice of a stored file's bytes, keyed by `(file_id, chunk_id)`, chunk size = 500 × 1024
bytes, `chunk_id` zero-based and contiguous.

### Ticket Attachment
A join record linking a ticket (and its thread entry) to a stored file. Referenced here as the
record that the `attachment.php` / `scp/attachment.php` download scripts resolve and authorize
against the parent ticket. (Full ticket-attachment lifecycle owned by FS-021.)

### Configuration Keys (consumed, owned by FS-091/FS-032)
- `allow_attachments` — master switch for attachments.
- `allow_online_attachments`, `allow_online_attachments_onlogin`, `allow_email_attachments` —
  channel-specific attachment switches.
- `allowed_filetypes` — comma-separated allow-list of extensions (or `.*`).
- `max_file_size` — maximum accepted upload size, in bytes (required when attachments are enabled).
- `max_user_file_uploads` — cap on the number of files a client may attach (bounded by a system max).

### Access-Hash Derivations (literal)
- Per-attachment access hash: `md5(file_id + session_id + file_hash)`.
- File combined download hash (64 chars): `lower(file_hash + md5(file_id + session_id + file_hash))`
  — first 32 chars = permanent content hash, last 32 chars = session-bound hash.

---

## User Flows / Interactions

### Flow 1: Create a Canned Response with Attachments
1. An authorized staff member opens the canned-response list and clicks "Add New Response".
2. They set Status, Department, Title, Response body, optional Notes, and select up to 10 files.
3. On submit, the system validates the fields (FS-022.7); on success it creates the response, then
   validates and stores each file and binds it to the new response.
4. The success message "Canned response added successfully" is shown and the list view returns.

### Flow 2: Edit a Response and Swap Attachments
1. The staff member opens an existing response from the list.
2. Existing attachments are shown each with a checked checkbox ("Uncheck to delete on submit").
3. They uncheck one attachment to remove it and select a new file to add (only possible if under the
   10-attachment cap).
4. On submit, the system updates the fields, removes the unchecked attachment (running orphan
   reclamation), uploads/binds the new file, and reloads the response.

### Flow 3: Bulk Enable/Disable/Delete
1. The staff member selects multiple responses in the list and clicks Enable, Disable, or Delete.
2. Delete prompts a confirmation warning that deletion (and its attachments) is irreversible.
3. The system applies the action, skipping any filter-referenced response on delete, and reports full
   or partial success.

### Flow 4: Use a Canned Response in a Reply (reference — FS-021/FS-040)
1. While replying to a ticket, the staff member selects a canned response from a dropdown.
2. The system returns the response body with the ticket's variables substituted, plus the list of the
   response's attachments (each pre-checked for inclusion).
3. On posting the reply, the selected canned attachments are attached to the new thread entry.

### Flow 5: Download an Attachment
1. A staff member or client clicks an attachment link carrying a session-bound access hash.
2. The relevant entry script (`attachment.php`, `scp/attachment.php`, `scp/file.php`, `scp/image.php`,
   or `kb/file.php`) validates the requester's session, the access hash, and (for ticket attachments)
   access to the parent ticket.
3. On success the file is streamed chunk-by-chunk — forced as a download (with a per-user-agent
   filename header) or rendered inline for images — with cache headers and 304 conditional support.

---

## Edge Cases & Error Scenarios

### EC-022.1: Unknown / Invalid Canned Response Id
**Scenario**: A request references a non-existent canned response id.
**Expected**: "Unknown or invalid canned response ID." is surfaced; no action proceeds against a
missing record.

### EC-022.2: Update Id Mismatch
**Scenario**: The hidden form id does not match the record being updated.
**Expected**: "Internal error. Try again" is raised and the update is rejected.

### EC-022.3: Empty Bulk Selection
**Scenario**: A mass-process action is submitted with no responses selected.
**Expected**: "You must select at least one canned response"; no rows are touched.

### EC-022.4: Partial Bulk Result
**Scenario**: A bulk enable/disable/delete affects fewer rows than were selected (e.g., some
filter-referenced responses skipped on delete).
**Expected**: A partial-count warning ("N of M ...") rather than a full-success message.

### EC-022.5: Attachment Cap Reached on Edit
**Scenario**: A response already has 10 attachments.
**Expected**: The add-file control is not rendered; no further attachment can be added until one is
removed.

### EC-022.6: Disallowed File Type or Oversized File
**Scenario**: A staff member attaches a file with a disallowed extension or larger than the configured
max.
**Expected**: A per-file error ("Invalid file type for <name>" or "File <name> (<size>) is too big.
Maximum of <max> allowed") and the file is not stored.

### EC-022.7: Bad or Spoofed Download Hash
**Scenario**: A download/display request presents a missing, wrong-length, or mismatched access hash.
**Expected**: A generic rejection ("Unknown or invalid file." / "Unknown attachment!" / "Access
Denied") and no bytes are served. Client-facing scripts deliberately avoid revealing storage paths.

### EC-022.8: Cross-Session Link Replay
**Scenario**: A valid download link from one session is opened in a different session.
**Expected**: The session-bound half of the hash no longer matches; the request is rejected.

### EC-022.9: Unauthorized Ticket-Attachment Access
**Scenario**: A client/staff member requests an attachment whose parent ticket they cannot access.
**Expected**: The parent-ticket access check fails and the download is denied even if the hash is
otherwise well-formed.

### EC-022.10: Conditional Request Hit
**Scenario**: A cached client re-requests a file it already has (matching `If-None-Match` /
`If-Modified-Since`).
**Expected**: HTTP 304 Not Modified with no body.

### EC-022.11: Deleting a Filter-Referenced Response
**Scenario**: A staff member tries to delete a response used by an email filter.
**Expected**: The delete is refused for that response; the edit form warns it is in use by filter(s).

### EC-022.12: Orphan File After Last Reference Removed
**Scenario**: The last canned/ticket/FAQ reference to a file is removed.
**Expected**: Orphan reclamation purges the ticket-type file record and its chunk rows; non-ticket
(`L`) files are exempt.

### EC-022.13: KB File Download Without Staff/Client Session
**Scenario**: A request hits `kb/file.php` with a valid 64-char combined hash but no authenticated
staff or client session.
**Expected**: The download still succeeds because the session-bound hash is the sole credential on
that path (the hash embeds the requester's session id at link-generation time). There is no
additional staff/client gate beyond the bootstrap and the hash check.

### EC-022.14: Re-enable / Re-disable Already-In-State Responses
**Scenario**: A bulk enable is applied to responses that are already enabled (or disable to already-
disabled ones).
**Expected**: Those rows are reported as "not affected" by the bulk update, producing a partial-count
warning ("N of M ... enabled/disabled") even though every selected response already holds (or now
holds) the intended state.

### EC-022.15: Single-Removal Boolean Does Not Reflect File Purge
**Scenario**: A single canned attachment is removed but the underlying file is still referenced
elsewhere (so no file is actually purged).
**Expected**: The binding row is deleted and orphan reclamation runs but purges nothing; the
single-removal helper still reports success because it reflects "a binding was deleted," and orphan
reclamation unconditionally reports success.

### EC-022.16: Empty / Whitespace File-Type Allow-List
**Scenario**: Attachment validation runs with the allowed-types configuration empty or unset.
**Expected**: Every file is treated as a disallowed type ("Invalid file type for <name>"); no file
passes the restricted type check unless the allow-list is `.*`.

### EC-022.17: Unusual Extension Length on Restricted Upload
**Scenario**: A file with a 1–2-character or 5+-character final extension is uploaded under restricted
validation.
**Expected**: The extension regex does not match, the whole filename is compared against the allow-
list, no match is found, and the file is rejected ("Invalid file type for <name>") unless `.*` is
configured.

---

## Dependencies

| Dependency | Specification | Relationship |
|------------|---------------|--------------|
| Staff authentication & session, manage-premade permission | FS-002 / FS-031 | Gates access to `canned.php`; provides `session_id` used in access hashes |
| Ticket reply composition, canned-response dropdown, attachment widgets | FS-021 | Consumes canned responses and the file store; owns ticket-attachment lifecycle |
| Ticket variable tokens & variable-replacer engine | FS-040 | Substitutes variables into the response body at use time |
| Inbound email filters | FS-042 | May reference a canned response as an auto-reply action; gates deletion (BS-022.6) |
| Knowledge base / FAQ | FS-050 | Shares the file store and orphan reclamation (FAQ attachments); `kb/file.php` download path |
| Departments | FS-030 | Provides the department scope options for a canned response |
| System/ticket settings (attachment config) | FS-032 | Owns `allow_attachments`, `allowed_filetypes`, `max_file_size`, `max_user_file_uploads` |
| Reference data, enums, table/config schema | FS-091 | Canonical definitions of canned/file/attachment tables, the `ft` enum, and config keys |
| Request lifecycle, CSRF, formatting helpers | FS-001 / FS-003 | CSRF tokens on forms; tag-stripping/format helpers; pagination |

---

## Known Limitations

### KL-022.1: Canned Responses Are Plain-Text Only
**Limitation**: Title, body, and notes are HTML-tag-stripped on save ("until support is added to
tickets"), so rich-text canned responses are not supported in 1.7.
**Impact**: No formatted/HTML canned replies; markup pasted in is lost.

### KL-022.2: Attachment Cap Is Hard-Coded at 10
**Limitation**: The 10-attachment-per-response cap is hard-coded with an in-code TODO to add an admin
setting; there is no UI to change it.
**Impact**: Administrators cannot raise or lower the cap without code changes.

### KL-022.3: File-Type Validation Is Extension-Based, Not MIME-Verified
**Limitation**: Type checking uses only the (last 3–4 char) file extension; the code carries a TODO
noting the MIME type should also be checked. A file can be disguised by renaming its extension.
**Impact**: A disallowed file type can pass if given an allowed extension; conversely the regex only
matches 3–4-character extensions, so a file whose final extension is 1–2 chars or 5+ chars cannot be
normalized to a comparison extension and is therefore **rejected** (unless `.*` is configured) — the
whole filename is compared and will not be in any literal allow-list.

### KL-022.4: De-Duplication Is Per-Upload (Time-Salted Hash)
**Limitation**: The content hash is salted with the upload time, so two uploads of identical bytes
produce different file records rather than sharing one. Reference counting / orphan reclamation
operate on these distinct records; true byte-level de-duplication is not achieved at upload time.
**Impact**: Identical files uploaded separately occupy separate storage; the "content-addressed"
model de-duplicates only when a record is bound to an existing file id (e.g., canned attachments
carried onto a reply), not on independent re-upload.

### KL-022.5: Access Hash Uses MD5
**Limitation**: Access/download hashes are MD5-based and keyed only on file id + session id + content
hash; there is no expiry beyond session lifetime.
**Impact**: Within a live session the link remains valid; the scheme relies on session secrecy rather
than a cryptographically strong, time-limited token.

### KL-022.6: Canned-Response Attachment Types Are Not Re-Validated on Some Paths
**Limitation**: The create/edit handler notes a TODO to validate attachment types on the canned path;
while the shared format helper performs validation, in-code comments flag that restricted validation
is not consistently applied to canned attachments as it is to ticket uploads.
**Impact**: Possible inconsistency between canned-response and ticket attachment type enforcement.

---

## Future Considerations
- Admin-configurable attachment cap (replace the hard-coded 10).
- Rich-text / HTML canned responses once ticket replies support HTML.
- MIME-verified file-type validation in addition to extension checks.
- True content de-duplication (drop the time salt from the content hash).
- Stronger, time-limited download tokens (replace MD5 session-bound hashes).
- A unified attachment-download entry point with consistent authorization across client/staff/KB.
