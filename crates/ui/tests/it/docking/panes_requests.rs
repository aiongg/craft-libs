//! `PaneArea` requests: a list body scrolls in a docked and a floating pane, a floating pane minimizes through a
//! request, Float on a top pane docks it back at the top, and a moved pane docks on the edge it is released over.

use craft_ui::buttons::Button;
use craft_ui::contract::{BodyCx, PaneEdge, Show as _, ShowCollection as _};
use craft_ui::docking::GroupDrag;
use craft_ui::list::{List, ListRow};
use craft_ui::patterns::dock::{PaneArea, PaneContent, PanePlace, PaneRequest, PaneSpec, PaneState};
use craft_ui::runtime::drag;
use craft_ui::testing::pointer::{moved, press, release, wheel};
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const FLOAT: Vec2 = Vec2::new(200.0, 140.0);
const WIDTH: f32 = 160.0;
const FRAME: Rect = Rect { min: Pos2::new(100.0, 80.0), max: Pos2::new(300.0, 220.0) };
/// A point on the floating title row, clear of its action parts.
const TITLE_AT: Vec2 = Vec2::new(24.0, 10.0);
const LAYERS: usize = 40;
const PANE: u32 = 0;

/// The app: it keeps the pane's state by applying the requests it gets.
struct App {
    default: PaneState,
    state: Rc<RefCell<Option<PaneState>>>,
    list: bool,
    layers: Vec<String>,
}

impl PaneContent<u32> for App {
    fn spec(&self, _: &u32) -> PaneSpec {
        let mut spec = PaneSpec::new("Pane 0", self.default, FLOAT);
        spec.collection_body = self.list;
        spec
    }

    fn state(&self, _: &u32) -> Option<PaneState> {
        *self.state.borrow()
    }

    fn body(&mut self, _: &u32, ui: &mut Ui, _: &mut BodyCx<'_>) {
        if self.list {
            let source = craft_behavior::keyed(&self.layers, |name: &String| name.clone());
            List::<String>::new("Layers").show_items(ui, &source, &mut |key, _| ListRow::new(key.clone()));
        } else {
            Button::new("Body").show(ui);
        }
    }
}

struct Bed {
    fixture: Fixture,
    requests: Rc<RefCell<Vec<PaneRequest<u32>>>>,
}

impl Bed {
    fn new(default: PaneState, start: PaneState, list: bool) -> Self {
        let state = Rc::new(RefCell::new(Some(start)));
        let requests = Rc::new(RefCell::new(Vec::new()));
        let seen = Rc::clone(&requests);
        let mut fixture = Fixture::new(move |ui| {
            let mut app = App { default, state: Rc::clone(&state), list, layers: (1..=LAYERS).map(|n| format!("Layer {n}")).collect() };
            let shown = PaneArea::new("Panes", &[PANE], &mut app).show(ui, |_, _| {});
            for request in shown.requests {
                if let Some(kept) = state.borrow_mut().as_mut() {
                    kept.apply(&request);
                }
                seen.borrow_mut().push(request);
            }
        });
        fixture.settle();
        Bed { fixture, requests }
    }

    fn sent(&self, request: &PaneRequest<u32>) -> bool {
        self.requests.borrow().contains(request)
    }
}

fn docked(edge: PaneEdge) -> PaneState {
    PaneState::docked(edge, WIDTH)
}

/// A pane that floats in `FRAME` and keeps the size it had docked.
fn floated() -> PaneState {
    let mut state = docked(PaneEdge::Start);
    state.place = PanePlace::Floating;
    state.float_rect = Some(FRAME);
    state
}

fn title_of(rect: Rect) -> Pos2 {
    rect.min + TITLE_AT
}

#[trial]
fn a_list_body_shows_and_scrolls_in_a_docked_and_a_floating_pane() {
    for start in [docked(PaneEdge::Start), floated()] {
        let mut bed = Bed::new(docked(PaneEdge::Start), start, true);
        let first = bed.fixture.by_label("Layer 1").unwrap_or_else(|| panic!("the first row of the list shows in {:?}", start.place));
        bed.fixture.frame(vec![moved(first.center())]);
        bed.fixture.frame(vec![wheel(-300.0)]);
        bed.fixture.idle(2);
        let after = bed.fixture.by_label("Layer 1");
        assert!(
            after.is_none_or(|row| row.rect.min.y < first.rect.min.y),
            "the list scrolled inside the {:?} pane: {:?} then {after:?}",
            start.place,
            first.rect
        );
    }
}

#[trial]
fn a_moved_pane_shows_the_drop_mark_on_the_other_edge_and_docks_there_on_release() {
    let mut bed = Bed::new(docked(PaneEdge::Start), floated(), false);
    let (from, to) = (title_of(FRAME), Pos2::new(392.0, 150.0));
    bed.fixture.frame(vec![moved(from)]);
    bed.fixture.frame(vec![press(from)]);
    for step in 1..=4 {
        bed.fixture.frame(vec![moved(from + (to - from) * (step as f32 / 4.0))]);
    }
    bed.fixture.idle(1);
    assert!(drag::active::<GroupDrag<u32>>(bed.fixture.ctx()).is_some(), "the move by the title started a window drag");
    assert!(drag::mark(bed.fixture.ctx()).is_some(), "the end edge band takes the drop and shows its mark");
    bed.fixture.frame(vec![release(to)]);
    bed.fixture.idle(2);
    assert!(bed.sent(&PaneRequest::Place(PANE, PanePlace::Docked(PaneEdge::End))), "the release docks it at the end edge: {:?}", bed.requests);
    let docked_at = bed.fixture.by_label("Pane 0").expect("the pane shows docked").center().x;
    assert!(docked_at > FRAME.center().x, "and it shows on the end side, right of where it floated: {docked_at}");
}
