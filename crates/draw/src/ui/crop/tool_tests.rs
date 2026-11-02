//! Tests of the Crop tool on the canvas tool host (task TO-C4).

use egui::{Event, Key, Modifiers, PointerButton, Pos2, Sense, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use kurbo::{Point, Rect, Size};

use super::{CropBox, CropDocKey, CropDocument, CropEvent, CropGuide, CropInput, CropRatio, CropSettings, CropTool};
use craft_ui::canvas::{CanvasId, CanvasTool as _, CanvasView, ToolGesture, ToolHost, ToolHostResponse};
use craft_ui::edit::EditEvent;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
use craft_ui::testing::canvas as rig;
use craft_ui::theme::{Base, install};

struct Host {
    tool: CropTool,
    settings: CropSettings,
    document: CropDocument,
    view: CanvasView,
    /// Every event of every pass, `Settings` already applied to `settings`.
    events: Vec<CropEvent>,
    responses: Vec<ToolHostResponse>,
    /// The keys the tool took, since the last `key`.
    taken: Vec<Key>,
    focus: bool,
    nudge_pause: Option<f64>,
}

fn document() -> CropDocument {
    CropDocument::new(CropDocKey(1), Size::new(400.0, 300.0))
}

fn host(settings: CropSettings) -> Host {
    let rect = egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0));
    Host {
        tool: CropTool::new(),
        settings,
        document: document(),
        view: CanvasView::new(rect, Point::new(200.0, 150.0), 1.0),
        events: Vec::new(),
        responses: Vec::new(),
        taken: Vec::new(),
        focus: false,
        nudge_pause: None,
    }
}

fn classic() -> CropSettings {
    let mut settings = CropSettings::default();
    settings.shield.classic_mode = true;
    settings
}

fn image() -> CropSettings {
    let mut settings = CropSettings::default();
    settings.shield.auto_center_preview = false;
    settings
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let id = CanvasId::new(ui, "crop");
                let canvas = id.interact(ui, host.view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                if std::mem::take(&mut host.focus) {
                    canvas.request_focus();
                }
                let mut pass = host.tool.pass(CropInput::new(&host.settings, &host.document).nudge_pause(host.nudge_pause));
                let response = ToolHost::new(id).document(host.document.size).show(ui, &host.view, &canvas, &mut pass);
                let out = pass.finish();
                for event in &out.events {
                    if let CropEvent::Settings(next) = event {
                        host.settings = next.clone();
                    }
                }
                host.events.extend(out.events.iter().cloned());
                if let Some(view) = response.view {
                    host.view = view;
                }
                host.taken.extend(response.took_key.map(|k| k.key));
                host.responses.push(response);
            },
            host,
        );
    h.step();
    h
}

fn focused(h: &mut Harness<'_, Host>) {
    h.state_mut().focus = true;
    h.step();
}

fn button(at: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers }
}

fn move_to(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.step();
}

fn press(h: &mut Harness<'_, Host>, at: Pos2) {
    move_to(h, at);
    h.event(button(at, true, Modifiers::NONE));
    h.step();
}

fn release(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(button(at, false, Modifiers::NONE));
    h.step();
}

/// Presses at `from`, moves through the middle to `to` and releases there.
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
    h.state_mut().taken.clear();
    h.key_press_modifiers(modifiers, key);
    h.run();
}

fn frame(h: &Harness<'_, Host>) -> CropBox {
    let Some(frame) = h.state().tool.frame() else { panic!("the tool shows a frame") };
    frame
}

fn upright(x0: f64, y0: f64, x1: f64, y1: f64) -> CropBox {
    CropBox::upright(Rect::new(x0, y0, x1, y1))
}

fn near(a: &CropBox, b: &CropBox) -> bool {
    a.corners().iter().zip(b.corners()).all(|(p, q)| p.distance(q) < 1e-6)
}

fn frame_events(h: &Harness<'_, Host>) -> Vec<EditEvent<CropBox>> {
    h.state().events.iter().filter_map(|e| if let CropEvent::Frame(e) = e { Some(e.clone()) } else { None }).collect()
}

fn took(h: &Harness<'_, Host>) -> Option<Key> {
    h.state().taken.last().copied()
}

fn settings_events(h: &Harness<'_, Host>) -> Vec<CropSettings> {
    h.state().events.iter().filter_map(|e| if let CropEvent::Settings(s) = e { Some(s.clone()) } else { None }).collect()
}

// Through `ToolHost`, a drag in the default frame draws, inside a drawn frame moves, on a corner resizes and outside turns, giving the frames `CropFrame::show` gives for the same pointer path, with Begin, Update and Commit.
#[test]
fn a_drag_draws_moves_resizes_and_turns_like_the_frame_show() {
    let mut h = harness(host(classic()));
    assert_eq!(frame(&h), upright(0.0, 0.0, 400.0, 300.0), "the default frame is the canvas");

    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));
    assert_eq!(frame(&h), upright(100.0, 80.0, 300.0, 220.0), "a drag in the default frame draws");
    let events = frame_events(&h);
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(events.iter().any(|e| matches!(e, EditEvent::Update { .. })), "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { changed: true, .. })), "{events:?}");
    assert_eq!(h.state().responses.iter().filter_map(|r| r.gesture).next_back(), Some(ToolGesture::Commit));

    drag(&mut h, pos2(200.0, 150.0), pos2(230.0, 160.0));
    assert_eq!(frame(&h), upright(130.0, 90.0, 330.0, 230.0), "a drag inside a drawn frame moves it");

    drag(&mut h, pos2(330.0, 230.0), pos2(350.0, 250.0));
    assert_eq!(frame(&h), upright(130.0, 90.0, 350.0, 250.0), "a drag on a corner resizes it");

    // The frame's centre is (240, 170): from the left of it to straight above it is a quarter turn.
    drag(&mut h, pos2(60.0, 170.0), pos2(240.0, 20.0));
    let turned = frame(&h);
    assert_eq!(turned.rect, Rect::new(130.0, 90.0, 350.0, 250.0));
    assert!((turned.angle - 90.0).abs() < 1e-6, "{turned:?}");
}

// The first pass with a document makes the default frame; after Apply and `applied(true)` or a Discard the next pass makes a new one.
#[test]
fn the_tool_always_shows_a_frame() {
    let mut h = harness(host(classic()));
    let canvas = upright(0.0, 0.0, 400.0, 300.0);
    assert_eq!(frame(&h), canvas);
    assert!(!h.state().tool.session().pending());

    let (settings, document) = (h.state().settings.clone(), h.state().document);
    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));
    let Some(CropEvent::Apply(commit)) = h.state_mut().tool.apply(&settings, &document, None) else { panic!("a drawn frame applies") };
    assert_eq!(commit.frame, upright(100.0, 80.0, 300.0, 220.0));
    assert_eq!(h.state_mut().tool.applied(true), None);
    h.step();
    assert_eq!(frame(&h), canvas, "a new default frame after the crop");

    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));
    assert_eq!(h.state_mut().tool.discard(), Some(CropEvent::Discard));
    h.step();
    assert_eq!(frame(&h), canvas, "a new default frame after a discard");
    assert_eq!(h.state_mut().tool.apply(&settings, &document, None), None, "the default frame does not apply");
}

// Enter gives Apply with the commit of the settings, Esc gives Discard, Esc during a drag gives the frame's Cancel and keeps the frame.
#[test]
fn enter_applies_escape_discards_and_escape_in_a_drag_undoes_the_drag() {
    let mut h = harness(host(classic()));
    focused(&mut h);
    let canvas = upright(0.0, 0.0, 400.0, 300.0);

    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));
    h.state_mut().events.clear();
    key(&mut h, Key::Enter, Modifiers::NONE);
    let commits: Vec<_> = h.state().events.iter().filter_map(|e| if let CropEvent::Apply(c) = e { Some(*c) } else { None }).collect();
    assert_eq!(commits.len(), 1, "{:?}", h.state().events);
    assert_eq!(commits[0].frame, upright(100.0, 80.0, 300.0, 220.0));
    assert_eq!((commits[0].target, commits[0].delete_pixels), (None, true));
    assert_eq!(took(&h), Some(Key::Enter));
    h.state_mut().tool.applied(true);
    h.step();

    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));
    h.state_mut().events.clear();
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(h.state().events, [CropEvent::Discard]);
    assert_eq!(frame(&h), canvas, "the next pass makes a default frame");

    // Esc during a drag undoes the drag and keeps the frame.
    h.state_mut().events.clear();
    press(&mut h, pos2(100.0, 80.0));
    move_to(&mut h, pos2(200.0, 200.0));
    assert_eq!(frame(&h), upright(100.0, 80.0, 200.0, 200.0));
    key(&mut h, Key::Escape, Modifiers::NONE);
    release(&mut h, pos2(200.0, 200.0));
    let events = frame_events(&h);
    assert!(matches!(events.last(), Some(EditEvent::Cancel { .. })), "{events:?}");
    assert!(!h.state().events.contains(&CropEvent::Discard));
    assert_eq!(frame(&h), canvas);
    assert!(!h.state().tool.session().pending());
}

// O, Shift+O, H and P return false during a drag and without a frame and give Settings otherwise; H is taken but inert in Classic Mode; I gives Settings whenever no drag runs.
#[test]
fn o_h_p_i_act_only_with_a_frame_and_no_drag() {
    let mut start = classic();
    start.guide = CropGuide::Triangle;
    let mut h = harness(host(start));
    focused(&mut h);

    key(&mut h, Key::O, Modifiers::NONE);
    assert_eq!(settings_events(&h).last().map(|s| s.guide), Some(CropGuide::GoldenRatio), "O cycles to the next overlay");
    h.state_mut().settings.guide = CropGuide::Triangle;
    key(&mut h, Key::O, Modifiers::SHIFT);
    assert_eq!(settings_events(&h).last().map(|s| s.guide_orientation), Some(1), "Shift+O turns the Triangle");
    assert_eq!(took(&h), Some(Key::O));

    // H is taken but inert in Classic Mode.
    h.state_mut().events.clear();
    key(&mut h, Key::H, Modifiers::NONE);
    assert_eq!(took(&h), Some(Key::H));
    assert!(settings_events(&h).is_empty());
    key(&mut h, Key::P, Modifiers::NONE);
    assert!(settings_events(&h).last().is_some_and(|s| !s.shield.classic_mode), "P leaves Classic Mode");
    key(&mut h, Key::H, Modifiers::NONE);
    assert!(settings_events(&h).last().is_some_and(|s| !s.shield.show_cropped_area), "H toggles Show Cropped Area");

    // I fills the size fields from the document.
    h.state_mut().events.clear();
    key(&mut h, Key::I, Modifiers::NONE);
    let sizes = settings_events(&h);
    assert_eq!(sizes.last().map(|s| (s.ratio, s.size.width.as_str())), Some((CropRatio::Size, "400 px")));

    // During a drag the letters go back to the app, and so does I.
    h.state_mut().events.clear();
    press(&mut h, pos2(100.0, 80.0));
    move_to(&mut h, pos2(200.0, 200.0));
    for letter in [Key::O, Key::H, Key::P, Key::I] {
        key(&mut h, letter, Modifiers::NONE);
        assert_eq!(took(&h), None, "{letter:?}");
    }
    assert!(settings_events(&h).is_empty());
    release(&mut h, pos2(200.0, 200.0));

    // No frame: the letters stay with the app; I still acts.
    let mut empty = host(CropSettings::default());
    empty.document = CropDocument::new(CropDocKey(1), Size::ZERO);
    let mut h = harness(empty);
    focused(&mut h);
    assert_eq!(h.state().tool.frame(), None);
    for letter in [Key::O, Key::H, Key::P] {
        key(&mut h, letter, Modifiers::NONE);
        assert_eq!(took(&h), None, "{letter:?}");
    }
    key(&mut h, Key::I, Modifiers::NONE);
    assert_eq!(took(&h), Some(Key::I));
    assert_eq!(settings_events(&h).len(), 1);
}

// In Image mode a turn sets the turned view through `set_view` and the frame stays upright; Classic Mode turns the frame; an undo step returns the turned view.
#[test]
fn image_mode_requests_the_turned_view() {
    let mut h = harness(host(image()));
    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));
    assert!(h.state().view.rotation.abs() < 1e-9);

    // From the left of the frame's centre (200, 150) to straight above it.
    drag(&mut h, pos2(40.0, 150.0), pos2(200.0, 20.0));
    let turned = frame(&h);
    assert!((turned.angle - -90.0).abs() < 1e-6, "{turned:?}");
    assert_eq!(turned.rect, Rect::new(100.0, 80.0, 300.0, 220.0));
    assert!((h.state().view.rotation - 90.0).abs() < 1e-6, "the view turns against the frame: {}", h.state().view.rotation);

    // The view comes back with the call, not on the next pass.
    let view = h.state().view;
    let Some(back) = h.state_mut().tool.undo(&view) else { panic!("a turn is one box step") };
    assert_eq!(h.state().tool.frame(), Some(upright(100.0, 80.0, 300.0, 220.0)));
    assert!(back.rotation.abs() < 1e-6, "undo returns the view turned back: {}", back.rotation);
    assert!((h.state().view.rotation - 90.0).abs() < 1e-6, "the host's view is still the old one until the app adopts it");

    // Classic Mode turns the frame over a fixed image.
    let mut h = harness(host(classic()));
    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));
    drag(&mut h, pos2(40.0, 150.0), pos2(200.0, 20.0));
    assert!((frame(&h).angle - 90.0).abs() < 1e-6);
    assert!(h.state().view.rotation.abs() < 1e-9);
}

// The armed button draws one line then disarms; Command held draws a line without arming; the release gives Straightened and a fitted frame as one box step; a line under `crop.straighten_min` changes nothing.
#[test]
fn straighten_one_shot_and_temporary_command() {
    let mut h = harness(host(classic()));
    let canvas = upright(0.0, 0.0, 400.0, 300.0);
    h.state_mut().tool.toggle_straighten();
    h.step();
    assert!(h.state().tool.straighten_armed());

    let (from, to) = (pos2(100.0, 100.0), pos2(300.0, 120.0));
    drag(&mut h, from, to);
    let Some(angle) = CropBox::level_angle(Point::new(100.0, 100.0), Point::new(300.0, 120.0)) else { panic!("a line has an angle") };
    assert!(h.state().events.iter().any(|e| matches!(e, CropEvent::Straightened(a) if (a - angle).abs() < 1e-9)), "{:?}", h.state().events);
    let Some(fitted) = canvas.fitted(Size::new(400.0, 300.0), angle) else { panic!("the frame fits") };
    assert!(near(&frame(&h), &fitted), "{:?}", frame(&h));
    assert!(!h.state().tool.straighten_armed(), "one line per click of the button");
    let view = h.state().view;
    assert!(h.state_mut().tool.undo(&view).is_some(), "one box step");
    assert_eq!(h.state().tool.frame(), Some(canvas));
    assert_eq!(h.state_mut().tool.undo(&view), None, "Straighten is one step");
    h.state_mut().tool.redo(&view);
    h.step();

    // Command held: a temporary Straighten that never arms.
    h.state_mut().events.clear();
    h.event(Event::ModifiersChanged(Modifiers::COMMAND));
    h.step();
    let (from, to) = (pos2(100.0, 100.0), pos2(300.0, 80.0));
    press(&mut h, from);
    move_to(&mut h, to);
    release(&mut h, to);
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.step();
    assert!(h.state().events.iter().any(|e| matches!(e, CropEvent::Straightened(a) if *a < 0.0)), "{:?}", h.state().events);
    assert!(!h.state().tool.straighten_armed());

    // A line under the minimum changes nothing.
    h.state_mut().events.clear();
    let at = frame(&h);
    h.state_mut().tool.toggle_straighten();
    h.step();
    drag(&mut h, pos2(100.0, 100.0), pos2(101.0, 100.0));
    assert!(!h.state().events.iter().any(|e| matches!(e, CropEvent::Straightened(_))), "{:?}", h.state().events);
    assert_eq!(frame(&h), at);
}

// `refit` with `editing` true twice (1:3, then 16:3) fits from the first frame, not from the 1:3 one (#7752).
#[test]
fn editing_a_ratio_refits_from_the_frame_the_edit_began_with() {
    let mut h = harness(host(classic()));
    drag(&mut h, pos2(50.0, 50.0), pos2(350.0, 250.0));
    let document = h.state().document;
    let mut settings = h.state().settings.clone();
    let mut refit = |h: &mut Harness<'_, Host>, width: f64, height: f64, editing: bool| {
        settings.ratio = CropRatio::Sides { width, height };
        let view = h.state().view;
        let event = h.state_mut().tool.refit(&view, &settings, &document, editing).map(|(event, _)| event);
        let Some(CropEvent::Frame(EditEvent::Set { value, .. })) = event else { return None };
        Some(value)
    };

    let Some(narrow) = refit(&mut h, 1.0, 3.0, true) else { panic!("1:3 fits inside the frame") };
    assert!((narrow.rect.width() - 200.0 / 3.0).abs() < 1e-9 && (narrow.rect.height() - 200.0).abs() < 1e-9, "{narrow:?}");
    assert_eq!(narrow.rect.center(), Point::new(200.0, 150.0), "about the centre");
    let Some(wide) = refit(&mut h, 16.0, 3.0, true) else { panic!("16:3 fits inside the first frame") };
    assert!((wide.rect.width() - 300.0).abs() < 1e-9 && (wide.rect.height() - 56.25).abs() < 1e-9, "{wide:?}");
    // The run ends: the next change fits from the frame as it is.
    assert_eq!(refit(&mut h, 16.0, 3.0, false), None);
    let Some(square) = refit(&mut h, 1.0, 1.0, false) else { panic!("1:1 fits inside the wide frame") };
    assert!((square.rect.width() - 56.25).abs() < 1e-9 && (square.rect.height() - 56.25).abs() < 1e-9, "{square:?}");
}

// A pass with another document's key gives Discard for a drawn frame and makes that document's default frame.
#[test]
fn a_second_document_discards_a_drawn_frame() {
    let mut h = harness(host(classic()));
    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));
    h.state_mut().events.clear();
    h.state_mut().document = CropDocument::new(CropDocKey(2), Size::new(300.0, 200.0));
    h.step();
    assert_eq!(h.state().events, [CropEvent::Discard]);
    assert_eq!(frame(&h), upright(0.0, 0.0, 300.0, 200.0));
    assert!(!h.state().tool.session().pending());
}

// `parts` gives the Crop frame group with nine parts; Page Down through the host picks them in order and an arrow moves the picked corner.
#[test]
fn parts_name_every_corner_edge_and_the_whole_frame() {
    let mut h = harness(host(classic()));
    h.run();
    h.get_by_label("Crop frame");
    for name in [
        "Whole frame",
        "Top left corner",
        "Top right corner",
        "Bottom right corner",
        "Bottom left corner",
        "Top edge",
        "Right edge",
        "Bottom edge",
        "Left edge",
    ] {
        h.get_by_label(name);
    }

    // The canvas is the first Tab stop, the parts group the second.
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::PageDown);
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(frame(&h), upright(1.0, 0.0, 400.0, 300.0), "Tab picked the whole frame, Page Down the top left corner");
}

// `leave` gives Apply for a drawn frame and nothing for the default; a refused ✓ gives the frame, its steps and the kept view back, a refused leave clears the session.
#[test]
fn leaving_applies_a_drawn_frame_and_a_refused_commit_restores_it() {
    let mut h = harness(host(classic()));
    let (settings, document) = (h.state().settings.clone(), h.state().document);
    assert_eq!(h.state_mut().tool.leave(&settings, &document, false), None, "the default frame is dropped");
    assert_eq!(h.state().tool.session().frame(), None);
    h.step();

    let drawn = upright(100.0, 80.0, 300.0, 220.0);
    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));
    assert_eq!(h.state_mut().tool.leave(&settings, &document, true), None, "a kept frame stays");
    let Some(CropEvent::Apply(commit)) = h.state_mut().tool.leave(&settings, &document, false) else { panic!("a drawn frame applies") };
    assert_eq!(commit.frame, drawn);
    // A refusal of the Apply from leaving clears the session: the next tool does not retry it.
    assert_eq!(h.state_mut().tool.applied(false), None);
    assert_eq!(h.state().tool.session().frame(), None);
    h.step();
    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));

    let view = h.state().view;
    let Some(CropEvent::Apply(_)) = h.state_mut().tool.apply(&settings, &document, Some(view)) else { panic!("a drawn frame applies") };
    // The app's command changed the box history; the refusal gives it back.
    h.state_mut().tool.undo(&view);
    assert_eq!(h.state_mut().tool.applied(false), Some(view), "the kept view comes back");
    assert_eq!(h.state().tool.session().frame(), Some(drawn));
    assert!(h.state().tool.session().can_undo());
}

// A second click of a run inside a drawn frame gives Apply; on the default frame or a handle it does not.
#[test]
fn a_double_click_inside_a_drawn_frame_applies() {
    let applies = |h: &Harness<'_, Host>| h.state().events.iter().filter(|e| matches!(e, CropEvent::Apply(_))).count();

    let mut h = harness(host(classic()));
    click(&mut h, pos2(200.0, 150.0));
    click(&mut h, pos2(200.0, 150.0));
    assert_eq!(applies(&h), 0, "the default frame does not apply");

    drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0));
    h.step();
    h.step();
    click(&mut h, pos2(300.0, 220.0));
    click(&mut h, pos2(300.0, 220.0));
    assert_eq!(applies(&h), 0, "a handle does not apply");

    for _ in 0..8 {
        h.step();
    }
    click(&mut h, pos2(200.0, 150.0));
    assert_eq!(applies(&h), 0, "one click does not apply");
    press(&mut h, pos2(200.0, 150.0));
    assert_eq!(applies(&h), 1, "the second click of the run applies");
    release(&mut h, pos2(200.0, 150.0));
    assert_eq!(applies(&h), 1);
}

fn picked(h: &mut Harness<'_, Host>) -> Option<u64> {
    let (settings, document) = (h.state().settings.clone(), h.state().document);
    let pass = h.state_mut().tool.pass(CropInput::new(&settings, &document));
    pass.picked()
}

// `leave` answers `None` while a drag runs and leaves the press to the host; once the drag ends the same call applies the frame (the app calls it again on later passes).
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn leaving_during_a_drag_waits_for_the_release() {
    let mut h = harness(host(classic()));
    let (settings, document) = (h.state().settings.clone(), h.state().document);
    rig::press(&mut h, pos2(100.0, 80.0), Modifiers::NONE);
    rig::move_to(&mut h, pos2(300.0, 220.0));
    assert!(h.state().tool.dragging());
    assert_eq!(h.state_mut().tool.leave(&settings, &document, false), None, "the drag keeps the frame");
    assert!(h.state().tool.dragging(), "the press the host drives is still there");
    rig::release(&mut h, pos2(300.0, 220.0), Modifiers::NONE);
    assert!(!h.state().tool.dragging());
    let Some(CropEvent::Apply(commit)) = h.state_mut().tool.leave(&settings, &document, false) else { panic!("the drawn frame applies") };
    assert_eq!(commit.frame, upright(100.0, 80.0, 300.0, 220.0));
}

// An Apply from `leave` that the app refuses clears the session instead of leaving the frame pending under another tool; an Apply from ✓ gives the frame back.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn a_refused_apply_from_leave_clears_the_session() {
    let mut h = harness(host(classic()));
    let (settings, document) = (h.state().settings.clone(), h.state().document);
    rig::drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0), Modifiers::NONE);
    let Some(CropEvent::Apply(_)) = h.state_mut().tool.leave(&settings, &document, false) else { panic!("a drawn frame applies") };
    assert_eq!(h.state_mut().tool.applied(false), None);
    assert_eq!(h.state().tool.frame(), None, "no pending crop under the next tool");
    assert!(!h.state().tool.session().pending());

    h.step();
    rig::drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0), Modifiers::NONE);
    let Some(CropEvent::Apply(_)) = h.state_mut().tool.apply(&settings, &document, None) else { panic!("a drawn frame applies") };
    h.state_mut().tool.applied(false);
    assert_eq!(h.state().tool.frame(), Some(upright(100.0, 80.0, 300.0, 220.0)), "a refused ✓ restores it");
}

// A document switch between Apply and `applied(false)` drops the kept frame: the old document's frame does not come back.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn a_document_switch_drops_the_kept_frame() {
    let mut h = harness(host(classic()));
    let (settings, document) = (h.state().settings.clone(), h.state().document);
    rig::drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0), Modifiers::NONE);
    let Some(CropEvent::Apply(_)) = h.state_mut().tool.apply(&settings, &document, None) else { panic!("a drawn frame applies") };
    h.state_mut().document = CropDocument::new(CropDocKey(2), Size::new(400.0, 300.0));
    h.step();
    h.state_mut().events.clear();
    assert_eq!(h.state_mut().tool.applied(false), None);
    h.step();
    assert!(!h.state().tool.session().pending(), "the old frame is not restored");
    assert!(!h.state().events.contains(&CropEvent::Discard));
}

// Drawing a new frame clears the pick, so the arrows never move a corner of the new frame.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn a_draw_press_clears_the_pick() {
    let mut h = harness(host(classic()));
    h.run();
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::PageDown);
    h.run();
    assert!(picked(&mut h).is_some(), "Page Down picked the top left corner");
    // Inside the default frame a press draws.
    rig::press(&mut h, pos2(200.0, 150.0), Modifiers::NONE);
    assert_eq!(picked(&mut h), None, "a Draw press clears the pick");
    rig::release(&mut h, pos2(200.0, 150.0), Modifiers::NONE);
}

// The tool's state after a pass is read from getters: the frame, the crop preview and a running drag.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn the_pass_state_is_read_from_the_tool() {
    let mut h = harness(host(classic()));
    assert!(!h.state().tool.shows_beyond_canvas());
    assert!(!h.state().tool.dragging());
    rig::press(&mut h, pos2(100.0, 80.0), Modifiers::NONE);
    rig::move_to(&mut h, pos2(200.0, 150.0));
    assert!(h.state().tool.dragging());
    assert!(h.state().tool.shows_beyond_canvas());
    rig::release(&mut h, pos2(200.0, 150.0), Modifiers::NONE);
    assert!(!h.state().tool.dragging());
    assert!(h.state().tool.shows_beyond_canvas(), "the frame has been pressed since it was made");
    assert_eq!(frame(&h), upright(100.0, 80.0, 200.0, 150.0));
}

// With a nudge pause a run of arrow nudges is one box step and counts as owning Undo from the first nudge; without it each nudge is a step.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn nudges_within_the_pause_are_one_box_step() {
    let mut h = harness(host(classic()));
    h.state_mut().nudge_pause = Some(30.0);
    focused(&mut h);
    rig::drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0), Modifiers::NONE);
    focused(&mut h);
    let drawn = upright(100.0, 80.0, 300.0, 220.0);
    rig::key(&mut h, Key::ArrowRight, Modifiers::NONE);
    rig::key(&mut h, Key::ArrowRight, Modifiers::NONE);
    rig::key(&mut h, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(frame(&h), upright(102.0, 81.0, 302.0, 221.0));
    assert!(h.state().tool.session().owns_undo());
    let view = h.state().view;
    assert!(h.state_mut().tool.undo(&view).is_some());
    assert_eq!(frame(&h), drawn, "one Undo takes back the whole run");

    // The pause ends the run.
    let mut h = harness(host(classic()));
    h.state_mut().nudge_pause = Some(0.01);
    focused(&mut h);
    rig::drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0), Modifiers::NONE);
    focused(&mut h);
    rig::key(&mut h, Key::ArrowRight, Modifiers::NONE);
    for _ in 0..4 {
        h.step();
    }
    rig::key(&mut h, Key::ArrowRight, Modifiers::NONE);
    let view = h.state().view;
    assert!(h.state_mut().tool.undo(&view).is_some());
    assert_eq!(frame(&h), upright(101.0, 80.0, 301.0, 220.0), "the second nudge was its own step");

    // No pause: every nudge is a step.
    let mut h = harness(host(classic()));
    focused(&mut h);
    rig::drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0), Modifiers::NONE);
    focused(&mut h);
    rig::key(&mut h, Key::ArrowRight, Modifiers::NONE);
    rig::key(&mut h, Key::ArrowRight, Modifiers::NONE);
    let view = h.state().view;
    assert!(h.state_mut().tool.undo(&view).is_some());
    assert_eq!(frame(&h), upright(101.0, 80.0, 301.0, 220.0));
}

// The hover cursor comes from `CropFrame::cursor`: resize arrows on a handle, the crosshair inside the default frame, Move inside a drawn one.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn the_cursor_comes_from_the_frame() {
    let mut h = harness(host(classic()));
    let icon = |h: &Harness<'_, Host>| h.state().responses.last().map(|r| r.cursor);
    rig::move_to(&mut h, pos2(200.0, 150.0));
    assert_eq!(icon(&h), Some(egui::CursorIcon::Crosshair), "inside the default frame");
    rig::drag(&mut h, pos2(100.0, 80.0), pos2(300.0, 220.0), Modifiers::NONE);
    rig::move_to(&mut h, pos2(200.0, 150.0));
    assert_eq!(icon(&h), Some(egui::CursorIcon::Move));
    rig::move_to(&mut h, pos2(300.0, 220.0));
    assert_eq!(icon(&h), Some(egui::CursorIcon::ResizeNwSe));
}
