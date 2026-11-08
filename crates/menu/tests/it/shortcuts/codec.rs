//! `ChordCodec`: the text form of a binding.

use craft_behavior::{Codec, NotAValue};
use craft_keymap::{Binding, Platform};
use craft_menu::shortcuts::ChordCodec;
use craft_ui::testing::trial;

/// A binding's text parses back to the binding, empty text is no binding and text that is not a shortcut is refused.
#[trial]
fn a_binding_round_trips_through_its_text_and_a_bad_text_is_refused() {
    let codec = ChordCodec::new(Platform::Mac);
    let binding = Binding::parse("Cmd+Shift+K Cmd+Shift+K").expect("a binding");
    let text = codec.format(&Some(binding.clone()));
    assert_eq!(codec.parse(&text, &None), Ok(Some(binding)));
    assert_eq!(codec.format(&None), "");
    assert_eq!(codec.parse("  ", &None), Ok(None));
    assert_eq!(codec.parse("Cmd+", &None), Err(NotAValue));
}
