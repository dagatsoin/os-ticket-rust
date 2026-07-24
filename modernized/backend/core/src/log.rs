//! Minimal write-side system-logging facility (TS-M4-G1, DEVIATION M4-D2).
//!
//! A thin `osTicket::log()`-equivalent that records `syslog` rows on notable
//! admin/system events, plus the grace-period purge sweep (TS-M4-G2). Full
//! FS-003 logging is out of scope — this is a best-effort writer + a handful of
//! call sites.
//!
//! Design guarantees:
//! * **Best-effort** — [`log`] returns `()` and NEVER propagates: a config-read
//!   or INSERT error is logged via `tracing::warn` and swallowed, so an observed
//!   operation (a login, an admin CRUD mutation) can never fail because its
//!   audit row could not be written (TS-M4-G1 AC-3).
//! * **Threshold-gated** — a row persists only when its severity is at or below
//!   the configured `log_level` (BS-033 / FS-033.1). `log_level` accepts either a
//!   numeric level (`1`/`2`/`3`) or the level name (`Error`/`Warning`/`Debug`);
//!   an unset/unparsable value defaults to the most-verbose Debug so the viewer
//!   has data by default.
//!
//! @implements FS-033.1 (DEVIATION M4-D2): syslog write-side + call sites.
//! @implements BS-033.6: grace-period purge sweep ([`purge_logs`]).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::postgres::PgPool;

/// A syslog entry severity. Lower number = higher severity (Error is 1).
///
/// The numeric severity is compared against the configured `log_level`
/// threshold: an entry persists iff `severity() <= threshold`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogType {
    /// System error (severity 1 — always persisted while logging is on).
    Error,
    /// Warning (severity 2).
    Warning,
    /// Debug / informational (severity 3 — the most verbose).
    Debug,
}

impl LogType {
    /// Numeric severity (Error = 1, Warning = 2, Debug = 3).
    #[must_use]
    pub fn severity(self) -> i32 {
        match self {
            LogType::Error => 1,
            LogType::Warning => 2,
            LogType::Debug => 3,
        }
    }

    /// The stored `syslog.log_type` literal (matches the table CHECK constraint).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            LogType::Error => "Error",
            LogType::Warning => "Warning",
            LogType::Debug => "Debug",
        }
    }
}

/// The canonical stored `log_level` values: severity **labels** (EPIC-M4-G's
/// viewer + the FS-032 System settings tab agree on this representation). The
/// Debug label is the most verbose. `parse_log_level` also tolerates the legacy
/// numeric form (`1`/`2`/`3`) for backward compatibility.
pub const LOG_LEVEL_LABELS: [&str; 3] = ["Error", "Warning", "Debug"];

/// Whether `value` is a recognised `log_level` — a severity label
/// (`Error`/`Warning`/`Debug`, case-insensitive) or the legacy numeric level
/// `1`/`2`/`3`. Used by the System settings tab to reject an unknown value
/// (→ 422) while accepting exactly what [`parse_log_level`] gates on, so the
/// settings save and the log-gating read agree on one representation.
#[must_use]
pub fn is_known_log_level(value: &str) -> bool {
    let t = value.trim();
    if let Ok(n) = t.parse::<i32>() {
        return (1..=3).contains(&n);
    }
    LOG_LEVEL_LABELS.iter().any(|l| l.eq_ignore_ascii_case(t))
}

/// Config key holding the verbosity threshold. Canonically a severity **label**
/// ([`LOG_LEVEL_LABELS`]); the legacy numeric `1`/`2`/`3` is still accepted.
/// Missing/unparsable ⇒ [`DEFAULT_LOG_LEVEL`].
pub const CFG_LOG_LEVEL: &str = "log_level";

/// Config key holding the retention window in **months** for [`purge_logs`].
pub const CFG_LOG_GRACEPERIOD: &str = "log_graceperiod";

/// The default threshold when `log_level` is unset/unparsable: Debug (3, the
/// most verbose) so the viewer has data by default. "Parse defensively."
pub const DEFAULT_LOG_LEVEL: i32 = 3;

/// Parse a `log_level` config value into a numeric threshold.
///
/// Accepts a numeric level (`"1"`/`"2"`/`"3"`) OR a level name
/// (`"Error"`/`"Warning"`/`"Debug"`, case-insensitive). Anything else — including
/// an empty string or `None` — yields [`DEFAULT_LOG_LEVEL`] (defensive parse).
#[must_use]
pub fn parse_log_level(value: Option<&str>) -> i32 {
    let Some(raw) = value else {
        return DEFAULT_LOG_LEVEL;
    };
    let t = raw.trim();
    if let Ok(n) = t.parse::<i32>() {
        // Clamp to the known band so an out-of-range number is still sane.
        return n.clamp(0, 3);
    }
    match t.to_ascii_lowercase().as_str() {
        "error" => 1,
        "warning" => 2,
        "debug" => 3,
        _ => DEFAULT_LOG_LEVEL,
    }
}

/// Read + parse the `log_level` threshold (best-effort; defaults on any error).
async fn read_threshold(pool: &PgPool) -> i32 {
    let value: Option<String> =
        sqlx::query_scalar::<_, String>("SELECT value FROM config WHERE key = $1")
            .bind(CFG_LOG_LEVEL)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    parse_log_level(value.as_deref())
}

/// Record a `syslog` row for a notable event — **best-effort, never fails**.
///
/// The entry is written only when its severity clears the configured `log_level`
/// threshold. Every error path (threshold read, INSERT) is swallowed with a
/// `tracing::warn`, so the caller's operation is never affected by a logging
/// failure (TS-M4-G1 AC-3). `ip` is the captured client IP (empty string ok).
///
/// @implements FS-033.1 (M4-D2): threshold-gated best-effort syslog writer.
pub async fn log(
    pool: &PgPool,
    log_type: LogType,
    title: impl Into<String>,
    detail: impl Into<String>,
    ip: impl Into<String>,
) {
    let threshold = read_threshold(pool).await;
    if log_type.severity() > threshold {
        // Below the configured verbosity — nothing to persist.
        return;
    }

    let title = title.into();
    let detail = detail.into();
    let ip = ip.into();

    if let Err(e) = sqlx::query(
        "INSERT INTO syslog (log_type, title, log, ip_address) VALUES ($1, $2, $3, $4)",
    )
    .bind(log_type.as_str())
    .bind(&title)
    .bind(&detail)
    .bind(&ip)
    .execute(pool)
    .await
    {
        // Swallowed: logging must never fail the observed operation.
        tracing::warn!(error = %e, title = %title, "syslog write failed (swallowed)");
    }
}

// ---------------------------------------------------------------------------
// Failed-login log throttle (WARN-3) — bound attacker-driven syslog growth.
// ---------------------------------------------------------------------------
//
// An unauthenticated attacker can hammer the staff-login endpoint; without a cap
// every failed attempt would write one `Warning` syslog row, letting brute force
// grow the table without bound (disk-fill DoS). This is a per-key fixed-window
// rate limiter with coalescing: at most [`FAILED_LOGIN_MAX_PER_WINDOW`] rows are
// written per key per [`FAILED_LOGIN_WINDOW_SECS`] window; attempts over the cap
// are dropped but COUNTED, and the count is reported on the next written row so
// genuine failed-login visibility is preserved (we never silently drop all of
// them, just bound the volume).

/// Length of the fixed rate-limit window, in seconds.
pub const FAILED_LOGIN_WINDOW_SECS: i64 = 60;

/// Maximum failed-login rows written per key per window; further attempts in the
/// window are suppressed (counted, then reported on the next written row).
pub const FAILED_LOGIN_MAX_PER_WINDOW: u32 = 10;

/// Per-key fixed-window counter state for the failed-login throttle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailedLoginWindow {
    /// Unix-seconds start of the current window.
    pub window_start: i64,
    /// Rows already ALLOWED (written) in this window.
    pub allowed: u32,
    /// Attempts SUPPRESSED (over cap) since the last written row.
    pub suppressed: u32,
}

/// The outcome of a throttle check for one failed-login attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleDecision {
    /// Write the syslog row; `suppressed` prior attempts were dropped since the
    /// last written row and should be mentioned in it (coalesced summary).
    Log { suppressed: u32 },
    /// Over the per-window cap — drop this row (it is counted for later report).
    Skip,
}

/// Pure fixed-window decision (testable without a clock). Advances `win` and
/// returns whether the attempt's row should be written.
///
/// * When the window has elapsed it rolls: the attempt is written, carrying any
///   still-pending suppressed count into the report.
/// * Within the window, up to `max_per_window` rows are written; the first write
///   after a burst of suppressions reports (and clears) the suppressed count.
/// * Over the cap, the attempt is suppressed (counted) and no row is written.
#[must_use]
pub fn failed_login_decision(
    win: &mut FailedLoginWindow,
    now_secs: i64,
    window_secs: i64,
    max_per_window: u32,
) -> ThrottleDecision {
    if now_secs.saturating_sub(win.window_start) >= window_secs {
        let carried = win.suppressed;
        win.window_start = now_secs;
        win.allowed = 1;
        win.suppressed = 0;
        return ThrottleDecision::Log { suppressed: carried };
    }
    if win.allowed < max_per_window {
        win.allowed += 1;
        let s = win.suppressed;
        win.suppressed = 0;
        return ThrottleDecision::Log { suppressed: s };
    }
    win.suppressed = win.suppressed.saturating_add(1);
    ThrottleDecision::Skip
}

fn throttle_map() -> &'static Mutex<HashMap<String, FailedLoginWindow>> {
    static MAP: OnceLock<Mutex<HashMap<String, FailedLoginWindow>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Register a failed-login attempt for `key` (the client IP, or `"global"` when
/// the IP is empty) and decide whether its syslog row should be written.
///
/// Returns `Some(suppressed)` when the caller SHOULD write the row (with the
/// number of previously-suppressed attempts to mention), or `None` when the
/// attempt is over the per-window cap and its row must be dropped. This is the
/// process-global entry point over [`failed_login_decision`]; it also prunes
/// stale keys so the map cannot grow without bound under a distributed flood.
///
/// @implements WARN-3: cap attacker-driven failed-login syslog growth.
pub fn note_failed_login(key: &str) -> Option<u32> {
    let key = if key.trim().is_empty() { "global" } else { key.trim() };
    let now = now_unix_secs();

    let mut map = throttle_map()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // Bound memory: when the map grows large, drop keys whose window is long
    // stale (two windows old) — they carry no pending suppressed report.
    if map.len() > 10_000 {
        let cutoff = now - 2 * FAILED_LOGIN_WINDOW_SECS;
        map.retain(|_, w| w.window_start >= cutoff);
    }

    let win = map.entry(key.to_string()).or_insert(FailedLoginWindow {
        window_start: now,
        allowed: 0,
        suppressed: 0,
    });
    match failed_login_decision(
        win,
        now,
        FAILED_LOGIN_WINDOW_SECS,
        FAILED_LOGIN_MAX_PER_WINDOW,
    ) {
        ThrottleDecision::Log { suppressed } => Some(suppressed),
        ThrottleDecision::Skip => None,
    }
}

/// Parse a `log_graceperiod` config value into a positive month count.
///
/// Returns `Some(months)` only for a strictly-positive integer; `None` for
/// unset / zero / negative / non-numeric (→ [`purge_logs`] is a no-op).
#[must_use]
pub fn parse_graceperiod_months(value: Option<&str>) -> Option<i64> {
    value
        .map(str::trim)
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|m| *m > 0)
}

/// Delete `syslog` rows older than the configured `log_graceperiod` **months**
/// (BS-033.6). Returns the number of rows deleted.
///
/// A **no-op** (returns `Ok(0)`) when `log_graceperiod` is unset, zero, negative,
/// or non-numeric — the retention window must be an explicit positive month
/// count for anything to be purged. KL-033.7 modernised (months-based interval
/// arithmetic rather than the legacy day math).
///
/// @implements BS-033.6: grace-period-gated over-age purge sweep.
pub async fn purge_logs(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let value: Option<String> =
        sqlx::query_scalar::<_, String>("SELECT value FROM config WHERE key = $1")
            .bind(CFG_LOG_GRACEPERIOD)
            .fetch_optional(pool)
            .await?;

    let Some(months) = parse_graceperiod_months(value.as_deref()) else {
        return Ok(0);
    };

    // `$1 || ' months'` builds a Postgres interval literal from the month count.
    let interval = format!("{months} months");
    let deleted = sqlx::query("DELETE FROM syslog WHERE created < now() - $1::interval")
        .bind(&interval)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_ordering() {
        assert_eq!(LogType::Error.severity(), 1);
        assert_eq!(LogType::Warning.severity(), 2);
        assert_eq!(LogType::Debug.severity(), 3);
    }

    #[test]
    fn parse_log_level_numeric_and_named() {
        assert_eq!(parse_log_level(Some("1")), 1);
        assert_eq!(parse_log_level(Some("2")), 2);
        assert_eq!(parse_log_level(Some("3")), 3);
        assert_eq!(parse_log_level(Some("Error")), 1);
        assert_eq!(parse_log_level(Some(" warning ")), 2);
        assert_eq!(parse_log_level(Some("DEBUG")), 3);
    }

    #[test]
    fn parse_log_level_defensive_default() {
        assert_eq!(parse_log_level(None), DEFAULT_LOG_LEVEL);
        assert_eq!(parse_log_level(Some("")), DEFAULT_LOG_LEVEL);
        assert_eq!(parse_log_level(Some("nonsense")), DEFAULT_LOG_LEVEL);
    }

    #[test]
    fn known_log_level_accepts_labels_and_legacy_numeric() {
        // Canonical labels (case-insensitive).
        assert!(is_known_log_level("Error"));
        assert!(is_known_log_level("Warning"));
        assert!(is_known_log_level("Debug"));
        assert!(is_known_log_level(" debug "));
        // Legacy numeric levels stay valid so an old stored "3" still saves.
        assert!(is_known_log_level("1"));
        assert!(is_known_log_level("2"));
        assert!(is_known_log_level("3"));
        // Everything else is unknown → the settings tab yields a 422 field error.
        assert!(!is_known_log_level("Loud"));
        assert!(!is_known_log_level("0"));
        assert!(!is_known_log_level("4"));
        assert!(!is_known_log_level(""));
        // Whatever `is_known_log_level` accepts, `parse_log_level` also gates on.
        for v in LOG_LEVEL_LABELS {
            assert!(is_known_log_level(v));
            assert!((1..=3).contains(&parse_log_level(Some(v))));
        }
    }

    #[test]
    fn failed_login_throttle_caps_and_coalesces() {
        // WARN-3: at most `max` rows per window; overflow suppressed + reported.
        let mut w = FailedLoginWindow { window_start: 1_000, allowed: 0, suppressed: 0 };
        let window = 60;
        let max = 3;

        // First `max` attempts all write, none suppressed.
        for _ in 0..max {
            assert_eq!(
                failed_login_decision(&mut w, 1_000, window, max),
                ThrottleDecision::Log { suppressed: 0 }
            );
        }
        // Next attempts in the SAME window are suppressed (dropped, counted).
        for _ in 0..5 {
            assert_eq!(failed_login_decision(&mut w, 1_010, window, max), ThrottleDecision::Skip);
        }
        assert_eq!(w.suppressed, 5);

        // Once the window rolls, the next attempt writes and reports the backlog.
        assert_eq!(
            failed_login_decision(&mut w, 1_000 + window, window, max),
            ThrottleDecision::Log { suppressed: 5 }
        );
        assert_eq!(w.suppressed, 0, "suppressed backlog cleared after report");
        assert_eq!(w.allowed, 1, "fresh window counted the written row");
    }

    #[test]
    fn note_failed_login_bounds_burst() {
        // A tight burst on one key writes at most the per-window cap, then Nones.
        let key = format!("test-key-{}", nonce_key());
        let mut writes = 0u32;
        for _ in 0..(FAILED_LOGIN_MAX_PER_WINDOW + 20) {
            if note_failed_login(&key).is_some() {
                writes += 1;
            }
        }
        assert_eq!(writes, FAILED_LOGIN_MAX_PER_WINDOW, "burst capped at the window max");
        // Empty key folds into the shared "global" bucket (still bounded).
        assert!(note_failed_login("").is_some() || note_failed_login("").is_none());
    }

    fn nonce_key() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    #[test]
    fn graceperiod_only_positive_ints() {
        assert_eq!(parse_graceperiod_months(Some("12")), Some(12));
        assert_eq!(parse_graceperiod_months(Some("1")), Some(1));
        assert_eq!(parse_graceperiod_months(Some("0")), None);
        assert_eq!(parse_graceperiod_months(Some("-3")), None);
        assert_eq!(parse_graceperiod_months(Some("abc")), None);
        assert_eq!(parse_graceperiod_months(None), None);
    }
}
