# FS-061: Upgrader & Database Migration Streams

## Overview

This specification describes the **in-application upgrade subsystem** of osTicket 1.7: the mechanism that detects when a running installation's database schema is older than the deployed code, and migrates that database forward by applying an ordered chain of cryptographically-keyed SQL patches and (where required) procedural data-migration tasks.

The subsystem exists because osTicket ships as files that an operator overwrites on disk (a "drop-in" code update); after the new code is in place, the **database schema is now behind the code**. The upgrader closes that gap. Until it does, the bootstrap layer treats the installation as **upgrade-pending** and forces the operator into the upgrade wizard before any normal control-panel work can resume (cross-ref FS-001 bootstrap gating).

The central design idea is a **hash-chained migration stream**. Each schema checkpoint has an 8-hex-character short signature. Migration patch files are named `<from-signature>-<to-signature>.patch.sql`; the migrater walks this chain link-by-link from the database's current recorded signature toward the stream's target signature (the stream tip declared in a `.sig` file shipped with the code). Some checkpoints additionally carry a **procedural task** (`.task.php`) — a resumable PHP migration that moves *data* (not just structure), e.g. moving on-disk attachments into the database, re-encrypting stored passwords, or normalising legacy comma-list columns into join tables. Long-running tasks run in **time-boxed batches** so they survive PHP execution-time limits, with their interim state stashed in the session between AJAX round-trips.

The wizard runs **only for a logged-in admin** inside the staff control panel, is driven by an AJAX progress loop (with a manual non-JS fallback), and is **fatal-on-error** (any failure aborts the whole run and demands a restore-from-backup).

### Scope boundaries

- **In scope:** upgrade detection (schema-signature vs code-shipped target), the hash-chained patch model, patch/cleanup/task file conventions, the wizard UX and access gating, per-patch batched execution and time-boxing, resumable data-migration tasks, abort/error handling and alerting, the AJAX progress protocol and manual fallback, and the post-upgrade "system warm" actions.
- **Out of scope (cross-referenced):** the from-scratch installer and its install SQL (FS-060); request-lifecycle bootstrap that *invokes* the upgrade gate (FS-001); the `schema_signature`/`config` table and full DB schema (FS-091); admin authentication that protects the wizard (FS-002); the underlying SQL-loading/prereq-check helper shared with the installer (FS-060); export-package generation that reuses the migration-stream listing (FS-090).

---

## Functional Requirements

### FS-061.1: Upgrade-pending detection by schema-signature comparison

**Description:** The system shall determine whether an installation needs upgrading by comparing, **per stream**, the schema signature recorded in the database against the target signature shipped with the code.

**Acceptance Criteria:**
- For each declared stream, the system reads the **target signature** from a file `streams/<stream>.sig` shipped alongside the code, and the **current signature** recorded in the `config` table under key `schema_signature` for that stream's namespace.
- If, for **any** stream, the recorded current signature differs from the shipped target signature (case-insensitive comparison), the installation is reported **upgrade-pending**.
- If every stream's recorded signature equals its shipped target, the installation is **not** upgrade-pending ("already upgraded to the current version").
- Detection is the basis of the global "system online" decision: an installation is considered online only when it is configured online **and** not upgrade-pending (cross-ref FS-001).
- The signature lookup is namespace-aware: it resolves the stream's own namespaced `schema_signature` config row; failing that, a legacy single-row signature; failing that, the md5 of a legacy pre-1.7 version column (`getDBVersion`) — see FS-061.10 for legacy detection.

### FS-061.2: Stream discovery from streams configuration

**Description:** The system shall discover the set of upgrade streams to process from a streams configuration file, defaulting to a single `core` stream when none is declared.

**Acceptance Criteria:**
- The system reads `streams/streams.cfg` from the upgrade directory; if that file is absent or empty, it defaults to the single stream name `core`.
- Each non-blank, non-comment line of the config names a stream; comments (text following `#`) and blank lines are ignored.
- A named stream is included only if **both** a signature file `<stream>.sig` exists **and** a directory `<stream>/` exists; otherwise the named stream is silently skipped.
- The trimmed contents of `<stream>.sig` are taken as that stream's target signature.
- The discovered stream list is computed once and cached for the duration of the request (static memoization).

### FS-061.3: Hash-chained patch resolution within a stream

**Description:** For a given stream, the system shall resolve the ordered list of SQL patches required to move the database from its current signature to the stream target, by following the patch-filename hash chain.

**Acceptance Criteria:**
- Patch files are named `<from8>-<to8>.patch.sql`, where `<from8>`/`<to8>` are the first 8 hex characters of the source and destination schema signatures respectively.
- Starting from the current signature's first 8 characters, the system globs for `<current8>-*.patch.sql`; the single match's destination 8 characters become the next link's source, and the loop repeats.
- Resolution stops when: (a) no patch leaves the current signature (assume fully patched), (b) the chain reaches the stream's target signature, or (c) **more than one** patch leaves the same signature (an ambiguous fork the linear walker cannot resolve — see EC-061.3).
- The resolved list preserves chain order (oldest patch first).
- A stream is **upgradable** only if at least one next patch exists from its current signature; a stream is **finished** when it has neither a next patch nor a pending task.

### FS-061.4: Per-patch batched application with time-boxing

**Description:** The system shall apply resolved patches to the database in time-boxed batches so that a single request cannot exceed the host's PHP execution-time limit.

**Acceptance Criteria:**
- A single upgrade invocation applies **at most five** patches.
- Before applying patches, the system records the start time and determines a maximum batch duration from the PHP `max_execution_time` setting (defaulting to 300 seconds when unavailable).
- Each patch's SQL is loaded and executed (cross-ref FS-060 SQL-loader): comments/remarks stripped, the `%TABLE_PREFIX%` placeholder replaced with the configured table prefix, statements split on `;` and run sequentially.
- A patch's final SQL statement updates the `config` `schema_signature` row for the stream namespace to the patch's destination signature — i.e. **each patch advances the recorded checkpoint** (the signature is the new HEAD after the patch).
- After applying a patch, if **no task** is attached to its checkpoint, the system runs any cleanup script (FS-061.6), then continues to the next patch — unless elapsed time exceeds **80%** of the max batch time, in which case it breaks and returns to be resumed on the next request.
- After applying a patch, if a **task is attached** to its checkpoint, the system sets the task state and breaks immediately so the task can run (FS-061.5).
- When the wizard's PHP execution-time limit can be lifted (host is not in safe-mode), the time limit is removed for the upgrade request.

### FS-061.5: Resumable procedural migration tasks

**Description:** The system shall support attaching a resumable procedural data-migration task to a schema checkpoint, executing it in batches across multiple requests until it reports completion.

**Acceptance Criteria:**
- A checkpoint carries a task when a file `<phash>.task.php` exists in the stream directory, where `<phash>` is the 17-character patch identifier `<from8>-<to8>`.
- A task file returns the **string name** of a class extending the abstract migration-task contract; the system instantiates that class. A task file that returns a non-string or names a non-existent class is logged as a "Bogus migration task" and treated as no task.
- The task contract exposes: a human-readable **description**, a **status** message, a **run(max_time)** method, an **isFinished()** predicate, and **sleep()/wakeup()** hooks to serialise/restore interim state.
- A task is executed by calling `run(max_time)` with a batch budget derived from `max_execution_time` (defaulting to 30 seconds). If `isFinished()` is false afterward, the task's `sleep()` payload is stored in the session and the task is re-entered on the next request via `wakeup()`; if true, the system runs the checkpoint cleanup script, clears the task's session state and patch hash, and proceeds.
- The entire task instance (and its captured state) persists in the session between calls, so all instance variables survive across requests.
- The upgrade run does not advance past a checkpoint with a pending (unfinished) task.
- The base task contract supplies defaults: `description` = "[Unnamed task]", `status` = "finished", `isFinished()` = true, `sleep()` = null, `wakeup()` = no-op, `run()` = no-op. A concrete task therefore is treated as **single-shot** (completes in one `run()` call) unless it overrides `isFinished()` to return false and implements `sleep()`/`wakeup()`. In this version, only the attachment-migration task is genuinely resumable; the session-, API-key-, password- and group-access-migration tasks complete in a single call (they ignore the `max_time` argument and leave `isFinished()` at its true default).

### FS-061.6: Per-checkpoint cleanup scripts

**Description:** The system shall optionally run a cleanup SQL script after a checkpoint's patch (and any task) completes.

**Acceptance Criteria:**
- A checkpoint carries a cleanup script when a file `<phash>.cleanup.sql` exists in the stream directory.
- Cleanup scripts are typically structural teardown that must run *after* dependent data has been migrated (e.g. dropping legacy columns/tables no longer needed once a task has copied their data forward).
- The cleanup script is loaded and run through the same SQL-loader as patches, but is **non-aborting on error**: a failed cleanup is logged ("Unable to process cleanup file") and the upgrade continues rather than aborting.
- When no cleanup script exists for a checkpoint, the step is a no-op.

### FS-061.7: Patch metadata extraction (version annotation)

**Description:** The system shall extract human-readable metadata from a patch file's leading comment block to label upgrade progress.

**Acceptance Criteria:**
- The system parses the patch file's leading `/** ... */` comment for `@key value` annotations (e.g. `@version v1.7.1`, `@signature <hash>`, `@schema <hash>`).
- The `@version` annotation, when present, is used as the human-facing "next version" label (e.g. "Upgrade to v1.7.1").
- When `@version` is absent, a fallback label is derived from the patch filename's destination signature segment.
- When no further patches remain, the next-version label is the literal `(Latest)`.

### FS-061.8: Multi-stream coordination

**Description:** The system shall coordinate upgrade of multiple parallel streams, advancing through all unfinished streams in turn.

**Acceptance Criteria:**
- A coordinator instantiates one stream-upgrader per discovered stream (FS-061.2), each seeded with its current signature (from config) and target signature (from `.sig`).
- The coordinator tracks the **current stream** in session state; when the current stream finishes, it advances to the next not-yet-finished stream.
- An installation is **upgradable** only when it is not aborted **and every** stream reports upgradable; a single non-upgradable stream blocks the whole run (see EC-061.4).
- All per-step operations (next action, next version, current schema signature, errors, task, upgrade) delegate to the current stream's upgrader.
- The classical single-stream case (`core` only) is the default; multi-stream supports customizations/plugins maintaining their own DB-update streams.

### FS-061.9: Upgrade wizard UX & access gating

**Description:** The system shall present a multi-screen admin-only upgrade wizard, selecting the screen from upgrade state, and shall require operator confirmation to begin.

**Acceptance Criteria:**
- The wizard is reachable only through the staff control panel (`scp/upgrade.php`); the legacy `setup/upgrade.php` entry redirects there.
- Every wizard screen aborts with "Access Denied" unless the request is within the staff control-panel include context **and** the current staff member is an admin.
- The wizard selects one screen from upgrade state: **prereq** (intro + requirements check + "Start Upgrade Now"), **upgrade** (in-progress action display + "Upgrade Now"), **done** (success), **aborted** (fatal-error report), or **rename** (config-file-rename instruction — see FS-061.13).
- The **prereq** screen displays the PHP and MySQL minimum-requirement checks with pass/fail indicators and the live PHP version, and recommends backing up the database first. The PHP check is a true version comparison (`PHP_VERSION` ≥ the configured minimum `4.3`); the MySQL check is **only** a "MySQL driver extension loaded" test — the displayed "v4.4 or greater" is a static label and **no MySQL version number is actually compared** (the indicator shows "module loaded" vs "missing!"). See FS-061.16.
- The **upgrade** screen warns the operator not to cancel or close the browser ("any errors at this stage will be fatal") and shows the current next-action label.
- The wizard adds an "Upgrader" submenu under the admin dashboard tab (active under the `dashboard` admin tab) and loads the AJAX progress driver script.
- All wizard POST forms carry a CSRF token, and the staff-control-panel POST gate **rejects any POST without a valid CSRF token with HTTP 400 "Valid CSRF Token Required"** — this applies to both the wizard form POSTs and the AJAX upgrade endpoint, since both boot through the staff include (cross-ref FS-001/FS-002). The AJAX upgrade route is **POST-only**.
- The wizard's POST handler accepts only the step values `prereq` and `upgrade`; any other `s` value yields the error "Unknown action!" and falls through to render the state-selected screen. POST is ignored entirely while the run is already aborted.

### FS-061.10: Legacy (pre-1.7) and pre-namespace version detection

**Description:** The system shall recognise installations predating the 1.7 namespaced-signature scheme and derive a starting signature for them.

**Acceptance Criteria:**
- When no namespaced `schema_signature` row exists, the system falls back to a legacy single-row `schema_signature` value if present.
- When neither exists (a pre-1.7 / v1.6 database), the system derives the starting signature as the md5 of the legacy `ostversion` config column value (`getDBVersion`).
- This derived signature seeds the hash chain so a 1.6 database can be walked forward through the patch sequence.

### FS-061.11: AJAX progress protocol with manual fallback

**Description:** The system shall drive the upgrade as a repeated AJAX progress loop, falling back to manual per-step submission when AJAX is unavailable.

**Acceptance Criteria:**
- The default (`ajax`) mode posts repeatedly to the upgrader AJAX endpoint; each response carries the next-action label, which is rendered to the progress UI.
- The AJAX endpoint is admin-gated (403 "Access Denied" otherwise) and returns:
  - **HTTP 200** + progress text while work remains. The text is the task next-action label when a pending task was processed, or the literal `"Upgraded to <next-version> ... post-upgrade checks!"` when a patch batch was applied, or otherwise the stream's next-action label. The client schedules the next poll (~200 ms).
  - **HTTP 201** + "We're done!" when no upgrade remains pending; before responding the endpoint closes/writes the session; the client shows "Cleaning up!..." and after ~3 s navigates to the wizard (carrying a cache-buster query) to render the done screen.
  - **HTTP 416** + "We have a problem ... wait a sec." when the run is aborted or has accumulated errors (checked both at entry and after a failed upgrade attempt).
- On an AJAX transport failure the client switches to **manual** mode: an HTTP **404** is interpreted as "Manual upgrade required (ajax failed)" and reloads the wizard with `m=manual`; other errors retry by reloading.
- In manual mode the operator advances by submitting the upgrade form per step (no JS); the wizard processes one step per POST (`doTask` if a task is pending, else apply a patch batch, else mark done).
- When switching to manual mode, the system logs a warning advising the operator to ensure the server's `AcceptPathInfo` directive is on (the usual cause of AJAX path-info failures).

### FS-061.12: Abort, error capture & alerting

**Description:** The system shall treat any upgrade error as fatal, abort the run, surface accumulated errors to the operator, and alert the operator/admin by email and log.

**Acceptance Criteria:**
- Any patch/SQL load error during a patch (the non-cleanup case) aborts the run; the run state is set to `aborted` and subsequent invocations short-circuit.
- On error the system: logs the error to the system log (alerting the admin), records the error message in the upgrader's error list, and sets the global run state to `aborted`.
- If the operator performing the upgrade has an email **different** from the admin's email, the system additionally emails that operator an alert (subject `[<stream>]: Upgrader Error`) via the configured alert/default email account, falling back to a raw system mail send.
- The **aborted** screen displays the captured error(s) and directs the operator to system logs / email, the upgrade guide, and to restore from backup before retrying.
- A run that has aborted is reported **not upgradable**.

### FS-061.13: Config-file rename precondition

**Description:** The system shall block the upgrade until a legacy configuration filename is renamed, to avoid post-upgrade conflicts.

**Acceptance Criteria:**
- When the active config file basename is `settings.php`, the wizard shows the **rename** screen instructing the operator to rename `include/settings.php` to `include/ost-config.php` and continue.
- The prereq step refuses to proceed ("Config file rename required to continue!") while the config basename is `settings.php`.

### FS-061.14: Post-upgrade completion actions

**Description:** On successful completion the system shall mark the run done, clean up upgrade state, and perform "warm the system" housekeeping.

**Acceptance Criteria:**
- When no upgrade remains pending, the run state is set to `done`. The act of setting the run state to `done` is what triggers ticket creation (the side-effect lives in the state setter), so it fires once regardless of whether `done` was reached via the AJAX or manual path.
- Reaching `done` creates a confirmation ticket with these exact field values: status `open`, source `Web`, the configured **default priority** and **default department**, help-topic id `0` (none), a random 6-digit ticket number, sender email literally `support@osticket.com`, sender name "osTicket Support", subject "osTicket Upgraded!".
- The ticket's first thread entry is created with thread type Message (`M`), source `Web`, title "osTicket Upgraded" (note: no trailing "!" — distinct from the ticket subject), and a body read from the upgrade message file (`msg/upgraded.txt`); if that file is empty/unreadable the body falls back to the literal "Congratulations and Thank you for choosing osTicket!".
- Ticket creation is best-effort: if the ticket INSERT fails (or no insert id is returned) the thread message is simply not written and the upgrade still reports done (no abort).
- The **done** screen destroys the upgrader's session state and presents success guidance (release notes, re-enabling the system from the admin panel, subscribing for updates).
- The success screen directs the operator that the system can now be brought back online from admin settings.

### FS-061.15: Persistence of upgrade run state across requests

**Description:** The system shall persist the in-progress upgrade run across the AJAX/manual request sequence using the session.

**Acceptance Criteria:**
- The run's **state**, **mode** (ajax/manual), **current stream**, current **patch hash**, and per-task interim payloads are all held in the session under an `ost_upgrader` namespace.
- On each request the coordinator rehydrates from session: if there is no current stream or the current stream is finished, it advances to the next unfinished stream.
- The migrater is reset (recomputed) after each patch batch so the next chain walk starts from the freshly-recorded signature.
- On `done`, the session upgrade state is cleared.

### FS-061.16: Minimum-requirement check semantics

**Description:** The system shall gate the upgrade on a PHP-version comparison and a MySQL-driver-presence test, exposing both as the prereq check.

**Acceptance Criteria:**
- `check_php` passes when the running PHP version compares greater-than-or-equal to the configured minimum (`4.3`).
- `check_mysql` passes when the MySQL driver extension is loaded; it performs **no** server-version comparison — the prereq screen's "v4.4 or greater" wording is purely informational and the indicator reflects only "module loaded" vs "missing!".
- `check_prereq` passes only when both `check_php` and `check_mysql` pass; otherwise the prereq step blocks with "Minimum requirements not met! Refer to Release Notes for more information".
- The same prereq contract (`prereq = {php:4.3, mysql:4.4}`) is shared with the installer (cross-ref FS-060); these floors reflect the 1.7-era contract, not the requirements of the actual migration assets (see KL-061.8).

### FS-061.17: Time-limit removal at stream-upgrader construction

**Description:** The system shall attempt to lift the PHP execution-time limit as soon as a stream-upgrader is constructed.

**Acceptance Criteria:**
- On constructing each stream-upgrader, if the host is **not** in safe-mode, the request's execution-time limit is removed (set to unlimited).
- This happens unconditionally per stream-upgrader instance (once per discovered stream), independent of whether any patches will actually be applied.
- On safe-mode hosts where the limit cannot be lifted, the upgrade still relies on the ≤5-patch cap, the 80%-time-box, resumable task batches, and signature-checkpointing to survive timeouts (cross-ref EC-061.9, KL-061.6).

---

## Business Rules

### BS-061-01: Upgrade-pending is a per-stream signature mismatch
**Rule:** An installation is upgrade-pending if, for any stream, the database-recorded `schema_signature` (per namespace) differs (case-insensitive) from the code-shipped `<stream>.sig` target. **Rationale:** The signature is the canonical record of "where the schema is"; the `.sig` is "where the code expects it to be." **Example:** core `.sig` = `16fcef4a13d6475a5f8bfef462b548e2`; config row `core` = `8aeda901…` ⇒ upgrade pending.

### BS-061-02: An upgrade-pending installation is forced into the wizard
**Rule:** While upgrade is pending, the admin control panel redirects all requests (except `upgrade.php` and `logs.php`) to the upgrade wizard, and the system is treated as offline. **Rationale:** Running new code against an old schema risks corruption. (Cross-ref FS-001 for the global online/offline gate; cross-ref FS-091 for the offline page.)

### BS-061-03: Only an authenticated admin may run the upgrade
**Rule:** Every wizard screen and the AJAX upgrade endpoint require an active staff session whose member is an admin; otherwise "Access Denied" (403). **Rationale:** Schema migration is destructive and privileged.

### BS-061-04: Patches chain by 8-hex-character signatures
**Rule:** Patch filenames encode `<from8>-<to8>` short signatures; the chain is walked one link at a time from the current signature to the stream target. **Rationale:** Filename-encoded links make the migration order self-describing and code-shipped. **Example:** `c00511c7-7be60a84.patch.sql` migrates from signature `c00511c7…` to `7be60a84…`.

### BS-061-05: Each patch advances the recorded signature
**Rule:** Every patch's final statement updates the stream's `config` `schema_signature` to the patch destination; the in-memory signature also advances to the new HEAD. **Rationale:** Makes each patch idempotently resumable — a re-run resolves the chain from the last committed checkpoint. **Example:** `…SET value='16fcef4a…' WHERE key='schema_signature' AND namespace='core'`.

### BS-061-06: A linear chain only — forks abort
**Rule:** If more than one patch leaves a given signature, the chain walker stops (no graph resolution); combined with FS-061.8, an unresolvable stream blocks the whole run. **Rationale:** osTicket 1.7's migrater is deliberately linear (the code notes a graph approach would be needed). (See EC-061.3.)

### BS-061-07: At most five patches per request; tasks break the batch
**Rule:** A single upgrade call applies ≤5 patches; encountering a checkpoint with a task breaks the batch immediately to run the task. **Rationale:** Bounds work per request and isolates data-migration tasks.

### BS-061-08: Patch batches are time-boxed to 80% of max execution time
**Rule:** Between patches, if elapsed time exceeds 80% of the max batch time (from `max_execution_time`, default 300s), the batch breaks to be resumed on the next request. **Rationale:** Avoid mid-statement PHP timeouts on slow hosts.

### BS-061-09: Tasks run in resumable batches and never restart from scratch
**Rule:** A migration task is invoked with a batch budget (default 30s); if unfinished, its `sleep()` state is stashed and `wakeup()` restores it on re-entry — the task must not restart from the beginning. **Rationale:** Large data migrations (e.g. attachments) cannot complete in one request. **Example:** the attachment migrater processes ~100-file batches at 90% of its time budget.

### BS-061-10: Cleanup scripts run after data migration and never abort
**Rule:** A checkpoint's `.cleanup.sql` runs only after the patch and any task complete, and a cleanup failure is logged but does not abort the upgrade. **Rationale:** Cleanups drop legacy structures whose data tasks have already moved; a missing legacy column shouldn't fail the whole upgrade. **Example:** `c00511c7-7be60a84.cleanup.sql` drops the legacy `email_banlist` table and obsolete columns.

### BS-061-11: Any patch error is fatal and aborts the entire run
**Rule:** A failure applying a (non-cleanup) patch sets run state to `aborted`, logs and alerts, and halts; the operator is told to restore from backup. **Rationale:** A partially-applied schema is unsafe to continue against.

### BS-061-12: The operator is alerted separately when not the admin
**Rule:** On error, if the upgrading operator's email differs from the admin email, the operator is emailed an alert in addition to the admin's log alert. **Rationale:** Whoever is driving the upgrade should see the failure even if they are not the configured admin.

### BS-061-13: Minimum prerequisites — PHP ≥ 4.3, MySQL extension loaded (≥ 4.4)
**Rule:** Prereq check passes only when the PHP version is ≥ 4.3 and the MySQL extension is loaded; failure blocks the upgrade with "Minimum requirements not met!". **Rationale:** The target code requires these. (Same prereq contract as FS-060.)

### BS-061-14: Legacy config filename must be renamed before upgrading
**Rule:** If the active config file is `settings.php`, the upgrade is blocked until it is renamed to `ost-config.php`. **Rationale:** Avoids post-upgrade config-resolution conflicts on installations carrying the older filename.

### BS-061-15: Streams default to `core`
**Rule:** Absent a valid `streams.cfg`, exactly one stream named `core` is processed; a declared stream is honoured only if both `<stream>.sig` and `<stream>/` exist. **Rationale:** Single-stream is the canonical case; multi-stream is opt-in for plugins/customizations.

### BS-061-16: Completion creates a confirmation ticket
**Rule:** Reaching `done` inserts an "osTicket Upgraded!" ticket (default dept/priority, web source) with a thread message from `msg/upgraded.txt`. **Rationale:** Leaves a visible, verifiable artifact that the upgrade ran and the ticket pipeline works.

### BS-061-17: The system stays offline until the operator re-enables it
**Rule:** Completing the upgrade does not auto-bring the help desk online; the operator must re-enable it from admin settings. **Rationale:** Lets the operator verify the upgraded system before exposing it. (Cross-ref FS-001/FS-032 online flag.)

### BS-061-18: MySQL prereq is a driver-presence test, not a version check
**Rule:** The MySQL minimum-requirement check passes whenever the MySQL driver extension is loaded; the advertised "v4.4 or greater" is a static label only, with no version comparison. Only the PHP check is a true version comparison (≥ 4.3). **Rationale:** osTicket 1.7 relied on the legacy `mysql` extension being available; the actual server-version floor was enforced elsewhere/by the migration assets, not by this gate.

### BS-061-19: The AJAX upgrade endpoint is CSRF-protected and POST-only
**Rule:** The AJAX upgrade endpoint is routed POST-only and, like every other staff POST, is rejected with HTTP 400 "Valid CSRF Token Required" if the request lacks a valid CSRF token; the wizard forms supply the token via the serialized form data. **Rationale:** The AJAX driver triggers the same destructive migration as the manual form and must carry the same protection. (Cross-ref FS-002.)

### BS-061-20: The confirmation ticket uses fixed synthetic identity fields
**Rule:** The "osTicket Upgraded!" completion ticket is inserted with hard-coded sender email `support@osticket.com`, sender name "osTicket Support", help-topic id 0, status `open`, source `Web`, a random 6-digit ticket number, and the configured default department/priority; its first thread entry is a Message (`M`) titled "osTicket Upgraded" (no trailing "!"). **Rationale:** The ticket is a self-test artifact, not a real customer ticket, so it uses canned identity values. (Cross-ref FS-091 ticket fields.)

### BS-061-21: Completion ticket creation is best-effort, never fatal
**Rule:** If the completion-ticket INSERT fails or returns no insert id, the thread message is skipped and the upgrade still reports `done`; ticket-creation failure never aborts or re-opens the run. **Rationale:** The system is already fully migrated by the time `done` is set; a missing self-test ticket must not undo a successful upgrade.

### BS-061-22: Most migration tasks are single-shot
**Rule:** A migration task is single-shot by default (one `run()` call, `isFinished()`=true); only a task that overrides `isFinished()`/`sleep()`/`wakeup()` is genuinely resumable. In this version only the attachment migration is resumable; the DB-session, API-key, password and group-access migrations each complete in one call. **Rationale:** Only the attachment migration moves enough data to risk exceeding a single request's time budget. (See BS-061-09 for the resumable case.)

---

## Data Requirements

The upgrader is largely **file-driven** (code-shipped migration assets) and **session-driven** (run state), reading from and writing to the persistent schema only via patches/tasks.

**Code-shipped migration assets** (under the upgrade directory `streams/`):
- `streams.cfg` — newline-delimited list of stream names (comments via `#`); optional (default `core`).
- `<stream>.sig` — the stream's **target** schema signature (full hash; first 8 chars used for chaining). Example: `core.sig` = `16fcef4a13d6475a5f8bfef462b548e2`.
- `<stream>/<from8>-<to8>.patch.sql` — a migration patch. Leading `/** … */` block carries `@version` / `@signature` / `@schema` annotations; body uses the `%TABLE_PREFIX%` placeholder and ends by updating the `schema_signature` config row.
- `<stream>/<from8>-<to8>.task.php` — optional procedural task; returns the migration-task class name as a string.
- `<stream>/<from8>-<to8>.cleanup.sql` — optional post-migration structural teardown.
- `msg/upgraded.txt` — body of the confirmation ticket created on completion.

**Persistent state read/written** (canonical schema in FS-091):
- `config` table: `schema_signature` (per `namespace`, e.g. `core`) — the current checkpoint, advanced by every patch. Legacy fallbacks: a single-row `schema_signature`, and pre-1.7 `ostversion`.
- Tables migrated by the shipped tasks observed in this version:
  - **Attachment migration** — moves on-disk attachment files into DB-backed file storage: reads `ticket_attachment`/`ticket`, writes file rows, sets `ticket_attachment.file_id`, unlinks the disk file. (Cross-ref FS-022 file storage, FS-091.)
  - **Password re-encryption** — re-encrypts stored mailbox passwords (`email.userpass`) under the current crypto scheme. (Cross-ref FS-003 crypto.)
  - **API-key migration** — converts a legacy comma-list `config.api_whitelist` into per-IP `api_key` rows. (Cross-ref FS-043.)
  - **Group→department access migration** — explodes legacy `groups.dept_access` comma-list into `group_dept_access` join rows. (Cross-ref FS-031.)
  - **DB-session migration** — writes the current PHP session into the DB-backed session store. (Cross-ref FS-002 sessions.)

**Session run-state** (`ost_upgrader` namespace): `state` (`upgrade`/`aborted`/`done` or empty=prereq), `mode` (`ajax`/`manual`), current `stream`, current `phash` (patch hash), and per-task serialised payloads keyed by patch hash.

**Identifier formats:**
- Schema signature: a full hash (md5-length in `core`); the **short signature** is its first 8 hex chars.
- Patch hash (`phash`): the 17-character `<from8>-<to8>` filename stem.

---

## User Flows / Interactions

### Flow A — Operator runs an AJAX-driven upgrade (happy path)
1. Operator overwrites osTicket code on disk with a newer version; the schema is now behind.
2. Operator logs in to the admin control panel; bootstrap detects upgrade-pending and redirects to the upgrade wizard (BS-061-02).
3. Wizard shows the **prereq** screen: PHP/MySQL checks, backup recommendation. Operator clicks "Start Upgrade Now".
4. Prereq validates (upgradable, prereqs met, config filename OK); run state → `upgrade`.
5. **upgrade** screen loads; the AJAX driver begins polling the upgrader endpoint.
6. Each poll applies up to 5 patches (or processes one task batch), advances the recorded signature, and returns the next-action label (HTTP 200); the UI updates ("Upgrade to v1.7.1 …", "Migrating … (status)").
7. When no upgrade remains, the endpoint returns HTTP 201 "We're done!"; the UI shows "Cleaning up!" and reloads the wizard.
8. Run state → `done`: a confirmation ticket is created; the **done** screen shows success and clears upgrade session state.
9. Operator re-enables the help desk from admin settings (BS-061-17).

### Flow B — Manual (no-JS) upgrade
1. Steps 1–4 as in Flow A.
2. If AJAX fails (e.g. path-info disabled), the driver switches to manual mode (logs an `AcceptPathInfo` warning) and reloads with `m=manual`.
3. The **upgrade** screen now advances per form POST: each submission processes one step (pending task → patch batch → done).
4. Operator repeats until the wizard reaches the **done** screen.

### Flow C — Long data-migration task (e.g. attachments)
1. During patching, the run reaches a checkpoint with a `.task.php` and breaks the patch batch.
2. The task class is instantiated; each poll calls `run(max_time)` for a time-boxed batch (e.g. ~100 attachments).
3. If `isFinished()` is false, the task's state is stashed (`sleep`) and restored (`wakeup`) on the next poll; progress is shown via the task status.
4. When finished, the checkpoint cleanup runs, task state clears, and patching resumes from the next link.

### Flow D — Config rename required
1. If the active config file is `settings.php`, the wizard shows the **rename** screen.
2. Operator renames `include/settings.php` → `include/ost-config.php` and clicks Continue; the upgrade proceeds.

### Flow E — Abort
1. A patch/SQL error occurs; the run state → `aborted`; the error is logged (admin alerted) and, if the operator differs from the admin, the operator is emailed.
2. The **aborted** screen shows the error(s) and directs the operator to logs/email and to restore from backup before retrying.

---

## Edge Cases

- **EC-061.1 — Already current:** If the database matches the code target, the wizard reports "Nothing to do! System already upgraded to <version> with no pending patches to apply." and offers no upgrade action.
- **EC-061.2 — Unsupported source version:** If upgrade is pending but the current signature is not upgradable (no chain leaves it), the wizard reports "The upgrader does NOT support upgrading from the current patch [<sig>]!". The AJAX endpoint aborts with "Upgrade Failed: Invalid or wrong hash [<sig>]".
- **EC-061.3 — Ambiguous fork:** If more than one patch leaves a signature, the chain walker stops at that point (no graph resolution); the stream becomes non-upgradable and the run cannot proceed past the fork.
- **EC-061.4 — One bad stream blocks all:** Because upgradability requires *every* stream to be upgradable, a single non-upgradable stream blocks the whole installation's upgrade.
- **EC-061.5 — Bogus task file:** A `.task.php` that returns a non-string or names a missing class is logged as "Bogus migration task" and treated as no task (the checkpoint proceeds without a task).
- **EC-061.6 — Missing/unreadable cleanup or patch file:** A missing patch/SQL file aborts the run via the SQL-loader ("Error accessing SQL file …"); a failed cleanup is logged but non-fatal (BS-061-10).
- **EC-061.7 — Attachment file not found on disk:** The attachment task probes both the 1.5-style and 1.6-style on-disk paths; if neither exists, the attachment is skip-listed (logged) and migration continues. An invalid upload directory aborts that task's batch with an error.
- **EC-061.8 — Attachment disk-unlink failure:** After copying an attachment into the DB and setting `file_id`, if removing the on-disk file fails it is logged but not retried (the nonzero `file_id` prevents reprocessing) — a possible orphaned disk file.
- **EC-061.9 — PHP execution timeout on slow hosts:** Mitigated by removing the time limit when not in safe-mode, the ≤5-patch cap, the 80%-of-max time-box, and resumable task batches; a host that times out mid-step relies on the recorded-signature checkpointing to resume.
- **EC-061.10 — AJAX transport / path-info failure:** A 404 from the AJAX endpoint triggers an automatic switch to manual mode with an `AcceptPathInfo` warning; other transport errors retry by reload.
- **EC-061.11 — Browser closed mid-upgrade:** The upgrade screen explicitly warns the operator not to cancel/close ("any errors at this stage will be fatal"); because state is session- and signature-checkpointed, re-entering the wizard resumes from the last committed checkpoint, but an interrupted task batch may need re-running.
- **EC-061.12 — Aborted run re-entry:** Once state is `aborted`, the AJAX endpoint and coordinator short-circuit to the aborted screen ("We have a problem … wait a sec.") until the operator restores from backup and restarts.
- **EC-061.13 — Pre-1.7 / v1.6 database:** Detected via the md5-of-`ostversion` fallback signature, which seeds the chain so a 1.6 schema can be walked forward (BS-061-15 / FS-061.10).
- **EC-061.14 — Concurrent upgrade attempts:** Run state lives in a single operator's session; there is no cross-session lock, so simultaneous admin upgrades are not coordinated beyond per-statement DB behavior.
- **EC-061.15 — Unknown wizard step:** A POST whose step value is neither `prereq` nor `upgrade` produces the error "Unknown action!" and the wizard simply re-renders the screen selected by current run state; no migration work is attempted.
- **EC-061.16 — POST while aborted is ignored:** The wizard's POST handler only runs when the run is not already aborted; once aborted, posting `prereq`/`upgrade` is a no-op and the aborted screen is shown.
- **EC-061.17 — Manual-step error promotion:** In the manual (`s=upgrade`) path, after attempting a task or patch batch, if the stream reports accumulated errors the wizard explicitly sets the run state to `aborted` for that request (in addition to the abort already set inside the error path).
- **EC-061.18 — Already-current via the bottom switch:** When no state is set and there is nothing to upgrade, the prereq screen is still selected but with the message "Nothing to do! System already upgraded to **&lt;version&gt;** with no pending patches to apply." (distinct from the POST-time "already upgraded to the current version" message).
- **EC-061.19 — MySQL driver missing:** If the MySQL extension is not loaded the prereq indicator shows "missing!" and `check_prereq` fails, blocking the upgrade — even though no MySQL server version is ever inspected (BS-061-18).

---

## Dependencies

- **FS-001 (App bootstrap & shared request lifecycle):** Provides the `isUpgradePending`/`isSystemOnline` gate, the control-panel redirect to the wizard, the offline treatment of an upgrade-pending system, and the `UPGRADE_DIR` constant. **This spec is the authoritative source for the upgrade-detection logic that FS-001 invokes.**
- **FS-060 (Installer & setup wizard):** The stream-upgrader extends the same setup-wizard base; the SQL-loader (comment-strip, `%TABLE_PREFIX%` substitution, statement split, abort-on-error), the prereq checks (PHP/MySQL), and the error/abort plumbing are shared with the installer. Install SQL and stream layout originate there.
- **FS-002 (Staff authentication & access control):** Admin-only gating of the wizard and AJAX endpoint; the DB-session-migration task writes the session store.
- **FS-003 (Crypto/validation/format infrastructure):** Used by the password-re-encryption task; logging/alerting plumbing.
- **FS-022 (Canned responses & ticket attachments / file storage):** Target of the attachment migration task (disk → DB-backed file storage).
- **FS-031 (Staff, groups & directory):** Target of the group→department-access migration.
- **FS-043 (External API & cron):** Target of the API-key migration.
- **FS-040 (Email accounts & outbound mail):** Used to deliver the operator error-alert email.
- **FS-090 (Shared UI & data export):** The export/package tooling reuses the migration-stream listing (`getUpgradeStreams`).
- **FS-091 (Reference data & data model):** Canonical owner of the `config`/`schema_signature` rows and all tables touched by patches/tasks — referenced, not restated here.

---

## Known Limitations

- **KL-061.1 — Linear migration chain only.** The migrater cannot resolve a branch where multiple patches leave the same signature; it simply stops (the code itself notes a graph approach would be required). Stream authors must keep a strictly linear chain.
- **KL-061.2 — No rollback / down-migrations.** Migration is forward-only; recovery from a failed upgrade is "restore from backup," not an automated reverse migration. Aborting leaves the schema partially advanced up to the last committed checkpoint.
- **KL-061.3 — Run state is session-bound, not locked.** Upgrade progress lives in one operator's session with no cross-session lock; concurrent or cross-browser upgrade attempts are not coordinated.
- **KL-061.4 — Fatal-on-error with manual recovery.** Any non-cleanup patch error aborts the entire run and requires operator intervention (logs/email/backup) — there is no automatic retry or partial-success continuation.
- **KL-061.5 — Attachment migration can orphan disk files.** If the on-disk file cannot be unlinked after a successful DB copy, the orphan is logged but neither retried nor cleaned up; conversely, files that cannot be located are skip-listed and silently left behind.
- **KL-061.6 — Time-limit reliance on host config.** Batching uses `max_execution_time` (with hard-coded defaults) and removes the limit only when not in safe-mode; on constrained hosts a mid-statement timeout still depends on signature-checkpoint resumption to recover.
- **KL-061.7 — Hash algorithm assumed.** The `core` stream's signature/hash algorithm is fixed (md5-style) and not configurable per stream in this version (a code TODO notes making it configurable).
- **KL-061.8 — Dated prerequisite floor.** The advertised minimums (PHP 4.3 / MySQL 4.4) reflect the 1.7-era contract and are not representative of versions required by the actual migration assets.
- **KL-061.9 — Completion side-effect ticket.** Successful completion always inserts an "osTicket Upgraded!" ticket (with fixed synthetic sender identity per BS-061-20); there is no option to suppress this artifact.
- **KL-061.10 — MySQL version is not actually enforced.** The prereq gate only checks that the MySQL driver extension is loaded; the displayed "v4.4 or greater" floor is never compared against the live server version, so an under-version MySQL server passes the gate (BS-061-18 / EC-061.19).
- **KL-061.11 — Convergent chains are walked, divergent chains are not.** The linear walker only inspects patches *leaving* a signature, so multiple patches *converging onto* the same destination are fine (the shipped `core` stream has several); only multiple patches *leaving* one signature (a true fork) stops the walk (KL-061.1 / EC-061.3).
- **KL-061.12 — Tasks ignore the time budget unless they choose to honour it.** The batch time argument passed to `run()` is advisory; single-shot tasks ignore it entirely, so a task that does too much work in one call can still exceed the execution-time limit despite the upgrader's batching machinery.
