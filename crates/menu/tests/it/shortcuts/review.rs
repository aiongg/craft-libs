//! The foreign import review.

use craft_keymap::{ContextRule, Flavor, ForeignEntry, ForeignKey, ForeignSet, ImportOptions, Platform};
use craft_ui::testing::trial;

use super::recorder::Rig;

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
    rig.shared.borrow_mut().review = Some((theirs(), options));
    rig.fixture.settle();
}

/// The review lists what changes and what did not match with its line, changes nothing until Import, and after
/// Import reports what was done.
#[trial]
fn the_foreign_import_review_lists_skipped_and_unreadable_and_imports_on_request() {
    let mut rig = Rig::new(Platform::Mac, |options| options.sets = true);
    open_review(&mut rig);
    assert!(rig.has("Import “Theirs”"));
    assert!(rig.shared.borrow().keymap.overrides().is_empty(), "the review changes nothing");
    rig.click("Import as New Set");
    assert!(rig.has("Imported as “Theirs”"));
    rig.click("Done");
    assert!(rig.has("Commands"));
}

/// Picking a suggestion replans the import and moves the item to Will change.
#[trial]
fn picking_a_suggestion_replans() {
    let mut rig = Rig::new(Platform::Mac, |options| options.sets = true);
    open_review(&mut rig);
    assert!(rig.has("Command for File > Sve Az"));
    rig.click("Command for File > Sve Az");
    rig.click("Save As…");
    assert!(!rig.has("Command for File > Sve Az"), "the pick moved to Will change");
}
