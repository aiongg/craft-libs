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
fn a_slider_publishes_a_node_only_when_shown_wide_enough() {
    let narrow = slider_at(0.0);
    assert!(narrow.tree().with_role(Role::SLIDER).is_empty(), "no slider node for a slider that shows nothing");
    assert_eq!(violations(narrow.ctx(), narrow.tree()), Vec::<String>::new());
    assert!(!slider_at(120.0).tree().with_role(Role::SLIDER).is_empty(), "a slider with room publishes its node");
}
