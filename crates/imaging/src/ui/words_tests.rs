use craft_ui::localized::{Labels, Words};

use crate::ui::{LevelsLabels, WarpNames, WarpSide};

fn identity_holds<L: Labels + Default + std::fmt::Debug>() {
    let shown = |labels: L| format!("{labels:?}");
    assert_eq!(shown(L::english().localized_in(&Words::new(|_| None))), shown(L::default()));
    assert_eq!(shown(L::english().localized_in(&Words::english())), shown(L::default()));
}

#[test]
fn levels_labels_localized_with_identity_equal_their_english() {
    identity_holds::<LevelsLabels>();
}

#[test]
fn warp_names_fill_placeholders() {
    let warp = WarpNames::english();
    assert_eq!((warp.anchor)(2, 3), "Row 2, column 3");
    assert_eq!((warp.handle)(2, 3, WarpSide::Right), "Row 2, column 3, right handle");
    let german = Words::new(|word| (word.key() == "warp.handleUp").then(|| "Zeile {row}, Spalte {column}, oben".to_owned()));
    assert_eq!((WarpNames::english().localized_in(&german).handle)(1, 2, WarpSide::Up), "Zeile 1, Spalte 2, oben");
}
