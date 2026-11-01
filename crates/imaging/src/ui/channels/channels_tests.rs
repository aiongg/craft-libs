//! Behaviour tests of the Channels panel over fake rows (XT, after CH1 and CH2).

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click on a row reports `Target`; Shift-click too.
#[test]
#[ignore = "skeleton"]
fn channels_click_targets_row() {
    todo!()
}

// Cmd/Ctrl-click reports `LoadSelection`: new, Shift add, Alt subtract, Shift+Alt intersect; no
// `Target`.
#[test]
#[ignore = "skeleton"]
fn channels_command_click_loads_selection() {
    todo!()
}

// A sweep down the eyes is one `Visibility` gesture from `Begin` to `Commit`; Esc cancels it.
#[test]
#[ignore = "skeleton"]
fn channels_eye_sweep_is_one_gesture() {
    todo!()
}

// A double click on an alpha row's thumbnail reports `Options`; on the composite it reports
// nothing.
#[test]
#[ignore = "skeleton"]
fn channels_double_click_thumbnail_opens_options() {
    todo!()
}

// A double click on an alpha row's name opens the rename; composite and colour rows don't rename.
#[test]
#[ignore = "skeleton"]
fn channels_double_click_name_renames_alpha_only() {
    todo!()
}

// An alpha row dragged between alpha rows or above Quick Mask reports `Reorder`; over colour and
// mask rows no marker shows and nothing is reported.
#[test]
#[ignore = "skeleton"]
fn channels_alpha_drag_reorders_and_refuses_other_rows() {
    todo!()
}

// Rows dropped on New, Delete and Load report `FooterDrop` with the key; a colour row on Delete
// reports nothing.
#[test]
#[ignore = "skeleton"]
fn channels_drop_on_footer_reports_button_and_key() {
    todo!()
}

// The thumbnail side is 28 at Compact and 30 at Comfortable; `.thumbnail_size` overrides it
// (the `.thumbnail_size` override is checked with the rows).
#[test]
fn channels_thumbnail_side_follows_the_size_preset() {
    use craft_theme::{Appearance, Contrast, Mode, Size, TextSize, builtins, resolve};
    use craft_ui::theme::{FontFamilies, UiTokens};

    let side = |size| {
        let appearance = Appearance { mode: Mode::Light, contrast: Contrast::Standard, size, text_size: TextSize::Small, reduced_motion: false };
        let theme = builtins().iter().find(|t| t.id.as_str() == "craft.studio").cloned().unwrap();
        let families = FontFamilies {
            ui: egui::FontFamily::Proportional,
            ui_semibold: egui::FontFamily::Proportional,
            mono: egui::FontFamily::Monospace,
            optical_size: false,
        };
        super::ChannelsStyle::of(&UiTokens::new(resolve(&theme, appearance, None).tokens, appearance, Mode::Light, families)).thumb
    };
    assert_eq!((side(Size::Compact), side(Size::Comfortable)), (28.0, 30.0));
}

// The chords read Cmd+2 for the composite, Cmd+3.. for colours, then alphas while at most 9; Quick
// Mask and the layer mask have none.
#[test]
#[ignore = "skeleton"]
fn channels_shortcut_labels_follow_row_order() {
    todo!()
}

// Cmd/Ctrl+Enter on the lead row reports `LoadSelection`.
#[test]
#[ignore = "skeleton"]
fn channels_keyboard_loads_selection() {
    todo!()
}

// Spot, Quick Mask and layer-mask rows draw their names italic.
#[test]
#[ignore = "skeleton"]
fn channels_layer_mask_row_is_italic() {
    todo!()
}

// Adding and removing channels keeps the other rows' textures (same `TextureId`).
#[test]
#[ignore = "skeleton"]
fn channels_rows_update_in_place() {
    todo!()
}
