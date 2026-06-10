# FS-003: Cryptography, Validation & Formatting Infrastructure

> **Band:** FS-00X — shared shell, bootstrap, session/auth, CSRF/crypto.
> **Source files:** `include/class.crypto.php`, `include/class.validator.php`, `include/class.format.php`, `include/class.charset.php`, `include/class.misc.php`, `include/class.signal.php`, `include/class.http.php`, `include/class.error.php`, `include/class.log.php`, `include/class.timezone.php`, `include/mysql.php`, `include/mysqli.php` (procedural data-access helper layer).
> **Reference-data note:** Reusable enum sets and the `syslog`/`timezone` table schemas are owned canonically by **FS-091 (Reference Data, Enums & Data Model)**; this spec cross-references them rather than restating them.

---

## Overview

This specification documents the **shared, cross-cutting utility layer** that nearly every other domain in osTicket depends on. It is not a user-facing feature; it is the set of reusable building blocks that perform:

- **Cryptography** — reversible encryption/decryption of secrets, keyed hashing, and cryptographically-strong random byte generation, with automatic backend (library) selection and self-describing ciphertext tags so encrypted values can be decrypted (and silently upgraded) later regardless of which library is currently installed.
- **Input validation** — declarative field-set validation plus standalone validators for email, phone/fax, URL, IP (v4/v6), username, password, ZIP code, and numeric/date/array types, each with exact length/format constraints.
- **Formatting & sanitization** — HTML sanitization and tag stripping, HTML entity encode/decode, MIME-header decoding, RFC 5987 filename decoding, phone-number display formatting, file-size humanization, text truncation/word-wrapping, URL-clickability rewriting, elapsed-time formatting, slug generation, and timezone-aware date rendering.
- **Character-set normalization & transcoding** — cleaning up bogus/ambiguous charset labels and converting text between encodings (notably to UTF-8) for inbound email handling.
- **Miscellaneous helpers** — random alphanumeric/numeric code generation, timezone/GMT/DB time conversions, current-URL reconstruction, and a time-of-day dropdown generator.
- **Signal (publish/subscribe) hooks** — a lightweight, string-named event bus used to decouple side-effects across the system.
- **HTTP response helpers** — verbose status-line generation, raw responses, redirects, and file-download header emission.
- **Error & logging primitives** — a formal error object that auto-logs on construction, plus read-model accessors for the system log (`syslog`) and the timezone reference table.
- **Procedural data-access helper layer** — the system-wide `db_*` helper API (connection lifecycle, query execution, parameterized/"smart" queries, result-set accessors, input escaping/SQL-injection defense, magic-quotes output reversal, session-variable read/write) that every other domain calls to reach the relational store. The same contract is implemented by two interchangeable driver back-ends (`include/mysql.php`, the legacy `mysql_*` driver, and `include/mysqli.php`, the `mysqli`-object driver); a build wires in exactly one. The bootstrap-owned subset (`db_connect`, `db_query`, `db_version`, `db_create_database`) is documented in FS-001/FS-060; the remainder of the helper API is specified here.

The behaviors described here are **literal extractions** of the existing implementation, including exact limits, regular-expression intents, message strings, tag formats, and fallback ordering.

---

## Functional Requirements

### FS-003.1 — Two-key reversible encryption

**Description:** The system shall encrypt clear-text input under a **master key** combined with a **sub-key** (namespace, default `"encryption"`), producing a self-describing, base64-wrapped, tagged ciphertext string. It shall decrypt such strings back to the original clear text using the same master key and sub-key.

**Acceptance criteria:**
- `encrypt(input, key, skey='encryption', crypt=null)` selects a crypto backend (FS-003.4), sets the master/sub keys on it, performs low-level encryption, and returns a string of the form `"$<tagNumber>$<base64(ciphertext)>"`. Returns `false` if no backend is available or low-level encryption fails.
- `decrypt(ciphertext, key, skey='encryption')` returns `false` immediately if `key` is empty, `ciphertext` is empty, or the first character of `ciphertext` is not `$`.
- Decryption splits the leading `$<tag>$` prefix off, looks up the backend by the embedded tag number, base64-decodes the remainder, and returns the recovered clear text. Returns `false` if the tag is missing, the backend for that tag is unavailable, or the backend does not currently `exists()`.
- The same master key may be reused across the system with different sub-keys so that two encrypted values share a master key but occupy different cryptographic namespaces (data encrypted under one namespace is not recoverable under another).

### FS-003.2 — Keyed hash

**Description:** The system shall produce a keyed cryptographic hash of an arbitrary string.

**Acceptance criteria:**
- `hash(string, key)` returns the SHA-512 hash of `string` computed under `key` (a keyed/HMAC-style hash).
- The hash routine is the basis for deriving per-message encryption keys (FS-003.3, `getKeyHash`).

### FS-003.3 — Subkey-derived per-message encryption key

**Description:** Each low-level encryption shall derive its actual binary key from the master key, the sub-key, and a per-message seed (the IV/salt), so that no two messages are encrypted with the identical key even under a shared master key.

**Acceptance criteria:**
- `getKeyHash(seed, len=32)` computes `hash( masterKey . md5(subKey), seed )` and returns the first `len` bytes of the result as the binary encryption key.
- The seed passed in is the per-message initialization vector (IV) or salt; this guarantees the derived key varies per message.
- Although the default `len` is **32**, the cipher backends always invoke `getKeyHash` with the **IV length** as the requested length (mcrypt passes the module's IV size, OpenSSL passes the method's IV length, phpseclib passes the fixed 16). For AES-128 the resulting effective binary key is therefore the **IV-length-sized prefix** (16 bytes), not the 32-byte default (BS-004).

### FS-003.4 — Automatic crypto backend selection & tagging

**Description:** The system shall automatically select the best available low-level encryption library based on installed runtime extensions, and shall tag every ciphertext with a stable numeric identifier so it can later be decrypted by the matching backend even after the preferred backend changes.

**Acceptance criteria:**
- Three backends are defined with stable top-level tag numbers: **mcrypt = 1**, **OpenSSL = 2**, **phpseclib = 3**.
- The available-backends list is built once (cached) and populated only with backends whose implementing class is present; selection order when no specific backend is requested is **OpenSSL, then mcrypt, then phpseclib**, returning the first whose `exists()` reports its underlying library is loaded.
- When a specific tag is requested, that exact backend is returned (or `null` if not registered).
- Each backend self-reports availability via `exists()`: OpenSSL requires the `openssl` extension and `openssl_cipher_iv_length`; mcrypt requires the `mcrypt` extension and `mcrypt_module_open`; phpseclib requires the `Crypt_AES` class to exist (always available as bundled pure-PHP fallback).
- Each backend internally tags its output with a per-backend **cipher id** so the exact algorithm/mode can be recovered on decrypt (see BS-003).
- Within a backend, cipher selection runs through a shared `getCipher(cid, callback)` helper: when a `cid` is supplied the matching cipher row is taken directly; when no `cid` is supplied the helper scans the backend's registered ciphers in registration order and selects the **first** for which the backend-specific `_checkCipher` predicate passes (the predicate confirms the underlying cipher/mode/method is actually usable in the current runtime). The selected cipher row is returned with its resolved `cid` merged in; if none qualifies, `null` is returned and encryption fails.
- **Per-backend `_checkCipher` predicates:** mcrypt requires a non-empty cipher name + mode, that the backend `exists()`, and that `mcrypt_module_open` succeeds for that name/mode; OpenSSL requires a method, that the backend `exists()`, and a non-zero `openssl_cipher_iv_length` for the method; phpseclib requires a mode, a non-zero `ivlen`, and that the named cipher class (`Crypt_AES`) exists.

### FS-003.5 — Backend cipher behavior (AES-128)

**Description:** Each backend shall encrypt/decrypt using AES-128 with a per-message random IV prepended to the ciphertext.

**Acceptance criteria:**
- **mcrypt** uses Rijndael-128 in CBC mode; it pads the plaintext to the block size using PKCS-style padding (each pad byte equals the pad length), prepends the IV, and on decrypt strips the trailing pad bytes.
- **OpenSSL** uses method `aes-128-cbc`; it generates a random IV of the cipher's IV length, prepends it to the raw ciphertext, and decrypts by splitting the IV back off.
- **phpseclib** uses `Crypt_AES` in CBC mode with a fixed **16-byte IV** (the IV length is documented as not changeable without breaking stored data).
- Each backend output is `"$<cipherId>$<IV><ciphertext>"`; the top-level wrapper (FS-003.1) base64-encodes this and prepends the backend tag.
- **OpenSSL** sources its IV specifically from `openssl_random_pseudo_bytes` (not the general `Crypto::random` chain), while mcrypt and phpseclib source their IV from `Crypto::random` (FS-003.6). All three pass the `OPENSSL_RAW_DATA` option (or boolean `true` when that constant is undefined) so the OpenSSL ciphertext is raw bytes, not base64.
- **Backend decrypt input guards (per backend, independent of the top-level guard):** each backend's `decrypt` returns `false` unless the inner string is non-empty and begins with `$`; it then splits off the inner `$<cid>$` prefix and returns `false` when the cid is missing, the post-prefix ciphertext is empty, or the cipher for that cid cannot be resolved. **mcrypt additionally** returns `false` when the resolved cipher's `cid` does not equal the embedded `cid`, and returns `false` when stripping the IV leaves an empty remainder (EC-016).

### FS-003.6 — Cryptographically-strong random bytes

**Description:** The system shall generate random byte strings of a requested length using the strongest source available, with a deterministic fallback chain.

**Acceptance criteria:**
- `random(len)` returns `len` bytes.
- On Windows-class hosts the source order is: `openssl_random_pseudo_bytes` (when available and runtime ≥ 5.3.4), then `mcrypt_create_iv` with the device-random source (when available and runtime ≥ 5.3.7).
- On non-Windows hosts the source order is: `openssl_random_pseudo_bytes`, then reading from `/dev/urandom` (the file handle is opened once and **cached statically** for reuse across calls within the request), then `mcrypt_create_iv` with the `MCRYPT_DEV_URANDOM` device source. (Unlike Windows, the non-Windows OpenSSL branch is **not** version-gated.)
- If no system source is available, a software fallback seeds AES-CTR from a mix of session id, microtime, and process id and streams encrypted counter values until `len` bytes are produced (a last-resort generator, explicitly weaker than OS entropy — see KL-002).

### FS-003.7 — Declarative field-set validation

**Description:** The system shall validate an associative array of input values against a declarative field specification, accumulating a per-field error map.

**Acceptance criteria:**
- A validator is constructed with a `fields` specification; each field declares at minimum `type`, `required` (boolean), `error` (the message used when that field fails), and optionally `min` (for integers).
- `validate(source, userinput=true)` resets errors, then errors out early with `"Invalid input"` if `source` is not an array or `"No fields set up"` if no fields are configured.
- When `userinput` is true and the runtime reports magic-quotes is on, the source is run through slash-stripping (FS-003.13) before validation.
- For each field: if the field is **not required and the value is falsy** (the empty/zero/unset test is value-falsiness, not `isset`), it is skipped; if the field is **required and the value is missing/empty** (empty is tolerated only for `int` type), the field's `error` message is recorded and the field is skipped.
- The required-vs-missing test is the literal condition `(required && !isset(value)) || (!value && type != 'int')`. Because the boolean operators are not parenthesized, the **second clause `(!value && type != 'int')` fires regardless of whether the field was declared required** — i.e. an empty/falsy value of any non-`int` type records the field's error even when `required` is false (EC-017). Only the explicit not-required-and-falsy skip above (evaluated first) prevents this, so the error path is reachable only for non-`int` fields whose value is falsy yet not caught by the earlier skip (e.g. the literal string `"0"`, which is falsy in PHP).
- The validator returns `true` only when the accumulated error map is empty; otherwise `false`. `iserror()` and `errors()` expose the state. A static `process(fields, vars, errors)` helper merges any failures into a caller-supplied error array and returns whether the merged set is empty.

### FS-003.8 — Per-type validation rules

**Description:** The system shall apply type-specific validation per field according to the declared `type` (case-insensitive).

**Acceptance criteria (exact rules per type):**
- `integer` / `int` — value must be numeric; if a `min` is declared, value must be ≥ `min`.
- `double` — value must be numeric.
- `text` / `string` — value must be a string.
- `array` — value must be a non-empty array.
- `radio` — the key must be set in the input.
- `date` — the value must parse as a date (interpretable timestamp); otherwise it fails.
- `time` — currently accepted unconditionally (no constraint enforced; see KL-003).
- `phone` / `fax` — must pass the phone check (FS-003.9, BS-010).
- `email` — must pass the email check (FS-003.9, BS-009).
- `url` — must pass the URL check (FS-003.9).
- `password` — minimum length **5 characters**; on failure the field error is suffixed with `" (5 chars min)"`.
- `username` — must pass the username check (FS-003.9, BS-013); on failure the field error is suffixed with `": <reason>"`.
- `zipcode` — must be numeric **and** exactly **5 characters** long.
- Any unrecognized/unset type fails with the field error suffixed by `" (type not set)"`.

### FS-003.9 — Standalone format validators

**Description:** The system shall expose validators callable without a validator instance for the common data types.

**Acceptance criteria:**
- `is_email(email)` — matches a single email against a local-part/domain pattern (BS-009). Domain TLD must be at least 2 alphanumeric characters; local part permits an extended set of symbols.
- `is_phone(phone)` — strips parentheses, hyphens, dots, plus signs, and spaces, then requires the remainder to be numeric and **7–16 digits** inclusive (BS-010). It validates shape/length only, not real-world routability.
- `is_url(url)` — passes when the value is non-empty and yields a parseable URL with a non-empty host component.
- `is_ip(ip)` — trims the value; when the runtime supports binary IP parsing, accepts any value that parses as a valid IPv4 **or** IPv6 address; otherwise falls back to a combined IPv4/IPv6 regular expression (BS-011). Empty values are rejected.
- `is_username(username, &error)` — requires **at least 2 characters** and that the value match only Unicode letters, digits, dot, underscore, and hyphen; sets `error` to `"At least two (2) characters"` or `"Username contains invalid characters"` respectively, and returns success only when `error` is empty (BS-013).

### FS-003.10 — HTML sanitization & safe HTML

**Description:** The system shall neutralize untrusted HTML so it can be safely stored/displayed, balancing tags and removing dangerous constructs.

**Acceptance criteria:**
- `html(html, config={'balance':1})` runs the HTML through the bundled HTML-sanitizing library with the supplied configuration (default config balances/closes unclosed tags).
- `safe_html(html)` enforces the config `{safe:1, balance:1, comment:1}` — excludes `applet`, `embed`, `iframe`, `object`, and `script` tags; balances/closes unclosed tags; and strips HTML comments.
- `sanitize(text, striptags=true)` first runs `safe_html`, then — when `striptags` is true — strips remaining tags **with decoding disabled** (BS-014). This is the path used for log titles/messages before persistence.

### FS-003.11 — HTML entity encode/decode & tag stripping

**Description:** The system shall provide consistent HTML entity encoding/decoding and tag stripping, recursing over arrays.

**Acceptance criteria:**
- `htmlencode(var)` / `htmlchars(var)` / `input(var)` — encode HTML entities in UTF-8 using `ENT_COMPAT | ENT_QUOTES` (plus `ENT_HTML401` on runtime ≥ 5.4); arrays are encoded element-wise.
- `htmldecode(var)` — reverses entity encoding in UTF-8 using `ENT_COMPAT` (plus `ENT_HTML401` on runtime ≥ 5.4); arrays handled element-wise.
- `striptags(var, decode=true)` — removes all tags; when `decode` is true the value is HTML-decoded first; arrays handled element-wise.

### FS-003.12 — Display formatting of free text

**Description:** The system shall transform stored free text into display-safe HTML.

**Acceptance criteria:**
- `display(text)` — when the "clickable URLs" config is enabled and text is present, rewrites links (FS-003.14); hard-wraps any run of **75+ word characters** at 70 characters; converts newlines to `<br>`.
- `truncate(string, len, hard=false)` — returns the string unchanged if `len` is falsy or longer than the string; otherwise cuts to `len` characters and, unless `hard` is set, trims back to the last space and appends `" ..."`. When the cut prefix contains **no space**, the soft path trims to position 0 (the no-space `strrpos` result), yielding just `" ..."` (the visible text is lost) (EC-019).
- `wrap(text, len=75)` — word-wraps the text at the given column with hard cutting.
- `stripEmptyLines(string)` — collapses **3 or more consecutive newlines** down to exactly 2.

### FS-003.13 — Slash stripping (magic-quotes normalization)

**Description:** The system shall recursively strip backslashes added by legacy magic-quotes runtimes.

**Acceptance criteria:**
- `strip_slashes(var)` strips slashes from a string, recursing into arrays element-wise. It is invoked automatically by the validator (FS-003.7) when magic-quotes is detected.

### FS-003.14 — Clickable-URL rewriting

**Description:** The system shall rewrite plain URLs, `www.` links, and email addresses in text into HTML anchors for display.

**Acceptance criteria:**
- `http(s)://…` links and `www.…` links are wrapped as anchors pointing at an internal redirect endpoint that carries the original URL plus an authentication token obtained from the global context (`getLinkToken()`), opening in a new window.
- Bare email addresses are wrapped as `mailto:` anchors. The email pattern used here requires a TLD of **2–4 characters** (`[a-z]{2,4}`), which is narrower than the standalone `is_email` validator's `≥ 2`-character TLD (BS-009) — so some addresses that validate via `is_email` are not auto-linked here (EC-018).
- The `http(s)://` and `www.` rewrites point the anchor `href` at the internal `l.php?url=<urlencoded-url>&auth=<token>` redirect endpoint with `target="_blank"`.
- This logic depends on the global app context for the link token (cross-reference FS-001 for `getLinkToken`).

### FS-003.15 — File-size, phone, slug, elapsed-time & array formatting

**Description:** The system shall provide assorted display formatters.

**Acceptance criteria:**
- `file_size(bytes)` — non-numeric input is returned unchanged; `< 1024` → `"<n> bytes"`; `< 102400` → kilobytes (÷ **1024**) rounded to 1 decimal with `" kb"`; otherwise megabytes (÷ **1024000**) rounded to 1 decimal with `" mb"` (BS-016). Note the divisors are inconsistent: kb uses the binary 1024 while mb uses 1024000 (= 1000 × 1024), so the "mb" figure is slightly larger than a true binary mebibyte (KL-010).
- `phone(phone)` — strips non-digits; **7 digits** → `"NNN-NNNN"`; **10 digits** → `"(NNN) NNN-NNNN"`; any other length → the original input unchanged (BS-017).
- `slugify(text)` — replaces runs of non-letter/non-digit (Unicode-aware) characters with `-`, trims leading/trailing `-`, lowercases, and returns `"n-a"` when the result is empty.
- `elapsedTime(sec)` — returns `""` for empty/non-numeric input; otherwise renders days/hours/minutes as `"<d>d,<h>h,<m>m"`, omitting the day and hour segments when zero (minutes always shown). The day/hour/minute decomposition relies on the **bcmath** extension (`bcmod`); on a runtime without bcmath this helper fails (KL-011).
- `array_implode(glue, separator, array)` — joins key/value pairs with `glue` between key and value and `separator` between pairs; nested arrays are comma-joined; non-array input is returned unchanged.

### FS-003.16 — MIME-header & RFC 5987 decoding

**Description:** The system shall decode encoded email header text and content-disposition filenames into a target encoding (default UTF-8).

**Acceptance criteria:**
- `mimedecode(text, encoding='UTF-8')` — decodes MIME-encoded header words in priority order: (1) when `imap_mime_header_decode` is available and returns parts, each part is transcoded from its declared charset to the target encoding via `Charset::transcode` and concatenated; else (2) when the text's **first character is `=`** and `iconv_mime_decode` is available, that is used with the target encoding; else (3) when the target is `utf-8` (case-insensitive) and `imap_utf8` is available, that is used. If none of the branches apply, the original text is returned unchanged.
- `decodeRfc5987(filename)` — parses the `charset'language'urlencoded-name` form of RFC 5987, URL-decodes the name, and transcodes it to UTF-8 from the declared charset; the language sub-component is ignored. Non-matching input is returned unchanged.

### FS-003.17 — Charset normalization & transcoding

**Description:** The system shall normalize bogus/ambiguous charset labels and transcode text between encodings, defaulting toward UTF-8.

**Acceptance criteria:**
- `Charset::normalize(charset)` maps charset labels per BS-018: `Windows-?<n>` → `"Windows-<n>"`; `ks_c_5601-1987…` (Korean) → `"cp949"`; any of `default`, `x-user-defined`, `iso`, `us-ascii`, or empty/whitespace → `"ISO-8859-1"`; otherwise the input is returned unchanged.
- `Charset::transcode(text, from, to)` — when `from` is empty and `mb_detect_encoding` is available, auto-detects the source encoding; trims and normalizes both `from` and `to` (BS-018); then converts via the **first available** of: (1) `iconv($from, $to.'//IGNORE', text)`; (2) `mb_convert_encoding(text, to, from)`; (3) **only when the normalized target is `utf-8` AND the normalized source is `ISO-8859-1`**, `utf8_encode(text)`. If none of those functions is available the text is left unconverted. If conversion yields an empty/false result but the original text was non-empty, the **original text is returned** (assume 8-bit latin-1) rather than losing data (BS-019).
- `Charset::utf8(text, charset=null)` — convenience wrapper transcoding to UTF-8 (`'utf-8'`).

### FS-003.18 — Random code generation

**Description:** The system shall generate random alphanumeric strings and random numeric ranges for identifiers and tokens.

**Acceptance criteria:**
- `Misc::randCode(len=8, chars=false)` — draws from the supplied character set or the default `a–z A–Z 0–9` (62 chars), sizing the random byte request from the bit-width of the character set (`ceil(log2(charCount))` bits per character, rounded up to a 4-byte boundary), sourcing entropy from the crypto random generator (FS-003.6), and returning a string of exactly `len` characters. The random bytes are consumed as little-endian 32-bit unsigned integers (`unpack('V*')`), each yielding `floor(32 / bitsPerChar)` characters by masking off `bitsPerChar` low bits at a time and reducing modulo the character-set size. Because of the modulo reduction the character distribution is **not perfectly uniform** when the set size is not a power of two (KL-012).
- `Misc::randNumber(len=6, start=false, end=false)` — returns a random integer in a `len`-digit range. When `len` is non-zero (the normal case) the range bounds are derived by right-padding: `start = "1"` right-padded with `0` to `len` digits (e.g. `len=6` → `100000`) and `end = "9"` right-padded with `9` to `len` digits (e.g. `999999`); the supplied `start`/`end` arguments are **ignored**. The explicit `start`/`end` arguments are honored **only when `len` is falsy (0)**; in that case the supplied `start`/`end` bound the range directly. Used to seed ticket-number generation. The draw uses the non-cryptographic `mt_rand` PRNG, not the crypto random source.
- `Misc::__rand_seed(value=0)` — seeds the (non-cryptographic) `mt_rand` PRNG by forming a 32-bit seed: the lower 16 bits from `(microtime × 1,000,000) mod 65535`, the upper 16 bits from `value mod 65535`.

### FS-003.19 — Time conversion helpers

**Description:** The system shall convert between database time, GMT, and user-local time using the configured DB timezone offset and the session timezone offset/DST flag.

**Acceptance criteria:**
- `Misc::gmtime()` — returns the current GMT timestamp computed as `time()` minus the **server's local UTC offset** (`date('Z')`, the local timezone offset in seconds), not a configured offset.
- `Misc::db2gmtime(var)` — converts a DB timestamp (string or int) to GMT by subtracting the configured **DB timezone offset** (hours). Returns **nothing (null)** when `var` is empty/falsy (does not default to current time) — only `dbtime` defaults an empty input to the current GMT time (EC-020).
- `Misc::dbtime(var=null)` — converts a user-local or GMT time to DB time: a null/empty value uses current GMT; otherwise the value is shifted from user-local to GMT using the session `TZ_OFFSET` (plus DST when `TZ_DST` is set), then shifted to DB time by the DB timezone offset.
- `Format::date(format, gmtimestamp, offset=0, daylight=false)` — returns `""` for empty/non-numeric input; otherwise applies the offset (plus DST adjustment when `daylight` is set) and formats; `Format::userdate(format, gmtime)` applies the session `TZ_OFFSET`/`TZ_DST`.
- `Format::db_date`, `db_datetime`, `db_daydatetime` — render a DB time using the configured date / date-time / day-date-time formats respectively, first converting via `db2gmtime` then via `userdate` (cross-reference FS-001/FS-032 for the format config keys).

### FS-003.20 — Current-URL reconstruction & time dropdown

**Description:** The system shall reconstruct the absolute URL of the current request and render a time-of-day selection control.

**Acceptance criteria:**
- `Misc::currentURL()` — builds `http`/`https` (based on the HTTPS server flag), host, non-standard port (anything other than 80), and request URI into an absolute URL, reconstructing the request URI from `PHP_SELF` + query string when the server does not provide it (IIS compatibility).
- `Misc::timeDropdown(hr, min, name='time')` — emits an HTML `<select>` of times in **15-minute increments** across a 24-hour day; hour is normalized modulo 24 (negative → 0); minute is normalized down to the nearest of `0/15/30/45`; the matching option is pre-selected; a blank `"Time"` option is the default.

### FS-003.21 — Signal (publish/subscribe) hooks

**Description:** The system shall provide a lightweight, string-named publish/subscribe mechanism so components can decouple side-effects without compile-time wiring.

**Acceptance criteria:**
- `Signal::connect(signal, callable, object=null, check=null)` — registers a subscriber under a string signal name. Subscribers are stored as `(object-class-filter, callable, predicate)` tuples in registration order.
- If `object` is provided it must be a class name (string); otherwise a runtime warning `"Invalid object: <x>: Expected class"` is raised. If `check` is provided it must be callable; otherwise a runtime warning `"Invalid check function: Must be callable"` is raised and the predicate is discarded.
- `Signal::send(signal, object, &data=null)` — invokes every subscriber of the named signal **in registration order**, passing `(object, data)`. The signal name need not be pre-registered; sending an unknown signal is a no-op.
- A subscriber is skipped when an `object`-class filter is set and the sending object is not an instance of that class, or when the predicate returns false for the `data`.
- Subscribers **cannot interrupt or halt delivery** to later subscribers; the `data` argument is passed by reference so subscribers may mutate it and propagate changes back to the originating context.

### FS-003.22 — HTTP response helpers

**Description:** The system shall provide helpers to emit HTTP status lines, full responses, redirects, and file downloads.

**Acceptance criteria:**
- `Http::header_code_verbose(code)` maps status codes to verbose status-line text per BS-020 (e.g. `200 OK`, `201 Created`, `204 NoContent`, `400 Bad Request`, `401 Unauthorized`, `403 Forbidden`, `404 Not Found`, `405 Method Not Allowed`, `416 Requested Range Not Satisfiable`); any unmapped code yields `500 Internal Server Error`.
- `Http::response(code, content, contentType='text/html', charset='UTF-8')` — emits the HTTP status line, a `Status` header, `Connection: Close`, the content type with charset, a `Content-Length`, prints the content, and terminates the request.
- `Http::redirect(url, delay=0, msg='')` — emits a `Location` redirect; for IIS versions earlier than 7.0 it instead emits a `Refresh` header with the delay (legacy IIS compatibility), then terminates.
- `Http::download(filename, type, data=null)` — emits no-cache headers, the content type, a `Content-Disposition` (the older Internet Explorer-on-Windows variant omits `attachment;`), binary transfer encoding, and — when `data` is provided — `Content-Length`, the bytes, and termination.

### FS-003.23 — Formal error object with auto-logging

**Description:** The system shall provide a formal error object that, on construction, normalizes its message and writes it to the system log; this stands in for exception throwing in legacy-runtime contexts.

**Acceptance criteria:**
- Constructing an `Error(message)` replaces the application root path in the message with `(root)/`, and — when the configured log level is **3 (Debug)** — appends a formatted backtrace (each frame as `#<i> <class><type><function> at [<file>:<line>]`).
- On construction the error logs itself via the global app context at error level, using its title `"<ClassName>: <title>"`.
- `InitialDataError` is a specialization with title `"Problem with install initial data"`.
- `raise_error(message, class=false)` instantiates an error of the named class (default `Error`), thereby logging it.

### FS-003.24 — System log read model

**Description:** The system shall provide a read-model accessor for individual persisted system-log entries.

**Acceptance criteria:**
- `Log::lookup(id)` returns a populated `Log` object only when `id` is numeric and the loaded entry's id matches the requested id; otherwise `null`.
- A loaded log exposes: id, type (`log_type` — one of `Error`/`Warning`/`Debug`), title, text (`log`), originating IP address, and creation date.
- The persisted log row is read from the system-log table (schema owned by **FS-091**, `ost_syslog`); the `log_type` enum value-list (`Error`/`Warning`/`Debug`) is owned by **FS-091.8**.
- **Cross-reference (write path):** log entries are *written* by the global app context's `log()` family, not by this read model. The three-level `log_type` enum value-list (Error/Warning/Debug) is owned by **FS-091.8**; the write-side collapse of syslog priorities into those levels, drop-below-configured-level filtering, sanitize-and-strip-before-insert, admin alerting, and grace-period purging are owned by **FS-001.12** (cross-reference FS-032/FS-033 for the log-level/grace-period config and the staff log viewer).

### FS-003.25 — Timezone reference read model

**Description:** The system shall provide a read-model accessor for timezone reference entries used in time conversion.

**Acceptance criteria:**
- `Timezone::lookup(id)` returns a populated `Timezone` object only when `id` is numeric and the loaded entry's id matches; otherwise `null`.
- `Timezone::getOffsetById(id)` returns the entry's UTC offset (hours) or `0` when no matching entry exists.
- A loaded timezone exposes its id and offset; rows are read from the timezone reference table (schema owned by **FS-091**, `ost_timezone`).
- The loaded row is stored on the object's data slot (`ht`); `getOffset` reads from it correctly, but `getName`/`getDesc` read from a different, never-populated slot (`info`) and therefore always return an undefined/empty value — a latent bug, so the timezone **name is not reliably retrievable** through this read model (KL-013).

### FS-003.26 — SQL input escaping (`db_input` / `db_real_escape`)

**Description:** The system shall expose a single escaping primitive through which **every** value embedded in a SQL string is made injection-safe, with a numeric-passthrough fast path; this is the system-wide SQL-injection defense.

**Acceptance criteria:**
- `db_input(var, quote=true)` is the public entry point. Behavior by input shape:
  - **Array** — recurses element-wise (`db_input` applied to each element, carrying the same `quote` flag), returning an array of escaped values.
  - **Numeric literal** — when `var` is truthy and matches a numeric pattern, the value is returned **verbatim, unescaped and unquoted** (so it can be embedded as a bare numeric literal). The two driver back-ends use slightly different numeric patterns: the legacy driver matches `^\d+(\.\d+)?$` (any unsigned integer or decimal); the `mysqli` driver matches `^(?:\d+\.\d+|[1-9]\d*)$` (a decimal, or an integer with no leading zero), so an integer with a leading zero (e.g. `"007"`) is treated as non-numeric and escaped/quoted by the `mysqli` driver but passed through by the legacy driver (BS-026, EC-021).
  - **All other values** — delegated to `db_real_escape(var, quote)`.
- `db_real_escape(val, quote=false)` performs the driver-level string escape (legacy: `mysql_real_escape_string`; `mysqli`: the connection object's `real_escape_string`) and, when `quote` is true, wraps the escaped result in single quotes (`'…'`). It is documented as **"Do not call this function directly…use db_input"** — callers go through `db_input` so the numeric fast path and default quoting apply.
- Because `db_input` defaults `quote=true`, a non-numeric value embedded via `db_input` arrives pre-quoted (no caller-supplied surrounding quotes needed); a numeric value arrives bare. Callers that need an escaped-but-unquoted fragment pass `quote=false`.
- `db_real_escape` is also used directly by the "smart query" helper (FS-003.27) to escape each positional argument before `sprintf` substitution.

### FS-003.27 — Query execution & "smart" parameterized query

**Description:** The system shall execute SQL through a thin wrapper that centralizes error logging, and shall additionally provide a positional-placeholder "smart" query helper.

**Acceptance criteria:**
- `db_query(query, logError=true)` runs the query against the active connection and returns the driver result handle (or false on failure). On failure, when `logError` is set and the global system object exists, it logs a database error titled `DB Error #<errno>` whose body is `[<query>]` plus the driver error text (cross-reference FS-001 for the bootstrap-owned connection and the `logDBError` write path). The `mysqli` driver additionally **retries up to 3 times** on deadlock error #1213 before giving up. (`db_query` itself is bootstrap-tagged FS-001; the surrounding helper family is specified here.)
- `db_squery(query, …)` — the "smart" query: it takes a query string with `?` positional placeholders followed by one argument per placeholder, replaces each `?` with `%s`, escapes **every** argument through `db_real_escape`, and assembles the final query via `sprintf` before delegating to `db_query`. (Note: because it escapes but does not numerically fast-path and substitutes through `%s`, every argument lands as an escaped string fragment; callers needing bare numerics build those into the query text themselves.)
- `db_count(query)` — convenience wrapper returning the first scalar of a count/aggregate query via `db_result(db_query(query))`.

### FS-003.28 — Result-set accessors & magic-quotes output reversal

**Description:** The system shall expose a uniform family of result-set accessor wrappers over the underlying driver, all routing scalar/row output through a magic-quotes reversal step so feature code sees clean values regardless of the legacy runtime.

**Acceptance criteria:**
- Scalar/row accessors (each returns null/0/false defensively when the result handle is falsy):
  - `db_result(res, row=0)` — the single scalar at the given row (legacy: `mysql_result`; `mysqli`: `data_seek(row)` then first column of `fetch_row`), passed through `db_output`.
  - `db_fetch_array(res, mode)` — the next row as an associative (default) or numeric/both array, passed through `db_output`.
  - `db_fetch_row(res)` — the next row as a numeric-indexed array, passed through `db_output`.
  - `db_fetch_field(res)` — the next field-metadata object (not output-filtered).
  - `db_assoc_array(res, mode)` — accumulates **all** remaining rows into an array of `db_fetch_array` rows (returns the accumulator, which is undefined/empty when there are zero rows — EC-022).
- Counts / cursor / identity accessors: `db_num_rows(res)` (rows in a result, 0 on falsy handle), `db_affected_rows()` (rows touched by the last write), `db_data_seek(res, n)` (move the row cursor to `n`), `db_data_reset(res)` (seek to row 0), `db_insert_id()` (auto-increment id of the last insert), `db_free_result(res)` (release the result handle).
- Error/metadata accessors: `db_error()` (last driver error text), `db_connect_error()` (connection error text), `db_errno()` (last driver error number), `db_field_type(res, col=0)` (the column's declared type / field metadata). These feed the `db_query` error-logging path and diagnostics.
- `db_output(var)` — **magic-quotes output reversal**: when the legacy `get_magic_quotes_runtime` directive is OFF (the modern case), the value is returned **unchanged**; only when it is ON does the helper recurse over arrays and `stripslashes` every **non-numeric** scalar (numeric scalars pass through). This compensates for runtimes that auto-added slashes to fetched data (BS-027). Because nearly every accessor above routes through `db_output`, the reversal is applied uniformly to all read paths.

### FS-003.29 — Connection lifecycle & MySQL session-variable read/write

**Description:** The system shall expose helpers to close the connection, select the active database, and read/write MySQL server session variables; one of those writes (forcing an empty `sql_mode`) is performed automatically on every connect.

**Acceptance criteria:**
- `db_close()` closes the active connection handle (legacy: `mysql_close`; `mysqli`: the connection object's `close`).
- `db_select_database(database)` selects the named database on the active connection, returning success only when a non-empty name was supplied and the driver-level select succeeded.
- `db_get_variable(variable, type='session')` reads a MySQL server variable via `SELECT @@<type>.<variable>` and returns its scalar value (e.g. `db_timezone()` is a convenience wrapper reading `@@session.time_zone`).
- `db_set_variable(variable, value, type='session')` writes a MySQL server variable via `SET <TYPE> <variable>=<db_input(value)>` (the value is escaped through `db_input`).
- **Connect-time `sql_mode` reset (side-effect):** on every successful connection, after setting the UTF-8 character set/collation, the connect routine calls `db_set_variable('sql_mode', '')`, forcing the session `sql_mode` to the **empty string**. This deliberately disables strict/traditional SQL modes (and other server-default modes) for the application's session so that osTicket's lax inserts (e.g. zero dates, implicit truncation, missing-default columns) do not raise errors on servers configured with a strict default `sql_mode` (BS-028, EC-023). (The `db_connect` routine itself is bootstrap-tagged FS-001; this requirement documents the session-variable side-effect it performs.)

### FS-003.30 — Dual driver back-ends, single contract

**Description:** The procedural `db_*` API shall be implemented by two interchangeable driver modules exposing the same function names, of which a build includes exactly one, so that all callers are driver-agnostic.

**Acceptance criteria:**
- `include/mysql.php` implements the contract over the legacy `mysql_*` extension; `include/mysqli.php` implements the identical contract over the `mysqli` object API. They define the **same** global function names, so feature code calls `db_input`/`db_query`/`db_fetch_array`/etc. without knowing which driver is active.
- Observable behavioral differences between the two implementations are limited and documented: the numeric-passthrough pattern in `db_input` (FS-003.26 / EC-021), the deadlock retry in `db_query` (FS-003.27, `mysqli` only), and the connection-error/SSL handling at connect time (FS-001). All other helper semantics are equivalent.
- The `mysqli` driver holds its connection handle in a module global (`$__db`) used by the accessors that need the connection object (e.g. `db_affected_rows`, `db_insert_id`, `db_real_escape`, `db_error`); the legacy driver relies on the extension's implicit "last connection" plus a `$dblink` global for close.

---

## Business Rules

- **BS-001 — Self-describing ciphertext format.** Every encrypted value is stored as `"$<backendTag>$<base64( $<cipherId>$<IV><rawCiphertext> )>"`. The leading `$<backendTag>$` selects the library (1=mcrypt, 2=OpenSSL, 3=phpseclib); the inner `$<cipherId>$` selects the algorithm/mode within that library; the IV is prepended to the raw ciphertext. This makes any stored value decryptable later regardless of the currently-preferred backend.

- **BS-002 — Backend preference order.** When no backend is explicitly requested, selection order is **OpenSSL → mcrypt → phpseclib**, choosing the first whose underlying library is loaded. phpseclib is the always-available pure-software fallback.

- **BS-003 — Cipher tags are immutable.** The cipher tag/algorithm definitions are marked "do not change or you lose your passwords"; the phpseclib IV length is fixed at 16 bytes for the same reason. Changing these values would render previously-encrypted data unrecoverable.

- **BS-004 — Per-message key derivation.** The actual binary encryption key is `firstBytes( SHA512_keyed( masterKey . md5(subKey), IV ), len )`. The IV/salt is the per-message seed, so the effective key differs per message even under a shared master key.

- **BS-005 — Sub-key namespacing.** A single master key may encrypt many kinds of data; the sub-key acts as a namespace so values encrypted in one namespace are not recoverable in another. Default sub-key is `"encryption"`.

- **BS-006 — Decrypt input guard.** Decryption is refused unless a key is present, the ciphertext is non-empty, and the ciphertext begins with `$`.

- **BS-007 — Random source priority.** Random bytes are sourced in strict priority order (OS/extension sources first), with a software AES-CTR generator only as a last resort.

- **BS-008 — Validation short-circuit per field.** Within a field, the first failing check records that field's single declared `error` message and stops evaluating that field; other fields continue. The overall result is failure if any field error exists.

- **BS-009 — Email format rule.** A valid email matches `local-part@domain` where the local part permits the extended symbol set `* + ! . & # $ | ' % / 0-9 a-z ^ _ \` { } = ? ~ : -` (case-insensitive), and the domain is one-or-more dot-separated alphanumeric/hyphen labels ending in a **TLD of ≥ 2 alphanumeric characters**.

- **BS-010 — Phone/fax rule.** After removing `( ) - . +` and spaces, the remainder must be **numeric and 7–16 characters** inclusive. This validates shape and length only — not whether the number is dialable.

- **BS-011 — IP rule.** An IP is valid if it parses (via binary IP parsing when available) as either IPv4 or IPv6; a combined IPv4/IPv6 regular expression is the fallback. Empty input is invalid.

- **BS-012 — Password minimum length.** Passwords must be **at least 5 characters**. (This is the field-validation floor used at input boundaries; staff account password policy/hashing is owned by FS-002.)

- **BS-013 — Username rule.** A username must be **at least 2 characters** and contain only Unicode letters, digits, `.`, `_`, and `-`. Violations yield `"At least two (2) characters"` or `"Username contains invalid characters"` respectively.

- **BS-014 — Log sanitization.** Text destined for the system log is run through safe-HTML sanitization; titles are additionally tag-stripped with decoding disabled before persistence.

- **BS-015 — Three-level logging.** Logging collapses to three stored levels (Error/Warning/Debug). The enum value-list is owned by **FS-091.8**; the collapse mapping plus drop-below-configured-level filtering are owned by **FS-001.12**.

- **BS-016 — File-size thresholds.** `< 1024 B` → bytes; `< 102400 B` → kb (÷1024, 1 decimal); otherwise mb (÷1,024,000, 1 decimal). Non-numeric input passes through unchanged.

- **BS-017 — Phone display formats.** Exactly **7 digits** → `NNN-NNNN`; exactly **10 digits** → `(NNN) NNN-NNNN`; any other digit count → original input unchanged.

- **BS-018 — Charset normalization map.** `Windows-?<n>` → `Windows-<n>`; `ks_c_5601-1987*` → `cp949`; `default` / `x-user-defined` / `iso` / `us-ascii` / empty → `ISO-8859-1`; all others unchanged.

- **BS-019 — Transcode is lossless-on-failure.** If transcoding produces an empty/false result while the original text was non-empty, the original text is returned (assumed latin-1/8-bit) rather than discarding content.

- **BS-020 — HTTP status verbosity map.** Recognized codes: `200 OK`, `201 Created`, `204 NoContent`, `400 Bad Request`, `401 Unauthorized`, `403 Forbidden`, `404 Not Found`, `405 Method Not Allowed`, `416 Requested Range Not Satisfiable`; everything else → `500 Internal Server Error`.

- **BS-021 — Signal delivery is ordered & uninterruptible.** Subscribers fire in registration order; no subscriber can stop delivery to others; `data` is passed by reference so subscribers may mutate it. Object-class filters and predicate checks may skip individual subscribers.

- **BS-022 — Debug-level backtraces.** A backtrace is appended to error messages only when the configured log level is exactly 3 (Debug).

- **BS-023 — Effective key length equals IV length.** Although `getKeyHash` defaults to a 32-byte output, every cipher backend requests the **IV length** as the key length, so for AES-128 the binary key is the 16-byte IV-length prefix of the keyed SHA-512 hash, not the full default.

- **BS-024 — `randNumber` ignores explicit bounds when `len` is set.** When `len` is non-zero, the range is `[1·10^(len-1)… , 9…]` derived purely from `len` (right-padding `1`/`9`); the `start`/`end` arguments take effect only when `len` is 0.

- **BS-025 — Backend-level decrypt envelope re-validation.** Independent of the top-level decrypt guard (BS-006), each cipher backend re-validates the inner `$<cid>$` envelope and refuses to decrypt on a missing cid, an empty post-IV remainder, or an unresolvable/mismatched cipher.

- **BS-026 — All SQL values pass through `db_input`; numeric values fast-path.** Every value embedded into a SQL statement is routed through `db_input`, which either (a) returns a numeric value verbatim (bare, unquoted) when it matches the driver's numeric pattern, or (b) escapes and (by default) single-quotes a non-numeric value via `db_real_escape`. This single funnel is the system-wide SQL-injection defense; `db_real_escape` must not be called directly. The two drivers' numeric patterns differ (legacy `^\d+(\.\d+)?$` vs. `mysqli` `^(?:\d+\.\d+|[1-9]\d*)$`), so a leading-zero integer is escaped/quoted under `mysqli` but passed bare under the legacy driver.

- **BS-027 — Result output reverses magic quotes only when the runtime added them.** `db_output` returns fetched values unchanged when `get_magic_quotes_runtime` is OFF (the modern default); only when it is ON does it `stripslashes` every non-numeric scalar (recursing over arrays). All result-set accessors route their output through `db_output`, so the compensation is applied uniformly and is a no-op on modern runtimes.

- **BS-028 — Every connection forces `sql_mode = ''`.** Immediately after connecting (and setting UTF-8), the connect routine issues `SET SESSION sql_mode=''`, clearing all server SQL modes for the application session. This is a deliberate compatibility choice: it suppresses strict-mode errors so osTicket's lax write patterns succeed on servers whose default `sql_mode` is strict/traditional. It is a per-session side-effect, not a server-global change.

- **BS-029 — One contract, two interchangeable drivers.** The `db_*` API is defined identically by the legacy `mysql_*` driver and the `mysqli` driver; a build links exactly one. Callers are driver-agnostic. The only documented behavioral divergences are the `db_input` numeric pattern (BS-026), the `mysqli` deadlock retry (FS-003.27), and connect-time SSL/error handling (FS-001).

---

## Data Requirements

This layer is mostly stateless utility code. Persistent data it reads:

- **System log entry** (read model — schema owned by FS-091, `ost_syslog`): `log_id`, `log_type` (`Error`/`Warning`/`Debug`), `title`, `log` (message text), `ip_address`, `created`, `updated`.
- **Timezone entry** (read model — schema owned by FS-091, `ost_timezone`): `id`, `offset` (UTC offset in hours), `timezone` (name).

Transient/contextual data it consumes:

- **Session timezone state:** `TZ_OFFSET` (hours), `TZ_DST` (boolean) — drives user-local date rendering and `dbtime`.
- **Configured offsets/formats (owned by FS-001/FS-032):** DB timezone offset; date / date-time / day-date-time formats; clickable-URLs flag; log level; log grace period.
- **Crypto keys:** an externally-supplied master key and a per-context sub-key (namespace); not stored by this layer.
- **Signal registry:** an in-process map of signal-name → ordered subscriber tuples `(object-class-filter, callable, predicate)`; exists only for the duration of a request.

---

## User Flows / Interactions

This is infrastructure with **no direct end-user interface**. Its behaviors surface indirectly through other specs:

- A user submitting a form triggers FS-003.7/FS-003.8 validation; failures present the field's declared `error` message.
- Inbound email text flows through FS-003.16/FS-003.17 (MIME/charset decoding) before threading (cross-reference FS-041).
- Stored ticket/message text is rendered to the browser through FS-003.10–FS-003.14 (sanitize/encode/clickable URLs).
- Admin-facing dates are rendered through FS-003.19 using session timezone state.
- Side-effecting events (e.g. ticket lifecycle hooks) are dispatched through FS-003.21 signals.
- Errors raised anywhere auto-append to the system log (FS-003.23) and become visible in the staff log viewer (cross-reference FS-033).

---

## Edge Cases

- **EC-001 — Unavailable crypto backend on decrypt.** If the backend identified by a ciphertext's tag is not currently installed, decryption returns `false` rather than throwing; callers must treat `false` as "could not decrypt."

- **EC-002 — Empty/garbage ciphertext.** Decryption of input that is empty or does not begin with `$` returns `false` immediately without attempting any cryptographic work.

- **EC-003 — No system entropy source.** When no OS/extension random source is available, the software AES-CTR fallback is used; output is still length-correct but of lower entropy quality (KL-002).

- **EC-004 — Empty int field treated as valid presence.** The required-field check tolerates an empty value for `int`-typed fields (an explicit `0`/empty integer is not treated as "missing"), unlike other types.

- **EC-005 — `time`-type fields accept anything.** Time-typed validation performs no check, so any value passes (KL-003).

- **EC-006 — Magic-quotes input.** When the legacy magic-quotes runtime is active, validated input is slash-stripped first; without this the values would carry spurious backslashes.

- **EC-007 — Non-array validation source.** A non-array `source` short-circuits with `"Invalid input"`; missing field config short-circuits with `"No fields set up"`.

- **EC-008 — Lossy/invalid charset.** A transcode that produces empty output for non-empty input returns the original bytes assumed to be latin-1 (BS-019), preventing silent data loss but potentially yielding mojibake.

- **EC-009 — Phone/file-size pass-through.** A phone number that is neither 7 nor 10 digits, and a non-numeric byte count, are returned unchanged rather than erroring.

- **EC-010 — Signal with class filter but non-class object.** Registering a subscriber whose `object` filter is not a string class name raises a runtime warning at connect time; a bad predicate is discarded with a warning.

- **EC-011 — Unknown signal send.** Sending a signal nobody subscribed to is a silent no-op.

- **EC-012 — Legacy IIS redirect.** On IIS versions earlier than 7.0, redirects use a `Refresh` header instead of `Location` because those servers mis-emitted status/headers for `Location` alone.

- **EC-013 — Internet Explorer download header.** For Internet-Explorer-on-Windows user agents, the `Content-Disposition` omits the `attachment;` prefix to avoid a known download bug.

- **EC-014 — Backtrace only at Debug level.** Error messages include a backtrace only when log level is 3; at lower levels errors log without a trace.

- **EC-015 — Log/timezone lookup id mismatch.** `Log::lookup`/`Timezone::lookup` return `null` for non-numeric ids or when the loaded row's id does not equal the requested id (defensive identity check).

- **EC-016 — Backend-level decrypt guards.** Independently of the top-level decrypt guard (EC-002), each cipher backend re-validates the inner `$<cid>$` envelope and returns `false` on a missing cid, an empty post-IV remainder, or an unresolvable cipher. The mcrypt backend additionally rejects a cid that does not match the resolved cipher's cid.

- **EC-017 — Empty non-`int` value records an error even when not required.** Because the required/missing condition is not fully parenthesized, a falsy value of any non-`int` type (e.g. the string `"0"`) can record the field's `error` even when the field is declared not-required — except where the earlier not-required-and-falsy skip already short-circuited it (FS-003.7).

- **EC-018 — Clickable email TLD narrower than the validator.** The clickable-URL email auto-linker only matches TLDs of 2–4 characters, so addresses with longer TLDs validate via `is_email` (≥ 2) yet are not rendered as `mailto:` links.

- **EC-019 — Soft truncate with no space.** When the truncated prefix contains no space, the soft (non-`hard`) path produces just `" ..."`, dropping the visible text, because the no-space lookup resolves to position 0.

- **EC-020 — `db2gmtime` of empty input returns null.** Empty/falsy input to `db2gmtime` returns null (no value) rather than the current time; only `dbtime` defaults an empty input to the current GMT time.

- **EC-021 — Leading-zero integer escaped differently per driver.** `db_input("007")` is treated as numeric and passed through **bare** by the legacy driver (pattern `^\d+(\.\d+)?$`), but as non-numeric — and therefore **escaped and single-quoted** — by the `mysqli` driver (pattern `^(?:\d+\.\d+|[1-9]\d*)$`). A query relying on the bare-numeric form for a leading-zero value behaves differently depending on which driver the build links (BS-026, BS-029).

- **EC-022 — `db_assoc_array` on an empty result is undefined/empty.** When the result has zero rows, `db_assoc_array` never enters its accumulation loop and returns its uninitialized accumulator (an undefined/empty value rather than an explicit empty array). Callers must treat a falsy return as "no rows."

- **EC-023 — Connect forces empty `sql_mode`, masking strict-mode errors.** Because every connection sets `sql_mode=''` (BS-028), a server administrator's strict/traditional default `sql_mode` is silently overridden for osTicket's session. Lax writes (zero dates, value truncation, missing column defaults) that would error under strict mode succeed instead; conversely, an operator cannot enforce strict mode on the application by configuring the server default alone (KL-015).

---

## Dependencies

- **FS-001 (App bootstrap & shared request lifecycle)** — provides the global app/config context used by `Error` (log level, `logError`), `clickableurls` (`getLinkToken`), the time helpers (DB timezone offset, date formats, clickable-URLs flag), and is the home of the syslog **write** path (`log()`/`logError`/`alertAdmin`/`purgeLogs` — write-side collapse/alerting/purge owned by FS-001.12; `log_type` enum owned by FS-091.8).
- **FS-002 (Staff authentication, sessions & access control)** — consumes the crypto, random-code, and password/username validators for account credentials.
- **FS-032 / FS-033 (Admin settings; logs/pages/content)** — own the log-level and log-grace-period configuration and the staff-facing system-log viewer that reads the entries surfaced by FS-003.24.
- **FS-041 (Inbound email pipeline)** — primary consumer of charset normalization/transcoding and MIME/RFC 5987 decoding.
- **FS-091 (Reference data, enums & data model)** — canonical owner of the `ost_syslog` and `ost_timezone` table schemas and of any shared enum sets (log types, status codes); the procedural `db_*` helper layer specified here (FS-003.26–FS-003.30) is the access mechanism FS-091 names for reaching every table in that model.
- **FS-001 (App bootstrap) / FS-060 (Installer)** — own the bootstrap/installer-facing subset of the `db_*` API (`db_connect`, `db_query`, `db_version`, `db_create_database`) and the connection sequence that triggers the connect-time `sql_mode=''` side-effect (FS-003.29 / BS-028); the remaining helper contracts are owned here.
- **Bundled third-party libraries** — the HTML sanitizer (used by `safe_html`/`sanitize`), the AES/Hash crypto primitives, and the pure-PHP crypto fallback are external dependencies invoked but not specified here.

---

## Known Limitations

- **KL-001 — AES-128 / CBC fixed.** All backends are pinned to AES-128 in CBC mode with cipher tags that "must not change," so the encryption strength/mode cannot be upgraded for existing data without a migration that re-encrypts every stored value.

- **KL-002 — Software random fallback is weak.** When no OS/extension entropy source exists, random bytes come from an AES-CTR stream seeded from session id, microtime, and PID — substantially weaker than OS entropy and predictable under adverse conditions.

- **KL-003 — `time`-type validation is a stub.** The `time` validation case enforces nothing; date validation is also a loose "is this parseable" check with TODOs noting GNU date/time formats are not actually enforced.

- **KL-004 — Email/phone/URL validators are shape-only.** They confirm format/length, not deliverability or real-world validity; `is_url` accepts anything `parse_url` reports a host for.

- **KL-005 — `currentURL` treats only port 80 as standard.** HTTPS on port 443 is rendered with an explicit `:443` because only port 80 is special-cased; this can produce non-canonical URLs.

- **KL-006 — Signals are in-process & synchronous only.** The publish/subscribe bus has no persistence, no async delivery, no error isolation between subscribers, and no signal registry — a throwing subscriber can disrupt the publisher; unknown signals are silently ignored.

- **KL-007 — Three-level log granularity.** All syslog priorities collapse to Error/Warning/Debug, so finer-grained severity (e.g. distinguishing Critical from Error, or Notice from Info) is lost in storage.

- **KL-008 — Username allows broad Unicode.** Usernames permit any Unicode letter plus `. _ -`, which can admit visually-confusable / homoglyph characters.

- **KL-009 — PHP-4-era constructs.** Much of this layer carries explicit PHP-4-compatibility scaffolding (old-style constructors, manual time math, the `Error` object substituting for exceptions), with in-code TODOs anticipating a move to PHP 5.

- **KL-010 — Inconsistent file-size scaling.** `file_size` divides by 1024 for the "kb" tier but by 1024000 for the "mb" tier, so the megabyte figure is computed on a slightly different (mixed binary/decimal) scale than the kilobyte figure.

- **KL-011 — `elapsedTime` requires bcmath.** The elapsed-time decomposition uses `bcmod`; on a runtime without the bcmath extension the helper cannot run.

- **KL-012 — Non-uniform random code distribution.** `randCode` reduces masked bit-groups modulo the character-set size; when that size is not a power of two (e.g. the default 62), the character distribution is slightly biased.

- **KL-013 — Timezone name is unreadable.** `Timezone::getName`/`getDesc` read from a data slot that is never populated by the loader (the row is stored elsewhere), so the timezone name always reads empty — only the numeric offset is reliably retrievable.

- **KL-014 — Crypto strength upgrades are silent-on-encrypt only.** New writes pick the currently-preferred backend (BS-002) and re-tag, but existing stored values are only re-tagged when they happen to be re-encrypted; there is no bulk re-key path, so a host can hold a mix of mcrypt/OpenSSL/phpseclib-tagged values indefinitely (compounds KL-001).

- **KL-015 — Connect-time `sql_mode=''` cannot be configured off.** The empty-`sql_mode` reset (BS-028) is hard-coded in the connect routine with no configuration flag, so a deployment that wants the safety of MySQL strict mode for osTicket's own session has no supported way to keep it; the application's lax write patterns (zero dates, implicit truncation) depend on the relaxed mode and would surface errors if strict mode were retained.

- **KL-016 — `db_real_escape` defends only the string fast-path; numerics are pattern-trusted.** The injection defense in `db_input` (BS-026) trusts any value matching the numeric regex to be embedded bare. The regexes are anchored and conservative, but the safety of the bare-numeric branch rests entirely on the pattern being correct for the target column; a non-numeric-but-pattern-matching edge (and the per-driver pattern divergence, EC-021) means the "numeric" passthrough is a pattern assertion, not a type guarantee.

---

## Future Considerations

- Re-key/re-encrypt migration tooling to allow the AES-128/CBC pinning (KL-001) to be lifted without data loss.
- Stronger, mandatory CSPRNG with a hard failure when no OS entropy source is present, replacing the software fallback (KL-002).
- Real date/time validation honoring the documented GNU format intent (KL-003).
- Optional finer-grained log severities and structured (machine-parseable) log payloads (KL-007).
- Signal-bus hardening: error isolation per subscriber, optional priority ordering, and a discoverable signal registry (KL-006).
