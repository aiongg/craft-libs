//! Hex text for colours of three channels in `0..=1`.

/// The colour of `text`: six hex digits after an optional `#`, with spaces trimmed round it. No
/// sign, no shorthand, no alpha; text that isn't exactly that is `None`.
pub fn parse_hex(text: &str) -> Option<[f32; 3]> {
    let digits = text.trim().trim_start_matches('#');
    // A sign would pass `from_str_radix`, and a multi-byte character would split a byte range.
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(digits.get(i..i + 2)?, 16).ok().map(|v| f32::from(v) / 255.0);
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// `color` as lowercase `#rrggbb`; each channel is clamped to `0..=1` and rounded to a byte.
pub fn format_hex(color: [f32; 3]) -> String {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", byte(color[0]), byte(color[1]), byte(color[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_text_round_trips_and_rejects_what_is_not_six_digits() {
        assert_eq!(parse_hex(" #336699 "), Some([0.2, 0.4, 0.6]));
        assert_eq!(parse_hex("FF8800").map(format_hex).as_deref(), Some("#ff8800"));
        for bad in ["", "#fff", "+12345", "12345g", "1234567", "#33 6699", "３３６６９９", "é23456"] {
            assert_eq!(parse_hex(bad), None, "{bad:?}");
        }
        assert_eq!(format_hex([1.5, -1.0, 0.5]), "#ff0080");
    }
}
