//! HTML sanitization of free-text input (pure functions).
//!
//! @implements FS-003.10: HTML sanitization & safe HTML — strip/escape
//!   disallowed HTML before persistence.
//! @implements BS-014: Log/text sanitization — destined-for-storage text is run
//!   through safe-HTML sanitization; the tag-stripping variant additionally
//!   removes all tags.
//!
//! Implemented with the `ammonia` crate (the Rust stand-in for the legacy
//! bundled HTML-sanitizing library). Two entry points mirror the legacy split:
//! `safe_html` (keep a safe subset of markup) and `sanitize` (strip ALL tags).

use std::sync::OnceLock;

use ammonia::Builder;

/// A reusable ammonia builder permitting only a safe markup subset.
///
/// Disallowed elements (e.g. `<script>`) are removed entirely; their *text*
/// content is preserved and HTML-escaped, matching the legacy "balance/close
/// and drop dangerous tags" behaviour (FS-003.10).
fn safe_builder() -> &'static Builder<'static> {
    static BUILDER: OnceLock<Builder<'static>> = OnceLock::new();
    // ammonia's default allow-list already excludes <script>/<style> and strips
    // dangerous attributes/URLs (javascript: URLs, event handlers). The default
    // safe set is exactly what FS-003.10's "safe HTML" path needs.
    BUILDER.get_or_init(Builder::default)
}

/// Sanitize free-text HTML, keeping only a safe subset of markup.
///
/// Disallowed tags (such as `<script>`) are stripped while their inner text is
/// preserved; safe formatting tags are kept. Used for rich free-text that may
/// legitimately contain limited markup.
///
/// @implements FS-003.10 / BS-014: safe-HTML sanitization.
#[must_use]
pub fn safe_html(input: &str) -> String {
    safe_builder().clean(input).to_string()
}

/// Sanitize free-text and strip ALL remaining tags, leaving plain text.
///
/// First runs [`safe_html`], then removes every remaining tag — the path the
/// legacy code uses for titles/log messages before persistence (BS-014). Safe
/// text content is preserved; markup is removed.
///
/// @implements FS-003.10 / BS-014: sanitize-then-strip-tags.
#[must_use]
pub fn sanitize(input: &str) -> String {
    // An empty tag allow-list removes every element but keeps text content.
    Builder::empty().clean(&safe_html(input)).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_script_tags_but_keeps_safe_text() {
        let out = sanitize("<script>alert(1)</script>hello");
        assert!(!out.contains("<script>"), "script tag must be removed");
        assert!(!out.contains("</script>"), "script close tag removed");
        assert!(out.contains("hello"), "safe text must be preserved");
        // The dangerous payload's *executable* form is gone.
        assert!(!out.contains("<"), "no tags remain after strip: {out}");
    }

    #[test]
    fn sanitize_removes_all_markup() {
        let out = sanitize("<b>bold</b> and <i>italic</i>");
        assert_eq!(out.trim(), "bold and italic");
    }

    #[test]
    fn safe_html_drops_script_but_may_keep_basic_formatting() {
        let out = safe_html("<p>hi</p><script>evil()</script>");
        assert!(!out.contains("script"), "script element removed: {out}");
        assert!(out.contains("hi"), "text preserved");
    }

    #[test]
    fn sanitize_neutralizes_event_handlers_and_js_urls() {
        let out = sanitize(r#"<a href="javascript:alert(1)" onclick="x()">link</a>"#);
        assert!(!out.to_lowercase().contains("javascript:"), "js url gone: {out}");
        assert!(!out.to_lowercase().contains("onclick"), "handler gone: {out}");
        assert!(out.contains("link"), "anchor text preserved");
    }

    #[test]
    fn plain_text_passes_through_unchanged() {
        assert_eq!(sanitize("just some text"), "just some text");
    }
}
