# FS-001: App Bootstrap & Shared Request Lifecycle

## Overview

The App Bootstrap & Shared Request Lifecycle specification defines the master initialization sequence that **every** web request flows through before any feature logic runs, plus the two realm-specific gate layers (client portal and staff control panel, with the admin control panel as a stricter sub-realm of staff). It is the structural spine of the application: it establishes the runtime environment (security hardening of the language runtime, default timezone, path constants, configurable table-name constants), connects to the persistence store, materializes the two long-lived singletons that the rest of the system reads from — the system object (`$ost`) and the runtime configuration object (`$cfg`) — opens a persistent session, enforces cross-site-request-forgery protection on every state-changing request, and builds the navigation model for the active realm.

This specification owns: the master bootstrap include (`main.inc.php`) and its ordered initialization steps; the client-realm gate (`client.inc.php`); the staff-realm gate (`scp/staff.inc.php`); the admin-realm gate (`scp/admin.inc.php`); the system object's responsibilities (`osTicket` class: config/session/CSRF accessors, system-state predicates, system logging, admin alerting, request/environment utilities); the runtime configuration object's read accessors and namespaced-key model (`OsticketConfig`); the navigation-model builders (`StaffNav`, `AdminNav`, `UserNav`); the landing page (`index.php`); and the offline page (`offline.php`).

It does NOT own: the actual authentication/credential mechanics (owned by FS-002 staff auth and FS-010 client portal), the cryptography/validation/formatting helpers it merely invokes (owned by FS-003), the persistent-session storage table mechanics (FS-002), the upgrade/migration machinery that the "upgrade pending" predicate consults (FS-061), per-page feature content, or the canonical configuration-key catalog and table schemas (owned by FS-091 reference data). Those are referenced as dependencies. Many configuration write-paths (`updateSystemSettings`, `updateTicketsSettings`, etc.) physically live in the config class but functionally belong to the admin settings feature (FS-032); this spec documents only the read-side accessors and the namespaced storage model, and cross-references FS-032 for the write/validation logic.

---

## Functional Requirements

### FS-001.1: Master Bootstrap Include

**Description**: The system shall route every web-facing entry point through a single master include that hardens the runtime, defines global constants, connects to persistence, and materializes the system and configuration singletons. No entry point may run feature logic before this include has completed successfully.

**Acceptance Criteria**:
- Every realm gate (`client.inc.php`, `scp/staff.inc.php`) and every standalone entry script includes the master include exactly once (via an idempotent include) before doing anything else.
- The master include refuses direct browser access: if the currently executing script's filename equals the master include's own filename, execution halts immediately with the message `kwaheri rafiki!`. (The same self-name guard appears in `client.inc.php` and `scp/staff.inc.php`, each refusing direct access with `kwaheri rafiki!` / `Access denied` respectively — see BS-001.)
- The master include performs, in this fixed order: (1) direct-access guard, (2) runtime/language hardening (BS-002), (3) timezone defaulting (BS-003), (4) directory-constant definition (FS-001.2), (5) version constant definition, (6) loading of the core system class and HTTP helper, (7) root-path resolution (FS-001.3), (8) configuration-file discovery and inclusion (FS-001.4), (9) include-path and table-constant setup (FS-001.2), (10) loading of shared helper classes, (11) database connection + singleton startup (FS-001.5), (12) session retrieval and global-default derivation, (13) request-data cleanup (BS-004).
- After the master include completes, the global system object `$ost`, the global configuration object `$cfg`, and the global session object `$session` are all available to downstream code.
- If any step in the bootstrap fails fatally, downstream feature code never executes (the request terminates with an error response — see FS-001.6).

### FS-001.2: Environment Constants & Table-Name Constants

**Description**: The system shall define a fixed set of directory-path constants and a complete set of persistence-table-name constants during bootstrap, so that all downstream code addresses files and tables through symbolic names rather than literals.

**Acceptance Criteria**:
- The bootstrap derives the absolute installation root from the real filesystem path of the master include's directory and exposes it as `ROOT_DIR` (always terminated with a forward slash, with backslashes normalized to forward slashes for cross-platform consistency).
- Derived directory constants: `INCLUDE_DIR` (= `ROOT_DIR` + `include/`), `PEAR_DIR`, `SETUP_DIR`, `UPGRADE_DIR`, `I18N_DIR`.
- A version constant `THIS_VERSION` is defined with the literal value `1.7-git` and is the value surfaced to the admin panel and returned by the system object's `getVersion()`.
- Session constants: `SESSION_SECRET` (a hash derived from `SECRET_SALT`) and `SESSION_TTL` (literal `86400` seconds = 24 hours).
- File-upload default `DEFAULT_MAX_FILE_UPLOADS` = the runtime's configured max-file-uploads if set, otherwise the literal `5`.
- Ticket-system defaults: `DEFAULT_PRIORITY_ID` = `1`; `EXT_TICKET_ID_LEN` = `6` (length of randomly generated external ticket numbers).
- `DEFAULT_PAGE_LIMIT` is derived after config load as the configured page size, falling back to the literal `25` when unset.
- Every persistence table is addressed through a `*_TABLE` constant computed as a configurable table prefix (`TABLE_PREFIX`, supplied by the config file) concatenated with the bare table name. The full set defined here includes (one-line summary; canonical schema owned by FS-091): config, syslog, session, file, file_chunk, staff, department, help_topic, groups, group_dept_access, team, team_member, page, faq (+ attachment/topic/category), canned_response (+ attachment), ticket (+ thread/attachment/priority/lock/event/email_info), email (+ template_group/template), filter (+ rule), sla, api_key, timezone, and a legacy `email_banlist` constant retained but no longer used as of v1.7.
- `PRIORITY_TABLE` is defined as an alias of `TICKET_PRIORITY_TABLE`.
- The path separator constant is selected at runtime: a semicolon on Windows, a colon otherwise; the runtime include path is then overwritten to include the current directory, the include directory, and the PEAR directory.

### FS-001.3: Root-Path (URL Base) Resolution

**Description**: The system shall determine the URL path under which the installation is served, so that generated links and redirects are correct even when the application lives below the web document root (e.g., under a user-directory mapping).

**Acceptance Criteria**:
- If `ROOT_PATH` is not already defined, the system resolves it from the request environment and the master include's directory, then defines `ROOT_PATH` normalized to a single trailing slash.
- Resolution rules: when run from the command line (no document root) or when the install directory equals the document root, the root path is `/`. Otherwise the system compares the executing script's full path against the document root to compute the URL prefix that maps to the install directory.
- If, after configuration load, `ROOT_PATH` is still undefined or empty, the request terminates with a 500 error instructing the operator to define the root path in the configuration file (FS-001.6).
- The resolved `ROOT_PATH` is used to build the assets path (client realm sets `ASSETS_PATH` = `ROOT_PATH` + `assets/default/`) and to redirect to the installer when no configuration exists.

### FS-001.4: Configuration-File Discovery & Bootstrap of Static Settings

**Description**: The system shall locate and load the operator-provided static configuration file (database credentials, table prefix, secret salt, admin email) before connecting to persistence, accommodating multiple historical file locations and gracefully handling the uninstalled state.

**Acceptance Criteria**:
- The system searches for the configuration file in this priority order, using the first found: (1) a legacy `ostconfig.php` at the install root (installs prior to v1.6 RC5), (2) a legacy `include/settings.php` (v1.6 RC5), (3) the current `include/ost-config.php` (v1.6 stable onward).
- If only the legacy `include/settings.php` is found AND the currently executing script is itself `settings.php`, the request halts with the message instructing the operator to rename `include/settings.php` to `include/ost-config.php`.
- If no configuration file is found but a `setup/` directory exists at the install root, the request is redirected to the installer (`ROOT_PATH` + `setup/`). (This is the uninstalled state — see FS-060 installer.)
- If no configuration file is found and no `setup/` directory exists, the request terminates with a 500 response reading **"Error loading settings. Contact admin."**
- After successfully including the configuration file, its absolute path is recorded in the `CONFIG_FILE` constant (used by the admin realm to check the file's permissions — FS-001.10).
- If the static configuration does not define a secret salt (`SECRET_SALT`), the bootstrap synthesizes one as a hash of the table prefix concatenated with the admin email, so that older installations still have a stable salt.

### FS-001.5: System & Configuration Singleton Startup

**Description**: The system shall, after a successful database connection, instantiate exactly one system object and one runtime configuration object per request, both globally accessible, and prime the session with default timezone settings.

**Acceptance Criteria**:
- The bootstrap connects to the database using the static configuration's host/user/password (passing SSL options when the configuration defines a database SSL certificate authority/cert/key), then selects the configured database name.
- After connection, the system object is started, which (a) opens a database-backed session with TTL = `SESSION_TTL`, (b) constructs the runtime configuration object reading from the `core` namespace of the config table, and (c) constructs the per-session CSRF token holder.
- On successful startup, the session's timezone offset and daylight-saving flag are seeded from the runtime configuration's defaults (`TZ_OFFSET`, `TZ_DST` in the session); these are later overridden per-user on staff login (FS-001.8) and per-staff in the staff gate.
- The runtime configuration object exposes a large catalog of typed read accessors over the `core` config namespace (booleans, ids, timeouts, formats, page sizes, etc. — summarized in Data Requirements; canonical key catalog owned by FS-091).
- The system object and configuration object are referenced everywhere downstream as the globals `$ost` and `$cfg`; the session as `$session`.
- **Active timezone-offset derivation** (BS-015): the configuration object computes the active `tz_offset` lazily at construction — if a default timezone id is configured and no `tz_offset` is already session-persisted, it resolves and session-persists the offset for that timezone id; if no default timezone id is configured, it session-persists `tz_offset` = `0` (compatibility for installations that stored a raw offset rather than a timezone instance). The seeded session `TZ_OFFSET` (above) reads through this derived value.
- **Pre-namespace configuration fallback** (BS-016): the configuration object first loads all rows of the `core` namespace; if that yields zero rows (an installation predating namespaced configuration), it falls back to loading the single configuration row with id `1` and treats each of its columns as a config key/value pair, so legacy single-row installs still hydrate.

### FS-001.6: Fatal-Error Handling During Bootstrap

**Description**: When bootstrap cannot establish a usable runtime, the system shall fail closed with a generic user-facing error and (where possible) alert the operator, never exposing internal failure detail to the requester.

**Acceptance Criteria**:
- A bootstrap fatal error is raised when any of the following occur: the database connection fails, the database cannot be selected, or the system/configuration singletons cannot be loaded. The internal failure reason is captured for the operator.
- On a bootstrap fatal error, the system attempts to email the operator (admin email from the static config) a notice titled **"osTicket Fatal Error"** whose body contains the failure reason and the current request URL, then terminates with a 500 response reading **"Fatal Error: Contact system administrator."** (no internal detail shown to the requester).
- An unresolved root path produces a distinct 500 response instructing the operator to define the root path in the configuration file.
- A missing/invalid configuration file produces the "Error loading settings. Contact admin." 500 response (FS-001.4).
- All 500 responses close the connection and exit; downstream feature code does not run.

### FS-001.7: System-State Predicates (Online / Offline / Upgrade-Pending)

**Description**: The system shall expose authoritative predicates describing whether the helpdesk is operational, so that realm gates can decide whether to serve, redirect to the offline page, or redirect to the upgrader.

**Acceptance Criteria**:
- "System online" is true only when ALL of: the help desk is configured online (config flag `isonline`), AND no schema upgrade is pending.
- "Upgrade pending" is true when any registered upgrade stream's current code-level hash signature differs from the schema signature recorded in the configuration (compared case-insensitively across all streams; the predicate returns true on the first mismatch). (Stream/migration detail owned by FS-061.)
- The schema-signature lookup itself is fallback-tiered: (1) the namespaced `schema_signature` config value for the requested namespace (or the default namespace), else (2) a direct read of `schema_signature` from the legacy single-row config record (id `1`), else (3) for very old (pre-1.7) installs, a hash of the legacy database-version value. This lets the upgrade predicate work across all historical schema layouts.
- "Help desk offline" is the logical inverse of the configured-online flag alone (it does NOT factor in upgrade-pending), and is exposed both on the system object's config and consulted by the staff gate to render an offline-mode notice.
- These predicates are pure reads; they have no side effects.
- The knowledge-base-enabled predicate is true only when the KB feature flag is set AND at least one published FAQ exists (consulted by the landing page and the client navigation — FS-001.13).

### FS-001.8: Client-Realm Gate

**Description**: For every public client-portal page, the system shall, after master bootstrap, enforce the offline state, resolve the optional client session, enforce CSRF on POST requests, and build the public navigation model.

**Acceptance Criteria**:
- The client gate defines client-only constants (`CLIENTINC_DIR`, the marker `OSTCLIENTINC`, `ASSETS_PATH`).
- **Offline enforcement**: unless the executing script is on a small exemption list (only `logo.php`), if the system is not online the gate includes the offline page (`offline.php`) and exits — the requested client page is never rendered.
- **Client session resolution**: if a stored client identity exists in the session (a client user id plus a session key), a client session object is constructed from it; the client is treated as logged in only if that object resolves to a real id AND validates. A logged-in client has their session activity refreshed; otherwise the client handle is cleared to "not logged in" (anonymous browsing is permitted on public pages).
- **CSRF enforcement**: on any POST request, if the CSRF token check fails, the gate redirects to the portal index and, as a fallback, halts with **"Action denied (400)!"** — the POST is never processed.
- The gate clears the standard per-page working variables (errors array, message string) and defines the client page limit (`PAGE_LIMIT` = `DEFAULT_PAGE_LIMIT`).
- The gate builds a public navigation model (`UserNav`) for the client (or anonymous visitor), defaulting to the `home` section.
- Client login is NOT account-based: the client "session" is keyed on a ticket-scoped identity (ticket id + key), not a registered user account. (Canonical client-auth model owned by FS-010.)

### FS-001.9: Staff-Realm Gate

**Description**: For every staff control-panel page, the system shall, after master bootstrap, require a valid authenticated staff session, enforce active-account/active-group and online-state rules for non-admins, enforce CSRF on POST, apply staff preferences, and build the staff navigation model.

**Acceptance Criteria**:
- The staff gate defines staff-only constants (`STAFFINC_DIR`, `SCP_DIR`, the markers `OSTSCPINC` / `OSTSTAFFINC`, and the premade-KB table constant) and refuses direct access (`Access denied`).
- A reusable login-redirect routine (`staffLoginPage(msg)`) is declared unless an AJAX layer already pre-declared it (so the AJAX interface can trap expired sessions): it stores the originally requested URL as the post-login destination, stores the message, renders the login page, and exits.
- **Authentication requirement**: a staff session is constructed from the stored staff user id. If it does not resolve to a real id OR does not validate, the gate redirects to the login page with one of these messages, selected in order: (a) a pending auth message previously stored in the session (then cleared), else (b) **"Session timed out due to inactivity"** when a staff id was present but the session no longer validates, else (c) **"Authentication Required"**.
- **Non-admin restrictions**: for a staff member who is NOT a super admin, the gate additionally requires that the account is active AND its group is active — otherwise it redirects to login with **"Access Denied. Contact Admin"**; and it requires the system to be online and no upgrade pending — otherwise it redirects to login with **"System Offline"**. (Super admins bypass both checks, so they can administer an offline or upgrade-pending system.)
- On passing auth, the staff session activity is refreshed (keep-alive).
- **CSRF enforcement**: on any POST, a failed CSRF check terminates with a 400 response **"Valid CSRF Token Required"**.
- A CSRF token is published into the page head as a meta tag (named `csrf_token`) for use by AJAX calls.
- **Staff preferences applied to session**: the staff member's timezone offset and daylight-saving observance are written to the session; the staff page limit (`PAGE_LIMIT`) is set from the staff's preference, falling back to `DEFAULT_PAGE_LIMIT`.
- The gate clears the standard staff working variables (errors, message, warning, system-notice strings, tabs, submenu arrays) and computes an exemption set of scripts (`logout.php`, `ajax.php`, `logs.php`, `upgrade.php`) that bypass the upgrade/forced-password interceptors below.
- **Upgrade interceptor**: if an upgrade is pending and the script is not exempt, the gate sets an error/notice ("System upgrade is pending — Upgrade Now") and renders the upgrader page, then exits. (See FS-061.)
- **Offline-mode notice**: otherwise, if the help desk is configured offline, a system notice is set explaining the client interface is disabled and only admins may access the control panel, with an enable link.
- The gate builds the staff navigation model (`StaffNav`) and the system notice/warning and page title (**"osTicket :: Staff Control Panel"**) are set on the system object.
- **Forced password change interceptor**: if the staff member is flagged to change their password and the script is not exempt, a "Password change required to continue" notice is set and the profile page is rendered, then the request exits. (Credential mechanics owned by FS-002.)

### FS-001.10: Admin-Realm Gate

**Description**: For admin control-panel pages, the system shall require admin privilege beyond the staff gate, emit a battery of security/maintenance warnings, and build the admin navigation model.

**Acceptance Criteria**:
- The admin gate first runs the full staff gate (via include), then requires that the resolved staff member is a super admin; if not, it redirects to (and renders, as a fallback) the staff index and exits.
- It defines admin markers (`OSTADMININC`, `ADMINPAGE` — the latter swaps the header to admin menus).
- **Upgrade-pending precedence**: if an upgrade is pending, the admin gate sets the upgrade notice and (unless the script is `upgrade.php` or `logs.php`) redirects to and renders the upgrader, then exits.
- **Security/maintenance warnings** (only evaluated when no upgrade is pending), set as a single system warning, first matching wins: (a) if the active config file is the legacy `settings.php`, advise renaming it (and hard-die if the running script is itself `settings.php`); (b) else if a `setup/` directory still exists, advise deleting the `setup/install` directory for security; (c) else if the config file exists AND is writable, the file-permission cache is cleared and the raw permission bits are re-read — only when the group-write bit (`0x0010`) or world-write bit (`0x0002`) is set is a warning surfaced advising the operator to remove write access (with an example `chmod 644`); (d) else if PHP register-globals is on, advise turning it off (message: "Please consider turning off register globals if possible"). The register-globals warning is only set when no higher-precedence warning already applies.
- The admin gate builds the admin navigation model (`AdminNav`), overwriting the staff nav set earlier, and sets the page title to **"osTicket :: Admin Control Panel"**.

### FS-001.11: Cross-Site Request Forgery Protection

**Description**: The system shall protect every state-changing (POST) request with a per-session token that the requester must echo back, accepting the token either as a form field or as an HTTP header, and logging rejected attempts.

**Acceptance Criteria**:
- A per-session CSRF token holder is created at system startup with a fixed token name (`__CSRFToken__`).
- A token is minted lazily on first read and rotated when absent or expired; the token value is derived from the session id, cryptographic random bytes, and the secret salt. Reading a still-valid token resets its activity timer.
- A CSRF check passes when the request supplies a matching token via the POST field named `__CSRFToken__` OR via the `X-CSRFToken` HTTP header; both are validated against the current (non-expired) token.
- A failed check is recorded as a warning in the system log under the title `Invalid CSRF Token <name>` (capturing the concatenation of the offending POST-field and header token values, and the request URL) and returns false; the calling gate is responsible for rejecting the request (client gate → redirect + die; staff gate → 400 response). The CSRF-failure warning is logged with admin-alert explicitly suppressed (it does NOT email the operator), to avoid alert storms from forged or stale POSTs.
- The POST-field token is checked before the header token; either a valid form field OR a valid header satisfies the check.
- The system object also exposes a separate "link token" (a hash of the CSRF token, secret salt, and session id) for protecting state-changing GET links, validated case-insensitively.
- The token-holder default timeout is disabled (0), so the CSRF token does not independently expire within a session unless explicitly configured otherwise.

### FS-001.12: System Logging & Admin Alerting

**Description**: The system shall provide a centralized logging facility that records events to a persistent system log at one of three severity levels and optionally alerts the operator by email, honoring a configurable minimum log level.

**Acceptance Criteria**:
- The facility exposes level-named entry points: debug, info, warning, error, and a database-error variant; the write side deliberately collapses finer syslog priorities into exactly three stored levels — **Error**, **Warning**, **Debug** ("Windows-style"). This requirement owns the write-side collapse/alerting/purge; the three-level enum value-list itself is owned by FS-091.8 (→ canonical: FS-091.8).
- A log entry is persisted to the system log table (with created/updated timestamps, a sanitized title, the stored level label, the sanitized message body, and the requester IP) ONLY when the configured minimum log level is at least the entry's level — unless a `force` flag overrides this (used so everything is logged during upgrades).
- When an entry is flagged to alert AND the configured log level admits the entry, the operator is alerted by email before (or in addition to) persistence.
- Admin alerting resolves the recipient as the configured admin email (falling back to the static config admin email), appends the current request URL to the message, and sends via the configured alert email account, else the default email account, or, as a last resort, the system mailer with a fixed `"osTicket Alerts"` from-name addressed from the recipient address. When admin alerting is itself asked to log its alert, it logs at critical priority WITHOUT re-alerting (to prevent alert→log→alert loops). Database-error alerts are suppressed unless the "alert on SQL error" config flag is set, to avoid alert storms.
- A log-purge routine deletes system-log rows older than a configurable grace period (in months); it is a no-op when the grace period is unset or non-numeric. (Invoked by cron — FS-043.)

### FS-001.13: Navigation Model Construction (Gate Ordering)

**Description**: On each request the active realm gate shall construct a realm-appropriate navigation model so the rendered chrome matches the realm. This requirement owns only the bootstrap/gate *ordering* — which gate builds which nav and the precedence between them; the navigation structure, links, submenus, render, and active-state model are owned by FS-090. → canonical: FS-090.1–FS-090.9/.28 (client nav FS-090.4/.28).

**Acceptance Criteria**:
- The client gate (FS-001.8) builds the public navigation model (`UserNav`), defaulting to the `home` section.
- The staff gate (FS-001.9) builds the staff navigation model (`StaffNav`).
- The admin gate (FS-001.10) builds the admin navigation model (`AdminNav`), which **overwrites** the staff nav set earlier in the same request (the admin gate runs the staff gate first, then replaces its nav).
- The structure, links, submenu composition, permission/flag conditioning, active-state model, and the silent-ignore behavior for activating a missing tab are all owned by FS-090 (see EC-015, folded into FS-090).

### FS-001.14: Landing Page

**Description**: The system shall serve a public help-desk landing page presenting the primary client actions and (conditionally) a knowledge-base pointer.

**Acceptance Criteria**:
- The landing page runs the client gate, sets the section to `home`, and renders the client header.
- If a landing page content record is configured, its body is rendered; otherwise a default heading **"Welcome to the Support Center"** is shown.
- The page presents two primary calls to action: "Open a New Ticket" (linking to `open.php`) and "Check Ticket Status" (linking to `view.php`), each with descriptive copy.
- A "Frequently Asked Questions (FAQs)" pointer to `kb/index.php` is shown only when the knowledge-base-enabled predicate is true.
- The client footer is rendered last.

### FS-001.15: Offline Page

**Description**: When the help desk is offline, the system shall serve a dedicated offline page to client-realm visitors rather than the requested page.

**Acceptance Criteria**:
- The offline page runs the client gate; if the system has since come online, it redirects to (and includes, as a fallback) the landing page and exits.
- It renders the client header (with no navigation), then a landing region containing either the configured offline-page content body or, when none is configured, the default heading **"Support Ticket System Offline"**, then the client footer.
- The offline page is the destination the client gate redirects anonymous/normal visitors to whenever the system is not online (FS-001.8); only super-admin staff bypass offline at the staff gate (FS-001.9).

### FS-001.16: System-Object Request & Environment Utilities

**Description**: The system object shall expose a battery of request/environment helper predicates and accessors used by downstream realms and features to interrogate the current request safely, independent of host quirks.

**Acceptance Criteria**:
- **HTTPS detection** (`is_https`): true when the request arrived over HTTPS — recognized either by the standard secure-connection server flag being `on`, OR by the forwarded-protocol header (`X-Forwarded-Proto`) being `https` (so installs behind an SSL-terminating proxy are correctly detected).
- **Command-line detection** (`is_cli`): true when the script is being executed via the command-line interface — recognized either by the runtime API name beginning with `cli`, OR (fallback for the CGI binary invoked from a shell) when neither a request method nor an HTTP host is present. (Used by cron/automail to bypass HTTP-only assumptions — FS-043.)
- **Path-info accessor** (`get_path_info`): returns the request path-info segment, preferring the standard `PATH_INFO` server value and falling back to `ORIG_PATH_INFO`; returns null when neither is present. (Consumed by the API/AJAX dispatcher — FS-043.)
- **Safe request-value accessor** (`get_var`): returns `vars[index]` only when `index` exists in the supplied array AND (when a type is requested) the value's type matches; otherwise returns the caller-supplied default. A companion (`get_db_input`) returns the same value pre-escaped for query embedding.
- **Allowed-file-type predicate** (`isFileTypeAllowed`): returns false when no file is supplied or no allowed-types are configured; returns true unconditionally when the configured allowed list is the wildcard `.*`; otherwise returns true only when the file's lowercased 3–4-character extension matches (with a leading dot) one of the comma-separated configured extensions. (File-type policy is consulted but owned by FS-022.)
- **Schema-signature accessor** (`getDBSignature(namespace='core')`): returns the schema signature recorded in configuration for the given namespace (delegates to the config object — used by the upgrade-pending predicate, FS-061).
- These utilities are pure reads with no persistence side effects (excepting the implicit lazy CSRF-token mint reachable through other accessors).

---

## Business Rules

### BS-001: Direct-Access Guard On Include Files

**Rule**: An include file that is meant to be required by other scripts (the master include, the client gate, the staff gate) must refuse to be requested directly by URL, halting immediately.

**Rationale**: These files assume a bootstrap context (constants, `$ost`, `$cfg`, a session). Invoked standalone they would either leak structure or error confusingly. The guard compares the currently executing script's base filename against the file's own base filename.

**Examples**:
- Requesting `main.inc.php` directly halts with `kwaheri rafiki!`.
- Requesting `scp/staff.inc.php` directly halts with `Access denied`.
- Requesting `index.php` (a real entry point) proceeds normally because its filename differs from the include's.

### BS-002: Runtime Security Hardening On Every Request

**Rule**: Before loading any configuration or feature code, the runtime must be hardened: legacy global-variable injection is neutralized, remote file inclusion/opening is disabled, session ids are kept out of URLs, response caching is disabled, and error display/reporting is set to a safe profile.

**Rationale**: osTicket must run safely across heterogeneous, often shared, hosting where insecure language defaults may be enabled. Hardening at the top of the bootstrap closes a class of injection and information-leak vulnerabilities regardless of host configuration.

**Examples**:
- If register-globals is enabled, any request variables that would shadow existing variables are unset and the directive is forced off.
- Remote URL fopen/include directives are forced off.
- Error reporting is set to all-except-notice (and except strict/deprecated where defined), so deprecation noise does not break pages.

### BS-003: Timezone Defaulting

**Rule**: If the runtime has no configured timezone, the system must establish one before any date logic runs: it lets the language layer auto-detect, and if that fails it defaults to `America/New_York` (Eastern).

**Rationale**: Date/time arithmetic (SLA, due dates, timestamps) must have a deterministic timezone. A hard fallback prevents fatal warnings on misconfigured hosts.

**Examples**:
- Host with a configured `date.timezone`: left untouched.
- Host with auto-detectable timezone: that timezone is set.
- Host where neither is available: `America/New_York` is forced.

### BS-004: Request-Data Normalization (Magic-Quotes Cleanup)

**Rule**: When the legacy magic-quotes behavior is active on the host, the bootstrap must strip the runtime-added slashes from all incoming request collections (POST, GET, REQUEST) so feature code sees clean input.

**Rationale**: Magic quotes double-escape input and corrupt data and queries. Normalizing once at the bootstrap keeps every downstream consumer free of host-dependent escaping artifacts.

**Examples**:
- On a host with magic quotes on, an apostrophe submitted in a form arrives un-backslashed to feature code.
- On a modern host (magic quotes off / removed), the cleanup is skipped.

### BS-005: Single Bootstrap, Two Singletons Per Request

**Rule**: Each web request constructs exactly one system object (`$ost`) and one runtime configuration object (`$cfg`); all realms and features read shared state through these globals rather than re-loading config or re-opening sessions.

**Rationale**: Centralizing config, session, and CSRF in one place per request guarantees a consistent view of system state and avoids redundant database work.

**Examples**:
- The staff gate, the admin gate, and every feature page all reference the same `$ost`/`$cfg` produced by the one master-include run.
- The admin gate runs the staff gate by include rather than re-bootstrapping, so the singletons are reused, not rebuilt.

### BS-006: Fail-Closed On Bootstrap Failure

**Rule**: Any failure to establish a usable runtime (no config, unresolved root path, DB unreachable, config not loadable) must terminate the request with a generic operator-facing error and must not reveal internal detail to the requester; where feasible, the operator is alerted by email.

**Rationale**: A half-initialized system is unsafe to serve. Failing closed protects data integrity and avoids leaking infrastructure detail, while the email alert gives the operator a chance to react.

**Examples**:
- DB down → fatal email titled "osTicket Fatal Error" to the admin, generic 500 to the visitor.
- Missing config file with no installer present → 500 "Error loading settings. Contact admin."
- Missing config file with installer present → redirect to setup (an actionable, non-fatal state).

### BS-007: Offline Mode Excludes Clients, Admits Only Super Admins

**Rule**: When the system is not online, public client pages are replaced by the offline page, and the staff control panel is accessible only to super-admin staff; ordinary staff are denied with "System Offline".

**Rationale**: An operator takes the system offline to perform maintenance or because the install is mid-upgrade. Clients must see a clean offline notice; only administrators may operate the panel to restore service.

**Examples**:
- A client visiting any portal page while offline is shown the offline page.
- A non-admin agent attempting to log in while offline is denied "System Offline".
- A super admin can still reach the control panel to bring the system back online.

### BS-008: Upgrade-Pending Forces The Upgrader

**Rule**: When the recorded schema signature does not match the current code's expected signature, the system is "upgrade pending": it is treated as not-online, and staff/admin requests (except a small exemption set) are intercepted and routed to the upgrader.

**Rationale**: Running application code against a stale schema risks data corruption. Forcing the upgrader first guarantees the schema is migrated before normal operation resumes. (Stream/signature mechanics owned by FS-061.)

**Examples**:
- After deploying new code, the first staff request is redirected to `upgrade.php`.
- `logout.php`, `ajax.php`, `logs.php`, `upgrade.php` are exempt so the operator can still log out, view logs, and run the upgrader itself.
- The client portal reports offline while an upgrade is pending.

### BS-009: CSRF Required On All POST Requests

**Rule**: Every POST request in both realms must carry a valid CSRF token (form field `__CSRFToken__` or `X-CSRFToken` header); a request that fails the check is rejected before any state change, and the failure is logged.

**Rationale**: Without per-request token validation, an attacker could trigger state changes via forged cross-site form submissions. Accepting the token via header additionally protects AJAX calls.

**Examples**:
- A client POST without a valid token is redirected to the portal index and denied "Action denied (400)!".
- A staff POST without a valid token returns 400 "Valid CSRF Token Required".
- An AJAX POST supplies the token via the `X-CSRFToken` header (published as a `csrf_token` meta tag by the staff gate).

### BS-010: Staff Session Validity Drives The Auth Gate

**Rule**: A staff request is authenticated only when a staff session resolves to a real staff id AND validates (active, non-expired); otherwise the request is bounced to login with a message that distinguishes "timed out" from "authentication required", preserving the originally requested URL for post-login return.

**Rationale**: Session validity (not mere presence of an id) is the authority on whether a staff member is logged in; distinguishing timeout from never-authenticated improves the operator experience, and preserving the destination supports deep-link login.

**Examples**:
- Expired session with a remembered staff id → "Session timed out due to inactivity".
- No staff id at all → "Authentication Required".
- A previously stored auth message (e.g., set by an AJAX trap) takes precedence and is then cleared.

### BS-011: Admin Privilege Is A Strict Super-Set Of Staff

**Rule**: The admin realm requires everything the staff realm requires, plus the super-admin flag; the admin gate runs the staff gate first and only then enforces the admin requirement.

**Rationale**: Administration is the most privileged surface. Layering it on top of the staff gate ensures admins are also subject to session, CSRF, and active-account rules, while the extra flag restricts the surface to administrators.

**Examples**:
- A logged-in non-admin agent who navigates to an admin page is redirected to the staff index.
- A super admin passes the staff gate and then the admin requirement, reaching the admin panel.

### BS-012: Three-Level Logging With Configurable Threshold

**Rule**: System logging collapses all severities into exactly three stored levels (Error, Warning, Debug) and persists an entry only when the configured minimum log level admits it (unless explicitly forced).

**Rationale**: A small, fixed level set keeps logs operator-readable; the configurable threshold lets operators control log volume, while the force flag guarantees full logging during sensitive operations like upgrades.

**Examples**:
- With log level "Error", a debug entry is not persisted.
- During an upgrade, forced logging records everything regardless of threshold.
- A database-error log does not email the admin unless the SQL-error-alert flag is enabled.

### BS-013: Namespaced, Lazily-Defaulted Configuration

**Rule**: Runtime configuration is stored as key/value rows under a namespace (`core` for system settings); reads resolve in precedence order session-override → database value → code default → caller default, and developer-declared defaults fill in for keys absent from the database.

**Rationale**: Namespacing lets multiple subsystems (and future plugins) keep separate settings; the precedence chain supports per-session overrides and lets new settings ship with sensible defaults before an operator ever saves them.

**Examples**:
- `allow_pw_reset` defaults to true and `pw_reset_window` to 30 even on installations that predate those keys.
- A session-level config override (`persist`) shadows the database value for the current request only.
- An accessor returns the caller-supplied default when the key exists in neither session, database, nor code defaults.

### BS-014: Page Size Falls Back To A Fixed Default

**Rule**: Pagination size is taken from configuration (and, for staff, from the staff member's own preference); when unset it falls back to the fixed default of 25 rows.

**Rationale**: Every list view needs a deterministic page size; a fixed fallback guarantees lists paginate sanely even before any size is configured.

**Examples**:
- Unconfigured install → 25 rows per page.
- A staff member who set a personal page limit sees their value on staff pages.
- A client page always uses the system default page limit.

### BS-015: Active Timezone Offset Is Lazily Derived And Session-Persisted

**Rule**: The active timezone offset (`tz_offset`) is computed once at configuration construction from the configured default-timezone id and cached in the session for the request; when no default timezone is configured it is fixed at `0`.

**Rationale**: Earlier osTicket versions stored a raw offset rather than a timezone-table reference; deriving and caching the offset at startup gives every downstream date computation a single, consistent value while remaining compatible with both storage layouts.

**Examples**:
- Install with a default timezone id and no cached offset → the offset for that zone is resolved and session-persisted.
- Legacy install with no default timezone id → `tz_offset` is persisted as `0`.
- Subsequent reads in the same request return the cached value rather than re-resolving.

### BS-016: Configuration Hydration Falls Back To The Legacy Single-Row Layout

**Rule**: When the namespaced configuration query returns no rows, the configuration object hydrates instead from the single legacy configuration record (id `1`), treating each column as a key/value pair.

**Rationale**: Installations predating namespaced configuration store all settings as columns of one row. The fallback lets the bootstrap (and the upgrader) read those settings before migration converts them to the key/value model.

**Examples**:
- Modern install → settings load from the `core`-namespace rows.
- Pre-namespace install → settings load from the columns of config row id `1`.
- The schema-signature read mirrors this fallback (namespaced value → id-1 column → hashed legacy db-version).

---

## Data Requirements

The bootstrap layer is primarily a coordinator; its durable data lives in the configuration table, the session table, and the system-log table (canonical schemas owned by FS-091). This section summarizes the entities the bootstrap reads or writes.

### Runtime Configuration (`config` table, `core` namespace)

A key/value store namespaced by a `namespace` column (default `core`), each row carrying id, key, value, and an updated timestamp. The bootstrap reads a broad catalog of settings via typed accessors. Representative groups (non-exhaustive; canonical key list in FS-091):

| Group | Representative keys / accessors |
|-------|--------------------------------|
| System state | `isonline`, `schema_signature` (→ online/offline/upgrade predicates) |
| Identity & URL | `helpdesk_title`, `helpdesk_url` (base URL), `admin_email` |
| Date/time | `default_timezone_id`, `tz_offset`, `enable_daylight_saving`, `time_format`, `date_format`, `datetime_format`, `daydatetime_format`, `db_tz_offset` |
| Defaults | `default_dept_id`, `default_sla_id`, `default_priority_id` (code default 1), `default_email_id`, `alert_email_id`, `default_smtp_id`, `default_template_id` |
| Pagination | `max_page_size` (→ `DEFAULT_PAGE_LIMIT`, fallback 25) |
| Sessions/login | `staff_session_timeout`, `staff_login_timeout`, `staff_max_logins`, `staff_ip_binding`, `client_session_timeout`, `client_login_timeout`, `client_max_logins` |
| Password reset | `allow_pw_reset` (default true), `pw_reset_window` (default 30 minutes; converted minutes→seconds on read), and a separate legacy `passwd_reset_period` value (distinct accessor, not minute-converted) |
| Logging | `log_level`, `log_graceperiod`, `send_sql_errors`, `send_login_errors`, `send_mailparse_errors` |
| Features | `enable_kb`, `enable_captcha`, `enable_auto_cron`, `enable_mail_polling`, `random_ticket_ids`, `clickable_urls` |
| Pages | `landing_page_id`, `offline_page_id`, `thank-you_page_id`, `client_logo_id` |

> Code-level defaults declared in the configuration object: `allow_pw_reset` = true, `pw_reset_window` = 30 (minutes; converted to seconds on read). Timeout accessors multiply stored minutes by 60.
>
> The configuration object also hosts numerous write-paths (`updateSystemSettings`, `updateTicketsSettings`, `updateEmailsSettings`, `updatePagesSettings`, `updateAutoresponderSettings`, `updateKBSettings`, `updateAlertsSettings`). These are the admin-settings save handlers and are owned by **FS-032** (admin settings); this spec covers only the read accessors and the namespaced storage model.

### Bootstrap Constants

| Constant | Value / derivation |
|----------|--------------------|
| `THIS_VERSION` | `1.7-git` |
| `ROOT_DIR`, `INCLUDE_DIR`, `PEAR_DIR`, `SETUP_DIR`, `UPGRADE_DIR`, `I18N_DIR` | Derived from install root |
| `ROOT_PATH` | Resolved URL base (FS-001.3) |
| `CONFIG_FILE` | Absolute path of the loaded config file |
| `SECRET_SALT` | From config, or synthesized hash of table-prefix + admin email |
| `SESSION_SECRET` | Hash of secret salt |
| `SESSION_TTL` | `86400` (24h) |
| `DEFAULT_PAGE_LIMIT` | Configured page size or `25` |
| `DEFAULT_MAX_FILE_UPLOADS` | Runtime max or `5` |
| `DEFAULT_PRIORITY_ID` | `1` |
| `EXT_TICKET_ID_LEN` | `6` |
| `THISPAGE` | Current request URL |
| `PATH_SEPARATOR` | `;` (Windows) or `:` (other) |
| `*_TABLE` (≈35) | `TABLE_PREFIX` + bare table name (FS-091) |

### Session Slots Touched By Bootstrap/Gates

| Slot | Meaning |
|------|---------|
| `_client` (userID, key) | Stored client identity for the portal session (FS-010) |
| `_staff` (userID, auth.dest, auth.msg) | Stored staff identity, post-login destination, pending message (FS-002) |
| `TZ_OFFSET`, `TZ_DST` | Active timezone offset and DST observance (config default, then per-user) |
| `csrf` (token, time) | Per-session CSRF token and activity timestamp |
| `cfg:<namespace>` | Per-session configuration overrides per namespace |

### System Log (`syslog` table)

Each entry: created/updated timestamps, sanitized title, level label (one of **Error / Warning / Debug**), sanitized message body, requester IP address. (Schema owned by FS-091; admin log views owned by FS-033.)

### Navigation Model (in-memory, per request)

Built fresh each request by the active realm gate; not persisted. Structure and field detail (`UserNav` / `StaffNav` / `AdminNav` shape, tabs/submenu maps, active-state) owned by FS-090. → canonical: FS-090.1–FS-090.9/.28.

---

## User Flows / Interactions

### Flow 1: Anonymous Client Visits The Portal (System Online)

1. The visitor requests the landing page.
2. The client gate runs master bootstrap, confirms the system is online, finds no stored client identity (anonymous), and skips CSRF (not a POST).
3. The public navigation is built defaulting to `home`; the landing page renders its configured (or default) body plus the two CTAs and, if KB is enabled and populated, the FAQ pointer.

### Flow 2: Client Visits While System Offline

1. The visitor requests any portal page.
2. The client gate runs master bootstrap and finds the system not online.
3. Unless the script is the exempt `logo.php`, the gate includes the offline page and exits; the offline page renders the configured offline body (or default heading) inside the client header/footer. The requested page never runs.

### Flow 3: Staff Member Loads A Control-Panel Page (Authenticated)

1. The agent requests a staff page; the staff gate runs master bootstrap and resolves the staff session.
2. The session validates → activity is refreshed; for a non-admin, active-account/active-group and online/not-upgrading checks pass.
3. CSRF meta is published; the agent's timezone and page limit are applied to the session; the staff navigation and page title are set.
4. With no pending upgrade or forced password change, the requested page renders.

### Flow 4: Staff Session Has Expired

1. The agent (whose session has timed out) requests a staff page.
2. The staff session fails to validate; the gate stores the requested URL as the post-login destination and renders the login page with "Session timed out due to inactivity", then exits.
3. After re-authenticating (FS-002), the agent is returned to the originally requested page.

### Flow 5: Non-Admin Reaches An Admin Page

1. A logged-in non-admin agent requests an admin page.
2. The admin gate runs the staff gate (passes), then finds the agent is not a super admin.
3. The agent is redirected to (and the staff index is rendered as a fallback), and the request exits — the admin page never renders.

### Flow 6: First Request After A Code Upgrade (Schema Stale)

1. An admin requests any control-panel page; bootstrap computes "upgrade pending" because the schema signature is stale.
2. The staff/admin gate sets the "System upgrade is pending" notice and, since the script is not exempt, redirects to and renders the upgrader, then exits.
3. The admin completes the upgrade (FS-061); subsequent requests proceed normally.

### Flow 7: Database Unreachable

1. Any request reaches the master bootstrap.
2. The database connection fails; the bootstrap captures the reason, emails the operator an "osTicket Fatal Error" notice with the reason and request URL, and returns a generic 500 "Fatal Error: Contact system administrator." to the requester.

---

## Edge Cases & Error Scenarios

### EC-001: Direct Request To An Include File
**Scenario**: A user requests `main.inc.php`, `client.inc.php`, or `scp/staff.inc.php` directly by URL.
**Expected**: The self-name guard halts immediately (`kwaheri rafiki!` / `Access denied`) before any bootstrap or feature logic runs (BS-001).

### EC-002: Configuration File Missing, Installer Present
**Scenario**: A fresh checkout with no `ost-config.php` but a `setup/` directory.
**Expected**: Bootstrap redirects the request to the installer rather than erroring (FS-001.4) — this is the normal "not yet installed" path (FS-060).

### EC-003: Configuration File Missing, No Installer
**Scenario**: No config file and the `setup/` directory has been removed (or never existed).
**Expected**: A 500 "Error loading settings. Contact admin." is returned (BS-006); the request cannot proceed.

### EC-004: Legacy `settings.php` Config In Use
**Scenario**: An old install still has its configuration in `include/settings.php`.
**Expected**: The system loads it but, in the admin realm, surfaces a persistent warning to rename it to `ost-config.php`; if the requested script is itself `settings.php`, the request hard-dies with the rename instruction (FS-001.4, FS-001.10).

### EC-005: Unresolved Root Path
**Scenario**: The URL base cannot be derived and the config file does not define `ROOT_PATH`.
**Expected**: A 500 instructs the operator to define the root path in `ost-config.php`; the request stops (FS-001.3, FS-001.6).

### EC-006: Missing Secret Salt On An Old Install
**Scenario**: An upgraded install whose config predates `SECRET_SALT`.
**Expected**: Bootstrap synthesizes a deterministic salt from the table prefix and admin email so CSRF, session, and link-token derivations remain stable (FS-001.4).

### EC-007: POST Without A Valid CSRF Token
**Scenario**: A forged or stale POST arrives in either realm.
**Expected**: Client realm redirects to the portal index and dies "Action denied (400)!"; staff realm returns 400 "Valid CSRF Token Required"; the attempt is logged as a warning. No state change occurs (BS-009, FS-001.11).

### EC-008: Disabled Staff Account Or Group
**Scenario**: A non-admin agent whose account or whose group has been deactivated tries to use the panel.
**Expected**: The staff gate denies with "Access Denied. Contact Admin" and routes to login (FS-001.9). Super admins are exempt from this check.

### EC-009: Forced Password Change Pending
**Scenario**: A staff member flagged to change their password requests a non-exempt page.
**Expected**: The gate intercepts and renders the profile page with "Password change required to continue", and the originally requested page does not render (FS-001.9). Exempt scripts (logout/ajax/logs/upgrade) bypass the interceptor.

### EC-010: Config-File World/Group Writable
**Scenario**: The configuration file's filesystem permissions allow group or world write.
**Expected**: The admin realm surfaces a warning advising a permissions tightening (e.g., `chmod 644`), unless a higher-precedence warning (legacy filename, setup dir present) already applies (FS-001.10).

### EC-011: Setup Directory Left In Place After Install
**Scenario**: A live install still has the `setup/` directory.
**Expected**: The admin realm warns the operator to delete the `setup/install` directory for security (FS-001.10).

### EC-012: AJAX Layer Pre-Declares The Login Redirect
**Scenario**: An AJAX request hits the staff gate after the session expired.
**Expected**: Because the AJAX layer may pre-declare the login-redirect routine, expired sessions are trapped per AJAX conventions rather than rendering a full login page (FS-001.9; AJAX details owned by FS-043).

### EC-013: System Comes Online While On The Offline Page
**Scenario**: An operator brings the system online while a visitor sits on the offline page and reloads.
**Expected**: The offline page detects the system is now online and redirects to (and includes) the landing page (FS-001.15).

### EC-014: Knowledge Base Enabled But No Published FAQs
**Scenario**: The KB feature flag is on but no FAQ is published.
**Expected**: The KB-enabled predicate returns false, so the KB navigation link and the landing-page FAQ pointer are hidden (FS-001.7, FS-001.13).

### EC-015: Setting Active Nav Tab That Does Not Exist
**Scenario**: A page asks the navigation model to activate a tab key that is not in the tab set.
**Expected**: Owned by FS-090 — the navigation model silently ignores a missing target tab. → canonical: FS-090.1–FS-090.9/.28.

### EC-016: Request Behind A Proxy (Forwarded-For)
**Scenario**: A request arrives with an `X-Forwarded-For` header.
**Expected**: Bootstrap overrides the remote address with the left-most forwarded address so logging and IP-bound features see the original client (FS-001.2 / logging). (Note the trust assumption in KL-005.)

### EC-017: Pre-Namespace Configuration Install
**Scenario**: An installation predating namespaced configuration is booted; the `core`-namespace query returns no rows.
**Expected**: The configuration object falls back to the single legacy config row (id `1`), hydrating every column as a key/value pair; the schema-signature read uses the same tiered fallback (BS-016, FS-001.5, FS-001.7).

### EC-018: Request Arrives Over HTTPS Via An SSL-Terminating Proxy
**Scenario**: A proxy terminates TLS and forwards the request to the application over plain HTTP, setting a forwarded-protocol header of `https`.
**Expected**: The HTTPS-detection utility reports the request as secure based on the forwarded-protocol header even though the direct connection flag is absent (FS-001.16). (Same unconditional-trust caveat as KL-005 applies to the forwarded header.)

### EC-019: Page Requested From The Command Line
**Scenario**: A script (e.g., automated mail fetch or cron) is invoked from the shell rather than a web server, so no request method/HTTP host is present.
**Expected**: The command-line-detection utility reports CLI execution (by runtime API name or the no-method/no-host fallback), and the root-path resolver returns `/` because no document root is set (FS-001.16, FS-001.3).

### EC-021: Connection Forces Empty `sql_mode` (Strict-Mode Suppression)
**Scenario**: The bootstrap establishes the database connection on a server whose default `sql_mode` is strict/traditional.
**Expected**: The connect routine, immediately after setting the UTF-8 character set/collation, issues `SET SESSION sql_mode=''` (via `db_set_variable`), clearing all SQL modes for the application's session so osTicket's lax write patterns (zero dates, implicit truncation, missing column defaults) do not raise errors. This is a per-session side-effect of every connect, not a server-global change. (Helper contract and consequences owned by FS-003.29 / BS-028 / EC-023 / KL-015.)

### EC-020: System Error Set But Never Retrievable
**Scenario**: Code sets a single system error on the system object and later attempts to read it back through the matching getter.
**Expected**: The getter returns empty due to the internal key-name mismatch; callers must not depend on this round-trip (KL-009).

---

## Dependencies

| Dependency | Specification | Relationship |
|------------|---------------|--------------|
| Persistence helpers (`db_connect`, `db_query`, `db_input`, …) | FS-003 / FS-091 | Bootstrap connects and all config/log/session reads go through these |
| Cryptography (`Crypto::random`), validation, formatting/sanitizing | FS-003 | CSRF token entropy; log sanitization; request normalization |
| HTTP helper (`Http::response`, `Http::redirect`) | FS-003 | Fatal responses, installer redirect, gate redirects |
| Mailer (`Mailer::sendmail`, alert email accounts) | FS-040 | Fatal-error alert and admin alerting from the logger |
| Persistent session store (`osTicketSession`) | FS-002 | System object opens a DB-backed session; gates refresh activity |
| Staff session & permissions (`StaffSession`, `isValid`, `isAdmin`, page limit, forced-password flag) | FS-002 | The staff/admin gate authenticates and applies preferences |
| Client session (`ClientSession`, `isValid`, ticket-scoped identity) | FS-010 | The client gate resolves the optional logged-in client |
| Upgrade streams & schema signatures (`DatabaseMigrater`, upgrader page) | FS-061 | "Upgrade pending" predicate and the upgrade interceptor |
| FAQ count for KB-enabled predicate | FS-050 | KB navigation/landing pointer gating |
| Page content records (landing/offline/thank-you, logos) | FS-033 | Landing and offline page bodies |
| Template variable replacement (`VariableReplacer`) | FS-040 | `replaceTemplateVariables` on the system object |
| Admin settings save handlers (`update*Settings`) | FS-032 | Physically in the config class; functionally the admin settings feature |
| Reference data: config-key catalog, table schemas, enum sets | FS-091 | Canonical definitions of everything bootstrap reads |
| Cron log purge invocation | FS-043 | Cron calls the logger's `purgeLogs` |

---

## Known Limitations

### KL-001: Bootstrap Forces Error Display On
**Limitation**: The bootstrap forces `display_errors` and `display_startup_errors` on, contradicting its own guidance comment that errors should be logged to a file with display off.
**Why It Exists**: Convenience default for development-era code.
**Impact**: On a misconfigured production host, runtime warnings/errors may be rendered into responses, potentially leaking paths or internals. Operators should disable error display in production.

### KL-002: CSRF Token Does Not Independently Expire By Default
**Limitation**: The CSRF token holder is created with a timeout of 0, so the token never expires independently of the session; "isExpired" is effectively always false in normal operation.
**Why It Exists**: The timeout feature is implemented but not enabled by the system object.
**Impact**: CSRF token lifetime is bounded only by the session, not by an independent inactivity window — a weaker posture than the class is capable of.

### KL-003: Self-Synthesized Secret Salt Is Predictable
**Limitation**: When the config lacks a secret salt, the synthesized salt is a hash of the (often default) table prefix plus the admin email — both low-entropy, sometimes guessable values.
**Why It Exists**: Backward compatibility for installs that predate the salt setting.
**Impact**: On such installs, CSRF and link tokens derive from a weakly-seeded salt; operators should set an explicit high-entropy `SECRET_SALT`.

### KL-004: Admin-Link Gate Vs. Direct Page Access
**Limitation**: The admin gate enforces the super-admin requirement on each admin page include, but several maintenance warnings (writable config, setup dir, register-globals) are advisory-only — they warn rather than block.
**Why It Exists**: They surface hardening opportunities without locking the operator out of administration.
**Impact**: A misconfigured-but-functional install keeps running with a persistent banner; nothing forces remediation.

### KL-005: X-Forwarded-For Is Trusted Unconditionally
**Limitation**: Bootstrap overwrites the remote address with the left-most `X-Forwarded-For` value whenever the header is present, without verifying the request actually came through a trusted proxy.
**Why It Exists**: To recover the real client IP behind reverse proxies/load balancers.
**Impact**: A client can spoof its logged/IP-bound address by sending a forged header when no proxy strips it; IP-bound features (e.g., staff IP binding, API key IP restriction) can be misled.

### KL-006: Legacy `email_banlist` Table Constant Retained But Unused
**Limitation**: A `BANLIST_TABLE` (`email_banlist`) constant is still defined though the code comments note it is no longer used as of v1.7 (banning moved into the filter pipeline).
**Why It Exists**: Backward compatibility / incomplete cleanup.
**Impact**: Dead constant; no runtime effect (FS-042 owns the current ban mechanism).

### KL-007: Three-Level Log Collapse Loses Granularity
**Limitation**: All syslog priorities are mapped onto just three stored levels (Error/Warning/Debug), discarding finer distinctions (info, notice, critical, emergency).
**Why It Exists**: Deliberate "Windows-style" simplification for operator readability.
**Impact**: Operators cannot filter by the original fine-grained severity in stored logs.

### KL-008: Config Write Handlers Live In The Bootstrap Config Class
**Limitation**: The configuration class mixes pure read accessors (bootstrap concern) with the admin settings write/validation handlers (an admin feature concern).
**Why It Exists**: Single-class config model.
**Impact**: Cross-cutting ownership; this spec scopes only the reads and defers writes to FS-032, which is a coupling worth noting for future refactoring.

---

### KL-009: System-Error Getter/Setter Key Mismatch
**Limitation**: The system object's single-error setter writes the value under one internal key (`error`) while the corresponding getter reads from a different key (`err`). As a result, a system error set via the setter can never be retrieved through the getter — the getter always returns empty.
**Why It Exists**: A typo in one of the two internal key names, never reconciled.
**Impact**: Any code path relying on round-tripping a single system error through this getter/setter pair is silently broken; in practice page-level errors are carried in a separate errors array (and the warning/notice slots are consistent), so the broken accessor is largely unused but remains a latent bug.

### KL-010: Config Permission Warning Skipped When File Reports Not-Writable
**Limitation**: The admin permission warning for a group/world-writable config file is only evaluated when the runtime first reports the file as writable by the running process; on hosts where the writability probe is unreliable (e.g., certain ownership/ACL combinations) the subsequent precise permission-bit check is never reached, so a genuinely loose-permission file may not be flagged.
**Why It Exists**: The writability probe is used as a cheap pre-gate before the more precise bit inspection.
**Impact**: The advisory may be suppressed on edge-case hosting setups; operators should not rely solely on the banner to detect insecure config-file permissions.

## Future Considerations

- Disable error display by default and route errors to a log file in production builds (KL-001).
- Enable an independent CSRF token inactivity timeout (the mechanism already exists — KL-002).
- Require an explicit high-entropy secret salt at install and deprecate the synthesized fallback (KL-003).
- Gate `X-Forwarded-For` trust behind a configurable trusted-proxy list (KL-005).
- Separate the configuration read model from the admin settings write handlers (KL-008).
- Preserve finer log severities or add a structured severity field alongside the three-level label (KL-007).
- Remove the dead `email_banlist` constant (KL-006).
