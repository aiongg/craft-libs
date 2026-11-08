//! A theme scope that names a mode resolves its body in that mode, whatever mode the root runs in.

use craft_theme::Mode;
use craft_ui::contract::{Show as _, ShowContainer as _, ThemeScope};
use craft_ui::display::Divider;
use craft_ui::layout_components::Stack;
use craft_ui::runtime::env::RootEnv;
use craft_ui::testing::{Frames, trial};
use egui::{Color32, Shape};

/// The colours a frame painted with fills and strokes.
fn colours(shapes: &[Shape]) -> Vec<Color32> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) => Some(rect.fill),
            Shape::LineSegment { stroke, .. } => Some(stroke.color),
            _ => None,
        })
        .collect()
}

fn divider_in(mode: Mode, scoped: Option<Mode>) -> Vec<Color32> {
    let mut frames = Frames::new();
    frames.root(RootEnv::default().mode(mode));
    let mut paint = |ui: &mut egui::Ui| match scoped {
        Some(scope) => {
            Stack::new().theme(ThemeScope::mode(scope)).show(ui, |ui, _| {
                Divider::horizontal().show(ui);
            });
        }
        None => {
            Divider::horizontal().show(ui);
        }
    };
    frames.frame_shapes(&mut paint);
    colours(&frames.frame_shapes(&mut paint))
}

#[trial]
fn a_mode_scope_paints_its_body_in_that_mode_in_either_root_mode() {
    let light = divider_in(Mode::Light, None);
    let dark = divider_in(Mode::Dark, None);
    assert_ne!(light, dark, "the divider differs between the modes");
    assert_eq!(divider_in(Mode::Light, Some(Mode::Dark)), dark);
    assert_eq!(divider_in(Mode::Dark, Some(Mode::Light)), light);
}
