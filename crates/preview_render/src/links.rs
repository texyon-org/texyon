//! Where a page's links point, and where its cross-references land.
//!
//! These are not drawn. They arrive as `pdf:` specials that describe an
//! *overlay* — a region of the page that is clickable, and a named spot another
//! page can jump to — and a viewer that only paints what the renderer draws
//! would leave `\ref`, `\cite` and `\url` inert.
//!
//! Only the fields that make a link work are recovered from the annotation
//! dictionaries here. They are PDF objects and could say a great deal more, but
//! nothing else survives the trip to an HTML overlay.

/// Where following a link goes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkAction {
    /// Somewhere outside the document.
    Uri(String),
    /// A named destination in this document, from `\ref`, `\cite` or a
    /// table of contents.
    Destination(String),
}

/// A rectangle on a page, in PostScript big points from its top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectBp {
    /// Left edge.
    pub left: f32,
    /// Top edge.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom edge.
    pub bottom: f32,
}

/// One clickable region of a page.
///
/// A link broken across a line break becomes several of these sharing one
/// action, which is what an overlay wants: each line gets its own box rather
/// than one box swallowing the paragraph between them.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    /// The region that responds to a click.
    pub rect: RectBp,
    /// Where following it goes.
    pub action: LinkAction,
}

/// A named spot a link can jump to.
#[derive(Clone, Debug, PartialEq)]
pub struct Destination {
    /// The name `\label` and its kin produced.
    pub name: String,
    /// Horizontal position in big points from the page's left edge.
    pub x_bp: f32,
    /// Vertical position in big points from the page's top edge.
    pub y_bp: f32,
}

/// Read the destination name out of a `pdf:dest` special.
///
/// The special reads `pdf:dest (name) [ ... ]`, where the name is a PDF string
/// and the array is the view to arrive at — which an HTML overlay cannot honour
/// and does not need, since it scrolls to the position the special was at.
pub fn parse_destination_name(args: &str) -> Option<String> {
    let rest = args.trim_start().strip_prefix("dest")?;
    let (name, _) = read_pdf_string(rest.trim_start())?;
    Some(name)
}

/// Read the action out of a `pdf:bann` annotation dictionary.
///
/// Returns `None` for an annotation that is not a link, or a link whose target
/// this cannot make sense of — an unresolved `\ref` produces exactly that, and
/// it is not worth an overlay that goes nowhere.
pub fn parse_link_action(args: &str) -> Option<LinkAction> {
    // `/S/URI/URI(https://example.com)`: the first `/URI` names the action
    // type and the second introduces the string. Requiring the string to
    // follow immediately is what tells them apart.
    if let Some(uri) = value_after_key(args, "/URI") {
        return Some(LinkAction::Uri(uri));
    }

    // `/S/GoTo/D(section.1)`, and `/Dest` for the spelling that omits the
    // action dictionary.
    for key in ["/D", "/Dest"] {
        if let Some(name) = value_after_key(args, key) {
            return Some(LinkAction::Destination(name));
        }
    }

    None
}

/// Find `key` where a PDF string follows it, and read that string.
///
/// A key whose value is a name rather than a string is skipped: this is looking
/// for `/URI(...)`, not for the `/URI` in `/S/URI`.
fn value_after_key(args: &str, key: &str) -> Option<String> {
    let mut from = 0;

    while let Some(at) = args[from..].find(key) {
        let after = from + at + key.len();
        from = after;

        // A longer key that merely starts with this one is a different key.
        if args[after..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        {
            continue;
        }

        if let Some((value, _)) = read_pdf_string(args[after..].trim_start()) {
            return Some(value);
        }
    }

    None
}

/// Read a parenthesised PDF string, returning it and what follows.
///
/// Handles the escapes a URL or a label can contain: an escaped parenthesis, an
/// escaped backslash, and the nested parentheses PDF allows unescaped as long
/// as they balance.
fn read_pdf_string(text: &str) -> Option<(String, &str)> {
    let rest = text.strip_prefix('(')?;
    let mut value = String::new();
    let mut depth = 1usize;
    let mut chars = rest.char_indices();

    while let Some((at, character)) = chars.next() {
        match character {
            '\\' => match chars.next() {
                // The escapes that matter here are the structural ones; PDF's
                // octal and control escapes do not appear in a label or a URL.
                Some((_, escaped)) => value.push(escaped),
                None => return None,
            },
            '(' => {
                depth += 1;
                value.push('(');
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((value, &rest[at + 1..]));
                }
                value.push(')');
            }
            _ => value.push(character),
        }
    }

    // An unterminated string is a malformed special, not an empty link.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_is_read_past_the_subtype_that_shares_its_name() {
        // `/S/URI` names the action; `/URI(...)` carries the target. Taking the
        // first match would find the former and read no string at all.
        let action = parse_link_action(
            "bann<</Type/Annot/Subtype/Link/Border[0 0 0]/A<</S/URI/URI(https://example.com/a)>>>>",
        );
        assert_eq!(
            action,
            Some(LinkAction::Uri("https://example.com/a".into()))
        );
    }

    #[test]
    fn an_internal_reference_names_its_destination() {
        let action =
            parse_link_action("bann<</Type/Annot/Subtype/Link/A<</S/GoTo/D(section.1)>>>>");
        assert_eq!(action, Some(LinkAction::Destination("section.1".into())));
    }

    #[test]
    fn a_destination_special_yields_its_name() {
        assert_eq!(
            parse_destination_name("dest (section.1) [@thispage /XYZ @xpos @ypos null]"),
            Some("section.1".into())
        );
    }

    #[test]
    fn a_url_containing_parentheses_survives() {
        // Wikipedia article titles do this constantly.
        let action = parse_link_action(
            r"bann<</A<</S/URI/URI(https://en.wikipedia.org/wiki/Bracket_\(disambiguation\))>>>>",
        );
        assert_eq!(
            action,
            Some(LinkAction::Uri(
                "https://en.wikipedia.org/wiki/Bracket_(disambiguation)".into()
            ))
        );
    }

    #[test]
    fn balanced_parentheses_need_no_escaping() {
        let action = parse_link_action("bann<</A<</S/URI/URI(https://x.test/a(b)c)>>>>");
        assert_eq!(action, Some(LinkAction::Uri("https://x.test/a(b)c".into())));
    }

    #[test]
    fn an_annotation_that_goes_nowhere_is_not_a_link() {
        assert_eq!(parse_link_action("bann<</Type/Annot/Subtype/Link>>"), None);
        assert_eq!(parse_link_action("bann<<>>"), None);
    }

    #[test]
    fn an_unterminated_string_is_refused() {
        assert_eq!(
            parse_link_action("bann<</A<</S/URI/URI(https://x.test"),
            None
        );
        assert_eq!(parse_destination_name("dest (unclosed"), None);
    }

    #[test]
    fn a_key_that_merely_starts_with_another_is_not_it() {
        // `/Domain` begins with `/D` but is not it.
        assert_eq!(parse_link_action("bann<</Domain(0 1)>>"), None);
    }
}
