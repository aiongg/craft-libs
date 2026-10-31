use egui::{Event, Key, Modifiers, PointerButton, Pos2, Sense, vec2};
use egui_kittest::Harness;
use kurbo::{Point, Rect, Vec2};

use super::{CropBox, CropFrame};
use crate::ui::CropNames;
use craft_ui::canvas::CanvasView;
use craft_ui::canvas::quad::BoxPart;
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

const RECT: Rect = Rect::new(100.0, 80.0, 300.0, 220.0);

struct Host {
    state: CropBox,
    before: Option<(CropBox, CanvasView)>,
    view: CanvasView,
    auto_center: bool,
    /// The host snaps the dragged part's x (a move: the frame's left edge) to this document x.
    snap_x: Option<f64>,
    default_frame: bool,
    told: std::rc::Rc<std::cell::Cell<Option<(BoxPart, CropBox)>>>,
    events: Vec<EditEvent<CropBox>>,
}

fn names() -> CropNames {
    CropNames {
        group: "Crop frame".into(),
        whole: "Whole frame".into(),
        corners: ["Top left corner", "Top right corner", "Bottom right corner", "Bottom left corner"].map(String::from),
        edges: ["Top edge", "Right edge", "Bottom edge", "Left edge"].map(String::from),
    }
}

/// A frame turned `angle` in a view that shows it upright, as the host sets it on picking the tool.
fn harness(angle: f64, auto_center: bool) -> Harness<'static, Host> {
    let state = CropBox { rect: RECT, angle };
    let host = Host {
        state,
        before: None,
        view: CanvasView::new(egui::Rect::ZERO, Point::new(200.0, 150.0), 1.0),
        auto_center,
        snap_x: None,
        default_frame: false,
        told: Default::default(),
        events: Vec::new(),
    };
    let mut h = Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            host.view.rect = rect;
            let state = host.state;
            let (snap_x, told) = (host.snap_x, host.told.clone());
            let r = CropFrame::new("crop", &state, names())
                .auto_center(host.auto_center)
                .default_frame(host.default_frame)
                .snap(move |part, b, _| {
                    told.set(Some((part, *b)));
                    let x = if part == BoxPart::Inside { b.rect.x0 } else { b.part_point(part).map_or(0.0, |p| p.x) };
                    Vec2::new(snap_x.map_or(0.0, |target| target - x), 0.0)
                })
                .show_image(ui, &canvas, &mut host.view);
            match &r.event {
                Some(EditEvent::Begin { .. }) => host.before = Some((host.state, host.view)),
                Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => host.state = *value,
                Some(EditEvent::Cancel { .. }) => {
                    if let Some((before, _)) = host.before.take() {
                        host.state = before;
                    }
                }
                Some(_) | None => {}
            }
            host.events.extend(r.event);
        },
        host,
    );
    h.run();
    let host = h.state_mut();
    host.view = CropFrame::view_for(&host.state, &host.view);
    h.run();
    h
}

fn view(h: &Harness<'_, Host>) -> CanvasView {
    h.state().view
}

fn screen(h: &Harness<'_, Host>, p: Point) -> Pos2 {
    view(h).to_screen(p)
}

fn quad(h: &Harness<'_, Host>) -> [Pos2; 4] {
    let (v, b) = (view(h), h.state().state);
    b.corners().map(|p| v.to_screen(p))
}

fn same(a: [Pos2; 4], b: [Pos2; 4]) -> bool {
    a.iter().zip(b).all(|(p, q)| p.distance(q) < 1e-2)
}

fn press(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run();
}

fn move_to(h: &mut Harness<'_, Host>, to: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(to));
    h.run();
}

fn release(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

#[test]
fn a_drag_inside_moves_the_frame_and_the_view_against_the_pointer() {
    let mut h = harness(30.0, false);
    let (before, v0) = (quad(&h), view(&h));
    let from = screen(&h, RECT.center());
    press(&mut h, from);
    move_to(&mut h, from + vec2(30.0, 20.0), Modifiers::NONE);
    release(&mut h, from + vec2(30.0, 20.0));
    let d = v0.unmap_vec(vec2(30.0, 20.0));
    let (b, v) = (h.state().state, view(&h));
    let moved = CropBox { rect: RECT, angle: 30.0 }.moved(-d);
    assert!(b.corners().iter().zip(moved.corners()).all(|(p, q)| p.distance(q) < 1e-6), "{b:?}");
    assert!((v.center - (v0.center - d)).hypot() < 1e-3, "{:?}", v.center);
    assert_eq!(v.rotation, v0.rotation);
    assert!(same(before, quad(&h)), "the frame stays put on screen");
}

#[test]
fn a_drag_outside_turns_the_view_to_minus_the_angle_and_shift_snaps() {
    let mut h = harness(0.0, false);
    let c = screen(&h, RECT.center());
    let start = c + vec2(150.0, 0.0);
    let before = quad(&h);
    let swept = |deg: f64| c + vec2((150.0 * deg.to_radians().cos()) as f32, (150.0 * deg.to_radians().sin()) as f32);
    press(&mut h, start);
    move_to(&mut h, swept(20.0), Modifiers::NONE);
    let (b, v) = (h.state().state, view(&h));
    assert!((b.angle + 20.0).abs() < 1e-3, "{b:?}");
    assert!((v.rotation - 20.0).abs() < 1e-3, "{v:?}");
    assert!(same(before, quad(&h)), "the frame stays upright and centred");
    move_to(&mut h, swept(22.0), Modifiers::SHIFT);
    let (b, v) = (h.state().state, view(&h));
    assert_eq!((b.angle, v.rotation), (-15.0, 15.0));
    // The view the drag works from is the press view: coming back gives the start exactly.
    move_to(&mut h, start, Modifiers::NONE);
    assert!(h.state().state.angle.abs() < 1e-6 && view(&h).rotation.abs() < 1e-6);
    release(&mut h, start);
}

#[test]
fn auto_center_keeps_the_frame_centre_on_screen_and_the_handle_under_the_pointer() {
    for angle in [0.0, 30.0] {
        let mut h = harness(angle, true);
        let b0 = h.state().state;
        let centre = screen(&h, b0.center());
        let corner = screen(&h, b0.corners()[2]);
        press(&mut h, corner);
        let to = corner + vec2(40.0, 25.0);
        move_to(&mut h, to, Modifiers::NONE);
        let b = h.state().state;
        assert!((b.rect.width() - RECT.width()).abs() > 1.0, "resized");
        assert!(screen(&h, b.center()).distance(centre) < 1e-2, "centre kept at {angle}");
        assert!(screen(&h, b.corners()[2]).distance(to) < 1e-1, "handle follows at {angle}");
        assert_eq!(view(&h).rotation, -angle);
        // On the image the opposite corner stays put.
        assert!(b.corners()[0].distance(b0.corners()[0]) < 1e-6);
        release(&mut h, to);
    }
}

#[test]
fn the_view_for_a_frame_keeps_the_pivot_on_screen_and_round_trips() {
    let v0 = CanvasView {
        rotation: 10.0,
        flip_x: true,
        ..CanvasView::new(egui::Rect::from_min_size(Pos2::new(5.0, 7.0), vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.5)
    };
    let b = CropBox { rect: RECT, angle: 37.0 };
    let pivot = Point::new(120.0, 90.0);
    let v = CropFrame::view_for_about(&b, &v0, pivot);
    assert!((v.rotation + 37.0).abs() < 1e-9);
    assert!(v.to_screen(pivot).distance(v0.to_screen(pivot)) < 1e-3);
    let back = CropFrame::view_for_about(&CropBox { rect: RECT, angle: -10.0 }, &v, pivot);
    assert!((back.rotation - 10.0).abs() < 1e-9 && (back.center - v0.center).hypot() < 1e-6, "{back:?}");
    let upright = CropFrame::view_for(&b, &v0);
    let q = b.corners().map(|p| upright.to_screen(p));
    assert!((q[0].y - q[1].y).abs() < 1e-3 && (q[0].x - q[3].x).abs() < 1e-3, "{q:?}");
}

#[test]
fn esc_during_a_drag_restores_the_frame_and_the_view() {
    let mut h = harness(0.0, false);
    let (b0, v0) = (h.state().state, view(&h));
    let c = screen(&h, RECT.center());
    press(&mut h, c + vec2(150.0, 0.0));
    move_to(&mut h, c + vec2(100.0, 100.0), Modifiers::NONE);
    assert_ne!(view(&h), v0);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!((h.state().state, view(&h)), (b0, v0));
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { .. })));
}

#[test]
fn arrows_move_the_image_under_the_frame() {
    let mut h = harness(30.0, false);
    let (before, v0) = (quad(&h), view(&h));
    h.key_press(Key::ArrowRight);
    h.run();
    let d = super::screen_step(&v0, Vec2::new(1.0, 0.0));
    assert!(same(before, quad(&h)), "the frame stays put on screen");
    assert!((view(&h).center - (v0.center - d)).hypot() < 1e-6);
}

#[test]
fn a_move_snaps_the_frame_and_stays_snapped_while_the_pointer_is_still() {
    for auto_center in [false, true] {
        let mut h = harness(0.0, auto_center);
        h.state_mut().snap_x = Some(0.0);
        let from = screen(&h, RECT.center());
        press(&mut h, from);
        // Raw: the frame's left edge would be at x = -3; the image follows the frame.
        let at = from + vec2(-103.0, 20.0);
        for pass in 0..4 {
            move_to(&mut h, at, Modifiers::NONE);
            let b = h.state().state;
            assert!(b.rect.x0.abs() < 1e-3, "pass {pass}, auto_center={auto_center}: {b:?}");
            assert!((b.rect.y0 - (RECT.y0 - 20.0)).abs() < 1e-3, "pass {pass}: {b:?}");
        }
        assert_eq!(h.state().told.get().map(|(part, _)| part), Some(BoxPart::Inside));
        release(&mut h, at);
    }
}

#[test]
fn an_auto_center_resize_snaps_the_handle_and_stays_there_while_the_pointer_is_still() {
    let mut h = harness(0.0, true);
    h.state_mut().snap_x = Some(250.0);
    let edge = screen(&h, Point::new(RECT.x1, 150.0));
    press(&mut h, edge);
    // The handle moves twice the pointer's travel: raw x1 would be 300 - 44 = 256.
    let at = edge + vec2(-22.0, 0.0);
    for pass in 0..4 {
        move_to(&mut h, at, Modifiers::NONE);
        let b = h.state().state;
        assert!((b.rect.x1 - 250.0).abs() < 1e-3 && (b.rect.x0 - RECT.x0).abs() < 1e-3, "pass {pass}: {b:?}");
    }
    assert!(view(&h).center != Point::new(200.0, 150.0), "the view slid with the frame");
    let (part, candidate) = h.state().told.get().expect("told");
    assert_eq!(part, BoxPart::Edge(1));
    assert!((candidate.rect.x1 - 256.0).abs() < 1e-3, "the candidate is the frame the pointer gives: {candidate:?}");
    release(&mut h, at);
    assert!((h.state().state.rect.x1 - 250.0).abs() < 1e-3);
}

#[test]
fn a_turn_with_a_still_pointer_gives_the_same_frame_every_pass() {
    let mut h = harness(0.0, true);
    h.state_mut().snap_x = Some(0.0);
    let from = screen(&h, Point::new(340.0, 260.0));
    press(&mut h, from);
    let at = screen(&h, Point::new(300.0, 290.0));
    move_to(&mut h, at, Modifiers::NONE);
    let (b, v) = (h.state().state, view(&h));
    assert!(b.angle != 0.0);
    for _ in 0..3 {
        move_to(&mut h, at, Modifiers::NONE);
        assert_eq!((h.state().state, view(&h)), (b, v));
    }
    release(&mut h, at);
}

#[test]
fn straightening_then_syncing_the_view_turns_it_by_minus_the_angle() {
    let h = harness(0.0, false);
    let fitted = CropBox { rect: RECT, angle: 0.0 }.fitted(kurbo::Size::new(640.0, 400.0), 7.0).expect("fits");
    let mut v = view(&h);
    CropFrame::sync_view(&mut v, &fitted);
    assert!((v.rotation + fitted.angle).abs() < 1e-9 && fitted.angle != 0.0, "{v:?} {fitted:?}");
}

#[test]
fn a_drawn_frame_turns_the_view_upright_and_centres_only_with_auto_center() {
    for auto_center in [false, true] {
        let mut h = harness(0.0, auto_center);
        {
            let host = h.state_mut();
            host.default_frame = true;
            host.view.rotation = 20.0;
        }
        h.run();
        let from = screen(&h, RECT.center());
        press(&mut h, from);
        move_to(&mut h, from + vec2(40.0, 30.0), Modifiers::NONE);
        release(&mut h, from + vec2(40.0, 30.0));
        let (b, v) = (h.state().state, view(&h));
        assert!(v.rotation.abs() < 1e-9, "upright at auto_center={auto_center}: {v:?}");
        assert_eq!((v.center - b.center()).hypot() < 1e-6, auto_center, "{v:?} {b:?}");
    }
}

#[test]
fn x_centres_the_view_on_the_swapped_frame_with_auto_center() {
    for auto_center in [false, true] {
        let mut h = harness(0.0, auto_center);
        h.state_mut().view.center += Vec2::new(30.0, 0.0);
        h.run();
        h.key_press(Key::X);
        h.run();
        let (b, v) = (h.state().state, view(&h));
        assert!(b.rect.width() < RECT.width(), "swapped {b:?}");
        assert_eq!((v.center - b.center()).hypot() < 1e-6, auto_center, "{v:?} {b:?}");
    }
}
