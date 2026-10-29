use egui::{Color32, Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect as ScreenRect, Sense, Shape, pos2, vec2};
use kurbo::{Point, Rect};

use super::{Constraint, Marquee, MarqueeDrag, MarqueeLook, MarqueeResponse, MarqueeShape, fixed_size_pixels, marquee_pixels, square_pixels};
use crate::canvas::{CanvasView, Claim};
use crate::edit::{CancelReason, EditEvent, EditPolicy, Gesture};

const SCREEN: ScreenRect = ScreenRect { min: pos2(0.0, 0.0), max: pos2(800.0, 600.0) };

/// A view whose zoom is 1 point per document unit, centred on the canvas.
fn view(rotation: f64) -> CanvasView {
    let rect = ScreenRect::from_min_size(pos2(100.0, 100.0), vec2(400.0, 300.0));
    CanvasView { rotation, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) }
}

#[test]
fn a_rotated_view_draws_the_rectangle_where_it_maps() {
    let r = Rect::new(150.0, 100.0, 260.0, 190.0);
    for rotation in [0.0, 30.0, 90.0, -119.0] {
        let view = view(rotation);
        let paths = Marquee::rect(r, "Selection").value("selection").paths(&view, MarqueeLook::Ants);
        assert_eq!(paths.len(), 1);
        assert!(paths[0].closed);
        assert_eq!(paths[0].points, view.quad(r).to_vec(), "rotation {rotation}");
    }
}

#[test]
fn an_ellipse_is_inscribed_in_the_rectangle_however_the_view_turns() {
    let r = Rect::new(150.0, 100.0, 290.0, 200.0);
    for (rotation, flip) in [(0.0, false), (0.0, true), (90.0, false), (30.0, false), (-119.0, true)] {
        let view = CanvasView { flip_x: flip, ..view(rotation) };
        let points = &Marquee::ellipse(r, "Selection").paths(&view, MarqueeLook::Ants)[0].points;
        assert!(points.len() >= 16 && points.len() <= 256);
        let c = r.center();
        for p in points {
            let q = view.to_doc(*p);
            // The documented ellipse, except in an axis-aligned view where it is the screen box's.
            let d = ((q.x - c.x) / (r.width() / 2.0)).powi(2) + ((q.y - c.y) / (r.height() / 2.0)).powi(2);
            assert!((d - 1.0).abs() < 1e-3, "rotation {rotation}: {q:?} off the ellipse ({d})");
        }
    }
}

#[test]
fn a_drawn_polygon_is_open_and_traced_segments_are_separate() {
    let view = view(0.0);
    let points = [Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(10.0, 10.0)];
    assert!(Marquee::polygon(points, "Lasso").paths(&view, MarqueeLook::Ants)[0].closed);
    assert!(!Marquee::polygon(points, "Lasso").paths(&view, MarqueeLook::Drawing)[0].closed);
    assert_eq!(Marquee::segments([(points[0], points[1]), (points[1], points[2])], "Selection").paths(&view, MarqueeLook::Ants).len(), 2);
}

/// A context with the suite default, reduced motion or not.
fn motion_context(reduced: bool) -> Context {
    use crate::theme::Base;
    use craft_theme::{AppDefaults, ModeChoice, Size, SuiteChoice, SystemAppearance, ThemeId, ThemePair, builtins};
    let defaults = AppDefaults { theme: ThemePair::same(ThemeId::suite_default()), mode: ModeChoice::Light, size: Size::Compact };
    let system = SystemAppearance { reduced_motion: Some(reduced), ..Default::default() };
    let base = Base::resolve(&SuiteChoice::default().with_defaults(&defaults), system, &defaults, None, &mut |id| {
        builtins().iter().find(|t| t.id == *id).cloned()
    });
    let ctx = Context::default();
    crate::theme::install(&ctx, &base);
    ctx
}

/// One frame at `time` drawing `marquee`: the shapes and the delay to the next repaint.
fn draw(ctx: &Context, time: f64, marquee: Marquee, view: &CanvasView) -> (Vec<Shape>, std::time::Duration) {
    let input = RawInput { screen_rect: Some(SCREEN), time: Some(time), ..RawInput::default() };
    let mut marquee = Some(marquee);
    let mut output = ctx.run_ui(input, |ui| {
        let canvas = ui.allocate_rect(view.rect, Sense::hover());
        if let Some(m) = marquee.take() {
            m.show(ui, &canvas, view);
        }
    });
    output.textures_delta.clear();
    let delay = output.viewport_output.values().next().map_or(std::time::Duration::MAX, |v| v.repaint_delay);
    let mut shapes = Vec::new();
    for clipped in output.shapes {
        fn flatten(s: Shape, out: &mut Vec<Shape>) {
            match s {
                Shape::Vec(v) => v.into_iter().for_each(|s| flatten(s, out)),
                s => out.push(s),
            }
        }
        flatten(clipped.shape, &mut shapes);
    }
    (shapes, delay)
}

#[test]
fn the_ants_cover_the_mapped_rectangle_and_march_until_reduced_motion_stops_them() {
    let r = Rect::new(150.0, 100.0, 260.0, 190.0);
    let view = view(30.0);
    let quad = ScreenRect::from_points(&view.quad(r)).expand(2.0);
    // Frames after the first, once the context has settled and asks only for what is drawn.
    let frames = |ctx: &Context| {
        for k in 0..3 {
            draw(ctx, 0.01 * f64::from(k), Marquee::rect(r, "Selection").value("selection"), &view);
        }
        (
            draw(ctx, 0.1, Marquee::rect(r, "Selection").value("selection"), &view),
            draw(ctx, 0.4, Marquee::rect(r, "Selection").value("selection"), &view),
        )
    };
    let ((first, _), (later, delay)) = frames(&motion_context(false));
    assert!(!first.is_empty());
    for s in &first {
        assert!(quad.contains_rect(s.visual_bounding_rect()), "{:?} outside {quad:?}", s.visual_bounding_rect());
    }
    assert_ne!(format!("{first:?}"), format!("{later:?}"), "they march");
    assert!(delay <= std::time::Duration::from_millis(100), "repaints every tick: {delay:?}");

    let ((still, _), (later, delay)) = frames(&motion_context(true));
    assert_eq!(format!("{still:?}"), format!("{later:?}"), "reduced motion: they stand still");
    assert!(delay > std::time::Duration::from_secs(60), "and ask for no repaint: {delay:?}");
}

#[test]
fn the_band_and_the_drawing_looks_use_the_marquee_colours() {
    let ctx = motion_context(false);
    let style = Marquee::style(&crate::theme::current(&ctx));
    let view = view(0.0);
    let r = Rect::new(150.0, 100.0, 260.0, 190.0);
    let colours = |look| -> Vec<Color32> {
        let (shapes, _) = draw(&ctx, 0.0, Marquee::rect(r, "Selection").value("selection").look(look), &view);
        shapes
            .iter()
            .filter_map(|s| match s {
                Shape::LineSegment { stroke, .. } => Some(stroke.color),
                Shape::Path(p) => match p.stroke.color {
                    egui::epaint::ColorMode::Solid(c) => Some(c),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    };
    let band = colours(MarqueeLook::Band);
    assert!(band.contains(&style.light) && band.contains(&style.dark));
    let drawing = colours(MarqueeLook::Drawing);
    assert_eq!(drawing, vec![style.drawing_halo.color, style.drawing_line.color]);
    assert_eq!(style.drawing_halo.color.a(), 140);
}

#[test]
fn the_marquee_is_a_graphics_object_inside_the_canvas_node_with_the_hosts_name_and_value() {
    use crate::localized::{LocaleNumbers, Unit};
    let ctx = motion_context(false);
    ctx.enable_accesskit();
    let view = view(0.0);
    let numbers = LocaleNumbers::from_tag("de");
    let unit = Unit::new("{n} px");
    let mut found = Vec::new();
    for marquee in [
        Marquee::rect(Rect::new(10.0, 20.0, 110.5, 70.0), "Auswahl").bounds_value(&numbers, &unit),
        Marquee::rect(Rect::new(0.0, 0.0, 5.0, 5.0), "Named").value("five by five"),
    ] {
        let mut marquee = Some(marquee);
        let mut canvas_id = Id::NULL;
        let output = ctx.run_ui(RawInput { screen_rect: Some(SCREEN), ..RawInput::default() }, |ui| {
            let canvas = ui.allocate_rect(view.rect, Sense::click_and_drag());
            canvas.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Canvas, true, "Document"));
            canvas_id = canvas.id;
            if let Some(m) = marquee.take() {
                m.show(ui, &canvas, &view);
            }
        });
        let update = output.platform_output.accesskit_update.clone().expect("accesskit update");
        output.drop_without_applying_deltas();
        let object = update
            .nodes
            .iter()
            .find(|(_, n)| n.role() == egui::accesskit::Role::GraphicsObject)
            .map(|(id, n)| (*id, n.clone()))
            .expect("a graphics object");
        // The object hangs below the canvas node (through egui's container of the child `Ui`).
        let parent_of = |child: egui::accesskit::NodeId| update.nodes.iter().find(|(_, n)| n.children().contains(&child)).map(|(id, _)| *id);
        let mut ancestors = Vec::new();
        let mut at = object.0;
        while let Some(parent) = parent_of(at) {
            ancestors.push(parent);
            at = parent;
        }
        assert!(ancestors.contains(&canvas_id.accesskit_id()), "inside the canvas node: {ancestors:?}");
        found.push((object.1.label().map(str::to_owned), object.1.value().map(str::to_owned)));
    }
    assert_eq!(found[0], (Some("Auswahl".to_owned()), Some("100,5 px × 50 px (10 px, 20 px)".to_owned())));
    assert_eq!(found[1], (Some("Named".to_owned()), Some("five by five".to_owned())));
}

#[test]
fn constraints_follow_photocraft() {
    let s = Point::new(10.0, 10.0);
    let near = |a: (Point, Point), b: [f64; 4]| {
        let got = [a.0.x, a.0.y, a.1.x, a.1.y];
        assert!(got.iter().zip(b).all(|(g, w)| (g - w).abs() < 1e-9), "{got:?} != {b:?}");
    };
    near(Constraint::Free.corners(s, Point::new(50.0, 30.0), false, false), [10.0, 10.0, 50.0, 30.0]);
    near(Constraint::Free.corners(s, Point::new(50.0, 30.0), true, false), [10.0, 10.0, 50.0, 50.0]);
    near(Constraint::Free.corners(s, Point::new(-30.0, 30.0), true, false), [10.0, 10.0, -30.0, 50.0]);
    near(Constraint::Free.corners(s, Point::new(50.0, 30.0), false, true), [-30.0, -10.0, 50.0, 30.0]);
    near(Constraint::Free.corners(s, Point::new(50.0, 30.0), true, true), [-30.0, -30.0, 50.0, 50.0]);
    // The larger drag extent wins; ⇧ changes nothing.
    near(Constraint::Ratio(4.0, 3.0).corners(s, Point::new(50.0, 20.0), false, false), [10.0, 10.0, 50.0, 40.0]);
    near(Constraint::Ratio(4.0, 3.0).corners(s, Point::new(50.0, 20.0), true, false), [10.0, 10.0, 50.0, 40.0]);
    // A fixed size is positioned at the pointer, whichever way the drag goes, or centred on it.
    near(Constraint::Size(20.0, 10.0).corners(s, Point::new(-50.0, 70.0), false, false), [-50.0, 70.0, -30.0, 80.0]);
    near(Constraint::Size(20.0, 10.0).corners(s, Point::new(50.0, 70.0), false, true), [40.0, 65.0, 60.0, 75.0]);
}

/// Drives a [`MarqueeDrag`] over a canvas through raw events, one pass per call. Document
/// coordinates are screen coordinates minus (100, 100).
struct Harness {
    ctx: Context,
    view: CanvasView,
    time: f64,
    modifiers: Modifiers,
    /// The modifiers the gesture reads in place of the input state.
    forced: Option<Modifiers>,
    constraint: Constraint,
    canvas: Id,
    /// Whether the marquee owned the press at the end of the last pass.
    owned: bool,
    /// A text field's typing run beside the canvas, begun in the first pass.
    typing: bool,
    /// The field's first pass has gone.
    began: bool,
    passes: usize,
    /// The pass in which the field's run committed.
    committed: Option<usize>,
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

fn key(key: Key, pressed: bool) -> Event {
    Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE }
}

impl Harness {
    fn new() -> Self {
        let mut h = Harness {
            ctx: motion_context(false),
            view: view(0.0),
            time: 0.0,
            modifiers: Modifiers::NONE,
            forced: None,
            constraint: Constraint::Free,
            canvas: Id::NULL,
            owned: false,
            typing: false,
            began: false,
            passes: 0,
            committed: None,
        };
        h.pass(vec![Event::PointerMoved(pos2(120.0, 120.0))]);
        h
    }

    fn pass(&mut self, events: Vec<Event>) -> MarqueeResponse {
        self.time += 0.016;
        let mut all = vec![Event::ModifiersChanged(self.modifiers)];
        all.extend(events);
        let input = RawInput { screen_rect: Some(SCREEN), time: Some(self.time), events: all, ..RawInput::default() };
        let (view, constraint, forced) = (self.view, self.constraint, self.forced);
        let mut out = MarqueeResponse::default();
        let mut canvas_id = Id::NULL;
        let mut owned = false;
        self.passes += 1;
        let (typing, pass, mut committed, first) = (self.typing, self.passes, self.committed, self.typing && !self.began);
        self.began |= typing;
        self.ctx
            .run_ui(input, |ui| {
                if typing {
                    let field = ui.make_persistent_id("typing field");
                    let response = ui.interact(ScreenRect::from_min_size(pos2(600.0, 500.0), vec2(20.0, 20.0)), field, Sense::click());
                    if first {
                        response.request_focus();
                    }
                    let mut gesture = Gesture::load(ui, field, &0, EditPolicy::default());
                    if first {
                        gesture.begin();
                    }
                    if matches!(gesture.finish(), Some(EditEvent::Commit { .. })) {
                        committed = Some(pass);
                    }
                }
                let canvas = ui.allocate_rect(view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                canvas_id = canvas.id;
                let mut drag = MarqueeDrag::new("test").shape(MarqueeShape::Rect).constraint(constraint);
                if let Some(m) = forced {
                    drag = drag.modifiers(m);
                }
                out = drag.show(ui, &canvas, &view);
                owned = Claim::owner(ui.ctx(), canvas.id).is_some();
            })
            .drop_without_applying_deltas();
        self.canvas = canvas_id;
        self.owned = owned;
        self.committed = committed;
        out
    }

    fn press(&mut self, at: Pos2) -> MarqueeResponse {
        let modifiers = self.modifiers;
        self.pass(vec![Event::PointerMoved(at), button(at, true, modifiers)])
    }

    fn drag(&mut self, to: Pos2) -> MarqueeResponse {
        self.pass(vec![Event::PointerMoved(to)])
    }

    fn release(&mut self, at: Pos2) -> MarqueeResponse {
        let modifiers = self.modifiers;
        self.pass(vec![button(at, false, modifiers)])
    }
}

fn corners(r: &MarqueeResponse) -> [f64; 4] {
    let r = r.rect.unwrap_or_else(|| panic!("no marquee: {r:?}"));
    [r.x0, r.y0, r.x1, r.y1]
}

const A: Pos2 = pos2(200.0, 200.0);

#[test]
fn a_drag_begins_past_the_slop_updates_and_commits_on_release() {
    let mut h = Harness::new();
    let press = h.press(A);
    assert_eq!((press.event, press.rect, press.clicked), (None, None, false));
    assert!(h.owned, "the press is the marquee's");
    assert_eq!(h.drag(A + vec2(2.0, 0.0)).event, None, "inside the click slop");
    let begun = h.drag(A + vec2(40.0, 30.0));
    assert!(matches!(begun.event, Some(EditEvent::Begin { .. })), "{:?}", begun.event);
    assert_eq!(corners(&begun), [100.0, 100.0, 140.0, 130.0]);
    let moved = h.drag(A + vec2(50.0, 40.0));
    assert!(matches!(moved.event, Some(EditEvent::Update { .. })), "{:?}", moved.event);
    assert_eq!(corners(&moved), [100.0, 100.0, 150.0, 140.0]);
    // Back inside the slop: still a drag.
    let back = h.drag(A + vec2(1.0, 1.0));
    assert!(matches!(back.event, Some(EditEvent::Update { .. })));
    let end = h.release(A + vec2(60.0, 50.0));
    match end.event {
        Some(EditEvent::Commit { value, changed, .. }) => {
            assert!(changed);
            assert_eq!((value.a, value.b), (Point::new(100.0, 100.0), Point::new(160.0, 150.0)));
        }
        other => panic!("{other:?}"),
    }
    assert!(!end.clicked);
    assert!(h.owned, "the release pass is still the marquee's");
    h.pass(vec![]);
    assert!(!h.owned);
}

#[test]
fn a_press_released_within_the_slop_is_a_click() {
    let mut h = Harness::new();
    h.modifiers = Modifiers::SHIFT;
    let press = h.press(A);
    assert_eq!(press.event, None);
    h.drag(A + vec2(1.0, 2.0));
    let end = h.release(A + vec2(1.0, 2.0));
    assert!(end.clicked);
    assert_eq!((end.event, end.rect, end.press_modifiers), (None, None, Modifiers::SHIFT));
    assert!(!h.pass(vec![]).clicked, "once");
}

#[test]
fn escape_cancels_and_the_rest_of_the_drag_does_nothing() {
    let mut h = Harness::new();
    h.press(A);
    h.drag(A + vec2(40.0, 30.0));
    let cancelled = h.pass(vec![key(Key::Escape, true)]);
    assert!(matches!(cancelled.event, Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{:?}", cancelled.event);
    assert_eq!(cancelled.rect, None);
    let more = h.drag(A + vec2(80.0, 60.0));
    assert_eq!((more.event, more.rect), (None, None));
    let end = h.release(A + vec2(80.0, 60.0));
    assert_eq!((end.event, end.clicked), (None, false));
}

#[test]
fn shift_and_alt_count_only_when_pressed_after_the_press_or_pressed_again() {
    let mut h = Harness::new();
    // Held at the press: the host's selection mode, not square.
    h.modifiers = Modifiers::SHIFT;
    h.press(A);
    assert_eq!(corners(&h.drag(A + vec2(40.0, 30.0))), [100.0, 100.0, 140.0, 130.0]);
    // Released, then pressed again: square.
    h.modifiers = Modifiers::NONE;
    h.drag(A + vec2(40.0, 30.0));
    h.modifiers = Modifiers::SHIFT;
    assert_eq!(corners(&h.drag(A + vec2(40.0, 30.0))), [100.0, 100.0, 140.0, 140.0]);
    h.release(A + vec2(40.0, 30.0));

    // Pressed after the press: square, and ⌥ draws from the centre.
    h.modifiers = Modifiers::NONE;
    h.press(A);
    h.drag(A + vec2(40.0, 30.0));
    h.modifiers = Modifiers::SHIFT;
    assert_eq!(corners(&h.drag(A + vec2(40.0, 30.0))), [100.0, 100.0, 140.0, 140.0]);
    h.modifiers = Modifiers::SHIFT | Modifiers::ALT;
    assert_eq!(corners(&h.drag(A + vec2(40.0, 30.0))), [60.0, 60.0, 140.0, 140.0]);
    let end = h.release(A + vec2(40.0, 30.0));
    assert_eq!(end.press_modifiers, Modifiers::NONE);
}

#[test]
fn alt_alone_draws_from_the_centre_and_shift_squares_towards_the_pointer_side() {
    let mut h = Harness::new();
    h.press(A);
    h.drag(A + vec2(40.0, 30.0));
    h.modifiers = Modifiers::ALT;
    assert_eq!(corners(&h.drag(A + vec2(40.0, 30.0))), [60.0, 70.0, 140.0, 130.0], "the press is the centre");
    h.modifiers = Modifiers::SHIFT;
    assert_eq!(corners(&h.drag(A + vec2(-40.0, 30.0))), [60.0, 100.0, 100.0, 140.0], "the square follows the pointer's quadrant");
    h.modifiers = Modifiers::NONE;
    assert_eq!(corners(&h.drag(A + vec2(-40.0, 30.0))), [60.0, 100.0, 100.0, 130.0], "released: free again");
}

#[test]
fn shift_and_alt_held_at_the_press_stay_the_hosts_until_both_are_released() {
    let mut h = Harness::new();
    h.modifiers = Modifiers::SHIFT | Modifiers::ALT;
    h.press(A);
    assert_eq!(corners(&h.drag(A + vec2(40.0, 30.0))), [100.0, 100.0, 140.0, 130.0]);
    // ⇧ released and pressed again squares; ⌥ is still the host's.
    h.modifiers = Modifiers::ALT;
    h.drag(A + vec2(40.0, 30.0));
    h.modifiers = Modifiers::SHIFT | Modifiers::ALT;
    assert_eq!(corners(&h.drag(A + vec2(40.0, 30.0))), [100.0, 100.0, 140.0, 140.0]);
}

#[test]
fn a_ratio_ignores_shift_and_alt_still_centres_it() {
    let mut h = Harness::new();
    h.constraint = Constraint::Ratio(4.0, 3.0);
    h.press(A);
    h.drag(A + vec2(10.0, 10.0));
    h.modifiers = Modifiers::SHIFT;
    assert_eq!(corners(&h.drag(A + vec2(40.0, 20.0))), [100.0, 100.0, 140.0, 130.0]);
    h.modifiers = Modifiers::SHIFT | Modifiers::ALT;
    assert_eq!(corners(&h.drag(A + vec2(40.0, 20.0))), [60.0, 70.0, 140.0, 130.0]);
}

#[test]
fn space_keeps_the_square_and_the_centre_while_it_moves_the_marquee() {
    let mut h = Harness::new();
    h.press(A);
    h.drag(A + vec2(40.0, 30.0));
    h.modifiers = Modifiers::SHIFT;
    assert_eq!(corners(&h.drag(A + vec2(40.0, 30.0))), [100.0, 100.0, 140.0, 140.0]);
    h.pass(vec![key(Key::Space, true)]);
    assert_eq!(corners(&h.drag(A + vec2(90.0, 80.0))), [150.0, 150.0, 190.0, 190.0]);
}

#[test]
fn space_moves_the_marquee_and_releasing_it_sizes_again() {
    let mut h = Harness::new();
    h.press(A);
    assert_eq!(corners(&h.drag(A + vec2(40.0, 30.0))), [100.0, 100.0, 140.0, 130.0]);
    h.pass(vec![key(Key::Space, true)]);
    assert_eq!(corners(&h.drag(A + vec2(90.0, 80.0))), [150.0, 150.0, 190.0, 180.0], "same size, moved with the pointer");
    h.pass(vec![key(Key::Space, false)]);
    assert_eq!(corners(&h.drag(A + vec2(100.0, 90.0))), [150.0, 150.0, 200.0, 190.0], "sizing from the moved corner");
    let end = h.release(A + vec2(100.0, 90.0));
    assert!(matches!(end.event, Some(EditEvent::Commit { .. })));
}

#[test]
fn a_fixed_size_is_positioned_by_the_drag() {
    let mut h = Harness::new();
    h.constraint = Constraint::Size(50.0, 20.0);
    h.press(A);
    assert_eq!(corners(&h.drag(A + vec2(40.0, 30.0))), [140.0, 130.0, 190.0, 150.0]);
    // Past the press the other way: the size stays.
    assert_eq!(corners(&h.drag(A - vec2(40.0, 30.0))), [60.0, 70.0, 110.0, 90.0]);
}

#[test]
fn a_fixed_size_previews_from_the_press_before_the_slop() {
    let mut h = Harness::new();
    h.constraint = Constraint::Size(50.0, 20.0);
    let press = h.press(A);
    assert!(matches!(press.event, Some(EditEvent::Begin { .. })), "{:?}", press.event);
    assert_eq!(corners(&press), [100.0, 100.0, 150.0, 120.0]);
    let moved = h.drag(A + vec2(2.0, 1.0));
    assert!(matches!(moved.event, Some(EditEvent::Update { .. })), "{:?}", moved.event);
    assert_eq!(corners(&moved), [102.0, 101.0, 152.0, 121.0], "inside the slop it already follows the pointer");
}

#[test]
fn a_fixed_size_click_commits_the_box_at_the_pointer() {
    let mut h = Harness::new();
    h.constraint = Constraint::Size(50.0, 20.0);
    h.press(A);
    let end = h.release(A);
    match end.event {
        Some(EditEvent::Commit { value, changed, .. }) => {
            assert!(changed);
            assert_eq!((value.a, value.b), (Point::new(100.0, 100.0), Point::new(150.0, 120.0)));
        }
        other => panic!("{other:?}"),
    }
    assert!(!end.clicked, "a fixed-size click is not a deselecting click");
}

#[test]
fn a_fixed_size_click_with_alt_centres_the_box_on_the_pointer() {
    let mut h = Harness::new();
    h.constraint = Constraint::Size(50.0, 20.0);
    h.press(A);
    h.modifiers = Modifiers::ALT;
    let held = h.drag(A);
    assert_eq!(corners(&held), [75.0, 90.0, 125.0, 110.0]);
    let end = h.release(A);
    assert!(matches!(end.event, Some(EditEvent::Commit { .. })), "{:?}", end.event);
    assert_eq!(corners(&end), [75.0, 90.0, 125.0, 110.0]);
}

#[test]
fn free_and_ratio_clicks_stay_clicks() {
    for constraint in [Constraint::Free, Constraint::Ratio(4.0, 3.0)] {
        let mut h = Harness::new();
        h.constraint = constraint;
        assert_eq!(h.press(A).event, None);
        let end = h.release(A);
        assert!(end.clicked && end.event.is_none(), "{constraint:?}");
    }
}

/// A shift square pressed at (50.3, 50.7) and dragged to (150.8, 90.0): the corners snapped on
/// their own are 101 x 102; the square is 102 x 102 and covers every pixel the drag touched.
#[test]
fn a_square_stays_square_after_rounding_to_pixels() {
    let (start, end) = (Point::new(50.3, 50.7), Point::new(150.8, 90.0));
    let (a, b) = Constraint::Free.corners(start, end, true, false);
    let snapped = marquee_pixels(a, b);
    assert_eq!((snapped.width(), snapped.height()), (101.0, 102.0));
    assert_eq!(square_pixels(snapped, start, end), Rect::new(50.0, 50.0, 152.0, 152.0));
    // Dragged up and left the short side grows away from the press, towards the pointer.
    let up = Rect::new(10.0, 20.0, 60.0, 30.0);
    assert_eq!(square_pixels(up, Point::new(60.0, 30.0), Point::new(10.0, 20.0)), Rect::new(10.0, -20.0, 60.0, 30.0));
    assert_eq!(square_pixels(up, Point::new(10.0, 20.0), Point::new(60.0, 30.0)), Rect::new(10.0, 20.0, 60.0, 70.0));
}

#[test]
fn a_marquee_snaps_outwards_to_whole_pixels_from_either_corner() {
    let (a, b) = (Point::new(50.3, 50.7), Point::new(150.8, 90.0));
    assert_eq!(marquee_pixels(a, b), Rect::new(50.0, 50.0, 151.0, 90.0));
    assert_eq!(marquee_pixels(b, a), marquee_pixels(a, b));
}

#[test]
fn a_fixed_size_keeps_its_exact_pixel_size_after_rounding() {
    for (w, h) in [(10.0, 10.0), (1.0, 1.0), (64.0, 32.0)] {
        let (a, b) = Constraint::Size(w, h).corners(Point::ZERO, Point::new(50.4, 60.6), false, false);
        assert_eq!(fixed_size_pixels(a, b, (w, h)), Rect::new(50.0, 61.0, 50.0 + w, 61.0 + h));
    }
    let (a, b) = Constraint::Size(10.0, 10.0).corners(Point::ZERO, Point::new(120.6, 100.4), false, false);
    assert_eq!(fixed_size_pixels(a, b, (10.0, 10.0)), Rect::new(121.0, 100.0, 131.0, 110.0));
    assert_eq!(fixed_size_pixels(Point::ZERO, Point::ZERO, (0.2, 3.6)), Rect::new(0.0, 0.0, 1.0, 4.0));
}

#[test]
fn another_overlay_that_owns_the_press_keeps_the_marquee_out() {
    let mut h = Harness::new();
    h.pass(vec![]);
    let guides = Id::unique("guides");
    let at = A;
    let modifiers = Modifiers::NONE;
    // The press is taken before the marquee runs, as the host orders its overlays.
    let input = RawInput {
        screen_rect: Some(SCREEN),
        time: Some(h.time + 0.016),
        events: vec![Event::PointerMoved(at), button(at, true, modifiers)],
        ..RawInput::default()
    };
    let view = h.view;
    let mut out = None;
    h.ctx
        .run_ui(input, |ui| {
            let canvas = ui.allocate_rect(view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
            assert!(Claim::take(ui.ctx(), canvas.id, guides));
            out = Some(MarqueeDrag::new("test").show(ui, &canvas, &view));
        })
        .drop_without_applying_deltas();
    assert_eq!(out, Some(MarqueeResponse::default()));
    let moved = h.drag(A + vec2(40.0, 30.0));
    assert_eq!((moved.event, moved.rect), (None, None));
}

#[test]
fn a_press_and_moves_in_one_frame_begin_the_drag_in_that_frame() {
    let mut h = Harness::new();
    let first = h.pass(vec![
        Event::PointerMoved(A),
        button(A, true, Modifiers::NONE),
        Event::PointerMoved(A + vec2(20.0, 10.0)),
        Event::PointerMoved(A + vec2(40.0, 30.0)),
    ]);
    assert!(matches!(first.event, Some(EditEvent::Begin { .. })), "{:?}", first.event);
    assert_eq!(corners(&first), [100.0, 100.0, 140.0, 130.0]);
    assert!(h.owned);
    // Moves and the release in one frame: the commit is at the release.
    let end = h.pass(vec![
        Event::PointerMoved(A + vec2(50.0, 40.0)),
        Event::PointerMoved(A + vec2(60.0, 50.0)),
        button(A + vec2(60.0, 50.0), false, Modifiers::NONE),
    ]);
    match end.event {
        Some(EditEvent::Commit { value, .. }) => assert_eq!((value.a, value.b), (Point::new(100.0, 100.0), Point::new(160.0, 150.0))),
        other => panic!("{other:?}"),
    }
}

/// A press during a text field's typing run reports nothing until the run's Commit has gone out;
/// the drag then begins from the press, and a press released meanwhile still ends as a click.
#[test]
fn a_press_during_a_typing_run_begins_after_the_runs_commit() {
    let mut h = Harness::new();
    h.typing = true;
    h.pass(vec![]);
    let press = h.press(A);
    assert_eq!((press.event, press.rect), (None, None));
    assert!(h.owned, "the marquee holds the press while it waits");
    let mut events = Vec::new();
    for i in 0..5 {
        let out = h.drag(A + vec2(30.0, 20.0 + i as f32));
        let waiting = h.committed.is_none_or(|c| c >= h.passes);
        assert!(!waiting || out.event.is_none(), "nothing before the run Commit: {out:?} {:?} {}", h.committed, h.passes);
        events.extend(out.event);
    }
    let out = h.release(A + vec2(30.0, 24.0));
    events.extend(out.event);
    assert!(h.committed.is_some(), "the run ended");
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { changed: true, .. })), "{events:?}");

    let mut h = Harness::new();
    h.typing = true;
    h.pass(vec![]);
    h.press(A);
    assert!(!h.release(A).clicked, "the click waits");
    let clicks = (0..5).filter(|_| h.pass(vec![]).clicked).count();
    assert_eq!(clicks, 1, "and runs once");
}

#[test]
fn modifiers_override_replaces_the_input_modifiers_in_the_marquee() {
    for (input, forced) in [(Modifiers::NONE, Modifiers::SHIFT), (Modifiers::SHIFT, Modifiers::NONE)] {
        let mut h = Harness::new();
        h.modifiers = input;
        h.forced = Some(forced);
        h.press(A);
        let end = h.release(A);
        assert!(end.clicked);
        assert_eq!(end.press_modifiers, forced, "input {input:?}");
    }
}
