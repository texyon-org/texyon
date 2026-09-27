//! Link and destination overlay recovered from `pdf:` annotation specials.

/// Where following a link goes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkAction {
    /// Somewhere outside the document.
    Uri(String),
    /// A named destination in this document.
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

/// One clickable region of a page, one per line of a broken link.
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
    /// Destination name.
    pub name: String,
    /// Horizontal position in big points from the page's left edge.
    pub x_bp: f32,
    /// Vertical position in big points from the page's top edge.
    pub y_bp: f32,
}

/// Name from `dest (name) [ ... ]`, ignoring the view array.
pub(crate) fn parse_destination_name(args: &str) -> Option<String> {
    let rest = args.trim_start().strip_prefix("dest")?;
    let (name, _) = read_pdf_string(rest.trim_start())?;
    Some(name)
}

/// Action of a `pdf:bann` dictionary, or `None` when it goes nowhere usable.
pub(crate) fn parse_link_action(args: &str) -> Option<LinkAction> {
    // In `/S/URI/URI(...)` only the second `/URI` is followed by a string.
    if let Some(uri) = value_after_key(args, "/URI") {
        return Some(LinkAction::Uri(uri));
    }

    for key in ["/D", "/Dest"] {
        if let Some(name) = value_after_key(args, key) {
            return Some(LinkAction::Destination(name));
        }
    }

    None
}

/// The PDF string following the first `key` that has one.
fn value_after_key(args: &str, key: &str) -> Option<String> {
    let mut from = 0;

    while let Some(at) = args[from..].find(key) {
        let after = from + at + key.len();
        from = after;

        // A longer key that merely starts with this one.
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

/// A parenthesized PDF string with balanced nesting, and what follows it.
fn read_pdf_string(text: &str) -> Option<(String, &str)> {
    let rest = text.strip_prefix('(')?;
    let mut value = String::new();
    let mut depth = 1usize;
    let mut chars = rest.char_indices();

    while let Some((at, character)) = chars.next() {
        match character {
            // Octal and control escapes do not occur in labels or URLs.
            '\\' => match chars.next() {
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

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_is_read_past_the_subtype_that_shares_its_name() {
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
        assert_eq!(parse_link_action("bann<</Domain(0 1)>>"), None);
    }
}
