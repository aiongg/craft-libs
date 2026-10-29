use std::cell::RefCell;

use egui::{Context, CursorIcon, Event, Id, Key, Modifiers, PointerButton, Pos2, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::Point;

use super::story::{LineHost, english_names};
use super::{GradientOverlay, LineCursor};
use crate::canvas::guides::GuideHost;
use crate::canvas::tests::TypingField;
use crate::canvas::{CanvasView, GradientLine, Guide, GuideNames, Guides, LineGradient, LinePart, Orientation, RulerUnits};
use crate::composites::gradient::stops::sample;
use crate::composites::{ColorStop, GradientStops, StopRef, StopRow, StopRows, StopRules};
use crate::edit::{self, CancelReason, EditEvent, GestureHost};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};
use craft_theme::Srgba;

const LINE: GradientLine = GradientLine { start: Point::new(100.0, 150.0), end: Point::new(300.0, 150.0) };

/// Four stops at 0, 0.3, 0.6 and 1; the second carries tag 7. Midpoints at 0.15, 0.45 and 0.8.
pub(super) fn gradient() -> LineGradient {
    let color = vec![
        ColorStop::new(0.0, Srgba::rgb(0.0, 0.0, 1.0)),
        ColorStop { tag: 7, ..ColorStop::new(0.3, Srgba::rgb(1.0, 1.0, 0.0)) },
        ColorStop::new(0.6, Srgba::rgb(1.0, 0.0, 0.0)),
        ColorStop::new(1.0, Srgba::rgb(1.0, 1.0, 1.0)),
    ];
    LineGradient { line: LINE, stops: GradientStops { color, opacity: Vec::new() } }
}

/// A host with a 400 × 300 canvas showing document (0, 0)–(400, 300) at zoom 1, the overlay on
/// its gradient and optionally guides under it and a text field's typing run beside it.
struct Host {
    host: LineHost,
    rules: StopRules,
    reverse: bool,
    alt_copies: bool,
    angle_base: f64,
    view: Option<CanvasView>,
    id: Option<Id>,
    events: Vec<EditEvent<LineGradient>>,
    color_requested: Vec<usize>,
    context: Option<LinePart>,
    hovered: Option<LinePart>,
    picked: Option<LinePart>,
    editable: bool,
    units: bool,
    value_text: Option<String>,
    claimed: bool,
    cursors: Vec<LineCursor>,
    guides: Option<GuideHost>,
    guide_events: usize,
    typing: TypingField,
}

fn harness(setup: impl FnOnce(&mut Host)) -> Harness<'static, Host> {
    let mut host = Host {
        host: LineHost::new(gradient()),
        rules: StopRules::default(),
        reverse: false,
        alt_copies: false,
        angle_base: 0.0,
        view: None,
        id: None,
        events: Vec::new(),
        color_requested: Vec::new(),
        context: None,
        hovered: None,
        picked: None,
        editable: true,
        units: false,
        value_text: None,
        claimed: false,
        cursors: Vec::new(),
        guides: None,
        guide_events: 0,
        typing: TypingField::default(),
    };
    setup(&mut host);
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            host.typing.show(ui);
            ui.add_space(40.0);
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
            host.view = Some(view);
            let value = host.host.value.clone();
            let seen = RefCell::new(Vec::new());
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let mut overlay = GradientOverlay::new("gradient", &value.line, &value.stops, english_names());
            if host.units {
                overlay = overlay.units(&px, &px);
            }
            if let Some(text) = &host.value_text {
                overlay = overlay.value(text);
            }
            let r = overlay
                .editable(host.editable)
                .rules(host.rules.clone())
                .reverse(host.reverse)
                .alt_copies(host.alt_copies)
                .angle_base(host.angle_base)
                .cursor(|c| {
                    seen.borrow_mut().push(c);
                    CursorIcon::Grab
                })
                .show(ui, &canvas, &view);
            host.cursors.extend(seen.into_inner());
            host.host.apply(&r);
            host.typing.note(r.event.is_some());
            host.events.extend(r.event.clone());
            host.color_requested.extend(r.color_requested.map(|(i, _)| i));
            host.context = r.context.or(host.context);
            (host.id, host.hovered, host.claimed, host.picked) = (Some(r.id), r.hovered, r.claimed, r.picked);
            if let Some(guides) = &mut host.guides {
                let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
                let names = GuideNames::new("Guides", "Vertical guide", "Horizontal guide");
                let g = Guides::new("guides", &guides.guides, &px, &px, names).interactive(true).show(ui, &canvas, &view);
                host.guide_events += usize::from(g.event.is_some());
                guides.apply(&g);
            }
        },
        host,
    )
}

/// Screen points of the drawn canvas.
struct Spots(CanvasView);

impl Spots {
    fn p(&self, p: Point) -> Pos2 {
        self.0.to_screen(p)
    }

    /// The screen point of location `t` on the test line.
    fn t(&self, t: f64) -> Pos2 {
        self.p(Point::new(100.0 + 200.0 * t, 150.0))
    }
}

fn spots(h: &Harness<'_, Host>) -> Spots {
    Spots(h.state().view.expect("drawn"))
}

fn value(h: &Harness<'_, Host>) -> LineGradient {
    h.state().host.value.clone()
}

fn locations(h: &Harness<'_, Host>) -> Vec<f32> {
    value(h).stops.color.iter().map(|s| s.location).collect()
}

fn selected(h: &Harness<'_, Host>) -> Option<StopRef> {
    GradientOverlay::selected(&h.ctx, h.state().id.expect("drawn"))
}

pub(super) fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

/// Moves to `from`, presses, moves to `to` and releases there in one frame, with `modifiers` held.
fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, modifiers));
    h.event(Event::PointerMoved(from.lerp(to, 0.5)));
    h.event(Event::PointerMoved(to));
    h.event(button(to, false, modifiers));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

/// Presses at `at` and runs, the button held.
fn press(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.event(button(at, true, Modifiers::NONE));
    h.run();
}

fn move_to(h: &mut Harness<'_, Host>, to: Pos2) {
    h.event(Event::PointerMoved(to));
    h.run();
}

fn release(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.event(button(at, false, Modifiers::NONE));
    h.run();
}

/// Queues `events` into one frame and runs it.
fn frame(h: &mut Harness<'_, Host>, events: impl IntoIterator<Item = Event>) {
    h.input_mut().events.extend(events);
    h.step();
}

/// The events' kinds without the updates.
pub(super) fn kinds(events: &[EditEvent<LineGradient>]) -> Vec<&'static str> {
    events
        .iter()
        .filter_map(|e| match e {
            EditEvent::Begin { .. } => Some("begin"),
            EditEvent::Update { .. } => None,
            EditEvent::Commit { .. } => Some("commit"),
            EditEvent::Cancel { .. } => Some("cancel"),
            EditEvent::Set { .. } => Some("set"),
        })
        .collect()
}

#[test]
fn a_press_on_a_stop_selects_it_and_its_drag_past_a_neighbour_is_one_gesture() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    let from = s.t(0.3);
    press(&mut h, from);
    assert_eq!(selected(&h), Some(StopRef::Color(1)), "selected in the press frame");
    assert!(h.state().events.is_empty());
    move_to(&mut h, s.t(0.7) + vec2(0.0, 5.0));
    assert_eq!(selected(&h), Some(StopRef::Color(2)), "the selection follows the stop past its neighbour");
    release(&mut h, s.t(0.7));
    assert_eq!(kinds(&h.state().events), ["begin", "commit"]);
    assert_eq!(locations(&h), [0.0, 0.6, 0.7, 1.0]);
    assert_eq!(value(&h).stops.color[2].tag, 7, "the stop keeps its tag");
    assert!(h.state().cursors.contains(&LineCursor::Drag(LinePart::Stop(1))));
}

#[test]
fn a_click_on_the_line_adds_a_stop_with_the_sampled_colour_and_selects_it() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    let at = s.t(0.725);
    h.event(Event::PointerMoved(at));
    h.run();
    assert_eq!(h.state().hovered, Some(LinePart::Line));
    assert_eq!(h.state().cursors.last(), Some(&LineCursor::Hover(LinePart::Line)));
    drag(&mut h, at, at, Modifiers::NONE);
    assert_eq!(kinds(&h.state().events), ["set"]);
    let stops = value(&h).stops;
    assert_eq!(stops.color[3].location, 0.725);
    assert_eq!(stops.color[3].color, sample(&gradient().stops, StopRows::Split, 0.725));
    assert_eq!(selected(&h), Some(StopRef::Color(3)));
}

#[test]
fn an_end_drag_moves_only_that_end_and_shift_snaps_from_the_angle_base() {
    let to = Point::new(350.0, 200.0);
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    drag(&mut h, s.p(LINE.end), s.p(to), Modifiers::NONE);
    assert_eq!(value(&h).line, GradientLine { start: LINE.start, end: to });
    assert_eq!(kinds(&h.state().events), ["begin", "commit"]);

    for (base, angle) in [(0.0_f64, 0.0_f64), (30.0, 30.0)] {
        let mut h = harness(|host| host.angle_base = base);
        h.run();
        let s = spots(&h);
        drag(&mut h, s.p(LINE.end), s.p(to), Modifiers::SHIFT);
        let line = value(&h).line;
        let len = (to - LINE.start).hypot();
        let want = LINE.start + kurbo::Vec2::from_angle(angle.to_radians()) * len;
        assert_eq!(line.start, LINE.start);
        assert!(line.end.distance(want) < 1e-3, "base {base}: {:?} want {want:?}", line.end);
    }
}

#[test]
fn a_line_drag_moves_both_ends() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    let from = s.t(0.225);
    drag(&mut h, from, from + vec2(20.0, 30.0), Modifiers::NONE);
    let by = kurbo::Vec2::new(20.0, 30.0);
    assert_eq!(value(&h).line, GradientLine { start: LINE.start + by, end: LINE.end + by });
    assert_eq!(value(&h).stops, gradient().stops);
}

#[test]
fn a_stop_torn_off_comes_back_in_the_same_gesture_and_is_removed_on_release_off_the_line() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    press(&mut h, s.t(0.3));
    move_to(&mut h, s.t(0.3) + vec2(0.0, 40.0));
    assert_eq!(locations(&h), [0.0, 0.6, 1.0], "torn off past 36 pt");
    assert_eq!(selected(&h), None, "nothing is selected while it is off");
    assert_eq!(h.state().cursors.last(), Some(&LineCursor::TornOff));
    move_to(&mut h, s.t(0.35) + vec2(0.0, 30.0));
    assert_eq!(locations(&h), [0.0, 0.35, 0.6, 1.0], "back within 36 pt");
    assert_eq!(value(&h).stops.color[1].tag, 7, "with its tag");
    assert_eq!(selected(&h), Some(StopRef::Color(1)));
    move_to(&mut h, s.t(0.35) + vec2(0.0, -50.0));
    release(&mut h, s.t(0.35) + vec2(0.0, -50.0));
    assert_eq!(kinds(&h.state().events), ["begin", "commit"], "one gesture");
    assert_eq!(locations(&h), [0.0, 0.6, 1.0]);

    // At the row's minimum the stop stays and moves.
    let mut h = harness(|host| host.rules.min = 4);
    h.run();
    let s = spots(&h);
    drag(&mut h, s.t(0.3), s.t(0.4) + vec2(0.0, 60.0), Modifiers::NONE);
    assert_eq!(locations(&h), [0.0, 0.4, 0.6, 1.0]);
}

#[test]
fn alt_drags_a_copy_only_with_alt_copies_and_a_copy_is_never_torn_off() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    drag(&mut h, s.t(0.3), s.t(0.7), Modifiers::ALT);
    assert_eq!(locations(&h), [0.0, 0.6, 0.7, 1.0], "Alt is not read");

    let mut h = harness(|host| host.alt_copies = true);
    h.run();
    let s = spots(&h);
    drag(&mut h, s.t(0.3), s.t(0.7), Modifiers::ALT);
    assert_eq!(locations(&h), [0.0, 0.3, 0.6, 0.7, 1.0]);
    assert_eq!(value(&h).stops.color[3].tag, 7, "the copy keeps its source's tag");
    assert_eq!(selected(&h), Some(StopRef::Color(3)), "the copy is selected");
    drag(&mut h, s.t(0.3), s.t(0.2) + vec2(0.0, 80.0), Modifiers::ALT);
    assert_eq!(value(&h).stops.color.len(), 6, "an Alt copy dragged off the line stays");
}

#[test]
fn a_midpoint_drag_clamps_to_the_range() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    drag(&mut h, s.t(0.45), s.t(0.595), Modifiers::NONE);
    assert_eq!(value(&h).stops.color[1].midpoint, 0.95);
    assert_eq!(locations(&h), [0.0, 0.3, 0.6, 1.0]);
    assert_eq!(selected(&h), Some(StopRef::Midpoint(StopRow::Color, 1)));
}

/// Two clicks at `at`, one per frame: frames are a quarter second apart, and egui's double-click
/// window is 0.3 s. Then frames enough that the next click starts afresh.
fn double_click(h: &mut Harness<'_, Host>, at: Pos2) {
    frame(h, [Event::PointerMoved(at), button(at, true, Modifiers::NONE), button(at, false, Modifiers::NONE)]);
    frame(h, [button(at, true, Modifiers::NONE), button(at, false, Modifiers::NONE)]);
    for _ in 0..3 {
        h.step();
    }
}

#[test]
fn a_double_click_on_an_end_asks_for_the_colour_of_the_stop_there() {
    for (reverse, start, end) in [(false, 0, 3), (true, 3, 0)] {
        let mut h = harness(|host| host.reverse = reverse);
        h.run();
        let s = spots(&h);
        double_click(&mut h, s.p(LINE.start));
        double_click(&mut h, s.p(LINE.end));
        assert_eq!(h.state().color_requested, [start, end], "reverse {reverse}");
        assert!(h.state().events.is_empty(), "clicks on the ends change nothing");
    }
}

#[test]
fn a_double_click_on_a_stop_asks_for_its_colour_and_a_secondary_click_reports_the_part() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    double_click(&mut h, s.t(0.6));
    assert_eq!(h.state().color_requested, [2]);
    let mid = s.t(0.15);
    let secondary = |pressed| Event::PointerButton { pos: mid, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    frame(&mut h, [Event::PointerMoved(mid), secondary(true), secondary(false)]);
    h.run();
    assert_eq!(h.state().context, Some(LinePart::Midpoint(0)));
    assert_eq!(selected(&h), Some(StopRef::Midpoint(StopRow::Color, 0)));
}

#[test]
fn esc_mid_drag_cancels_and_the_rest_of_the_press_does_nothing() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    GradientOverlay::select(&h.ctx, h.state().id.expect("drawn"), Some(StopRef::Color(3)));
    press(&mut h, s.t(0.3));
    move_to(&mut h, s.t(0.7));
    assert_eq!(locations(&h), [0.0, 0.6, 0.7, 1.0], "the host shows the drag");
    h.key_press(Key::Escape);
    h.run();
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })));
    assert_eq!(value(&h), gradient());
    assert_eq!(selected(&h), Some(StopRef::Color(3)), "the selection goes back");
    let count = h.state().events.len();
    move_to(&mut h, s.t(0.9));
    release(&mut h, s.t(0.9));
    assert_eq!(h.state().events.len(), count, "the rest of the press does nothing");
    assert_eq!(value(&h), gradient());
}

#[test]
fn a_quick_drag_in_one_frame_starts_at_the_press() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    // The release is over the end handle; the press was on the stop.
    let (from, to) = (s.t(0.3), s.t(0.99));
    frame(&mut h, [Event::PointerMoved(from), button(from, true, Modifiers::NONE), Event::PointerMoved(to), button(to, false, Modifiers::NONE)]);
    h.run();
    assert_eq!(locations(&h), [0.0, 0.6, 0.99, 1.0]);
    assert_eq!(value(&h).line, LINE);
}

#[test]
fn a_press_off_the_parts_or_with_command_is_left_to_the_host() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    let off = s.p(Point::new(200.0, 250.0));
    press(&mut h, off);
    assert!(!h.state().claimed);
    assert_eq!(h.state().hovered, None);
    release(&mut h, off);
    let stop = s.t(0.3);
    h.event(Event::ModifiersChanged(Modifiers::COMMAND));
    h.event(Event::PointerMoved(stop));
    h.run();
    assert_eq!(h.state().hovered, None, "⌘ presses are the host's");
    h.event(button(stop, true, Modifiers::COMMAND));
    h.run();
    assert!(!h.state().claimed);
    move_to(&mut h, s.t(0.5));
    release(&mut h, s.t(0.5));
    assert!(h.state().events.is_empty());
    assert_eq!(value(&h), gradient());
}

#[test]
fn the_overlay_takes_a_press_on_a_stop_before_a_guide_under_it() {
    let mut h = harness(|host| host.guides = Some(GuideHost::new(vec![Guide::new(1, Orientation::Vertical, 160.0)])));
    h.run();
    let s = spots(&h);
    drag(&mut h, s.t(0.3), s.t(0.4), Modifiers::NONE);
    assert_eq!(locations(&h), [0.0, 0.4, 0.6, 1.0]);
    assert_eq!(h.state().guide_events, 0);
    assert_eq!(h.state().guides.as_ref().map(|g| g.guides[0].position), Some(160.0));
}

/// A press on a stop while a text field's typing run has the focus waits for the run's Commit
/// and drags from the value it left; a click on the line adds its stop after the Commit too.
#[test]
fn a_press_during_a_typing_run_begins_after_the_runs_commit() {
    let mut h = harness(|host| host.typing.on = true);
    h.step();
    let s = spots(&h);
    let from = s.t(0.3);
    h.event(Event::PointerMoved(from));
    h.step();
    h.event(button(from, true, Modifiers::NONE));
    h.step();
    assert!(h.state().events.is_empty(), "the press waits: {:?}", h.state().events);
    assert!(h.state().claimed, "the overlay holds the press meanwhile");
    h.event(Event::PointerMoved(s.t(0.7)));
    for _ in 0..4 {
        h.step();
    }
    release(&mut h, s.t(0.7));
    assert!(h.state().typing.commit_came_first());
    assert_eq!(kinds(&h.state().events), ["begin", "commit"]);
    assert_eq!(locations(&h), [0.0, 0.6, 0.7, 1.0]);

    let mut h = harness(|host| host.typing.on = true);
    h.step();
    let s = spots(&h);
    let at = s.t(0.725);
    h.event(Event::PointerMoved(at));
    h.step();
    // Pressed in one frame and released in the next: a release in the press frame would move the
    // focus off the field before the overlay is drawn (egui's click elsewhere), and nothing waits.
    frame(&mut h, [button(at, true, Modifiers::NONE)]);
    assert!(h.state().events.is_empty(), "the click waits");
    frame(&mut h, [button(at, false, Modifiers::NONE)]);
    h.run();
    assert!(h.state().typing.commit_came_first());
    assert_eq!(kinds(&h.state().events), ["set"]);
    assert_eq!(value(&h).stops.color.len(), 5);
}

/// A document whose undo history holds whole snapshots.
pub(super) struct History {
    pub(super) now: LineGradient,
    pub(super) undo: Vec<LineGradient>,
}

impl History {
    pub(super) fn apply(&mut self, value: LineGradient) {
        self.undo.push(self.now.clone());
        self.now = value;
    }
}

impl edit::UndoMarks for History {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(before) = self.undo.get(mark) {
            self.now = before.clone();
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

type Shared = (History, GestureHost<History>, Option<LineGradient>, Option<CanvasView>, Vec<EditEvent<LineGradient>>);

/// Another control's Set during a stop drag (a gradient editor's, made from the stops it shows)
/// ends the drag; the drag goes on from the host's value at its next move, as a gesture of its
/// own.
#[test]
fn a_drag_the_host_ended_for_another_controls_set_goes_on_from_the_hosts_value() {
    let state: Shared = (History { now: gradient(), undo: Vec::new() }, GestureHost::new(), None, None, Vec::new());
    let mut h = Harness::<Shared>::builder()
        .with_size([480.0, 400.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, (doc, host, set, view_out, events): &mut Shared| {
                let ctx = ui.ctx().clone();
                ui.add_space(40.0);
                let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
                let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
                *view_out = Some(view);
                let value = doc.now.clone();
                let r = GradientOverlay::new("gradient", &value.line, &value.stops, english_names()).show(ui, &canvas, &view);
                if let Some(event) = r.event {
                    events.push(event.clone());
                    host.handle(&ctx, doc, event, History::apply);
                }
                if let Some(value) = set.take() {
                    host.handle(&ctx, doc, EditEvent::set(value, Modifiers::NONE), History::apply);
                }
            },
            state,
        );
    h.run();
    let view = h.state().3.expect("drawn");
    let at = |t: f64| view.to_screen(Point::new(100.0 + 200.0 * t, 150.0));
    h.event(Event::PointerMoved(at(0.3)));
    h.event(button(at(0.3), true, Modifiers::NONE));
    h.run();
    h.event(Event::PointerMoved(at(0.5)));
    h.run();
    assert_eq!(h.state().0.now.stops.color[1].location, 0.5);
    h.event(Event::PointerMoved(at(0.7)));
    h.run();
    assert_eq!(h.state().0.now.stops.color[2].location, 0.7, "past its neighbour");
    // Another control recolours the last stop.
    let mut recoloured = h.state().0.now.clone();
    recoloured.stops.color[3].color = Srgba::rgb(0.0, 1.0, 0.0);
    // The same Set also moves the dragged stop: the pointer wins for its location.
    recoloured.stops.color[2].location = 0.65;
    h.state_mut().2 = Some(recoloured);
    h.run();
    assert_eq!(h.state().0.now.stops.color[3].color, Srgba::rgb(0.0, 1.0, 0.0));
    h.event(Event::PointerMoved(at(0.75)));
    h.run();
    h.event(button(at(0.75), false, Modifiers::NONE));
    h.run();
    let doc = &h.state().0.now;
    let locations: Vec<f32> = doc.stops.color.iter().map(|s| s.location).collect();
    assert_eq!(locations, [0.0, 0.6, 0.75, 1.0], "the drag goes on with its stop, whose location follows the pointer, not the Set");
    assert_eq!(doc.stops.color[2].tag, 7);
    assert_eq!(doc.stops.color[3].color, Srgba::rgb(0.0, 1.0, 0.0), "the Set's other changes stay");
    let begins: Vec<_> = h.state().4.iter().filter(|e| matches!(e, EditEvent::Begin { .. })).map(|e| e.id()).collect();
    assert_eq!(begins.len(), 2, "a gesture of its own: {:?}", kinds(&h.state().4));
    assert_ne!(begins[0], begins[1]);
    assert!(matches!(h.state().4.last(), Some(EditEvent::Commit { .. })));
}

fn click(h: &mut Harness<'_, Host>, at: Pos2) {
    press(h, at);
    release(h, at);
}

fn key(h: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

fn picked(h: &Harness<'_, Host>) -> Option<LinePart> {
    h.state().picked
}

/// Tab into the group, then Home and Page Down to `want`.
fn pick(h: &mut Harness<'_, Host>, want: LinePart) {
    if !h.ctx.memory(|m| m.has_focus(h.state().id.expect("drawn"))) {
        key(h, Modifiers::NONE, Key::Tab);
    }
    key(h, Modifiers::NONE, Key::Home);
    for _ in 0..12 {
        if picked(h) == Some(want) {
            break;
        }
        key(h, Modifiers::NONE, Key::PageDown);
    }
    assert_eq!(picked(h), Some(want));
    h.state_mut().events.clear();
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn the_page_keys_pick_the_line_the_start_the_stops_and_midpoints_by_location_then_the_end() {
    let mut h = harness(|_| {});
    h.run();
    key(&mut h, Modifiers::NONE, Key::Tab);
    assert_eq!(picked(&h), Some(LinePart::Line), "gaining focus picks the first part");
    let order = [
        LinePart::Start,
        LinePart::Stop(0),
        LinePart::Midpoint(0),
        LinePart::Stop(1),
        LinePart::Midpoint(1),
        LinePart::Stop(2),
        LinePart::Midpoint(2),
        LinePart::Stop(3),
        LinePart::End,
    ];
    let mut selected_then = selected(&h);
    for part in order {
        key(&mut h, Modifiers::NONE, Key::PageDown);
        assert_eq!(picked(&h), Some(part));
        selected_then = match part {
            LinePart::Stop(i) => Some(StopRef::Color(i)),
            LinePart::Midpoint(i) => Some(StopRef::Midpoint(StopRow::Color, i)),
            _ => selected_then,
        };
        assert_eq!(selected(&h), selected_then, "{part:?}: picking a stop or midpoint selects it, a handle keeps the selection");
    }
    key(&mut h, Modifiers::NONE, Key::PageDown);
    assert_eq!(picked(&h), Some(LinePart::End), "the end is last");
    key(&mut h, Modifiers::NONE, Key::PageUp);
    assert_eq!(picked(&h), Some(LinePart::Stop(3)));
    assert!(h.state().events.is_empty(), "picking edits nothing");
}

#[test]
fn left_and_right_move_a_stop_or_midpoint_by_one_percent_and_ten_with_shift_whichever_way_the_line_runs() {
    for reverse in [false, true] {
        let mut h = harness(|host| host.reverse = reverse);
        h.run();
        pick(&mut h, LinePart::Stop(1));
        key(&mut h, Modifiers::NONE, Key::ArrowRight);
        assert!(near(locations(&h)[1], 0.31), "reverse {reverse}: {:?}", locations(&h));
        key(&mut h, Modifiers::SHIFT, Key::ArrowLeft);
        assert!(near(locations(&h)[1], 0.21), "Left lowers the location, reverse {reverse}: {:?}", locations(&h));
        assert_eq!(value(&h).stops.color[1].tag, 7, "the stop is the one moved");
        assert_eq!(kinds(&h.state().events), ["set", "set"]);

        h.state_mut().events.clear();
        for (modifiers, k) in [
            (Modifiers::NONE, Key::ArrowUp),
            (Modifiers::NONE, Key::ArrowDown),
            (Modifiers::ALT, Key::ArrowRight),
            (Modifiers::COMMAND, Key::ArrowLeft),
        ] {
            key(&mut h, modifiers, k);
        }
        assert!(h.state().events.is_empty(), "Up, Down and the Alt and Cmd arrows are the host's");

        pick(&mut h, LinePart::Midpoint(0));
        key(&mut h, Modifiers::NONE, Key::ArrowRight);
        assert!(near(value(&h).stops.color[0].midpoint, 0.51));
        key(&mut h, Modifiers::SHIFT, Key::ArrowLeft);
        assert!(near(value(&h).stops.color[0].midpoint, 0.41));
        for _ in 0..6 {
            key(&mut h, Modifiers::SHIFT, Key::ArrowLeft);
        }
        assert!(near(value(&h).stops.color[0].midpoint, 0.05), "a midpoint stays in its range");
    }
}

#[test]
fn a_stop_moved_past_its_neighbour_by_the_keys_stays_picked_and_selected() {
    let mut h = harness(|_| {});
    h.run();
    pick(&mut h, LinePart::Stop(1));
    for _ in 0..40 {
        key(&mut h, Modifiers::NONE, Key::ArrowRight);
    }
    assert!(near(locations(&h)[2], 0.7), "{:?}", locations(&h));
    assert_eq!(value(&h).stops.color[2].tag, 7);
    assert_eq!(picked(&h), Some(LinePart::Stop(2)));
    assert_eq!(selected(&h), Some(StopRef::Color(2)));
}

#[test]
fn the_arrows_move_an_end_or_the_line_in_document_units() {
    let mut h = harness(|_| {});
    h.run();
    pick(&mut h, LinePart::End);
    key(&mut h, Modifiers::NONE, Key::ArrowRight);
    key(&mut h, Modifiers::SHIFT, Key::ArrowDown);
    assert_eq!((value(&h).line.start, value(&h).line.end), (LINE.start, Point::new(301.0, 160.0)));
    pick(&mut h, LinePart::Start);
    key(&mut h, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(value(&h).line.start, Point::new(99.0, 150.0));
    pick(&mut h, LinePart::Line);
    key(&mut h, Modifiers::SHIFT, Key::ArrowUp);
    assert_eq!((value(&h).line.start, value(&h).line.end), (Point::new(99.0, 140.0), Point::new(301.0, 150.0)));
    assert_eq!(kinds(&h.state().events), ["set"]);
    assert_eq!(value(&h).stops, gradient().stops, "the stops stay");
}

#[test]
fn delete_and_backspace_remove_a_stop_and_pick_the_one_that_takes_its_index_but_not_below_the_minimum() {
    let mut h = harness(|_| {});
    h.run();
    pick(&mut h, LinePart::Stop(1));
    key(&mut h, Modifiers::NONE, Key::Delete);
    assert_eq!(locations(&h), [0.0, 0.6, 1.0]);
    assert_eq!((picked(&h), selected(&h)), (Some(LinePart::Stop(1)), Some(StopRef::Color(1))));
    key(&mut h, Modifiers::NONE, Key::Backspace);
    assert_eq!(locations(&h), [0.0, 1.0]);
    assert_eq!(kinds(&h.state().events), ["set", "set"]);
    h.state_mut().events.clear();
    key(&mut h, Modifiers::NONE, Key::Delete);
    assert_eq!(locations(&h), [0.0, 1.0], "the row keeps its minimum");
    assert!(h.state().events.is_empty());

    let mut h = harness(|_| {});
    h.run();
    pick(&mut h, LinePart::Midpoint(1));
    key(&mut h, Modifiers::NONE, Key::Delete);
    assert!(h.state().events.is_empty(), "Delete acts on a stop, not a midpoint");
}

#[test]
fn insert_and_cmd_enter_add_a_stop_halfway_after_the_picked_one_and_pick_it() {
    let mut h = harness(|_| {});
    h.run();
    pick(&mut h, LinePart::Stop(1));
    key(&mut h, Modifiers::NONE, Key::Insert);
    assert!(near(locations(&h)[2], 0.45), "{:?}", locations(&h));
    assert_eq!((picked(&h), selected(&h)), (Some(LinePart::Stop(2)), Some(StopRef::Color(2))));
    key(&mut h, Modifiers::COMMAND, Key::Enter);
    assert!(near(locations(&h)[3], 0.525), "{:?}", locations(&h));
    assert_eq!(picked(&h), Some(LinePart::Stop(3)));
    assert_eq!(kinds(&h.state().events), ["set", "set"]);

    let mut h = harness(|_| {});
    h.run();
    pick(&mut h, LinePart::Line);
    key(&mut h, Modifiers::NONE, Key::Insert);
    assert!(near(locations(&h)[3], 0.8), "with nothing picked, across the widest gap: {:?}", locations(&h));
    let colored = value(&h).stops.color[3].color;
    assert_eq!(colored, Srgba { a: 1.0, ..sample(&gradient().stops, StopRows::Split, 0.8) });
}

#[test]
fn enter_and_space_on_a_colour_stop_or_an_end_ask_for_the_colour() {
    let mut h = harness(|_| {});
    h.run();
    pick(&mut h, LinePart::Stop(2));
    key(&mut h, Modifiers::NONE, Key::Enter);
    key(&mut h, Modifiers::NONE, Key::Space);
    pick(&mut h, LinePart::End);
    key(&mut h, Modifiers::NONE, Key::Enter);
    pick(&mut h, LinePart::Start);
    key(&mut h, Modifiers::NONE, Key::Space);
    pick(&mut h, LinePart::Midpoint(0));
    key(&mut h, Modifiers::NONE, Key::Enter);
    pick(&mut h, LinePart::Line);
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(h.state().color_requested, [2, 2, 3, 0], "a stop, an end (the stop there), not a midpoint or the line");
    assert!(h.state().events.is_empty());

    let mut h = harness(|host| host.reverse = true);
    h.run();
    pick(&mut h, LinePart::Start);
    key(&mut h, Modifiers::NONE, Key::Enter);
    assert_eq!(h.state().color_requested, [3], "reversed, the start is the last stop");
}

#[test]
fn the_release_of_a_drag_gives_the_group_the_keys() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    drag(&mut h, s.t(0.3), s.t(0.5), Modifiers::NONE);
    assert_eq!(picked(&h), Some(LinePart::Stop(1)));
    assert!(near(locations(&h)[1], 0.5));
    key(&mut h, Modifiers::NONE, Key::ArrowRight);
    assert!(near(locations(&h)[1], 0.51), "the key acts on the dragged stop: {:?}", locations(&h));

    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    let end = s.t(1.0);
    drag(&mut h, end, end + vec2(0.0, 30.0), Modifiers::NONE);
    key(&mut h, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(value(&h).line.end, Point::new(300.0, 181.0));
}

/// The stroked shapes of the last frame in `color`.
fn painted(h: &Harness<'_, Host>, color: egui::Color32) -> Vec<egui::Shape> {
    fn collect(shape: &egui::Shape, color: egui::Color32, out: &mut Vec<egui::Shape>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, color, out)),
            egui::Shape::LineSegment { stroke, .. } if stroke.color == color => out.push(shape.clone()),
            egui::Shape::Rect(r) if r.stroke.color == color && r.stroke.width > 0.0 => out.push(shape.clone()),
            egui::Shape::Circle(c) if c.stroke.color == color && c.stroke.width > 0.0 => out.push(shape.clone()),
            egui::Shape::Path(p) if p.stroke.color == egui::epaint::ColorMode::Solid(color) && p.stroke.width > 0.0 => out.push(shape.clone()),
            _ => {}
        }
    }
    let mut out = Vec::new();
    h.output().shapes.iter().for_each(|c| collect(&c.shape, color, &mut out));
    out
}

fn style(h: &Harness<'_, Host>) -> super::GradientLineStyle {
    GradientOverlay::style(&crate::theme::current(&h.ctx))
}

#[test]
fn the_focus_marks_show_after_keyboard_input_only() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    let focus = style(&h).focus.color;
    click(&mut h, s.t(0.3));
    assert!(painted(&h, focus).is_empty(), "no ring after the pointer");
    key(&mut h, Modifiers::NONE, Key::PageDown);
    assert_eq!(picked(&h), Some(LinePart::Midpoint(1)));
    assert_eq!(painted(&h, focus).len(), 1, "a ring round the picked diamond");
    key(&mut h, Modifiers::NONE, Key::Home);
    assert_eq!(painted(&h, focus).len(), 2, "two focus lines beside the picked line");
    key(&mut h, Modifiers::NONE, Key::PageDown);
    assert_eq!(painted(&h, focus).len(), 1, "a ring round the start handle");
}

#[test]
fn a_picked_end_is_drawn_selected() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    let selected_color = style(&h).selected;
    let at_end =
        |h: &Harness<'_, Host>| painted(h, selected_color).iter().any(|shape| matches!(shape, egui::Shape::Circle(c) if c.center == s.t(1.0)));
    assert!(!at_end(&h));
    click(&mut h, s.t(1.0));
    assert_eq!(picked(&h), Some(LinePart::End));
    assert!(at_end(&h), "the end the press picked is selected");
    click(&mut h, s.t(0.3));
    assert!(!at_end(&h), "picking a stop clears it");
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

/// The group's graphics objects: name, value and `selected`.
fn objects(h: &Harness<'_, Host>) -> Vec<(String, String, bool)> {
    let group = h.get_by_label("Gradient");
    descendants(group.accesskit_node())
        .into_iter()
        .filter(|n| n.role() == egui::accesskit::Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default(), n.data().is_selected() == Some(true)))
        .collect()
}

#[test]
fn the_group_and_parts_are_named_and_valued_as_the_bar_reads_them() {
    let mut h = harness(|host| host.units = true);
    h.run();
    GradientOverlay::select(&h.ctx, h.state().id.expect("drawn"), Some(StopRef::Color(1)));
    h.run();
    let group = h.get_by_label("Gradient");
    assert_eq!(group.accesskit_node().role(), egui::accesskit::Role::Group);
    assert_eq!(group.accesskit_node().value().as_deref(), Some("4 stops"));
    let want = [
        ("Gradient line", "4 stops", false),
        ("Start point", "100.0 px, 150.0 px", false),
        ("Color stop 1 of 4", "0%, #0000FF", false),
        ("Midpoint 1", "50%", false),
        ("Color stop 2 of 4", "30%, #FFFF00", true),
        ("Midpoint 2", "50%", false),
        ("Color stop 3 of 4", "60%, #FF0000", false),
        ("Midpoint 3", "50%", false),
        ("Color stop 4 of 4", "100%, #FFFFFF", false),
        ("End point", "300.0 px, 150.0 px", false),
    ];
    let want: Vec<_> = want.iter().map(|(n, v, s)| (n.to_string(), v.to_string(), *s)).collect();
    assert_eq!(objects(&h), want);

    key(&mut h, Modifiers::NONE, Key::Tab);
    key(&mut h, Modifiers::NONE, Key::End);
    let group = h.get_by_label("Gradient");
    assert_eq!(group.accesskit_node().active_descendant().and_then(|n| n.label()).as_deref(), Some("End point"));
}

#[test]
fn the_group_value_and_the_end_values_are_the_hosts_and_the_ends_have_none_without_units() {
    let mut h = harness(|host| host.value_text = Some("Radial, 4 stops".into()));
    h.run();
    let group = h.get_by_label("Gradient");
    assert_eq!(group.accesskit_node().value().as_deref(), Some("Radial, 4 stops"));
    let o = objects(&h);
    assert_eq!((o[0].1.as_str(), o[1].1.as_str(), o[9].1.as_str()), ("Radial, 4 stops", "", ""));
}

#[test]
fn a_not_editable_overlay_has_its_parts_but_no_tab_stop_and_no_selection() {
    let mut h = harness(|host| host.editable = false);
    h.run();
    GradientOverlay::select(&h.ctx, h.state().id.expect("drawn"), Some(StopRef::Color(1)));
    h.run();
    let o = objects(&h);
    assert_eq!(o.len(), 10);
    assert!(o.iter().all(|(_, _, selected)| !selected));
    let group = h.get_by_label("Gradient");
    assert!(!group.accesskit_node().data().supports_action(egui::accesskit::Action::Focus), "no Tab stop");
    key(&mut h, Modifiers::NONE, Key::Tab);
    key(&mut h, Modifiers::NONE, Key::PageDown);
    key(&mut h, Modifiers::NONE, Key::Insert);
    assert_eq!(picked(&h), None);
    assert!(h.state().events.is_empty(), "the keys do nothing");
}

/// Presses and releases at `at` in one frame, the pointer already there.
fn tap(h: &mut Harness<'_, Host>, at: Pos2) {
    frame(h, [Event::PointerMoved(at), button(at, true, Modifiers::NONE), button(at, false, Modifiers::NONE)]);
    h.run();
}

#[test]
fn a_one_frame_click_on_the_line_adds_a_stop_like_a_two_frame_click() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    tap(&mut h, s.t(0.75));
    assert_eq!(kinds(&h.state().events), ["set"]);
    assert_eq!(locations(&h), [0.0, 0.3, 0.6, 0.75, 1.0]);
    assert_eq!(selected(&h), Some(StopRef::Color(3)));
}

#[test]
fn a_one_frame_click_picks_a_stop_a_midpoint_and_an_end() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    for (at, part, sel) in [
        (s.t(0.3), LinePart::Stop(1), Some(StopRef::Color(1))),
        (s.t(0.45), LinePart::Midpoint(1), Some(StopRef::Midpoint(StopRow::Color, 1))),
        (s.t(1.0), LinePart::End, Some(StopRef::Midpoint(StopRow::Color, 1))),
    ] {
        tap(&mut h, at);
        assert_eq!((picked(&h), selected(&h)), (Some(part), sel));
    }
    assert!(h.state().events.is_empty(), "picking edits nothing");
}

#[test]
fn a_one_frame_double_click_asks_for_the_colour_and_a_one_frame_secondary_click_reports_the_part() {
    let mut h = harness(|_| {});
    h.run();
    let s = spots(&h);
    let at = s.t(0.6);
    frame(
        &mut h,
        [
            Event::PointerMoved(at),
            button(at, true, Modifiers::NONE),
            button(at, false, Modifiers::NONE),
            button(at, true, Modifiers::NONE),
            button(at, false, Modifiers::NONE),
        ],
    );
    h.run();
    assert_eq!(h.state().color_requested, [2]);
    let at = s.t(0.3);
    let secondary = |pressed| Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    frame(&mut h, [Event::PointerMoved(at), secondary(true), secondary(false)]);
    h.run();
    assert_eq!(h.state().context, Some(LinePart::Stop(1)));
    assert_eq!(selected(&h), Some(StopRef::Color(1)));
}
