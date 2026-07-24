//! `%{token}` variable substitution engine (TS-M2-C1).
//!
//! A pure, transport-agnostic text templater reused by canned responses (D2) and
//! email templates (E2/E3). It resolves `%{name}` and `%{a.b.c}` dot-paths
//! against a [`VarContext`], leaving unknown tokens **verbatim** (so an authoring
//! mistake surfaces as a visible `%{...}` rather than silently-dropped content)
//! and always exposing `%{url}` from a caller-supplied base URL.
//!
//! Grammar (legacy-faithful, `class.variable.php`):
//! `%{` `[A-Za-z_]` `[\w._]+` `}` — the token name starts with a letter or
//! underscore and is followed by **one or more** word/dot chars, so a
//! single-character name like `%{x}` is NOT a token (the `+` needs ≥1 trailing
//! char) and is preserved verbatim.
//!
//! @implements FS-040.11: variable-substitution grammar `%{token}` + dot-paths.
//! @implements BS-040.14: dot-path traversal resolves nested tokens.
//! @implements BS-040.17: an unknown token is left literally in the output.
//! @implements BS-040.18: substitution applies across an array (and a string).
//! @implements BS-040.19: `%{url}` is always present from the base-URL input.

use std::collections::BTreeMap;

/// The reserved token always available from the base-URL input (BS-040.19).
pub const URL_TOKEN: &str = "url";

/// A resolution context: a tree of named scalars/objects the engine walks with a
/// dot-path. Designed as a small owned tree so any caller (canned D2, email
/// E2/E3) can assemble one from a ticket row, config, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarContext {
    /// A leaf string value.
    Scalar(String),
    /// A named object whose fields are resolved by the next dot-path segment.
    Object(BTreeMap<String, VarContext>),
}

impl VarContext {
    /// An empty object context.
    #[must_use]
    pub fn object() -> Self {
        VarContext::Object(BTreeMap::new())
    }

    /// Insert a scalar field (chainable). Overwrites any existing field.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if let VarContext::Object(ref mut map) = self {
            map.insert(key.into(), VarContext::Scalar(value.into()));
        }
        self
    }

    /// Insert a nested object field (chainable).
    #[must_use]
    pub fn with_object(mut self, key: impl Into<String>, child: VarContext) -> Self {
        if let VarContext::Object(ref mut map) = self {
            map.insert(key.into(), child);
        }
        self
    }

    /// Build the M2 token catalog context for a ticket: the `ticket.*` object
    /// (`number`, `name`, `subject`, `email`, `status`, `create_date`) plus the
    /// nested `ticket.dept.name`. Callers (canned D2, email E2/E3) assemble this
    /// from a ticket row and pass it to [`VariableReplacer::new`] alongside the
    /// `helpdesk_url` base (§6) backing `%{url}`.
    ///
    /// @implements FS-040.11: the M2 `%{ticket.*}` + `%{ticket.dept.name}` catalog.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn ticket(
        number: impl Into<String>,
        name: impl Into<String>,
        subject: impl Into<String>,
        email: impl Into<String>,
        status: impl Into<String>,
        create_date: impl Into<String>,
        dept_name: impl Into<String>,
    ) -> Self {
        let ticket = VarContext::object()
            .with("number", number)
            .with("name", name)
            .with("subject", subject)
            .with("email", email)
            .with("status", status)
            .with("create_date", create_date)
            .with_object("dept", VarContext::object().with("name", dept_name));
        VarContext::object().with_object("ticket", ticket)
    }

    /// Resolve a dot-path (already split into segments) to a scalar string.
    ///
    /// Returns `None` when any segment is missing or a non-leaf is reached at the
    /// end of the path (the caller then preserves the token verbatim).
    fn resolve(&self, segments: &[&str]) -> Option<String> {
        match (self, segments.split_first()) {
            // Path fully consumed at a scalar ⇒ that's the value.
            (VarContext::Scalar(s), None) => Some(s.clone()),
            // Path remaining at a scalar, or path ending at an object ⇒ no value.
            (VarContext::Scalar(_), Some(_)) => None,
            (VarContext::Object(_), None) => None,
            // Descend one object segment.
            (VarContext::Object(map), Some((head, rest))) => map.get(*head)?.resolve(rest),
        }
    }
}

/// The `%{token}` replacer.
///
/// Holds the resolution [`VarContext`] plus the base URL that backs the reserved
/// `%{url}` token. Construct via [`VariableReplacer::new`] and render with
/// [`VariableReplacer::render`] (single string) or
/// [`VariableReplacer::render_all`] (array of strings, BS-040.18).
#[derive(Debug, Clone)]
pub struct VariableReplacer {
    context: VarContext,
    base_url: String,
}

impl VariableReplacer {
    /// Build a replacer over `context`, with `base_url` backing `%{url}`.
    ///
    /// `%{url}` is always available regardless of `context`, satisfying
    /// BS-040.19; a `url` key already in `context` is shadowed by `base_url`.
    pub fn new(context: VarContext, base_url: impl Into<String>) -> Self {
        Self {
            context,
            base_url: base_url.into(),
        }
    }

    /// Render one template string, substituting every recognised `%{token}` and
    /// leaving unknown tokens verbatim.
    #[must_use]
    pub fn render(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            // Look for the `%{` opener.
            if bytes[i] == b'%' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                if let Some((name, end)) = parse_token(text, i) {
                    match self.lookup(name) {
                        Some(value) => out.push_str(&value),
                        None => out.push_str(&text[i..end]), // verbatim
                    }
                    i = end;
                    continue;
                }
            }
            // Not the start of a valid token: copy this char and advance. (Index
            // by char boundary so multi-byte UTF-8 is preserved.)
            let ch = text[i..].chars().next().expect("valid char boundary");
            out.push(ch);
            i += ch.len_utf8();
        }
        out
    }

    /// Render an array of template strings (BS-040.18): subject + body, etc.
    #[must_use]
    pub fn render_all(&self, texts: &[&str]) -> Vec<String> {
        texts.iter().map(|t| self.render(t)).collect()
    }

    /// Resolve a token name (a dot-path) to its replacement, or `None` to keep it
    /// verbatim. The reserved `%{url}` always resolves to the base URL.
    fn lookup(&self, name: &str) -> Option<String> {
        if name == URL_TOKEN {
            return Some(self.base_url.clone());
        }
        let segments: Vec<&str> = name.split('.').collect();
        self.context.resolve(&segments)
    }
}

/// Parse a token starting at `start` (which must point at `%`). Returns the inner
/// token name and the byte index just past the closing `}`, or `None` if the
/// text at `start` is not a grammar-valid token (it is then copied verbatim).
///
/// Grammar: `%{` `[A-Za-z_]` `[\w._]+` `}` — first name char is a letter or `_`,
/// followed by ≥1 of word/`.`/`_`, so the whole name is ≥2 chars (AC-4).
fn parse_token(text: &str, start: usize) -> Option<(&str, usize)> {
    let rest = &text[start..];
    let inner = rest.strip_prefix("%{")?;
    let close = inner.find('}')?;
    let name = &inner[..close];

    let mut chars = name.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    // Need ≥1 trailing char (the legacy `+`), each a word char / `.` / `_`.
    let mut trailing = 0usize;
    for c in chars {
        if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
            trailing += 1;
        } else {
            return None; // an invalid char (space, `%`, …) ⇒ not a token.
        }
    }
    if trailing == 0 {
        return None; // single-char name like `%{x}` ⇒ preserved (AC-4).
    }

    // end = start + len("%{") + name bytes + len("}")
    let end = start + 2 + name.len() + 1;
    Some((name, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The M2 catalog context for a sample ticket (mirrors AC-6's shape).
    fn ticket_ctx() -> VarContext {
        let ticket = VarContext::object()
            .with("number", "123456")
            .with("name", "Mia Example")
            .with("subject", "Printer broken")
            .with("email", "mia@example.com")
            .with("status", "open")
            .with("create_date", "2026-06-11T10:00:00Z")
            .with_object("dept", VarContext::object().with("name", "Support"));
        VarContext::object().with_object("ticket", ticket)
    }

    fn replacer() -> VariableReplacer {
        VariableReplacer::new(ticket_ctx(), "http://localhost:3702")
    }

    // --- AC-1: BS-040.14 dot-path traversal resolves nested tokens. ----------
    #[test]
    fn dot_path_resolves_nested_tokens() {
        let out = replacer().render("#%{ticket.number} — %{ticket.dept.name}");
        assert_eq!(out, "#123456 — Support");
        assert!(!out.contains("%{"), "no literal token remains");
    }

    // --- AC-2: BS-040.17 an unknown token is left literally. ------------------
    #[test]
    fn unknown_token_is_preserved_verbatim() {
        let out = replacer().render("%{ticket.bogus}");
        assert_eq!(out, "%{ticket.bogus}");
    }

    // --- AC-3: BS-040.19 %{url} is always present without ticket context. -----
    #[test]
    fn url_is_always_present_from_base_url() {
        // A context with NO `url` field still resolves %{url} from the base URL.
        let r = VariableReplacer::new(VarContext::object(), "https://help.example.com");
        assert_eq!(r.render("%{url}"), "https://help.example.com");
    }

    // --- AC-4: BS-040.14 a single-char token name is NOT recognised. ----------
    #[test]
    fn single_char_token_name_is_preserved() {
        assert_eq!(replacer().render("%{x}"), "%{x}");
        // …but a two-char name IS a token (resolves or preserved-as-unknown).
        assert_eq!(replacer().render("%{xy}"), "%{xy}"); // unknown ⇒ verbatim
    }

    // --- AC-5: BS-040.18 substitution applies across an array. ----------------
    #[test]
    fn substitution_applies_across_an_array() {
        let out = replacer().render_all(&["%{ticket.subject}", "Ticket %{ticket.number}"]);
        assert_eq!(out, vec!["Printer broken", "Ticket 123456"]);
    }

    // --- Grammar/edge cases ---------------------------------------------------
    #[test]
    fn all_catalog_tokens_resolve() {
        let r = replacer();
        assert_eq!(r.render("%{ticket.number}"), "123456");
        assert_eq!(r.render("%{ticket.name}"), "Mia Example");
        assert_eq!(r.render("%{ticket.subject}"), "Printer broken");
        assert_eq!(r.render("%{ticket.email}"), "mia@example.com");
        assert_eq!(r.render("%{ticket.status}"), "open");
        assert_eq!(r.render("%{ticket.create_date}"), "2026-06-11T10:00:00Z");
        assert_eq!(r.render("%{ticket.dept.name}"), "Support");
        assert_eq!(r.render("%{url}"), "http://localhost:3702");
    }

    #[test]
    fn url_base_shadows_a_context_url_field() {
        let ctx = VarContext::object().with("url", "WRONG");
        let r = VariableReplacer::new(ctx, "http://right");
        assert_eq!(r.render("%{url}"), "http://right");
    }

    #[test]
    fn malformed_openers_are_copied_verbatim() {
        let r = replacer();
        assert_eq!(r.render("100% done"), "100% done");
        assert_eq!(r.render("%{unclosed"), "%{unclosed");
        assert_eq!(r.render("%{has space}"), "%{has space}");
        assert_eq!(r.render("a%{}b"), "a%{}b"); // empty name ⇒ not a token
        assert_eq!(r.render("%{1abc}"), "%{1abc}"); // must start with letter/_
    }

    #[test]
    fn resolving_an_object_path_end_preserves_token() {
        // %{ticket} ends on an Object (not a scalar) ⇒ unknown ⇒ verbatim.
        assert_eq!(replacer().render("%{ticket}"), "%{ticket}");
        // descending past a scalar also fails ⇒ verbatim.
        assert_eq!(replacer().render("%{ticket.number.x}"), "%{ticket.number.x}");
    }

    #[test]
    fn multiple_and_adjacent_tokens_and_unicode() {
        let out = replacer().render("café %{ticket.number}%{ticket.status}");
        assert_eq!(out, "café 123456open");
    }

    #[test]
    fn leading_underscore_name_is_a_valid_token() {
        let ctx = VarContext::object().with("_x", "ok");
        let r = VariableReplacer::new(ctx, "u");
        assert_eq!(r.render("%{_x}"), "ok");
    }
}
