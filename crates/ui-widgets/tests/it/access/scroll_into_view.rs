//! A screen reader's Scroll Into View shows its node and leaves keyboard focus where it was; a Focus request moves it.

use craft_ui::buttons::Button;
use craft_ui::contract::Show as _;
use craft_ui::input::{Key, Mods};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Harness, trial};

#[trial]
fn scroll_into_view_moves_no_focus() {
    let mut fixture = Harness::new_ui(|ui| {
        Button::new("First").show(ui);
        Button::new("Second").show(ui);
    });
    fixture.settle();
    fixture.key(Key::Tab, Mods::NONE).settle();
    let focused = fixture.focused_stop();
    assert!(focused.is_some(), "Tab focuses the first button");
    let second = fixture.by_label("Second").expect("the second button");
    fixture.access(AccessActionKind::ScrollIntoView, &second).settle();
    assert_eq!(fixture.focused_stop(), focused, "Scroll Into View moved no focus");
    fixture.access(AccessActionKind::Focus, &second).settle();
    assert_ne!(fixture.focused_stop(), focused, "a Focus request moves it");
}
