# FS-003 Gap Report — Cryptography, Validation & Formatting Infrastructure

Phase-2 GAP-CLOSE run. Source slice: `class.crypto.php`, `class.validator.php`, `class.format.php`, `class.charset.php`, `class.misc.php`, `class.signal.php`, `class.http.php`, `class.error.php`, `class.log.php`, `class.timezone.php`. Report spec only — no source modified.

| ID | Type | Severity | What the code does | What the spec said | Fix applied |
|----|------|----------|--------------------|--------------------|-------------|
| G-01 | INCORRECT | High | `randNumber(len,start,end)`: when `len!=0` it ignores `start`/`end` and derives bounds by right-padding `1`/`9` to `len` digits; honors `start`/`end` only when `len==0`. Uses `mt_rand`. | "returns a random integer in a range bounded by `len` digits"; `start`/`end` params never mentioned. | FS-003.18 rewritten + new BS-024 documenting the `len==0` override path and mt_rand source. |
| G-02 | INCORRECT | Med | `Timezone::getName/getDesc` read `$this->info['timezone']`, but loader populates `$this->ht` — name always reads empty (latent bug). | "A loaded timezone exposes its id and offset" (silent on name reliability). | FS-003.25 note + KL-013 documenting the unreadable name. |
| G-03 | INCORRECT | Med | `file_size` mb tier divides by `1024000` (= 1000×1024), kb tier by `1024` — inconsistent scales. | "megabytes (dividing by 1,024,000)" — implied a clean value, no inconsistency flagged. | FS-003.15 / BS-016 clarified divisors + KL-010 on the mixed binary/decimal scale. |
| G-04 | MISSING | Med | Each cipher backend's `decrypt` independently re-validates the inner `$<cid>$` envelope; mcrypt also rejects cid-mismatch and empty post-IV remainder. | Only the top-level decrypt guard (FS-003.1/BS-006) documented. | FS-003.5 backend-decrypt-guard bullet + BS-025 + EC-016. |
| G-05 | MISSING | Med | Validator required-check is `(required && !isset(v)) || (!v && type!='int')` — unparenthesized; 2nd clause fires for falsy non-`int` values regardless of `required`. | "required and the value is missing/empty" — implied the check is gated on `required`. | FS-003.7 literal-condition note + EC-017. |
| G-06 | IMPRECISE | Med | `getKeyHash` defaults len=32 but all backends pass the IV length (16 for AES-128) → effective key is 16 bytes. | FS-003.3 stated default 32 with no note that callers override to IV length. | FS-003.3 bullet + BS-023. |
| G-07 | IMPRECISE | Low | `getCipher(cid,callback)` shared helper: direct lookup by cid, else first cipher passing the backend `_checkCipher` predicate; merges resolved `cid` into the row. Per-backend predicates enumerated. | FS-003.4 said each backend "internally tags" output; selection mechanics + predicates undocumented. | FS-003.4 two bullets on the shared selector and per-backend `_checkCipher`. |
| G-08 | IMPRECISE | Low | `Charset::transcode` 3rd fallback (`utf8_encode`) applies ONLY when normalized to=utf-8 AND from=ISO-8859-1; ordered iconv→mb_convert→utf8_encode. | "falling back to multibyte conversion, then to a UTF-8 latin-1 conversion" — missed the conditional gate. | FS-003.17 transcode bullet tightened. |
| G-09 | IMPRECISE | Low | `mimedecode` exact 3-branch priority: imap_mime_header_decode → (text[0]=='=' && iconv_mime_decode) → (to==utf-8 && imap_utf8); else unchanged. | "falls back through alternate decoders depending on available runtime functions and the leading-`=` marker." | FS-003.16 mimedecode bullet enumerated. |
| G-10 | MISSING | Low | Clickable-URL email auto-linker uses TLD `[a-z]{2,4}` — narrower than `is_email`'s `{2,}`. | FS-003.14 silent on email TLD; only `mailto` wrapping mentioned. | FS-003.14 bullets + EC-018. |
| G-11 | MISSING | Low | `random()` non-Windows `/dev/urandom` handle is opened once and cached statically; non-Windows OpenSSL branch is not version-gated (Windows is). | FS-003.6 listed source order but not static caching or the version-gate asymmetry. | FS-003.6 non-Windows bullet expanded. |
| G-12 | MISSING | Low | OpenSSL backend sources IV from `openssl_random_pseudo_bytes` directly (not `Crypto::random`); all backends pass `OPENSSL_RAW_DATA`/true. | FS-003.5 said each backend prepends a random IV; IV source + raw-data option unstated. | FS-003.5 IV-source bullet added. |
| G-13 | MISSING | Low | `elapsedTime` decomposition uses `bcmod` (bcmath extension dependency). | No dependency noted. | FS-003.15 elapsedTime bullet + KL-011. |
| G-14 | MISSING | Low | `truncate` soft path with no space in prefix → `strrpos` returns false→0 → output is just `" ..."` (text lost). | "trims back to the last space" — no-space case unspecified. | FS-003.12 truncate bullet + EC-019. |
| G-15 | MISSING | Low | `db2gmtime` returns null (no value) for empty input; only `dbtime` defaults to current GMT. | FS-003.19 conflated the two; empty-input behavior unstated. | FS-003.19 db2gmtime/gmtime bullets + EC-020. |
| G-16 | IMPRECISE | Low | `gmtime()` = `time() - date('Z')` (server's local UTC offset in seconds), not a configured offset. | "time() minus the local UTC offset" — ambiguous about source. | FS-003.19 gmtime bullet clarified. |
| G-17 | IMPRECISE | Low | `randCode` consumes bytes as little-endian uint32 (`unpack('V*')`), masks `bitsPerChar` at a time, reduces modulo set size (non-uniform for non-power-of-2 sets). | FS-003.18 said "sizing the random byte request from the bit-width" — packing/modulo bias unstated. | FS-003.18 randCode bullet + KL-012. |
| G-18 | MISSING | Low | `__rand_seed` builds a 32-bit seed: low 16 bits = (microtime×1e6) mod 65535, high 16 bits = value mod 65535. | "seeds … from a mix of the supplied value and current microtime." | FS-003.18 `__rand_seed` bullet detailed. |
| G-19 | MISSING | Info | No bulk re-key path; stored values stay on whichever backend tag they were last encrypted under. | KL-001 covered the AES-128/CBC pin but not the mixed-backend persistence consequence. | KL-014 added. |

## Summary

- **Gaps found / fixed:** 19 / 19.
- **By type:** MISSING 11, IMPRECISE 6, INCORRECT 3 (G-01, G-02, G-03 — including one latent source bug surfaced as KL-013).
- **Most significant:**
  1. **G-01** — `randNumber` `start`/`end` arguments are silently dead unless `len==0`; spec implied they were honored. Materially affects ticket-number range reasoning.
  2. **G-02** — `Timezone::getName/getDesc` is a real source bug (wrong member slot) making the timezone name permanently unreadable through this read model.
  3. **G-05/G-06** — validator required-check operator-precedence quirk (falsy non-`int` values error regardless of `required`) and the effective-key-length-equals-IV-length crypto fact (16-byte key, not 32).
- **New ids added to FS-003:** BS-023, BS-024, BS-025 (3 BS); EC-016 through EC-020 (5 EC); KL-010 through KL-014 (5 KL). No new FR sub-numbers (all corrections folded into existing FS-003.1–FS-003.25 acceptance criteria). No existing ids renumbered.
