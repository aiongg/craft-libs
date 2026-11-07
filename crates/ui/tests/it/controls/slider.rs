//! `Slider`: the arrow keys step toward the end edge of the reading direction, and Shift steps by the large step.

use craft_i18n::Direction;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::controls::slider::Slider;
use craft_ui::testing::{Fixture, Story, StoryData, StoryOutput, trial};
use egui::{Key, Modifiers, Ui};

const START: f64 = 50.0;
const STEP: f64 = 5.0;

fn show(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", START)), 0.0..=100.0).step(STEP).label("Opacity").show(ui).into()
}

fn after(direction: Direction, key: Key, modifiers: Modifiers) -> f64 {
    let mut fixture = Fixture::story(&Story::new("slider", show));
    fixture.direction(direction).settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(key, modifiers);
    fixture.idle(2);

    *fixture.data().value("value", START)
}

#[trial]
fn a_slider_steps_toward_the_end_edge_of_the_reading_direction() {
    assert_eq!(after(Direction::Ltr, Key::ArrowRight, Modifiers::NONE), START + STEP);
    assert_eq!(after(Direction::Ltr, Key::ArrowLeft, Modifiers::NONE), START - STEP);
    assert_eq!(after(Direction::Rtl, Key::ArrowLeft, Modifiers::NONE), START + STEP, "the mirrored slider grows toward the left");
    assert_eq!(after(Direction::Ltr, Key::ArrowRight, Modifiers::SHIFT), 100.0, "Shift steps ten times as far, to the end of the range");
}
