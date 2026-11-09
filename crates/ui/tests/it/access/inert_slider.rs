//! A component shown at a width that cannot hold it shows nothing, and its node promises nothing the role rules need.

use craft_ui::contract::{Bind, Props as _, Role, Show as _};
use craft_ui::testing::access::violations;
use craft_ui::testing::{Fixture, trial};
use craft_ui::value_drag::Slider;
use egui::{Pos2, Rect, Vec2};

const HEIGHT: f32 = 20.0;

fn slider_at(width: f32) -> Fixture {
    let mut value = 5.0;
    let mut fixture = Fixture::new(move |ui| {
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(width, HEIGHT));
        Slider::new(Bind::bound(&mut value), 0.0..=10.0).label("Opacity").show_at(ui, rect);
    });
    fixture.settle();
    fixture
}

#[trial]
fn a_slider_shown_too_narrow_publishes_no_slider_node() {
    let fixture = slider_at(0.0);
    assert!(fixture.tree().with_role(Role::SLIDER).is_empty(), "no slider node for a slider that shows nothing");
    assert_eq!(violations(fixture.ctx(), fixture.tree()), Vec::<String>::new());
}
