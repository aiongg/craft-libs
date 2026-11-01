//! Tests of the brush card parts and layout (task BR-BP1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// All seven part combinations: every part rect lies inside the card, none overlap, and the name
// spans the bottom row unless only the tip and the name show (then it sits in the stroke's cell).
#[test]
#[ignore = "skeleton"]
fn card_layout_places_tip_stroke_and_name() {
    todo!()
}

// At scale 1 a tip-only cell is 60 × 70 and a tip-and-name cell 150 × 70; a full card is 300 wide.
#[test]
#[ignore = "skeleton"]
fn tip_only_and_tip_and_name_cards_use_their_fixed_cells() {
    todo!()
}

// Heights and padding stop shrinking at scale 0.6 while widths keep shrinking; at 0.7 and below
// the number band is gone and the tip takes its cell.
#[test]
#[ignore = "skeleton"]
fn heights_floor_at_0_6_and_the_number_drops_at_0_7() {
    todo!()
}

// `set` refuses to clear the last part; `normalised` turns all three on when none is.
#[test]
#[ignore = "skeleton"]
fn at_least_one_part_stays_on() {
    todo!()
}

// `tip_thumb_scale`: 0.3 under 12 px, 1 from 40 px, size / 40 between; NaN gives 1.
#[test]
#[ignore = "skeleton"]
fn tip_thumbnails_scale_with_the_preset_size() {
    todo!()
}

// A card row's accessible name is "{name}, {size} px" with the size rounded; its label stays the name.
#[test]
#[ignore = "skeleton"]
fn card_rows_are_named_with_their_size() {
    todo!()
}
