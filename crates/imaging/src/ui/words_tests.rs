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

#[test]
fn registered_labels_localized_with_identity_equal_their_english() {
    identity_holds::<crate::ui::adjust::AdjustLabels>();
    identity_holds::<crate::ui::adjust::stories::AdjustStoryLabels>();
    identity_holds::<crate::ui::liquify::LiquifyLabels>();
    identity_holds::<crate::ui::liquify::stories::LiquifyStoryLabels>();
    identity_holds::<crate::ui::filter_gallery::FilterGalleryLabels>();
    identity_holds::<crate::ui::filter_gallery::stories::FilterGalleryStoryLabels>();
    identity_holds::<crate::ui::wide_angle::WideAngleLabels>();
    identity_holds::<crate::ui::layer_style::LayerStyleLabels>();
    identity_holds::<crate::ui::layer_style::BlendIfBarLabels>();
    identity_holds::<crate::ui::layer_style::ContourFieldLabels>();
    identity_holds::<crate::ui::layer_style::stories::LayerStyleStoryLabels>();
    identity_holds::<crate::ui::blur_pins::BlurPinsLabels>();
    identity_holds::<crate::ui::blur_pins::stories::BlurPinsStoryLabels>();
    identity_holds::<crate::ui::fill::FillLabels>();
    identity_holds::<crate::ui::fill::stories::FillStoryLabels>();
    identity_holds::<crate::ui::live_preview::stories::PreviewStoryLabels>();
    identity_holds::<crate::ui::warp::perspective::stories::PerspectiveStoryLabels>();
    identity_holds::<crate::ui::warp::perspective::PerspectiveWarpLabels>();
    identity_holds::<crate::ui::warp::perspective::PerspectiveBarLabels>();
    identity_holds::<crate::ui::warp::fields_stories::WarpFieldsStoryLabels>();
    identity_holds::<crate::ui::warp::WarpFieldsLabels>();
    identity_holds::<crate::ui::warp::pin::stories::PinWarpStoryLabels>();
    identity_holds::<crate::ui::warp::pin::PinWarpLabels>();
    identity_holds::<crate::ui::warp::pin::PinWarpBarLabels>();
    identity_holds::<crate::ui::blend_keys_stories::BlendKeysStoryLabels>();
    identity_holds::<crate::ui::mask_view::MaskViewLabels>();
    identity_holds::<crate::ui::select::SelectLabels>();
    identity_holds::<crate::ui::select::stories::SelectToolStoryLabels>();
    identity_holds::<crate::ui::vanishing_point::VanishingPointLabels>();
    identity_holds::<crate::ui::vanishing_point::stories::VanishingPointStoryLabels>();
    identity_holds::<crate::ui::select_mask::SelectMaskLabels>();
    identity_holds::<crate::ui::select_mask::stories::SelectMaskStoryLabels>();
    identity_holds::<crate::ui::selection::stories::SelectionStoryLabels>();
    identity_holds::<crate::ui::selection::SelectionOverlayLabels>();
    identity_holds::<crate::ui::transform_preview::stories::TransformPreviewStoryLabels>();
}

#[test]
fn counted_words_are_registered() {
    let all: Vec<_> = crate::ui::WORDS.iter().flat_map(|table| table.iter().copied()).collect();
    assert_eq!(crate::ui::COUNTS.len(), 10);
    for count in crate::ui::COUNTS {
        assert!(all.contains(&count.word), "{:?} is not a registered word", count.word);
    }
}
