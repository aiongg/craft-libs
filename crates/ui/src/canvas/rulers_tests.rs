use egui::accesskit::{self, Role};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect as ScreenRect, Sense, Shape, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::{RulerNames, RulerPart, Rulers};
use crate::canvas::guides::GuideHost;
use crate::canvas::{CanvasView, GuideNames, GuidePlace, Guides, Orientation, RulerUnits};
use crate::edit::{CancelReason, EditEvent};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, install};

const SIZE: f32 = 16.0;

struct Host {
    zoom: f64,
    flip: bool,
    rotation: f64,
    /// The size of the canvas inside the rulers.
    canvas: egui::Vec2,
    zero: Point,
    locked: bool,
    guides: GuideHost,
    view: Option<CanvasView>,
    origin: Vec<EditEvent<Point>>,
    resets: usize,
    contexts: Vec<RulerPart>,
    guide_events: Vec<(Option<u64>, EditEvent<GuidePlace>)>,
    /// The host's Set Zero Point… and Reset Zero Point commands, run after the rulers.
    set_zero: Option<Point>,
    reset_zero: bool,
}

impl Host {
    fn new() -> Self {
        Self {
            zoom: 1.0,
            flip: false,
            rotation: 0.0,
            canvas: vec2(400.0, 300.0),
            zero: Point::ZERO,
            locked: false,
            guides: GuideHost::new(Vec::new()),
            view: None,
            origin: Vec::new(),
            resets: 0,
            contexts: Vec::new(),
            guide_events: Vec::new(),
            set_zero: None,
            reset_zero: false,
        }
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let size = host.canvas + vec2(SIZE + 40.0, SIZE + 40.0);
    Harness::<Host>::builder().with_size(size).with_step_dt(1.0 / 60.0).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let full = ScreenRect::from_min_size(pos2(20.0, 20.0), host.canvas + vec2(SIZE, SIZE));
            let content = ScreenRect::from_min_max(full.min + vec2(SIZE, SIZE), full.max);
            let canvas = ui.interact(content, ui.scope_id().with("canvas"), Sense::click_and_drag() - Sense::FOCUSABLE);
            let center = Point::new(f64::from(host.canvas.x) / 2.0, f64::from(host.canvas.y) / 2.0);
            let view = CanvasView { rotation: host.rotation, flip_x: host.flip, ..CanvasView::new(content, center, host.zoom) };
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped());
            let names = GuideNames::new("Guides", "Vertical guide", "Horizontal guide");
            let guides = Guides::new("guides", &host.guides.guides, &px, &px, names)
                .selected(host.guides.selected)
                .interactive(true)
                .bounds(Rect::new(0.0, 0.0, f64::from(host.canvas.x), f64::from(host.canvas.y)))
                .zero(host.zero)
                .show(ui, &canvas, &view);
            host.guide_events.extend(guides.event.clone());
            host.guides.apply(&guides);
            let names = RulerNames::new("Horizontal ruler", "Vertical ruler", "Ruler origin");
            let rulers = Rulers::new("rulers", &px, &px, names).zero(host.zero).origin_locked(host.locked).show(ui, full, &view, Some(guides.id));
            host.origin.extend(rulers.origin.clone());
            host.resets += usize::from(rulers.reset_origin);
            host.contexts.extend(rulers.context);
            if let Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) = rulers.origin {
                host.zero = value;
            }
            if let Some(point) = host.set_zero.take() {
                Rulers::set_zero(ui.ctx(), rulers.id, point);
            }
            if std::mem::take(&mut host.reset_zero) {
                Rulers::reset_zero(ui.ctx(), rulers.id);
            }
        },
        host,
    )
}

fn view(h: &Harness<'_, Host>) -> CanvasView {
    h.state().view.expect("drawn")
}

fn corner(h: &Harness<'_, Host>) -> Pos2 {
    view(h).rect.min - vec2(SIZE / 2.0, SIZE / 2.0)
}

fn top_ruler(h: &Harness<'_, Host>, x: f32) -> Pos2 {
    pos2(x, view(h).rect.min.y - SIZE / 2.0)
}

fn left_ruler(h: &Harness<'_, Host>, y: f32) -> Pos2 {
    pos2(view(h).rect.min.x - SIZE / 2.0, y)
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

fn press(h: &mut Harness<'_, Host>, at: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(at));
    h.run();
    h.event(button(at, true, modifiers));
    h.run();
}

fn move_to(h: &mut Harness<'_, Host>, to: Pos2) {
    h.event(Event::PointerMoved(to));
    h.run();
}

fn release(h: &mut Harness<'_, Host>, at: Pos2, modifiers: Modifiers) {
    h.event(button(at, false, modifiers));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    press(h, from, modifiers);
    move_to(h, from.lerp(to, 0.5));
    move_to(h, to);
    release(h, to, modifiers);
}

fn kinds(h: &Harness<'_, Host>) -> Vec<&'static str> {
    h.state()
        .origin
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "begin",
            EditEvent::Update { .. } => "update",
            EditEvent::Commit { changed: true, .. } => "commit",
            EditEvent::Commit { changed: false, .. } => "commit unchanged",
            EditEvent::Cancel { .. } => "cancel",
            EditEvent::Set { .. } => "set",
        })
        .collect()
}

fn near(a: Point, b: Point) -> bool {
    (a - b).hypot() < 1e-2
}

fn px() -> RulerUnits {
    RulerUnits::pixels(LocaleNumbers::from_tag("en").ungrouped())
}

/// PhotoCraft's tick cases: labelled steps follow 1, 2, 2.5, 5 × 10ⁿ with at least 60 points
/// between labels, whole for pixels; labels are distances without a minus sign.
#[test]
fn tick_steps_and_labels_follow_photocraft() {
    let pixels = px();
    for (zoom, step) in [(1.0, 100.0), (0.1, 1000.0), (4.0, 20.0), (60.0, 1.0), (0.01, 10_000.0)] {
        assert_eq!(pixels.step(zoom, 60.0), (step, 10), "zoom {zoom}");
    }
    let units = RulerUnits { ticks: crate::canvas::Ticks::Decimal { whole: false }, ..px() };
    for (min, step) in [(0.3, 0.5), (0.2, 0.2), (0.22, 0.25)] {
        // `min` units between labels: 60 points at 60 / min points per unit.
        let (got, subdivisions) = units.step(60.0 / min, 60.0);
        assert!((got - step).abs() < 1e-9 && subdivisions == 10, "min {min}: {got}");
    }
    assert_eq!(pixels.tick_label(-500.0, 100.0), "500");
    assert_eq!(pixels.tick_label(12.6, 5.0), "13", "whole pixels");
    assert_eq!(pixels.tick_label(1200.0, 100.0), "1200", "ungrouped");
    assert_eq!(units.tick_label(-0.25, 0.25), "0.25");
    assert_eq!(units.tick_label(0.5, 0.5), "0.50");
    assert_eq!(units.tick_label(0.75, 0.25), "0.75");
    // 2.5-unit steps (millimetres at about 700 %) keep their half, which PhotoCraft rounded away.
    assert_eq!(units.tick_label(2.5, 2.5), "2.5");
    assert_eq!(units.tick_label(-7.5, 2.5), "7.5");
    assert_eq!(units.tick_label(5.0, 2.5), "5.0");
    assert_eq!(units.tick_label(0.075, 0.025), "0.075");
    assert_eq!(units.tick_label(-30.0, 10.0), "30", "whole when the step is at least 1");
    for step in [100.0, 0.25] {
        let mut v = -1000.0;
        while v <= 1000.0 {
            assert!(!units.tick_label(v, step).contains('-') && !pixels.tick_label(v, step).contains('-'), "{v}");
            v += step;
        }
    }
}

/// Ticks and labels are clipped away from the zero point's corner at any zoom, flip and rotation
/// (PhotoCraft's test).
#[test]
fn labels_stay_out_of_the_corner() {
    for (zoom, flip, rotation, width) in [(1.0, false, 0.0, 560.0), (1.0, false, 0.0, 800.0), (0.25, true, 0.0, 560.0), (2.0, false, 30.0, 560.0)] {
        let host = Host { zoom, flip, rotation, canvas: vec2(width - 80.0, 300.0), ..Host::new() };
        let mut h = harness(host);
        h.run();
        let corner = ScreenRect::from_center_size(corner(&h), vec2(SIZE, SIZE));
        h.event(Event::PointerMoved(corner.center()));
        h.run_steps(4);
        let (mut ticks, mut labels) = (0, 0);
        for shape in &h.output().shapes {
            let is_tick = matches!(shape.shape, Shape::Mesh(_)) && shape.shape.visual_bounding_rect().width() <= 1.5;
            let is_label = matches!(shape.shape, Shape::Text(_));
            if is_tick || is_label {
                ticks += usize::from(is_tick);
                labels += usize::from(is_label);
                let visible = shape.shape.visual_bounding_rect().intersect(shape.clip_rect);
                assert!(!visible.intersect(corner.shrink(0.5)).is_positive(), "zoom {zoom}, flip {flip}, rotation {rotation}: {visible:?}");
            }
        }
        assert!(ticks > 2 && labels > 0, "zoom {zoom}: {ticks} ticks, {labels} labels");
    }
}

fn labels(h: &Harness<'_, Host>) -> Vec<(String, Pos2)> {
    h.output().shapes.iter().filter_map(|s| if let Shape::Text(t) = &s.shape { Some((t.galley.text().to_owned(), t.pos)) } else { None }).collect()
}

#[test]
fn the_zero_label_sits_at_the_zero_point_in_a_rotated_view_and_labels_count_distances() {
    for rotation in [0.0, 30.0, 90.0] {
        let host = Host { rotation, zero: Point::new(100.0, 40.0), ..Host::new() };
        let mut h = harness(host);
        h.run_steps(2);
        let v = view(&h);
        let at = v.to_screen(Point::new(100.0, 40.0));
        let labels = labels(&h);
        let top = |text: &str, x: f32| labels.iter().any(|(t, p)| t == text && p.y < v.rect.min.y && (p.x - (x + 2.0)).abs() < 1.0);
        assert!(top("0", at.x), "rotation {rotation}: a 0 on the top ruler at the zero point: {labels:?}");
        // The rulers stay level: the 100 label is 100 points to the left of the zero point.
        assert!(top("100", at.x - 100.0), "rotation {rotation}: 100 at the level position: {labels:?}");
        assert!(!labels.iter().any(|(t, _)| t.contains('-')));
    }
}

#[test]
fn the_left_ruler_centres_the_characters_of_a_label() {
    let mut h = harness(Host::new());
    h.run_steps(2);
    let v = view(&h);
    let centres: Vec<f32> = h
        .output()
        .shapes
        .iter()
        .filter_map(|s| if let Shape::Text(t) = &s.shape { Some(t) } else { None })
        .filter(|t| t.pos.x < v.rect.min.x - 1.0 && t.pos.y > v.rect.min.y)
        .map(|t| t.pos.x + t.galley.size().x * 0.5)
        .collect();
    assert!(centres.len() > 2);
    for c in centres {
        assert!((c - (v.rect.min.x - SIZE / 2.0)).abs() < 0.01, "character centre {c}");
    }
}

#[test]
fn dragging_the_corner_moves_the_zero_point_with_cross_hairs() {
    let mut h = harness(Host::new());
    h.run();
    let target = view(&h).to_screen(Point::new(100.0, 40.0));
    let start = corner(&h);
    press(&mut h, start, Modifiers::NONE);
    move_to(&mut h, start + vec2(10.0, 10.0));
    move_to(&mut h, target);
    let cross = h.output().shapes.iter().any(|s| {
        let r = s.shape.visual_bounding_rect();
        matches!(s.shape, Shape::Mesh(_)) && (r.center().x - target.x).abs() < 1.0 && r.height() > 200.0
    });
    assert!(cross, "a vertical cross hair through the pointer");
    assert_eq!(h.state().zero, Point::ZERO, "the zero point moves on release");
    release(&mut h, target, Modifiers::NONE);
    assert!(near(h.state().zero, Point::new(100.0, 40.0)), "{:?}", h.state().zero);
    let kinds = kinds(&h);
    assert_eq!((kinds.first(), kinds.last()), (Some(&"begin"), Some(&"commit")));
}

#[test]
fn shift_snaps_the_zero_point_to_ticks_from_the_current_zero_point() {
    let mut h = harness(Host { zero: Point::new(100.0, 40.0), ..Host::new() });
    h.run();
    let target = view(&h).to_screen(Point::new(123.0, 57.0));
    let start = corner(&h);
    drag(&mut h, start, target, Modifiers::SHIFT);
    // Ticks are 10 points apart at zoom 1, counted from (100, 40).
    assert!(near(h.state().zero, Point::new(120.0, 60.0)), "{:?}", h.state().zero);
}

#[test]
fn dropped_back_on_the_corner_the_zero_point_stays() {
    let mut h = harness(Host { zero: Point::new(100.0, 40.0), ..Host::new() });
    h.run();
    let start = corner(&h);
    press(&mut h, start, Modifiers::NONE);
    move_to(&mut h, start + vec2(80.0, 80.0));
    move_to(&mut h, start + vec2(3.0, 3.0));
    release(&mut h, start + vec2(3.0, 3.0), Modifiers::NONE);
    assert!(near(h.state().zero, Point::new(100.0, 40.0)));
    assert_eq!(kinds(&h).last(), Some(&"commit unchanged"));
}

#[test]
fn double_clicking_the_corner_resets_the_zero_point() {
    let mut h = harness(Host { zero: Point::new(100.0, 40.0), ..Host::new() });
    h.run();
    let at = corner(&h);
    h.event(Event::PointerMoved(at));
    h.step();
    // One frame per event: the second click has to follow the first within egui's window.
    for _ in 0..2 {
        h.event(button(at, true, Modifiers::NONE));
        h.step();
        h.event(button(at, false, Modifiers::NONE));
        h.step();
    }
    h.run_steps(2);
    assert_eq!(h.state().resets, 1);
    assert!(h.state().origin.is_empty(), "a reset is not a drag");
}

#[test]
fn escape_cancels_a_zero_point_drag() {
    let mut h = harness(Host::new());
    h.run();
    let (start, target) = (corner(&h), view(&h).to_screen(Point::new(100.0, 40.0)));
    press(&mut h, start, Modifiers::NONE);
    move_to(&mut h, start + vec2(10.0, 10.0));
    move_to(&mut h, target);
    h.key_press(Key::Escape);
    h.run();
    move_to(&mut h, target + vec2(5.0, 5.0));
    release(&mut h, target, Modifiers::NONE);
    assert!(matches!(h.state().origin.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{:?}", kinds(&h));
    assert_eq!(h.state().zero, Point::ZERO);
    assert_eq!(kinds(&h).iter().filter(|k| **k == "cancel").count(), 1, "nothing more after the cancel: {:?}", kinds(&h));
}

#[test]
fn a_locked_zero_point_takes_no_drag_or_reset() {
    let mut h = harness(Host { locked: true, ..Host::new() });
    h.run();
    let (start, target) = (corner(&h), view(&h).to_screen(Point::new(100.0, 40.0)));
    drag(&mut h, start, target, Modifiers::NONE);
    assert!(h.state().origin.is_empty() && h.state().zero == Point::ZERO);
}

#[test]
fn a_drag_out_of_each_ruler_makes_the_right_guide() {
    let mut h = harness(Host::new());
    h.run();
    let v = view(&h);
    let (from, to) = (top_ruler(&h, v.rect.min.x + 150.0), v.to_screen(Point::new(150.0, 80.0)));
    drag(&mut h, from, to, Modifiers::NONE);
    let (from, to) = (left_ruler(&h, v.rect.min.y + 150.0), v.to_screen(Point::new(210.0, 150.0)));
    drag(&mut h, from, to, Modifiers::NONE);
    let guides: Vec<_> = h.state().guides.guides.iter().map(|g| (g.orientation, g.position)).collect();
    assert_eq!(guides, [(Orientation::Horizontal, 80.0), (Orientation::Vertical, 210.0)]);
    // Alt turns the guide the other way.
    let from = top_ruler(&h, v.rect.min.x + 50.0);
    drag(&mut h, from, v.to_screen(Point::new(60.0, 220.0)), Modifiers::ALT);
    assert_eq!(h.state().guides.guides.last().map(|g| g.orientation), Some(Orientation::Vertical));
}

#[test]
fn a_secondary_click_names_the_part() {
    let mut h = harness(Host::new());
    h.run();
    let v = view(&h);
    for (at, part) in [
        (top_ruler(&h, v.rect.min.x + 50.0), RulerPart::Horizontal),
        (left_ruler(&h, v.rect.min.y + 50.0), RulerPart::Vertical),
        (corner(&h), RulerPart::Corner),
    ] {
        h.event(Event::PointerMoved(at));
        h.run();
        for pressed in [true, false] {
            h.event(Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
            h.run();
        }
        assert_eq!(h.state().contexts.last(), Some(&part));
    }
}

#[test]
fn the_rulers_and_the_corner_are_in_the_accesskit_tree() {
    let mut h = harness(Host { zero: Point::new(100.0, 40.0), ..Host::new() });
    h.run_steps(2);
    for (name, orientation) in [("Horizontal ruler", accesskit::Orientation::Horizontal), ("Vertical ruler", accesskit::Orientation::Vertical)] {
        let node = h.get_by_label(name);
        let data = node.accesskit_node().data().clone();
        assert_eq!(data.role(), Role::Group);
        assert_eq!(data.orientation(), Some(orientation));
        assert_eq!(data.value(), Some("px"));
        assert!(!data.supports_action(accesskit::Action::Focus), "{name} is not a Tab stop");
    }
    let corner = h.get_by_label("Ruler origin");
    let data = corner.accesskit_node().data().clone();
    assert_eq!(data.role(), Role::GraphicsObject);
    assert_eq!(data.value(), Some("100.0 px, 40.0 px"));
    assert!(!data.supports_action(accesskit::Action::Focus));
}

#[test]
fn the_story_switches_units_from_the_menu_a_secondary_click_opens_at_the_pointer() {
    let story = &super::stories()[0];
    let mut h = Harness::<crate::story::Args>::builder()
        .with_size([520.0, 420.0])
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(story.show, story.defaults());
    h.run_steps(3);
    let ruler = h.get_by_label("Horizontal ruler").rect();
    let at = pos2(ruler.center().x, ruler.center().y);
    h.event(Event::PointerMoved(at));
    h.run();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
        h.run();
    }
    h.run_steps(3);
    let inches = h.get_by_label("Inches").rect();
    // The zero point commands an app also has in its menu bar.
    h.get_by_label("Reset Zero Point");
    h.get_by_label("Set Zero Point…");
    assert!((inches.left() - at.x).abs() < 40.0 && inches.top() >= at.y - 2.0, "the menu is at the pointer: {inches:?} for {at:?}");
    assert_eq!(h.get_by_label("Pixels").accesskit_node().data().toggled(), Some(accesskit::Toggled::True), "the current unit is checked");
    let to = inches.center();
    h.event(Event::PointerMoved(to));
    h.run();
    for pressed in [true, false] {
        h.event(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        h.run();
    }
    h.run_steps(3);
    assert_eq!(h.state().choice("units"), "Inches");
    assert_eq!(h.get_by_label("Horizontal ruler").accesskit_node().data().value(), Some("in"));
}

/// Queues `events` into one frame, as winit delivers a quick gesture, and runs it.
fn frame(h: &mut Harness<'_, Host>, events: impl IntoIterator<Item = Event>) {
    h.input_mut().events.extend(events);
    h.step();
}

#[test]
fn a_quick_zero_point_drag_with_its_press_and_moves_in_one_frame() {
    let mut h = harness(Host::new());
    h.run();
    let (start, target) = (corner(&h), view(&h).to_screen(Point::new(100.0, 40.0)));
    // The press frame ends over the canvas; the press is the corner's.
    frame(
        &mut h,
        [
            Event::PointerMoved(start),
            button(start, true, Modifiers::NONE),
            Event::PointerMoved(start + vec2(5.0, 5.0)),
            Event::PointerMoved(start + vec2(40.0, 30.0)),
        ],
    );
    assert!(view(&h).rect.contains(start + vec2(40.0, 30.0)));
    frame(&mut h, [Event::PointerMoved(target)]);
    frame(&mut h, [Event::PointerMoved(target + vec2(1.0, 1.0)), Event::PointerMoved(target), button(target, false, Modifiers::NONE)]);
    h.run();
    assert!(near(h.state().zero, Point::new(100.0, 40.0)), "{:?} after {:?}", h.state().zero, kinds(&h));
    assert_eq!((kinds(&h).first(), kinds(&h).last()), (Some(&"begin"), Some(&"commit")));
}

#[test]
fn the_hosts_zero_point_commands_report_like_the_corner() {
    let mut h = harness(Host { zero: Point::new(100.0, 40.0), ..Host::new() });
    h.run();
    h.state_mut().set_zero = Some(Point::new(12.34567, 7.0));
    h.run();
    assert_eq!(kinds(&h), ["set"]);
    assert_eq!(h.state().zero, Point::new(12.346, 7.0), "written to the zero point's decimals");
    h.state_mut().reset_zero = true;
    h.run();
    assert_eq!(h.state().resets, 1);
    // Locked: the commands do nothing.
    h.state_mut().locked = true;
    h.state_mut().set_zero = Some(Point::new(50.0, 50.0));
    h.state_mut().reset_zero = true;
    h.run();
    assert_eq!((kinds(&h).len(), h.state().resets, h.state().zero), (1, 1, Point::new(12.346, 7.0)));
}

#[test]
fn left_ruler_labels_sit_at_their_level_positions_in_flipped_and_rotated_views() {
    let pad = crate::theme::current(&egui::Context::default()).component.ruler.label_pad;
    for (rotation, flip) in [(0.0, true), (30.0, true), (90.0, false), (90.0, true)] {
        let host = Host { rotation, flip, zero: Point::new(100.0, 40.0), ..Host::new() };
        let mut h = harness(host);
        h.run_steps(2);
        let v = view(&h);
        let at = v.to_screen(Point::new(100.0, 40.0));
        let left: Vec<(String, Pos2)> = labels(&h).into_iter().filter(|(_, p)| p.x < v.rect.min.x && p.y > v.rect.min.y).collect();
        let first_char = |text: &str, y: f32| left.iter().any(|(t, p)| t == text && (p.y - (y + pad)).abs() < 1.0);
        assert!(first_char("0", at.y), "rotation {rotation}, flip {flip}: 0 at the zero point's level: {left:?}");
        assert!(first_char("1", at.y + 100.0), "rotation {rotation}, flip {flip}: 100 below it: {left:?}");
    }
}
