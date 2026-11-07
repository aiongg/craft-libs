//! `HexCodec`: which texts are colours and what a colour reads back as.

use craft_behavior::Codec;
use craft_theme::Rgba;
use craft_ui::testing::trial;
use craft_ui::text::HexCodec;

const ORANGE: Rgba = Rgba([0xff, 0x88, 0x00, 0xff]);
const TRANSLUCENT: Rgba = Rgba([0x10, 0x20, 0x30, 0x80]);

fn parse(text: &str) -> Option<Rgba> {
    HexCodec.parse(text, &TRANSLUCENT).ok()
}

/// Six digits, three digits, with or without the hash and in either case read as the same colour, and the colour
/// keeps the alpha it had when editing began.
#[trial]
fn hex_text_reads_with_or_without_hash_and_shorthand() {
    let opaque = |r, g, b| Some(Rgba([r, g, b, 0x80]));
    assert_eq!(parse("#FF8800"), opaque(0xff, 0x88, 0x00));
    assert_eq!(parse("ff8800"), opaque(0xff, 0x88, 0x00));
    assert_eq!(parse("#f80"), opaque(0xff, 0x88, 0x00));
    assert_eq!(parse("  #Abc "), opaque(0xaa, 0xbb, 0xcc));
}

/// Eight digits carry their own alpha; anything else is not a colour.
#[trial]
fn hex_text_with_alpha_wins_and_other_text_is_refused() {
    assert_eq!(parse("#ff880040"), Some(Rgba([0xff, 0x88, 0x00, 0x40])));
    for text in ["", "#", "#12", "#12345", "#1234567", "#gg0000", "+f8000", "#ff 880"] {
        assert_eq!(parse(text), None, "{text:?}");
    }
}

/// A colour formats as `#rrggbb` and reads back as itself; a translucent one keeps its alpha.
#[trial]
fn a_colour_round_trips_through_its_text() {
    assert_eq!(HexCodec.format(&ORANGE), "#ff8800");
    for colour in [ORANGE, TRANSLUCENT] {
        assert_eq!(HexCodec.parse(&HexCodec.format(&colour), &ORANGE), Ok(colour));
    }
}
