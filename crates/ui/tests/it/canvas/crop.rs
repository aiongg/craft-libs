//! `CropOverlay`: the frame's gestures and keys through real input.

use craft_behavior::geometry::{BoxPart, CropBox, SnapKind, SnapLine, SnapTarget, SnapTargets, crop_view, screen_step};
use craft_behavior::kurbo::{Point, Rect, Vec2 as DocVec};
use craft_behavior::{Axis, CanvasView, EditEvent};
use craft_ui::canvas::{Canvas, CropOverlay, CropShield, ShieldColor};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, Frames, focus, input, layout, pointer, trial};
use egui::{Color32, Event, Key, Modifiers, Pos2, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const FRAME: Rect = Rect::new(40.0, 30.0, 200.0, 140.0);

struct Shown {
    crop: Rc<RefCell<CropBox>>,
    applied: Rc<Cell<bool>>,
    discarded: Rc<Cell<bool>>,
    fixture: Fixture,
}

fn shown(configure: impl Fn(CropOverlay<'_>) -> CropOverlay<'_> + 'static) -> Shown {
    let crop = Rc::new(RefCell::new(CropBox::upright(FRAME)));
    let (applied, discarded) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(false)));
    let (c, a, d) = (Rc::clone(&crop), Rc::clone(&applied), Rc::clone(&discarded));
    let mut fixture = Fixture::new(move |ui| {
        let _ = Canvas::new("Document").show(ui, |ui, body| {
            let out = configure(CropOverlay::new("Crop", Bind::bound(&mut *c.borrow_mut()))).show_at(ui, body.rect);
            a.set(a.get() || out.apply);
            d.set(d.get() || out.discard);
        });
    });
    fixture.settle();
    Shown { crop, applied, discarded, fixture }
}

fn travel(fixture: &Fixture) -> f32 {
    4.0 * craft_ui::testing::input::max_click_dist(fixture.ctx())
}

#[trial]
fn a_corner_drag_resizes_and_keeps_the_opposite_corner() {
    let mut s = shown(|c| c);
    let corner = s.fixture.by_label("Crop corner, bottom right").expect("corner");
    let by = travel(&s.fixture);
    s.fixture.drag(&corner, Vec2::new(by, by));
    let rect = s.crop.borrow().rect;
    assert_eq!((rect.x0, rect.y0), (FRAME.x0, FRAME.y0));
    assert!(rect.x1 > FRAME.x1 && rect.y1 > FRAME.y1, "{rect:?}");
}

#[trial]
fn enter_applies_and_escape_discards_on_a_focused_part() {
    let mut s = shown(|c| c);
    let body = s.fixture.by_label("Whole frame").expect("whole frame");
    s.fixture.click(&body);
    s.fixture.key(Key::Enter, Modifiers::NONE);
    assert!(s.applied.get());
    s.fixture.key(Key::Escape, Modifiers::NONE);
    assert!(s.discarded.get());
}

#[trial]
fn an_arrow_moves_the_whole_frame_and_x_swaps_it() {
    let mut s = shown(|c| c);
    let body = s.fixture.by_label("Whole frame").expect("whole frame");
    s.fixture.click(&body);
    s.fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert!(s.crop.borrow().rect.x0 > FRAME.x0);
    let before = s.crop.borrow().rect;
    s.fixture.key(Key::X, Modifiers::NONE);
    let after = s.crop.borrow().rect;
    assert!((after.width() - before.height()).abs() < 1e-9 && (after.height() - before.width()).abs() < 1e-9, "{after:?}");
}

// Real input through the canvas: a frame per event batch, the view and the shown options held by the rig.

/// The room the rig shows the canvas in.
const ROOM: Vec2 = Vec2::new(400.0, 300.0);
/// The frame the drags start from, centred in the room's default view.
const RECT: Rect = Rect::new(100.0, 80.0, 300.0, 220.0);

#[derive(Clone)]
struct Setup {
    ratio: Option<f64>,
    default_frame: bool,
    image_mode: bool,
    auto_center: bool,
    bounds: Option<Rect>,
    snap_x: Option<f64>,
    snap_y: Option<f64>,
    straighten: bool,
    view: CanvasView,
}

impl Setup {
    fn new() -> Self {
        Setup {
            ratio: None,
            default_frame: false,
            image_mode: false,
            auto_center: false,
            bounds: None,
            snap_x: None,
            snap_y: None,
            straighten: false,
            view: CanvasView::new(egui::Rect::ZERO, Point::new(200.0, 150.0), 1.0),
        }
    }

    /// Image mode: the view shows `angle`'s frame upright.
    fn image(angle: f64, auto_center: bool) -> Self {
        let mut setup = Setup::new();
        setup.image_mode = true;
        setup.auto_center = auto_center;
        setup.view = crop_view(&CropBox::new(RECT, angle), &setup.view);
        setup
    }
}

/// What the overlay reported across the passes.
#[derive(Default)]
struct Seen {
    applied: bool,
    discarded: bool,
    straightened: Option<f64>,
    snapped: Vec<SnapLine>,
    picked: Option<BoxPart>,
    hovered: Option<BoxPart>,
    sizing: Option<(f64, f64)>,
    turning: Option<f64>,
    events: Vec<&'static str>,
}

struct Rig {
    frames: Frames,
    show: Box<dyn FnMut(&mut egui::Ui)>,
    crop: Rc<RefCell<CropBox>>,
    setup: Rc<RefCell<Setup>>,
    seen: Rc<RefCell<Seen>>,
}

impl Rig {
    fn new(crop: CropBox, setup: Setup) -> Self {
        let (crop, setup, seen) = (Rc::new(RefCell::new(crop)), Rc::new(RefCell::new(setup)), Rc::new(RefCell::new(Seen::default())));
        let (c, st, sn) = (Rc::clone(&crop), Rc::clone(&setup), Rc::clone(&seen));
        let show = move |ui: &mut egui::Ui| {
            let mut s = st.borrow().clone();
            s.view.rect = ui.available_rect_before_wrap();
            st.borrow_mut().view.rect = s.view.rect;
            let _ = Canvas::new("Document").view(s.view).show(ui, |ui, body| {
                let mut targets = SnapTargets::new();
                if let Some(x) = s.snap_x {
                    targets.add(SnapTarget::new(Axis::Y, x, SnapKind::Guide));
                }
                if let Some(y) = s.snap_y {
                    targets.add(SnapTarget::new(Axis::X, y, SnapKind::Guide));
                }
                let mut value = c.borrow_mut();
                let mut overlay = CropOverlay::new("Crop", Bind::bound(&mut *value)).snap(&targets);
                if let Some(ratio) = s.ratio {
                    overlay = overlay.ratio(ratio);
                }
                if let Some(bounds) = s.bounds {
                    overlay = overlay.bounds(bounds);
                }
                if s.default_frame {
                    overlay = overlay.default_frame();
                }
                if s.image_mode {
                    overlay = overlay.image_mode();
                }
                if s.auto_center {
                    overlay = overlay.auto_center();
                }
                if s.straighten {
                    overlay = overlay.straighten();
                }
                let out = overlay.show_at(ui, body.rect);
                let mut seen = sn.borrow_mut();
                if let Some(view) = out.view {
                    st.borrow_mut().view = view;
                }
                let kind = match &out.edit.event {
                    Some(EditEvent::Begin { .. }) => Some("begin"),
                    Some(EditEvent::Update { .. }) => Some("update"),
                    Some(EditEvent::Commit { .. }) => Some("commit"),
                    Some(EditEvent::Cancel { .. }) => Some("cancel"),
                    Some(EditEvent::Set { .. }) => Some("set"),
                    Some(_) => Some("other"),
                    None => None,
                };
                if let Some(kind) = kind
                    && seen.events.last() != Some(&kind)
                {
                    seen.events.push(kind);
                }
                seen.applied |= out.apply;
                seen.discarded |= out.discard;
                seen.straightened = out.straightened.or(seen.straightened);
                seen.snapped = out.snapped;
                seen.picked = out.picked;
                seen.hovered = out.hovered;
                seen.sizing = out.sizing;
                seen.turning = out.turning;
            });
        };
        let mut rig = Rig { frames: Frames::new(), show: Box::new(show), crop, setup, seen };
        for _ in 0..4 {
            rig.frame(Vec::new());
        }
        rig
    }

    fn upright() -> Self {
        Rig::new(CropBox::upright(RECT), Setup::new())
    }

    fn with(setup: impl FnOnce(&mut Setup)) -> Self {
        let mut s = Setup::new();
        setup(&mut s);
        Rig::new(CropBox::upright(RECT), s)
    }

    fn frame(&mut self, events: Vec<Event>) {
        let room = egui::Rect::from_min_size(Pos2::ZERO, ROOM);
        let show = &mut self.show;
        self.frames.frame(events, |ui| layout::in_region(ui, room, |ui| show(ui)));
    }

    fn view(&self) -> CanvasView {
        self.setup.borrow().view
    }

    /// The screen point of document point `(x, y)`.
    fn at(&self, x: f64, y: f64) -> Pos2 {
        self.view().to_screen(Point::new(x, y))
    }

    fn crop(&self) -> CropBox {
        *self.crop.borrow()
    }

    fn rect(&self) -> Rect {
        self.crop().rect
    }

    fn events(&self) -> Vec<&'static str> {
        self.seen.borrow().events.clone()
    }

    fn press(&mut self, at: Pos2) {
        self.frame(vec![pointer::moved(at)]);
        self.frame(vec![pointer::press(at)]);
    }

    fn move_to(&mut self, to: Pos2) {
        self.frame(vec![pointer::moved(to)]);
    }

    fn release(&mut self, at: Pos2) {
        self.frame(vec![pointer::release(at)]);
        self.frame(Vec::new());
    }

    /// Press at `from`, move through the middle to `to`, release, with `mods` held.
    fn drag(&mut self, from: Pos2, to: Pos2, mods: Modifiers) {
        self.frames.hold(mods);
        self.press(from);
        self.move_to(from.lerp(to, 0.5));
        self.move_to(to);
        self.release(to);
        self.frames.hold(Modifiers::NONE);
    }

    /// Press at `from` and move to `to` with the button held.
    fn hold_at(&mut self, from: Pos2, to: Pos2) {
        self.press(from);
        self.move_to(from.lerp(to, 0.5));
        self.move_to(to);
        self.frame(Vec::new());
    }

    fn click(&mut self, at: Pos2) {
        self.frame(vec![pointer::moved(at)]);
        self.frame(vec![pointer::press(at)]);
        self.frame(vec![pointer::release(at)]);
        self.frame(Vec::new());
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![key_event(key, true, modifiers), key_event(key, false, modifiers)]);
        self.frame(Vec::new());
    }

    fn hover(&mut self, at: Pos2) {
        self.move_to(at);
        self.frame(Vec::new());
    }

    fn cursor(&self) -> egui::CursorIcon {
        self.frames.cursor_icon()
    }

    fn texts(&self) -> Vec<String> {
        self.frames.texts().iter().map(|text| text.text.clone()).collect()
    }

    fn painted(&self, needle: &str) -> bool {
        self.texts().iter().any(|text| text.contains(needle))
    }
}

fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    if pressed { input::key_press(key, modifiers) } else { focus::key_release(key, modifiers) }
}

fn near(a: &CropBox, b: &CropBox) -> bool {
    a.corners().iter().zip(b.corners()).all(|(p, q)| p.distance(q) < 1e-6) && (a.angle - b.angle).abs() < 1e-6
}

fn screen_quad(rig: &Rig) -> [Pos2; 4] {
    let view = rig.view();
    rig.crop().corners().map(|p| view.to_screen(p))
}

fn same_quad(a: [Pos2; 4], b: [Pos2; 4]) -> bool {
    a.iter().zip(b).all(|(p, q)| p.distance(q) < 1e-2)
}

#[trial]
fn a_drag_inside_moves_the_frame_and_a_turned_frame_moves_on_the_page() {
    let mut rig = Rig::upright();
    let from = rig.at(150.0, 110.0);
    rig.drag(from, from + Vec2::new(30.0, 20.0), Modifiers::NONE);
    assert_eq!(rig.crop(), CropBox::upright(RECT + DocVec::new(30.0, 20.0)));
    let events = rig.events();
    assert_eq!((events.first(), events.last()), (Some(&"begin"), Some(&"commit")), "{events:?}");
    assert_eq!(rig.seen.borrow().picked, Some(BoxPart::Inside));

    let turned = CropBox::new(RECT, 30.0);
    let mut rig = Rig::new(turned, Setup::new());
    let from = rig.at(200.0, 150.0);
    rig.drag(from, from + Vec2::new(-25.0, 10.0), Modifiers::NONE);
    assert!(near(&rig.crop(), &turned.moved(DocVec::new(-25.0, 10.0))), "{:?}", rig.crop());
}

#[trial]
fn a_drag_inside_moves_the_frame_and_a_drag_inside_the_default_frame_draws() {
    let mut s = shown(|c| c);
    let body = s.fixture.by_label("Whole frame").expect("whole frame");
    let by = travel(&s.fixture);
    s.fixture.drag(&body, Vec2::new(by, 0.0));
    let moved = s.crop.borrow().rect;
    assert!(moved.x0 > FRAME.x0 && (moved.width() - FRAME.width()).abs() < 1e-9, "{moved:?}");

    let mut s = shown(|c| c.default_frame());
    let body = s.fixture.by_label("Whole frame").expect("whole frame");
    s.fixture.drag(&body, Vec2::new(by, by));
    let drawn = s.crop.borrow().rect;
    assert!(drawn.width() < FRAME.width() && drawn.width() > 0.0, "{drawn:?}");
}

#[trial]
fn an_edge_takes_the_pointer_near_it_in_a_turned_flipped_view() {
    let mut rig = Rig::with(|s| {
        s.view.rotation = 30.0;
        s.view.flip_x = true;
    });
    let edge = rig.at(RECT.x1, RECT.center().y);
    let out = rig.view().map_vec(DocVec::new(1.0, 0.0)).normalized();
    rig.hover(edge + out * 3.0);
    assert_eq!(rig.seen.borrow().hovered, Some(BoxPart::Edge(1)));
    rig.hover(edge + out * 40.0);
    assert_eq!(rig.seen.borrow().hovered, Some(BoxPart::Outside));
}

#[trial]
fn a_corner_drag_resizes_along_the_frame_axes_in_a_turned_view() {
    let b = CropBox::new(RECT, 20.0);
    let mut rig = Rig::new(b, {
        let mut s = Setup::new();
        s.view.rotation = 40.0;
        s.view.flip_x = true;
        s
    });
    let view = rig.view();
    let corner = view.to_screen(b.corners()[2]);
    let to = corner + Vec2::new(14.0, -9.0);
    let want = b.resized(BoxPart::Corner(2), view.to_doc(to) - view.to_doc(corner), None, false);
    rig.drag(corner, to, Modifiers::NONE);
    assert!(near(&rig.crop(), &want), "{:?} vs {want:?}", rig.crop());
    assert_eq!(rig.seen.borrow().picked, Some(BoxPart::Corner(2)));

    // Shift keeps the proportions; Alt resizes about the centre.
    let mut rig = Rig::upright();
    let (from, to) = (rig.at(300.0, 220.0), rig.at(340.0, 230.0));
    rig.drag(from, to, Modifiers::SHIFT);
    let r = rig.rect();
    assert!((r.width() / r.height() - 200.0 / 140.0).abs() < 1e-9, "{r:?}");
    let mut rig = Rig::upright();
    rig.drag(from, to, Modifiers::ALT);
    let c = rig.rect().center();
    assert!((c.x - 200.0).abs() < 1e-9 && (c.y - 150.0).abs() < 1e-9, "{:?}", rig.rect());
}

#[trial]
fn a_ratio_is_held_by_every_gesture() {
    let mut rig = Rig::with(|s| s.ratio = Some(1.0));
    let (from, to) = (rig.at(300.0, 150.0), rig.at(340.0, 150.0));
    rig.drag(from, to, Modifiers::NONE);
    let r = rig.rect();
    assert!((r.width() - r.height()).abs() < 1e-9 && (r.width() - 240.0).abs() < 1e-9, "{r:?}");

    let mut rig = Rig::with(|s| {
        s.ratio = Some(2.0);
        s.default_frame = true;
    });
    let (from, to) = (rig.at(120.0, 100.0), rig.at(180.0, 110.0));
    rig.drag(from, to, Modifiers::NONE);
    let r = rig.rect();
    assert!((r.width() / r.height() - 2.0).abs() < 1e-9, "{r:?}");
}

#[trial]
fn a_held_ratio_is_kept_by_a_corner_drag() {
    let ratio = 2.0;
    let mut s = shown(move |c| c.ratio(ratio));
    let corner = s.fixture.by_label("Crop corner, bottom right").expect("corner");
    let by = travel(&s.fixture);
    s.fixture.drag(&corner, Vec2::new(by, 0.0));
    let rect = s.crop.borrow().rect;
    assert!((rect.width() / rect.height() - ratio).abs() < 1e-6, "{rect:?}");
}

#[trial]
fn a_drag_outside_turns_the_frame_and_shift_snaps_the_turn() {
    let (from_doc, to_doc) = ((340.0, 260.0), (300.0, 290.0));
    let mut rig = Rig::upright();
    let (from, to) = (rig.at(from_doc.0, from_doc.1), rig.at(to_doc.0, to_doc.1));
    rig.drag(from, to, Modifiers::NONE);
    let angle = rig.crop().angle;
    assert!(angle.abs() > 1.0, "{angle}");
    assert_eq!(rig.rect(), RECT, "turning keeps the rect");

    let mut rig = Rig::upright();
    rig.drag(from, to, Modifiers::SHIFT);
    let angle = rig.crop().angle;
    assert!((angle / 15.0 - (angle / 15.0).round()).abs() < 1e-9 && angle.abs() > 1.0, "{angle}");
}

#[trial]
fn image_mode_a_drag_outside_turns_the_view_and_shift_snaps() {
    let mut rig = Rig::new(CropBox::upright(RECT), Setup::image(0.0, false));
    let c = rig.at(200.0, 150.0);
    let start = c + Vec2::new(150.0, 0.0);
    let before = screen_quad(&rig);
    let swept = |deg: f32| c + Vec2::new(150.0 * deg.to_radians().cos(), 150.0 * deg.to_radians().sin());
    rig.press(start);
    rig.move_to(swept(20.0));
    rig.frame(Vec::new());
    assert!((rig.crop().angle + 20.0).abs() < 1e-3, "{:?}", rig.crop());
    assert!((rig.view().rotation - 20.0).abs() < 1e-3, "{:?}", rig.view());
    assert!(same_quad(before, screen_quad(&rig)), "the frame stays upright and centred");
    rig.frames.hold(Modifiers::SHIFT);
    rig.move_to(swept(22.0));
    rig.frame(Vec::new());
    assert_eq!((rig.crop().angle, rig.view().rotation), (-15.0, 15.0));
    // The view the drag works from is the press view: coming back gives the start exactly.
    rig.frames.hold(Modifiers::NONE);
    rig.move_to(start);
    rig.frame(Vec::new());
    assert!(rig.crop().angle.abs() < 1e-6 && rig.view().rotation.abs() < 1e-6);
    rig.release(start);
}

#[trial]
fn a_drag_inside_the_default_frame_draws_a_new_frame_with_shift_alt_and_space() {
    let drawing = |mods: Modifiers| {
        let mut rig = Rig::with(|s| s.default_frame = true);
        let (from, to) = (rig.at(150.0, 100.0), rig.at(190.0, 120.0));
        rig.drag(from, to, mods);
        rig.rect()
    };
    assert_eq!(drawing(Modifiers::NONE), Rect::new(150.0, 100.0, 190.0, 120.0));
    assert_eq!(drawing(Modifiers::SHIFT), Rect::new(150.0, 100.0, 190.0, 140.0));
    assert_eq!(drawing(Modifiers::ALT), Rect::new(110.0, 80.0, 190.0, 120.0));

    // Space moves the frame being drawn without resizing it; sizing resumes from the moved anchor.
    let mut rig = Rig::with(|s| s.default_frame = true);
    let (from, to) = (rig.at(150.0, 100.0), rig.at(190.0, 120.0));
    rig.hold_at(from, to);
    rig.frame(vec![key_event(Key::Space, true, Modifiers::NONE)]);
    rig.move_to(to + Vec2::new(20.0, 10.0));
    assert_eq!(rig.rect(), Rect::new(170.0, 110.0, 210.0, 130.0), "same size, moved");
    rig.frame(vec![key_event(Key::Space, false, Modifiers::NONE)]);
    rig.move_to(to + Vec2::new(40.0, 20.0));
    rig.release(to + Vec2::new(40.0, 20.0));
    assert_eq!(rig.rect(), Rect::new(170.0, 110.0, 230.0, 140.0));
}

#[trial]
fn a_frame_under_one_unit_is_cancelled_and_the_previous_one_returns() {
    let mut rig = Rig::with(|s| s.default_frame = true);
    let from = rig.at(150.0, 100.0);
    rig.hold_at(from, from + Vec2::new(5.0, 5.0));
    assert_ne!(rig.crop(), CropBox::upright(RECT), "the new frame shows while it is dragged");
    let back = from + Vec2::new(0.3, 0.2);
    rig.move_to(back);
    rig.release(back);
    assert_eq!(rig.crop(), CropBox::upright(RECT));
    assert_eq!(rig.events().last(), Some(&"cancel"), "{:?}", rig.events());

    // A resize that collapses a side does the same.
    let mut rig = Rig::upright();
    let (edge, to) = (rig.at(300.0, 150.0), rig.at(100.0, 150.0));
    rig.drag(edge, to, Modifiers::NONE);
    assert_eq!(rig.crop(), CropBox::upright(RECT));
    assert_eq!(rig.events().last(), Some(&"cancel"));
}

#[trial]
fn a_side_under_one_unit_cancels_the_drag() {
    let mut s = shown(|c| c);
    let corner = s.fixture.by_label("Crop corner, top left").expect("corner");
    s.fixture.drag(&corner, Vec2::new(FRAME.width() as f32 + 0.2, 0.0));
    assert_eq!(s.crop.borrow().rect, FRAME, "the frame returns to what it was");
}

#[trial]
fn a_press_released_without_moving_changes_nothing() {
    let mut rig = Rig::upright();
    let at = rig.at(150.0, 110.0);
    rig.click(at);
    assert_eq!(rig.crop(), CropBox::upright(RECT));
    assert!(!rig.events().contains(&"commit"), "{:?}", rig.events());
}

#[trial]
fn a_quick_drag_in_one_frame_starts_at_the_press() {
    let mut rig = Rig::upright();
    let from = rig.at(300.0, 220.0);
    let to = from + Vec2::new(40.0, 10.0);
    rig.frame(vec![pointer::moved(from)]);
    rig.frame(vec![pointer::press(from), pointer::moved(from + Vec2::new(20.0, 5.0)), pointer::moved(to)]);
    rig.frame(vec![pointer::release(to)]);
    rig.frame(Vec::new());
    assert_eq!(rig.rect(), Rect::new(100.0, 80.0, 340.0, 230.0));
    let events = rig.events();
    assert_eq!((events.first(), events.last()), (Some(&"begin"), Some(&"commit")), "{events:?}");
}

#[trial]
fn double_click_inside_applies_but_not_on_a_handle_or_the_default_frame() {
    let double = |rig: &mut Rig, at: Pos2| {
        // Two clicks a few frames apart, inside the double-click window.
        rig.frame(vec![pointer::moved(at)]);
        rig.frame(vec![pointer::press(at)]);
        rig.frame(vec![pointer::release(at)]);
        rig.frame(vec![pointer::press(at)]);
        rig.frame(vec![pointer::release(at)]);
        rig.frame(Vec::new());
    };
    let mut rig = Rig::upright();
    let at = rig.at(150.0, 110.0);
    double(&mut rig, at);
    assert!(rig.seen.borrow().applied);
    assert_eq!(rig.crop(), CropBox::upright(RECT), "clicks don't move the frame");

    let mut rig = Rig::upright();
    let at = rig.at(300.0, 220.0);
    double(&mut rig, at);
    assert!(!rig.seen.borrow().applied, "on a handle");

    let mut rig = Rig::with(|s| s.default_frame = true);
    let at = rig.at(150.0, 110.0);
    double(&mut rig, at);
    assert!(!rig.seen.borrow().applied, "the untouched default frame");
}

#[trial]
fn escape_undoes_a_running_drag_then_discards() {
    let mut rig = Rig::upright();
    let from = rig.at(150.0, 110.0);
    rig.hold_at(from, from + Vec2::new(30.0, 20.0));
    assert_ne!(rig.crop(), CropBox::upright(RECT));
    rig.key(Key::Escape, Modifiers::NONE);
    assert_eq!(rig.crop(), CropBox::upright(RECT));
    assert!(!rig.seen.borrow().discarded, "the first Escape only cancels the drag");
    rig.release(from + Vec2::new(30.0, 20.0));
    rig.key(Key::Escape, Modifiers::NONE);
    assert!(rig.seen.borrow().discarded);
}

#[trial]
fn page_keys_pick_every_part_and_the_arrows_resize_the_picked_one() {
    let mut rig = Rig::upright();
    let at = rig.at(150.0, 110.0);
    rig.click(at);
    let mut seen = vec![rig.seen.borrow().picked];
    for _ in 0..8 {
        rig.key(Key::PageDown, Modifiers::NONE);
        seen.push(rig.seen.borrow().picked);
    }
    let want: Vec<_> = std::iter::once(BoxPart::Inside).chain((0..4).map(BoxPart::Corner)).chain((0..4).map(BoxPart::Edge)).map(Some).collect();
    assert_eq!(seen, want);

    let mut rig = Rig::upright();
    let at = rig.at(150.0, 110.0);
    rig.click(at);
    rig.key(Key::PageDown, Modifiers::NONE);
    rig.key(Key::ArrowRight, Modifiers::SHIFT);
    let r = rig.rect();
    assert_eq!((r.x0, r.y0, r.x1, r.y1), (RECT.x0 + 10.0, RECT.y0, RECT.x1, RECT.y1), "the top left corner moved ten");
}

#[trial]
fn the_readout_shows_the_crop_size_or_the_angle_only_during_the_drag() {
    let mut rig = Rig::upright();
    let from = rig.at(300.0, 220.0);
    rig.hold_at(from, from + Vec2::new(20.0, 10.0));
    assert!(rig.painted("W:") && rig.painted("220") && rig.painted("150"), "{:?}", rig.texts());
    assert_eq!(rig.seen.borrow().sizing, Some((220.0, 150.0)));
    rig.release(from + Vec2::new(20.0, 10.0));
    assert!(!rig.painted("W:"), "only during the drag");

    let mut rig = Rig::upright();
    let from = rig.at(340.0, 260.0);
    rig.hold_at(from, from + Vec2::new(-30.0, 30.0));
    assert!(rig.painted("Angle:") && rig.seen.borrow().turning.is_some(), "{:?}", rig.texts());

    // Moving shows none.
    let mut rig = Rig::upright();
    let from = rig.at(150.0, 110.0);
    rig.hold_at(from, from + Vec2::new(20.0, 5.0));
    assert!(!rig.painted("W:") && !rig.painted("Angle:"), "{:?}", rig.texts());
}

#[trial]
fn the_readout_shows_in_image_mode_too() {
    let mut rig = Rig::new(CropBox::upright(RECT), Setup::image(0.0, false));
    let from = rig.at(300.0, 220.0);
    rig.hold_at(from, from + Vec2::new(20.0, 10.0));
    assert!(rig.painted("W:"), "{:?}", rig.texts());
}

#[trial]
fn the_turn_angle_never_reads_minus_zero() {
    let mut rig = Rig::upright();
    let from = rig.at(340.0, 260.0);
    rig.hold_at(from, from + Vec2::new(0.0, 8.0));
    rig.move_to(from + Vec2::new(0.0, -0.01));
    rig.frame(Vec::new());
    assert!(rig.painted("Angle:"), "{:?}", rig.texts());
    assert!(rig.texts().iter().all(|text| !text.contains("-0.0")), "{:?}", rig.texts());
}

#[trial]
fn the_pointer_follows_the_part_under_it() {
    let mut rig = Rig::upright();
    for ((x, y), want) in [
        ((300.0, 150.0), egui::CursorIcon::ResizeHorizontal),
        ((200.0, 80.0), egui::CursorIcon::ResizeVertical),
        ((300.0, 220.0), egui::CursorIcon::ResizeNwSe),
        ((300.0, 80.0), egui::CursorIcon::ResizeNeSw),
        ((200.0, 150.0), egui::CursorIcon::Move),
        ((350.0, 270.0), egui::CursorIcon::None),
    ] {
        let at = rig.at(x, y);
        rig.hover(at);
        assert_eq!(rig.cursor(), want, "({x}, {y})");
    }
    // Inside the untouched default frame a drag draws.
    let mut rig = Rig::with(|s| s.default_frame = true);
    let at = rig.at(200.0, 150.0);
    rig.hover(at);
    assert_eq!(rig.cursor(), egui::CursorIcon::Crosshair);
    let from = rig.at(150.0, 100.0);
    rig.hold_at(from, from + Vec2::new(20.0, 10.0));
    assert_eq!(rig.cursor(), egui::CursorIcon::Crosshair, "while drawing");
}

#[trial]
fn the_resize_arrow_follows_the_frame_angle_the_view_rotation_and_the_flip() {
    let edge = |angle: f64, rotation: f64, flip: bool| {
        let b = CropBox::new(RECT, angle);
        let mut setup = Setup::new();
        setup.view.rotation = rotation;
        setup.view.flip_x = flip;
        let mut rig = Rig::new(b, setup);
        let right = rig.view().to_screen(b.corners()[1].midpoint(b.corners()[2]));
        rig.hover(right);
        rig.cursor()
    };
    assert_eq!(edge(0.0, 0.0, false), egui::CursorIcon::ResizeHorizontal);
    assert_eq!(edge(90.0, 0.0, false), egui::CursorIcon::ResizeVertical);
    assert_eq!(edge(45.0, 0.0, false), egui::CursorIcon::ResizeNwSe);
    assert_eq!(edge(0.0, 90.0, false), egui::CursorIcon::ResizeVertical, "the view's rotation adds to the frame's");
    assert_eq!(edge(45.0, 0.0, true), egui::CursorIcon::ResizeNeSw, "a flipped view mirrors the arrow");
}

#[trial]
fn degenerate_frames_never_panic() {
    for rect in [Rect::new(100.0, 100.0, 100.0, 150.0), Rect::new(100.0, 100.0, 100.0 + 1e-6, 150.0), Rect::new(f64::NAN, 0.0, 10.0, 10.0)] {
        let mut rig = Rig::new(CropBox::upright(rect), Setup::new());
        let from = rig.at(105.0, 120.0);
        rig.drag(from, from + Vec2::new(5.0, 5.0), Modifiers::NONE);
        rig.key(Key::ArrowLeft, Modifiers::NONE);
    }
}

#[trial]
fn bounds_keep_the_frame_inside_and_the_snap_moves_the_dragged_handle() {
    let mut rig = Rig::with(|s| s.bounds = Some(Rect::new(0.0, 0.0, 400.0, 300.0)));
    let from = rig.at(300.0, 220.0);
    rig.drag(from, from + Vec2::new(300.0, 200.0), Modifiers::NONE);
    assert_eq!(rig.rect(), Rect::new(100.0, 80.0, 400.0, 300.0));
    let from = rig.at(200.0, 150.0);
    rig.drag(from, from + Vec2::new(-400.0, -400.0), Modifiers::NONE);
    assert_eq!(rig.rect(), Rect::new(0.0, 0.0, 300.0, 220.0));
    rig.key(Key::ArrowLeft, Modifiers::SHIFT);
    assert_eq!(rig.rect().x0, 0.0, "keys stay inside too");

    let mut rig = Rig::with(|s| {
        s.snap_x = Some(310.0);
        s.snap_y = Some(230.0);
    });
    let from = rig.at(300.0, 220.0);
    rig.drag(from, from + Vec2::new(13.0, 7.0), Modifiers::NONE);
    assert_eq!(rig.rect(), Rect::new(100.0, 80.0, 310.0, 230.0));
}

// Straighten.

#[trial]
fn a_straighten_drag_reports_the_angle_and_sends_no_edit() {
    let mut rig = Rig::with(|s| s.straighten = true);
    let from = rig.at(120.0, 100.0);
    rig.hold_at(from, from + Vec2::new(100.0, -10.0));
    assert!(rig.painted("Angle:"), "the angle shows beside the pointer while the line is drawn: {:?}", rig.texts());
    rig.release(from + Vec2::new(100.0, -10.0));
    let want = CropBox::level_angle(Point::new(120.0, 100.0), Point::new(220.0, 90.0));
    assert_eq!(rig.seen.borrow().straightened, want);
    assert!(want.is_some());
    assert_eq!(rig.crop(), CropBox::upright(RECT), "the frame is left alone");
    assert!(rig.events().is_empty(), "{:?}", rig.events());
}

#[trial]
fn the_end_points_of_the_straighten_line_take_the_snap() {
    let mut rig = Rig::with(|s| {
        s.straighten = true;
        s.snap_x = Some(120.0);
        s.snap_y = Some(100.0);
    });
    let from = rig.at(123.0, 97.0);
    rig.drag(from, from + Vec2::new(100.0, 12.0), Modifiers::NONE);
    // The start snaps to (120, 100); the end (223, 109) has no target in reach.
    let want = CropBox::level_angle(Point::new(120.0, 100.0), Point::new(223.0, 109.0));
    assert_eq!(rig.seen.borrow().straightened, want);
}

// Image mode.

#[trial]
fn image_mode_a_drag_inside_moves_the_frame_and_the_view_against_the_pointer() {
    let mut rig = Rig::new(CropBox::new(RECT, 30.0), Setup::image(30.0, false));
    let (before, v0) = (screen_quad(&rig), rig.view());
    let from = rig.at(200.0, 150.0);
    rig.drag(from, from + Vec2::new(30.0, 20.0), Modifiers::NONE);
    let d = v0.unmap_vec(Vec2::new(30.0, 20.0));
    let moved = CropBox::new(RECT, 30.0).moved(-d);
    assert!(near(&rig.crop(), &moved), "{:?}", rig.crop());
    let v = rig.view();
    assert!((v.center - (v0.center - d)).hypot() < 1e-3, "{:?}", v.center);
    assert_eq!(v.rotation, v0.rotation);
    assert!(same_quad(before, screen_quad(&rig)), "the frame stays put on screen");
}

#[trial]
fn image_mode_auto_center_keeps_the_frame_centre_on_screen_and_the_handle_under_the_pointer() {
    for angle in [0.0, 30.0] {
        let b0 = CropBox::new(RECT, angle);
        let mut rig = Rig::new(b0, Setup::image(angle, true));
        let view0 = rig.view();
        let centre = view0.to_screen(b0.center());
        let corner = view0.to_screen(b0.corners()[2]);
        let to = corner + Vec2::new(40.0, 25.0);
        rig.press(corner);
        rig.move_to(to);
        rig.frame(Vec::new());
        let (b, view) = (rig.crop(), rig.view());
        assert!((b.rect.width() - RECT.width()).abs() > 1.0, "resized");
        assert!(view.to_screen(b.center()).distance(centre) < 1e-2, "centre kept at {angle}");
        assert!(view.to_screen(b.corners()[2]).distance(to) < 1e-1, "handle follows at {angle}");
        assert_eq!(view.rotation, -angle);
        assert!(b.corners()[0].distance(b0.corners()[0]) < 1e-6, "the opposite corner stays put on the image");
        rig.release(to);
    }
}

#[trial]
fn image_mode_escape_restores_the_frame_and_the_view() {
    let mut rig = Rig::new(CropBox::upright(RECT), Setup::image(0.0, false));
    let (b0, v0) = (rig.crop(), rig.view());
    let c = rig.at(200.0, 150.0);
    rig.press(c + Vec2::new(150.0, 0.0));
    rig.move_to(c + Vec2::new(100.0, 100.0));
    rig.frame(Vec::new());
    assert_ne!(rig.view(), v0);
    rig.key(Key::Escape, Modifiers::NONE);
    assert_eq!((rig.crop(), rig.view()), (b0, v0));
    assert_eq!(rig.events().last(), Some(&"cancel"));
}

#[trial]
fn image_mode_arrows_move_the_image_under_the_frame() {
    let mut rig = Rig::new(CropBox::new(RECT, 30.0), Setup::image(30.0, false));
    let (before, v0) = (screen_quad(&rig), rig.view());
    let at = rig.at(200.0, 150.0);
    rig.click(at);
    rig.key(Key::ArrowRight, Modifiers::NONE);
    let d = screen_step(&v0, DocVec::new(1.0, 0.0));
    assert!(same_quad(before, screen_quad(&rig)), "the frame stays put on screen");
    assert!((rig.view().center - (v0.center - d)).hypot() < 1e-6, "{:?}", rig.view().center);
}

#[trial]
fn image_mode_a_move_snaps_and_stays_snapped_while_the_pointer_is_still() {
    for auto_center in [false, true] {
        let mut rig = Rig::new(CropBox::upright(RECT), {
            let mut s = Setup::image(0.0, auto_center);
            s.snap_x = Some(0.0);
            s
        });
        let from = rig.at(200.0, 150.0);
        rig.press(from);
        // The frame moves against the pointer: raw, its left edge would be at x = -3; the image follows the frame.
        let at = from + Vec2::new(103.0, 20.0);
        for pass in 0..4 {
            rig.move_to(at);
            let r = rig.rect();
            assert!(r.x0.abs() < 1e-3, "pass {pass}, auto_center={auto_center}: {r:?}");
            assert!((r.y0 - (RECT.y0 - 20.0)).abs() < 1e-3, "pass {pass}: {r:?}");
        }
        assert!(!rig.seen.borrow().snapped.is_empty(), "the snap line is reported");
        rig.release(at);
    }
}

#[trial]
fn image_mode_an_auto_center_resize_snaps_the_handle_and_stays_there_while_the_pointer_is_still() {
    let mut rig = Rig::new(CropBox::upright(RECT), {
        let mut s = Setup::image(0.0, true);
        s.snap_x = Some(250.0);
        s
    });
    let centre0 = rig.view().center;
    let edge = rig.at(RECT.x1, 150.0);
    rig.press(edge);
    // The handle moves twice the pointer's travel: raw x1 would be 300 - 44 = 256.
    let at = edge + Vec2::new(-22.0, 0.0);
    for pass in 0..4 {
        rig.move_to(at);
        let r = rig.rect();
        assert!((r.x1 - 250.0).abs() < 1e-3 && (r.x0 - RECT.x0).abs() < 1e-3, "pass {pass}: {r:?}");
    }
    assert!(rig.view().center != centre0, "the view slid with the frame");
    rig.release(at);
    assert!((rig.rect().x1 - 250.0).abs() < 1e-3);
}

#[trial]
fn image_mode_a_turn_with_a_still_pointer_gives_the_same_frame_every_pass() {
    let mut rig = Rig::new(CropBox::upright(RECT), {
        let mut s = Setup::image(0.0, true);
        s.snap_x = Some(0.0);
        s
    });
    let from = rig.at(340.0, 260.0);
    rig.press(from);
    let at = rig.at(300.0, 290.0);
    rig.move_to(at);
    let (b, v) = (rig.crop(), rig.view());
    assert!(b.angle != 0.0);
    for _ in 0..3 {
        rig.move_to(at);
        assert_eq!((rig.crop(), rig.view()), (b, v));
    }
    rig.release(at);
}

#[trial]
fn image_mode_a_drawn_frame_turns_the_view_upright_and_centres_only_with_auto_center() {
    for auto_center in [false, true] {
        let mut setup = Setup::image(0.0, auto_center);
        setup.default_frame = true;
        setup.view.rotation = 20.0;
        let mut rig = Rig::new(CropBox::upright(RECT), setup);
        let from = rig.at(200.0, 150.0);
        rig.drag(from, from + Vec2::new(40.0, 30.0), Modifiers::NONE);
        let (b, v) = (rig.crop(), rig.view());
        assert!(v.rotation.abs() < 1e-9, "upright at auto_center={auto_center}: {v:?}");
        assert_eq!((v.center - b.center()).hypot() < 1e-6, auto_center, "{v:?} {b:?}");
    }
}

#[trial]
fn image_mode_x_centres_the_view_on_the_swapped_frame_with_auto_center() {
    for auto_center in [false, true] {
        let mut setup = Setup::image(0.0, auto_center);
        setup.view.center += DocVec::new(30.0, 0.0);
        let mut rig = Rig::new(CropBox::upright(RECT), setup);
        let at = rig.at(200.0, 150.0);
        rig.click(at);
        rig.key(Key::X, Modifiers::NONE);
        let (b, v) = (rig.crop(), rig.view());
        assert!(b.rect.width() < RECT.width(), "swapped {b:?}");
        assert_eq!((v.center - b.center()).hypot() < 1e-6, auto_center, "{v:?} {b:?}");
    }
}

// The shield and the saved settings.

const CANVAS: Color32 = Color32::from_rgb(40, 40, 40);

/// The default shield with `change` applied: the type is `#[non_exhaustive]`.
fn shield(change: impl FnOnce(&mut CropShield)) -> CropShield {
    let mut s = CropShield::default();
    change(&mut s);
    s
}

#[trial]
fn the_shield_dims_outside_the_frame_lighter_while_dragging_and_not_at_all_when_off() {
    let defaults = CropShield::default();
    let resting = defaults.fill(CANVAS, false).expect("shown");
    // Auto Adjust lightens it while dragging; without it the opacity holds.
    let dragging = defaults.fill(CANVAS, true).expect("shown");
    assert!(dragging.a() < resting.a(), "dragging lightens the shield");
    let plain = shield(|s| s.auto_adjust = false);
    assert_eq!(plain.fill(CANVAS, true), plain.fill(CANVAS, false));
    // Disabled or fully transparent draws nothing.
    assert_eq!(shield(|s| s.enabled = false).fill(CANVAS, false), None);
    assert_eq!(shield(|s| s.opacity = 0.0).fill(CANVAS, false), None);
    // A lower opacity is lighter, in both states.
    let half = shield(|s| s.opacity = 50.0);
    let (half_resting, half_dragging) = (half.fill(CANVAS, false).expect("shown"), half.fill(CANVAS, true).expect("shown"));
    assert!(half_resting.a() < resting.a() && half_dragging.a() < dragging.a(), "a lower opacity is lighter");
}

#[trial]
fn the_shield_colour_comes_from_the_canvas_or_the_custom_choice() {
    let custom = shield(|s| {
        s.color = ShieldColor::Custom;
        s.custom_color = [200, 10, 30];
        s.opacity = 100.0;
    });
    assert_eq!(custom.fill(CANVAS, false), Some(Color32::from_rgb(200, 10, 30)));
    let matched = shield(|s| s.opacity = 100.0);
    assert_eq!(matched.fill(Color32::from_rgb(184, 184, 184), false), Some(Color32::from_rgb(184, 184, 184)));
}

#[trial]
fn a_hidden_cropped_area_is_the_opaque_canvas_whatever_the_shield() {
    for enabled in [true, false] {
        let s = shield(|s| {
            s.show_cropped_area = false;
            s.enabled = enabled;
            s.color = ShieldColor::Custom;
            s.custom_color = [255, 0, 0];
            s.opacity = 10.0;
        });
        assert_eq!(s.fill(CANVAS, false), Some(CANVAS));
        assert_eq!(s.fill(CANVAS, true), Some(CANVAS));
    }
    // Classic mode keeps the cropped area shown whatever the stored choice.
    let classic = shield(|s| {
        s.show_cropped_area = false;
        s.classic_mode = true;
        s.opacity = 100.0;
    });
    assert_eq!(classic.fill(CANVAS, false), Some(CANVAS));
    let mut custom = classic.clone();
    custom.color = ShieldColor::Custom;
    custom.custom_color = [9, 9, 9];
    assert_eq!(custom.fill(CANVAS, false), Some(Color32::from_rgb(9, 9, 9)));
}

#[trial]
fn stray_shield_opacities_are_clamped() {
    for (o, want) in [(f32::NAN, 0.75), (f32::INFINITY, 0.75), (f32::NEG_INFINITY, 0.75), (-20.0, 0.0), (250.0, 1.0), (40.0, 0.4)] {
        let s = shield(|s| s.opacity = o);
        assert!((s.opacity() - want).abs() < 1e-6, "{o}");
        assert_eq!(s.fill(CANVAS, false).map_or(0, |c| c.to_srgba_unmultiplied()[3]), (want * 255.0f32).round() as u8, "{o}");
    }
}

#[trial]
fn older_saved_shield_settings_get_the_defaults_and_round_trip() {
    let older: CropShield = toml_edit::de::from_str("enabled = false").expect("a legacy sample with one key");
    assert_eq!(older, shield(|s| s.enabled = false));
    let saved = toml_edit::ser::to_string(&older).expect("saves");
    assert_eq!(toml_edit::de::from_str::<CropShield>(&saved).expect("loads"), older);
    assert!(saved.contains("color = \"matchCanvas\""), "{saved}");
    assert!(toml_edit::de::from_str::<CropShield>("opacity = \"high\"").is_err());
}
