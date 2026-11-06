//! Tests of the Slice tools and the slice overlay. PhotoCraft's
//! `tools/slice_ui/tests.rs` cases move here.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, Sense, Shape, pos2, vec2};
use egui_kittest::Harness;
use kurbo::{Point, Rect};

use super::{SliceCommand, SliceEvent, SliceItem, SliceMode, SliceOrigin, SliceOverlay, SliceRefusal, SliceTool, SliceToolSettings};
use craft_ui::canvas::{CanvasId, CanvasView, Constraint, SnapKind, SnapTarget, SnapTargets, ToolHost};
use craft_ui::edit::CancelReason;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
use craft_ui::testing::canvas;
use craft_ui::theme::{Base, Tokens, TokensOf as _, current, install};

struct Host {
    tool: SliceTool,
    mode: SliceMode,
    items: Vec<SliceItem>,
    selected: Option<usize>,
    locked: bool,
    hide_auto: bool,
    settings: SliceToolSettings,
    view: CanvasView,
    canvas: Option<CanvasId>,
    /// The host's snap targets for the whole run.
    targets: Option<SnapTargets>,
    focus: bool,
    /// Every event of every pass.
    events: Vec<SliceEvent>,
    /// The keys the host took from the app.
    taken: Vec<Key>,
}

fn item(number: usize, rect: [f64; 4], origin: SliceOrigin) -> SliceItem {
    SliceItem { number, id: (origin != SliceOrigin::Auto).then_some(number as u32), rect: Rect::new(rect[0], rect[1], rect[2], rect[3]), origin }
}

fn user(number: usize, rect: [f64; 4]) -> SliceItem {
    item(number, rect, SliceOrigin::User)
}

fn harness(mode: SliceMode, items: Vec<SliceItem>, selected: Option<usize>) -> Harness<'static, Host> {
    let rect = egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0));
    let host = Host {
        tool: SliceTool::new(),
        mode,
        items,
        selected,
        locked: false,
        hide_auto: false,
        settings: SliceToolSettings::default(),
        view: CanvasView::new(rect, Point::new(200.0, 150.0), 1.0),
        canvas: None,
        targets: None,
        focus: false,
        events: Vec::new(),
        taken: Vec::new(),
    };
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let id = CanvasId::new(ui, "slice");
                host.canvas = Some(id);
                let canvas = id.interact(ui, host.view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                if std::mem::take(&mut host.focus) {
                    canvas.request_focus();
                }
                host.tool
                    .mode(host.mode)
                    .items(&host.items)
                    .selected(host.selected)
                    .locked(host.locked)
                    .hide_auto(host.hide_auto)
                    .settings(host.settings);
                let mut tool_host = ToolHost::new(id);
                if let Some(targets) = &host.targets {
                    tool_host = tool_host.snap(targets);
                }
                let response = tool_host.show(ui, &host.view, &canvas, &mut host.tool);
                host.taken.extend(response.took_key.map(|k| k.key));
                for event in host.tool.take_events() {
                    match &event {
                        SliceEvent::Select { number, add: false, .. } => host.selected = *number,
                        SliceEvent::Move { number, event: EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } } => {
                            if let Some(i) = host.items.iter_mut().find(|i| i.number == *number) {
                                i.rect = *value;
                            }
                        }
                        _ => {}
                    }
                    host.events.push(event);
                }
            },
            host,
        );
    h.step();
    h
}

fn button(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn move_to(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.step();
}

fn press(h: &mut Harness<'_, Host>, at: Pos2) {
    move_to(h, at);
    h.event(button(at, true));
    h.step();
}

fn release(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(button(at, false));
    h.step();
}

fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    press(h, from);
    move_to(h, from.lerp(to, 0.5));
    move_to(h, to);
    release(h, to);
}

fn click(h: &mut Harness<'_, Host>, at: Pos2) {
    press(h, at);
    release(h, at);
}

fn key(h: &mut Harness<'_, Host>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

fn focus(h: &mut Harness<'_, Host>) {
    h.state_mut().focus = true;
    h.step();
}

fn phase<T>(event: &EditEvent<T>) -> &'static str {
    match event {
        EditEvent::Begin { .. } => "begin",
        EditEvent::Update { .. } => "update",
        EditEvent::Commit { .. } => "commit",
        EditEvent::Cancel { .. } => "cancel",
        EditEvent::Set { .. } => "set",
        _ => "other",
    }
}

fn value<T: Copy>(event: &EditEvent<T>) -> Option<T> {
    match event {
        EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } => Some(*value),
        _ => None,
    }
}

fn draws<'a>(h: &'a Harness<'_, Host>) -> Vec<&'a EditEvent<Rect>> {
    h.state().events.iter().filter_map(|e| if let SliceEvent::Draw(e) = e { Some(e) } else { None }).collect()
}

fn moves<'a>(h: &'a Harness<'_, Host>) -> Vec<&'a EditEvent<Rect>> {
    h.state().events.iter().filter_map(|e| if let SliceEvent::Move { event, .. } = e { Some(event) } else { None }).collect()
}

fn resizes<'a>(h: &'a Harness<'_, Host>) -> Vec<&'a EditEvent<Rect>> {
    h.state().events.iter().filter_map(|e| if let SliceEvent::Resize { event, .. } = e { Some(event) } else { None }).collect()
}

fn selects(h: &Harness<'_, Host>) -> Vec<Option<usize>> {
    h.state().events.iter().filter_map(|e| if let SliceEvent::Select { number, .. } = e { Some(*number) } else { None }).collect()
}

fn commands(h: &Harness<'_, Host>) -> Vec<SliceCommand> {
    h.state().events.iter().filter_map(|e| if let SliceEvent::Command(c) = e { Some(*c) } else { None }).collect()
}

fn committed<T: Copy>(events: &[&EditEvent<T>]) -> Option<T> {
    events.iter().rev().find(|e| matches!(e, EditEvent::Commit { .. })).and_then(|e| value(e))
}

// Normal: a 2 × 2 px drag selects the slice under the press (`Select`), a 3 × 1 one draws; Fixed
// Size draws its size on a click.
#[test]
fn slice_draw_needs_a_drag_over_two_pixels_except_fixed_size() {
    let mut h = harness(SliceMode::Slice, vec![user(1, [0.0, 0.0, 100.0, 100.0])], None);
    drag(&mut h, pos2(50.0, 50.0), pos2(52.0, 52.0));
    assert!(draws(&h).is_empty(), "{:?}", h.state().events);
    assert_eq!(selects(&h), [Some(1)]);

    drag(&mut h, pos2(200.0, 200.0), pos2(203.0, 201.0));
    let phases: Vec<_> = draws(&h).into_iter().map(phase).collect();
    assert_eq!(phases.first(), Some(&"begin"), "{phases:?}");
    assert_eq!(phases.last(), Some(&"commit"), "{phases:?}");
    assert_eq!(committed(&draws(&h)), Some(Rect::new(200.0, 200.0, 203.0, 201.0)));

    h.state_mut().events.clear();
    h.state_mut().settings = SliceToolSettings::new(Constraint::Size(20.0, 10.0));
    click(&mut h, pos2(300.0, 100.0));
    let phases: Vec<_> = draws(&h).into_iter().map(phase).collect();
    assert_eq!(phases, ["begin", "update", "commit"]);
    assert_eq!(committed(&draws(&h)), Some(Rect::new(300.0, 100.0, 320.0, 110.0)));
}

// A press on the visible piece of a slice covered by a later one moves the whole slice (one
// `Move` with its full rect).
#[test]
fn slice_select_moves_whole_slice_when_press_hits_a_covered_piece() {
    let mut h = harness(SliceMode::Select, vec![user(1, [0.0, 0.0, 100.0, 100.0]), user(2, [50.0, 50.0, 150.0, 150.0])], None);
    drag(&mut h, pos2(20.0, 20.0), pos2(30.0, 20.0));
    assert_eq!(selects(&h), [Some(1)]);
    assert_eq!(committed(&moves(&h)), Some(Rect::new(10.0, 0.0, 110.0, 100.0)));
    assert!(matches!(h.state().events.last(), Some(SliceEvent::Move { number: 1, .. })));

    // Where both cover the point, the later slice is on top.
    h.state_mut().events.clear();
    drag(&mut h, pos2(100.0, 100.0), pos2(100.0, 110.0));
    assert_eq!(selects(&h), [Some(2)]);
    assert_eq!(committed(&moves(&h)), Some(Rect::new(50.0, 60.0, 150.0, 160.0)));
}

// Dragging the right handle of a selected user slice reports `Resize`; past the left edge it
// flips; layer and auto slices show no handles.
#[test]
fn slice_handles_resize_the_edges_they_hang_off() {
    let rect = [100.0, 100.0, 200.0, 200.0];
    let mut h = harness(SliceMode::Select, vec![user(1, rect)], Some(1));
    drag(&mut h, pos2(200.0, 150.0), pos2(230.0, 150.0));
    assert_eq!(committed(&resizes(&h)), Some(Rect::new(100.0, 100.0, 230.0, 200.0)));
    assert!(moves(&h).is_empty() && draws(&h).is_empty());

    let mut h = harness(SliceMode::Select, vec![user(1, rect)], Some(1));
    drag(&mut h, pos2(200.0, 150.0), pos2(80.0, 150.0));
    assert_eq!(committed(&resizes(&h)), Some(Rect::new(80.0, 100.0, 100.0, 200.0)));

    for origin in [SliceOrigin::Layer, SliceOrigin::Auto] {
        let mut h = harness(SliceMode::Select, vec![item(1, rect, origin)], Some(1));
        drag(&mut h, pos2(198.0, 150.0), pos2(230.0, 150.0));
        assert!(resizes(&h).is_empty() && moves(&h).is_empty(), "{origin:?}: {:?}", h.state().events);
    }
}

// A drag on an auto slice selects it and moves nothing; with `hide_auto` it can't be picked.
#[test]
fn auto_slices_do_not_move_and_hide_when_asked() {
    let items = vec![user(1, [0.0, 0.0, 50.0, 50.0]), item(2, [50.0, 0.0, 200.0, 100.0], SliceOrigin::Auto)];
    let mut h = harness(SliceMode::Select, items, None);
    drag(&mut h, pos2(100.0, 50.0), pos2(120.0, 60.0));
    assert_eq!(selects(&h), [Some(2)]);
    assert!(moves(&h).is_empty() && resizes(&h).is_empty() && draws(&h).is_empty(), "{:?}", h.state().events);

    h.state_mut().events.clear();
    h.state_mut().hide_auto = true;
    click(&mut h, pos2(100.0, 50.0));
    // The selected auto slice is no longer a part, so the host also drops the pick.
    assert!(selects(&h).iter().all(Option::is_none) && !selects(&h).is_empty(), "{:?}", h.state().events);
}

// Delete in Select with a user slice selected reports `Command(Delete)` and takes the key; with an
// auto slice, or in Slice mode, the key is not taken (PhotoCraft: Layer › Delete never sees it).
#[test]
fn slice_delete_key_deletes_the_slice_not_the_layer() {
    let items = vec![user(1, [0.0, 0.0, 50.0, 50.0]), item(2, [50.0, 0.0, 200.0, 100.0], SliceOrigin::Auto)];
    let mut h = harness(SliceMode::Select, items.clone(), Some(1));
    focus(&mut h);
    key(&mut h, Key::Delete, Modifiers::NONE);
    assert_eq!(commands(&h), [SliceCommand::Delete]);
    assert_eq!(h.state().taken, [Key::Delete]);

    let mut h = harness(SliceMode::Select, items.clone(), Some(2));
    focus(&mut h);
    key(&mut h, Key::Delete, Modifiers::NONE);
    assert!(commands(&h).is_empty() && h.state().taken.is_empty());

    // A layer slice is stored, so it deletes like a user slice.
    let mut h = harness(SliceMode::Select, vec![item(1, [0.0, 0.0, 50.0, 50.0], SliceOrigin::Layer)], Some(1));
    focus(&mut h);
    key(&mut h, Key::Delete, Modifiers::NONE);
    assert_eq!(commands(&h), [SliceCommand::Delete]);

    let mut h = harness(SliceMode::Slice, items, Some(1));
    focus(&mut h);
    key(&mut h, Key::Backspace, Modifiers::NONE);
    assert!(commands(&h).is_empty() && h.state().taken.is_empty());
}

// Locked: a press in either mode clears the selection (`Select { number: None }`, as PhotoCraft)
// and reports `Refused(Locked)`, and nothing else.
#[test]
fn locked_slices_refuse_every_gesture() {
    for mode in [SliceMode::Slice, SliceMode::Select] {
        let mut h = harness(mode, vec![user(1, [0.0, 0.0, 100.0, 100.0])], Some(1));
        h.state_mut().locked = true;
        h.step();
        drag(&mut h, pos2(50.0, 50.0), pos2(80.0, 80.0));
        let select = SliceEvent::Select { number: None, add: false, modifiers: Modifiers::NONE };
        assert_eq!(h.state().events, [select, SliceEvent::Refused(SliceRefusal::Locked)], "{mode:?}");
        assert_eq!(h.state().selected, None, "{mode:?}");
    }
}

/// The texts `overlay` draws.
fn badge_texts(items: &[SliceItem]) -> (Vec<String>, usize) {
    fn walk(shape: &Shape, out: &mut Vec<String>, marks: &mut usize) {
        match shape {
            Shape::Text(text) => out.push(text.galley.text().to_owned()),
            Shape::Rect(rect) if rect.fill == egui::Color32::WHITE => *marks += 1,
            Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out, marks)),
            _ => {}
        }
    }
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    let view = CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
    let numbers = LocaleNumbers::from_tag("en");
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        let style = SliceTool::style(&Tokens::of(ui));
        SliceOverlay::new(items).paint(ui.painter(), &view, &style, &numbers);
    });
    output.textures_delta.clear();
    let (mut out, mut marks) = (Vec::new(), 0);
    output.shapes.iter().for_each(|clipped| walk(&clipped.shape, &mut out, &mut marks));
    (out, marks)
}

// The overlay draws no badge on a slice narrower than the badge's least width, "03" on a user slice
// and "04" and a mark on a layer slice.
#[test]
fn slice_badge_hides_on_small_slices_and_marks_layer_slices() {
    let narrow = {
        let ctx = egui::Context::default();
        install(&ctx, &Base::suite_default());
        f64::from(SliceTool::style(&current(&ctx)).badge_min.x) - 1.0
    };
    let items = [user(1, [0.0, 0.0, narrow, 30.0]), user(3, [50.0, 0.0, 100.0, 50.0]), item(4, [120.0, 0.0, 180.0, 50.0], SliceOrigin::Layer)];
    let (texts, marks) = badge_texts(&items);
    assert_eq!(texts, ["03", "04"]);
    assert_eq!(marks, 1, "only the layer slice has a mark");
}

// Page Down picks the next slice part (`Select`), arrows move a user slice, Enter reports
// `Command(Options)`, double click too.
#[test]
fn slice_keyboard_picks_moves_and_opens_options() {
    let items = vec![user(1, [0.0, 0.0, 50.0, 50.0]), user(2, [100.0, 100.0, 150.0, 150.0]), user(3, [200.0, 100.0, 250.0, 150.0])];
    let mut h = harness(SliceMode::Select, items, None);
    // The canvas is the first Tab stop, the parts group the second; its first part is picked.
    key(&mut h, Key::Tab, Modifiers::NONE);
    key(&mut h, Key::Tab, Modifiers::NONE);
    assert_eq!(h.state().selected, Some(1));
    key(&mut h, Key::PageDown, Modifiers::NONE);
    assert_eq!(h.state().selected, Some(2));

    key(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.state().items[1].rect, Rect::new(101.0, 100.0, 151.0, 150.0));
    key(&mut h, Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(h.state().items[1].rect, Rect::new(101.0, 110.0, 151.0, 160.0));
    assert!(moves(&h).iter().all(|e| phase(e) == "set"));

    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(commands(&h), [SliceCommand::Options]);

    h.state_mut().events.clear();
    click(&mut h, pos2(220.0, 120.0));
    click(&mut h, pos2(220.0, 120.0));
    assert_eq!(commands(&h), [SliceCommand::Options]);
    assert_eq!(selects(&h), [Some(3), Some(3)]);
}

// Esc and `ToolHost::cancel` mid-drag report `Cancel` for `Draw`, `Move` and `Resize`.
#[test]
fn slice_cancels_on_escape_and_on_tool_switch() {
    let rect = [100.0, 100.0, 200.0, 200.0];
    let cases: [(SliceMode, Pos2, Pos2); 3] = [
        (SliceMode::Slice, pos2(300.0, 20.0), pos2(340.0, 60.0)),
        (SliceMode::Select, pos2(150.0, 150.0), pos2(170.0, 150.0)),
        (SliceMode::Select, pos2(200.0, 150.0), pos2(230.0, 150.0)),
    ];
    for escape in [true, false] {
        for (mode, from, to) in cases {
            let mut h = harness(mode, vec![user(1, rect)], Some(1));
            press(&mut h, from);
            move_to(&mut h, to);
            if escape {
                h.key_press(Key::Escape);
                h.run();
            } else {
                let (ctx, Some(id)) = (h.ctx.clone(), h.state().canvas) else { panic!("the canvas was shown") };
                ToolHost::cancel(&ctx, id, &mut h.state_mut().tool, CancelReason::Removed);
                h.step();
            }
            let state = h.state();
            let last = match state.events.last() {
                Some(SliceEvent::Draw(e) | SliceEvent::Move { event: e, .. } | SliceEvent::Resize { event: e, .. }) => phase(e),
                other => panic!("{other:?}"),
            };
            assert_eq!(last, "cancel", "{mode:?} at {from:?}, escape {escape}");
            assert!(draws(&h).iter().chain(&moves(&h)).chain(&resizes(&h)).all(|e| phase(e) != "commit"));
        }
    }
}

/// A distance from a handle's centre that is inside its reach and not at the centre.
fn grab_offset(h: &Harness<'_, Host>) -> f32 {
    SliceTool::style(&current(&h.ctx)).handle_hit / 2.0 - 1.0
}

// A snapped resize ends the edge on the guide, whatever the press's offset from the handle: the
// press is off the handle point but within its reach, the guide at x = 230.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn a_snapped_resize_lands_the_edge_on_the_guide() {
    let mut h = harness(SliceMode::Select, vec![user(1, [100.0, 100.0, 200.0, 200.0])], Some(1));
    let mut targets = SnapTargets::new();
    targets.add(SnapTarget { vertical: true, position: 230.0, kind: SnapKind::Guide, span: None });
    h.state_mut().targets = Some(targets);
    h.step();
    let off = grab_offset(&h);
    canvas::drag(&mut h, pos2(200.0 + off, 150.0), pos2(223.0 + off, 150.0), Modifiers::NONE);
    assert_eq!(committed(&resizes(&h)), Some(Rect::new(100.0, 100.0, 230.0, 200.0)), "{:?}", h.state().events);
}

// A click on a handle, off its centre, resizes nothing.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn a_click_on_a_handle_resizes_nothing() {
    let mut h = harness(SliceMode::Select, vec![user(1, [100.0, 100.0, 200.0, 200.0])], Some(1));
    let off = grab_offset(&h);
    canvas::press(&mut h, pos2(200.0 + off, 150.0), Modifiers::NONE);
    canvas::release(&mut h, pos2(200.0 + off, 150.0), Modifiers::NONE);
    assert!(resizes(&h).is_empty(), "{:?}", h.state().events);
}
