//! Tests of the settings and the dialog's samples (L2).

use craft_theme::Size;
use craft_ui::list::{ListRow, ThumbSize};
use craft_ui::theme::{Base, Tokens, TokensOf as _, install, scope};
use egui::Context;

use super::*;
use crate::ui::panel::LayersStyle;

// `LayersSettings::default()` is Medium, Entire Document, filters shown, the first four switches on,
// badges off; a section with missing fields loads with those defaults; a JSON round trip keeps every
// field.
#[test]
fn settings_defaults_missing_fields_and_round_trip() {
    let defaults = LayersSettings::default();
    assert_eq!(defaults.options.thumbnail_size, ThumbSize::Medium);
    assert_eq!(defaults.options.thumbnail_contents, ThumbContents::EntireDocument);
    assert!(defaults.show_filters);
    let o = &defaults.options;
    assert!(
        o.use_default_masks_on_fill_layers && o.expand_new_effects && o.add_copy_to_copied_layers_and_groups && o.preview_on_canvas_when_reordering
    );
    assert!(!o.show_layer_mask_badges);

    let partial: LayersSettings =
        serde_json::from_str(r#"{"options":{"thumbnail_size":"Large","expand_new_effects":false}}"#).expect("partial section loads");
    assert_eq!(partial.options.thumbnail_size, ThumbSize::Large);
    assert!(!partial.options.expand_new_effects);
    assert!(partial.show_filters && partial.options.use_default_masks_on_fill_layers, "missing fields take their defaults");
    assert_eq!(serde_json::from_str::<LayersSettings>("{}").expect("empty section loads"), defaults);

    let changed = LayersSettings {
        options: LayersOptions {
            thumbnail_size: ThumbSize::None,
            thumbnail_contents: ThumbContents::LayerBounds,
            use_default_masks_on_fill_layers: false,
            expand_new_effects: false,
            add_copy_to_copied_layers_and_groups: false,
            preview_on_canvas_when_reordering: false,
            show_layer_mask_badges: true,
        },
        show_filters: false,
    };
    let json = serde_json::to_string(&changed).expect("serializes");
    assert_eq!(serde_json::from_str::<LayersSettings>(&json).expect("round trip"), changed);
}

// Each size's sample is twice the row thumbnail of that size at Compact and at Comfortable; None has
// no sample.
#[test]
fn option_samples_are_twice_the_row_thumbnails() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut seen = Vec::new();
    for size in [Size::Compact, Size::Comfortable] {
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            scope(
                ui,
                |o| o.size = Some(size),
                |ui| {
                    let tokens = Tokens::of(ui);
                    let (row, layers) = (ListRow::<u8>::style(&tokens), LayersStyle::of(&tokens));
                    assert_eq!(sample_side(&row, &layers, ThumbSize::None), 0.0);
                    for thumb in [ThumbSize::Small, ThumbSize::Medium, ThumbSize::Large] {
                        let side = sample_side(&row, &layers, thumb);
                        assert_eq!(side, 2.0 * row.thumb_side(thumb), "{size:?} {thumb:?}");
                        seen.push(side);
                    }
                },
            );
        });
        output.drop_without_applying_deltas();
    }
    assert_eq!(seen.len(), 6, "both presets were checked");
}
