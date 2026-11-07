//! The chip field's draft rules and the alt text flag rule.

use craft_behavior::Codec as _;
use craft_ui::testing::trial;
use craft_ui::text::{AltText, ChipCodec};

fn chips(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

/// A draft names one chip per piece between separators, trimmed; pieces already in the list and empty pieces add
/// nothing, and the new chips follow the old ones in the order typed.
#[trial]
fn a_draft_adds_its_pieces_after_the_existing_chips() {
    let codec = ChipCodec::default();
    let start = chips(&["kite"]);
    assert_eq!(codec.parse("red, beach ;kite,, \n sky", &start), Ok(chips(&["kite", "red", "beach", "sky"])));
    assert_eq!(codec.parse("  ,; ", &start), Ok(start.clone()), "a draft of separators adds nothing");
    assert_eq!(codec.parse("red, red", &Vec::new()), Ok(chips(&["red"])), "a piece the draft names twice is added once");
    assert_eq!(codec.parse("two words", &Vec::new()), Ok(chips(&["two words"])), "a space does not end a chip");
}

/// The text part shows only the draft: committing leaves it empty, whatever the chips are.
#[trial]
fn the_draft_text_is_empty_after_a_commit() {
    let codec = ChipCodec::default();
    let value = chips(&["kite", "red"]);
    assert_eq!(codec.format(&value), "");
    assert_eq!(codec.normalise("red", &value), "");
}

/// Mark as decorative starts checked only when there is at least one object and every object is decorative.
#[trial]
fn decorative_starts_checked_only_when_every_object_is() {
    assert!(!AltText::starts_decorative([]));
    assert!(!AltText::starts_decorative([true, false]));
    assert!(!AltText::starts_decorative([false]));
    assert!(AltText::starts_decorative([true]));
    assert!(AltText::starts_decorative([true, true, true]));
}
