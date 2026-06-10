# FS-010: Public Client Portal — Ticket Lookup, View & Login

## Overview

The Public Client Portal is the unauthenticated/lightly-authenticated front end through which end users (the people who open support tickets) reach the help desk. It comprises the **landing page** (`index.php`), the **"Check Ticket Status" login form** (`view.php` + `login.php`), the **per-ticket conversation view** (the `tickets.inc.php` / `view.inc.php` templates), and the shared **client page shell** (`client.inc.php` bootstrap + `header.inc.php` / `footer.inc.php` chrome + `UserNav` link bar).

The defining behavioral fact of this portal is that **there is no client account**. A client is modeled entirely on the data of a single ticket: authentication is the pairing of a **ticket number** with the **email address** recorded on that ticket (interactive login), or an emailed **access token** that auto-logs the user in (link login). On success a short-lived **client session** is established (`ClientSession`, keyed on `email` + ticket number in `$_SESSION['_client']`). The session grants the user read access to their ticket's thread (messages and staff responses only — never internal notes), the ability to post a reply (which reopens a closed ticket), and — when the operator has enabled "related tickets" — a list of every ticket sharing that email address.

This specification owns: the landing-page two-button hub, the login/lookup form and its validation, the interactive and access-link login flows, brute-force throttling (per-session strike counter + lockout), the client session lifecycle (creation, token hashing, idle expiry, validation, regeneration), the client-facing ticket view (header info, thread rendering, reply form, attachment links), the "My Tickets" list (search/sort/paginate by email), and the client page shell/header/footer/nav.

**Owned elsewhere (referenced, not restated):** ticket creation and the `open.php` submission form (FS-011); captcha verification *use* on the submission form (FS-011 — this spec documents only the captcha image generator that the portal exposes); the staff-side ticket view and the `Ticket::postMessage` reply-posting internals (FS-021); the `Ticket` / thread / `ost_session` / config table schemas and enum sets (FS-091); the application bootstrap (`main.inc.php`, `$ost`, `$cfg`), CSRF mechanics, and `UserNav` base (FS-001); cryptography/validation/formatting helpers (`Validator`, `Format`) (FS-003); knowledge base / FAQ links surfaced on the landing page (FS-050).

> **Snapshot note (load-bearing):** In this 1.7 source snapshot the root script `tickets.php` is **absent**. Both `view.php` and `login.php` end by `require('tickets.php')`, and the templates `tickets.inc.php` / `view.inc.php` are clearly the bodies that `tickets.php` would render and host the reply-POST handler. This spec documents the *behavior the present files encode and expect* (including the `tickets.php` contract they depend on); the missing controller is recorded as **KL-010.1**. Treat every reference to "the ticket-view controller" as the logical `tickets.php` that these templates require.

---

## Functional Requirements

### FS-010.1: Support-Center Landing Page

**Description**: The system shall present an unauthenticated landing page (`index.php`) that acts as the portal's hub, offering the two primary entry points (open a new ticket, check ticket status) and, when configured, a custom welcome body and a knowledge-base prompt.

**Acceptance Criteria**:
- The page boots through the client shell (`client.inc.php`; see FS-010.9) with section context `home`.
- A `#landing_page` region renders, in order:
  - A welcome body: if a configured landing page exists (`$cfg->getLandingPage()`), its stored body HTML is echoed; otherwise the literal fallback heading `<h1>Welcome to the Support Center</h1>`.
  - A `#new_ticket` panel headed **"Open A New Ticket"**, the prompt text *"Please provide as much detail as possible so we can best assist you. To update a previously submitted ticket, please login."*, and a green button **"Open a New Ticket"** linking to `open.php`.
  - A `#check_status` panel headed **"Check Ticket Status"**, the prompt text *"We provide archives and history of all your current and past support requests complete with responses."*, and a blue button **"Check Ticket Status"** linking to `view.php`.
- When the knowledge base is enabled (`$cfg->isKnowledgebaseEnabled()`), a closing line renders: *"Be sure to browse our [Frequently Asked Questions (FAQs)](kb/index.php), before opening a ticket."* Otherwise that line is omitted.
- The landing page requires no session; it renders identically for guests and logged-in clients (the header chrome differs per FS-010.10).

### FS-010.2: Check-Ticket-Status Login Form

**Description**: The system shall present a login/lookup form that collects an email address and a ticket ID to authenticate a client into their ticket.

**Acceptance Criteria**:
- The form (`#clientLogin`, rendered by `login.inc.php`) is shown by both `login.php` (the canonical login page, nav active = `status`) and is the form a client uses to "View Status".
- The template renders a heading **"Check Ticket Status"** and the prompt *"To view the status of a ticket, provide us with the login details below."* above the form.
- The form posts to `login.php` via HTTP POST and includes a CSRF token field (`csrf_token()`).
- Fields:
  - **E-Mail Address** — text input `lemail`, prefilled from `$_POST['lemail']` else `$_GET['e']`.
  - **Ticket ID** — text input `lticket`, prefilled from `$_POST['lticket']` else `$_GET['t']`.
- Submit button label: **"View Status"**.
- Any login error string (`$errors['login']`) is rendered in bold above the fields.
- A closing line offers recovery: *"If this is your first time contacting us or you've lost the ticket ID, please [open a new ticket](open.php)."*
- Prefill values pass through input-sanitization (`Format::input`) before being echoed into the value attributes.
- The form is only reachable from a context where the client shell has defined `OSTCLIENTINC`; a direct include otherwise dies with `Access Denied`.

### FS-010.3: Interactive Login (Ticket ID + Email)

**Description**: The system shall authenticate a client interactively by matching a submitted ticket number and email against an existing ticket, establishing a client session on success and redirecting to that ticket's view.

**Acceptance Criteria**:
- Triggered by a POST to `login.php`. The CSRF token is validated first; an invalid/absent token yields HTTP 400 "Valid CSRF Token Required". On a valid token, the CSRF token is **rotated** (the used token cannot be replayed) before the login attempt proceeds (anti-brute-force; see BS-010.6).
- The attempt calls the client login routine (`Client::login`) with the trimmed ticket ID (`lticket`), trimmed email (`lemail`), and **no** auth token (interactive logins do not use the emailed token).
- **Validation order** (see BS-010.1):
  1. The submitted email must be a syntactically valid email and the ticket ID must be non-empty; otherwise the error *"Valid email and ticket number required"* is set and the attempt fails without a database lookup.
  2. The ticket is looked up by external ticket number **and** email together (`Ticket::lookupByExtId(ticketID, email)`); the ticket must exist.
  3. The email on the matched ticket must equal the submitted email (case-insensitive).
- On success: a `ClientSession` is created, `$_SESSION['_client']` is populated (see Data Requirements), a session token hash is set, a debug login entry is logged (`"<email>/<ticket#> logged in [<ip>]"`), and the PHP session ID is regenerated (old session destroyed). The routine returns the authenticated client object.
- After success the controller redirects (`Location: tickets.php?id=<ticket#>`) to the ticket view and includes that controller; the ticket owner is assumed to be the logged-in user (a documented simplification — see KL-010.2).
- On failure with no specific error already set, the generic message *"Authentication error - try again!"* is surfaced; the login form re-renders with the message.

### FS-010.4: Access-Link (Auto) Login

**Description**: The system shall auto-authenticate a client who arrives via an emailed access link carrying the ticket number, email, and a valid per-ticket auth token, without requiring the client to type anything.

**Acceptance Criteria**:
- Triggered by `view.php` when the visitor is not already a valid client and the GET query carries all three of `t` (ticket number), `e` (email), and `a` (auth token).
- Auto-login is permitted **only** for GET requests, and for GET requests the auth token is **required** (a GET attempt without `a` fails with error *"Invalid method"*; see BS-010.2).
- The auth token must equal the ticket's computed auth token (`Ticket::getAuthToken()` = a salted hash of the ticket id + lowercased email + secret salt). A mismatch fails the attempt.
- The same email/ticket validation as interactive login applies (FS-010.3 validation order); on success the identical session-establishment + ID-regeneration steps run.
- On success, if the resolved ticket number equals the requested `t`, the controller redirects to `tickets.php?id=<ticket#>`.
- After the auto-login attempt (success or not) `view.php` falls through to render the ticket-view controller (`tickets.php`); an unauthenticated visitor who provided no/invalid link parameters is therefore presented the Check-Ticket-Status login form by that controller.
- The auth token is a **bearer credential**: anyone holding the link can read the ticket. There is no expiry on the token itself (only the resulting session has an idle timeout). See KL-010.3.

### FS-010.5: Brute-Force Throttling & Lockout

**Description**: The system shall throttle repeated failed client logins using a per-session strike counter and a temporary lockout, and shall alert/log on excessive attempts.

**Acceptance Criteria**:
- Each failed login increments a per-session strike counter (`$_SESSION['_client']['strikes']`).
- When strikes exceed the configured maximum (`$cfg->getClientMaxLogins()`), the session is **intended** to be locked: the error becomes *"Access Denied"* with body *"Forgot your login info? Please [open a new ticket](open.php)."*, a "last strike" timestamp is recorded, and an error-level event *"Excessive login attempts (user)"* is logged (and emailed to the admin when login-error alerts are enabled, `$cfg->alertONLoginError()`). **In this source snapshot this entire lockout branch is unreachable dead code** — see BS-010.12 / KL-010.8: the generic *"Invalid login"* error is assigned into `$errors` immediately before the lockout guard, and that guard is gated on `$errors` being empty (`if (!$errors && strikes > max)`), so it can never fire. The strike counter still increments, but the lockout, the *"Access Denied"* message, the `laststrike` timestamp, and the admin alert are never produced in this snapshot.
- While locked: any further login attempt within the configured lockout window (`$cfg->getClientLoginTimeout()`, stored as minutes, applied as seconds) is rejected with error *"Excessive failed login attempts"* / body *"You've reached maximum failed login attempts allowed. Try again later or [open a new ticket](open.php)"*, and the last-strike timestamp is renewed (the window restarts on each attempt during lockout). This lockout-window check runs at the **top** of the login routine, before any validation, keyed solely on the presence of `laststrike`. **Because `laststrike` is only ever assigned inside the unreachable lockout branch above (BS-010.12 / KL-010.8), this window check never engages in this snapshot either** — the throttling is effectively inert end-to-end.
- When the lockout window has elapsed, the last-strike timestamp is cleared and the strike counter is reset, allowing a fresh round of attempts.
- Every second failed attempt (strikes even, i.e. `strikes % 2 == 0`) that is not a full lockout logs a warning-level event *"Failed login attempt (user)"* with email, ticket #, IP, and time.
- All alert/log payloads include the submitted email, ticket number, client IP, timestamp, and the attempt count.

### FS-010.6: Client Session Lifecycle & Validation

**Description**: The system shall maintain a client session that is established on login, validated on every client page load, refreshed (sliding token) while active, and considered invalid once idle past the client session timeout.

**Acceptance Criteria**:
- A client session (`ClientSession`) is constructed from the email + ticket number stored in `$_SESSION['_client']`. It loads the underlying ticket data (the `Client` model) and wraps a `UserSession` keyed on the lowercased email.
- The session is held in `$_SESSION['_client']` with keys: `userID` (email), `key` (ticket number — *"acts as password when used with email"* per the source), `token` (the session hash token), plus `strikes`/`laststrike` for throttling. `$_SESSION['TZ_OFFSET']` and `$_SESSION['TZ_DST']` are set from config at login.
- **Token shape, hashing, and refresh**: owned by [FS-002.11](./FS-002-staff-authentication-sessions-access-control.md) (`UserSession`). The client realm reuses that shared primitive; this spec records only the client-specific deltas (id = lowercased email, key = ticket number, IP-binding disabled, idle window from `client_session_timeout`).
- **Validity check** (`ClientSession::isValid`) requires all of: a loaded ticket id is present; the current PHP `session_id()` matches the one captured at session construction; and the stored token validates per FS-002.11 and is **not idle** beyond the client session timeout (`$cfg->getClientTimeout()` = `client_session_timeout` minutes × 60). Client sessions are **not** IP-bound (the IP-binding check is disabled for clients; contrast staff sessions).
- **Sliding refresh**: on every client page load where the session is valid, the token is refreshed (`refreshSession` writes a fresh token with the current time), extending the idle window.
- On login the PHP session ID is regenerated and the previous session record is destroyed from the session store, mitigating fixation.
- A session that fails validation is dropped: `$thisclient` is set to null and the visitor is treated as a guest.

> **Note**: See [FS-091](./FS-091-reference-data-enums-data-model.md) for the canonical `ost_session` table schema and config keys (`client_session_timeout`, `client_login_timeout`, `client_max_logins`). See [FS-002](./FS-002-staff-authentication-sessions-access-control.md) for the parallel `StaffSession` (which differs by using the staff timeout and honoring IP-binding when enabled).

### FS-010.7: Client Ticket View (Thread, Header, Reply)

**Description**: The system shall present a logged-in client a read view of their ticket — header metadata, the message/response thread (excluding internal notes), and a reply form — with an access guard scoping the view to the owning client.

**Acceptance Criteria**:
- The view renders only when `OSTCLIENTINC` is defined, a valid `$thisclient` exists, a `$ticket` is loaded, **and** `$ticket->checkClientAccess($thisclient)` passes; otherwise the template dies with `Access Denied!` (see BS-010.4 for the access rule).
- **Header block** (`#ticketInfo`): the ticket number with a "Reload" link (`view.php?id=<#>`), and two info tables showing **Ticket Status** (capitalized), **Department**, **Create Date**, **Name**, **Email**, and **Phone**. The department shown is the ticket's department **only if that department is public**; otherwise the system default department is substituted so internal department names are not leaked (see BS-010.5). The subject is shown below.
- **Thread** (`#ticketThread`): iterating the client thread (`Ticket::getClientThread`, restricted to types Message `M` and Response `R`):
  - Internal notes (type `N`) are never returned and, defensively, any entry whose type is not message/response is skipped (guarding against backend mistakes — BS-010.7).
  - Each entry shows its created datetime, a poster label, and the formatted body; an empty body (`-`) renders as `(EMPTY)`.
  - For staff **responses**, the staff poster name is blanked when the operator hides staff names (`$cfg->hideStaffName()`) or the entry has no staff id (see BS-010.8).
  - When an entry has attachments, its attachment download links are rendered below the body (the links resolve the underlying stored files — see FS-022/FS-091).
- **Reply form** (`#reply`, posts multipart to `tickets.php?id=<#>#reply` with hidden `id=<#>` and `a=reply`): includes a CSRF token, a required **Message** textarea, and — when online attachments are allowed (`$cfg->allowOnlineAttachments()`) — a multi-file attachment input (`attachments[]`). When the ticket is closed, the hint text reads *"Ticket will be reopened on message post"*; otherwise *"To best assist you, please be specific and detailed"*. A `$errors['message']` validation message renders inline beside the Message label. Buttons: **Post Reply**, **Reset**, **Cancel** (history back).
- **Error repopulation**: on a failed POST (when `$_POST` and `$errors` are both present), the template re-populates the Message textarea from the HTML-escaped prior input (`Format::htmlchars($_POST)['message']`), so the client does not lose their typed reply on a validation error.
- The ticket-view template renders its **own** error/notice/warning message banner (`#msg_error` / `#msg_notice` / `#msg_warning`, same precedence as the shell) between the thread and the reply form, in addition to the shell header's banner.
- Posting a reply reopens a closed ticket (the message-post path reopens on end-user reply); the reply-POST handling itself is performed by the ticket-view controller and the ticket message-post routine.

> **Note**: The reply-POST handler (validation of the message, attachment intake, `Ticket::postMessage`, reopen, autoresponse/alerts) lives in the ticket-view controller (`tickets.php`, absent here — KL-010.1) and in [FS-021](./FS-021-staff-ticket-view-workflow.md) / [FS-011](./FS-011-public-ticket-submission.md). This spec owns only the client-facing form contract and the reopen-on-reply expectation.

### FS-010.8: "My Tickets" List (Related Tickets by Email)

**Description**: When the operator enables related tickets, the system shall present a logged-in client a searchable, sortable, paginated list of every ticket that shares their email address.

**Acceptance Criteria**:
- The list (`tickets.inc.php`) renders only when `OSTCLIENTINC` is defined, `$thisclient` is a valid client, **and** `$cfg->showRelatedTickets()` is true; otherwise it dies with `Access Denied`. When related tickets are disabled, the client is limited to the single ticket they logged into (see BS-010.9).
- The result set is every ticket whose `email` equals the client's email (the email is the scoping key — not a per-ticket owner id).
- **Status filter**: a `status` request value of `open` or `closed` filters accordingly; any other value is ignored (treated as "all"). When no status is requested and the client has open tickets, the list defaults to **open**.
- **Search**: with `a=search` and a query `q`, a numeric query matches by ticket number prefix; a non-numeric query performs a deep search across ticket subject and thread body (joining message/response thread entries).
- **Sort**: the recognized sort keys are `id`, `name`, `subject`, `email`, `status`, `dept`, `date` (mapping to `ticketID`, `ticket.name`, `ticket.subject`, `ticket.email`, `ticket.status`, `dept_name`, `ticket.created`); default sort = `date`, default order = `ASC`; invalid sort/order values fall back to the defaults. **Latent defect (KL-010.6):** the column-header sort anchors emit `sort=ID` and `sort=subj`, which (lowercased to `id`/`subj`) do not all match the recognized keys — `subj` is not a key (the key is `subject`) so the Subject header silently falls back to the date sort. Additionally, the order-by fallback literal is `'ticket_created'` (an underscore form that is not the real `ticket.created` column), reachable only if the sort map lookup ever yields empty.
- **Pagination**: results are paged at the client page limit (`PAGE_LIMIT` = `DEFAULT_PAGE_LIMIT`) with page links; a caption summarizes the showing range and the active status/search context (e.g. *"Search Results: ... Open Tickets"*).
- **Columns**: Ticket #, Create Date, Status, Subject, Department, Phone Number. Subject is truncated (40 chars in the cell, department truncated to 30); an attachment icon is appended when the ticket has attachments; answered **open** tickets are bolded (both subject and ticket #). Department falls back to the default public department name (`Dept::getDefaultDeptName()`) when the ticket's department row is non-public. A status `<select>` shows live open count always and a closed count only when the client has closed tickets; an empty result renders *"Your query did not match any records"*.
- **Phone column is a latent defect (KL-010.6)**: the list's SELECT projects ticket subject/name/email/status/source/created and an `attachments` count, but it does **not** project the `phone`/`phone_ext` columns, while each row echoes `$row['phone']`/`$row['phone_ext']`. As written the Phone Number cell is always empty.
- Each result row's ticket-number/subject anchor exposes the ticket's email as the link `title`, and applies a source-specific icon class (`<source>Ticket`) derived from the ticket source value.
- Each row links to `tickets.php?id=<#>` (the per-ticket view).
- **Email-only client resolution** (`Client::lookupByEmail` → `Client::getLastTicketIdByEmail`): the system can resolve a `Client` model from an email address **alone** (without a ticket number) by selecting a single ticket number for that email and then loading the `Client` from it (`lookup(ticketId, email)`). The selecting query is `SELECT ticketID FROM ost_ticket WHERE email=<email> ORDER BY created LIMIT 1`. This path underpins email-scoped client construction where no ticket number is in hand.
- **Selector defect (KL-010.11)**: the method is named `getLastTicketIdByEmail` and is described in code as returning the "most-recent" ticket, but the query orders `created` **ascending** (no `DESC`), so it actually returns the **oldest** ticket for that email. The resolved `Client` is therefore seeded from the client's first-ever ticket rather than their latest — a name/comment-vs-code contradiction. Because the `Client` model carries the same `name`/`email`/`phone` for any of that email's tickets, the practical impact is limited to which ticket id seeds the model, but the selection is the opposite of what the name implies.

> **Note**: The client status/open/closed counts come from `Client::getTicketStats` → `Ticket::getClientStats(email)` (a per-email aggregate of open vs. closed). See [FS-091](./FS-091-reference-data-enums-data-model.md) for ticket status enum values.

### FS-010.9: Client Page Bootstrap & Guard (`client.inc.php`)

**Description**: The system shall provide a shared client-page bootstrap that boots the application, enforces the offline gate and CSRF protection on POSTs, resolves the current client session, and seeds the client navigation.

**Acceptance Criteria**:
- The file refuses direct execution (dies *"kwaheri rafiki!"* if requested as the script itself) and fatally errors if the application bootstrap (`main.inc.php`) is missing.
- It boots through `main.inc.php` (yielding `$ost`, `$cfg`, and the `INCLUDE_DIR`/`ROOT_PATH` constants) and defines client-scope constants `CLIENTINC_DIR`, `OSTCLIENTINC`, and `ASSETS_PATH`.
- **Offline gate**: if the help desk is not online (`$ost->isSystemOnline()` false), the offline page is shown and the request exits — except for `logo.php`, which is always served (see BS-010.10).
- Required client classes are loaded (client, ticket, department).
- It resets request-scoped vars (`$errors=[]`, `$msg=''`, `$thisclient=$nav=null`), then resolves the client session: if both `$_SESSION['_client']['userID']` and `['key']` are present, it constructs a `ClientSession`; if that session is valid it is refreshed (sliding token), otherwise `$thisclient` is discarded (set null).
- **CSRF on POST**: any POST without a valid CSRF token is redirected to `index.php` (and, defensively, dies *"Action denied (400)!"* if the redirect fails). This is the blanket guard for client POSTs (the login controller additionally checks/rotates its own token per FS-010.3).
- It sets the client page limit (`PAGE_LIMIT = DEFAULT_PAGE_LIMIT`) and constructs the client navigation (`UserNav($thisclient, 'home')`).

### FS-010.10: Client Page Shell — Header, Footer & Navigation

**Description**: The system shall wrap every client page in a consistent shell: a branded header with an identity/login strip, a navigation link bar derived from session state, a message banner, and a footer.

**Acceptance Criteria**:
- **Header** (`header.inc.php`): sets the page title to the configured help-desk title (else *"osTicket :: Support Ticket System"*), declares UTF-8, loads the core + theme stylesheets, a print stylesheet, and the portal JavaScript, and renders a logo (`logo.php`) linking home.
- **Identity strip**:
  - For a valid logged-in client (`$thisclient` present, an object, and `isValid()`): shows the client's name, a **"My Tickets (N)"** link (only when related tickets are enabled, N = `getNumTickets()`), and a **"Log Out"** link carrying a link-token (`logout.php?auth=<token>` from `$ost->getLinkToken()`). **Note**: the link-token on this URL is *advisory only* — `logout.php` does not actually enforce it (a tokenless/forged logout GET still tears the session down). See BS-010.14 / EC-010.13 / KL-010.10.
  - For a guest: shows *"Guest User - [Log In](login.php)"* — but **only when `$nav` is set**. On a page that boots without a `$nav` (e.g. a template rendered outside the normal shell), neither the guest strip nor the nav bar is shown.
- **Navigation bar** (`#nav`): rendered from `UserNav::getNavLinks()` **only when `$nav` is set**; when `$nav` is unset a horizontal rule (`<hr>`) is rendered in its place. Each link's anchor classes include the link key and an `active` class when active. The link set adapts to session and config:
  - Always: **Support Center Home** (`index.php`); **Open New Ticket** (`open.php`).
  - When the knowledge base is enabled: **Knowledgebase** (`kb/index.php`).
  - When a valid client is present: **My Tickets (N)** (`tickets.php`) if related tickets are enabled, else **View Ticket Thread** (`tickets.php?id=<#>`).
  - When no valid client is present: **Check Ticket Status** (`view.php`).
- **Message banner**: exactly one of an error (`$errors['err']`, `#msg_error`), a notice (`$msg`, `#msg_notice`), or a warning (`$warn`, `#msg_warning`) is shown, in that precedence order.
- **Footer** (`footer.inc.php`): a copyright line with the current year, a "Powered by osTicket" link, and the shared overlay/loading scaffolding.

### FS-010.11: Captcha Image Generation

**Description**: The system shall expose an endpoint that generates a random captcha image and records its hash in the session, for use by forms that require human verification.

**Acceptance Criteria**:
- `captcha.php` boots the application and emits a PNG captcha image (length 5, font 12, background image directory `images/captcha/`). The image is drawn as a black string centered over a randomly chosen background PNG.
- Generation requires the GD image extension specifically (`extension_loaded('gd')` and `function_exists('gd_info')`); if GD is unavailable the generator returns immediately, emitting nothing — and, importantly, **does not clear or set `$_SESSION['captcha']`** in that case (the clear/set both occur only on the GD-present path).
- On generation (GD present), the session captcha value is first cleared to an empty string, the image is output, and then `$_SESSION['captcha']` is set to the **MD5 hash of the generated string**. The plaintext is never stored in the session — only its hash, which a consuming form compares against the user's typed answer.
- The captcha string is an uppercase substring of a random MD5 (`strtoupper(substr(md5(rand(0,9999)), rand(0,24), len))`); the background is chosen at random from a fixed set of ten background PNGs: `cottoncandy.png`, `grass.png`, `ripple.png`, `silk.png`, `whirlpool.png`, `bubbles.png`, `crackle.png`, `lines.png`, `sand.png`, `snakeskin.png`.
- The `Captcha` class's own constructor defaults are length 6 / font 7, but the portal endpoint constructs it with length 5 / font 12.

> **Note**: The *consumption* of the captcha (the "must match `md5(typed)` == `$_SESSION['captcha']`" check, and the rule that captcha is required for guest ticket submission per `enableCaptcha`) belongs to the ticket-submission form — see [FS-011](./FS-011-public-ticket-submission.md). This requirement owns only the image-generation endpoint and the session-hash side effect.

---

## Business Rules

### BS-010.1: Identity Is Ticket-Number + Email, Validated In Order

**Rule**: A client is authenticated by the pair (ticket number, email recorded on that ticket). The login routine validates in a fixed order: (1) syntactic check — email must be a valid email and ticket number non-empty; (2) the ticket must exist when looked up by number **and** email; (3) the ticket's stored email must equal the submitted email (case-insensitive). Any failure yields a login failure.

**Rationale**: osTicket has no client account/password store; the ticket itself is the credential record. The email is the durable key (it scopes related tickets); the ticket number plus a matching email proves the requester is the person the ticket was opened for. *(Source comment: "osTicket uses email address and ticket ID to authenticate the user … Client is modeled on the info of the ticket used to login.")*

**Examples**:
- Email `jane@x.com` + ticket `123456` where the ticket's email is `Jane@X.com` → success (case-insensitive match).
- A malformed email → immediate *"Valid email and ticket number required"*, no database lookup.
- A real ticket number with the wrong email → *"Invalid login"* (no match returned).

### BS-010.2: Auth Token Is Mandatory For Link (GET) Login, Forbidden For Interactive (POST) Login

**Rule**: Automatic login is allowed only on GET requests and **requires** the per-ticket auth token; a GET login attempt without the token fails with *"Invalid method"*. Interactive POST login does **not** use the auth token at all (it relies on the email/ticket pairing).

**Rationale**: The emailed access link is a bearer credential delivered over GET; requiring the token prevents enumeration of tickets by guessing numbers on GET URLs. The interactive form already requires the user to know both the ticket number and email, so it does not need the token.

**Examples**:
- `view.php?t=123456&e=jane@x.com&a=<valid-token>` → auto-login succeeds.
- `view.php?t=123456&e=jane@x.com` (no `a`) → *"Invalid method"*, no auto-login; falls through to the login form.
- POST to `login.php` with email+ticket and no token → normal interactive login.

### BS-010.3: Auth Token Is A Salted Hash Of Ticket + Email

**Rule**: A ticket's auth token is the MD5 hash of the internal ticket id concatenated with the lowercased ticket email and a server secret salt. A link login succeeds only when the supplied token equals this computed token.

**Rationale**: Derives an unguessable per-ticket credential from stable ticket data plus a server secret, so links can be emailed without storing a separate token, while remaining unforgeable without the secret.

**Examples**:
- The same ticket always produces the same token (stable link), until the secret salt changes.
- Two tickets with different ids/emails produce different tokens.

### BS-010.4: Client Ticket Access Is Scoped By Email (Or, If Enabled, Login Ticket)

**Rule**: A client may view a ticket only if the client's email matches the ticket's email (case-insensitive). Additionally, when related tickets are enabled, a client may view a ticket whose number equals the ticket the client logged into. Any other access attempt is denied.

**Rationale**: Email is the ownership key for the help desk's view of "this person's tickets". The login-ticket clause supports the single-ticket (related-tickets-disabled) case where the client's email might differ in edge cases but they hold a valid session for that specific ticket.

**Examples**:
- A client logged in as `jane@x.com` can open any ticket whose email is `jane@x.com`.
- That client cannot open a ticket belonging to `bob@y.com` → `Access Denied!`.

### BS-010.5: Non-Public Department Names Are Never Leaked To Clients

**Rule**: In any client-facing view, a ticket whose assigned department is not public is displayed with the system default department name instead of the real department name.

**Rationale**: Internal routing departments may have names that reveal internal structure or be otherwise unsuitable for client display; substituting the public default avoids information leakage.

**Examples**:
- A ticket in a private "Escalations-Tier3" department shows the default public department (e.g. "Support") to the client.
- A ticket in a public department shows that department's real name.

### BS-010.6: Each Login Attempt Consumes A One-Time CSRF Token

**Rule**: The login controller validates the CSRF token, then rotates it so the same token cannot be reused on a subsequent attempt; a new token must be fetched (a fresh form load) for each attempt.

**Rationale**: Beyond standard CSRF defense, token rotation per attempt frustrates both parallel and serial brute-force scripts, which would otherwise replay a single captured token across many guesses.

**Examples**:
- A script that POSTs the same captured CSRF token twice has its second attempt rejected with HTTP 400.
- A legitimate user who mistypes simply reloads the form (getting a fresh token) and retries.

### BS-010.7: Internal Notes Are Never Shown To Clients

**Rule**: The client thread view returns only message (`M`) and response (`R`) entries; internal notes (`N`) are excluded at the query level, and any non-message/non-response entry that somehow appears is additionally skipped at render time.

**Rationale**: Internal notes are staff-only working notes; exposing them would breach the staff/client boundary. The double guard (query filter + render skip) defends against backend mistakes.

**Examples**:
- A ticket with two client messages, one staff response, and three internal notes shows the client exactly three entries (two messages + one response).

### BS-010.8: Staff Identity In Responses Is Configurable

**Rule**: In the client thread, the poster name on a staff response is blanked when the operator has enabled "hide staff name", or when the response carries no staff id.

**Rationale**: Some help desks present a unified "Support" voice rather than individual agent names; the config toggle and the missing-staff-id guard both blank the poster.

**Examples**:
- With hide-staff-name on, all responses show a blank poster.
- With it off, a response posted by staff member "Alex" shows "Alex".

### BS-010.9: Related-Tickets Visibility Is Operator-Gated

**Rule**: The "My Tickets" list and the multi-ticket navigation/identity affordances appear only when the operator enables related tickets; otherwise a client sees only the single ticket they authenticated into.

**Rationale**: Grouping all tickets by shared email is a privacy/scope decision left to the operator (a shared mailbox could otherwise expose unrelated tickets to anyone who opened one).

**Examples**:
- Related tickets on: header shows "My Tickets (5)"; the list page is reachable.
- Related tickets off: header shows "View Ticket Thread"; the list page dies with `Access Denied`.

### BS-010.10: Offline Gate Applies To All Client Pages Except The Logo

**Rule**: When the help desk is offline, every client page short-circuits to the offline page and exits, with the sole exception of the logo endpoint, which is always served.

**Rationale**: The offline page itself (and any branded shell) must still render its logo; serving the logo regardless avoids a broken offline screen.

**Examples**:
- Visiting `index.php` while offline shows the offline page.
- The offline page's logo image still loads (the logo endpoint is exempt).

### BS-010.11: Client Sessions Are Not IP-Bound; Staff Sessions May Be

**Rule**: Client session validation does not check the requester IP against the session IP. (The shared session machinery supports IP-binding, but it is disabled for clients.)

**Rationale**: Clients frequently follow emailed links from different networks/devices (mobile vs. desktop), so IP-binding would break legitimate access; staff access is more controlled and can be IP-bound by config.

**Examples**:
- A client opens the access link on desktop, then on mobile (different IP) within the idle window — both work.

### BS-010.12: Client Lockout Branch Is Unreachable In This Snapshot (Defect)

**Rule**: As written, the client login routine assigns the generic *"Invalid login"* error into the error collection on every failed attempt, then guards the excessive-attempts lockout on that same collection being empty (`if (!$errors && strikes > max)`). The two are mutually exclusive, so the lockout branch — which is the only place that sets the `laststrike` timestamp, the *"Access Denied"* message, and the admin alert — never executes. The upstream lockout-window check is keyed on `laststrike`, which is therefore never set, so it too never engages.

**Rationale**: This is a code-order defect, not an intentional design. Contrast the staff login routine ([FS-002](./FS-002-staff-authentication-sessions-access-control.md)), whose structurally similar lockout block does **not** pre-assign a per-attempt error before its `if (!$errors && strikes > max)` guard and therefore does fire. The spec documents the intended behavior (BS-010.5) but flags that, in this snapshot, only the per-attempt strike increment and the every-other-attempt warning log (BS-010.5 final clause) are actually live; the lockout, the *"Access Denied"* body, and the excessive-attempts admin alert are dead.

**Examples**:
- A scripted attacker exceeds `client_max_logins` failures in one session: the strike counter keeps climbing and warning-level *"Failed login attempt (user)"* events log on even counts, but the user is never actually locked out and no *"Excessive login attempts (user)"* admin alert is sent.

### BS-010.13: My-Tickets Status Defaults To "All" When The Client Has No Open Tickets

**Rule**: In the My-Tickets list, when the request carries no `status` value the list defaults to **open** only if the client currently has at least one open ticket; if the client has zero open tickets, no status filter is applied and the list shows **all** tickets. An explicit `status` value other than `open`/`closed` is normalized to "all".

**Rationale**: Defaulting to "open" on an empty open set would render an empty page; falling back to "all" surfaces the client's (closed) history instead.

**Examples**:
- A client with 2 open + 3 closed tickets, no `status` param → defaults to Open (shows 2).
- A client with 0 open + 4 closed tickets, no `status` param → shows All (4 closed).

### BS-010.14: Logout Link-Token Guard Is Non-Blocking (Defect)

**Rule**: The client logout link is advertised as `logout.php?auth=<link-token>`, and the script does check `$_GET['auth']` against `$ost->validateLinkToken()`. However, on a missing/invalid token the handler only emits `@header('Location: index.php')` and **does not `exit`**: execution falls straight through to `$_SESSION['_client']=array(); session_unset(); session_destroy();`. The link-token therefore does not actually guard logout — any reachable `logout.php` GET (with no `auth`, or a forged/expired one) destroys the client session. Logout is effectively unauthenticated (a low-severity logout-CSRF).

**Rationale**: This is a statement-ordering defect, not intentional design: the guard was meant to confirm the user deliberately clicked the emailed logout link before tearing the session down, but the absent `exit` neutralizes it. This mirrors the staff-side equivalent — see [FS-002](./FS-002-staff-authentication-sessions-access-control.md) EC-002-14 / KL-002-10, whose logout link-token guard has the identical inert-redirect defect. Restoring the protection requires halting (`exit`) after the redirect on token failure.

**Examples**:
- `logout.php` (no `auth` param at all) → session still destroyed; user returned to guest state.
- `logout.php?auth=<wrong>` → redirect header emitted toward `index.php`, but the session is destroyed anyway before the redirect takes effect.
- A third-party page that forces the browser to fetch `logout.php` logs the client out without any valid token.

### BS-010.15: Logo Endpoint Suppresses Session Writes (Client-Realm Mirror Of The API Realm)

**Rule**: Before booting the client shell, `logo.php` installs a no-op session save handler (`session_set_save_handler('noop', …)`) and defines `DISABLE_SESSION` so that serving the inline help-desk logo image neither allocates nor refreshes a session record. This is layered on top of the offline exemption (BS-010.10): the logo endpoint is both the sole offline-exempt client page **and** the sole client page that fetches without touching the session store.

**Rationale**: The logo is requested as an embedded `<img>` on essentially every client page (and on the offline page). Writing a session record for each inline-image fetch would needlessly grow the session table and could race the parent page's own session write. Suppressing the write makes the logo a pure static-asset fetch. This is the **client-realm** counterpart of the stateless-API session suppression documented for the API realm in [FS-043](./FS-043-external-api-cron-scheduler.md) BS-436 (same `noop` save-handler + `DISABLE_SESSION` mechanism, different realm and trigger).

**Examples**:
- A client page embeds `logo.php`; the image renders but no new/updated `ost_session` row is written for that sub-request.
- While the help desk is offline, the offline page's logo still loads (BS-010.10) and likewise writes no session record (BS-010.15).

---

## Data Requirements

> Canonical table/column schemas and enum sets live in [FS-091](./FS-091-reference-data-enums-data-model.md). This section lists only the fields this portal reads/writes functionally.

### Client Model (derived from a ticket row)

The `Client` model is populated from a single ticket row (`ost_ticket`) selected by external ticket number (and optionally email): `ticket_id` (internal id), `ticketID` (external number), `name`, `email`, `phone`, `phone_ext`. Derived: `fullname` (capitalized name), `username`/`email` (the email), `ticketID` (external number). No separate client/user table exists.

### Client Session State (`$_SESSION['_client']`)

| Key | Meaning | Set when |
|-----|---------|----------|
| `userID` | The client's email (the durable identity key) | on successful login |
| `key` | The external ticket number ("acts as password when used with email") | on successful login |
| `token` | Session token (shape/hashing owned by FS-002.11 `UserSession`) | on login + every refresh |
| `strikes` | Failed-login counter (this session) | incremented per failed login |
| `laststrike` | Lockout timestamp | *intended* to be set when strikes exceed max — but the setting branch is unreachable in this snapshot (BS-010.12 / KL-010.8), so it is never actually written |

Additional session keys set at login: `$_SESSION['TZ_OFFSET']`, `$_SESSION['TZ_DST']` (timezone display from config). `$_SESSION['captcha']` holds the MD5 of the current captcha string (set by the captcha endpoint).

### Ticket Statistics (per email)

`Ticket::getClientStats(email)` returns `{open, closed}` counts aggregated over all tickets with that email; surfaced as `getNumTickets`, `getNumOpenTickets`, `getNumClosedTickets`.

### Relevant Configuration Keys (read by this portal)

| Config key (via getter) | Use |
|-------------------------|-----|
| `client_session_timeout` (`getClientTimeout`, minutes×60) | Client session idle expiry |
| `client_login_timeout` (`getClientLoginTimeout`, minutes×60) | Lockout window after excessive failures |
| `client_max_logins` (`getClientMaxLogins`) | Failed-attempt threshold before lockout |
| `show_related_tickets` (`showRelatedTickets`) | Enables My-Tickets list & multi-ticket nav |
| `hide_staff_name` (`hideStaffName`) | Blanks staff poster names in responses |
| `allow_online_attachments` (`allowOnlineAttachments`) | Shows the reply attachment input |
| `send_login_errors` (`alertONLoginError`) | Emails admin on excessive login attempts |
| `enable_kb` (`isKnowledgebaseEnabled`) | Shows FAQ link & KB nav |
| `landing_page_id` (`getLandingPage`) | Custom landing-page body |
| `default_dept_id` (`getDefaultDept`) | Public department substituted for private ones |
| Help-desk title (`getTitle`) | Page title / logo alt |

Captcha generation parameters: length 5, font 12, background image set of ten fixed PNGs.

### Key URLs / Endpoints

| URL | Role |
|-----|------|
| `index.php` | Landing/hub page |
| `open.php` | New-ticket form (FS-011) |
| `view.php` | Check-status entry; handles access-link auto-login |
| `login.php` | Interactive login controller + form |
| `tickets.php` *(absent in snapshot — KL-010.1)* | Ticket view + My-Tickets list + reply POST handler |
| `logout.php?auth=<token>` | Client logout |
| `captcha.php` | Captcha PNG generator |
| `kb/index.php` | Knowledge base (FS-050) |
| `logo.php` | Help-desk logo (offline-exempt) |

---

## User Flows / Interactions

### Flow 1: Interactive Status Check

1. Guest clicks "Check Ticket Status" on the landing page (or "Log In" in the header) → `login.php`.
2. The Check-Ticket-Status form renders (nav active = "Check Ticket Status").
3. The user enters email + ticket ID and submits.
4. CSRF is validated and rotated; `Client::login` validates and matches the ticket.
5. On success: a client session is created, the PHP session id regenerated, and the user is redirected to `tickets.php?id=<#>` — their ticket view.
6. On failure: the form re-renders with the relevant error (validation, invalid login, or lockout).

### Flow 2: Access-Link (Email) Login

1. The user clicks an emailed link `view.php?t=<#>&e=<email>&a=<token>`.
2. `view.php` detects no current valid session and all three params present; calls `Client::login` with the auth token (GET).
3. The token is checked against the ticket's computed token; on match a session is established.
4. The user is redirected to `tickets.php?id=<#>` and lands directly on their ticket — no typing required.
5. If the token is missing/invalid, the user instead sees the Check-Ticket-Status form.

### Flow 3: Read Thread & Post a Reply

1. A logged-in client views `tickets.php?id=<#>` (the ticket view).
2. Access is checked (`checkClientAccess`); the header, thread (messages + responses only), and reply form render.
3. The client types a message (and optionally attaches files, if allowed) and clicks "Post Reply" (multipart POST, `a=reply`).
4. The controller validates and posts the message; if the ticket was closed it is reopened on post.
5. The thread re-renders with the new message appended.

### Flow 4: Browse My Tickets (Related Tickets Enabled)

1. The header shows "My Tickets (N)"; the client clicks it → `tickets.php` (list mode).
2. The list shows all tickets sharing the client's email, defaulting to Open.
3. The client searches (`q`), filters by status, sorts a column, or pages through results.
4. Clicking a row opens that ticket's view (Flow 3).

### Flow 5: Logout

1. The client clicks "Log Out" (`logout.php?auth=<link-token>`).
2. The client session is destroyed; the user returns to a guest state (header shows "Guest User - Log In").

> **Note**: The `auth` link-token is intended to confirm the user deliberately clicked the logout link, but the guard is inert in this snapshot — `logout.php` emits a redirect on a missing/invalid token yet does not `exit`, so the session is destroyed regardless. Any reachable `logout.php` GET (tokenless or forged) logs the client out. See BS-010.14 / EC-010.13 / KL-010.10.

---

## Edge Cases & Error Scenarios

### EC-010.1: Excessive Failed Logins → Temporary Lockout (Intended; Inert In Snapshot)
**Scenario**: A user (or script) exceeds the max failed-attempt count.
**Intended Behavior**: The session is locked with *"Access Denied"*; subsequent attempts within the lockout window are rejected with *"Excessive failed login attempts"* and renew the window. After the window elapses, strikes reset and login is allowed again. An admin alert/log is emitted on the lockout trip.
**Actual Snapshot Behavior**: The lockout never trips (BS-010.12 / KL-010.8). The user continues to receive the generic *"Invalid login"* error on each attempt; the strike counter increments and even-numbered attempts log a warning, but no *"Access Denied"*, no `laststrike`, no window enforcement, and no excessive-attempts admin alert occur.

### EC-010.2: GET Login Without Auth Token
**Scenario**: A GET request supplies email + ticket but no `a` token.
**Behavior**: Auto-login fails with *"Invalid method"*; no session is created; the user falls through to the Check-Ticket-Status form.

### EC-010.3: Valid Ticket Number, Wrong Email
**Scenario**: A correct ticket number is paired with an email that does not match the ticket.
**Behavior**: The lookup (by number + email) returns nothing → generic *"Invalid login"*; strike counter increments.

### EC-010.4: Malformed Email Submitted
**Scenario**: The email field is not a valid email.
**Behavior**: Immediate *"Valid email and ticket number required"* before any database lookup; the attempt does not consume a real lookup but does count as a failure.

### EC-010.5: Direct Include Of A Template
**Scenario**: A view template (`login.inc.php`, `view.inc.php`, `tickets.inc.php`) is requested directly without the client shell having defined `OSTCLIENTINC`.
**Behavior**: The template dies with `Access Denied`.

### EC-010.6: Client Opens A Ticket Not Theirs
**Scenario**: A logged-in client requests a ticket whose email differs (and, with related tickets off, is not their login ticket).
**Behavior**: `checkClientAccess` fails → the ticket view dies with `Access Denied!`.

### EC-010.7: Session Idle Beyond Timeout
**Scenario**: A client returns after the client session timeout has elapsed without activity.
**Behavior**: `isValid` fails on the idle check; `$thisclient` is dropped; the user is treated as a guest and must re-authenticate.

### EC-010.8: PHP Session ID Mismatch
**Scenario**: The stored session's captured session id no longer matches the current `session_id()` (e.g. after regeneration or a stale tab).
**Behavior**: `isValid` fails; the session is treated as invalid (guest).

### EC-010.9: Help Desk Offline
**Scenario**: A client navigates to any client page while the system is offline.
**Behavior**: The offline page renders and the request exits — except the logo endpoint, which still serves (BS-010.10).

### EC-010.10: Captcha Library (GD) Unavailable
**Scenario**: `captcha.php` runs where the GD image extension is not loaded.
**Behavior**: No image is emitted (the generator returns early before touching the session); `$_SESSION['captcha']` is left unchanged (neither cleared nor set), so any previously stored hash persists and a consuming form would have no fresh captcha to match. *(Captcha consumption/fallback behavior is owned by FS-011.)*

### EC-010.11: Empty Thread Body
**Scenario**: A thread entry has an empty body (`-`).
**Behavior**: The view renders `(EMPTY)` in place of the body.

### EC-010.12: Related Tickets Disabled But List Requested
**Scenario**: A client manually navigates to the list view while related tickets are disabled.
**Behavior**: The list template dies with `Access Denied`; the client is confined to their single ticket.

### EC-010.13: Tokenless / Forged Logout GET
**Scenario**: A `logout.php` request arrives with no `auth` token, or with an invalid/forged one (e.g. triggered by a third-party page).
**Behavior**: The link-token check fails and a redirect header (`Location: index.php`) is emitted, but because the handler does not `exit`, execution falls through and the client session is destroyed anyway (`$_SESSION['_client']=array(); session_unset(); session_destroy();`). Logout therefore succeeds without a valid token — the guard is inert (BS-010.14 / KL-010.10). Severity is low (forced logout / logout-CSRF only; no read access is gained). Mirrors the staff-side EC-002-14.

---

## Dependencies

| Dependency | Specification | Relationship |
|------------|---------------|--------------|
| App bootstrap (`main.inc.php`, `$ost`, `$cfg`), CSRF token machinery, `UserNav` base | FS-001 | Client shell boots through it; CSRF guard + nav rendering depend on it |
| Staff authentication & `StaffSession` (parallel session realm; IP-binding contrast) | FS-002 | `class.usersession.php` shares `UserSession`; staff session differs by timeout + IP-binding |
| Validation / formatting helpers (`Validator::is_email`, `Format::input/htmlchars/display/db_datetime/truncate/phone`) | FS-003 | Used throughout login validation & view rendering |
| New-ticket submission form (`open.php`) + captcha *consumption* + `enableCaptcha` | FS-011 | Landing/login pages link to it; this spec generates the captcha image, FS-011 verifies it |
| Staff ticket view, thread model, `Ticket::postMessage`, reopen, attachment intake | FS-021 | The reply-POST handler (in the absent `tickets.php`) and message posting live there |
| Attachment / stored-file links (`getAttachmentsLinks`) | FS-022 | Client thread renders download links to stored files |
| Knowledge base / FAQ links (`kb/index.php`, `isKnowledgebaseEnabled`) | FS-050 | Surfaced on landing page and client nav |
| Ticket / thread / session table schemas, status enums, config keys | FS-091 | Canonical data model the portal reads/writes |

---

## Known Limitations

### KL-010.1: The Ticket-View Controller (`tickets.php`) Is Absent In This Snapshot (HIGH IMPACT)
**Limitation**: `view.php` and `login.php` both end by `require('tickets.php')`, and `tickets.inc.php` / `view.inc.php` are the view bodies it would host (including the `a=reply` POST handler), but no `tickets.php` exists at the repository root in this source slice.
**Why It Exists**: Either trimmed from this snapshot or expected to be supplied by the deployment; the templates and redirects assume it.
**Impact**: As shipped here, a successful login redirect and the reply-post flow have no controller to land on. The reply-POST handling, the My-Tickets list controller, and the per-ticket view controller behaviors are documented from the templates' expectations rather than from an executing controller. Restoring `tickets.php` (binding `view.inc.php`/`tickets.inc.php`, wiring the reply POST to `Ticket::postMessage`) is required for a working portal.

### KL-010.2: "Ticket Owner Is Assumed" — No True Multi-View For CC'd Users
**Limitation**: The code repeatedly notes that on login the requester is assumed to be the ticket owner; planned per-auth-token multi-view (e.g. distinguishing BCC'd/CC'd recipients from the owner) is unimplemented (TODOs in `view.php` and `Client::login`).
**Impact**: Anyone who can authenticate to a ticket sees it as the owner; there is no differentiated read scope for collaborators/CCs.

### KL-010.3: Access-Link Token Has No Independent Expiry
**Limitation**: The emailed auth token is a stable salted hash of ticket + email with no per-token expiry; it remains valid until the secret salt changes. Only the resulting session has an idle timeout.
**Impact**: A leaked/forwarded access link grants ticket read access indefinitely. (Mitigations: secret-salt rotation invalidates all tokens at once; sessions still expire on idle.)

### KL-010.4: Ticket Number Functions As A Shared Secret / Password
**Limitation**: The source explicitly treats the ticket number as the "password" alongside the email (`$_SESSION['_client']['key']` comment). Ticket numbers are random but finite-length; email + a guessable/leaked ticket number is sufficient to authenticate.
**Impact**: Brute-force throttling (BS-010.5/EC-010.1) is the primary defense; there is no per-account password to strengthen. Ticket-number length and randomness (FS-091, `EXT_TICKET_ID_LEN`) govern guess resistance.

### KL-010.5: Throttling Is Per-Session Only
**Limitation**: The strike counter and lockout live in the PHP session (`$_SESSION['_client']['strikes']/['laststrike']`), not in a server-side per-IP or per-email store.
**Impact**: An attacker who discards cookies/session between attempts is not throttled by this mechanism (the warning/alert logging still fires per attempt, but the lockout resets with a fresh session). Network-level or web-server rate limiting would be needed for robust protection.

### KL-010.6: My-Tickets List Has Several Latent Defects (Search, Sort, Phone Column)
**Limitation**: The My-Tickets list (`tickets.inc.php`) contains four independent latent defects in this snapshot:
1. **Numeric search uses an undefined term** — the numeric-search branch interpolates `$queryterm` into its `LIKE 'ticketID LIKE '<term>%'` clause, but `$queryterm` is only assigned in the *non-numeric* (deep-search) branch, so a numeric ticket-number search builds its clause from an empty/undefined term and does not match by number prefix as intended.
2. **Subject sort header mismatch** — the Subject column header links with `sort=subj`, which is not a recognized sort key (`subject` is), so clicking it silently sorts by date instead.
3. **Order-by fallback literal** — the order-by fallback is the underscore literal `'ticket_created'` rather than the real `ticket.created` column.
4. **Phone column never populated** — the row SELECT omits `phone`/`phone_ext` while the row markup echoes them, leaving the Phone Number cell always blank.
**Impact**: Numeric search, the Subject sort, and the Phone column do not behave as the UI implies. These are latent defects in `tickets.inc.php` rather than deliberate design choices; flagged for the gap-close phase.

### KL-010.7: No i18n — User-Facing Strings Are Inline Literals
**Limitation**: All portal messages, labels, and prompts are hard-coded inline (no message catalog/gettext for the client side).
**Impact**: Localizing the client portal requires editing templates/classes directly; the exact literals captured in this spec are the only source of those strings.

### KL-010.8: The Client Brute-Force Lockout Is Dead Code In This Snapshot (HIGH IMPACT)
**Limitation**: The excessive-attempts lockout branch in the client login routine is guarded by `if (!$errors && strikes > max)`, but the routine has already assigned the generic *"Invalid login"* error into `$errors` immediately above that guard, so the guard's `!$errors` condition is always false. Consequently the lockout, the *"Access Denied"* message, the `laststrike` timestamp, and the excessive-attempts admin alert are never produced; the upstream lockout-window check is keyed on `laststrike` and therefore also never engages (BS-010.12). Only the per-attempt strike increment and the every-other-attempt warning log are live.
**Why It Exists**: A statement-ordering defect — the staff login routine (FS-002) places its strike increment and lockout block without pre-setting a per-attempt error, so the equivalent staff lockout *does* fire. The client routine sets `$errors['login']='Invalid login'` first, neutralizing its own lockout guard.
**Impact**: The portal's primary documented defense against ticket-number/email guessing (BS-010.5, EC-010.1, KL-010.4) is non-functional as shipped here. Brute-force resistance rests entirely on ticket-number length/randomness and any external (web-server/network) rate limiting. Restoring the lockout requires reordering the strike/lockout logic so the lockout guard evaluates before (or independently of) the generic *"Invalid login"* assignment.

### KL-010.9: Deep-Search Thread Join Can Multiply Rows / Depend On Distinct Counting
**Limitation**: The My-Tickets deep search joins the ticket-thread table (message/response entries only) to match a term against thread bodies; the listing query groups by ticket id and the total is computed with `count(DISTINCT ticket.ticket_id)`, but the result query relies on the `GROUP BY` to collapse the per-thread-entry fan-out.
**Impact**: Pagination totals and per-row attachment counts depend on the distinct/group-by collapsing the join fan-out correctly; a malformed grouping would skew counts. Documented as a structural sensitivity rather than an observed failure.

### KL-010.10: Logout Link-Token Guard Is Inert — Logout Is Effectively Unauthenticated
**Limitation**: `logout.php` validates `$_GET['auth']` against `$ost->validateLinkToken()`, but on failure it emits a redirect header without `exit`, so the session-destroy block runs regardless. The link-token therefore does not guard logout: any reachable `logout.php` GET — tokenless or forged — tears the client session down (BS-010.14 / EC-010.13).
**Why It Exists**: A missing `exit` after the redirect (statement-ordering defect). The staff realm carries the identical defect — see [FS-002](./FS-002-staff-authentication-sessions-access-control.md) KL-002-10 / EC-002-14.
**Impact**: Low-severity logout-CSRF: a third party can force a client logout via a cross-site request, but gains no read access (the worst case is a denial of the user's own session). Restoring the protection requires `exit`ing after the failure redirect so the teardown only runs on a valid token.

### KL-010.11: Email-Only Client Resolver Returns The OLDEST Ticket Despite Its "Last" Name
**Limitation**: `Client::getLastTicketIdByEmail($email)` (consumed by `Client::lookupByEmail`) selects `SELECT ticketID FROM ost_ticket WHERE email=? ORDER BY created LIMIT 1` — ascending, with no `DESC` — so it returns the **oldest** ticket for that email, contradicting both the method name ("Last") and the in-code comment ("most-recent"). The email-only `Client` resolution path (FS-010.8) is therefore seeded from the client's first-ever ticket.
**Why It Exists**: The `ORDER BY created` clause omits a `DESC` qualifier; the name/comment encode the intended (newest-first) behavior that the SQL does not deliver.
**Impact**: Low — the `Client` model carries the same `name`/`email`/`phone` regardless of which of that email's tickets seeds it, so downstream identity fields are unaffected; only the seed `ticketID` is the opposite of intended. Flagged for the gap-close phase (add `DESC` to align the query with the name) and to prevent any future consumer from relying on a "latest ticket" semantic that the resolver does not actually provide.

---

## Future Considerations

- Restore/implement `tickets.php` to bind the client ticket view, My-Tickets list, and reply-POST handler (KL-010.1).
- Per-auth-token multi-view distinguishing ticket owners from CC'd/collaborator recipients (KL-010.2).
- Time-bounded or single-use access-link tokens, independent of session idle timeout (KL-010.3).
- Optional client accounts with real passwords as an alternative to ticket-number-as-password (KL-010.4).
- Durable, per-IP/per-email brute-force throttling beyond the per-session counter (KL-010.5).
- Fix the numeric-search term in the My-Tickets list (KL-010.6).
- Externalize client-facing strings for localization (KL-010.7).
