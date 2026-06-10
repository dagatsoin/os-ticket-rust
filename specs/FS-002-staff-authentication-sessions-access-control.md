# FS-002: Staff Authentication, Sessions & Access Control

## Overview

This specification documents how members of staff authenticate into the **Staff Control Panel (SCP)**, how their authenticated sessions are established, kept alive, validated and destroyed, and how the system decides what each staff member is permitted to do once inside. It covers:

- The staff **login** flow (credential check, brute-force "strike" lockout, CSRF rotation, redirect to the originally requested page).
- The staff **logout** flow (link-token confirmation, session destruction).
- The staff **password-reset** flow (request a reset email, follow a tokenised link, log in, change password).
- **Password storage and verification** behavior (portable bcrypt/phpass hashing with legacy MD5 fallback and silent rehash).
- **Session** establishment, the per-session validation token, idle timeout, optional IP binding, session id regeneration, and the database-backed session store.
- **CSRF** protection: a per-session rotating token enforced on every state-changing request, plus a derived single-use "link token" used to confirm link clicks (e.g. logout).
- The **authorization model**: the admin-vs-staff distinction (`isadmin`), per-staff active/group-active/vacation availability, the **Group** permission flags, and **Department** access (primary department + managed departments + group→department access rows), including the "access-limited" (assigned-only) restriction.

This spec describes WHAT happens and WHAT rules govern it. It is technology-agnostic; concrete literals, field names, table names, enum values and configuration keys are extracted because they are behavioral facts. Where the canonical definition of a stored field, enum set, table schema or configuration default belongs to the reference-data spec, this document references **FS-091** rather than restating it.

> Scope boundary: **Client** (ticket-requester) sessions are NOT account-based and are documented in FS-010. This spec addresses the staff realm only, except where the shared `UserSession` token mechanism is common to both (documented here as shared infrastructure because it physically lives in the staff session classes).

---

## Functional Requirements

### FS-002.1: Staff login

**Description**: A staff member authenticates by submitting a username (or email address) and password to the login page. On success they are redirected into the control panel; on failure they receive a generic error and the failure is counted toward a lockout threshold.

**Acceptance Criteria**:
- The login page accepts a `userid` field (interpreted as either a username or an email address — see FS-002.9) and a `passwd` field.
- The submission MUST carry a valid CSRF token (see FS-002.7); a missing/invalid token rejects the request with response code **400** and the message "Valid CSRF Token Required", and the credentials are NOT evaluated.
- On a successful CSRF check, the CSRF token is **rotated** before credentials are evaluated, so that each login attempt requires a freshly fetched token (defends against parallel and serial brute-force attempts).
- Credentials are valid only when a staff account exists for the supplied identifier AND the password verifies against the stored hash (FS-002.5).
- On success the user is redirected to the **destination** captured when the gate first challenged them (FS-002.8), defaulting to the control-panel home page when no safe destination exists.
- On failure the page is re-displayed with the message "Invalid login" (or a more specific message such as the lockout message), and the strike counter is incremented (FS-002.2).
- Both username and password are required; a request with an empty username, empty password, OR a purely numeric username is rejected with "Username and password required" before any lookup.
- A successful login emits an `auth.login.succeeded` signal; a failed login emits an `auth.login.failed` signal carrying the attempted username and password.

### FS-002.2: Brute-force strike lockout

**Description**: Repeated failed staff logins are counted; exceeding a configured threshold locks further attempts for a configured timeout window.

**Acceptance Criteria**:
- A per-session **strike counter** is incremented on every failed login attempt.
- When the strike counter exceeds the configured maximum (`staff_max_logins`, default **4**), the system records a "last strike" timestamp and rejects further attempts with "Forgot your login info? Contact Admin."
- While locked, every attempt during the timeout window (`staff_login_timeout`, default **2** minutes) is rejected with "Max. failed login attempts reached", and the lockout timer is **reset to now** on each attempt (so persistent hammering continually re-arms the window).
- Once the timeout window has elapsed since the last strike, the lockout clears: the last-strike timestamp and the strike counter are reset, and normal attempts resume.
- Crossing the threshold logs a **warning** ("Excessive login attempts (<username>)") containing the username, requester IP, timestamp, attempt number and timeout duration; this alert is emailed to the configured alert address only when login-error alerting is enabled (`alert_email` / `send_login_attempt_alert`).
- Every **other** failed attempt (even attempt count) logs a lower-severity warning "Failed staff login attempt (<username>)" with username, IP, time and attempt number (not emailed).
- The "Forgot my password" link appears on the login form only after **more than one** strike AND only when password reset is enabled (FS-002.3).

### FS-002.3: Staff password reset

**Description**: A staff member who cannot log in may request a password-reset email, follow the tokenised link, re-identify themselves, be logged in under a forced-change condition, and set a new password.

**Acceptance Criteria**:
- The reset feature is available only when password reset is enabled (`allow_pw_reset`, default **enabled**). When disabled, navigating to the reset page sets the auth message "Password resets are disabled" and redirects to the control-panel home.
- **Step — request (sendmail)**: the user submits an identifier (`userid`); if it resolves to a staff account, a reset email is sent (FS-002.4). When the identifier cannot be verified, the page shows "Unable to verify username <value>". (A request that resolves to a real account does not reveal whether the email send succeeded beyond switching to a "sent" confirmation template — see FS-002.18 for the actual, inverted, trigger condition.)
- **Step — follow link**: visiting the reset page with a `token` query parameter that maps to a valid staff account presents the re-identification form with the prompt "Re-enter your username or email"; an unknown/expired token redirects to the control-panel home.
- **Step — re-identify & log in (newpasswd)**: the user re-submits their identifier together with the token. The system requires that (a) the identifier resolves to a real account ("Invalid user-id given" otherwise), and (b) the token maps to that exact account id ("Invalid reset token" otherwise). On success the account is flagged **force-password-change**; if that update fails the message is "Unable to reset password"; otherwise an `auth.pwreset.login` signal is emitted (carrying a mutable `page` defaulting to the control-panel home), the user is logged in (FS-002.6), the token is stored on the session as `reset-token`, and the user is redirected to the (possibly signal-modified) page.
- **Validity-window enforcement is effectively absent at this step**: the intended `pw_reset_window` (default **30** minutes) check is present in code but its boolean condition is defective and never rejects a token that genuinely exists in the store (see EC-002-13 / KL-002-09). A token remains usable until it is explicitly cancelled (password change or a fresh login), not until the window elapses.
- **Step — change password**: because the account is now force-change, the staff gate (FS-002.10) routes the user to the profile page to set a new password. Submitting a new password with a valid in-session `reset-token` bypasses the normal "current password required" check (the reset token substitutes for the current password).
- Every reset POST MUST carry a valid CSRF token; otherwise the request is rejected with response **400** "Valid CSRF Token Required".
- A successful password change cancels (deletes) all outstanding reset tokens for that account and clears the in-session `reset-token` (FS-002.4 token store).

### FS-002.4: Reset-email token issuance and storage

**Description**: When a reset email is issued, a high-entropy token is generated, persisted server-side keyed to the requesting account, and embedded in a link inside the templated email.

**Acceptance Criteria**:
- The reset token is a randomly generated code of **48** characters (~290 bits of entropy).
- The token is stored in the configuration store under namespace **`pwreset`**, with the token string as the key and the staff account id as the value; its stored last-modified timestamp is used to enforce the validity window.
- The email is composed from the `staff.pwreset` message template (resolved from the staff member's department template, falling back to the system default template); if the template is missing, an error is returned instead of an email.
- The link embedded in the email points at the staff password-reset page with the token as a query parameter, using the configured base URL.
- The sending address is the configured alert email, falling back to the configured default email.
- An `auth.pwreset.email` signal is emitted, allowing the sending email and template variables to be observed/modified before send.
- Cancelling reset tokens for an account deletes every `pwreset`-namespace entry whose value equals that account id.

### FS-002.5: Password verification with legacy fallback and silent rehash

**Description**: A submitted password is checked first against a portable bcrypt/phpass hash; legacy MD5-hashed passwords are still accepted but transparently upgraded.

**Acceptance Criteria**:
- The submitted password is first compared against the stored hash using the portable hashing scheme (bcrypt via the system crypt where available, otherwise a portable phpass `$P$`/`$H$` MD5-based hash; see FS-002.13). A match authenticates the user.
- If the portable comparison fails, the stored hash is compared against a raw (unsalted) MD5 of the submitted password; a match authenticates the user (legacy fallback). The legacy comparison is short-circuited to a failure when the submitted password is empty.
- On a successful **legacy MD5** match, the system attempts to silently re-hash the password with the portable scheme and persist it (a single `UPDATE` of the stored hash). If auto-update is disabled (`$autoupdate=false`) OR the persistence query fails, the account is instead flagged **force-password-change** — and the user is still authenticated this time regardless. Note the rehash is attempted with no error feedback to the caller.
- Verification used purely for comparison (e.g. confirming the current password before a profile password change) does NOT perform the silent rehash side-effect.
- An empty submitted password never authenticates.

### FS-002.6: Session establishment on login

**Description**: A successful credential check (or successful reset login) establishes an authenticated session and records last-login metadata.

**Acceptance Criteria**:
- On login the account's `lastlogin` timestamp is set to the current time.
- If a password reset is **due** by policy (FS-002.12) and the account is NOT an administrator, the account is flagged force-password-change as part of login.
- The prior `_staff` session payload is cleared and replaced with the authenticated payload: the staff account id is stored as `userID`, and the per-session validation **token** is set (FS-002.11).
- The session records the staff member's timezone offset and daylight-saving preference for use across the panel.
- The PHP session id is **regenerated** on login, and the previous session id's server-side record is destroyed (session fixation defence).
- A debug log entry "Staff login" is recorded with the username and requester IP.

### FS-002.7: CSRF protection on state-changing requests

**Description**: Every form POST and AJAX state-change carries a per-session CSRF token that the server validates; the token can rotate and can expire on inactivity.

**Acceptance Criteria**:
- A CSRF token is held in the session and exposed to forms as a hidden input and to AJAX via a page meta tag named `csrf_token`.
- The token **mechanism** — derivation (session id + 16 random bytes + secret salt, hashed), rotation, timestamp reset, and the POST-body (`__CSRFToken__`) vs `X-CSRFToken`-header presentation/ordering — is owned by **FS-001.11**; this spec consumes it.
- A request satisfies the CSRF check when the token is presented per the FS-001.11 mechanism and the presented value matches the current token and the token is not expired.
- On every staff page, any POST that fails the CSRF check is rejected with response **400** "Valid CSRF Token Required" and the failure is logged as a warning.
- The login page additionally **rotates** the CSRF token after a successful check so that the consumed token cannot be reused (anti-brute-force).
- The token may carry an optional inactivity timeout; when a non-zero timeout is configured, a token older than the timeout is treated as expired (forcing rotation) regardless of session lifetime.

### FS-002.8: Post-login redirect to original destination

**Description**: When an unauthenticated request is intercepted, the originally requested URL is captured so the user can be returned there after logging in.

**Acceptance Criteria**:
- When the staff gate challenges an unauthenticated request, it stores the original request URI as the auth **destination** and an explanatory message.
- After a successful login the user is redirected to that stored destination.
- The stored destination is honoured only when it does not reference the login page or an AJAX endpoint; otherwise the user is sent to the control-panel home page (prevents redirect loops and meaningless landings).

### FS-002.9: Identifier resolution (username or email)

**Description**: A staff identifier supplied at login or reset is resolved to an account by interpreting it as a numeric id, an email address, or a username.

**Acceptance Criteria**:
- A purely numeric identifier is looked up by staff id.
- An identifier that is a syntactically valid email address (FS-091) is looked up by email.
- Any other identifier is looked up by username.
- At login, a purely numeric `userid` is explicitly rejected ("Username and password required") rather than treated as an id.

### FS-002.10: Logout

**Description**: A staff member logs out via a link guarded by a single-use link token; the session is destroyed and the user is returned to the login page.

**Acceptance Criteria**:
- The logout action is intended to require an `auth` link-token query parameter that validates against the session-derived link token (FS-002.7 derived token); the link-token comparison is case-insensitive.
- **Known defect**: on a missing/invalid link token the handler only emits a redirect header toward the control-panel home but does NOT halt execution. Because the script falls through, it proceeds to destroy the session regardless. The link-token "protection" therefore does NOT actually prevent forced logout — any reachable logout URL (even without a valid `auth` token) terminates the session. See EC-002-14 / KL-002-10.
- On reaching the logout body, a debug log entry "Staff logout" is recorded with the username and requester IP.
- The `_staff` session payload is cleared, all session variables are unset, and the session is destroyed.
- The user is redirected to the login page (the login page is then re-included so the login form renders).

### FS-002.11: Per-request session validation (the authenticated gate)

**Description**: Every staff page re-validates the session before serving content, enforcing identity, idle timeout, optional IP binding, system availability and account status.

**Acceptance Criteria**:
- On every staff page the gate reconstructs the staff session from the stored `userID` and validates it.
- A session is valid only when: (a) the account id resolves, (b) the current PHP session id matches the session id bound into the staff session object, and (c) the stored session token validates (FS-002.11 token rules) within the configured idle timeout and, when enabled, with a matching IP binding.
- An invalid or absent session re-challenges the user with the login page. The message is the stored auth message if present, otherwise "Session timed out due to inactivity" when a `userID` existed but failed validation, otherwise "Authentication Required".
- The session validation token is `"<hash>:<time>:<md5(ip)>"`, where `<hash>` is the MD5 of `<time> + session secret + account id`. Validation recomputes the hash from the embedded time and rejects on mismatch, rejects when the elapsed time exceeds the configured idle timeout, and (when IP binding is enabled) rejects when the embedded `md5(ip)` does not match the current requester IP.
- Each successful page load **refreshes** the token (re-issuing it with the current time), so activity keeps the session alive up to the idle timeout (`staff_session_timeout`, default **30** minutes).
- The idle timeout is independent of the absolute session record lifetime (the DB session row carries its own expiry — see FS-002.14).

### FS-002.12: Periodic forced password change (password aging)

**Description**: Administrators may configure a maximum password age; non-admin staff whose password exceeds that age are forced to change it at next login.

**Acceptance Criteria**:
- The password-age policy is `passwd_reset_period`, expressed in **months** (default **0** = disabled).
- A reset is "due" when a non-zero period is configured AND the time since the password was last set exceeds the period. A "month" is treated as exactly **30 days**: the threshold in seconds is `passwd_reset_period × 30 × 24 × 60 × 60` (i.e. 2,592,000 s per configured month), regardless of actual calendar-month lengths.
- The age basis (`passwd_change`) is computed once at account-load time as `now − age-anchor`, where the anchor is the account's `passwdreset` timestamp, falling back to the account's creation timestamp when no reset has occurred. (A code comment flags a potential timezone caveat in this difference.)
- When a reset is due, login flags non-administrator accounts as force-password-change (administrators are exempt from this automatic forcing at login — FS-002.6).

### FS-002.13: Password hashing characteristics

**Description**: Passwords are stored using a portable hashing framework that prefers bcrypt and falls back to a portable MD5-based scheme, parameterised by a work factor.

**Acceptance Criteria**:
- New password hashes are produced with a configurable work factor; the default work factor is **8** and the accepted range is **4–31** (values outside the range are coerced to the default).
- Hashing tries three backends in order and returns the first whose output is the expected length: (1) native bcrypt → a **60**-character `$2a$` hash; (2) extended-DES (`crypt` with a `_`-prefixed salt) → a **20**-character hash; (3) the portable `$P$` phpass scheme → a **34**-character hash. The first two are skipped when the platform lacks the corresponding `crypt` capability. If none yields the expected length, hashing returns a literal `'*'` (treated as a null/failed hash by the caller).
- The iteration-count / work-factor is encoded into the salt: the portable scheme records `work_factor + 5` (clamped to 30); bcrypt records the two-digit work factor directly.
- Verification recognises both portable `$P$`/`$H$` hashes and native crypt hashes (bcrypt or extended-DES) so that existing stored hashes of any of these forms validate; the portable check is attempted first and a leading `*` result falls through to a native `crypt` comparison.
- Hashing/verification never logs or persists the plaintext password.

### FS-002.14: Database-backed session store

**Description**: Sessions are persisted in a server-side store rather than relying solely on PHP's default file handler, with per-row expiry, owner and request metadata.

**Acceptance Criteria**:
- The effective TTL is resolved by taking the first non-zero of: the constructor-supplied TTL, the runtime `session.gc_maxlifetime` setting, then the `SESSION_TTL` constant (**86400 seconds / 24 hours**). The bootstrap starts the session with `SESSION_TTL` as the constructor TTL, so the effective TTL is normally 86400 s.
- The session cookie is named **`OSTSESSID`**; the cookie path is the application root path; the cookie domain is derived from the request host (omitted when the host has no dot or is a bare IP); the cookie is marked secure when the request is HTTPS.
- **Install-state branch**: when no database schema version is recorded (fresh/uninstalled system, `OsticketConfig::getDBVersion()` falsy), the handler installs the custom cookie parameters and the database save-handler set as described here. **When a schema version IS recorded (a normal installed system), the constructor instead returns a plain default `session_start()` immediately — the custom cookie-parameter call and the database save-handler registration are NOT applied on that path.** (A `DISABLE_SESSION` define short-circuits to no session at all.) The behavioral consequences below therefore describe the handler set as written; whether it is engaged depends on this branch.
- The cookie lifetime and the `session.gc_maxlifetime` override are set from the **raw constructor TTL argument**, not the resolved effective TTL — so when the constructor receives 0 (or `gc_maxlifetime` would otherwise apply) the cookie can be a session cookie / GC lifetime can be 0 even though row-expiry uses the resolved TTL.
- Each session row stores: session id, session data (stored as raw bytes), an expiry timestamp, an updated timestamp, the owning staff id (the current `$thisstaff` id, or 0 for anonymous), the requester IP, and the user agent. Canonical schema is in FS-091 (`session` table). The write uses a REPLACE (upsert keyed on session id).
- On write, the expiry is set to now plus the resolved TTL; a write is reported successful only when the upsert affects a row.
- Reads return a session row only when it exists AND its expiry is still in the future; the read result is cached per session id within the handler instance.
- Garbage collection deletes all session rows whose expiry is in the past.
- Regenerating the session id (via the handler's `regenerate_id`, and via login) destroys the prior session row.
- The store can enumerate currently-online users (staff ids `> 0` with unexpired sessions), optionally restricted to those whose row was updated within the last N seconds — used for "who is online" displays.
- A shutdown function (`session_write_close`) is registered to force session persistence at request end.

### FS-002.15: Group permission flags (capability authorization)

**Description**: Each staff member belongs to exactly one **Group**, whose boolean permission flags grant ticket-handling and content-management capabilities.

**Acceptance Criteria**:
- A staff member's group conveys these capability flags (each a boolean): create tickets, edit tickets, delete tickets, close tickets, post a ticket reply, assign tickets, transfer tickets, ban emails, view staff stats, manage "premade" content (canned responses), manage FAQ. Canonical field names/enum are in FS-091 (`groups` table).
- A group is **active** when its enabled flag is set; staff in a disabled group are denied access at the gate (FS-002.16) unless they are administrators.
- "Can manage tickets" is a derived capability: true when the staff member is an administrator OR may delete tickets OR may close tickets.
- The flag "show assigned tickets" is honoured only for administrators or department managers (otherwise treated as not applicable).
- A group cannot be deleted while it still has members; group→department access rows are removed when the group is deleted.

### FS-002.16: Account availability and the admin-vs-staff distinction

**Description**: Beyond group capabilities, each account carries status flags that govern whether it may sign in at all and whether it is "available", plus an administrator flag that unlocks the admin control panel.

**Acceptance Criteria**:
- An account is **active** (`isactive`), **visible** (`isvisible`), and may be **on vacation** (`onvacation`); it is **available** only when active AND in an active group AND not on vacation.
- The **administrator** flag (`isadmin`) marks an account as a super-admin. Administrators bypass the offline/upgrade-pending and disabled-account/disabled-group access blocks at the gate, and are exempt from automatic forced password change at login.
- A non-administrator is denied access ("Access Denied. Contact Admin") at the gate when their account is inactive OR their group is disabled.
- A non-administrator is denied access ("System Offline") at the gate when the system is offline or an upgrade is pending; only administrators may use the panel in those states.
- **Upgrade-pending diversion applies to everyone, admins included.** Independently of the non-admin "System Offline" block, when an upgrade is pending the gate diverts ANY non-exempt staff page to the upgrade screen (showing "System upgrade is pending"). Exempt pages are logout, AJAX, logs and upgrade. So an administrator is not blocked by "System Offline" but IS routed into the upgrade flow until the upgrade completes (offline-mode alone, without a pending upgrade, only shows an informational notice to admins and does not divert them).
- The system refuses to save a staff edit that would leave **zero active administrators** (the last row with `isadmin=1 AND isactive=1` cannot have its admin flag or active flag removed): rejected with "Cowardly refusing to remove or lock out the only active administrator". The check fires when the submitted `isadmin` or `isactive` value is not exactly the string `'1'`, and only blocks when the existing single active admin is the very account being edited.
- A staff member cannot delete their own account; deleting another account unassigns their open tickets (sets assignee to none) and removes their team memberships.

### FS-002.17: Department access model

**Description**: Visibility/access to departments derives from the staff member's primary department, the departments they manage, and the departments their group is granted, modulated by an "access-limited" restriction.

**Acceptance Criteria**:
- A staff member's accessible departments are the union of: their primary department, any department they manage (department manager), and the departments granted to their group via group→department access rows.
- A staff member **can access** a department only when that department is in their accessible set AND they are NOT access-limited.
- **Access-limited** (a.k.a. "assigned-only", `assigned_only`) restricts the staff member to tickets assigned to them rather than all tickets in their accessible departments; for such a staff member department-level access checks return false.
- A staff member **is a manager** of a department when that department's manager id equals their account id.
- Group→department access rows are managed per group (add/remove); the canonical join is in FS-091 (`group_dept_access`).
- **Team** membership is a separate cross-department grouping: a staff member may belong to teams, used for assignment; canonical schema in FS-091 (`team`, `team_member`).

### FS-002.18: Reset-request confirmation template selection (inverted trigger)

**Description**: After a reset request resolves to a real account, the page decides whether to show the "reset sent" confirmation template; the actual decision is driven by the truthiness of the send routine's return value, which is inverted relative to intent.

**Acceptance Criteria**:
- When the submitted identifier resolves to a staff account, the request handler invokes the account's reset-email routine and switches to the `pwreset.sent.php` confirmation template **only when that routine returns a falsy value**.
- The reset-email routine returns: an error object (truthy) when the `staff.pwreset` template cannot be retrieved (so the confirmation template is NOT shown), and otherwise the result of the underlying mail send (commonly an empty/falsy return), which DOES switch to the confirmation template.
- Consequently the "reset sent" confirmation is effectively shown on the normal (template-found) path and suppressed when the template is missing — the success/confirmation messaging is keyed off a falsy/void return rather than an explicit success flag. The token is persisted to the `pwreset` store as part of this routine before the mail is dispatched.
- When the identifier does not resolve, neither the routine nor the confirmation template runs, and the page shows "Unable to verify username <value>".

### FS-002.19: Default panel preferences applied at the gate

**Description**: On every validated staff page the gate seeds shared per-request preference values from the staff account.

**Acceptance Criteria**:
- The session timezone offset and daylight-saving preference are (re)written from the staff account on every validated page load (in addition to being set at login).
- A page-size limit constant is defined from the staff member's configured page size, falling back to the system default page size (**25** when unconfigured) when the staff value is empty.
- The CSRF meta header is added to the page output for AJAX consumption (FS-002.7); per-page working variables (errors, messages, tabs, submenus) are initialised empty.

---

## Business Rules

### BS-002-01: CSRF required on all staff POSTs
**Rule**: Every state-changing staff request (login, password reset, and every authenticated POST) MUST present a valid CSRF token; otherwise it is rejected with response **400** "Valid CSRF Token Required" and the credentials/action are not processed. The token derivation, rotation, and body-field/header presentation are owned by **FS-001.11**; this rule states only the staff-realm enforcement consequence.
**Rationale**: Prevents cross-site request forgery against privileged staff actions.

### BS-002-02: Login rotates the CSRF token
**Rule**: A successful CSRF check on the login page rotates the token (rotation mechanism owned by **FS-001.11**) to a new value before credentials are evaluated; the consumed token cannot be reused for a subsequent attempt. This login-page rotation is auth-specific behavior owned here.
**Rationale**: Forces each brute-force attempt to fetch a fresh token, frustrating parallel/serial automated guessing.

### BS-002-03: Strike threshold and lockout window
**Rule**: After more than `staff_max_logins` (default 4) failed attempts in a session, further attempts are locked for `staff_login_timeout` (default 2) minutes; each attempt during the lockout resets the lockout timer; after the window elapses the counter and timer reset.
**Rationale**: Throttles credential-guessing while self-healing after the cooldown.

### BS-002-04: Generic failure messaging
**Rule**: Ordinary credential failures return "Invalid login"; lockout returns "Max. failed login attempts reached" or "Forgot your login info? Contact Admin."; missing fields return "Username and password required". The system does not disclose whether the username exists.
**Rationale**: Avoids username enumeration and information leakage.

### BS-002-05: Password verification order and silent upgrade
**Rule**: Verify portable hash first; on failure fall back to raw MD5; a successful MD5 match triggers a silent re-hash to the portable scheme, or forces a password change if the re-hash cannot be persisted.
**Rationale**: Migrates legacy credentials to stronger hashing without forcing immediate user friction.

### BS-002-06: Default work factor and range
**Rule**: Password hashes use work factor 8 by default; any requested work factor outside 4–31 is coerced to 8.
**Rationale**: Ensures a sane, consistent computational cost for hashing.

### BS-002-07: Reset token entropy, storage and lifetime
**Rule**: A reset token is a 48-character random alphanumeric code stored under config namespace `pwreset` (token string as the key, staff id as the value); the store's per-row `updated` timestamp is *intended* to bound validity to `pw_reset_window` (default 30) minutes from issuance. The token is single-purpose by cancellation (removed on password change and on a successful normal login), NOT by reliable time-expiry: the window check at both the reset-login step and the profile-change step is defective and does not actually reject genuinely-stored tokens by age (see KL-002-09). A token therefore persists until explicitly cancelled.
**Rationale**: High-entropy, account-bound reset credential; the time-bound is documented as intended-but-non-functional so the security posture is not overstated.

### BS-002-08: Reset disabled gate
**Rule**: When `allow_pw_reset` is disabled, the reset page is unavailable ("Password resets are disabled") and the "Forgot my password" link is suppressed.
**Rationale**: Lets administrators turn off self-service reset entirely.

### BS-002-09: Forgot-password link visibility
**Rule**: The "Forgot my password" link is shown on the login form only after more than one failed strike AND only when password reset is enabled.
**Rationale**: Surfaces recovery only to users who appear to be struggling, when recovery is permitted.

### BS-002-10: Reset-token substitutes for current password
**Rule**: During a password change performed under an active in-session `reset-token` that maps to the current account and is within its window, the "current password required" check is bypassed; the new password must still be at least 6 characters and the two entries must match.
**Rationale**: Users who reset cannot supply their (forgotten) current password.

### BS-002-11: Minimum password length and difference
**Rule**: A new password must be at least **6** characters; the two confirmation entries must match; on profile self-change the new password MUST differ from the current password.
**Rationale**: Baseline password quality and prevents no-op changes.

### BS-002-12: Session token composition and idle timeout
**Rule**: The session validation token binds the staff id, an issue time, and the requester IP hash; the session is valid only while (now − issue time) ≤ `staff_session_timeout` (default 30) minutes; each authenticated page refreshes the token. When `staff_ip_binding` is enabled (default disabled) the requester IP hash must also match.
**Rationale**: Sliding idle expiry with optional IP pinning.

### BS-002-13: Session id regeneration on privilege change
**Rule**: The PHP session id is regenerated and the prior server-side session row destroyed upon successful login (and upon explicit regeneration).
**Rationale**: Prevents session fixation.

### BS-002-14: Logout link-token confirmation (intended, but not enforced)
**Rule**: Logout is *intended* to proceed only when the `auth` query parameter matches the session-derived link token (MD5 of the CSRF token + secret salt + session id, compared case-insensitively); on mismatch a redirect to the control-panel home is *intended*. In the shipped code the mismatch branch emits the redirect header but does not stop execution, so the session is destroyed regardless of the token. The link-token check is therefore non-blocking and does not actually prevent forced logout.
**Rationale**: Documents the intended forced-logout-CSRF defence and its non-functional implementation so downstream consumers do not rely on it.

### BS-002-15: Database session expiry and TTL
**Rule**: Session rows persist in the `session` store with an expiry of now + TTL (default 86400 s); reads ignore expired rows; GC purges expired rows. The cookie is named `OSTSESSID`, scoped to the root path, secure under HTTPS.
**Rationale**: Server-authoritative session lifetime independent of client cookie behavior.

### BS-002-16: One active administrator must always remain
**Rule**: A staff save that would remove or deactivate the last remaining active administrator (`isadmin=1 AND isactive=1`) is rejected.
**Rationale**: Prevents permanent lockout of the admin panel.

### BS-002-17: Self-deletion forbidden; cleanup on delete
**Rule**: A staff member cannot delete their own account; deleting another account reassigns their **open** tickets to "unassigned" and removes their team memberships.
**Rationale**: Avoids orphaned self-deletion and dangling assignments/memberships.

### BS-002-18: Group capability gating
**Rule**: A staff member may perform a ticket/content operation only when their group's corresponding capability flag is set (or, for delete/close-derived "manage", when they are an administrator). Administrators bypass capability flags implicitly via the admin distinction.
**Rationale**: Role-based authorization driven by group flags.

### BS-002-19: Availability composition
**Rule**: A staff member is "available" (eligible for assignment/listing as available) only when active AND in an enabled group AND not on vacation.
**Rationale**: Single definition of staff availability used across assignment and stats.

### BS-002-20: Department access requires membership and non-restriction
**Rule**: A staff member can access a department only when it is in their accessible set (primary dept ∪ managed depts ∪ group-granted depts) AND they are not access-limited (assigned-only).
**Rationale**: Department visibility is the union of three sources, suppressed for assigned-only staff.

### BS-002-21: Admin bypass of system-state and account-state blocks
**Rule**: Administrators are not blocked by the "System Offline" gate (offline mode or pending upgrade) and are not blocked by their own group-disabled/inactive checks; non-admins are blocked in all of those conditions. However, a *pending upgrade* still diverts everyone (admins included) to the upgrade screen via the separate non-exempt diversion (BS-002-25) — admin bypass covers the offline gate, not the upgrade flow itself. Plain offline mode (no pending upgrade) shows admins an informational notice without diversion.
**Rationale**: Administrators must retain access to recover/maintain the system, yet must complete a pending upgrade before resuming normal work.

### BS-002-25: Pending-upgrade diversion overrides normal pages
**Rule**: While an upgrade is pending, every non-exempt staff page is replaced by the upgrade screen with the notice "System upgrade is pending"; exempt pages are logout, AJAX, logs and upgrade. This applies to all staff regardless of admin status.
**Rationale**: Forces the schema to be brought current before the panel is used.

### BS-002-26: Session-store engagement depends on install state
**Rule**: The database-backed session save-handler and custom cookie parameters are installed only when no schema version is recorded (uninstalled state); on an installed system the bootstrap uses a plain default `session_start()` and the custom handler/cookie configuration is not applied on that path. The cookie lifetime and GC max-lifetime are set from the raw constructor TTL argument rather than the resolved effective TTL.
**Rationale**: Captures the actual conditions under which DB-backed sessions and custom cookie scoping take effect.

### BS-002-27: Reset-email confirmation keyed off a falsy return
**Rule**: The "reset sent" confirmation template is shown when the reset-email routine returns a falsy value; a missing `staff.pwreset` template makes the routine return a truthy error and suppresses the confirmation. Success messaging is thus inverted relative to an explicit success flag.
**Rationale**: Documents the observed (counter-intuitive) confirmation trigger.

### BS-002-22: Username format
**Rule**: The username character-class/length pattern and its error strings ("At least two (2) characters" / "Username contains invalid characters") are owned by **FS-003.9 (BS-013)**. Auth-domain rule retained here: usernames and emails must be unique across staff, and an email may not collide with a configured system email address.
**Rationale**: Consistent, unambiguous, collision-free identifiers.

### BS-002-23: Login redirect safety
**Rule**: The post-login destination is honoured only when it is non-empty and references neither the login page nor an AJAX endpoint; otherwise the control-panel home is used.
**Rationale**: Prevents redirect loops and landing on non-page endpoints.

### BS-002-24: Force-change interrupts the session
**Rule**: While an account is flagged force-password-change, the staff gate diverts the user to the profile page to set a new password on every non-exempt page until the change is made; exempt endpoints are logout, AJAX, logs and upgrade.
**Rationale**: Guarantees the password is changed before normal work resumes.

---

## Data Requirements

> Canonical table/column definitions and enum sets live in **FS-091**. This section lists only the behaviorally significant data this domain reads/writes.

- **Staff account** (`staff`): identity (`staff_id`, `username`, `email`, `firstname`, `lastname`); credential (`passwd` hash, `passwdreset` timestamp, `change_passwd` force-change flag); status (`isadmin`, `isactive`, `isvisible`, `onvacation`, `assigned_only`); membership (`group_id`, `dept_id`); preferences (`timezone_id`, `timezone_offset`, `daylight_saving`, `max_page_size`, `auto_refresh_rate`, `signature`, `default_signature_type`, `default_paper_size`, `show_assigned_tickets`); audit (`created`, `updated`, `lastlogin`).
- **Group** (`groups`): `group_id`, `group_name`, `group_enabled`, and the capability flags `can_create_tickets`, `can_edit_tickets`, `can_delete_tickets`, `can_close_tickets`, `can_post_ticket_reply`, `can_assign_tickets`, `can_transfer_tickets`, `can_ban_emails`, `can_view_staff_stats`, `can_manage_premade`, `can_manage_faq`.
- **Group → department access** (`group_dept_access`): rows linking `group_id` ↔ `dept_id`.
- **Team membership** (`team_member`): rows linking `staff_id` ↔ `team_id`.
- **Session** (`session`): `session_id`, `session_data`, `session_expire`, `session_updated`, `user_id` (owning staff id or 0), `user_ip`, `user_agent`.
- **Reset-token store** (config namespace `pwreset`): key = token string, value = staff id, with a last-modified timestamp used for window enforcement.
- **In-session state** (`_staff` session payload): `userID`, validation `token`, `strikes`, `laststrike`, `reset-token`, and a transient `auth` block (`dest`, `msg`); plus `csrf` (token + time), `TZ_OFFSET`, `TZ_DST`.
- **Configuration keys** (defaults in FS-091): `staff_max_logins` (4), `staff_login_timeout` (2 min), `staff_session_timeout` (30 min), `staff_ip_binding` (off), `passwd_reset_period` (0 months), `allow_pw_reset` (on), `pw_reset_window` (30 min), plus derived constants `SESSION_TTL` (86400 s), session cookie name `OSTSESSID`, default hashing work factor (8).

---

## User Flows / Interactions

### Flow A — Standard login
1. Unauthenticated staff requests any control-panel page → gate captures destination + message, shows login form (with fresh CSRF token).
2. Staff enters username/email + password and submits.
3. Server validates CSRF, rotates token, resolves identifier, verifies password.
4. On success → last-login recorded, session established, session id regenerated, redirect to captured destination (or home).
5. On failure → strike incremented, "Invalid login" shown; "Forgot my password" link appears after the second strike (if reset enabled).

### Flow B — Lockout and cooldown
1. Repeated failures increment strikes; warnings logged on alternate failures.
2. Crossing `staff_max_logins` records a last-strike time, logs/optionally emails an excessive-attempts alert, and shows "Forgot your login info? Contact Admin."
3. Subsequent attempts within `staff_login_timeout` are rejected ("Max. failed login attempts reached") and re-arm the timer.
4. After the window elapses, the counter/timer reset and login is retryable.

### Flow C — Password reset (full path)
1. Staff clicks "Forgot my password" (visible after 2nd strike, reset enabled).
2. Staff enters identifier → reset email with a 48-char token link is sent (or "Unable to verify username").
3. Staff follows the emailed link → re-identification form ("Re-enter your username or email").
4. Staff re-enters identifier + token → token validated against account and window → account flagged force-change → logged in → redirected to home with `reset-token` on session.
5. Gate diverts to profile (force-change) → staff sets a new password (≥6 chars, matching, reset-token substitutes for current password) → tokens cancelled → normal session resumes.

### Flow D — Authenticated page request
1. Gate rebuilds the staff session from `userID` and validates token (hash, idle timeout, optional IP binding, session-id match).
2. Invalid → re-challenge with appropriate message ("Session timed out due to inactivity" / "Authentication Required").
3. Valid → non-admins checked for active/group-active and system-online/upgrade state; admins bypass.
4. Token refreshed (sliding expiry); CSRF meta added; force-change diversion if flagged; page served.

### Flow E — Logout
1. Staff hits the logout endpoint (normally via a link carrying the `auth` link token).
2. Gate checks the link token; on mismatch it emits a home redirect header but does NOT stop (EC-002-14) — execution continues regardless of the token.
3. Either way → log "Staff logout" → clear `_staff`, unset all session vars, destroy session → redirect to login (and re-include the login page).

---

## Edge Cases

### EC-002-01: Numeric username at login
A purely numeric `userid` is rejected with "Username and password required" rather than being treated as a staff id, even though numeric identifiers are valid for internal lookups.

### EC-002-02: Lockout timer perpetually re-armed
While locked out, every attempt resets the last-strike timestamp to now, so an attacker that keeps trying never lets the cooldown elapse — but also never gains access; a legitimate user must simply stop for the full window.

### EC-002-03: Disabling login-strike protection
The login script contains a commented line that, if enabled, clears the `_staff` session each request and thereby disables strike counting; in shipped behavior strikes ARE enforced.

### EC-002-04: Reset email send-failure ambiguity
When a reset request resolves to a real account but the email fails to send, the flow falls back to the "reset sent" confirmation template; the user is not clearly told the send failed.

### EC-002-05: Forced change on legacy hash that cannot be upgraded
If a legacy MD5 password matches but the silent re-hash cannot be persisted, the account is flagged force-password-change, so the user authenticates but is immediately required to set a new password.

### EC-002-06: Admin password aging exemption
Even when `passwd_reset_period` is configured, administrators are NOT auto-forced to change their password at login (the force-change is applied only to non-admins), so an admin's password can age past the period without an automatic block.

### EC-002-07: Last-admin protection
Attempting to remove the admin flag or deactivate the only remaining active administrator is rejected; the operation cannot proceed until another active administrator exists.

### EC-002-08: Self-delete no-op
A staff member's attempt to delete their own account returns without effect (returns 0 deleted), regardless of permissions.

### EC-002-09: Cookie domain edge cases
The session cookie domain is omitted when the request host contains no dot or is a bare IP address (avoids setting an invalid cookie domain), which affects whether the cookie is shared across subdomains.

### EC-002-10: IP binding and proxies
When IP binding is enabled, a requester whose source IP changes mid-session (e.g. roaming/proxy/NAT changes) fails session validation and is logged out, even within the idle window.

### EC-002-11: Department-access fallback query
When the primary accessible-departments query yields no rows, the system falls back to the union of the group's granted departments and the staff member's primary department id (defensive fallback).

### EC-002-12: Access-limited staff and department checks
For an access-limited (assigned-only) staff member, department-level access checks always return false even for their own department, because visibility is intended to be assignment-scoped rather than department-scoped.

### EC-002-13: Reset-token window check is a no-op
At the reset-login step (and the profile password-change step) the validity-window condition is structured as "token has NO stored timestamp AND the window has elapsed". Since a real stored token always has a timestamp, the first clause is false and the whole condition never triggers — so a stored reset token is never rejected for age. The token effectively never expires by time; it is only invalidated by explicit cancellation.

### EC-002-14: Logout fires without a valid link token
Because the invalid-link-token branch of logout redirects but does not exit, requesting the logout URL with a missing or wrong `auth` token still destroys the session. A staff member can be forcibly logged out by any link/redirect that hits the logout endpoint, defeating the intended link-token guard.

### EC-002-15: Hashing returns the literal "*" on total failure
If none of the three hashing backends produces an expected-length result, the hasher returns the literal string `'*'`; the password layer treats this as a null hash. A stored `'*'` (or `'*0'`/`'*1'`) can never be matched by the portable check, so an account whose hash was stored as `'*'` cannot authenticate via the portable path.

### EC-002-16: Force-change diversion does not special-case AJAX/API
The forced-password-change diversion routes to the profile page for every non-exempt page; a source-code note observes that AJAX and API requests are not separately handled here, so a force-change condition can surface in unexpected ways for non-browser requests (AJAX is exempt only when the running script's basename is literally `ajax.php`).

### EC-002-17: Online-users list returns staff ids only
The "who is online" enumeration filters `user_id > 0`, so anonymous/client sessions (stored with `user_id = 0`) are excluded; only authenticated staff appear. A staff member with multiple concurrent unexpired session rows can appear more than once (the id repeats per row).

### EC-002-18: Installed-system sessions are not DB-backed via this handler
On a normally-installed system the session bootstrap returns a plain `session_start()` before the custom DB save-handler and cookie parameters are registered (FS-002.14 install-state branch), so the database-backed `session` table behaviors documented here are engaged primarily in the uninstalled/setup state; an installed deployment falls back to PHP's configured default session handling unless re-wired elsewhere.

---

## Dependencies

- **FS-001 (App bootstrap & shared request lifecycle)** — instantiates the global system/config singletons, starts the session, mounts the CSRF token, and includes the staff gate (`staff.inc.php`) that this spec's per-request validation runs inside.
- **FS-003 (Cryptography, validation & formatting infrastructure)** — random-byte generation (CSRF salt source), the email/username/IP/phone validators used for identifier resolution and profile validation, the signal pub/sub bus (`auth.*`, `model.*` signals), HTTP response helper (the 400 CSRF responses), and logging.
- **FS-031 (Admin — staff, groups & directory)** — the admin-side create/edit/delete UI for staff accounts and groups; this spec documents the underlying account/group/permission model those screens manipulate.
- **FS-040 (Email accounts, templates & outbound mail)** — the `staff.pwreset` message template, template-variable substitution, and the outbound mailer used to deliver reset emails.
- **FS-091 (Reference data, enums & data model)** — canonical schemas for `staff`, `groups`, `group_dept_access`, `team`/`team_member`, `session`, and the canonical configuration-key defaults referenced throughout.
- **FS-010 (Public client portal)** — shares the `UserSession` token mechanism (client sessions use the same `sessionToken`/`isvalidSession` primitives) but with separate session keys, timeouts and binding policy; documented there.

---

## Known Limitations

### KL-002-01: Strike state is session-bound, not account-bound
Brute-force strikes and lockout are tracked in the attacker's PHP session, not against the targeted account or source IP. An attacker who discards cookies (new session per attempt) is not throttled by the per-session counter; only the periodic warning/alert logging would surface the activity.

### KL-002-02: Portable-hash fallback uses MD5 internally
On platforms without native bcrypt, the portable fallback hash (`$P$`) is an iterated-MD5 construction; it is far stronger than a single MD5 but weaker than bcrypt. Hashing strength therefore depends on the host platform's crypt support.

### KL-002-03: Legacy MD5 passwords accepted indefinitely
Pre-existing unsalted MD5 password hashes remain valid until the user next logs in (which triggers silent upgrade) or is force-changed; until then those credentials are stored as plain MD5.

### KL-002-04: Session secret derived from configuration salt
The session-token secret is derived deterministically from the secret salt (itself derived from the table prefix and admin email in absence of an explicit configured salt); a predictable or leaked salt weakens the session-token and link-token integrity guarantees.

### KL-002-05: IP binding is opt-in and off by default
Session IP binding (`staff_ip_binding`) defaults to disabled, so by default a stolen session token (within the idle window) is not pinned to the original IP.

### KL-002-06: Idle timeout uses client-supplied token time
The idle-timeout check trusts the timestamp embedded in the validation token; the token's integrity (and thus the timeout's trustworthiness) rests entirely on the secret-salted hash, not on a server-stored issue time independent of the token.

### KL-002-07: Reset flow re-asks for identifier
The reset-login step requires the user to re-enter their username/email together with the token, even though the token already maps to a specific account; this is a usability quirk and a mild additional knowledge factor rather than a security control.

### KL-002-08: Password reset email does not confirm the account silently
The "unable to verify username" message on the request step does disclose whether an identifier maps to a staff account, partially undermining the otherwise enumeration-resistant login messaging.

### KL-002-09: Reset-token time window is not enforced
The `pw_reset_window` validity check at both the reset-login and profile-change steps is structurally defective (EC-002-13) and never rejects a genuinely-stored token by age. A reset token is invalidated only by explicit cancellation (a completed password change, or any subsequent successful login), so a leaked but un-cancelled reset link remains usable long past the configured window.

### KL-002-10: Logout link-token guard is non-blocking
The link-token confirmation on logout (EC-002-14) does not halt execution on mismatch, so the forced-logout-CSRF protection it is meant to provide is absent — the session is destroyed even when the `auth` token is missing or wrong.

### KL-002-11: DB-backed session handler bypassed on installed systems
On an installed system the session bootstrap returns a default `session_start()` before the custom DB save-handler and cookie scoping are applied (EC-002-18), so the documented database-session features (per-row expiry/owner/IP/agent, online-users enumeration, custom cookie domain/secure flags) are not necessarily in force at runtime on a normal deployment.

### KL-002-12: Months approximated as 30 days for password aging
The password-aging threshold treats every configured month as exactly 30 days (FS-002.12), so the effective forced-change interval drifts from calendar months as the configured period grows.

## Future Considerations

- Track brute-force strikes against the account and/or source IP (persisted) rather than the per-session counter, to resist cookie-discarding attackers.
- Make session IP binding and a minimum password policy (length/complexity/history) configurable with stronger defaults.
- Apply password-aging forced change to administrators as well, with a break-glass exception, to close EC-002-06.
- Replace the iterated-MD5 portable fallback with a modern KDF where native bcrypt/argon2 is unavailable.
