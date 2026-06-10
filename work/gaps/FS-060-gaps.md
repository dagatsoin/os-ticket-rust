# FS-060 Installer & Setup Wizard — Gap Report (Phase 2)

Source slice re-read: `setup/index.php`, `setup/install.php`, `setup/setup.inc.php`,
`setup/inc/class.installer.php`, `include/class.setup.php`, all `setup/inc/*.inc.php`
step views, `include/ost-sampleconfig.php`, `setup/inc/ost-sampleconfig.php`,
`include/class.migrater.php` (`getUpgradeStreams`), `include/class.validator.php`
(username/password rules), and seed claims spot-checked against
`setup/inc/streams/core/install-mysql.sql`.

The existing spec was already high-quality; most gaps are IMPRECISE mechanism
details and a few genuinely MISSING behaviors. No INCORRECT material errors that
would mislead an implementer were found except one mislocated session side-effect.

| # | ID(s) | Type | Sev | What the code does | What the spec said | Fix applied |
|---|-------|------|-----|--------------------|--------------------|-------------|
| 1 | FS-060.1 / FS-060.8 / KL-060-02 | INCORRECT | Med | `$_SESSION['info']` is set in `install.php` after a successful install and is consumed by the dead `subscribe.inc.php` view; `install-done.inc.php` does NOT read it (derives URL from the `URL` constant). | FS-060.8 said the *completion view* "records `$_SESSION['info']` … used for the optional subscribe step". | Corrected FS-060.8: dispatcher (not the view) records it; subscribe view consumes it; done view does not. Documented `ucfirst(fname.' '.lname)` shape. |
| 2 | FS-060.1 | MISSING | Med | `install.php` line 79-81: a GET request `?s=ns` while the pointer is `subscribe` advances `subscribe`→`done` (the "No thanks." link). Only GET-driven transition in the wizard. | Spec only documented POST-driven transitions; `s=ns` branch undocumented. | Added the `?s=ns` GET transition + documented the `subscribe` POST validation rules (name/email/≥1 of alerts|news) and its pre-fill/default-checked behavior to FS-060.1. |
| 3 | FS-060.7 / BS-060-10 | IMPRECISE | High | Stream list comes from `include/upgrader/streams/streams.cfg` (defaults to `core`); a stream registers only if both `<stream>.sig` and `<stream>/` exist; the "signature" is the trimmed *contents of the `.sig` file*, compared (case-insensitively) against the SQL file's md5. Missing/unopenable schema → "Internal Error … (#1)"; mismatch error echoes both hashes. | Spec said "compute its md5 and compare against the stream's recorded signature" and "default: a single core stream" without the `streams.cfg`/`.sig` mechanism, the dual existence requirement, or the (#1) error. | Rewrote FS-060.7 stream bullets and BS-060-10 with the full mechanism. |
| 4 | FS-060.5 | MISSING | Med | Two distinct fallback error strings at two layers: the install routine sets "Missing or invalid data…"; the step dispatcher sets "Error installing osTicket - correct the errors below and try again." when `install()` returns false with no `err` key. | Spec documented only the inner "Missing or invalid data…" fallback. | Added the outer dispatcher-level fallback to FS-060.5. |
| 5 | FS-060.1 bootstrap | MISSING | Low | `setup.inc.php` sets `error_reporting = E_ALL & ~E_NOTICE` then strips `E_STRICT` / `E_DEPRECATED|E_USER_DEPRECATED` when defined; sets `session.use_trans_sid=0`, `session.cache_limiter=nocache`; conditionally loads `mysqli.php` vs `mysql.php`; pre-loads validator/passwd/format/misc. `index.php` just requires `install.php`. | Spec listed display_errors/magic_quotes/register_globals/URL/SETUPINC only. | Expanded FS-060.1 bootstrap ACs with error_reporting, session ini, driver load, helper preloads, and the index shim. |
| 6 | FS-060.1 / EC-060-12 | IMPRECISE | Low | `URL` = scheme + HTTP_HOST + dirname(PHP_SELF), then `rtrim(..., 'setup')` (literal char-set strip, not a suffix strip) — can over-trim dirs ending in those chars. | Spec said "from the request host + script directory" without the trailing-`setup` rtrim or its char-set quirk. | Documented exact URL derivation in FS-060.1 + new EC-060-12 for the rtrim quirk. |
| 7 | BS-060-02 / EC-060-13 | IMPRECISE | Low | Two divergent config templates: `include/ost-sampleconfig.php` guards on `INCLUDE_DIR` + carries SSL/`MAIL_EOL`/`ROOT_PATH` commented blocks; `setup/inc/ost-sampleconfig.php` guards on `ROOT_PATH` + omits them. Redirect target is `ROOT_PATH+'setup/install.php'`; missing-installer die is "Error: Contact system admin.". Direct-access guard also fires when bootstrap constant undefined. | Spec treated the two paths as equivalent; redirect/die strings + guard condition imprecise. | Rewrote BS-060-02 with both variants, exact guard, redirect target, and die string; added EC-060-13. |
| 8 | KL-060-09 | MISSING | Low | MySQL min-version check compares `explode('.', ...)` arrays, not a semantic version compare; ignores components past the first two. | Coarse comparison noted only loosely in KL-060-01. | Added detail to FS-060.6 + new KL-060-09. |
| 9 | KL-060-10 | MISSING | Low | Schema-signature hash algorithm is hard-coded `md5` despite an in-code TODO to make it per-stream configurable. | Not called out as a limitation. | New KL-060-10. |
| 10 | EC-060-14 / KL-060-11 | MISSING | Low | Welcome ticket uses hard-coded sentinels (`priority_id=0`, `topic_id=0`, source `Web`, email `support@osticket.com`, name `osTicket Support`, random 6-digit ticketID) and is written via raw INSERT, bypassing routing/SLA/autoresponse/filters/events. | BS-060-18 noted it is best-effort but not the sentinel values nor pipeline bypass. | New EC-060-14 + KL-060-11. |
| 11 | BS-060-03 | IMPRECISE | Low | `file-unclean` also selected when config is unreadable/empty; `clearstatcache()` called before `file-perm` so a just-applied chmod is re-read. | Spec listed the three downgrade views but not the empty/unreadable case or clearstatcache. | Added both to BS-060-03. |

## Confirmed-accurate (no change needed)
- Field validation rules: password ≥5 chars, username ≥2 chars + `[\p{L}\d._-]`,
  reserved-name list `admin/admins/username/osticket` (case-insensitive, no trim) —
  all verbatim in `class.validator.php`; EC-060-10 casing/whitespace bypass holds.
- Seed-data claims (Default SLA grace_period=48, departments Support/Billing,
  `default_priority_id=2`) spot-checked against `install-mysql.sql` — accurate.
- BS-060-04..09, BS-060-11..18 and existing EC/KL otherwise match the source.

## Totals
- Gaps found: **11** (MISSING 6, IMPRECISE 4, INCORRECT 1).
- All 11 fixed in `specs/FS-060-installer-setup-wizard.md`.
- New rule artifacts added: **0 new FR** (FS numbers unchanged; existing FRs
  expanded in place), **0 new BS** (BS-060-02/03/10 expanded in place),
  **3 new EC** (EC-060-12, EC-060-13, EC-060-14), **3 new KL** (KL-060-09,
  KL-060-10, KL-060-11).
