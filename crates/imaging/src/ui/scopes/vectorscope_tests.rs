//! The vectorscope's geometry.

use egui::{Rect, pos2, vec2};

use super::scope_point;

// `scope_point` puts red at 3 o'clock with `red_right` and at −100° without; saturation 1 is the
// radius (half the shorter side less 14).
#[test]
fn vectorscope_places_red_at_the_chosen_angle() {
    let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(200.0, 100.0));
    let (center, radius) = (rect.center(), 36.0);
    let near = |a: egui::Pos2, b: egui::Pos2| (a - b).length() < 1e-3;
    assert!(near(scope_point(rect, true, 0.0, 1.0), center + vec2(radius, 0.0)));
    assert!(near(scope_point(rect, true, 0.0, 0.5), center + vec2(radius * 0.5, 0.0)));
    let angle = -100.0f32.to_radians();
    assert!(near(scope_point(rect, false, 0.0, 1.0), center + vec2(angle.cos(), angle.sin()) * radius));
    assert!(near(scope_point(rect, true, 0.0, 0.0), center));
}
