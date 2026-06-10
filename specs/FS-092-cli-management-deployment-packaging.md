# FS-092: CLI Management, Deployment & Packaging Tooling

## Overview

This specification documents osTicket's developer/operator **command-line tooling
subsystem** — the family of scripts under `setup/cli/` that exist outside the normal
web request lifecycle and are invoked from a shell. None of this behavior is reachable
through the web UI; it serves the maintainers and operators who build, deploy, and
restore osTicket installations.

The subsystem comprises four logical concerns:

1. **A CLI dispatcher and an argparse-style option framework** (`setup/cli/manage.php`,
   `setup/cli/modules/class.module.php`). A single front-controller (`manage.php`)
   discovers self-registering action *modules*, routes a named action to one of them,
   and prints aggregated help. Every module is built on a shared `Module` base that
   parses options/arguments, validates required arguments, and renders per-module help.

2. **Deployment and unpacking** (`setup/cli/modules/deploy.php`,
   `setup/cli/modules/unpack.php`). These place osTicket source files into a target
   installation path — `unpack` from a built tarball/zip's `upload/` tree, `deploy`
   directly from a working source repository (the continuous-deployment model). Both
   can relocate the bulk source `include/` directory outside the web root and rewrite
   the `INCLUDE_DIR` definition in `main.inc.php` accordingly.

3. **The release packager** (`setup/cli/package.php`). It runs the regression test
   harness, assembles a `stage/` build tree mirroring the shipped archive layout, then
   version-stamps the build (`THIS_VERSION`) and **hardens it for production by forcing
   `display_errors`/`display_startup_errors` off** — the inverse of the install-time
   tuning documented in FS-060 — and finally produces `.tar.bz2` and `.zip` release
   archives.

4. **The backup import / restore pipeline** (`setup/cli/modules/import.php`). It is the
   structural counterpart to the FS-090.27 full-database backup exporter: it consumes
   the exporter's record-separated, JSON-block dump format and reconstructs
   `CREATE TABLE`, `CREATE INDEX`, and batched multi-row `INSERT` SQL, either emitting
   SQL to standard output or (when primed) executing it against the live database.

> **Cross-spec notes.**
> - The dump *format* consumed by the importer is owned by **FS-090.27** (full-database
>   backup exporter). This spec documents only the *consumption / restore* side.
> - The `THIS_VERSION` constant and the install-time `display_errors=on` tuning are
>   owned by **FS-060.1** (installer bootstrap) and **FS-001** (request lifecycle /
>   runtime hardening). This spec documents only the *release-time inverse transform*
>   the packager applies to the shipped tree.
> - The `INCLUDE_DIR` constant and `main.inc.php` bootstrap are owned by **FS-001**;
>   this spec documents only how the deployment/unpack tooling *rewrites* that line.
> - Canonical table names/schemas are owned by **FS-091**; the importer reconstructs
>   table DDL purely from metadata carried in the dump, not from any schema authority.

Source: `setup/cli/manage.php`, `setup/cli/modules/class.module.php`,
`setup/cli/package.php`, `setup/cli/modules/deploy.php`, `setup/cli/modules/unpack.php`,
`setup/cli/modules/import.php`. (The `export` module is owned by FS-090.27; the
`setup/test/**` self-test harness is classified as a test harness and is out of scope.)

---

## Functional Requirements

### FS-092.1: CLI front controller & action dispatch

**Description**: The system shall provide a single command-line front controller that
routes a named *action* to one of a set of self-registering action modules.

**Acceptance Criteria**:
- The front controller is invoked as `manage.php <action> [options] [arguments]`. The
  first positional argument is the action name.
- Invocation is permitted **only** from the command-line SAPI; a non-CLI invocation
  terminates with `"Management only supported from command-line"`.
- On startup the front controller installs a no-op session save handler (CLI tooling
  must not start or persist a real session).
- To resolve an action, the controller includes every script matching
  `modules/*.php` (each of which registers itself, see FS-092.2), then looks up the
  action name in the module registry. If a module is found, the controller delegates
  execution to it.
- Before delegating, the controller removes the consumed action token from the global
  argument vector so the target module's own option parser sees only *its* options and
  arguments (the action name is not re-interpreted as a positional argument).
- An unrecognized action writes `"Unknown action given"` to the error stream and then
  prints the aggregated help listing (FS-092.3).
- `manage.php --help` with no action prints the aggregated help listing.
- The front controller is itself a module (its action name is the basename of the
  invoked script); it declares a single required argument `action` and disables
  automatic help so that a bare `--help` reaches its custom help routine.

### FS-092.2: Self-registering action module model

**Description**: Each action shall be implemented as a module that registers itself
under an action name into a global registry at load time, so the dispatcher can
discover it without a central manifest.

**Acceptance Criteria**:
- A module registers by calling a static `register(action, class)` that instantiates
  the class and stores it in a global `registered_modules` map keyed by the action
  name (e.g. `export`, `import`, `deploy`, `unpack`).
- A static `getInstance(action)` returns the registered instance for an action name.
- Including a module script is sufficient to register it; registration occurs at the
  bottom of each module file as a side effect of loading.
- The in-tree modules are: `export` (FS-090.27), `import` (FS-092.10–092.16),
  `unpack` (FS-092.7–092.8), and `deploy` (FS-092.6).

### FS-092.3: Aggregated and per-module help rendering

**Description**: The system shall render help both at the aggregate level (all
available modules) and at the per-module level (a single module's usage, options, and
arguments).

**Acceptance Criteria**:
- Aggregate help (from the front controller) loads every `modules/*.php` script, prints
  the front controller's own usage/options, then appends one line per registered module
  showing the action name padded to a fixed column followed by that module's one-line
  prologue description.
- Per-module help prints, in order: the module's prologue (if any); a `Usage:` line
  built from a usage template with `$script` replaced by the program-plus-module name
  and `$args` replaced by the space-joined argument names; an `Options:` section listing
  every option (sorted by key) via the option's formatted switch/help rendering
  (FS-092.5); an `Arguments:` section listing each positional argument name with its
  word-wrapped help; and an optional epilog (word-wrapped).
- Every module automatically carries a built-in `-h` / `--help` boolean option; when set
  (and automatic help is enabled for that module) the module prints its help and exits
  before running.

### FS-092.4: Option parsing (argparse-style)

**Description**: The system shall parse command-line options and positional arguments
into structured option and argument maps, supporting long/short switches, attached and
separated values, several accumulation actions, and type coercion.

**Acceptance Criteria**:
- Parsing walks the argument vector (excluding the program name). A token containing `=`
  is split at the first `=`; the right-hand side is pushed back onto the stream as the
  value for the preceding switch (so `--input=FILE` and `--input FILE` are equivalent).
- A token is matched against each registered option by exact short or long form; on a
  match the option consumes its value (if it takes one) from the remaining stream.
- A token that matches no option and does not begin with `-` is collected as a
  positional argument (in order).
- After parsing, each declared positional argument is bound by index to a name; a missing
  required positional argument produces an option error (FS-092.5) naming it as required.
- After parsing, every declared option that was not supplied on the command line is
  populated with its declared default value.
- Value handling honors the option's `action`:
  - `store` (default): store the next token as the value.
  - `store_true` / `store_false`: take no value; store the corresponding boolean.
  - `store_const`: take no value; store the option's declared constant.
  - `append`: accumulate repeated occurrences into a list (each occurrence appends one
    value), enabling multi-valued options (e.g. `--table` repeated).
- When an option declares type `int`, its consumed value is coerced to an integer.
- A consumed value whose first character is `-` is treated as absent (so a following
  switch is not mis-consumed as this option's value).

### FS-092.5: Option help formatting, required-argument validation & error exit

**Description**: The framework shall format each option for the help screen and shall
abort with a usage screen when a required argument is missing or an option error occurs.

**Acceptance Criteria**:
- An option renders as its short and long forms, including a metavar placeholder when it
  takes an argument; optional-argument options (`nargs` of `?`) render the metavar in
  brackets. Help text is whitespace-collapsed and word-wrapped into an aligned right
  column; over-long switch columns push the help onto the next line.
- A required positional argument that is absent triggers an option error: the framework
  prints `"Error: <name> is a required argument"`, then the full help, then terminates.
- Any option error prints `"Error: <message>"`, the help, and terminates the process.
- Output is written through thin stdout/stderr stream wrappers; normal program output
  goes to standard output and diagnostics to standard error.

### FS-092.6: Continuous deployment from a source repository

**Description**: The system shall deploy osTicket directly from a working source
repository into a target installation path, supporting dry-run preview, optional setup-
folder inclusion, and preservation of an existing relocated `include/` directory.

**Acceptance Criteria**:
- Invocation: `deploy <install-path> [--dry-run|-t] [--setup|-s] [--include|-i path]
  [--verbose|-v]`.
- `--dry-run` (`-t`) lists the files that *would* be copied without copying them.
- `--setup` (`-s`) includes the `setup/` folder in the deployment (useful for deploying
  a brand-new installation); by default `setup/` is excluded from deployment.
- The deployer locates the repository root by walking parent directories until it finds
  one containing `main.inc.php`.
- If the install path does not exist it is created (recursively); if it cannot be created
  the deployment terminates with an error.
- The deployment detects whether the target is an **upgrade** by the presence of an
  existing `main.inc.php` at the destination. On upgrade, the current `INCLUDE_DIR`
  value is read from the existing `main.inc.php` and reused; otherwise the include
  location is taken from `--include` (if given) or defaults to `<install-path>/include`.
- Everything except the `include/` directory is copied from the repo root into the
  destination, excluding `include`, version-control metadata (`.git*`), editor swap
  files (`*.sw[a-z]`), and documentation (`*.md`, `*.txt`); the `include/` contents are
  copied separately to the resolved include location, excluding the live
  `include/ost-config.php`.
- The `INCLUDE_DIR` line in `main.inc.php` is rewritten (FS-092.8) only for a fresh
  install (not an upgrade) when the include path differs from the default in-tree
  location and the run is not a dry run.

### FS-092.7: Unpack a built archive into an installation path

**Description**: The system shall unpack a built osTicket archive's `upload/` tree into
a target installation path, optionally relocating the `include/` directory to a separate
(more secure) location.

**Acceptance Criteria**:
- Invocation: `unpack <install-path> [--include|-i path] [--verbose|-v]`.
- The single required argument is the destination installation path. If it does not
  exist it is created recursively; failure to create it terminates with an error.
- The unpacker locates the build's `upload/` folder by walking parent directories until
  one containing an `upload` directory is found.
- The `--include` (`-i`) option specifies an alternate full path for the `include/`
  folder so the bulk source can live outside the web-served install path; the folder is
  auto-created if absent. The help text explicitly recommends this relocation for better
  security.
- The unpacker detects an **upgrade** by the presence of an existing `main.inc.php` at
  the destination; on upgrade the current `INCLUDE_DIR` is read before the file is
  overwritten and the include contents are unpacked to that same (possibly relocated)
  location, after which `INCLUDE_DIR` is rewritten to reflect it.
- On a fresh install, everything except `include/` is unpacked to the destination; the
  `include/` contents are unpacked either to the `--include` location (with `INCLUDE_DIR`
  rewritten to point at it) or to the default `<install-path>/include`.

### FS-092.8: `INCLUDE_DIR` rewriting & `ROOT_DIR`-relative resolution

**Description**: When the source `include/` directory is relocated, the system shall
rewrite the `INCLUDE_DIR` definition in the destination `main.inc.php` to point at the
new location, preferring a portable `ROOT_DIR`-relative form.

**Acceptance Criteria**:
- The rewriter reads `main.inc.php` line by line and finds the line defining
  `INCLUDE_DIR`.
- If the new include path is a subpath of the install destination, the definition is
  written relative to `ROOT_DIR` (the destination root): `define('INCLUDE_DIR', ROOT_DIR
  . '<relative>'); // Set by installer`.
- Otherwise the definition is written with the new path as an absolute literal.
- Exactly the first matching `INCLUDE_DIR` definition line is replaced; the file is
  re-written in place. A failed write terminates with an error
  (`"Unable to configure location of INCLUDE_DIR in main.inc.php"`).
- Reading the current `INCLUDE_DIR` during an upgrade is done by extracting and
  evaluating the `INCLUDE_DIR` definition line(s) from the existing `main.inc.php`,
  having first ensured `ROOT_DIR` is defined as the destination root so a
  `ROOT_DIR`-relative definition resolves correctly.

### FS-092.9: Idempotent copy with content-hash skip and recursion control

**Description**: The deploy/unpack copy routine shall skip files whose destination
already holds identical content, support bounded or unbounded recursion, and honor
file/folder exclusion patterns.

**Acceptance Criteria**:
- The copy routine takes a source glob, a destination root, a recursion depth, and an
  exclusion pattern (single pattern or list of patterns matched against file/folder
  paths).
- A file whose destination copy already exists and whose content hash matches the source
  is **not** re-copied (idempotent re-runs copy only changed files).
- Recursion depth controls how many directory levels are descended: `0`/false disables
  recursion; `-1` recurses without limit; positive values bound the depth. The current
  directory entries `.` and `..` are never descended.
- When dry-run is active, destinations are listed but no directories are created and no
  files are copied; verbose mode (also implied by dry-run) writes each affected
  destination path to standard output.
- Destination directories are created (recursively) on demand when a file needs to be
  written into a not-yet-existing folder.

### FS-092.10: Release packager — pre-flight, staging, version stamping & archiving

**Description**: The system shall build distributable release archives by running the
regression tests, assembling a staged build tree, version-stamping it, hardening it for
production, and producing `.tar.bz2` and `.zip` archives.

**Acceptance Criteria**:
- The packager runs only from the command-line SAPI; otherwise it terminates with
  `"Only command-line packaging is supported"`.
- The packager locates the repository root by walking parent directories until one
  containing `main.inc.php` is found.
- It first runs the regression test harness; if the harness reports any failures the
  packager aborts with `"Regression tests failed. Cowardly refusing to package"` and
  produces no archive.
- It (re)creates a clean `stage/` directory: if `stage/` already exists, all files are
  removed and all subdirectories are removed deepest-first before re-staging.
- The version label is obtained from the source-control description (`git describe`) and
  is used both for the version stamp (below) and the archive file names.
- After staging and stamping, the staged tree is archived twice — once as a bzip2-
  compressed tar (`osTicket-<version>.tar.bz2`) and once as a zip
  (`osTicket-<version>.zip`).

### FS-092.11: Release packager — staged layout (`stage/upload` tree)

**Description**: The packager shall assemble the staged tree to mirror the shipped
archive layout, separating the web-served `upload/` tree, the standalone `scripts/`
folder, and top-level license/documentation.

**Acceptance Criteria**:
- Application source is staged under `stage/upload/`: root `*.php` files and `web.config`;
  the client interface asset folders (`assets`, `css`, `images`, `js`, excluding `*less`
  sources); the `api/` and `pages/` trees; the knowledge base (`kb/*.php`); the staff
  control panel (`scp/*.php` plus its `css`/`images`/`js`); the bulk source `include/`
  tree (excluding the live `ost-config.php` and editor swap files); and the installer
  (`setup/` PHP/text/HTML files plus its asset folders and its schema-stream `*.sql`
  files, excluding the `scripts`, `test`, and `stage` subtrees).
- Operator helper scripts are staged into a top-level `stage/scripts/` folder (from
  `setup/scripts/*`, excluding the `*stage` template), **outside** the `upload/` tree.
- License and documentation files (`*.txt`, `*.md`) are staged at the stage root; staged
  Markdown files are then renamed to `.txt`.
- Exclusion patterns applied during staging include editor swap files (`*.sw[a-z]`),
  Less sources (`*less`), the live config file (`*ost-config.php`), and the
  setup `scripts`/`test`/`stage` subtrees.

### FS-092.12: Release packager — version stamp & production error-display hardening

**Description**: After staging, the packager shall rewrite version and error-display
settings across the staged build so the shipped product carries the release version and
suppresses on-screen error output — the inverse of the install-time tuning.

**Acceptance Criteria**:
- Across every `*.inc.php` in the staged tree, the packager rewrites the `THIS_VERSION`
  definition to the resolved release version label (FS-092.10), preserving the original
  indentation.
- In the same pass it rewrites any `display_errors` initialization to `0` and any
  `display_startup_errors` initialization to `0`, preserving indentation.
- This is the deliberate inverse of the install-time behavior: the installer bootstrap
  (FS-060.1) forces `display_errors`/`display_startup_errors` **on** so install failures
  are visible, whereas the shipped/packaged build forces them **off** so a production
  deployment does not leak errors to end users (see KL-092-04 and FS-001 runtime
  hardening).

### FS-092.13: Backup importer — stream input, header verification & restore loop

**Description**: The system shall import an osTicket database backup produced by the
FS-090.27 exporter, verifying the dump header before reconstructing each table in turn.

**Acceptance Criteria**:
- Invocation: `import [--input|-i FILE] [--compress|-z] [--table|-t TABLE]…
  [--drop|-D]`.
- `--input` (`-i`) names the input file or stream; the default reads from standard input.
- `--compress` (`-z`) reads zlib-compressed input (matching the exporter's `-z`).
- `--table` (`-t`) may be supplied repeatedly to name specific tables to restore
  (accumulated via the `append` action); the default is to restore all tables.
- `--drop` (`-D`) emits `DROP TABLE IF EXISTS` before each create statement.
- The importer boots the full application (`main.inc.php`) and the JSON helper before
  reading, so it can resolve the configured table prefix and database helpers.
- The importer opens the (optionally compressed) input stream; an unopenable stream
  writes `"Unable to open input stream"` and terminates.
- The header is verified first (FS-092.14); a failed verification terminates with
  `"Unable to verify backup header"`.
- After header verification the importer loops, importing one table at a time
  (FS-092.15) until the table loop returns false (end of stream / unreadable block),
  then closes the stream.

### FS-092.14: Backup importer — dump-format consumption (block framing)

**Description**: The importer shall consume the exporter's record-separated,
JSON-encoded block format (FS-090.27) and validate the backup signature and database
type.

**Acceptance Criteria**:
- A *block* is read by accumulating bytes from the stream up to (and excluding) the
  record-separator byte (`\x1e`), then JSON-decoding the accumulated text into a
  structured value (matching the exporter's `\x1e`-terminated JSON blocks, FS-090.27).
- A non-empty block that fails to decode writes `"Unable to read block from input"` and
  terminates; an empty trailing block yields no value (end of stream).
- The first block read is the header; verification requires the first header element to
  equal the backup signature (`osTicket-Backup`, FS-090.27), else it writes
  `"Header mismatch -- not an osTicket backup"` and fails.
- The metadata record following the header must report database type `mysql`; any other
  type writes `"Only mysql imports are supported currently"` and fails.
- The metadata record is retained as the source installation info; in particular its
  recorded source table prefix is used when reconstructing index DDL (FS-092.16).

### FS-092.15: Backup importer — per-table reconstruction loop

**Description**: For each table block in the dump, the importer shall emit the table's
`CREATE TABLE` and `CREATE INDEX` statements and then stream its rows until the table's
end marker.

**Acceptance Criteria**:
- Importing a table reads the next block; a missing block ends the loop, and a block that
  is not a `table` block writes `"Unable to read table header"` and ends the loop.
- A progress line `"Importing table: <name>"` is written to the error stream.
- The importer then emits the table's `CREATE TABLE` (FS-092.16) and its `CREATE INDEX`
  statements (FS-092.16), then reads successive row blocks, buffering each into batched
  inserts (FS-092.16) until an `end-table` block is read, at which point a final flush is
  forced and the table import returns success.
- Reaching end-of-stream without an `end-table` marker returns failure for that table.

### FS-092.16: Backup importer — DDL reconstruction & batched INSERT emission

**Description**: The importer shall reconstruct `CREATE TABLE`, `CREATE INDEX`, and
batched multi-row `INSERT` SQL purely from the column/index metadata and row data carried
in the dump, applying the locally configured table prefix.

**Acceptance Criteria**:
- **CREATE TABLE**: A create statement is built against the locally configured table
  prefix concatenated with the dumped table name. For each dumped column it emits
  `` `<Field>` <Type> `` plus `NOT NULL` when the column is non-nullable,
  `DEFAULT CURRENT_TIMESTAMP` for that special default, a sanitized `DEFAULT <value>` for
  any other non-null default, and the column's `Extra` clause. Primary-key columns are
  assembled (in index sequence, descending where the dumped collation is not ascending)
  into a `PRIMARY KEY (...)` clause. The table is created with `DEFAULT CHARSET=utf8`.
  When `--drop` is set, a `DROP TABLE IF EXISTS` is emitted first.
- **CREATE INDEX**: For each non-primary index, columns are grouped by index name (in
  sequence, descending where the dumped collation is not ascending) and emitted as
  `CREATE [UNIQUE] INDEX <name> USING <type> ON <prefixed-table> (<cols>)`. The target
  table name is derived by stripping the dump's **source** table prefix from the recorded
  index table name and re-prefixing with the local prefix.
- **Batched INSERT**: Rows are buffered into a multi-row `INSERT INTO <prefixed-table>
  (<cols>) VALUES (...),(...)…`. Each value is emitted as the numeric literal when
  numeric, as a hexadecimal binary literal (`0x…`) when a non-empty non-numeric value,
  or as an empty string literal when empty/null. The buffer is flushed when the
  accumulated value length exceeds ~16 KB or when a forced flush occurs at end-of-table,
  after which the insert header and buffer are reset.
- **Statement sink**: Every reconstructed statement is either executed directly against
  the live database (when a "prime-time" / live mode is in effect) or written to standard
  output terminated with `;` and a newline (the default, per the module epilog: "The SQL
  of the import is written to standard output").
- **TRUNCATE path**: A restore-into-existing-table helper can emit
  `TRUNCATE TABLE <prefixed-table>` followed by `DROP INDEX IF EXISTS` for each
  non-primary index (used when restoring over an existing schema rather than creating it
  fresh).

---

## Business Rules

### BS-092-01: CLI-only execution

**Rule**: Both the management front controller and the release packager refuse to run
unless invoked from the command-line SAPI.

**Rationale**: These tools manipulate the filesystem and database outside any request
authorization context; exposing them to a web request would be a remote-execution risk.

**Examples**:
- `manage.php` from a web request → `"Management only supported from command-line"`.
- `package.php` from a web request → `"Only command-line packaging is supported"`.

### BS-092-02: Action token is consumed before module parsing

**Rule**: The dispatcher removes the matched action name from the argument vector before
delegating, so the target module's parser sees only its own options/arguments.

**Rationale**: Without this, the action name would be re-collected by the module as a
spurious positional argument and could fail required-argument validation or mis-bind.

### BS-092-03: Module self-registration

**Rule**: A module becomes available purely by being present as `modules/*.php` and
calling the static `register(action, class)` at load; there is no central manifest of
actions.

**Rationale**: New tooling can be dropped in without editing the dispatcher.

### BS-092-04: Idempotent copy via content-hash skip

**Rule**: Deploy/unpack skip copying any file whose destination already holds
byte-identical content (content-hash match).

**Rationale**: Re-running a deploy after a small change copies only what changed,
minimizing churn and downtime.

### BS-092-05: Include relocation prefers `ROOT_DIR`-relative form

**Rule**: When the relocated `include/` path is a subpath of the install root, the
rewritten `INCLUDE_DIR` is expressed relative to `ROOT_DIR` rather than as an absolute
literal.

**Rationale**: A portable relative definition survives moving the install root; an
absolute literal would have to be hand-edited.

### BS-092-06: Upgrade preserves the existing include location

**Rule**: When a destination already contains `main.inc.php`, deploy/unpack treat the
operation as an upgrade and reuse the include location currently recorded in
`main.inc.php` instead of re-deriving it from defaults or options.

**Rationale**: An administrator who relocated `include/` for security must not have that
relocation silently reverted by an upgrade.

### BS-092-07: Tests gate the release

**Rule**: The packager runs the regression test harness first and refuses to build any
archive if any test fails.

**Rationale**: A failing build must never be shipped; the "cowardly refusing to package"
guard makes this a hard stop, not a warning.

### BS-092-08: Shipped builds suppress error display (inverse of install-time)

**Rule**: The packager forces `display_errors=0` and `display_startup_errors=0` across
the staged build, the deliberate inverse of the installer bootstrap which forces them on.

**Rationale**: Visible errors aid installation/debugging but would leak internal detail
in a production deployment; the difference between the dev tree and the shipped product
is exactly this transform plus the `THIS_VERSION` stamp.

### BS-092-09: Only MySQL backups are importable

**Rule**: The importer accepts a dump only when the header signature matches and the
recorded database type is `mysql`; any other database type is rejected.

**Rationale**: The DDL reconstruction emits MySQL-flavored SQL (backtick quoting,
`USING <type>` index syntax, `DEFAULT CHARSET=utf8`); other dialects are not supported.

### BS-092-10: Table prefix is re-applied locally on import

**Rule**: Reconstructed DDL/DML applies the *local* installation's configured table
prefix, stripping the dump's recorded source prefix from index table names first.

**Rationale**: A backup taken from one installation can be restored into another whose
table prefix differs.

### BS-092-11: Binary-safe row values via hex literals

**Rule**: Non-numeric, non-empty row values are emitted as hexadecimal binary literals
(`0x…`) rather than quoted strings.

**Rationale**: This is binary-safe and avoids charset/escaping ambiguity for arbitrary
column content (including serialized blobs).

### BS-092-12: Batched inserts bounded by accumulated value length

**Rule**: Row inserts are buffered and flushed whenever the accumulated value text
exceeds ~16 KB, or forcibly at end-of-table.

**Rationale**: Bounding the per-statement size keeps each `INSERT` under server packet
limits while still amortizing statement overhead across many rows.

### BS-092-13: Default import sink is SQL to stdout, not live execution

**Rule**: By default the importer writes reconstructed SQL to standard output; it
executes against the live database only when a live ("prime-time") mode is in effect.

**Rationale**: Emitting SQL is non-destructive and lets an operator review or pipe the
restore; direct execution is opt-in.

---

## Data Requirements

The CLI tooling consumes/produces the following data, described functionally:

- **Option model** — per declared option: short form, long form, help text, action
  (`store`/`store_true`/`store_false`/`store_const`/`append`), destination key, value
  type (`string`/`int`), constant (for `store_const`), default value, metavar
  placeholder, and argument count.
- **Module registry** — a global map from action name to module instance.
- **Backup dump (input to importer; format owned by FS-090.27)** — a stream of
  `\x1e`-terminated JSON blocks: a header block (first element = backup signature
  `osTicket-Backup`, backup version), a metadata record (application version, source
  table prefix, secret salt, database type, upgrade-stream list), then per table a
  `table` block (table name with source prefix stripped, column schema list, index
  list), zero or more row blocks (positional value arrays), and an `end-table` block.
  Column schema fields consumed: `Field`, `Type`, `Null`, `Default`, `Extra`. Index
  fields consumed: `Key_name`, `Column_name`, `Seq_in_index`, `Collation`, `Non_unique`,
  `Index_type`, `Table`.
- **Staged build tree (packager output)** — `stage/upload/<app-tree>`,
  `stage/scripts/`, and top-level license/docs, version-stamped and error-display-
  hardened, then archived as `osTicket-<version>.tar.bz2` and `osTicket-<version>.zip`.
- **`main.inc.php` (deploy/unpack target)** — the single line defining `INCLUDE_DIR`,
  read on upgrade and rewritten on relocation (FS-092.8). The constant `THIS_VERSION`
  (read/rewritten by the packager) and `display_errors`/`display_startup_errors`
  initializations are also operated on by the packager.

---

## User Interactions (Flows)

### Flow A — List available actions
1. Operator runs `manage.php --help`.
2. The dispatcher loads all modules and prints its own usage plus a one-line summary per
   registered action.

### Flow B — Run an action
1. Operator runs `manage.php <action> [options] [args]`.
2. The dispatcher strips the action token, loads modules, resolves the action, and
   delegates; the module parses its own options/arguments and runs, printing its own
   help on `--help` or on a required-argument error.

### Flow C — Build a release
1. Maintainer runs `package.php` from a shell at/under the repo.
2. The packager runs the test harness (aborting on any failure), wipes and rebuilds
   `stage/`, copies the shipped layout, stamps `THIS_VERSION` and forces error display
   off, then writes the `.tar.bz2` and `.zip` archives named for the `git describe`
   version.

### Flow D — Deploy / unpack into an install path
1. Operator runs `manage.php deploy <path>` (from a source repo) or
   `manage.php unpack <path>` (from a built archive), optionally with
   `--include <secure-path>`, `--dry-run`, `--setup`, or `--verbose`.
2. The tool creates the destination if needed, detects upgrade-vs-fresh by an existing
   `main.inc.php`, copies the tree (skipping unchanged files), places `include/` at the
   resolved location, and rewrites `INCLUDE_DIR` when relocated.

### Flow E — Restore a backup
1. Operator pipes/points a backup dump into `manage.php import` (optionally `-z` for
   compressed input, `-D` to drop first, `-t TABLE` to limit tables).
2. The importer boots the app, verifies the header (signature + `mysql`), and for each
   table emits `CREATE TABLE`, `CREATE INDEX`, and batched `INSERT` SQL to standard
   output (default) or executes it directly (live mode).

---

## Edge Cases & Error Scenarios

### EC-092-01: Non-CLI invocation
Running `manage.php` or `package.php` outside the CLI SAPI terminates immediately with a
CLI-only message; nothing is executed.

### EC-092-02: Unknown action
An unrecognized action name writes `"Unknown action given"` to standard error and then
prints the aggregated help; the process does not run any module.

### EC-092-03: Missing required argument
`deploy`/`unpack` with no install-path (or any module missing a required positional
argument) prints `"Error: <name> is a required argument"` plus the module help and exits.

### EC-092-04: Value mis-consumption guard
A value token beginning with `-` is treated as absent for the preceding option, so a
following switch is not swallowed as that option's argument.

### EC-092-05: Destination cannot be created
If the deploy/unpack install path does not exist and cannot be created, the operation
terminates with `"Destination path does not exist and cannot be created"` (or, for the
relocated include folder, `"Unable to create folder for include/ files"`).

### EC-092-06: `INCLUDE_DIR` rewrite failure
If `main.inc.php` cannot be written after computing the new include path, the tool
terminates with `"Unable to configure location of INCLUDE_DIR in main.inc.php"`.

### EC-092-07: Re-run skips identical files
Re-running a deploy/unpack against an already-current destination copies only files whose
content hash differs; identical files are silently skipped (verbose/dry-run still lists
intended targets).

### EC-092-08: Tests fail during packaging
Any regression-test failure aborts packaging with
`"Regression tests failed. Cowardly refusing to package"`; no `stage/` archive is
produced.

### EC-092-09: Backup header mismatch
A dump whose first block does not match the backup signature is rejected with
`"Header mismatch -- not an osTicket backup"`, then a top-level
`"Unable to verify backup header"` termination.

### EC-092-10: Non-MySQL backup
A dump whose metadata reports a database type other than `mysql` is rejected with
`"Only mysql imports are supported currently"`.

### EC-092-11: Unreadable / undecodable block
A non-empty block that fails to JSON-decode writes `"Unable to read block from input"`
and terminates the import; an unopenable input stream writes `"Unable to open input
stream"` and terminates.

### EC-092-12: Table block out of order
If, during the table loop, the next block is not a `table` block, the importer writes
`"Unable to read table header"` and ends the loop (no further tables imported).

### EC-092-13: Truncated table stream
A table whose row stream ends without an `end-table` marker returns failure for that
table, terminating the import loop without a final flush of that table's buffered rows.

---

## Dependencies

- **FS-090.27 (Full-Database Backup Exporter)** — owns the dump format the importer
  consumes (`\x1e`-framed JSON blocks, signature, metadata record, table/row/end-table
  blocks). FS-092 documents the restore counterpart only.
- **FS-060.1 (Installer bootstrap)** — owns the `THIS_VERSION` constant and the install-
  time `display_errors=on` tuning that the packager inverts; the packager also bundles
  the installer (`setup/`) into the shipped `upload/` tree.
- **FS-001 (App bootstrap / request lifecycle)** — owns `main.inc.php`, the `INCLUDE_DIR`
  and `ROOT_DIR` constants the deploy/unpack tooling rewrites, and the runtime-hardening
  posture the packaged build adopts.
- **FS-091 (Reference data & data model)** — owns the canonical table names/schemas; the
  importer reconstructs DDL solely from dump metadata, not from this authority.
- **The regression test harness (`setup/test/**`)** — invoked by the packager as a
  release gate; classified as a test harness and not specified here.
- **Source-control description (`git describe`)** — supplies the release version label.
- **External archivers (`tar`/`bzip2`, `zip`) and stream filters (zlib)** — used to
  produce/consume compressed archives and streams.

---

## Known Limitations

### KL-092-01: No transactional / atomic restore
The importer emits or executes statements sequentially with no surrounding transaction
or rollback; a failure partway leaves a partially restored database (or partially
emitted SQL).

### KL-092-02: `change_include_dir` is line-pattern based
Reading the existing `INCLUDE_DIR` during an upgrade evaluates the matching definition
line(s) directly; an installation whose `INCLUDE_DIR` value references another
variable/define (rather than a literal or a `ROOT_DIR`-relative literal) is not handled
(noted in-source).

### KL-092-03: Packager couples to host shell utilities
Staging version-stamping and error-display hardening, plus archive creation, shell out to
host utilities (find/sed/xargs/tar/zip); the packager assumes a Unix-like toolchain and
a working `git`.

### KL-092-04: Release/dev divergence is an unenforced transform
The only behavioral difference the packager introduces between the dev tree and the
shipped product is the `THIS_VERSION` stamp and the forced `display_errors`/
`display_startup_errors=0`. Nothing verifies the shipped build actually carries these;
running the dev tree in production would leave errors visible (the install-time default).

### KL-092-05: Single-prefix index reconstruction
Index DDL reconstruction strips exactly the dump's recorded source prefix before
re-applying the local prefix; a dump whose recorded prefix does not match the actual
prefix embedded in its index `Table` values would produce malformed target names.

### KL-092-06: `--table` filtering is declared but not enforced in the loop
The importer accepts repeated `--table` options to limit the restore, but the per-table
loop carries an in-source `TODO` for honoring included/excluded tables; in practice all
tables present in the stream are processed.

### KL-092-07: Build layout is hard-coded
The shipped staging layout (which folders go where, which patterns are excluded) is
encoded directly in the packager script; adding a new top-level asset directory requires
editing the packager.
