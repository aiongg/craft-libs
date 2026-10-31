//! The id the colour picker shows under is the one `Interactive::id_in` gives its salt, so a host
//! that reads focus or state before showing it finds the same widget.

use egui::{Context, Pos2, RawInput, Rect, vec2};

use super::ColorPicker;
use craft_ui::localized::LocaleNumbers;
use craft_ui::primitives::Interactive;
use craft_ui::theme::{Base, install};

#[test]
fn id_in_is_the_shown_id() {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let numbers = LocaleNumbers::from_tag("en");
    for _ in 0..2 {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), ..Default::default() };
        ctx.run_ui(input, |ui| {
            let picker = Interactive::id_in(ui, "picker");
            let shown = ColorPicker::new("picker", "Colour", craft_theme::Srgba::rgb(0.2, 0.4, 0.6), &numbers).show(ui).edit.response.id;
            assert_eq!(shown, picker, "ColorPicker");
        })
        .textures_delta
        .clear();
    }
}
