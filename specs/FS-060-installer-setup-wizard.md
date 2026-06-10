# FS-060: Installer & Setup Wizard

## Overview

The installer is the one-time, web-driven setup wizard that brings a fresh osTicket
deployment from "files uploaded" to "running helpdesk". It is reached at
`setup/install.php` and walks the operator through three sequential gates —
**prerequisite check**, **configuration-file check**, and **basic installation
form** — before executing the install: connecting to (and if necessary creating) a
MySQL database, loading the schema "stream" SQL, seeding all default data, creating
the primary administrator account, rewriting the configuration file in place, and
displaying a post-install hardening / next-steps screen.

The installer is intentionally idempotent-by-refusal: once the configuration file
has been flipped to `OSTINSTALLED = TRUE`, the wizard refuses to run again and
redirects operators to the admin panel (upgrade path) instead. The wizard is a
distinct subsystem from the runtime application (it has its own bootstrap
`setup/setup.inc.php` and its own include directory `setup/inc/`) and from the
**upgrader** (FS-061), which handles migration of an already-installed schema.

Source: `setup/index.php`, `setup/install.php`, `setup/setup.inc.php`,
`setup/inc/class.installer.php`, `include/class.setup.php` (`SetupWizard`),
`setup/inc/*.inc.php` (step views), `setup/inc/ost-sampleconfig.php` /
`include/ost-sampleconfig.php` (config template), `setup/inc/streams/core/install-mysql.sql`
(schema + seed data).

> **Cross-spec note.** The full table schema is owned canonically by **FS-091**
> (reference data & data model). This spec documents only the seed *data* the
> installer writes and the install-time behavior. Config keys are shared with
> FS-001/FS-032/FS-091; SLA/priority/department/topic semantics are owned by
> FS-030/FS-032; the migration-stream signature mechanism is shared with FS-061.

---

## Functional Requirements

### FS-060.1: Installer bootstrap & step state machine

**Description**: The installer maintains a single-valued step pointer in the
operator's session and advances it one step at a time. The pointer is
`$_SESSION['ost_installer']['s']` with allowed values `prereq`, `config`,
`install`, `subscribe`, `done`. Each POST carries a hidden field `s` naming the
*current* step being submitted; on successful validation the pointer is advanced
to the next step. The view rendered on each request is selected from the *current*
pointer value, not the submitted one.

**Acceptance Criteria**:
- The setup bootstrap (`setup.inc.php`) starts a session, defines `THIS_VERSION`
  (`1.7-git`), forces `display_errors`/`display_startup_errors` on (so install
  failures are visible), disables `magic_quotes_gpc`, neutralizes
  `register_globals` if enabled, and defines `URL` from the request host + script
  directory and `SETUPINC=true` (a guard constant every step view checks).
- The bootstrap additionally sets `error_reporting` to `E_ALL & ~E_NOTICE`, then
  also strips `E_STRICT` (PHP ≥ 5.4) and `E_DEPRECATED | E_USER_DEPRECATED`
  (PHP ≥ 5.3) when those constants are defined (so the install screens are not
  cluttered by notices/strict/deprecation warnings while still surfacing real
  errors), and sets `session.use_trans_sid=0` and `session.cache_limiter=nocache`
  on the installer session.
- `URL` is computed as the request scheme (`https` when `$_SERVER['HTTPS']=='on'`,
  else `http`) + `HTTP_HOST` + the script directory (`dirname(PHP_SELF)`), with a
  trailing `setup` segment right-trimmed off (so `URL` is the helpdesk root rather
  than the `setup/` directory). It ends with a trailing path separator and is used
  verbatim as the read-only Helpdesk URL on the install form and as
  `helpdesk_url` (EC-060-12).
- The setup bootstrap loads the database driver conditionally: `mysqli.php` when
  the `mysqli` extension is loaded, otherwise `mysql.php` (EC-060-02). It also
  pre-loads the validator, password, format, and misc helper classes used by the
  install routine.
- `setup/index.php` is a one-line shim that simply `require`s `install.php`; both
  entry points reach the identical wizard.
- Each step view begins with `if(!defined('SETUPINC')) die('Kwaheri!')` so the
  partials cannot be requested directly.
- With no prior session state the default view is the prerequisite screen
  (`install-prereq.inc.php`).
- The valid forward transitions are: `prereq` → `config`, `config` → `install`,
  `install` → `done` (the `subscribe` step exists in code but is marked TODO/RC
  and is not reached by the install POST handler, which jumps install → done
  directly).
- A separate GET branch exists for the dead `subscribe` step: a request with
  `?s=ns` ("No thanks." link) while the pointer is already `subscribe` advances
  the pointer to `done`. This is the only GET-driven transition; all other
  advances are POST-driven (KL-060-02).
- The `subscribe` POST handler (when reached) requires a non-empty trimmed `name`,
  a valid `email`, and at least one of `alerts`/`news` checked ("Check one or
  more"); on success the pointer advances to `done`. The `subscribe.inc.php` view
  pre-fills its fields from `$_SESSION['info']` (the admin display name + email
  captured at install time) and defaults both notification checkboxes to checked.

### FS-060.2: Prerequisite check (step `prereq`)

**Description**: Before installation the wizard verifies the server meets minimum
requirements and reports recommended-but-optional extensions.

**Acceptance Criteria**:
- Two **required** checks, each rendered with a pass/fail (`yes`/`no`) indicator:
  - **PHP version** ≥ `4.3` — pass when `version_compare(PHP_VERSION, '4.3') >= 0`;
    the running `PHP_VERSION` is displayed.
  - **MySQL** — pass when the `mysql` PHP extension is loaded
    (`extension_loaded('mysql')`); displays "module loaded" or "missing!". The
    label advertises "MySQL v4.4 or greater"; the actual runtime version check
    against `4.4` is enforced later at DB-connect time (FS-060.6), not here.
- Two **recommended** checks (informational only, never block): `gd` extension and
  `imap` extension, each shown with a pass/fail indicator.
- Submitting the prereq form runs `check_prereq()` = `check_php() && check_mysql()`.
  On pass, the pointer advances to `config`; on fail, the prereq screen redisplays
  with the error "Minimum requirements not met!".

### FS-060.3: Configuration-file check (step `config`)

**Description**: The installer requires a writable runtime config file
`include/ost-config.php`. This step ensures the file exists, is a clean
(uninstalled) template, and is writable before the install form is shown.

**Acceptance Criteria**:
- The config file path is `../include/ost-config.php` (constant
  `OSTICKET_CONFIGFILE`).
- The wizard selects one of four config-related views by checking, in order:
  1. File does not exist → `file-missing.inc.php` (instructs operator to copy
     `include/ost-sampleconfig.php` to `include/ost-config.php`).
  2. File unreadable, empty, OR already contains
     `define('OSTINSTALLED',TRUE);` → `file-unclean.inc.php` ("osTicket is already
     installed?").
  3. File not writable → `file-perm.inc.php` (chmod-to-`0666` instructions).
  4. All checks pass → `install.inc.php` (the basic-install form).
- Submitting the config step re-runs the existence + writability checks. If the
  file is missing it errors "Configuration file does NOT exist…"; if not writable
  it errors "Write access required to continue"; otherwise the pointer advances to
  `install`.
- The `file-missing` and `file-perm` views POST `s=config` ("Continue") so the
  operator can re-check after fixing permissions/copying the template.

### FS-060.4: Basic installation form (step `install` — input collection)

**Description**: The install form collects all data required to provision the
helpdesk in three labelled groups: System Settings, Admin User, Database Settings.
All fields are required.

**Acceptance Criteria**:
- **System Settings**: read-only Helpdesk URL (the auto-detected `URL`), Helpdesk
  Name (`name`), Default Email (`email`).
- **Admin User**: First Name (`fname`), Last Name (`lname`), Email Address
  (`admin_email`), Username (`username`), Password (`passwd`), Retype Password
  (`passwd2`).
- **Database Settings**: MySQL Table Prefix (`prefix`, defaulted to `ost_`),
  MySQL Hostname (`dbhost`, defaulted to `localhost`), MySQL Database (`dbname`),
  MySQL Username (`dbuser`), MySQL Password (`dbpass`).
- On a re-displayed form after a validation error, all previously entered values
  are re-populated (HTML-escaped) except where corrected; on first display the
  prefix/host defaults are shown.
- Submitting POSTs `s=install` and triggers the install routine (FS-060.5/.6/.7).

### FS-060.5: Install-form field validation

**Description**: Before touching the database the installer validates every field
and a set of cross-field rules; any failure aborts with field-level errors and the
form is redisplayed.

**Acceptance Criteria** (each error returns the operator to the form):
- Required + type validation per field: `name` (string), `email` (valid email),
  `fname` (string), `lname` (string), `admin_email` (valid email), `username`
  (username format), `passwd` (password, ≥ 5 chars min per validator), `passwd2`
  (string), `prefix`/`dbhost`/`dbname`/`dbuser`/`dbpass` (string). A generic
  fallback error "Missing or invalid data - correct the errors and try again."
  is shown if field validation fails without a specific message. This generic
  fallback is set inside the install routine itself. There is also a *second*,
  outer fallback set by the step dispatcher: if `install()` returns false but did
  not register any `err` key, the dispatcher injects "Error installing osTicket -
  correct the errors below and try again." So a failed install always surfaces at
  least one top-level error string even when the routine returned silently.
- Cross-field rules (see Business Rules):
  - Admin email must not equal the system Default Email (BS-060-04).
  - `passwd` must equal `passwd2` (BS-060-05).
  - Table prefix must end with an underscore `_` (BS-060-06).
  - Username must not be one of the reserved/predictable names (BS-060-07).
  - If `dbhost` carries a `:port`, the port (if numeric) must be in 1–65535
    (BS-060-08).

### FS-060.6: Database connection, version & prefix-collision check

**Description**: After field validation passes, the installer connects to MySQL,
enforces the minimum server version, selects or creates the target database, and
guards against installing over an existing osTicket with the same table prefix.

**Acceptance Criteria**:
- Connect using `dbhost`/`dbuser`/`dbpass`; on failure error
  "Unable to connect to MySQL server." plus the driver error.
- Enforce minimum MySQL version `4.4`: if the server version is lower, error
  "osTicket requires MySQL 4.4 or better!". The comparison is performed by
  splitting both the reported server version and the required `4.4` on `.` and
  comparing the resulting arrays element-by-element, not via a numeric/semantic
  version compare — a coarse check that can misorder unusual version strings
  (KL-060-09).
- Select `dbname`; if it does not exist, attempt to create it. If neither selecting
  nor creating succeeds, error "Database doesn't exist" / "Unable to create the
  database." If select still fails after create, error "Unable to select the
  database".
- **Prefix-collision guard**: run `SELECT * FROM <prefix>config LIMIT 1`. If it
  succeeds (a `config` table already exists under that prefix), abort with
  "another installation with same table prefix exists!" and field error
  "Prefix already in-use" (BS-060-09).
- On a clean database, best-effort set the database default charset/collation to
  `utf8` / `utf8_general_ci` (failure is non-fatal).

### FS-060.7: Schema load, default seeding, admin & config provisioning

**Description**: With a validated database the installer loads the schema SQL
stream(s), verifies each stream's cryptographic signature, seeds default data,
creates the administrator, writes runtime config rows, and rewrites the config
file. The whole routine returns true only if every step succeeded.

**Acceptance Criteria**:
- Define `ADMIN_EMAIL` and `PREFIX` from the submitted values (needed for in-install
  SQL error reporting).
- Re-verify the config file is readable and openable for writing before loading
  schema (errors `#2`/`#3`).
- The stream list is built per the `streams.cfg` discovery rule **owned by FS-061.2
  (BS-061-15)** (one stream/line, `#` comments, requires `<stream>.sig` + `<stream>/`
  dir, defaults to single stream `core`). Install-time fact retained here: the
  "recorded signature" for each registered stream is the trimmed *contents of its
  `.sig` file* — it is NOT computed at install time (BS-060-10).
- For each registered stream: locate the schema file
  `setup/inc/streams/<stream>/install-mysql.sql`. If the file is missing or cannot
  be opened, abort with a per-stream "Internal Error - please make sure your
  download is the latest (#1)". Otherwise compute the `md5` of its contents and
  compare (case-insensitively) against the stream's recorded `.sig` signature; on
  mismatch abort with an "Unknown or invalid schema signature" error that echoes
  both the expected and computed hashes (BS-060-10). If present and the signature
  matches, load it via `load_sql_file()`.
- `load_sql_file()`/`load_sql()` strip `#`/`--` comments, replace every
  `%TABLE_PREFIX%` token with the chosen prefix, set `SQL_MODE=""`, then execute
  the statements split on `;`. Any statement failure aborts the parse (error `#4`).
- Loading the core schema both creates all tables AND seeds default data (kinds of
  seeded rows listed under Data Requirements; the literal seed VALUES are canonical
  in FS-091 "Seeded Reference Rows").
- Read back the seed key IDs: first SLA id, first department id, first email
  template group id, first group id, and the timezone id whose `offset = -5.0`
  (US Eastern), to wire the admin and config rows (BS-060-11).
- **Create the admin staff row** (BS-060-12): `isactive=1`, `isadmin=1`,
  `group_id` = first group (Admins), `dept_id` = first department, `timezone_id`
  = Eastern, `max_page_size=25`, with the submitted email/first/last/username and
  the bcrypt/phpass hash of the password. Failure → error `#6`.
- **Create the three default system email addresses** from the Default Email's
  local domain (BS-060-13): `Support` = the entered email; `osTicket Alerts` =
  `alerts@<domain>`; an unnamed `noreply@<domain>`.
- **Write runtime config** (BS-060-14): UPDATE the `core` namespace config rows for
  `isonline=0`, `default_email_id` (= Support email id), `alert_email_id` (= alerts
  email id), `default_dept_id`, `default_sla_id`, `default_timezone_id` (Eastern),
  `default_template_id`, `admin_email`, `schema_signature` (= core stream
  signature), `helpdesk_url` (= URL), `helpdesk_title` (= entered Helpdesk Name).
  For every non-core stream, INSERT a `schema_signature` config row in that stream's
  namespace. Any failure → error `#7`.
- After all DB work succeeds, **rewrite the config file last** (so a mid-install
  failure leaves the installer re-runnable — BS-060-15): flip `OSTINSTALLED` to
  TRUE and substitute `%ADMIN-EMAIL`, `%CONFIG-DBHOST`, `%CONFIG-DBNAME`,
  `%CONFIG-DBUSER`, `%CONFIG-DBPASS`, `%CONFIG-PREFIX`, and `%CONFIG-SIRI` (a fresh
  32-char random secret salt). Truncate + write the file; failure → error `#5`.
- **Post-config finishing touches** (best-effort, not aborting): set every email
  row's `dept_id`, set the first department's `email_id`/`autoresp_email_id` to the
  Support email, create a welcome "osTicket Installed!" open ticket + its initial
  thread message (body from `setup/inc/msg/installed.txt` or a fallback string),
  and write a `Debug` syslog entry recording the completed install with the
  operator's IP.

### FS-060.8: Completion screen & post-install hardening (step `done`)

**Description**: On success the wizard advances to `done` and shows the
congratulations / next-steps screen, including the mandatory security-hardening
instruction to remove write access from the config file.

**Acceptance Criteria**:
- The completion view (`install-done.inc.php`) shows "Congratulations!", the
  helpdesk URL, a link to the Staff/Admin Control Panel (`../scp/admin.php`), the
  hardening instruction (change `include/ost-config.php` back to read-only, e.g.
  `chmod 0644`, via CLI / FTP / Cpanel — BS-060-16), and static links to the
  osTicket forums/wiki and post-install setup guide. It re-derives its display
  URL directly from the `URL` constant, not from session state.
- On a successful install, the step dispatcher (in `install.php`, not the done
  view) records `$_SESSION['info']` with the admin's display name
  (`ucfirst(fname . ' ' . lname)`), admin email, and URL. This session payload is
  consumed by the dead `subscribe.inc.php` view to pre-fill its form; the live
  `done` view does not read it (KL-060-02).
- Defensive fallback: if the `done` pointer is reached but the config file no longer
  exists (`config_exists()` is false), the prereq screen (`install-prereq.inc.php`)
  is shown instead of the completion screen.

### FS-060.9: Re-run protection (already-installed detection)

**Description**: The installer must refuse to overwrite a live install and must
redirect operators to the runtime/admin path.

**Acceptance Criteria**:
- The default (no-session) branch fails to the "already installed" view
  (`file-unclean.inc.php`) if ANY of these legacy/installed markers exist: legacy
  `include/settings.php`, legacy `ostconfig.php` at root, or `ost-config.php`
  containing `define('OSTINSTALLED',TRUE);` (BS-060-01).
- The runtime config template itself redirects any direct hit to the installer
  while `OSTINSTALLED != TRUE`, and dies if the installer files are missing
  (BS-060-02).
- The `config`/`install` steps independently re-detect an installed config
  (`OSTINSTALLED,TRUE` present) and route to `file-unclean.inc.php` rather than the
  install form (BS-060-03).

---

## Business Rules

### BS-060-01: Already-installed detection blocks fresh install
**Rule**: The installer treats the presence of `include/settings.php`,
root `ostconfig.php`, OR `OSTINSTALLED=TRUE` inside `ost-config.php` as proof of a
prior install and renders the "already installed" screen instead of the install form.
**Rationale**: Prevents data loss / schema clobber on a live helpdesk; routes
upgraders to the admin panel.
**Examples**:
- Re-visiting `setup/install.php` after a successful install → "osTicket is already
  installed?" with a link to `scp/admin.php`.

### BS-060-02: Runtime config self-redirects to installer until installed
**Rule**: The config template (`ost-sampleconfig.php` / `ost-config.php`) defines
`OSTINSTALLED` and, while it is FALSE, redirects every request to
`ROOT_PATH + 'setup/install.php'` (and dies "Error: Contact system admin." if that
installer file is missing). Direct access to the config file itself is denied with
`die('kwaheri rafiki!')` — the guard fires when the running script's basename
matches the config file's basename OR when the expected bootstrap constant is not
defined (so the file cannot be requested outside the boot chain). **Two template
copies exist and differ in their guard/redirect constant**: `include/ost-sampleconfig.php`
(the copy the operator renames to `ost-config.php`) guards on `INCLUDE_DIR` being
defined and carries additional commented-out optional settings (`MAIL_EOL`,
`ROOT_PATH` override, and MySQL SSL `DBSSLCA`/`DBSSLCERT`/`DBSSLKEY`); the
`setup/inc/ost-sampleconfig.php` copy guards on `ROOT_PATH` and omits those
commented blocks (EC-060-13).
**Rationale**: Guarantees an unconfigured deployment always lands in the wizard.

### BS-060-03: Install-step views re-validate clean config
**Rule**: Even mid-flow, the `config`/`install` step selector re-checks the config
file and downgrades to `file-missing` / `file-unclean` / `file-perm` if it has
become missing, already-installed, or read-only. The `file-unclean` branch also
fires when the config file is unreadable or empty (`file_get_contents` returns
falsy). Before showing `file-perm` the selector calls `clearstatcache()` so a
just-applied `chmod 0666` is re-read rather than served from PHP's stat cache.

### BS-060-04: Admin email must differ from system default email
**Rule**: `admin_email` may not equal (case-insensitive) the system Default Email
`email`. Violation → field error "Conflicts with system email above".
**Rationale**: The system addresses (Support/Alerts/noreply) and a staff identity
must be distinct to avoid mail-loop and routing confusion.

### BS-060-05: Password confirmation must match
**Rule**: `passwd` must equal `passwd2` (case-sensitive). Violation → "passwords to
not match!".

### BS-060-06: Table prefix must end with underscore
**Rule**: A non-empty `prefix` must end in `_` (e.g. `ost_`). Violation →
"Bad prefix. Must have underscore (_) at the end."
**Rationale**: All table names are formed as `<prefix><name>`; the underscore keeps
generated identifiers readable and collision-resistant.

### BS-060-07: Admin username must not be predictable
**Rule**: The lowercased `username` must not be one of `admin`, `admins`,
`username`, `osticket`. Violation → "Bad username". Username must also be ≥ 2 chars
and match the allowed character set (letters/digits/`._-`).
**Rationale**: Hardens the primary admin account against trivial credential guessing.

### BS-060-08: Database port range validation
**Rule**: If `dbhost` contains a `:port` suffix and the port is numeric, it must be
within 1–65535; otherwise field error "Invalid database port number".

### BS-060-09: Prefix-collision aborts install
**Rule**: If `SELECT * FROM <prefix>config LIMIT 1` succeeds, an installation
already occupies that prefix in the target database; the install aborts with
"another installation with same table prefix exists!" / "Prefix already in-use".
**Rationale**: Prevents two osTicket installs from sharing a table prefix in one DB.

### BS-060-10: Schema streams are signature-verified before load
**Rule**: The set of streams is discovered via the `streams.cfg` rule **owned by
FS-061.2 (BS-061-15)** (defaults to a single `core` stream; requires a `<stream>.sig`
file + `<stream>/` directory to register). Install-time verification behavior
retained here: each schema stream file's content `md5` (algorithm hard-coded to
`md5` for all streams — KL-060-10) must equal the trimmed contents of that stream's
`.sig` file. A missing/unreadable schema file aborts with "Internal Error - please
make sure your download is the latest (#1)"; a hash mismatch aborts with an
"Unknown or invalid schema signature" error that echoes the expected and computed
hashes. The verified `core` signature is persisted as the runtime
`schema_signature` (and a `schema_signature` config row is inserted per non-core
stream in its own namespace).
**Rationale**: Detects tampered/incomplete downloads and anchors the upgrader's
migration baseline (FS-061).

### BS-060-11: Default-record IDs are resolved by deterministic lookup
**Rule**: The admin row and config wiring reference the *first* SLA, department,
email-template-group, and group rows (`ORDER BY <id> LIMIT 1`) and the timezone row
with `offset = -5.0` (US Eastern) as the seeded defaults.
**Rationale**: The seed SQL inserts these in a known order; the installer binds to
them rather than hardcoding numeric IDs.

### BS-060-12: A single all-powerful admin is created
**Rule**: Exactly one staff account is created at install with `isadmin=1`,
`isactive=1`, assigned to the first group (Admins) and first department (Support),
US-Eastern timezone, `max_page_size=25`, and the bcrypt/phpass-hashed password.
**Rationale**: Bootstraps control-panel access; further staff are added post-install.

### BS-060-13: Three system email addresses are derived from the default email
**Rule**: From the entered Default Email `name@domain`, the installer seeds:
`Support` = the full entered email; `osTicket Alerts` = `alerts@<domain>`; an
unnamed `noreply@<domain>`. `default_email_id` points to Support; `alert_email_id`
points to Alerts.
**Rationale**: Provides the minimal outbound/alert mail identities the system needs.

### BS-060-14: Runtime config rows are populated from the install form
**Rule**: The installer UPDATEs the seeded `core` config rows to bind the chosen
default email/alert/department/SLA/timezone/template IDs, admin email,
`schema_signature`, helpdesk URL, and helpdesk title; and INSERTs a per-namespace
`schema_signature` for any non-core stream. `isonline` is set to `0` (helpdesk
starts offline).
**Rationale**: Turns the inert default config into a working, operator-named system,
intentionally offline until the admin reviews settings.

### BS-060-15: Config file is rewritten last for recoverability
**Rule**: The `ost-config.php` rewrite (flipping `OSTINSTALLED` to TRUE and filling
the DB credentials + secret salt) is the final step, performed only after all DB
provisioning succeeded. A failure before this point leaves `OSTINSTALLED=FALSE` so
the wizard can be safely re-run.
**Rationale**: Avoids a half-installed state that is locked out of its own installer.

### BS-060-16: Operator is instructed to remove config write access
**Rule**: The completion screen mandates removing write permission from
`include/ost-config.php` (e.g. `chmod 0644`) and reminds the operator this was
deferred from the install-time `0666` requirement.
**Rationale**: The wizard needed write access; leaving it writable post-install is a
security exposure.

### BS-060-17: A secret salt is generated per install
**Rule**: `%CONFIG-SIRI` in the config template is replaced by a freshly generated
32-character random code (`SECRET_SALT`), unique to each installation.
**Rationale**: Seeds the runtime crypto/HMAC operations with per-deployment entropy.

### BS-060-18: Welcome ticket & install syslog are best-effort
**Rule**: After config rewrite the installer creates a sample "osTicket Installed!"
open web ticket with an initial message thread and writes a `Debug`-level syslog
entry; these are non-fatal and never block completion.

---

## Data Requirements

> Schema (table/column definitions) is canonical in **FS-091**. Below is the *seed
> data the installer writes*, plus the data it collects and the config file it
> produces.

### Operator-supplied install inputs
| Field | Group | Validation |
|-------|-------|------------|
| `name` | System | required string → `helpdesk_title` |
| `email` | System | required valid email → Support system email |
| `fname`, `lname` | Admin | required strings → admin first/last name |
| `admin_email` | Admin | required valid email; ≠ `email` |
| `username` | Admin | username format; not reserved (BS-060-07) |
| `passwd`, `passwd2` | Admin | password ≥5 chars; must match |
| `prefix` | DB | required; must end `_` (default `ost_`) |
| `dbhost` | DB | required; optional `:port` 1–65535 (default `localhost`) |
| `dbname` | DB | required; created if absent |
| `dbuser`, `dbpass` | DB | required |

### Seeded default data (from `install-mysql.sql`, `core` stream)

> The literal seed VALUE tables (priority colors/urgency/public flags, the Default
> SLA grace period, the 12 email-template code-names, the group permission-flag
> matrix, the ban-list sample rule, the timezone offset list, the CMS pages, and the
> ~90 `core` config defaults) are **canonical in FS-091 "Seeded Reference Rows"**
> (priorities → FS-091.5, SLA → FS-091.6, email-template group + 12 code-names →
> FS-091.7, sample canned attachment file → FS-091.14, full config key list →
> FS-091/FS-032). This spec lists only *what kinds of rows* the core schema load
> seeds and the install-time behavior that binds them; it does not re-tabulate the
> values. Mirrors the pattern already used above for config keys.

Loading the `core` schema seeds (kinds only — values per FS-091):
- **SLA** (1): the seeded `Default SLA` row (grace period etc. → FS-091.6).
- **Departments** (2): `Support` and `Billing` (semantics → FS-030).
- **Priorities** (4): the seeded priority rows; default priority is config
  `default_priority_id = 2` (normal). (Tag/color/urgency/public values → FS-091.5.)
- **Help Topics** (2): `Support` and `Billing` (semantics → FS-030).
- **Email template group** (1) + **12 templates**: the `osTicket Default Template`
  group and its seeded template rows (code-names + bodies → FS-091.7).
- **Groups** (3) + permission flags: `Admins`, `Managers`, `Staff` (the per-group
  permission-flag matrix → FS-091 Seeded Groups).
- **Group↔department access**: every group is granted access to every seeded
  department (Cartesian seed) (D05X-5; values → FS-091).
- **Team** (1): `Level I Support`.
- **Ban-list filter** (1): the `SYSTEM BAN LIST` reserved filter + its sample
  `filter_rule` (execorder, flags, and the sample address → FS-091.9 / FS-042).
- **Canned responses** (2): the two seeded sample responses, the first carrying a
  sample attachment.
- **Sample file** (1) + chunk: the seeded `osTicket.txt` content-addressed file
  linked to the first canned response (bytes/content → FS-091.14).
- **Timezones**: the full seeded offset list; the install binds the default to
  `offset = -5.0` (Eastern) (D05X-8; offset list → FS-091).
- **CMS pages** (3): `Offline` / `Thank you` / `Landing`, each wired to a `core`
  config row (`offline_page_id`, `thank-you_page_id`, `landing_page_id`).
- **Config** (`core` namespace): the seeded default key/value rows; the installer
  then overwrites the deployment-specific subset (BS-060-14). *(Full key list owned
  by FS-091/FS-032.)*

### Records written by the installer (not by the SQL seed)
- 1 admin **staff** row (BS-060-12).
- 3 **email** rows (BS-060-13); department/email cross-wiring updates.
- Config UPDATEs/INSERTs (BS-060-14) including `schema_signature`, `helpdesk_url`,
  `helpdesk_title`, `admin_email`, and the default-id bindings.
- 1 welcome **ticket** + 1 **ticket_thread** message; 1 **syslog** `Debug` row
  (BS-060-18).

### Config file template (`ost-sampleconfig.php`) placeholders
`OSTINSTALLED` (FALSE→TRUE), `%CONFIG-SIRI` (→ 32-char `SECRET_SALT`),
`%ADMIN-EMAIL`, `%CONFIG-DBHOST`, `%CONFIG-DBNAME`, `%CONFIG-DBUSER`,
`%CONFIG-DBPASS`, `%CONFIG-PREFIX`; constants `DBTYPE='mysql'`, `TABLE_PREFIX`.

---

## User Flows / Interactions

### Flow A — Happy-path fresh install
1. Operator uploads files and browses to the deployment; the unconfigured
   `ost-config.php` redirects to `setup/install.php` (BS-060-02).
2. **Prereq screen**: required PHP/MySQL checks shown green; operator clicks
   "Continue »" → pointer `prereq` → `config`.
3. **Config screen**: if the template was already copied + writable, the install
   form renders directly. (Otherwise the operator follows the
   missing/permission instructions, fixes them, clicks "Continue", and re-checks.)
4. **Install form**: operator fills System / Admin / Database groups and clicks
   "Install Now". A "Doing stuff!" overlay is shown.
5. Installer validates fields + cross-field rules, connects/creates the DB, loads
   the signature-verified schema, seeds defaults, creates the admin + system
   emails, writes config rows, rewrites `ost-config.php`, and creates the welcome
   ticket/syslog. Pointer → `done`.
6. **Done screen**: congratulations + hardening instruction (chmod the config file
   read-only) + links to the helpdesk URL and `scp/admin.php`. Operator logs in to
   the admin panel and proceeds to post-install configuration.

### Flow B — Config file missing / not writable
1. Config step detects the missing or read-only file and renders
   `file-missing.inc.php` (copy `ost-sampleconfig.php` → `ost-config.php`) or
   `file-perm.inc.php` (`chmod 0666`).
2. Operator performs the fix and clicks "Continue" (re-POSTs `s=config`).
3. On success, the install form renders.

### Flow C — Already installed
1. Operator browses to `setup/install.php` on a live system.
2. Re-run protection (BS-060-01/03) renders `file-unclean.inc.php` with a link to
   the admin panel for the upgrade path (FS-061). No install form is shown.

---

## Edge Cases

### EC-060-01: Prereq label vs. real MySQL version check
The prereq screen only checks that the `mysql` extension is *loaded*; the actual
server-version-≥-4.4 check happens later at DB-connect (FS-060.6). A server passing
the prereq screen can still fail the install with "osTicket requires MySQL 4.4 or
better!".

### EC-060-02: `mysqli` present but `mysql` extension absent
The setup bootstrap loads `mysqli.php` when `mysqli` is available, else `mysql.php`
— but the prereq check (`check_mysql`) tests only `extension_loaded('mysql')`. A
`mysqli`-only host therefore fails the prereq gate even though the runtime could use
`mysqli`.

### EC-060-03: Database does not exist but cannot be created
If the DB user lacks `CREATE DATABASE` and the named DB is absent, the install
aborts with "Database doesn't exist" / "Unable to create the database."

### EC-060-04: Tables already present under chosen prefix
If a `<prefix>config` table already exists (prior partial/complete install in the
same DB), the install aborts with "Prefix already in-use" (BS-060-09) — the operator
must change the prefix or drop the old tables.

### EC-060-05: Schema signature mismatch / corrupted download
A modified or truncated `install-mysql.sql` produces an md5 that differs from the
registered signature; the install aborts with an "Unknown or invalid schema
signature" message and does not partially apply (within that stream; the abort is
per-stream).

### EC-060-06: Mid-install DB failure before config rewrite
Because the config file is rewritten last (BS-060-15), a failure during schema load
or seeding leaves `OSTINSTALLED=FALSE`; the wizard remains re-runnable — but
partially-created tables under the prefix will trigger EC-060-04 on retry unless the
DB is cleaned.

### EC-060-07: Config-file write fails after DB provisioning
If the final `ost-config.php` write/truncate fails (error `#5`) the database is fully
provisioned but `OSTINSTALLED` stays FALSE — the system cannot boot and the operator
must fix file permissions and re-run (hitting EC-060-04).

### EC-060-08: SQL statement failure during seed load
`load_sql()` splits the schema on `;` and aborts on the first failing statement
(error `#4`). With `display_errors` on, the raw SQL + DB error is echoed to the
operator (debug mode is hard-coded true in the installer).

### EC-060-09: Admin email equals a derived system address
Validation blocks `admin_email == email` (BS-060-04) but does not block
`admin_email` from matching the derived `alerts@<domain>` / `noreply@<domain>`
addresses; such overlap is not detected.

### EC-060-10: Reserved-username bypass via casing/whitespace
The reserved-name check lowercases but does not trim; only exact lowercase matches
of `admin`/`admins`/`username`/`osticket` are blocked. Near-variants
(e.g. `admin1`) pass.

### EC-060-11: Direct access to step partials
Every step view dies with `Kwaheri!`/`Kwaheri rafiki!` if `SETUPINC` is undefined,
preventing direct browsing of `setup/inc/*.inc.php`.

### EC-060-12: URL auto-detection trims a trailing `setup` segment
`URL` is built from the request host + the script directory, then has a trailing
`setup` right-trimmed off so the helpdesk root (not `setup/`) is recorded. Because
the trim is a literal character-set strip (`rtrim(..., 'setup')`), a deployment in
a directory whose name happens to end in those characters (e.g. `/myhelpsetup/`)
could have extra trailing characters stripped — producing a slightly wrong
auto-detected helpdesk URL that the operator cannot edit on the install form.

### EC-060-13: Two divergent config templates
The installer references the config template at two paths (`include/ost-sampleconfig.php`
and `setup/inc/ost-sampleconfig.php`) whose direct-access guards key off different
constants (`INCLUDE_DIR` vs `ROOT_PATH`) and whose comment blocks differ
(SSL/`MAIL_EOL`/`ROOT_PATH` notes present only in the `include/` copy). The
operator is instructed to copy the `include/` copy; copying the `setup/inc/` copy
instead yields a functionally equivalent but less-documented runtime config.

### EC-060-14: Welcome ticket uses fixed sentinel values
The seeded welcome ticket is inserted with hard-coded `priority_id=0`,
`topic_id=0`, source `Web`, requester email `support@osticket.com`, requester name
`osTicket Support`, and a random 6-digit `ticketID`. These reference no seeded
priority/topic rows (id 0 does not exist) and must be cleaned up alongside the
other sample data (KL-060-07).

---

## Dependencies

- **FS-091 — Reference data & data model**: canonical table schema and full config
  key catalog; this spec seeds, but does not define, those tables.
- **FS-061 — Upgrader & migration streams**: shares `DatabaseMigrater::getUpgradeStreams()`
  and the schema-signature mechanism; the upgrade path is where already-installed
  operators are sent (BS-060-01).
- **FS-001 — Bootstrap & config**: the runtime reads the `ost-config.php` the
  installer produces (`OSTINSTALLED`, DB creds, `SECRET_SALT`, `TABLE_PREFIX`) and
  the `core` config rows the installer wrote.
- **FS-002 — Staff auth**: consumes the seeded admin staff row + group/permission
  model; password hashing via `Passwd::hash` (bcrypt/phpass).
- **FS-003 — Validation/crypto/misc**: `Validator::process` (field + email +
  username validation), `Misc::randCode` (secret salt) / `Misc::randNumber`
  (welcome ticket id).
- **FS-030/FS-032 — Departments/topics/SLA/priorities**: semantics of the seeded
  routing/SLA/priority records.
- **FS-040 — Email accounts/templates**: the seeded email rows + 12 default
  templates + template group.
- **FS-050 — KB/FAQ & canned responses**: seeded canned responses + sample
  attachment; FAQ tables created (unseeded).
- MySQL server (≥ 4.4) reachable with the supplied credentials; PHP ≥ 4.3 with the
  `mysql` extension (and recommended `gd`/`imap`).

---

## Known Limitations

### KL-060-01: Installer is MySQL-only and version checks are coarse
`DBTYPE` is hard-coded `mysql`; the installer assumes a MySQL backend and uses a
loose `version_compare`/`explode('.')` comparison against `4.4`. No other database
engine is supported.

### KL-060-02: `subscribe` step is dead/RC scaffolding
The `subscribe` step (newsletter/alerts opt-in) and `subscribe.inc.php` exist with
validation logic, but the install POST handler jumps `install` → `done` directly
(TODO comment "Go to subscribe step"); the step is unreachable in normal flow.

### KL-060-03: No automatic rollback of partial installs
On a mid-install failure the installer aborts but does not drop the tables it
already created; the operator must manually clean the database before retrying or
change the prefix (interacts with EC-060-04/06).

### KL-060-04: Hard-coded debug echo of SQL errors
`$debug` is hard-coded `true` in `install()`, so raw SQL statements and DB errors
are echoed to the browser on failure — convenient for diagnosis but potentially
leaking schema/credential context to whoever runs the installer.

### KL-060-05: Helpdesk starts offline
`isonline` is seeded/forced to `0`; the freshly installed helpdesk is offline until
an admin enables it in settings. This is by design but is a common post-install
surprise.

### KL-060-06: Default timezone fixed to US Eastern
The default timezone is hard-bound to `offset = -5.0` regardless of the operator's
locale; there is no timezone selection on the install form.

### KL-060-07: Sample/test data is seeded into production
A sample ban-list rule (`test@example.com`), two sample canned responses, a sample
attachment file, and a welcome ticket are seeded into every fresh install and must
be cleaned up manually.

### KL-060-08: Config-file hardening is advisory only
Removing write access from `ost-config.php` post-install is presented as
instructions on the done screen but is not enforced or verified by the system.

### KL-060-09: MySQL version comparison is by array, not semantic
The minimum-version gate compares `explode('.', server_version)` against
`explode('.', '4.4')` as PHP arrays rather than using a version-aware comparison.
This works for normal `major.minor[...]` strings but can misorder unusual or
vendor-suffixed version strings, and ignores anything past the first two
dot-separated components.

### KL-060-10: Schema-signature algorithm is hard-coded md5
Although the streams configuration is intended to make the hash algorithm
per-stream configurable (noted by an in-code TODO), the installer hard-codes `md5`
for every stream's schema-signature verification. There is no current way to use a
stronger digest for the install-time integrity check.

### KL-060-11: Welcome ticket bypasses the normal ticket-creation pipeline
The sample "osTicket Installed!" ticket and its thread message are written with
direct SQL INSERTs rather than through the runtime ticket-creation path (FS-011/
FS-021). It therefore skips help-topic routing, autoresponses, SLA/due-date
assignment, filters, and events — it exists purely as a visible artifact in the
ticket queue.
```
