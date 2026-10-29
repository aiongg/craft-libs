//! The gallery's appearance setup, the path `Gallery` runs before it calls `install`, conforms to
//! the design system's reference for every built-in theme, mode, contrast and size.

use craft_theme::{ContrastChoice, InterfaceFont, ModeChoice, Size, TextSize};
use craft_ui::conformance::{Case, Conformance};
use craft_ui::direction::Direction;
use craft_ui::theme::install;
use craft_ui_gallery::{Appearance, faces};
use egui::Context;

fn ui_face(font: InterfaceFont) -> &'static str {
    match font {
        InterfaceFont::Suite => faces::UI_FACES[0],
        InterfaceFont::System => faces::SYSTEM,
    }
}

/// The appearance bar's settings for `case`.
fn appearance(case: &Case) -> Appearance {
    let selection = case.selection();
    Appearance {
        theme: selection.theme.light.as_str().to_owned(),
        dark: selection.mode == ModeChoice::Dark,
        high_contrast: selection.contrast == ContrastChoice::High,
        comfortable: selection.size == Size::Comfortable,
        text_size: TextSize::ALL.iter().position(|t| *t == selection.text_size).unwrap_or(1),
        rtl: case.dir() == Direction::Rtl,
        ui_face: ui_face(case.font()).to_owned(),
        ..Appearance::default()
    }
}

#[test]
fn the_gallery_appearance_matches_the_design_system() {
    let check = Conformance::new(
        |ctx: &Context, case| {
            let base = appearance(case).configure(ctx, None, false);
            install(ctx, &base);
        },
        |ctx, font| faces::install(ctx, ui_face(font), faces::MONO_FACES[0]),
    );
    // The appearance bar has a light/dark toggle and a high-contrast toggle, so its setup cannot
    // follow the OS: the System mode and System contrast cases are not its to pass.
    let selectable = |case: &Case| case.selection().mode != ModeChoice::System && case.selection().contrast != ContrastChoice::System;
    check.assert_all(Case::matrix().into_iter().chain(Case::axes()).filter(selectable));
}
