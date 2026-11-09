//! How modifier keys pick the selection operation.

use craft_behavior::CursorBadge;
use craft_behavior::geometry::SelectionOperation::{self, Add, Intersect, New, Subtract};
use craft_keymap::Mods;

#[test]
fn alt_with_nothing_selected_stays_new_but_a_chosen_bar_mode_is_kept() {
    let both = Mods { shift: true, alt: true, ..Mods::NONE };
    assert_eq!(SelectionOperation::resolve(Subtract, Mods::SHIFT, true), Add);
    assert_eq!(SelectionOperation::resolve(New, Mods::ALT, true), Subtract);
    assert_eq!(SelectionOperation::resolve(New, Mods::ALT, false), New);
    assert_eq!(SelectionOperation::resolve(New, both, false), New);
    assert_eq!(SelectionOperation::resolve(Subtract, Mods::NONE, false), Subtract);
    assert_eq!(SelectionOperation::resolve(Intersect, Mods::NONE, false), Intersect);
}

