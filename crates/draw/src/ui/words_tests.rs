use std::sync::Arc;

use craft_i18n::{Embedded, I18n};
use craft_ui::localized::{Labels, Words};
use egui::Context;

use super::{
    BezierNames, CropLabels, CropNames, GradientLabels, GradientLineNames, PenNames, PerspectiveCropNames, TransformFieldLabels, TransformLabels,
    TransformNames, WORDS,
};
use crate::ui::gradient::counts;

static CATALOGS: &Embedded = &[("ru", "@plural\t{n} color stop|{n} color stops\t{n} цвет|{n} цвета|{n} цветов\n", "")];

fn identity_holds<L: Labels + Default + std::fmt::Debug>() {
    let shown = |labels: L| format!("{labels:?}");
    assert_eq!(shown(L::english().localized_in(&Words::new(|_| None))), shown(L::default()));
    assert_eq!(shown(L::english().localized_in(&Words::english())), shown(L::default()));
}

#[test]
fn labels_localized_with_identity_equal_their_english() {
    identity_holds::<GradientLineNames>();
    identity_holds::<GradientLabels>();
    identity_holds::<CropNames>();
    identity_holds::<CropLabels>();
    identity_holds::<PerspectiveCropNames>();
    identity_holds::<TransformNames>();
    identity_holds::<TransformLabels>();
    identity_holds::<TransformFieldLabels>();
}

#[test]
fn canvas_names_reach_their_fields_and_closures_fill_placeholders() {
    let by_word = Words::new(|word| Some(format!("<{}>{}", word.key(), word.english.find('{').map_or("", |at| &word.english[at..]))));
    let crop = format!("{:?}", CropNames::english().localized_in(&by_word));
    for word in CropNames::WORDS {
        assert_eq!(crop.matches(&format!("\"<{}>", word.key())).count(), 1, "{}", word.key());
    }
    let fields = TransformFieldLabels::english().localized_in(&by_word);
    assert_eq!((fields.skew_h.glyph.as_str(), fields.skew_v.name.as_str()), ("<transform.skewHGlyph>", "<transform.skewVName>"));

    assert_eq!(PenNames::english().anchor.as_ref()(3, 5), "Anchor 3 of 5");
    assert_eq!(BezierNames::english().anchor.as_ref()(1, 2), "Anchor 1 of 2");
}

#[test]
fn counted_words_are_registered() {
    let all: Vec<_> = WORDS.iter().flat_map(|table| table.iter().copied()).collect();
    for count in counts::ALL {
        assert!(all.contains(&count.word), "{:?} is not a registered word", count.word);
    }
}

#[test]
fn stop_counts_read_one_and_other_in_english() {
    let ctx = Context::default();
    let english = Words::english();
    let labels = GradientLabels::default();
    let colors = |n| english.counted(&ctx, counts::COLOR_STOPS, &labels.color_stops, n);
    let opacities = |n| english.counted(&ctx, counts::OPACITY_STOPS, &labels.opacity_stops, n);
    assert_eq!(format!("{}{}{}", colors(1), labels.stops_separator, opacities(2)), "1 color stop, 2 opacity stops");
    assert_eq!((colors(2), opacities(1)), ("2 color stops".to_owned(), "1 opacity stop".to_owned()));
}

#[test]
fn a_three_form_locale_picks_the_form_of_a_stop_count() {
    let ctx = Context::default();
    let (i18n, problems) = I18n::new(CATALOGS, &["ru"]);
    assert!(problems.is_empty(), "{problems:?}");
    let ru = i18n.from_code("ru").unwrap();
    let russian = Words::from_i18n(Arc::new(i18n), ru);
    let text = GradientLabels::default().color_stops;
    let count = |n| russian.counted(&ctx, counts::COLOR_STOPS, &text, n);
    assert_eq!((count(1), count(2), count(5)), ("1 цвет".to_owned(), "2 цвета".to_owned(), "5 цветов".to_owned()));
}

#[test]
fn registered_labels_localized_with_identity_equal_their_english() {
    identity_holds::<crate::ui::picture_fill::PictureFillLabels>();
    identity_holds::<crate::ui::picture_fill::stories::PictureStoryLabels>();
    identity_holds::<crate::ui::gradient::map::GradientMapLabels>();
    identity_holds::<crate::ui::connector::ConnectorToolLabels>();
    identity_holds::<crate::ui::connector::ConnectorBarLabels>();
    identity_holds::<crate::ui::connector::stories::ConnectorStoryLabels>();
    identity_holds::<crate::ui::slice::SliceToolLabels>();
    identity_holds::<crate::ui::slice::SliceLabels>();
    identity_holds::<crate::ui::slice::stories::SliceStoryLabels>();
    identity_holds::<crate::ui::pattern::PatternFillLabels>();
    identity_holds::<crate::ui::pattern::stories::PatternStoryLabels>();
    identity_holds::<crate::ui::brush_presets::BrushPickerLabels>();
    identity_holds::<crate::ui::brush_presets::BrushCardLabels>();
    identity_holds::<crate::ui::brush_presets::stories::BrushPresetStoryLabels>();
    identity_holds::<crate::ui::symmetry::SymmetryLabels>();
    identity_holds::<crate::ui::transform::session_stories::TransformSessionStoryLabels>();
    identity_holds::<crate::ui::measure::stories::MeasureStoryLabels>();
    identity_holds::<crate::ui::symmetry::stories::SymmetryStoryLabels>();
    identity_holds::<crate::ui::crop::stories_tool::CropStoryLabels>();
    identity_holds::<crate::ui::stroke::stories::StrokeStoryLabels>();
    identity_holds::<crate::ui::transform::TransformBarLabels>();
    identity_holds::<crate::ui::transform::TransformMenuLabels>();
    identity_holds::<crate::ui::crop::CropBarLabels>();
    identity_holds::<crate::ui::stroke::StrokeLabels>();
    identity_holds::<crate::ui::brush_settings::BrushSettingsLabels>();
    identity_holds::<crate::ui::brush_settings::stories::BrushSettingsStoryLabels>();
    identity_holds::<crate::ui::pen::PathOptionsLabels>();
    identity_holds::<crate::ui::add_beside_stories::AddBesideStoryLabels>();
    identity_holds::<crate::ui::measure::RulerToolLabels>();
    identity_holds::<crate::ui::measure::RulerBarLabels>();
    identity_holds::<crate::ui::shape_drag::ShapeDragToolLabels>();
    identity_holds::<crate::ui::shape_drag::CreateShapeLabels>();
    identity_holds::<crate::ui::shape_drag::stories::ShapeDragStoryLabels>();
    identity_holds::<crate::ui::artboard::ArtboardToolLabels>();
    identity_holds::<crate::ui::artboard::ArtboardLabels>();
    identity_holds::<crate::ui::artboard::stories::ArtboardStoryLabels>();
    identity_holds::<crate::ui::shape::ShapeLabels>();
    identity_holds::<crate::ui::shape::stories::ShapeStoryLabels>();
    identity_holds::<crate::ui::add_beside::AddBesideLabels>();
    identity_holds::<crate::ui::corner::CornerWidgetsLabels>();
}
