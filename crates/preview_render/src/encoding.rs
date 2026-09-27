//! Type1 built-in encoding extraction for fonts without an external `.enc`.

use crate::Resource;

pub(crate) fn type1_builtin(bytes: &[u8]) -> Option<Resource> {
    let cleartext = if bytes.len() >= 6 && bytes[..2] == [0x80, 0x01] {
        let length = u32::from_le_bytes(bytes[2..6].try_into().ok()?) as usize;
        bytes.get(6..6 + length)?
    } else {
        bytes
    };
    let text = String::from_utf8_lossy(cleartext);
    let body = text.get(text.find("/Encoding")?..)?;
    let body = &body[..body.find(" def").unwrap_or(body.len())];
    if body.contains("StandardEncoding") {
        return None;
    }
    let mut names = vec![".notdef".to_owned(); 256];
    let mut found = false;
    // A `/` starts a name even when glued on, as in hfbright's `dup 0/Gamma put`.
    let body = body.replace('/', " /");
    let mut tokens = body.split_whitespace();
    while let Some(token) = tokens.next() {
        if token != "dup" {
            continue;
        }
        let (Some(code), Some(name), Some(put)) = (tokens.next(), tokens.next(), tokens.next())
        else {
            break;
        };
        let (Ok(code), Some(name)) = (code.parse::<usize>(), name.strip_prefix('/')) else {
            continue;
        };
        if code < names.len() && put == "put" {
            names[code] = name.to_owned();
            found = true;
        }
    }
    found.then(|| Resource::from_bytes(format!("[ /{} ]", names.join(" /")).into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_vector_with_glued_name_tokens() {
        let resource = type1_builtin(
            b"/Encoding 256 array\n 0 1 255 { 1 index exch /.notdef put} for\ndup 0/Gamma put\ndup 1/Delta put\nreadonly def",
        )
        .unwrap();
        let text = std::str::from_utf8(&resource.bytes).unwrap();
        assert_eq!(text.split_whitespace().nth(1), Some("/Gamma"));
        assert_eq!(text.split_whitespace().nth(2), Some("/Delta"));
    }

    #[test]
    fn extracts_custom_type1_vector() {
        let resource = type1_builtin(
            b"/Encoding 256 array\ndup 15 /bullet put\ndup 3 /asteriskmath put\nreadonly def",
        )
        .unwrap();
        let text = std::str::from_utf8(&resource.bytes).unwrap();
        assert_eq!(text.split_whitespace().nth(4), Some("/asteriskmath"));
        assert_eq!(text.split_whitespace().nth(16), Some("/bullet"));
    }
}
