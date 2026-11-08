//! The on-screen keyboard pane.

use craft_keymap::Platform;
use craft_ui::testing::trial;

use super::fixture::Rig;

/// A key is named by its holders, the modifier toggles change which holders it shows, and Assign to Selected gives
/// the selected command the chosen key.
#[trial]
fn the_keyboard_names_holders_and_assigns_to_the_selected_command() {
    let mut rig = Rig::tuned(Platform::Mac, |options| options.keyboard = true);
    assert!(rig.has("M: Marquee Tool, Elliptical Marquee Tool"));
    assert!(rig.has("L: Loop (Timeline)"));
    assert!(rig.has("F7"), "a key nothing holds is named by its own name");

    rig.click("⌘ Command");
    assert!(rig.has("⌘S: Save"), "{:?}", rig.names_with(craft_ui::contract::Role::BUTTON, "S"));
    assert!(rig.has("⌘B: Bold (Text)"));
    rig.click("⌘ Command");

    rig.click("L: Loop (Timeline)");
    assert!(rig.has("Key: L"));
    rig.click("Save");
    rig.click("Assign to Selected");
    assert_eq!(
        rig.bindings("file.save"),
        ["Cmd+S", "L"],
        "buttons {:?}, rows {:?}",
        rig.names_with(craft_ui::contract::Role::BUTTON, ""),
        rig.names_with(craft_ui::contract::Role::ROW, "")
    );
    assert!(rig.has("L: Save, Loop (Timeline)"));
}
