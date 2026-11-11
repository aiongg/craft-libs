//! Bytes to text, for the character sets messages still arrive in.

use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252};

/// Text from bytes in a named character set.
///
/// Knows every character set of the WHATWG Encoding Standard by any of its names (UTF-8, the
/// Windows and ISO-8859 sets, KOI8, Shift_JIS, EUC-JP, ISO-2022-JP, GBK, Big5, EUC-KR, UTF-16…),
/// as a browser does. ISO-8859-1 is read as Windows-1252, its superset, because text that says
/// Latin-1 very often holds curly quotes and euro signs (the standard does the same). A missing
/// name, or US-ASCII with bytes above 0x7F, is guessed with [`guess`]. A name the standard doesn't
/// know, or one it reads as "replacement" (ISO-2022-KR, HZ…), is read as UTF-8, with U+FFFD for
/// what doesn't fit.
pub fn decode(bytes: &[u8], label: &str) -> String {
    let label = label.trim().trim_matches(['"', '\'']).trim().to_ascii_lowercase();
    if matches!(label.as_str(), "" | "us-ascii" | "ascii" | "ansi_x3.4-1968" | "iso646-us") {
        return guess(bytes);
    }
    let encoding = encoding(&label).unwrap_or(UTF_8);
    // ASCII reads the same in every set that keeps it; UTF-16 and ISO-2022-JP don't.
    if bytes.is_ascii() && encoding.is_ascii_compatible() {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    if encoding == UTF_16LE || encoding == UTF_16BE {
        // "UTF-16" names no byte order (it is read as little-endian): a byte order mark says which.
        return encoding.decode(bytes).0.into_owned();
    }
    // Elsewhere a byte order mark is text like any other (the reader drops one that opens a body).
    encoding.decode_without_bom_handling(bytes).0.into_owned()
}

/// The character set a name means: the standard's own names, and a few more that mail uses.
/// None for a name it doesn't know and for its "replacement" sets.
fn encoding(label: &str) -> Option<&'static Encoding> {
    let label = match label {
        "latin-1" => "latin1",
        "latin9" | "latin-9" => "iso-8859-15",
        "latin-2" => "latin2",
        "cp-1250" => "windows-1250",
        "cp-1252" => "windows-1252",
        other => other,
    };
    Encoding::for_label_no_replacement(label.as_bytes())
}

/// Text from bytes that don't say what they are: UTF-8 when they are valid UTF-8, else Windows-1252.
pub fn guess(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => WINDOWS_1252.decode_without_bom_handling(bytes).0.into_owned(),
    }
}

/// Line endings as `\n`: CRLF and a lone CR both become LF.
pub fn unix_lines(text: &str) -> String {
    if !text.contains('\r') {
        return text.to_string();
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn other_sets_of_the_standard() {
        assert_eq!(decode(b"\xD0\xD2\xC9\xD7\xC5\xD4", "koi8-r"), "привет");
        assert_eq!(decode(b"\x93\xFA\x96\x7B", "Shift_JIS"), "日本");
        // ISO-2022-JP is seven-bit, and still not ASCII.
        assert_eq!(decode(b"\x1B$BF|K\\\x1B(B", "iso-2022-jp"), "日本");
        assert_eq!(decode(b"h\0i\0", "utf-16le"), "hi");
        assert_eq!(decode(b"\xFE\xFF\0h\0i", "utf-16"), "hi");
        assert_eq!(decode(b"\xA4", "latin9"), "€");
    }
}
