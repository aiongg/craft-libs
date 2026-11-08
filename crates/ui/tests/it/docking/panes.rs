//! `PaneArea`: docked panes share an edge in `panes` order with the first outermost, and a one-slot edge shows its last
//! closable pane beside the panes that cannot close.

use craft_ui::contract::{BodyCx, PaneEdge};
use craft_ui::patterns::dock::{PaneArea, PaneContent, PaneSpec, PaneState, SidePolicy};
use craft_ui::testing::{Fixture, trial};
use egui::{Ui, Vec2};

const FLOAT: Vec2 = Vec2::new(240.0, 160.0);
const WIDTH: f32 = 120.0;
const OPEN: [u32; 3] = [0, 1, 2];
/// The pane that cannot close.
const PINNED: u32 = 2;

struct Panes;

impl PaneContent<u32> for Panes {
    fn spec(&self, pane: &u32) -> PaneSpec {
        let mut spec = PaneSpec::new(format!("Pane {pane}"), PaneState::docked(PaneEdge::Start, WIDTH), FLOAT);
        spec.closable = *pane != PINNED;
        spec
    }

    fn body(&mut self, _: &u32, _: &mut Ui, _: &mut BodyCx<'_>) {}
}

fn fixture(policy: SidePolicy) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let mut content = Panes;
        PaneArea::new("Panes", &OPEN, &mut content).policy(policy).show(ui, |_, _| {});
    });
    fixture.settle();
    fixture
}

fn at(fixture: &Fixture, pane: u32) -> Option<f32> {
    fixture.by_label(&format!("Pane {pane}")).map(|target| target.center().x)
}

#[trial]
fn columns_put_the_first_pane_outermost() {
    let fixture = fixture(SidePolicy::Columns);
    let places: Vec<Option<f32>> = OPEN.iter().map(|pane| at(&fixture, *pane)).collect();
    assert!(places.iter().all(Option::is_some), "every docked pane shows: {places:?}");
    assert!(places[0] < places[1] && places[1] < places[2], "the panes follow the open order from the edge inward: {places:?}");
}

#[trial]
fn one_slot_shows_the_last_closable_pane_and_the_pinned_one() {
    let fixture = fixture(SidePolicy::OneSlot);
    assert_eq!(at(&fixture, 0), None, "an earlier closable pane gives up the slot");
    assert!(at(&fixture, 1).is_some(), "the last closable pane takes the slot");
    assert!(at(&fixture, PINNED).is_some(), "a pane that cannot close shows beside the slot");
}
