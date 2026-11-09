//! The foreign import review.

use craft_keymap::{ContextRule, Flavor, ForeignEntry, ForeignKey, ForeignSet, ImportOptions, Profiles};
use craft_ui::testing::trial;

use super::fixture::Rig;

fn theirs() -> ForeignSet {
    let read = |text: &str| ForeignKey::read(Some("Default"), text, Flavor::Windows);
    ForeignSet::new(vec![
        ForeignEntry::new(Some("File"), ["Save"]).with_key(read("Ctrl+Alt+S")).at_line(3),
        ForeignEntry::new(Some("File"), ["Frobnicate"]).with_key(read("Ctrl+F")).at_line(4),
        ForeignEntry::new(Some("File"), ["Export…"]).with_key(read("Ctrl+Blorp")).at_line(5),
        ForeignEntry::new(Some("Object"), ["Arrange", "Bring to Front"]).with_key(read("Num 7")).at_line(6),
        ForeignEntry::new(Some("File"), ["Sve Az"]).with_key(read("Ctrl+F9")).at_line(7),
    ])
    .with_name("Theirs")
}

fn open_review(rig: &mut Rig) {
    let options = ImportOptions { contexts: vec![("Default".into(), ContextRule::Home)], suggest: true, ..ImportOptions::default() };
    rig.review_import(theirs(), options);
}

/// Picking a suggestion replans the import and moves the item to Will change.
#[trial]
fn picking_a_suggestion_replans() {
    let mut rig = Rig::mac_with_sets(Profiles::default());
    open_review(&mut rig);
    assert!(rig.has("Command for File > Sve Az"));
    rig.click("Command for File > Sve Az");
    rig.click("Save As…");
    assert!(!rig.has("Command for File > Sve Az"), "the pick moved to Will change");
}
