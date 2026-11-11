//! `TransformBox`, `TransformOverlay` and `TransformTool`: the parts the pointer takes, what each drag does, the
//! keys, the readout and the overlay's edits.

use craft_behavior::geometry::quad::{self, BoxPart, BoxRules, BoxState, TransformMode};
use craft_behavior::kurbo::{Point, Rect, Vec2 as DocVec};
use craft_behavior::{CanvasView, EditEvent, PartKey};
use craft_draw::ui::transform::{RotateZone, TransformBox, TransformEdit, TransformOverlay, TransformSession, TransformSettings};
use craft_keymap::Mods;
use craft_ui::canvas::Canvas;
use craft_ui::canvas::{Guide, GuideKey};
use craft_ui::canvas_controls::Guides;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The box before any drag, in document units.
const RECT: Rect = Rect::new(100.0, 80.0, 300.0, 220.0);

/// The part key the tests address the box's parts by.
const BOX: PartKey = PartKey::new("box");

#[derive(Clone, Copy, Debug, PartialEq)]
enum Seen {
    Begin,
    Update,
    Commit,
    Cancel,
    Set,
}

#[derive(Clone)]
struct Config {
    rules: BoxRules,
    zone: RotateZone,
    inside_moves: bool,
    reference_point: bool,
    pixel_grid: bool,
    mods: Option<Mods>,
    rotation: f64,
    flip: bool,
    guides: bool,
}

/// What the host keeps of the box and what the passes reported.
struct Host {
    state: BoxState,
    before: Option<BoxState>,
    events: Vec<Seen>,
    applied: usize,
    discarded: usize,
    picked: Option<BoxPart>,
    hovered: Option<BoxPart>,
    id: egui::Id,
    view: CanvasView,
    guide_events: usize,
}

struct Harness {
    fixture: Fixture,
    host: Rc<RefCell<Host>>,
    config: Rc<RefCell<Config>>,
}

fn defaults() -> Config {
    Config {
        rules: BoxRules::default(),
        zone: RotateZone::Outside,
        inside_moves: true,
        reference_point: true,
        pixel_grid: false,
        mods: None,
        rotation: 0.0,
        flip: false,
        guides: false,
    }
}

fn harness(state: BoxState, setup: impl FnOnce(&mut Config)) -> Harness {
    let mut config = defaults();
    setup(&mut config);
    let config = Rc::new(RefCell::new(config));
    let host = Rc::new(RefCell::new(Host {
        state,
        before: None,
        events: Vec::new(),
        applied: 0,
        discarded: 0,
        picked: None,
        hovered: None,
        id: egui::Id::NULL,
        view: CanvasView::new(egui::Rect::NOTHING, Point::ZERO, 1.0),
        guide_events: 0,
    }));
    let (shown_config, shown_host) = (Rc::clone(&config), Rc::clone(&host));
    let mut guides = vec![Guide::new(GuideKey::new(1), craft_behavior::Axis::Y, 300.0)];
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let config = shown_config.borrow().clone();
        let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0)
            .rotation(config.rotation)
            .flip_x(config.flip);
        let state = {
            let mut host = shown_host.borrow_mut();
            host.view = view;
            host.state
        };
        let canvas = Canvas::new("Document").view(view).show(ui, |ui, body| {
            let mut guide_edits = 0;
            if config.guides {
                let output = Guides::new("Guides", Bind::bound(&mut guides)).interactive(true).show_at(ui, body.rect);
                guide_edits = usize::from(!output.edit.events.is_empty());
            }
            let mut control = TransformBox::new("Free Transform", Bind::given(state), RECT, 0.0)
                .rules(config.rules)
                .rotate_zone(config.zone)
                .inside_moves(config.inside_moves)
                .reference_point(config.reference_point)
                .pixel_grid(config.pixel_grid);
            if let Some(mods) = config.mods {
                control = control.mods(mods);
            }
            (control.show_at(ui, body.rect), guide_edits)
        });
        if let Some((output, guide_edits)) = canvas.inner {
            let mut host = shown_host.borrow_mut();
            host.guide_events += guide_edits;
            host.id = output.edit.response.id;
            host.applied += usize::from(output.apply);
            host.discarded += usize::from(output.discard);
            (host.picked, host.hovered) = (output.picked, output.hovered);
            for event in &output.edit.events {
                match event {
                    EditEvent::Begin { .. } => {
                        host.events.push(Seen::Begin);
                        host.before = Some(host.state);
                    }
                    EditEvent::Update { value, .. } => {
                        host.events.push(Seen::Update);
                        host.state = *value;
                    }
                    EditEvent::Commit { after, .. } => {
                        host.events.push(Seen::Commit);
                        host.state = *after;
                    }
                    EditEvent::Set { after, .. } => {
                        host.events.push(Seen::Set);
                        host.state = *after;
                    }
                    EditEvent::Cancel { .. } => {
                        host.events.push(Seen::Cancel);
                        host.state = host.before.take().unwrap_or(host.state);
                    }
                    _ => {}
                }
            }
        }
    });
    fixture.settle();
    Harness { fixture, host, config }
}

fn upright() -> Harness {
    harness(BoxState::from_rect(RECT), |_| {})
}

fn frame(h: &mut Harness, events: Vec<egui::Event>) {
    h.fixture.frame(events);
}

impl Harness {
    fn screen(&self, p: Point) -> Pos2 {
        self.host.borrow().view.to_screen(p)
    }

    fn state(&self) -> BoxState {
        self.host.borrow().state
    }

    fn count(&self, seen: Seen) -> usize {
        self.host.borrow().events.iter().filter(|&&event| event == seen).count()
    }

    fn part(&self, part: BoxPart) -> Option<craft_ui::testing::Target> {
        self.fixture.part(self.host.borrow().id, BOX.at(part.index()))
    }

    fn hover(&mut self, at: Pos2) {
        frame(self, vec![pointer::moved(at)]);
        frame(self, vec![pointer::moved(at)]);
    }

    /// Presses at `from` and moves to `to` in two steps, the button held.
    fn hold_drag(&mut self, from: Pos2, to: Pos2) {
        frame(self, vec![pointer::moved(from)]);
        frame(self, vec![pointer::press(from)]);
        frame(self, vec![pointer::moved(from.lerp(to, 0.5))]);
        frame(self, vec![pointer::moved(to)]);
    }

    /// A whole drag with `mods` held.
    fn drag(&mut self, from: Pos2, to: Pos2, mods: Modifiers) {
        self.fixture.hold(mods);
        self.hold_drag(from, to);
        frame(self, vec![pointer::release(to)]);
        self.fixture.hold(Modifiers::NONE);
        frame(self, Vec::new());
    }

    fn click(&mut self, at: Pos2) {
        self.fixture.click_at(at);
    }
}

/// A screen displacement.
fn by(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// A document displacement.
fn doc(x: f64, y: f64) -> DocVec {
    DocVec::new(x, y)
}

fn near(a: &BoxState, b: &BoxState) -> bool {
    a.quad.iter().chain([&a.pivot]).zip(b.quad.iter().chain([&b.pivot])).all(|(p, q)| p.distance(*q) < 1e-3)
}

fn expected(h: &Harness, start: &BoxState, part: BoxPart, from: Pos2, to: Pos2, mods: Mods) -> BoxState {
    let view = h.host.borrow().view;
    quad::drag(start, start, part, view.to_doc(from), view.to_doc(to), mods, &h.config.borrow().rules)
}

#[trial]
fn the_nearest_handle_wins_on_a_small_box() {
    let small = BoxState::from_rect(Rect::new(190.0, 140.0, 210.0, 160.0));
    for (press, part) in [(Point::new(193.0, 141.0), BoxPart::Corner(0)), (Point::new(198.0, 141.0), BoxPart::Edge(0))] {
        let mut h = harness(small, |_| {});
        let from = h.screen(press);
        let to = from + by(-20.0, -30.0);
        let want = expected(&h, &small, part, from, to, Mods::NONE);
        h.drag(from, to, Modifiers::NONE);
        assert!(near(&h.state(), &want), "{part:?}: {:?}", h.state());
    }
}

#[trial]
fn a_handle_takes_the_pointer_within_12_points_in_a_turned_flipped_view() {
    for (offset, part) in [(11.9, BoxPart::Corner(2)), (12.1, BoxPart::Outside)] {
        let mut h = harness(BoxState::from_rect(RECT), |config| {
            config.rotation = 30.0;
            config.flip = true;
        });
        let corner = h.screen(Point::new(RECT.x1, RECT.y1));
        let out = (corner - h.screen(RECT.center())).normalized();
        let at = corner + out * offset;
        h.hover(at);
        assert_eq!(h.host.borrow().hovered, Some(part), "{offset} pt from the corner");
        let start = h.state();
        let to = at + out * 30.0;
        let want = expected(&h, &start, part, at, to, Mods::NONE);
        h.drag(at, to, Modifiers::NONE);
        assert!(near(&h.state(), &want), "{offset} pt: {:?}", h.state());
    }
}

#[trial]
fn each_part_drags_as_quad_drag_has_it_in_every_mode() {
    let modes = [TransformMode::Free, TransformMode::Rotate, TransformMode::Skew, TransformMode::Distort, TransformMode::Perspective];
    let parts = [
        (BoxPart::Corner(2), Point::new(300.0, 220.0)),
        (BoxPart::Edge(1), Point::new(300.0, 150.0)),
        (BoxPart::Inside, Point::new(150.0, 110.0)),
        (BoxPart::Outside, Point::new(340.0, 260.0)),
        (BoxPart::Pivot, RECT.center()),
    ];
    for mode in modes {
        for (part, at) in parts {
            let mut h = harness(BoxState::from_rect(RECT), |config| config.rules = BoxRules::in_mode(mode));
            let start = h.state();
            let from = h.screen(at);
            let to = from + by(30.0, 20.0);
            let rules = h.config.borrow().rules;
            let want = if rules.allows(part) { expected(&h, &start, part, from, to, Mods::NONE) } else { start };
            h.drag(from, to, Modifiers::NONE);
            assert!(near(&h.state(), &want), "{mode:?} {part:?}: {:?}", h.state());
            assert_eq!(h.host.borrow().applied, 0, "{mode:?} {part:?}: a drag doesn't apply");
            assert_eq!(h.host.borrow().events.is_empty(), near(&start, &want), "{mode:?} {part:?}: {:?}", h.host.borrow().events);
        }
    }
}

#[trial]
fn a_click_outside_applies_and_a_drag_outside_rotates() {
    let mut h = upright();
    let at = h.screen(Point::new(340.0, 260.0));
    h.click(at);
    assert_eq!(h.host.borrow().applied, 1, "a click outside applies");
    assert!(h.host.borrow().events.is_empty() && h.state() == BoxState::from_rect(RECT));

    h.drag(at, at + by(-40.0, 30.0), Modifiers::NONE);
    assert_eq!(h.host.borrow().applied, 1, "a drag doesn't apply");
    assert!(h.state().angle().abs() > 1.0, "the drag turns the box: {}", h.state().angle());
}

#[trial]
fn a_double_click_inside_applies() {
    let mut h = upright();
    let at = h.screen(Point::new(150.0, 110.0));
    frame(&mut h, vec![pointer::moved(at), pointer::press(at), pointer::release(at)]);
    frame(&mut h, vec![pointer::press(at), pointer::release(at)]);
    frame(&mut h, Vec::new());
    assert_eq!(h.host.borrow().applied, 1);
    assert!(h.host.borrow().events.is_empty(), "clicks don't move the box");
}

#[trial]
fn esc_cancels_a_drag_and_the_rest_of_the_press_does_nothing() {
    let mut h = upright();
    let from = h.screen(Point::new(150.0, 110.0));
    h.hold_drag(from, from + by(40.0, 0.0));
    assert!(
        near(&h.state(), &BoxState::from_rect(RECT).moved(doc(40.0, 0.0))),
        "the host shows the drag: {:?} {:?}",
        h.state(),
        h.host.borrow().events
    );
    h.fixture.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.count(Seen::Cancel), 1, "{:?}", h.host.borrow().events);
    assert_eq!(h.state(), BoxState::from_rect(RECT), "Esc undoes the drag");
    let count = h.host.borrow().events.len();
    frame(&mut h, vec![pointer::moved(from + by(60.0, 0.0))]);
    frame(&mut h, vec![pointer::release(from + by(60.0, 0.0))]);
    assert_eq!(h.host.borrow().events.len(), count, "the rest of the press does nothing");
    assert_eq!(h.host.borrow().applied, 0);
}

#[trial]
fn esc_before_the_first_move_ends_the_press() {
    let mut h = upright();
    let from = h.screen(Point::new(150.0, 110.0));
    frame(&mut h, vec![pointer::moved(from)]);
    frame(&mut h, vec![pointer::press(from)]);
    h.fixture.key(Key::Escape, Modifiers::NONE);
    frame(&mut h, vec![pointer::moved(from + by(40.0, 0.0))]);
    frame(&mut h, vec![pointer::release(from + by(40.0, 0.0))]);
    frame(&mut h, Vec::new());
    assert!(h.host.borrow().events.is_empty(), "{:?}", h.host.borrow().events);
    assert_eq!(h.state(), BoxState::from_rect(RECT));
    assert_eq!(h.host.borrow().discarded, 0, "the box stays open");
}

#[trial]
fn the_band_zone_leaves_the_rest_of_the_outside_to_the_tool() {
    let mut h = harness(BoxState::from_rect(RECT), |config| config.zone = RotateZone::Band);
    let far = h.screen(Point::new(200.0, 30.0));
    h.hover(far);
    assert_eq!(h.host.borrow().hovered, None);
    h.click(far);
    assert_eq!(h.host.borrow().applied, 0, "a click outside the band is the tool's");
    h.drag(far, far + by(40.0, 30.0), Modifiers::NONE);
    assert!(h.host.borrow().events.is_empty());

    // 15 pt out along the corner's diagonal: in the band, it rotates.
    let corner = h.screen(Point::new(300.0, 220.0));
    let out = (corner - h.screen(RECT.center())).normalized();
    let at = corner + out * 15.0;
    h.hover(at);
    assert_eq!(h.host.borrow().hovered, Some(BoxPart::Outside));
    h.drag(at, at + by(-30.0, 20.0), Modifiers::NONE);
    assert!(h.state().angle().abs() > 1.0);
}

#[trial]
fn arrows_move_the_picked_part_as_a_plain_drag_of_it_would() {
    let mut h = upright();
    let corner = h.part(BoxPart::Corner(0)).expect("the corner has a part");
    h.fixture.click(&corner);
    assert_eq!(h.host.borrow().picked, Some(BoxPart::Corner(0)), "a press picks the part");
    let start = h.state();
    h.fixture.key(Key::ArrowLeft, Modifiers::NONE);
    let want = quad::drag(&start, &start, BoxPart::Corner(0), start.quad[0], start.quad[0] + doc(-1.0, 0.0), Mods::NONE, &BoxRules::default());
    assert_eq!(h.state(), want, "a plain corner drag: proportional about the opposite corner");
    assert!((h.state().quad[0].y - start.quad[0].y).abs() > 1e-6, "proportional: the height follows");
    assert!(matches!(h.host.borrow().events.last(), Some(Seen::Set)));

    assert!(h.part(BoxPart::Inside).is_some(), "the whole box has a part");
    // The middle of the box is the reference point's: press the whole box away from it.
    let inside = h.screen(Point::new(150.0, 110.0));
    h.fixture.click_at(inside);
    let before = h.state();
    h.fixture.key(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(h.state(), before.moved(doc(0.0, 10.0)), "Shift moves ten units");
}

#[trial]
fn page_keys_skip_the_parts_the_mode_refuses() {
    use BoxPart::{Corner, Inside, Pivot};
    let mut h = harness(BoxState::from_rect(RECT), |config| config.rules = BoxRules::in_mode(TransformMode::Distort));
    assert!(h.part(Inside).is_some(), "the whole box has a part");
    // The middle of the box is the reference point's: press the whole box away from it.
    let inside = h.screen(Point::new(150.0, 110.0));
    h.fixture.click_at(inside);
    let mut order = vec![h.host.borrow().picked];
    for _ in 0..5 {
        h.fixture.key(Key::PageDown, Modifiers::NONE);
        order.push(h.host.borrow().picked);
    }
    assert_eq!(order, [Some(Inside), Some(Corner(0)), Some(Corner(1)), Some(Corner(2)), Some(Corner(3)), Some(Pivot)]);
    assert!(h.part(BoxPart::Edge(0)).is_none(), "an edge is no part in Distort");
    h.fixture.key(Key::Home, Modifiers::NONE);
    assert_eq!(h.host.borrow().picked, Some(Inside));
}

#[trial]
fn the_readout_shows_what_each_gesture_changes() {
    let cases = [
        (Point::new(340.0, 260.0), by(-30.0, 30.0), "Angle:"),
        (Point::new(300.0, 220.0), by(20.0, 0.0), "W:"),
        (Point::new(150.0, 110.0), by(20.0, -10.0), "ΔX:"),
    ];
    for (press, by, label) in cases {
        let mut h = upright();
        let from = h.screen(press);
        h.hold_drag(from, from + by);
        assert!(h.fixture.texts().iter().any(|text| text.text.contains(label)), "{label}");
        frame(&mut h, vec![pointer::release(from + by)]);
        frame(&mut h, Vec::new());
        assert!(!h.fixture.texts().iter().any(|text| text.text.contains(label)), "the readout goes with the gesture: {label}");
    }
}

#[trial]
fn the_box_takes_a_press_on_a_handle_before_a_guide_under_it() {
    let mut h = harness(BoxState::from_rect(RECT), |config| config.guides = true);
    // The guide runs through the box's right edge, at x = 300; the corner at (300, 80) is the box's.
    let corner = h.screen(Point::new(300.0, 80.0));
    h.drag(corner, corner + by(20.0, 0.0), Modifiers::NONE);
    assert!(h.host.borrow().events.contains(&Seen::Commit), "the box took the press");
    assert_eq!(h.host.borrow().guide_events, 0, "the guide did not move");
}

#[trial]
fn alt_beyond_the_box_places_the_reference_point_there_and_drags_it() {
    let mut h = upright();
    let at = h.screen(Point::new(340.0, 260.0));
    h.fixture.hold(Modifiers::ALT);
    frame(&mut h, vec![pointer::moved(at)]);
    frame(&mut h, vec![pointer::press(at)]);
    frame(&mut h, vec![pointer::release(at)]);
    h.fixture.hold(Modifiers::NONE);
    frame(&mut h, Vec::new());
    assert!(
        near(&BoxState::new([Point::ZERO; 4], h.state().pivot), &BoxState::new([Point::ZERO; 4], Point::new(340.0, 260.0))),
        "{:?}",
        h.state().pivot
    );
    assert_eq!(h.state().quad, BoxState::from_rect(RECT).quad, "the corners stay");
    assert_eq!(h.host.borrow().applied, 0, "an Alt click does not apply");

    let to = at + by(10.0, 10.0);
    h.drag(at, to, Modifiers::ALT);
    assert!((h.state().pivot - Point::new(350.0, 270.0)).hypot() < 1e-3, "{:?}", h.state().pivot);
}

#[trial]
fn a_pixel_grid_box_fed_back_as_state_does_not_drift_over_a_drag() {
    let mut h = harness(BoxState::from_rect(RECT), |config| config.pixel_grid = true);
    let from = h.screen(Point::new(300.0, 220.0));
    frame(&mut h, vec![pointer::moved(from)]);
    frame(&mut h, vec![pointer::press(from)]);
    for step in 1..=12 {
        frame(&mut h, vec![pointer::moved(from + by(0.4 * step as f32, 0.3 * step as f32))]);
    }
    let held = h.state();
    frame(&mut h, vec![pointer::release(from + by(4.8, 3.6))]);
    frame(&mut h, Vec::new());
    for corner in h.state().quad {
        assert!((corner.x - corner.x.round()).abs() < 1e-9 && (corner.y - corner.y.round()).abs() < 1e-9, "{corner:?} is on whole units");
    }
    assert!((h.state().quad[2].x - held.quad[2].x).abs() <= 1.0, "no drift from the held box to the committed one");
}

#[trial]
fn an_external_change_during_a_pixel_grid_drag_still_cancels() {
    let mut h = harness(BoxState::from_rect(RECT), |config| config.pixel_grid = true);
    let from = h.screen(Point::new(300.0, 220.0));
    h.hold_drag(from, from + by(20.0, 20.0));
    assert_eq!(h.count(Seen::Begin), 1);
    h.host.borrow_mut().state = BoxState::from_rect(RECT).moved(doc(0.0, 50.0));
    frame(&mut h, vec![pointer::moved(from + by(30.0, 30.0))]);
    frame(&mut h, vec![pointer::release(from + by(30.0, 30.0))]);
    assert_eq!(h.count(Seen::Cancel), 1, "{:?}", h.host.borrow().events);
    assert_eq!(h.count(Seen::Commit), 0);
}

#[trial]
fn a_release_with_the_pointer_gone_is_not_a_click_outside() {
    let mut h = upright();
    let at = h.screen(Point::new(340.0, 260.0));
    frame(&mut h, vec![pointer::moved(at)]);
    frame(&mut h, vec![pointer::press(at)]);
    frame(&mut h, vec![pointer::gone()]);
    frame(&mut h, vec![pointer::release(at)]);
    frame(&mut h, Vec::new());
    assert_eq!(h.host.borrow().applied, 0);
}

/// The session, the overlay's reports and the hosts' counters.
struct OverlayHost {
    session: TransformSession<()>,
    kinds: Vec<&'static str>,
    applied: usize,
    discarded: usize,
    view: CanvasView,
}

fn overlay(shown: bool, setup: impl FnOnce(&mut TransformSession<()>)) -> (Fixture, Rc<RefCell<OverlayHost>>) {
    let mut settings = TransformSettings::default();
    settings.show_reference_point = shown;
    let mut session = TransformSession::new(RECT).settings(&settings);
    setup(&mut session);
    let host = Rc::new(RefCell::new(OverlayHost {
        session,
        kinds: Vec::new(),
        applied: 0,
        discarded: 0,

        view: CanvasView::new(egui::Rect::NOTHING, Point::ZERO, 1.0),
    }));
    let shown_host = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
        let settings = TransformSettings::default();
        let output = {
            let host = shown_host.borrow();
            let session = &host.session;
            Canvas::new("Document")
                .view(view)
                .show(ui, |ui, body| TransformOverlay::new("Free Transform", session, &settings).show_at(ui, body.rect))
                .inner
        };
        let mut host = shown_host.borrow_mut();
        host.view = view;
        if let Some(output) = output {
            for edit in output.edits {
                host.kinds.push(match &edit {
                    TransformEdit::Box(EditEvent::Begin { .. }) => "begin",
                    TransformEdit::Box(EditEvent::Update { .. }) => "update",
                    TransformEdit::Box(EditEvent::Commit { .. }) => "commit",
                    _ => "other",
                });
                host.session.apply(edit, 0.0);
            }
            host.applied += usize::from(output.apply);
            host.discarded += usize::from(output.discard);
        }
    });
    fixture.settle();
    (fixture, host)
}

#[trial]
fn a_hidden_reference_point_is_not_drawn_hit_or_picked() {
    let (mut fixture, host) = overlay(false, |_| {});
    let centre = host.borrow().view.to_screen(RECT.center());
    fixture.frame(vec![pointer::moved(centre)]);
    // An Alt press away from the handles places nothing.
    let away = host.borrow().view.to_screen(Point::new(150.0, 110.0));
    fixture.hold(Modifiers::ALT);
    fixture.frame(vec![pointer::moved(away)]);
    fixture.frame(vec![pointer::press(away)]);
    fixture.frame(vec![pointer::release(away)]);
    fixture.hold(Modifiers::NONE);
    fixture.frame(Vec::new());
    assert_eq!(host.borrow().session.state().pivot, RECT.center());
    assert!(host.borrow().kinds.is_empty());
    let parts = fixture.by_role(craft_ui::contract::Role::BUTTON).len();
    assert_eq!(parts, 9, "the whole box, four corners and four edges");
}
