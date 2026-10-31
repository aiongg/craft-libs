use craft_theme::{Appearance, Contrast, Mode, Size, TextSize, builtins, resolve};
use egui::accesskit::Role;
use egui::epaint::ColorMode;
use egui::{Color32, CursorIcon, Event, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect, Vec2};

use super::geometry::{Hit, drawn, handle_part, hit, resized, resized_turned, turn, turned_angle, within};
use super::{CropBox, CropFrame, CropLabels, CropNames, turn_toward};
use crate::ui::CropGuide;
use craft_ui::canvas::quad::BoxPart;
use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, FontFamilies, ToEgui, UiTokens, current, install};

/// The frame before any gesture.
const RECT: Rect = Rect::new(100.0, 80.0, 300.0, 220.0);

fn names() -> CropNames {
    CropNames {
        group: "Crop frame".into(),
        whole: "Whole frame".into(),
        corners: ["Top left corner", "Top right corner", "Bottom right corner", "Bottom left corner"].map(String::from),
        edges: ["Top edge", "Right edge", "Bottom edge", "Left edge"].map(String::from),
    }
}

fn near(a: &CropBox, b: &CropBox) -> bool {
    let (p, q) = (a.corners(), b.corners());
    p.iter().zip(q).all(|(p, q)| p.distance(q) < 1e-6)
}

// Pure geometry.

#[test]
fn hit_finds_handles_inside_and_outside_with_a_reach_that_shrinks_inside_small_frames() {
    let b = CropBox::upright(Rect::new(10.0, 10.0, 110.0, 60.0));
    let at = |x, y| hit(&b, Point::new(x, y), 4.0);
    assert_eq!(at(10.0, 10.0), Hit::Handle(-1, -1));
    assert_eq!(at(112.0, 61.0), Hit::Handle(1, 1));
    assert_eq!(at(60.0, 8.0), Hit::Handle(0, -1));
    assert_eq!(at(108.0, 30.0), Hit::Handle(1, 0));
    assert_eq!(at(60.0, 30.0), Hit::Inside);
    assert_eq!(at(150.0, 30.0), Hit::Outside);
    assert_eq!(at(113.9, 30.0), Hit::Handle(1, 0), "outside the full reach");
    assert_eq!(at(114.1, 30.0), Hit::Outside);
    // A tiny frame keeps a movable middle: inside, the reach is a quarter of the side.
    let tiny = CropBox::upright(Rect::new(0.0, 0.0, 4.0, 4.0));
    assert_eq!(hit(&tiny, Point::new(2.0, 2.0), 8.0), Hit::Inside);
    assert_eq!(hit(&tiny, Point::new(-7.0, 2.0), 8.0), Hit::Handle(-1, 0));
    assert_eq!(hit(&tiny, Point::new(0.9, 2.0), 8.0), Hit::Handle(-1, 0), "a quarter of the side: 1");
    assert_eq!(hit(&tiny, Point::new(1.1, 2.0), 8.0), Hit::Inside);
}

#[test]
fn a_turned_frame_is_hit_in_its_own_axes() {
    let b = CropBox { rect: Rect::new(40.0, 30.0, 100.0, 70.0), angle: 90.0 };
    // A quarter turn: the frame covers x 50..90, y 20..80 on the page.
    assert_eq!(hit(&b, Point::new(70.0, 75.0), 2.0), Hit::Inside);
    assert_eq!(hit(&CropBox::upright(b.rect), Point::new(70.0, 75.0), 2.0), Hit::Outside);
    assert_eq!(hit(&b, Point::new(45.0, 50.0), 2.0), Hit::Outside);
    assert_eq!(hit(&b, Point::new(90.0, 20.0), 2.0), Hit::Handle(-1, -1), "the upright top left is now top right");
    assert_eq!(hit(&b, Point::new(70.0, 80.0), 2.0), Hit::Handle(1, 0), "its right edge faces down");
    let want = [(90.0, 20.0), (90.0, 80.0), (50.0, 80.0), (50.0, 20.0)];
    for (c, (x, y)) in b.corners().iter().zip(want) {
        assert!(c.distance(Point::new(x, y)) < 1e-9, "{c:?}");
    }
    let bounds = b.turned_bounds();
    assert!((bounds.x0 - 50.0).abs() < 1e-9 && (bounds.y1 - 80.0).abs() < 1e-9, "{bounds:?}");
}

#[test]
fn handles_resize_with_shift_alt_and_a_ratio() {
    let r = Rect::new(20.0, 20.0, 100.0, 60.0);
    let d = Vec2::new;
    assert_eq!(resized(r, 1, 0, d(20.0, 10.0), None, false), Rect::new(20.0, 20.0, 120.0, 60.0));
    assert_eq!(resized(r, 1, 1, d(40.0, 10.0), None, false), Rect::new(20.0, 20.0, 140.0, 70.0));
    // A ratio: the larger relative extent wins at a corner, the opposite corner stays.
    let k = 120.0 / 50.0;
    let held = resized(Rect::new(20.0, 20.0, 140.0, 70.0), -1, -1, d(-10.0, -20.0), Some(k), false);
    assert_eq!((held.x1, held.y1), (140.0, 70.0));
    assert!((held.width() / held.height() - k).abs() < 1e-9, "{held:?}");
    assert!((held.height() - 70.0).abs() < 1e-9, "the larger relative extent: {held:?}");
    // ⌥ about the centre.
    assert_eq!(resized(Rect::new(40.0, 20.0, 80.0, 60.0), 1, 0, d(10.0, 0.0), None, true), Rect::new(30.0, 20.0, 90.0, 60.0));
    // A ratio on an edge: centred on the other axis.
    assert_eq!(resized(Rect::new(40.0, 20.0, 80.0, 60.0), 1, 0, d(20.0, 0.0), Some(1.0), false), Rect::new(40.0, 10.0, 100.0, 70.0));
    // Past the other side the frame flips.
    assert_eq!(resized(Rect::new(20.0, 20.0, 60.0, 60.0), 1, 0, d(-50.0, 0.0), None, false), Rect::new(10.0, 20.0, 20.0, 60.0));
}

#[test]
fn a_turned_frame_resizes_along_its_axes_and_keeps_the_opposite_edge() {
    let b = CropBox { rect: Rect::new(40.0, 30.0, 100.0, 70.0), angle: 90.0 };
    // At 90° the right edge faces down the page: dragging down 10 grows the width.
    let n = resized_turned(&b, 1, 0, Vec2::new(0.0, 10.0), None, false);
    assert!((n.rect.width() - 70.0).abs() < 1e-9 && (n.rect.height() - 40.0).abs() < 1e-9, "{n:?}");
    let (old, new) = (b.corners(), n.corners());
    assert!(old[0].distance(new[0]) < 1e-9 && old[3].distance(new[3]) < 1e-9, "the left edge stays on the page");
    let same = resized_turned(&b, 1, 0, Vec2::new(25.0, 0.0), None, false);
    assert!(near(&same, &b), "a sideways drag does nothing to that handle");
    let plain = resized_turned(&CropBox::upright(RECT), 1, 1, Vec2::new(5.0, 6.0), None, false);
    assert_eq!(plain.rect, resized(RECT, 1, 1, Vec2::new(5.0, 6.0), None, false));
}

#[test]
fn a_new_frame_is_square_from_the_centre_or_holds_the_ratio() {
    let (a, b) = (Point::new(100.0, 100.0), Point::new(140.0, 120.0));
    assert_eq!(drawn(a, b, None, Modifiers::NONE), Rect::new(100.0, 100.0, 140.0, 120.0));
    assert_eq!(drawn(a, b, None, Modifiers::SHIFT), Rect::new(100.0, 100.0, 140.0, 140.0));
    assert_eq!(drawn(a, b, None, Modifiers::ALT), Rect::new(60.0, 80.0, 140.0, 120.0));
    assert_eq!(drawn(a, b, Some(2.0), Modifiers::NONE), Rect::new(100.0, 100.0, 140.0, 120.0));
    assert_eq!(drawn(a, Point::new(120.0, 140.0), Some(2.0), Modifiers::NONE), Rect::new(100.0, 100.0, 180.0, 140.0));
}

#[test]
fn turning_snaps_to_the_step_and_stays_in_range() {
    let c = Point::new(0.0, 0.0);
    let at = |deg: f64| Point::new(deg.to_radians().cos() * 10.0, deg.to_radians().sin() * 10.0);
    assert!((turned_angle(c, 0.0, 0.0, at(40.0), None) - 40.0).abs() < 1e-9);
    assert!((turned_angle(c, 0.0, 0.0, at(40.0), Some(15.0)) - 45.0).abs() < 1e-9);
    assert!((turned_angle(c, 0.0, 170.0, at(40.0), None) - -150.0).abs() < 1e-9, "wraps into -180..=180");
    assert!(turn(Point::new(1.0, 0.0), c, 90.0).distance(Point::new(0.0, 1.0)) < 1e-12, "clockwise on screen");
}

#[test]
fn bounds_cut_an_unturned_frame_and_move_a_turned_one() {
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
    let out = CropBox::upright(Rect::new(-20.0, 10.0, 450.0, 200.0));
    assert_eq!(within(out, bounds, true, false).unwrap().rect, Rect::new(0.0, 10.0, 400.0, 200.0));
    assert_eq!(within(CropBox::upright(Rect::new(-20.0, 10.0, 80.0, 60.0)), bounds, false, false).unwrap().rect, Rect::new(0.0, 10.0, 100.0, 60.0));
    let turned = CropBox { rect: Rect::new(-30.0, 100.0, 70.0, 150.0), angle: 90.0 };
    let moved = within(turned, bounds, false, false).unwrap();
    assert!(moved.turned_bounds().x0 >= -1e-9 && moved.angle == 90.0);
    assert!(within(turned, bounds, true, false).is_none(), "a resize that leaves is refused");
}

#[test]
fn crop_size_is_whole_units_of_at_least_one() {
    assert_eq!(CropBox::upright(Rect::new(0.0, 0.0, 10.4, 0.2)).crop_size(), Some((10.0, 1.0)));
    assert_eq!(CropBox::upright(Rect::new(0.0, 0.0, f64::NAN, 5.0)).crop_size(), None);
    assert_eq!(CropBox { rect: Rect::new(0.0, 0.0, 30.0, 10.0), angle: 20.0 }.swapped().rect, Rect::new(10.0, -10.0, 20.0, 20.0));
    assert_eq!(handle_part(1, -1), BoxPart::Corner(1));
    assert_eq!(handle_part(-1, 0), BoxPart::Edge(3));
}

// The component, through real input.

struct Host {
    state: CropBox,
    /// The frame when a drag began, to restore on a cancel.
    before: Option<CropBox>,
    rotation: f64,
    flip: bool,
    view: Option<CanvasView>,
    events: Vec<EditEvent<CropBox>>,
    applied: usize,
    discarded: usize,
    picked: Option<BoxPart>,
    hovered: Option<BoxPart>,
    dragging: bool,
    turning: Option<f64>,
    sizing: Option<(f64, f64)>,
    default_frame: bool,
    ratio: Option<f64>,
    shield: Option<Color32>,
    guide: Option<(CropGuide, u8)>,
    bounds: Option<Rect>,
    readout: bool,
    /// The host snaps dragged points to a grid of this pitch.
    snap: Option<f64>,
    /// A text field beside the canvas takes the keyboard focus.
    text: bool,
    text_value: String,
    /// A button beside the canvas takes the keyboard focus.
    button: bool,
}

fn harness_with(state: CropBox, rotation: f64, flip: bool, setup: impl FnOnce(&mut Host)) -> Harness<'static, Host> {
    let mut host = Host {
        state,
        before: None,
        rotation,
        flip,
        view: None,
        events: Vec::new(),
        applied: 0,
        discarded: 0,
        picked: None,
        hovered: None,
        dragging: false,
        turning: None,
        sizing: None,
        default_frame: false,
        ratio: None,
        shield: None,
        guide: None,
        bounds: None,
        readout: false,
        snap: None,
        text: false,
        text_value: String::new(),
        button: false,
    };
    setup(&mut host);
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            if host.text {
                let label = ui.label("Name");
                let field = ui.add(egui::TextEdit::singleline(&mut host.text_value).id_salt("text field")).labelled_by(label.id);
                field.request_focus();
            }
            if host.button {
                ui.button("Other").request_focus();
            }
            ui.add_space(40.0);
            // Not a Tab stop, so Tab goes to the frame.
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView { rotation: host.rotation, flip_x: host.flip, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) };
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let state = host.state;
            let pitch = host.snap;
            let mut frame = CropFrame::new("crop", &state, names())
                .shield(host.shield)
                .guide(host.guide)
                .ratio(host.ratio)
                .default_frame(host.default_frame)
                .units(&px, &px);
            if let Some(pitch) = pitch {
                frame = frame.snap(move |part, b, _| {
                    let p = b.part_point(part).unwrap_or_default();
                    Vec2::new((p.x / pitch).round() * pitch - p.x, (p.y / pitch).round() * pitch - p.y)
                });
            }
            if let Some(bounds) = host.bounds {
                frame = frame.bounds(bounds);
            }
            if host.readout {
                frame = frame.readout(CropLabels::new("W:", "H:", "Angle:"));
            }
            let r = frame.show(ui, &canvas, &view);
            match &r.event {
                Some(EditEvent::Begin { .. }) => host.before = Some(host.state),
                Some(EditEvent::Update { value, .. }) => host.state = *value,
                Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                    host.before = None;
                    host.state = *value;
                }
                Some(EditEvent::Cancel { .. }) => {
                    if let Some(before) = host.before.take() {
                        host.state = before;
                    }
                }
                Some(_) | None => {}
            }
            host.events.extend(r.event.clone());
            host.applied += usize::from(r.apply);
            host.discarded += usize::from(r.discard);
            (host.picked, host.hovered) = (r.picked, r.hovered);
            (host.dragging, host.turning, host.sizing) = (r.dragging, r.turning, r.sizing);
        },
        host,
    )
}

fn harness(setup: impl FnOnce(&mut Host)) -> Harness<'static, Host> {
    harness_with(CropBox::upright(RECT), 0.0, false, setup)
}

fn view(h: &Harness<'_, Host>) -> CanvasView {
    h.state().view.expect("drawn")
}

fn screen(h: &Harness<'_, Host>, p: Point) -> Pos2 {
    view(h).to_screen(p)
}

fn state(h: &Harness<'_, Host>) -> CropBox {
    h.state().state
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

/// Moves to `from`, presses, moves to `to` and releases there, with `modifiers` held.
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

/// Presses at `from` and moves to `to`, the button held.
fn hold(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.run();
    h.event(Event::PointerMoved(to));
    h.run();
}

fn frame(h: &mut Harness<'_, Host>, events: impl IntoIterator<Item = Event>) {
    h.input_mut().events.extend(events);
    h.step();
}

fn kinds(h: &Harness<'_, Host>) -> Vec<&'static str> {
    h.state()
        .events
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "begin",
            EditEvent::Update { .. } => "update",
            EditEvent::Commit { .. } => "commit",
            EditEvent::Cancel { .. } => "cancel",
            EditEvent::Set { .. } => "set",
            _ => "other",
        })
        .collect()
}

#[test]
fn a_drag_inside_moves_the_frame_and_a_turned_frame_moves_on_the_page() {
    let mut h = harness(|_| {});
    h.run();
    let from = screen(&h, Point::new(150.0, 110.0));
    drag(&mut h, from, from + vec2(30.0, 20.0), Modifiers::NONE);
    assert_eq!(state(&h), CropBox::upright(RECT + Vec2::new(30.0, 20.0)));
    assert_eq!(kinds(&h).first(), Some(&"begin"));
    assert_eq!(kinds(&h).last(), Some(&"commit"));
    assert_eq!(h.state().picked, Some(BoxPart::Inside));

    let turned = CropBox { rect: RECT, angle: 30.0 };
    let mut h = harness_with(turned, 0.0, false, |_| {});
    h.run();
    let from = screen(&h, RECT.center());
    drag(&mut h, from, from + vec2(-25.0, 10.0), Modifiers::NONE);
    assert!(near(&state(&h), &turned.moved(Vec2::new(-25.0, 10.0))), "{:?}", state(&h));
    assert_eq!(state(&h).angle, 30.0);
}

#[test]
fn a_press_released_without_moving_changes_nothing() {
    for default_frame in [false, true] {
        let mut h = harness(|host| host.default_frame = default_frame);
        h.run();
        for at in [Point::new(150.0, 110.0), Point::new(300.0, 220.0), Point::new(350.0, 270.0)] {
            let p = screen(&h, at);
            drag(&mut h, p, p, Modifiers::NONE);
        }
        assert!(h.state().events.is_empty(), "{:?}", kinds(&h));
        assert_eq!(state(&h), CropBox::upright(RECT));
        assert_eq!(h.state().applied, 0, "a click outside does not apply");
    }
}

#[test]
fn an_edge_takes_the_pointer_within_8_points_in_a_turned_flipped_view() {
    for (offset, want) in [(7.9, BoxPart::Edge(1)), (8.1, BoxPart::Outside)] {
        let mut h = harness_with(CropBox::upright(RECT), 30.0, true, |_| {});
        h.run();
        let v = view(&h);
        let edge = screen(&h, Point::new(RECT.x1, RECT.center().y));
        // Out of the right edge along the frame's x axis, as the view draws it.
        let out = v.map_vec(Vec2::new(1.0, 0.0)).normalized();
        h.event(Event::PointerMoved(edge + out * offset));
        h.run();
        assert_eq!(h.state().hovered, Some(want), "{offset} pt from the edge");
    }
}

#[test]
fn a_corner_drag_resizes_along_the_frame_axes_in_a_turned_view() {
    let b = CropBox { rect: RECT, angle: 20.0 };
    let mut h = harness_with(b, 40.0, true, |_| {});
    h.run();
    let v = view(&h);
    let corner = screen(&h, b.corners()[2]);
    let to = corner + vec2(14.0, -9.0);
    let want = resized_turned(&b, 1, 1, v.to_doc(to) - v.to_doc(corner), None, false);
    drag(&mut h, corner, to, Modifiers::NONE);
    assert!(near(&state(&h), &want), "{:?} vs {want:?}", state(&h));
    assert_eq!(h.state().picked, Some(BoxPart::Corner(2)));

    // ⇧ keeps the proportions; ⌥ resizes about the centre.
    let mut h = harness(|_| {});
    h.run();
    let (from, to) = (screen(&h, Point::new(300.0, 220.0)), screen(&h, Point::new(340.0, 230.0)));
    drag(&mut h, from, to, Modifiers::SHIFT);
    let r = state(&h).rect;
    assert!((r.width() / r.height() - 200.0 / 140.0).abs() < 1e-9, "{r:?}");
    let mut h = harness(|_| {});
    h.run();
    drag(&mut h, from, to, Modifiers::ALT);
    let r = state(&h).rect;
    assert!((r.center().x - 200.0).abs() < 1e-9 && (r.center().y - 150.0).abs() < 1e-9, "{r:?}");
}

#[test]
fn a_ratio_is_held_by_every_gesture() {
    let mut h = harness(|host| host.ratio = Some(1.0));
    h.run();
    let (from, to) = (screen(&h, Point::new(300.0, 150.0)), screen(&h, Point::new(340.0, 150.0)));
    drag(&mut h, from, to, Modifiers::NONE);
    let r = state(&h).rect;
    assert!((r.width() - r.height()).abs() < 1e-9 && (r.width() - 240.0).abs() < 1e-9, "{r:?}");

    let mut h = harness(|host| {
        host.ratio = Some(2.0);
        host.default_frame = true;
    });
    h.run();
    let (from, to) = (screen(&h, Point::new(120.0, 100.0)), screen(&h, Point::new(180.0, 110.0)));
    drag(&mut h, from, to, Modifiers::NONE);
    let r = state(&h).rect;
    assert!((r.width() / r.height() - 2.0).abs() < 1e-9, "{r:?}");
}

#[test]
fn a_drag_outside_turns_the_frame_and_shift_snaps_the_turn() {
    let mut h = harness(|_| {});
    h.run();
    let (from, to) = (screen(&h, Point::new(340.0, 260.0)), screen(&h, Point::new(300.0, 290.0)));
    drag(&mut h, from, to, Modifiers::NONE);
    let c = RECT.center();
    let want = turned_angle(c, super::direction(c, Point::new(340.0, 260.0)), 0.0, Point::new(300.0, 290.0), None);
    assert!((state(&h).angle - want).abs() < 1e-6 && want.abs() > 1.0, "{} vs {want}", state(&h).angle);
    assert_eq!(state(&h).rect, RECT, "turning keeps the rect");

    let mut h = harness(|_| {});
    h.run();
    drag(&mut h, from, to, Modifiers::SHIFT);
    let angle = state(&h).angle;
    assert!((angle / 15.0 - (angle / 15.0).round()).abs() < 1e-9 && angle.abs() > 1.0, "{angle}");
}

#[test]
fn a_drag_inside_the_default_frame_draws_a_new_frame_with_shift_alt_and_space() {
    let setup = |host: &mut Host| host.default_frame = true;
    let (from_doc, to_doc) = (Point::new(150.0, 100.0), Point::new(190.0, 120.0));
    let mut h = harness(setup);
    h.run();
    let (from, to) = (screen(&h, from_doc), screen(&h, to_doc));
    drag(&mut h, from, to, Modifiers::NONE);
    assert_eq!(state(&h), CropBox::upright(Rect::new(150.0, 100.0, 190.0, 120.0)));

    let mut h = harness(setup);
    h.run();
    drag(&mut h, from, to, Modifiers::SHIFT);
    assert_eq!(state(&h).rect, Rect::new(150.0, 100.0, 190.0, 140.0));
    let mut h = harness(setup);
    h.run();
    drag(&mut h, from, to, Modifiers::ALT);
    assert_eq!(state(&h).rect, Rect::new(110.0, 80.0, 190.0, 120.0));

    // Space moves the frame being drawn without resizing it; sizing resumes from the moved anchor.
    let mut h = harness(setup);
    h.run();
    hold(&mut h, from, to);
    h.key_down(Key::Space);
    h.event(Event::PointerMoved(to + vec2(20.0, 10.0)));
    h.run();
    assert_eq!(state(&h).rect, Rect::new(170.0, 110.0, 210.0, 130.0), "same size, moved");
    h.key_up(Key::Space);
    h.event(Event::PointerMoved(to + vec2(40.0, 20.0)));
    h.event(button(to + vec2(40.0, 20.0), false, Modifiers::NONE));
    h.run();
    assert_eq!(state(&h).rect, Rect::new(170.0, 110.0, 230.0, 140.0));
}

#[test]
fn a_frame_under_one_unit_is_cancelled_and_the_previous_one_returns() {
    let mut h = harness(|host| host.default_frame = true);
    h.run();
    let from = screen(&h, Point::new(150.0, 100.0));
    hold(&mut h, from, from + vec2(5.0, 5.0));
    assert_ne!(state(&h), CropBox::upright(RECT), "the host shows the new frame while it is dragged");
    h.event(Event::PointerMoved(from + vec2(0.3, 0.2)));
    h.event(button(from + vec2(0.3, 0.2), false, Modifiers::NONE));
    h.run();
    assert_eq!(state(&h), CropBox::upright(RECT));
    assert_eq!(kinds(&h).last(), Some(&"cancel"), "{:?}", kinds(&h));

    // A resize that collapses a side does the same.
    let mut h = harness(|_| {});
    h.run();
    let edge = screen(&h, Point::new(300.0, 150.0));
    let to = screen(&h, Point::new(100.0, 150.0));
    drag(&mut h, edge, to, Modifiers::NONE);
    assert_eq!(state(&h), CropBox::upright(RECT));
    assert_eq!(kinds(&h).last(), Some(&"cancel"));
}

#[test]
fn a_quick_drag_in_one_frame_starts_at_the_press() {
    let mut h = harness(|_| {});
    h.run();
    let from = screen(&h, Point::new(300.0, 220.0));
    let to = from + vec2(40.0, 10.0);
    frame(
        &mut h,
        [Event::PointerMoved(from), button(from, true, Modifiers::NONE), Event::PointerMoved(from + vec2(20.0, 5.0)), Event::PointerMoved(to)],
    );
    frame(&mut h, [button(to, false, Modifiers::NONE)]);
    h.run();
    assert_eq!(state(&h).rect, Rect::new(100.0, 80.0, 340.0, 230.0));
    assert_eq!(kinds(&h).first(), Some(&"begin"));
    assert_eq!(kinds(&h).last(), Some(&"commit"));
}

#[test]
fn double_click_inside_applies_but_not_on_a_handle_or_the_default_frame() {
    let click_twice = |h: &mut Harness<'_, Host>, at: Pos2| {
        // A click per frame: frames are a quarter second apart, and egui's double-click window is 0.3 s.
        frame(h, [Event::PointerMoved(at), button(at, true, Modifiers::NONE), button(at, false, Modifiers::NONE)]);
        frame(h, [button(at, true, Modifiers::NONE), button(at, false, Modifiers::NONE)]);
        h.run();
    };
    let mut h = harness(|_| {});
    h.run();
    let at = screen(&h, Point::new(150.0, 110.0));
    click_twice(&mut h, at);
    assert_eq!(h.state().applied, 1);
    assert!(h.state().events.is_empty(), "clicks don't move the frame");

    let mut h = harness(|_| {});
    h.run();
    let at = screen(&h, Point::new(300.0, 220.0));
    click_twice(&mut h, at);
    assert_eq!(h.state().applied, 0, "on a handle");

    let mut h = harness(|host| host.default_frame = true);
    h.run();
    let at = screen(&h, Point::new(150.0, 110.0));
    click_twice(&mut h, at);
    assert_eq!(h.state().applied, 0, "the untouched default frame");
}

#[test]
fn enter_applies_and_esc_undoes_a_drag_then_discards() {
    let mut h = harness(|_| {});
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().applied, 1);

    let from = screen(&h, Point::new(150.0, 110.0));
    hold(&mut h, from, from + vec2(40.0, 0.0));
    assert_eq!(state(&h), CropBox::upright(RECT + Vec2::new(40.0, 0.0)), "the host shows the drag");
    assert!(h.state().dragging);
    h.key_press(Key::Escape);
    h.run();
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })));
    assert_eq!(state(&h), CropBox::upright(RECT), "Esc undoes the drag");
    assert_eq!(h.state().discarded, 0);
    let count = h.state().events.len();
    h.event(Event::PointerMoved(from + vec2(60.0, 0.0)));
    h.event(button(from + vec2(60.0, 0.0), false, Modifiers::NONE));
    h.run();
    assert_eq!(h.state().events.len(), count, "the rest of the press does nothing");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().discarded, 1, "a second Esc discards");
    assert_eq!(h.state().applied, 1);
}

#[test]
fn esc_before_the_first_move_ends_the_press() {
    let mut h = harness(|_| {});
    h.run();
    let from = screen(&h, Point::new(150.0, 110.0));
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.run();
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().discarded, 0);
    h.event(Event::PointerMoved(from + vec2(30.0, 0.0)));
    h.run();
    h.event(button(from + vec2(30.0, 0.0), false, Modifiers::NONE));
    h.run();
    assert!(h.state().events.is_empty());
    assert_eq!(state(&h), CropBox::upright(RECT));
}

#[test]
fn arrows_move_the_frame_on_the_page_x_swaps_and_a_picked_part_resizes() {
    let b = CropBox { rect: RECT, angle: 30.0 };
    let mut h = harness_with(b, 0.0, false, |_| {});
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(state(&h), b.moved(Vec2::new(1.0, 0.0)), "a turned frame moves on the page and keeps its angle");
    assert!(matches!(h.state().events.last(), Some(EditEvent::Set { .. })));
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run();
    assert_eq!(state(&h), b.moved(Vec2::new(1.0, 10.0)));
    let before = state(&h);
    h.key_press(Key::X);
    h.run();
    assert_eq!(state(&h), before.swapped());
    assert_eq!(state(&h).angle, 30.0);
    assert!((state(&h).rect.width() - 140.0).abs() < 1e-9 && (state(&h).rect.center() - before.rect.center()).hypot() < 1e-9);

    let mut h = harness(|_| {});
    h.run();
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state().picked, Some(BoxPart::Inside), "focus picks the whole frame");
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().picked, Some(BoxPart::Corner(0)));
    h.key_press(Key::ArrowLeft);
    h.run();
    assert_eq!(state(&h).rect, Rect::new(99.0, 80.0, 300.0, 220.0), "the corner moves as a drag would");
}

#[test]
fn keys_are_ignored_while_a_text_field_has_focus() {
    let mut h = harness(|host| host.text = true);
    h.run();
    h.run();
    assert!(h.ctx.text_edit_focused(), "the field holds the keyboard");
    h.key_press(Key::ArrowRight);
    h.key_press(Key::Enter);
    h.key_press(Key::X);
    h.run();
    assert!(h.state().events.is_empty());
    assert_eq!((h.state().applied, h.state().discarded), (0, 0));
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

#[test]
fn page_keys_pick_every_part_and_each_has_a_name() {
    let mut h = harness(|_| {});
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let mut seen = vec![h.state().picked];
    for _ in 0..8 {
        h.key_press(Key::PageDown);
        h.run();
        seen.push(h.state().picked);
    }
    let want: Vec<_> = std::iter::once(BoxPart::Inside).chain((0..4).map(BoxPart::Corner)).chain((0..4).map(BoxPart::Edge)).map(Some).collect();
    assert_eq!(seen, want);
    let group = h.get_by_label("Crop frame");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    assert_eq!(group.accesskit_node().value().as_deref(), Some("200.0 px × 140.0 px, 100.0 px, 80.0 px, 0.0°"));
    let objects: Vec<_> = descendants(group.accesskit_node())
        .into_iter()
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default(), n.data().is_selected()))
        .collect();
    let labels: Vec<_> = objects.iter().map(|(l, ..)| l.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Whole frame",
            "Top left corner",
            "Top right corner",
            "Bottom right corner",
            "Bottom left corner",
            "Top edge",
            "Right edge",
            "Bottom edge",
            "Left edge"
        ]
    );
    assert_eq!(objects.iter().filter(|(.., selected)| *selected == Some(true)).count(), 1);
    assert_eq!(objects[3].1, "300.0 px, 220.0 px");
    assert_eq!(objects[8].1, "100.0 px, 150.0 px");
    let stops = descendants(group.accesskit_node()).into_iter().filter(|n| n.data().supports_action(egui::accesskit::Action::Focus)).count();
    assert_eq!(stops, 0, "one Tab stop");
}

fn readout(h: &Harness<'_, Host>) -> Option<String> {
    h.query_by(|n| n.role() == Role::Tooltip).and_then(|n| n.accesskit_node().label())
}

#[test]
fn the_readout_shows_the_crop_size_or_the_angle() {
    let cases = [(Point::new(300.0, 220.0), vec2(20.0, 10.0), "W: 220 px\nH: 150 px"), (Point::new(340.0, 260.0), vec2(-30.0, 30.0), "Angle: ")];
    for (at, by, want) in cases {
        let mut h = harness(|host| host.readout = true);
        h.run();
        let from = screen(&h, at);
        hold(&mut h, from, from + by);
        let text = readout(&h).unwrap_or_default();
        assert!(text.starts_with(want), "{at:?}: {text:?}");
        assert!(h.state().sizing.is_some() || h.state().turning.is_some());
        h.event(button(from + by, false, Modifiers::NONE));
        h.run();
        assert_eq!(readout(&h), None, "only during the drag");
    }
    // Moving shows none, and neither does a frame without the host's labels.
    let mut h = harness(|host| host.readout = true);
    h.run();
    let from = screen(&h, Point::new(150.0, 110.0));
    hold(&mut h, from, from + vec2(20.0, 5.0));
    assert_eq!(readout(&h), None);
    let mut h = harness(|_| {});
    h.run();
    let from = screen(&h, Point::new(300.0, 220.0));
    hold(&mut h, from, from + vec2(20.0, 10.0));
    assert_eq!(readout(&h), None);
}

#[test]
fn the_turn_angle_never_reads_minus_zero() {
    let mut h = harness(|host| host.readout = true);
    h.run();
    let from = screen(&h, Point::new(340.0, 260.0));
    hold(&mut h, from, from + vec2(0.0, 0.01));
    h.event(Event::PointerMoved(from + vec2(0.0, -0.01)));
    h.run();
    let text = readout(&h).unwrap_or_default();
    assert!(text.starts_with("Angle: ") && !text.contains("-0.0"), "{text:?}");
}

fn cursor(h: &Harness<'_, Host>) -> CursorIcon {
    h.output().platform_output.cursor_icon
}

#[test]
fn the_pointer_follows_the_part_under_it() {
    let mut h = harness(|_| {});
    h.run();
    for (at, want) in [
        (Point::new(300.0, 150.0), CursorIcon::ResizeHorizontal),
        (Point::new(200.0, 80.0), CursorIcon::ResizeVertical),
        (Point::new(300.0, 220.0), CursorIcon::ResizeNwSe),
        (Point::new(300.0, 80.0), CursorIcon::ResizeNeSw),
        (Point::new(200.0, 150.0), CursorIcon::Move),
        (Point::new(350.0, 270.0), CursorIcon::None),
    ] {
        h.event(Event::PointerMoved(screen(&h, at)));
        h.run();
        assert_eq!(cursor(&h), want, "{at:?}");
    }
    // Inside the untouched default frame a drag draws.
    let mut h = harness(|host| host.default_frame = true);
    h.run();
    h.event(Event::PointerMoved(screen(&h, Point::new(200.0, 150.0))));
    h.run();
    assert_eq!(cursor(&h), CursorIcon::Crosshair);
    // While drawing.
    let from = screen(&h, Point::new(150.0, 100.0));
    hold(&mut h, from, from + vec2(20.0, 10.0));
    assert_eq!(cursor(&h), CursorIcon::Crosshair);
}

#[test]
fn the_resize_arrow_follows_the_frame_angle_the_view_rotation_and_the_flip() {
    let edge = |angle: f64, rotation: f64, flip: bool| {
        let b = CropBox { rect: RECT, angle };
        let mut h = harness_with(b, rotation, flip, |_| {});
        h.run();
        let right = screen(&h, b.corners()[1].midpoint(b.corners()[2]));
        h.event(Event::PointerMoved(right));
        h.run();
        cursor(&h)
    };
    assert_eq!(edge(0.0, 0.0, false), CursorIcon::ResizeHorizontal);
    assert_eq!(edge(90.0, 0.0, false), CursorIcon::ResizeVertical);
    assert_eq!(edge(45.0, 0.0, false), CursorIcon::ResizeNwSe);
    assert_eq!(edge(0.0, 90.0, false), CursorIcon::ResizeVertical, "the view's rotation adds to the frame's");
    assert_eq!(edge(45.0, 0.0, true), CursorIcon::ResizeNeSw, "a flipped view mirrors the arrow");
}

#[test]
fn the_turn_pointer_bends_towards_the_corner_of_the_pointers_quadrant() {
    let v = CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
    let b = CropBox::upright(RECT);
    let s = std::f32::consts::FRAC_1_SQRT_2;
    for (p, want) in [((20.0, 20.0), (s, s)), ((380.0, 20.0), (-s, s)), ((380.0, 280.0), (-s, -s)), ((20.0, 280.0), (s, -s))] {
        let d = turn_toward(&v, &b, Pos2::new(p.0, p.1));
        assert!((d.x - want.0).abs() < 1e-6 && (d.y - want.1).abs() < 1e-6, "{p:?}: {d:?}");
    }
    // They turn with the frame: a quarter of the diagonal, 30° on.
    let turned = CropBox { rect: RECT, angle: 30.0 };
    let d = turn_toward(&v, &turned, Pos2::new(20.0, 20.0));
    let (sin, cos) = 30f32.to_radians().sin_cos();
    assert!((d.x - s * (cos - sin)).abs() < 1e-5 && (d.y - s * (cos + sin)).abs() < 1e-5, "{d:?}");
}

// Drawing.

/// The 3 point segments of the last frame, in drawing order.
fn brackets(h: &Harness<'_, Host>) -> Vec<[Pos2; 2]> {
    fn collect(shape: &Shape, out: &mut Vec<[Pos2; 2]>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            Shape::LineSegment { points, stroke } if (stroke.width - 3.0).abs() < 1e-6 => out.push(*points),
            _ => {}
        }
    }
    let mut out = Vec::new();
    h.output().shapes.iter().for_each(|c| collect(&c.shape, &mut out));
    out
}

fn len(s: &[Pos2; 2]) -> f32 {
    s[0].distance(s[1])
}

#[test]
fn brackets_are_14_long_up_to_a_third_of_the_side_and_bars_a_quarter() {
    let mut h = harness(|_| {});
    h.run();
    let segments = brackets(&h);
    assert_eq!(segments.len(), 12, "8 bracket arms and 4 bars");
    assert!(segments[..8].iter().all(|s| (len(s) - 14.0).abs() < 1e-4));
    assert!(segments[8..].iter().all(|s| (len(s) - 14.0).abs() < 1e-4), "bars are 2 × min(14, side / 4) / 2");

    // A 30 × 24 frame: the arm is a third of the short side (8); the bars are min(8, side / 4).
    let mut h = harness_with(CropBox::upright(Rect::new(100.0, 100.0, 130.0, 124.0)), 0.0, false, |_| {});
    h.run();
    let segments = brackets(&h);
    assert!(segments[..8].iter().all(|s| (len(s) - 8.0).abs() < 1e-4), "{segments:?}");
    let (horizontal, vertical): (Vec<_>, Vec<_>) = segments[8..].iter().partition(|s| (s[0].y - s[1].y).abs() < 1e-4);
    assert!(horizontal.iter().all(|s| (len(s) - 7.5).abs() < 1e-4), "{horizontal:?}");
    assert!(vertical.iter().all(|s| (len(s) - 6.0).abs() < 1e-4), "{vertical:?}");
}

#[test]
fn a_view_turned_a_quarter_draws_the_frames_own_shapes_and_a_turned_square_maps_onto_itself() {
    let mut upright = harness(|_| {});
    upright.run();
    let mut turned = harness_with(CropBox::upright(RECT), 90.0, false, |_| {});
    turned.run();
    let (a, b) = (brackets(&upright), brackets(&turned));
    assert_eq!(a.len(), b.len());
    for (s, t) in a.iter().zip(&b) {
        assert!((len(s) - len(t)).abs() < 1e-3, "lengths are the frame's own: {s:?} {t:?}");
    }
    let square = Rect::new(150.0, 100.0, 250.0, 200.0);
    let mut q0 = harness_with(CropBox::upright(square), 0.0, false, |_| {});
    q0.run();
    let mut q1 = harness_with(CropBox { rect: square, angle: 90.0 }, 0.0, false, |_| {});
    q1.run();
    let key = |segments: Vec<[Pos2; 2]>| {
        let mut v: Vec<_> =
            segments.iter().map(|s| (s[0].x.min(s[1].x).round() as i32, s[0].y.min(s[1].y).round() as i32, len(s).round() as i32)).collect();
        v.sort_unstable();
        v
    };
    assert_eq!(key(brackets(&q0)), key(brackets(&q1)));
}

#[test]
fn degenerate_frames_draw_nothing_and_never_panic() {
    for rect in [Rect::new(100.0, 100.0, 100.0, 150.0), Rect::new(100.0, 100.0, 100.0 + 1e-6, 150.0), Rect::new(f64::NAN, 0.0, 10.0, 10.0)] {
        let mut h = harness_with(CropBox::upright(rect), 0.0, false, |host| host.shield = Some(Color32::from_black_alpha(100)));
        h.run();
        assert!(brackets(&h).is_empty(), "{rect:?}");
        let from = screen(&h, Point::new(105.0, 120.0));
        drag(&mut h, from, from + vec2(5.0, 5.0), Modifiers::NONE);
        h.key_press(Key::ArrowLeft);
        h.run();
    }
}

/// The filled rects and meshes of the last frame in `color`.
fn shield_shapes(h: &Harness<'_, Host>, color: Color32) -> (usize, usize) {
    fn count(shape: &Shape, color: Color32, out: &mut (usize, usize)) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| count(s, color, out)),
            Shape::Rect(r) if r.fill == color => out.0 += 1,
            Shape::Mesh(m) if !m.vertices.is_empty() && m.vertices.iter().all(|v| v.color == color) => out.1 += 1,
            _ => {}
        }
    }
    let mut out = (0, 0);
    h.output().shapes.iter().for_each(|c| count(&c.shape, color, &mut out));
    out
}

#[test]
fn the_shield_is_four_rects_upright_and_one_mesh_turned() {
    let dim = Color32::from_rgba_unmultiplied(10, 20, 30, 190);
    let mut h = harness(|host| host.shield = Some(dim));
    h.run();
    assert_eq!(shield_shapes(&h, dim), (4, 0));
    let mut h = harness_with(CropBox { rect: RECT, angle: 20.0 }, 0.0, false, |host| host.shield = Some(dim));
    h.run();
    assert_eq!(shield_shapes(&h, dim), (0, 1));
    let mut h = harness_with(CropBox::upright(RECT), 30.0, false, |host| host.shield = Some(dim));
    h.run();
    assert_eq!(shield_shapes(&h, dim), (0, 1), "a rotated view");
    let mut h = harness_with(CropBox::upright(RECT), 90.0, true, |host| host.shield = Some(dim));
    h.run();
    assert_eq!(shield_shapes(&h, dim), (4, 0), "a quarter turn keeps the axes");
    let mut h = harness(|_| {});
    h.run();
    assert_eq!(shield_shapes(&h, dim), (0, 0), "no shield unless the host passes one");
}

#[test]
fn guides_draw_in_the_guide_colour_only_when_the_host_passes_one() {
    fn count(shape: &Shape, color: Color32, out: &mut usize) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| count(s, color, out)),
            Shape::Path(p) if p.stroke.color == ColorMode::Solid(color) => *out += 1,
            _ => {}
        }
    }
    let lines = |h: &Harness<'_, Host>, color: Color32| {
        let mut n = 0;
        h.output().shapes.iter().for_each(|c| count(&c.shape, color, &mut n));
        n
    };
    let mut h = harness(|_| {});
    h.run();
    let guide = CropFrame::style(&current(&h.ctx)).guide;
    assert_eq!(lines(&h, guide.color), 0);
    let mut h = harness(|host| host.guide = Some((CropGuide::Thirds, 0)));
    h.run();
    assert_eq!(lines(&h, guide.color), 4);
    assert!((guide.width - 1.0).abs() < 1e-6);
}

#[test]
fn bounds_keep_the_frame_inside_and_the_host_snap_moves_the_dragged_handle() {
    let mut h = harness(|host| host.bounds = Some(Rect::new(0.0, 0.0, 400.0, 300.0)));
    h.run();
    let from = screen(&h, Point::new(300.0, 220.0));
    drag(&mut h, from, from + vec2(300.0, 200.0), Modifiers::NONE);
    assert_eq!(state(&h).rect, Rect::new(100.0, 80.0, 400.0, 300.0));
    let from = screen(&h, Point::new(200.0, 150.0));
    drag(&mut h, from, from + vec2(-400.0, -400.0), Modifiers::NONE);
    assert_eq!(state(&h).rect, Rect::new(0.0, 0.0, 300.0, 220.0));
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowLeft);
    h.run();
    assert_eq!(state(&h).rect.x0, 0.0, "keys stay inside too");

    let mut h = harness(|host| host.snap = Some(10.0));
    h.run();
    let from = screen(&h, Point::new(300.0, 220.0));
    drag(&mut h, from, from + vec2(13.0, 7.0), Modifiers::NONE);
    assert_eq!(state(&h).rect, Rect::new(100.0, 80.0, 310.0, 230.0));
}

fn crop_style() -> super::CropStyle {
    let appearance =
        Appearance { mode: Mode::Dark, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
    let theme = builtins().iter().find(|t| t.id.as_str() == "craft.pro").unwrap().clone();
    CropFrame::style(&UiTokens::new(resolve(&theme, appearance, None).tokens, appearance, Mode::Dark, FontFamilies::default()))
}

/// The host-driven calls give the frame that `show` gives for the same gesture.
#[test]
fn the_host_driven_press_track_release_match_show() {
    let style = crop_style();
    let turned = CropBox { rect: RECT, angle: 20.0 };
    // (frame, view rotation, view flip, default frame, press point, travel, modifiers): move,
    // resize a corner in a turned view, turn, draw.
    let cases = [
        (CropBox::upright(RECT), 0.0, false, false, Point::new(150.0, 110.0), vec2(30.0, 20.0), Modifiers::NONE),
        (turned, 40.0, true, false, turned.corners()[2], vec2(14.0, -9.0), Modifiers::SHIFT),
        (CropBox::upright(RECT), 0.0, false, false, Point::new(350.0, 110.0), vec2(0.0, 40.0), Modifiers::NONE),
        (CropBox::upright(RECT), 0.0, false, true, Point::new(150.0, 110.0), vec2(40.0, 20.0), Modifiers::ALT),
    ];
    for (b, rotation, flip, default_frame, at, by, mods) in cases {
        let mut h = harness_with(b, rotation, flip, |host| host.default_frame = default_frame);
        h.run();
        let (from, to) = (screen(&h, at), screen(&h, at) + by);
        drag(&mut h, from, to, mods);

        let mut v = view(&h);
        let f = CropFrame::new("crop", &b, names()).default_frame(default_frame);
        let mut press = f.press(&v, &style, from).expect("press");
        assert!(!press.began());
        assert_eq!(f.track(&mut v, &mut press, from, mods, false), super::CropTrack { begun: false, state: None });
        let first = f.track(&mut v, &mut press, from.lerp(to, 0.5), mods, false);
        assert!(first.begun, "{first:?}");
        let end = f.track(&mut v, &mut press, to, mods, false).state.expect("a frame");
        assert_eq!(f.release(&mut v, &style, &press, Some(to)), super::CropRelease::Commit(end));
        assert_eq!(press.last(), &end);
        assert!(near(&state(&h), &end), "{:?} vs {end:?}", state(&h));
        assert_eq!(state(&h).angle, end.angle);
    }
}

#[test]
fn the_host_driven_hit_and_double_click_follow_the_frame() {
    let style = crop_style();
    let view = CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
    let b = CropBox::upright(RECT);
    let f = CropFrame::new("crop", &b, names());
    let at = |p: Point| view.to_screen(p);
    assert_eq!(f.hit(&view, &style, at(Point::new(150.0, 110.0))), Some(BoxPart::Inside));
    assert_eq!(f.hit(&view, &style, at(Point::new(300.0, 220.0))), Some(BoxPart::Corner(2)));
    assert_eq!(f.hit(&view, &style, at(Point::new(20.0, 20.0))), Some(BoxPart::Outside));
    assert!(f.double_click(&view, &style, at(Point::new(150.0, 110.0))));
    assert!(!f.double_click(&view, &style, at(Point::new(300.0, 220.0))));
    let d = CropFrame::new("crop", &b, names()).default_frame(true);
    assert!(!d.double_click(&view, &style, at(Point::new(150.0, 110.0))));
    let s = CropFrame::new("crop", &b, names()).straighten(true);
    assert_eq!(s.hit(&view, &style, at(Point::new(150.0, 110.0))), None);
}

fn plain_view() -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0)
}

/// A press reports what it does, for the host's cursor and readout.
#[test]
fn a_press_reports_its_kind() {
    use super::CropPressKind::{Draw, Move, Resize, Straighten, Turn};
    let style = crop_style();
    let view = plain_view();
    let b = CropBox::upright(RECT);
    let kind = |f: &CropFrame<'_>, p: Point| f.press(&view, &style, view.to_screen(p)).expect("press").kind();
    let f = CropFrame::new("crop", &b, names());
    assert_eq!(kind(&f, Point::new(150.0, 110.0)), Move);
    assert_eq!(kind(&f, Point::new(300.0, 220.0)), Resize(BoxPart::Corner(2)));
    assert_eq!(kind(&f, Point::new(200.0, 80.0)), Resize(BoxPart::Edge(0)));
    assert_eq!(kind(&f, Point::new(20.0, 20.0)), Turn);
    assert_eq!(kind(&CropFrame::new("crop", &b, names()).default_frame(true), Point::new(150.0, 110.0)), Draw);
    assert_eq!(kind(&CropFrame::new("crop", &b, names()).straighten(true), Point::new(150.0, 110.0)), Straighten);
}

/// Without a frame every press draws one from the press point.
#[test]
fn a_press_with_no_frame_draws_a_new_frame() {
    let style = crop_style();
    let mut view = plain_view();
    let f = CropFrame::unframed("crop", names());
    let (from, to) = (view.to_screen(Point::new(50.0, 60.0)), view.to_screen(Point::new(150.0, 120.0)));
    assert_eq!(f.hit(&view, &style, from), Some(BoxPart::Inside));
    assert!(!f.double_click(&view, &style, from));
    let mut press = f.press(&view, &style, from).expect("press");
    assert_eq!(press.kind(), super::CropPressKind::Draw);
    let end = f.track(&mut view, &mut press, to, Modifiers::NONE, false).state.expect("a frame");
    assert_eq!(end.rect, Rect::new(50.0, 60.0, 150.0, 120.0));
    assert_eq!(f.release(&mut view, &style, &press, Some(to)), super::CropRelease::Commit(end));
}

/// The plain-rect hit test and the frame's give the same parts and reach.
#[test]
fn the_rect_hit_is_the_frames_hit() {
    let style = crop_style();
    let view = plain_view();
    let b = CropBox::upright(RECT);
    let f = CropFrame::new("crop", &b, names());
    let reach = view.doc_len(style.hit);
    for p in [(150.0, 110.0), (300.0, 220.0), (205.0, 76.0), (100.0, 150.0), (20.0, 20.0), (306.0, 150.0), (320.0, 150.0)] {
        let p = Point::new(p.0, p.1);
        assert_eq!(f.hit(&view, &style, view.to_screen(p)), Some(super::hit_rect(RECT, p, reach)), "{p:?}");
    }
    assert_eq!(super::hit_rect(RECT, Point::new(100.0, 80.0), reach), BoxPart::Corner(0));
    assert_eq!(super::hit_rect(RECT, Point::new(200.0, 224.0), reach), BoxPart::Edge(2));
    assert_eq!(super::hit_rect(RECT, Point::new(20.0, 20.0), reach), BoxPart::Outside);
}

/// The host-driven gesture readout is written with the units the host passes, from a bare
/// context, and not at all when it passes none.
#[test]
fn paint_gesture_takes_the_labels_and_units_as_values() {
    use egui::{LayerId, Order};
    let style = crop_style();
    let view = plain_view();
    let b = CropBox::upright(RECT);
    let labels = CropLabels::new("W:", "H:", "Angle:");
    let units = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
    let tooltips = |readout: Option<(&CropLabels, &RulerUnits)>| {
        let ctx = egui::Context::default();
        let mut found = 0;
        ctx.run_ui(Default::default(), |ui| {
            let f = CropFrame::new("crop", &b, names());
            let mut v = view;
            let from = view.to_screen(Point::new(300.0, 220.0));
            let mut press = f.press(&v, &style, from).expect("press");
            f.track(&mut v, &mut press, from + vec2(20.0, 10.0), Modifiers::NONE, false);
            f.paint_gesture(&ui.painter_at(view.rect), &style, &press, from + vec2(20.0, 10.0), readout);
        })
        .drop_without_applying_deltas();
        ctx.memory(|m| found += m.areas().visible_layer_ids().iter().filter(|l: &&LayerId| l.order == Order::Tooltip).count());
        found
    };
    assert_eq!(tooltips(None), 0);
    assert_eq!(tooltips(Some((&labels, &units))), 1);
}

#[test]
fn the_style_is_the_crop_tokens_and_a_halo_shows_in_high_contrast_only() {
    let tokens = |contrast| {
        let appearance = Appearance { mode: Mode::Dark, contrast, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
        let theme = builtins().iter().find(|t| t.id.as_str() == "craft.pro").unwrap().clone();
        UiTokens::new(resolve(&theme, appearance, None).tokens, appearance, Mode::Dark, FontFamilies::default())
    };
    let normal = CropFrame::style(&tokens(Contrast::Standard));
    let t = tokens(Contrast::Standard);
    let (c, o) = (&t.component.crop, &t.color.overlay);
    assert_eq!((normal.frame.width, normal.frame.color), (c.line, o.crop.to_egui()));
    assert_eq!((normal.guide.width, normal.guide.color), (c.guide_line, o.crop_guide.to_egui()));
    assert_eq!((normal.bracket.width, normal.hit, normal.bracket_arm), (c.bracket, c.hit, c.bracket_arm));
    assert_eq!(normal.halo.width, 0.0);
    let high = CropFrame::style(&tokens(Contrast::High));
    assert!(high.halo.width > high.bracket.width);
}

#[test]
fn a_turn_stops_at_the_bounds() {
    let tight = Rect::new(100.0, 80.0, 300.0, 220.0);
    let (from, to) = (Point::new(340.0, 260.0), Point::new(300.0, 290.0));
    let mut h = harness(|host| host.bounds = Some(tight));
    h.run();
    let (a, b) = (screen(&h, from), screen(&h, to));
    drag(&mut h, a, b, Modifiers::NONE);
    assert_eq!(state(&h), CropBox::upright(RECT), "a turn that would leave the bounds is refused");

    let mut h = harness(|host| host.bounds = Some(Rect::new(0.0, 0.0, 400.0, 300.0)));
    h.run();
    let (a, b) = (screen(&h, from), screen(&h, to));
    drag(&mut h, a, b, Modifiers::NONE);
    let t = state(&h).turned_bounds();
    assert!(state(&h).angle.abs() > 1.0 && t.x0 >= 0.0 && t.y0 >= 0.0 && t.x1 <= 400.0 && t.y1 <= 300.0, "{:?}", state(&h));
}

#[test]
fn a_held_ratio_is_never_cut_by_the_bounds() {
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
    let wide = CropBox::upright(Rect::new(100.0, 80.0, 500.0, 280.0));
    assert_eq!(within(wide, bounds, true, false).unwrap().rect, Rect::new(100.0, 80.0, 400.0, 280.0));
    assert!(within(wide, bounds, true, true).is_none(), "the cut would change the 2:1 ratio");
    assert!(within(CropBox::upright(Rect::new(100.0, 80.0, 300.0, 180.0)), bounds, true, true).is_some());

    let mut h = harness_with(CropBox::upright(Rect::new(100.0, 80.0, 200.0, 180.0)), 0.0, false, |host| {
        host.ratio = Some(1.0);
        host.bounds = Some(bounds);
    });
    h.run();
    let from = screen(&h, Point::new(200.0, 180.0));
    let to = screen(&h, Point::new(500.0, 200.0));
    drag(&mut h, from, to, Modifiers::NONE);
    let r = state(&h).rect;
    assert!((r.width() - r.height()).abs() < 1e-9 && r.x1 <= 400.0 && r.y1 <= 300.0, "{r:?}");
}

#[test]
fn enter_is_ignored_while_another_widget_has_the_focus() {
    let mut h = harness(|host| host.button = true);
    h.run();
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().applied, 0, "Enter belongs to the focused button");
}
