//! `HexCodec`: which texts are colours and what a colour reads back as.

use craft_behavior::Codec;
use craft_theme::Rgba;
use craft_ui::testing::trial;
use craft_ui::text::HexCodec;

const ORANGE: Rgba = Rgba([0xff, 0x88, 0x00, 0xff]);
const TRANSLUCENT: Rgba = Rgba([0x10, 0x20, 0x30, 0x80]);

fn parse(text: &str) -> Option<Rgba> {
    HexCodec::default().parse(text, &TRANSLUCENT).ok()
}

/// Six digits, three digits, with or without the hash and in either case read as the same colour, and the colour keeps
/// the alpha it had when editing began; eight digits carry their own alpha; anything else is not a colour.
#[trial]
fn hex_text_parses_every_form_and_refuses_the_rest() {
    let opaque = |r, g, b| Some(Rgba([r, g, b, 0x80]));
    let mut cases = vec![
        ("#FF8800", opaque(0xff, 0x88, 0x00)),
        ("ff8800", opaque(0xff, 0x88, 0x00)),
        ("#f80", opaque(0xff, 0x88, 0x00)),
        ("  #Abc ", opaque(0xaa, 0xbb, 0xcc)),
        ("#ff880040", Some(Rgba([0xff, 0x88, 0x00, 0x40]))),
    ];
    cases.extend(["", "#", "#12", "#12345", "#1234567", "#gg0000", "+f8000", "#ff 880"].map(|text| (text, None)));
    for (text, colour) in cases {
        assert_eq!(parse(text), colour, "{text:?}");
    }
}

/// A colour formats as `#rrggbb` and reads back as itself; a translucent one keeps its alpha.
#[trial]
fn a_colour_round_trips_through_its_text() {
    assert_eq!(HexCodec::default().format(&ORANGE), "#ff8800");
    for colour in [ORANGE, TRANSLUCENT] {
        let codec = HexCodec::default();
        assert_eq!(codec.parse(&codec.format(&colour), &ORANGE), Ok(colour));
    }
}
