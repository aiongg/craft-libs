//! Hidden characters.

use super::marks::*;

// Space, tab, U+2028 and U+00A0 map to · » ¬ °; a letter has no mark.
#[test]
fn characters_map_to_their_marks() {
    let marks: Vec<_> = [' ', '\t', '\u{2028}', '\u{A0}'].into_iter().map(|c| HiddenMark::of(c).map(HiddenMark::glyph)).collect();
    assert_eq!(marks, [Some("·"), Some("»"), Some("¬"), Some("°")]);
    assert_eq!(HiddenMark::of('a'), None);
    assert_eq!(HiddenMark::of('\n'), None);
}
