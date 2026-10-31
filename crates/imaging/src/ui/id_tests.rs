//! The id a widget shows under is the one its `id_in` gives its salt, so a host that reads focus
//! or state before showing it finds the same widget.

use egui::{Context, Pos2, RawInput, Rect, Ui, vec2};

use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

use crate::ui::{Levels, LevelsEditor};

fn run(show: impl FnMut(&mut Ui)) {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut show = show;
    for _ in 0..2 {
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))), ..Default::default() };
        ctx.run_ui(input, &mut show).textures_delta.clear();
    }
}

#[test]
fn id_in_is_the_shown_id() {
    let numbers = LocaleNumbers::from_tag("en");
    let levels = Levels::identity(0.0..=255.0);
    run(|ui| {
        let levels_id = LevelsEditor::id_in(ui, "levels");
        assert_eq!(LevelsEditor::new("levels", "Levels", &levels, &numbers).show(ui).response.id, levels_id, "LevelsEditor");
    });
}
