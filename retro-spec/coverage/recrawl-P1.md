# Re-crawl Coverage Report — Partition P1 (Root entry scripts + top-level includes)

Verification round, adversarial / fresh-eyes. Read-only on PHP + specs.
Reviewer partition: **P1** (34 files, ~1,797 non-empty src lines).
Date: 2026-06-10.

## Method

Walked every file top-to-bottom: each class, method, top-level statement block,
branch with distinct behavior, error path. Resolved cited spec ids by opening the
owning spec (FS-001, FS-002, FS-003, FS-010, FS-011, FS-022, FS-032, FS-033,
FS-041, FS-043, FS-050, FS-060, FS-090, FS-091) and confirming the requirement
actually documents the tagged unit. Spot-checked id resolution on well over 30%
of tags (every cross-spec id in mysql.php / mysqli.php / api.tickets.php /
api.cron.php / the client-portal entry scripts was opened and confirmed).

## Summary counts

| Metric | Count |
|--------|------:|
| Units examined (classes / methods / functions / top-level entry blocks / distinct branches) | ~155 |
| COVERED (tag present AND cited id documents the unit) | ~150 |
| UNCOVERED | 2 |
| MIS-TAGGED | 1 (same site as UNCOVERED finding #1) |
| TRIVIA (bare redirects, license headers, self-name guards subsumed by BS-001, direct-access dies) | ~8 |

Overall P1 tagging is unusually thorough and accurate. Every cross-spec id I
spot-checked resolved to a real requirement that genuinely describes the tagged
behavior (e.g. FS-003.26–.29 for the db_* drivers, FS-043.4/.6/.9/.10 +
BS-433/436/438 for the API controllers, FS-041.1/.2/.6 + BS-041.2/.3/.7 for the
pipe/email path, FS-010.x + BS-010.x for the client portal, FS-060.6/.7 +
BS-060-02 for the installer-facing db/config code). The non-`###` BS rules in
FS-003 (BS-024/026/027/028) and FS-060 (BS-060-02) also resolve. Only the two
findings below survived the adversarial pass.

## Findings (UNCOVERED / MIS-TAGGED)

| # | file:line | unit | what the code does | why not covered / right id | suggested owner + new rule? |
|---|-----------|------|--------------------|-----------------------------|------------------------------|
| 1 | `logout.php:18-28` | logout flow (link-token guard) | On a missing/invalid `$_GET['auth']` link token the code emits `@header('Location: index.php')` but **does NOT `exit`** — execution falls straight through and still runs `$_SESSION['_client']=array(); session_unset(); session_destroy();`. So the link-token check is effectively non-blocking: a tokenless / forged logout GET still destroys the client session. | **MIS-TAGGED + UNCOVERED.** Tag at line 18 claims `FS-001.11 … validateLinkToken guards the logout GET link before tearing the session down`, but the code does not guard it (no exit). FS-010 Flow 5 + the `logout.php?auth=<token>` nav reference (FS-010 §"My Tickets"/nav) imply the token authenticates logout, yet the real behavior (token failure does not prevent logout; logout is effectively unauthenticated, i.e. a low-severity logout-CSRF) is documented nowhere. | FS-010 (client portal) — add an **EC** (e.g. "EC-010.x: Logout link-token is advisory — a tokenless/forged logout GET still destroys the session because the guard lacks an `exit`") and/or a **KL**. The line-18 tag should be corrected to stop claiming the token "guards" logout. |
| 2 | `logo.php:22-24` | offline-exempt logo endpoint session suppression | Installs a no-op session save handler (`session_set_save_handler('noop',…)`) and defines `DISABLE_SESSION` **before** booting `client.inc.php`, so the inline image fetch creates/writes no session record. This is a **client-realm** endpoint doing the same session-disable the API realm does. | **UNCOVERED.** Tag cites `FS-001.8: Client-Realm Gate — disables session writes for the inline image fetch`, but FS-001.8 contains no session-disable acceptance criterion at all; it only describes offline enforcement, client-session resolution, CSRF, nav. The `DISABLE_SESSION`/noop-save-handler mechanism is documented **only** for the API realm (FS-043 §"Stateless API note" / BS-436), not for the client logo endpoint. BS-010.10 covers the offline *exemption* of logo.php but not the session-write suppression. | FS-001 (or FS-010) — extend an acceptance criterion / add a BS: "The logo endpoint installs a no-op session save handler and sets `DISABLE_SESSION` so inline-image fetches do not allocate/refresh a session record" (mirrors FS-043 BS-436 for the client realm). |

## Soft observations (not counted as findings)

- `ajax.php:30` tags the `/config/client` route as bare `FS-033` (no `.N`), self-noting
  "no dedicated .N". This is acceptable: the `/config/client` projection IS documented
  in **FS-033.10**'s companion-endpoint note (lines 291-295), which assigns the consumer
  to FS-010/FS-011. Tightening the tag to `FS-033.10` would be more precise, but the
  behavior is genuinely covered — not a gap.
- `api/index.php` (bare `header('Location: ../')`) and `include/index.php` (identical)
  are untagged but pure TRIVIA (directory-listing redirect stubs); correctly skip-worthy.
- `main.inc.php` forced `display_errors` on, the `BANLIST_TABLE` dead constant, and the
  `X-Forwarded-For` overwrite are all explicitly tagged to KL-001 / KL-006 / KL-005 —
  good defensive tagging of known limitations.
- `class.timezone.php` `getName()`/`getDesc()` reading the never-populated `$this->info`
  slot is correctly tagged to KL-013 (timezone name not reliably retrievable).

## Round 2 closure (2026-06-10)

Both P1 findings are **CLOSED**.

- **Finding #1 — `logout.php:18-28` (inert link-token guard / logout-CSRF):** documented in FS-010 as new **BS-010.14** (Logout Link-Token Guard Is Non-Blocking), **EC-010.13** (Tokenless / Forged Logout GET), and **KL-010.10** (guard inert; logout effectively unauthenticated), each cross-referencing the staff-side mirror **FS-002 KL-002-10 / EC-002-14**. FS-010.10 identity-strip criterion + Flow 5 updated to stop implying the token guards logout. `logout.php` retagged: the false `FS-001.11` "guards the logout" line replaced with `@implements BS-010.14 / EC-010.13 / KL-010.10` (FS-010.6 line retained).
- **Finding #2 — `logo.php:22-24` (session-write suppression):** documented in FS-010 as new **BS-010.15** (Logo Endpoint Suppresses Session Writes), cross-referencing the API-realm mirror **FS-043 BS-436** and noting the BS-010.10 offline-exemption interplay. `logo.php` retagged from the criterion-less `FS-001.8` to `@implements BS-010.15`.
