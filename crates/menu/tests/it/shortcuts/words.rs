//! The dialog's words: chord text in the scope's key names.

use craft_i18n::{WordSet as _, Words};
use craft_keymap::{Chord, Platform, Trigger};
use craft_menu::shortcuts::trigger_text;
use craft_ui::display::DisplayWords;
use craft_ui::testing::trial;

/// A key name from the catalog replaces the key's English name; a double press is written twice; macOS glyphs
/// need no translation.
#[trial]
fn trigger_text_uses_localized_key_names() {
    let german = DisplayWords::english().localized_in(&Words::new(|w| (w.id == "kbd.delete").then(|| "Entf".to_owned())));
    let english = DisplayWords::english();
    let delete = Trigger::parse("Cmd+Shift+Delete").expect("a trigger");
    assert_eq!(trigger_text(&german, &delete, Platform::Windows), "Ctrl+Shift+Entf");
    assert_eq!(trigger_text(&german, &Trigger::parse("F7 F7").expect("a trigger"), Platform::Windows), "F7 F7");
    assert_eq!(trigger_text(&english, &Trigger::single(Chord::parse("⇧⌘Z").expect("a chord")), Platform::Mac), "⇧⌘Z");
    assert_eq!(trigger_text(&german, &delete, Platform::Mac), "⇧⌘⌦", "a glyph is not translated");
}
