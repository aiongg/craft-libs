//! A docked pane's inner edge resizes it: one `PaneRequest::Resize` when the drag ends, with the pane showing the
//! new extent in that frame. A pane on a side edge has the same width and a mirrored rect in both reading directions.

use crate::body::kit::Slab;
use craft_i18n::Direction;
use craft_ui::buttons::Button;
use craft_ui::contract::{BodyCx, ContainerProps, Mount, PaneEdge, Props as _, ShellPane, Show as _, ShowContainer as _};
use craft_ui::patterns::dock::{PaneArea, PaneContent, PaneRequest, PaneSpec, PaneState};
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const FLOAT: Vec2 = Vec2::new(200.0, 140.0);
const WIDTH: f32 = 160.0;
const GROW: f32 = 40.0;
const SLACK: f32 = 0.5;
const PANE: u32 = 0;

/// The app: it keeps the pane's state by applying the requests it gets.
struct App {
    state: Rc<RefCell<PaneState>>,
}

impl PaneContent<u32> for App {
    fn spec(&self, _: &u32) -> PaneSpec {
        PaneSpec::new("Pane 0", PaneState::docked(PaneEdge::Start, WIDTH), FLOAT)
    }

    fn state(&self, _: &u32) -> Option<PaneState> {
        Some(*self.state.borrow())
    }

    fn body(&mut self, _: &u32, ui: &mut Ui, _: &mut BodyCx<'_>) {
        Button::new("Body").show(ui);
    }
}

/// The rect of the pane's node: the largest node named by the pane's title.
fn pane_rect(fixture: &Fixture) -> Rect {
    let tree = fixture.tree();
    tree.keys()
        .into_iter()
        .filter(|key| tree.name_of(*key) == Some("Pane 0"))
        .filter_map(|key| tree.bounds_of(key))
        .max_by(|a, b| a.area().total_cmp(&b.area()))
        .expect("the pane shows")
}

#[trial]
fn dragging_a_docked_panes_inner_edge_sends_one_resize_and_shows_it() {
    let state = Rc::new(RefCell::new(PaneState::docked(PaneEdge::Start, WIDTH)));
    let requests = Rc::new(RefCell::new(Vec::new()));
    let (kept, seen) = (Rc::clone(&state), Rc::clone(&requests));
    let mut fixture = Fixture::new(move |ui| {
        let mut app = App { state: Rc::clone(&kept) };
        let shown = PaneArea::new("Panes", &[PANE], &mut app).show(ui, |_, _| {});
        for request in shown.requests {
            kept.borrow_mut().apply(&request);
            seen.borrow_mut().push(request);
        }
    });
    fixture.settle();
    let before = pane_rect(&fixture);
    assert!((before.width() - WIDTH).abs() < SLACK, "the pane shows its state's width: {before:?}");
    // Just inside the inner edge, where the grip lies.
    let at = before.right_center() - Vec2::X;
    let by = Vec2::X * GROW;
    for events in [vec![moved(at)], vec![press(at)], vec![moved(at + by / 2.0)], vec![moved(at + by)], vec![release(at + by)]] {
        fixture.frame(events);
    }
    let resizes: Vec<f32> =
        requests.borrow().iter().filter_map(|request| if let PaneRequest::Resize(_, size) = request { Some(*size) } else { None }).collect();
    assert_eq!(resizes.len(), 1, "one resize, when the drag ends: {:?}", requests.borrow());
    assert!((resizes[0] - (WIDTH + GROW)).abs() < SLACK, "the resize carries the new extent: {resizes:?}");
    let after = pane_rect(&fixture);
    assert!((after.width() - (WIDTH + GROW)).abs() < SLACK, "the pane shows the new extent in the frame the drag ends: {after:?}");
    assert!((after.left() - before.left()).abs() < SLACK, "the outer edge stays: {after:?} {before:?}");
}

/// Shows a bare pane on `edge` in a fixture under `direction`; returns the fixture and the pane's rect.
fn bare_pane(edge: PaneEdge, direction: Direction) -> (Fixture, Rect) {
    let rect = Rc::new(Cell::new(Rect::NOTHING));
    let seen = Rc::clone(&rect);
    let mut fixture = Fixture::new(move |ui| {
        let props = ContainerProps::new().mount(Mount::Pane(ShellPane::new("side").edge(edge).extent(WIDTH)));
        let shown = Slab::new(props).id_salt("pane").show(ui, |ui, _| {
            Button::new("Body").show(ui);
        });
        seen.set(shown.output.rect);
    });
    fixture.direction(direction).settle();
    (fixture, rect.get())
}

#[trial]
fn a_side_pane_has_the_same_width_and_a_mirrored_rect_in_both_directions() {
    for edge in [PaneEdge::Start, PaneEdge::End] {
        let (ltr, ltr_pane) = bare_pane(edge, Direction::Ltr);
        let (rtl, rtl_pane) = bare_pane(edge, Direction::Rtl);
        let (ltr_shown, rtl_shown) = (ltr.shown(), rtl.shown());
        assert!((ltr_shown.width() - rtl_shown.width()).abs() < SLACK, "{edge:?}: the used width is the same: {ltr_shown:?} {rtl_shown:?}");
        assert!((ltr_pane.width() - rtl_pane.width()).abs() < SLACK, "{edge:?}: the pane keeps its width: {ltr_pane:?} {rtl_pane:?}");
        let mirrored = (ltr_pane.left() - ltr_shown.left()) - (rtl_shown.right() - rtl_pane.right());
        assert!(mirrored.abs() < SLACK, "{edge:?}: the pane lies at the mirrored edge: {ltr_pane:?} {rtl_pane:?} in {ltr_shown:?} {rtl_shown:?}");
    }
}
