//! Stories of the swatch pair.

use super::{PairWell, SwatchPair, Well};
use craft_behavior::Mixed;
use craft_theme::Srgba;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::{Key, Modifiers, Ui};

/// Tab arrives at the active well; the arrow moves to the other well and makes it active.
fn next_well(fixture: &mut Fixture) {
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
}

fn wells(ring: bool) -> [PairWell; 2] {
    [
        PairWell::new("Foreground colour", Srgba::rgb(0.1, 0.3, 0.8)),
        PairWell::new("Background colour", Srgba { r: 1.0, g: 1.0, b: 1.0, a: 0.7 }).ring(ring),
    ]
}

#[story(SwatchPair<'static>, covers = [Default], drive = Drive::Custom(next_well))]
fn swatch_pair(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    SwatchPair::new("Colours", wells(false), Bind::bound(data.value("active", Well::Primary)))
        .swap_button("Swap colours")
        .default_button("Default colours")
        .show(ui)
        .into()
}

#[story(SwatchPair<'static>, covers = [], drive = Drive::Never)]
fn swatch_pair_ring(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    SwatchPair::new("Fill and stroke", wells(true), Bind::bound(data.value("active", Well::Secondary))).show(ui).into()
}

#[story(SwatchPair<'static>, covers = [], drive = Drive::Never)]
fn swatch_pair_host_picker(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    SwatchPair::new("Colours", wells(false), Bind::bound(data.value("active", Well::Primary))).popover(false).click_activates(false).show(ui).into()
}

#[story(SwatchPair<'static>, covers = [Mixed], drive = Drive::Never)]
fn swatch_pair_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let wells = [PairWell::new("Fill", Mixed::<Srgba>::Mixed), PairWell::new("Stroke", Srgba::rgb(0.1, 0.3, 0.8)).ring(true)];
    SwatchPair::new("Fill and stroke", wells, Bind::bound(data.value("active", Well::Primary))).show(ui).into()
}
