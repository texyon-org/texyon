//! Page-size special parsing, independent of rendering.

pub(crate) const LETTER_BP: [f32; 2] = [612.0, 792.0];

pub(crate) fn parse_special(contents: &[u8]) -> Option<[f32; 2]> {
    let text = std::str::from_utf8(contents).ok()?.trim();
    if let Some(rest) = text.strip_prefix("pdf:pagesize") {
        let rest = rest.trim();
        if rest == "default" {
            return Some(LETTER_BP);
        }
        let mut width = None;
        let mut height = None;
        let words: Vec<_> = rest.split_whitespace().collect();
        for pair in words.windows(2) {
            match pair[0] {
                "width" => width = parse_dimension(pair[1]),
                "height" => height = parse_dimension(pair[1]),
                _ => {}
            }
        }
        return Some([width?, height?]);
    }
    let rest = text
        .strip_prefix("papersize=")
        .or_else(|| text.strip_prefix("papersize "))?;
    let (width, height) = rest.split_once(',')?;
    Some([
        parse_dimension(width.trim())?,
        parse_dimension(height.trim())?,
    ])
}

fn parse_dimension(value: &str) -> Option<f32> {
    let split = value
        .find(|ch: char| !ch.is_ascii_digit() && ch != '.' && ch != '-' && ch != '+')
        .unwrap_or(value.len());
    let number: f32 = value[..split].parse().ok()?;
    let scale = match value[split..].trim() {
        "bp" | "" => 1.0,
        "pt" => 72.0 / 72.27,
        "in" => 72.0,
        "cm" => 72.0 / 2.54,
        "mm" => 72.0 / 25.4,
        _ => return None,
    };
    let result = number * scale;
    Some(if result == 0.0 { 0.0 } else { result })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pdf_and_dvips_page_sizes() {
        assert_eq!(
            parse_special(b"pdf:pagesize width 612bp height 792bp"),
            Some(LETTER_BP)
        );
        assert_eq!(parse_special(b"papersize=8.5in,11in"), Some(LETTER_BP));
        assert_eq!(parse_special(b"pdf:pagesize default"), Some(LETTER_BP));
    }
}
