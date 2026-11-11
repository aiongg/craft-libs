//! The colour area, strip, ramp, disc and wheel: the colour maths and the edits they report.

use std::cell::RefCell;
use std::rc::Rc;

use craft_color::ui::{WheelGeometry, hue_at, triangle_corners, triangle_point, triangle_sv};
use craft_theme::ColorModel;
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, pos2, vec2};

/// The room the parts are shown in.
const ROOM: egui::Vec2 = egui::Vec2::new(400.0, 300.0);

/// A fixture showing `show` over a shared value.
fn fixture<T: 'static>(value: T, show: impl Fn(&mut egui::Ui, &mut T) + 'static) -> (Fixture, Rc<RefCell<T>>) {
    let shared = Rc::new(RefCell::new(value));
    let held = shared.clone();
    let mut fixture = Fixture::new(move |ui| show(ui, &mut held.borrow_mut()));
    fixture.room(ROOM);
    fixture.settle();
    (fixture, shared)
}

fn within(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// The wheel's pure geometry: a triangle point reads back as its saturation and brightness, and the ring hue is whole
/// degrees counter-clockwise from the right.
#[trial]
fn the_wheel_geometry_round_trips() {
    let center = pos2(100.0, 100.0);
    let corners = triangle_corners(center, 60.0, 210.0);
    for (s, b) in [(0.0, 1.0), (1.0, 1.0), (0.4, 0.7), (0.9, 0.2)] {
        let (read_s, read_b) = triangle_sv(corners, triangle_point(corners, s, b)).expect("a triangle point reads back");
        assert!(within(read_s, s) && within(read_b, b), "({s}, {b}) read as ({read_s}, {read_b})");
    }
    assert_eq!(hue_at(center, center + vec2(10.0, 0.0)), Some(0.0));
    assert_eq!(hue_at(center, center + vec2(0.0, -10.0)), Some(90.0), "up is 90 degrees");
    assert_eq!(hue_at(center, Pos2::new(f32::NAN, 0.0)), None);
    let geometry = WheelGeometry::new(Rect::from_min_size(Pos2::ZERO, vec2(200.0, 200.0)), 4.0, (8.0, 24.0), 0.09, 1.0);
    assert!(geometry.inner < geometry.outer && geometry.triangle < geometry.inner);
}
