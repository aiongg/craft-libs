//! The craft `Id` keeps egui's values, so state saved under an id keeps its key across the move off egui.

use craft_ui_contract::contract::{Id, IdSalt};

#[test]
fn ids_keep_their_egui_values() {
    assert_eq!(Id::new("panel").value(), egui::Id::unique("panel").value());
    assert_eq!(Id::new(("row", 7_u64)).value(), egui::Id::unique(("row", 7_u64)).value());
    let (ours, theirs) = (Id::new("panel"), egui::Id::unique("panel"));
    assert_eq!(ours.with(3_usize).with("field").value(), theirs.with(3_usize).with("field").value());
    assert_eq!(Id::NULL.with("root").value(), egui::Id::NULL.with("root").value());
    assert_eq!(ours.with_salt(IdSalt::new("x")).value(), theirs.with_salt(egui::IdSalt::new("x")).value());
    // An id hashed inside a tuple hashes as its value, as egui's does.
    assert_eq!(Id::new((ours, 1_u32)).value(), egui::Id::unique((theirs, 1_u32)).value());
}
