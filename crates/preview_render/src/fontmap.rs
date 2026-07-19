//! Minimal pdftex-style font map parsing for classic TeX fonts.

use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub(crate) struct FontMapEntry {
    pub(crate) program: Option<String>,
    pub(crate) encoding: Option<String>,
}

#[derive(Default)]
pub(crate) struct FontMap {
    entries: BTreeMap<String, FontMapEntry>,
}

impl FontMap {
    pub(crate) fn parse_into(&mut self, bytes: &[u8]) {
        for line in String::from_utf8_lossy(bytes).lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(['%', '#']) {
                continue;
            }
            let Some((name, entry)) = parse_line(line) else {
                continue;
            };
            self.entries.entry(name).or_insert(entry);
        }
    }

    pub(crate) fn get(&self, name: &str) -> Option<&FontMapEntry> {
        self.entries.get(name)
    }
}

fn parse_line(line: &str) -> Option<(String, FontMapEntry)> {
    let mut outside_quotes = String::with_capacity(line.len());
    let mut quoted = false;
    for character in line.chars() {
        if character == '"' {
            quoted = !quoted;
            outside_quotes.push(' ');
        } else if !quoted {
            outside_quotes.push(character);
        }
    }
    let mut tokens = outside_quotes.split_whitespace();
    let name = tokens.next()?.to_owned();
    let mut entry = FontMapEntry::default();
    for token in tokens {
        let Some(file) = token.strip_prefix('<') else {
            continue;
        };
        let file = file.trim_start_matches(['<', '[']);
        if file.is_empty() {
            continue;
        }
        if file.to_ascii_lowercase().ends_with(".enc") {
            entry.encoding.get_or_insert_with(|| file.to_owned());
        } else {
            entry.program.get_or_insert_with(|| file.to_owned());
        }
    }
    Some((name, entry))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reencoding_and_partial_download_markers() {
        let mut map = FontMap::default();
        map.parse_into(
            b"% comment\nec-lmr10 LMRoman10-Regular \"enclmec ReEncodeFont\" <lm-ec.enc <lmr10.pfb\nfoo Foo <[foo.enc <<foo.pfb\n",
        );
        let latin_modern = map.get("ec-lmr10").unwrap();
        assert_eq!(latin_modern.encoding.as_deref(), Some("lm-ec.enc"));
        assert_eq!(latin_modern.program.as_deref(), Some("lmr10.pfb"));
        let foo = map.get("foo").unwrap();
        assert_eq!(foo.encoding.as_deref(), Some("foo.enc"));
        assert_eq!(foo.program.as_deref(), Some("foo.pfb"));
    }
}
