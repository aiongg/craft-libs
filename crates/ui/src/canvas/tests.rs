use egui::{Context, CursorIcon, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect as ScreenRect, Shape, Stroke, pos2, vec2};
use kurbo::{Point, Rect, Vec2};

use super::readout::ReadoutLayout;
use super::{CanvasView, Claim, Readout, crisp_shape, hit, resize_cursor};
use crate::behavior::nudge;
use crate::edit::order::Turn;
use crate::edit::{EditPolicy, Gesture};
use crate::theme::{Tokens, TokensOf as _};

fn screen() -> ScreenRect {
    ScreenRect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))
}

/// PhotoCraft's rotated-ruler cases: rotations, both flips, and stretched pixels.
fn views() -> Vec<CanvasView> {
    let mut out = Vec::new();
    for rotation in [0.0, 30.0, 45.0, 90.0, -119.0, 180.0, 270.0] {
        for flip_x in [false, true] {
            for aspect in [1.0, 0.5, 2.0] {
                out.push(CanvasView { rotation, flip_x, aspect, ..CanvasView::new(screen(), Point::new(1000.0, 700.0), 0.5) });
            }
        }
    }
    out
}

fn near(a: Pos2, b: Pos2) -> bool {
    a.distance(b) < 1e-2
}

#[test]
fn to_doc_inverts_to_screen_across_rotation_flip_and_aspect() {
    for view in views() {
        for p in [Point::new(0.0, 0.0), Point::new(300.0, 120.0), Point::new(1000.0, 700.0), Point::new(-250.5, 1999.25)] {
            let back = view.to_doc(view.to_screen(p));
            assert!((back - p).hypot() < 1e-2, "{view:?}: {p:?} came back as {back:?}");
            assert!(near(view.to_screen(p), (view.affine() * p).to_egui_pos()), "{view:?}: affine disagrees at {p:?}");
        }
        let v = Vec2::new(12.0, -7.5);
        assert!((view.unmap_vec(view.map_vec(v)) - v).hypot() < 1e-4, "{view:?}: map_vec round trip");
        // A document vector moves its end point by its mapped vector.
        let p = Point::new(40.0, 60.0);
        assert!(near(view.to_screen(p + v), view.to_screen(p) + view.map_vec(v)), "{view:?}: map_vec agrees with to_screen");
    }
}

#[test]
fn the_centre_is_drawn_at_the_rect_centre_and_flip_mirrors_the_screen() {
    let view = CanvasView::new(screen(), Point::new(1000.0, 700.0), 0.5);
    assert!(near(view.to_screen(view.center), screen().center()));
    // Upright, unflipped: +x goes right, +y down, zoom × aspect along x.
    let wide = CanvasView { aspect: 2.0, ..view };
    assert!(near(wide.to_screen(Point::new(1100.0, 700.0)), screen().center() + vec2(100.0, 0.0)));
    assert_eq!(wide.zoom_x(), 1.0);
    let flipped = CanvasView { flip_x: true, ..view };
    assert!(near(flipped.to_screen(Point::new(1100.0, 700.0)), screen().center() - vec2(50.0, 0.0)));
    // Clockwise: +x points down after a quarter turn.
    let turned = CanvasView { rotation: 90.0, ..view };
    assert!(near(turned.to_screen(Point::new(1100.0, 700.0)), screen().center() + vec2(0.0, 50.0)));
    assert!(turned.axis_aligned() && !CanvasView { rotation: 30.0, ..view }.axis_aligned());
}

#[test]
fn quad_and_visible_follow_the_view() {
    for view in views() {
        let r = Rect::new(100.0, 200.0, 500.0, 260.0);
        let quad = view.quad(r);
        for (corner, p) in quad.iter().zip([Point::new(100.0, 200.0), Point::new(500.0, 200.0), Point::new(500.0, 260.0), Point::new(100.0, 260.0)]) {
            assert!(near(*corner, view.to_screen(p)), "{view:?}");
        }
        let visible = view.visible();
        for corner in [screen().left_top(), screen().right_top(), screen().right_bottom(), screen().left_bottom()] {
            let d = view.to_doc(corner);
            assert!(visible.inflate(1e-6, 1e-6).contains(d), "{view:?}: {d:?} outside {visible:?}");
        }
    }
    let view = CanvasView::new(screen(), Point::new(1000.0, 700.0), 0.5);
    assert_eq!(view.visible(), Rect::new(200.0, 100.0, 1800.0, 1300.0));
    assert_eq!(view.doc_len(4.0), 8.0);
}

/// PhotoCraft's `ruler_xform` cases (#3233, #2955): the level view keeps the zoom, flip and
/// aspect, drops the rotation, and draws the zero point where the rotated view does.
#[test]
fn level_keeps_the_zero_points_screen_position() {
    for view in views() {
        for zero in [Point::new(0.0, 0.0), Point::new(300.0, 120.0)] {
            let level = view.level(zero);
            assert_eq!((level.rotation, level.zoom, level.flip_x, level.aspect), (0.0, view.zoom, view.flip_x, view.aspect));
            assert!(
                near(level.to_screen(zero), view.to_screen(zero)),
                "{view:?}: zero at {:?}, want {:?}",
                level.to_screen(zero),
                view.to_screen(zero)
            );
            // 500 document units further on are 500 × zoom (× aspect along x) screen points away.
            let dir = if view.flip_x { -1.0 } else { 1.0 };
            let x = level.to_screen(zero + Vec2::new(500.0, 0.0)) - level.to_screen(zero);
            let y = level.to_screen(zero + Vec2::new(0.0, 500.0)) - level.to_screen(zero);
            assert!((x.x - dir * (250.0 * view.aspect) as f32).abs() < 1e-2 && x.y.abs() < 1e-2, "{view:?}: x scale {x:?}");
            assert!((y.y - 250.0).abs() < 1e-2 && y.x.abs() < 1e-2, "{view:?}: y scale {y:?}");
            // The top ruler spans the same length of document as the upright view.
            let span = level.to_doc(screen().right_top()).x - level.to_doc(screen().left_top()).x;
            assert!((span.abs() - 1600.0 / view.aspect).abs() < 1e-2, "{view:?}: span {span}");
        }
    }
}

#[test]
fn resize_cursors_turn_with_the_view() {
    use CursorIcon::{ResizeHorizontal as H, ResizeNeSw as NeSw, ResizeNwSe as NwSe, ResizeVertical as V};
    let level = CanvasView::new(screen(), Point::new(400.0, 300.0), 1.0);
    // A vertical guide moves along x, a horizontal one along y.
    for (rotation, flip_x, want) in [(0.0, false, [H, V]), (90.0, false, [V, H]), (45.0, false, [NwSe, NeSw]), (0.0, true, [H, V])] {
        let view = CanvasView { rotation, flip_x, ..level };
        let got = [resize_cursor(&view, Vec2::new(1.0, 0.0)), resize_cursor(&view, Vec2::new(0.0, 1.0))];
        assert_eq!(got, want, "view turned {rotation}°, flip {flip_x}");
    }
}

#[test]
fn hit_distances_are_to_the_drawn_geometry() {
    let (a, b) = (pos2(0.0, 0.0), pos2(100.0, 100.0));
    assert!((hit::segment(pos2(50.0, 54.0), a, b) - 2.0 * 2f32.sqrt()).abs() < 1e-4);
    assert_eq!(hit::segment(pos2(-3.0, -4.0), a, b), 5.0, "past the end: to the end point");
    assert_eq!(hit::point(pos2(3.0, 4.0), a), 5.0);
    let square = [pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(10.0, 10.0), pos2(0.0, 10.0)];
    assert_eq!(hit::polyline(pos2(-2.0, 5.0), &square, true), 2.0);
    assert_eq!(hit::polyline(pos2(-2.0, 5.0), &square, false), 29f32.sqrt(), "open: no edge back to the start");
    assert_eq!(hit::polyline(pos2(0.0, 0.0), &[], false), f32::INFINITY);
}

fn mesh_rect(shape: &Shape) -> ScreenRect {
    let Shape::Mesh(mesh) = shape else { panic!("not a mesh: {shape:?}") };
    mesh.calc_bounds()
}

#[test]
fn crisp_lines_cover_whole_device_pixels() {
    let stroke = Stroke::new(1.0, egui::Color32::WHITE);
    // PhotoCraft's `round() + 0.5` at 1 pixel per point: the pixel row from the rounded position.
    assert_eq!(
        mesh_rect(&crisp_shape(pos2(10.0, 10.7), pos2(50.0, 10.7), stroke, 1.0)),
        ScreenRect::from_min_max(pos2(10.0, 11.0), pos2(50.0, 12.0))
    );
    for ppp in [1.0, 1.25, 1.5, 2.0] {
        for y in [10.0, 10.3, 33.7] {
            let r = mesh_rect(&crisp_shape(pos2(5.3, y), pos2(60.1, y), stroke, ppp));
            let device = |v: f32| v * ppp;
            assert!([r.min.x, r.max.x, r.min.y, r.max.y].iter().all(|v| (device(*v) - device(*v).round()).abs() < 1e-3), "ppp {ppp}: {r:?}");
            assert_eq!(device(r.height()).round(), (ppp).round().max(1.0), "ppp {ppp}: one point is whole device pixels");
            assert!((r.center().y - y).abs() <= 1.0 / ppp, "ppp {ppp}: the line stays at {y}: {r:?}");
        }
        let r = mesh_rect(&crisp_shape(pos2(7.2, 40.0), pos2(7.2, 0.0), Stroke::new(2.0, egui::Color32::WHITE), ppp));
        assert_eq!((r.width() * ppp).round(), (2.0 * ppp).round(), "ppp {ppp}: vertical, 2 points");
    }
    assert!(matches!(crisp_shape(pos2(0.0, 0.0), pos2(10.0, 10.0), stroke, 1.5), Shape::LineSegment { .. }), "slanted lines stay antialiased");
}

/// One pass with `events`, calling `run` with the context.
fn pass(ctx: &Context, events: Vec<Event>, run: impl FnOnce(&Context)) {
    let input = RawInput { screen_rect: Some(screen()), events, ..RawInput::default() };
    let mut run = Some(run);
    ctx.run_ui(input, |ui| {
        if let Some(run) = run.take() {
            run(ui.ctx());
        }
    })
    .drop_without_applying_deltas();
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

#[test]
fn a_claim_holds_from_the_press_through_the_release_pass() {
    let ctx = Context::default();
    let canvas = Id::unique("claim-canvas");
    let (guides, tool) = (Id::unique("guides"), Id::unique("tool"));
    let at = pos2(100.0, 100.0);
    pass(&ctx, vec![Event::PointerMoved(at)], |ctx| {
        assert!(!Claim::take(ctx, canvas, guides), "no press, nothing to take");
        assert_eq!(Claim::owner(ctx, canvas), None);
    });
    pass(&ctx, vec![button(at, true)], |ctx| {
        assert!(Claim::take(ctx, canvas, guides));
        assert!(!Claim::take(ctx, canvas, tool), "the second caller doesn't get it");
        assert!(Claim::take(ctx, canvas, guides), "the owner keeps it");
        assert_eq!(Claim::owner(ctx, Id::unique("other-canvas")), None);
    });
    pass(&ctx, vec![Event::PointerMoved(at + vec2(20.0, 0.0))], |ctx| assert!(Claim::is_owner(ctx, canvas, guides), "held during the drag"));
    pass(&ctx, vec![button(at + vec2(20.0, 0.0), false)], |ctx| assert!(Claim::is_owner(ctx, canvas, guides), "and in the release pass"));
    pass(&ctx, vec![], |ctx| assert_eq!(Claim::owner(ctx, canvas), None, "gone after the release"));
    pass(&ctx, vec![button(at, true)], |ctx| assert!(Claim::take(ctx, canvas, tool), "the next press is free"));
    // A press and release in one pass hold for that pass only; a new press frees an old claim.
    pass(&ctx, vec![button(at, false)], |_| {});
    pass(&ctx, vec![button(at, true), button(at, false)], |ctx| assert!(Claim::take(ctx, canvas, guides)));
    pass(&ctx, vec![button(at, true)], |ctx| {
        assert_eq!(Claim::owner(ctx, canvas), None);
        assert!(Claim::take(ctx, canvas, tool));
    });
}

#[test]
fn a_claim_belongs_to_its_primary_press() {
    let ctx = Context::default();
    let canvas = Id::unique("claim-canvas");
    let (guides, tool) = (Id::unique("guides"), Id::unique("tool"));
    let at = pos2(100.0, 100.0);
    let secondary = |pressed| Event::PointerButton { pos: at, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE };
    pass(&ctx, vec![Event::PointerMoved(at)], |_| {});
    // The press frame runs a second pass (a sizing pass): the claim neither ends nor goes to
    // another overlay there.
    let mut passes = 0;
    ctx.run_ui(RawInput { screen_rect: Some(screen()), events: vec![button(at, true)], ..RawInput::default() }, |ui| {
        passes += 1;
        let ctx = ui.ctx();
        if passes == 1 {
            assert!(Claim::take(ctx, canvas, guides));
            ctx.request_discard("test");
        } else {
            assert!(Claim::is_owner(ctx, canvas, guides), "the second pass keeps the claim");
            assert!(!Claim::take(ctx, canvas, tool));
        }
    })
    .drop_without_applying_deltas();
    assert_eq!(passes, 2);
    // A secondary press and release while the primary is held leave the claim alone.
    pass(&ctx, vec![secondary(true)], |ctx| {
        assert!(Claim::is_owner(ctx, canvas, guides), "a right-click during the drag");
        assert!(!Claim::take(ctx, canvas, tool));
    });
    pass(&ctx, vec![Event::PointerMoved(at + vec2(10.0, 0.0))], |ctx| assert!(Claim::is_owner(ctx, canvas, guides)));
    pass(&ctx, vec![secondary(false)], |ctx| assert!(Claim::is_owner(ctx, canvas, guides)));
    pass(&ctx, vec![Event::PointerMoved(at + vec2(20.0, 0.0))], |ctx| assert!(Claim::is_owner(ctx, canvas, guides)));
    pass(&ctx, vec![button(at + vec2(20.0, 0.0), false)], |ctx| assert!(Claim::is_owner(ctx, canvas, guides)));
    // A claim nobody looked at while its press ended and a new one began: gone.
    pass(&ctx, vec![button(at, true)], |ctx| assert!(Claim::take(ctx, canvas, guides)));
    pass(&ctx, vec![button(at, false)], |_| {});
    pass(&ctx, vec![button(at, true)], |_| {});
    pass(&ctx, vec![Event::PointerMoved(at + vec2(5.0, 0.0))], |ctx| assert_eq!(Claim::owner(ctx, canvas), None, "a press nobody saw begin"));
}

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

/// The nudge of one pass with `events`, and the arrow events left after it.
fn nudge_with(events: Vec<Event>) -> (Option<Vec2>, usize) {
    let ctx = Context::default();
    let mut out = (None, 0);
    let input = RawInput { screen_rect: Some(screen()), events, ..RawInput::default() };
    ctx.run_ui(input, |ui| {
        let moved = nudge::read(ui, nudge::STEP, nudge::BIG_STEP);
        out = (moved, ui.input(|i| i.events.iter().filter(|e| matches!(e, Event::Key { .. })).count()));
    })
    .drop_without_applying_deltas();
    out
}

#[test]
fn nudge_reads_one_arrow_along_the_document_axes() {
    let plain = Modifiers::NONE;
    assert_eq!(nudge_with(vec![key(Key::ArrowRight, plain)]), (Some(Vec2::new(1.0, 0.0)), 0));
    assert_eq!(nudge_with(vec![key(Key::ArrowLeft, plain)]), (Some(Vec2::new(-1.0, 0.0)), 0));
    assert_eq!(nudge_with(vec![key(Key::ArrowDown, plain)]), (Some(Vec2::new(0.0, 1.0)), 0));
    assert_eq!(nudge_with(vec![key(Key::ArrowUp, Modifiers::SHIFT)]), (Some(Vec2::new(0.0, -10.0)), 0));
    // One key per call: the second stays for the next reader.
    assert_eq!(nudge_with(vec![key(Key::ArrowUp, plain), key(Key::ArrowRight, plain)]), (Some(Vec2::new(0.0, -1.0)), 1));
    // Other chords and keys are left alone.
    assert_eq!(nudge_with(vec![key(Key::ArrowRight, Modifiers::COMMAND)]), (None, 1));
    assert_eq!(nudge_with(vec![key(Key::ArrowRight, Modifiers::ALT)]), (None, 1));
    assert_eq!(nudge_with(vec![key(Key::A, plain)]), (None, 1));
}

#[test]
fn nudge_with_alt_reports_alt_and_takes_alt_shift() {
    let read = |events: Vec<Event>| {
        let ctx = Context::default();
        let mut out = None;
        let input = RawInput { screen_rect: Some(screen()), events, ..RawInput::default() };
        ctx.run_ui(input, |ui| out = nudge::read_with_alt(ui, nudge::STEP, nudge::BIG_STEP)).drop_without_applying_deltas();
        out
    };
    assert_eq!(read(vec![key(Key::ArrowRight, Modifiers::NONE)]), Some((Vec2::new(1.0, 0.0), false)));
    assert_eq!(read(vec![key(Key::ArrowUp, Modifiers::SHIFT)]), Some((Vec2::new(0.0, -10.0), false)));
    assert_eq!(read(vec![key(Key::ArrowLeft, Modifiers::ALT)]), Some((Vec2::new(-1.0, 0.0), true)));
    assert_eq!(read(vec![key(Key::ArrowDown, Modifiers::ALT | Modifiers::SHIFT)]), Some((Vec2::new(0.0, 10.0), true)));
    assert_eq!(read(vec![key(Key::ArrowDown, Modifiers::COMMAND)]), None);
}

#[test]
fn the_turn_cursor_hides_the_pointer_and_paints_on_the_tooltip_layer() {
    let ctx = Context::default();
    crate::theme::install(&ctx, &crate::theme::Base::suite_default());
    let input = || RawInput { screen_rect: Some(screen()), ..RawInput::default() };
    let mut out = ctx.run_ui(input(), |ui| super::turn_cursor(ui.ctx(), pos2(100.0, 100.0), vec2(1.0, 1.0)));
    out.textures_delta.clear();
    assert_eq!(out.platform_output.cursor_icon, CursorIcon::None);
    assert!(!out.shapes.is_empty());
    let mut out = ctx.run_ui(input(), |ui| super::turn_cursor(ui.ctx(), pos2(100.0, 100.0), vec2(0.0, 0.0)));
    out.textures_delta.clear();
    assert_eq!(out.platform_output.cursor_icon, CursorIcon::Default, "nothing without a direction");
    assert!(out.shapes.is_empty());
}

#[test]
fn nudge_leaves_a_focused_text_field_its_arrows() {
    let ctx = Context::default();
    let mut text = String::from("abc");
    let mut moved = Some(Vec2::ZERO);
    for events in [vec![], vec![key(Key::ArrowLeft, Modifiers::NONE)]] {
        let input = RawInput { screen_rect: Some(screen()), events, ..RawInput::default() };
        ctx.run_ui(input, |ui| {
            let response = ui.text_edit_singleline(&mut text);
            response.request_focus();
            moved = nudge::read(ui, nudge::STEP, nudge::BIG_STEP);
        })
        .drop_without_applying_deltas();
    }
    assert_eq!(moved, None);
}

#[test]
fn readout_rows_put_labels_at_the_start_and_values_on_the_end_edge() {
    let style = with_ui(|ui| Readout::style(&Tokens::of(ui)));
    let rows = [(vec2(14.0, 12.0), vec2(40.0, 12.0)), (vec2(30.0, 12.0), vec2(22.0, 14.0))];
    let width = 2.0 * style.pad.x + 30.0 + style.col_gap + 40.0;
    let ltr = ReadoutLayout::new(&style, &rows, false);
    assert_eq!(ltr.size, vec2(width, 2.0 * style.pad.y + 12.0 + style.row_gap + 14.0));
    assert_eq!(ltr.rows[0], (pos2(style.pad.x, style.pad.y), pos2(width - style.pad.x - 40.0, style.pad.y)));
    let second = style.pad.y + 12.0 + style.row_gap;
    assert_eq!(ltr.rows[1], (pos2(style.pad.x, second + 1.0), pos2(width - style.pad.x - 22.0, second)), "centred on the taller one");
    let rtl = ReadoutLayout::new(&style, &rows, true);
    assert_eq!(rtl.size, ltr.size);
    assert_eq!(rtl.rows[0], (pos2(width - style.pad.x - 14.0, style.pad.y), pos2(style.pad.x, style.pad.y)));
    // Compact tokens: PhotoCraft's 16 × 18 offset, 7 × 4 padding, 12 between columns, 1 between rows.
    assert_eq!((style.offset, style.pad, style.col_gap, style.row_gap), (vec2(16.0, 18.0), vec2(7.0, 4.0), 12.0, 1.0));
}

#[test]
fn the_readout_is_a_tooltip_node_holding_its_rows() {
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut text = None;
    for _ in 0..2 {
        let output = ctx.run_ui(RawInput { screen_rect: Some(screen()), ..RawInput::default() }, |ui| {
            Readout::new("readout-test").row("W:", "120 px").row("H:", "80 px").show(ui.ctx(), pos2(100.0, 100.0));
        });
        let update = output.platform_output.accesskit_update.clone().expect("accesskit update");
        output.drop_without_applying_deltas();
        text = update.nodes.iter().find(|(_, n)| n.role() == egui::accesskit::Role::Tooltip).and_then(|(_, n)| n.label().map(str::to_owned));
    }
    assert_eq!(text.as_deref(), Some("W: 120 px\nH: 80 px"));
}

/// What `f` returns from inside one pass.
fn with_ui<R>(f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let ctx = Context::default();
    let mut f = Some(f);
    let mut out = None;
    ctx.run_ui(RawInput { screen_rect: Some(screen()), ..RawInput::default() }, |ui| {
        if let Some(f) = f.take() {
            out = Some(f(ui));
        }
    })
    .drop_without_applying_deltas();
    out.expect("one pass")
}

trait ToEguiPos {
    fn to_egui_pos(self) -> Pos2;
}

impl ToEguiPos for Point {
    fn to_egui_pos(self) -> Pos2 {
        pos2(self.x as f32, self.y as f32)
    }
}

#[test]
fn the_story_canvas_view_follows_its_args() {
    let mut args = super::story::PlainArgs::default();
    let view = with_ui(|ui| super::story::document(ui, args.view(), "Document").view);
    assert_eq!((view.zoom, view.rotation, view.flip_x, view.aspect), (1.0, 0.0, false, 1.0));
    assert_eq!(view.center, super::story::DOC.center());
    args.rotation = 30.0;
    let turned = with_ui(|ui| super::story::document(ui, args.view(), "Document").view);
    assert_eq!(turned.rotation, 30.0);
}

/// A press taken while a field's typing run has the focus waits for the run's ending: the claim
/// holds through the release and until the owner's turn, the turn comes once and after the run's
/// Commit, and the claim ends with it. An owner that stops asking loses the claim.
#[test]
fn a_claimed_press_waits_for_a_typing_run_and_holds_through_the_wait() {
    let ctx = Context::default();
    let canvas = Id::unique("claim-canvas");
    let (overlay, tool, field) = (Id::unique("overlay"), Id::unique("tool"), Id::unique("field"));
    let at = pos2(100.0, 100.0);
    let pass_nr = std::cell::Cell::new(0);
    let committed = std::cell::Cell::new(None);
    let mut turns = Vec::new();
    // The field is drawn in every pass and has the focus only in the first two.
    let run = |events: Vec<Event>, ask: bool, turns: &mut Vec<Turn>| {
        pass_nr.set(pass_nr.get() + 1);
        let nr = pass_nr.get();
        let input = RawInput { screen_rect: Some(screen()), events, ..RawInput::default() };
        ctx.run_ui(input, |ui| {
            let response = ui.interact(ScreenRect::from_min_size(pos2(300.0, 300.0), vec2(20.0, 20.0)), field, egui::Sense::click());
            if nr <= 2 {
                response.request_focus();
            }
            let mut gesture = Gesture::load(ui, field, &0, EditPolicy::default());
            if nr == 1 {
                gesture.begin();
            }
            if matches!(gesture.finish(), Some(crate::edit::EditEvent::Commit { .. })) {
                committed.set(Some(nr));
            }
            let ctx = ui.ctx();
            if nr == 2 {
                assert!(Claim::take(ctx, canvas, overlay));
                assert!(!Claim::take(ctx, canvas, tool));
            }
            if ask {
                turns.push(Claim::turn(ctx, canvas, overlay));
            }
        })
        .drop_without_applying_deltas();
    };
    run(vec![Event::PointerMoved(at)], false, &mut turns);
    run(vec![button(at, true)], true, &mut turns);
    run(vec![button(at, false)], true, &mut turns);
    assert!(Claim::is_owner(&ctx, canvas, overlay), "held through the release while it waits");
    assert_eq!(turns[0], Turn::Waits);
    while turns.last() == Some(&Turn::Waits) && turns.len() < 6 {
        run(vec![], true, &mut turns);
    }
    assert_eq!(turns.last(), Some(&Turn::Runs), "{turns:?}");
    assert!(committed.get().is_some_and(|c| c < pass_nr.get()), "the run's Commit went out first: {:?} {}", committed.get(), pass_nr.get());
    run(vec![], true, &mut turns);
    assert_eq!(turns.last(), Some(&Turn::Now), "it runs once");
    assert_eq!(Claim::owner(&ctx, canvas), None, "a released press's claim ends with its turn");
}

/// A second press while the first waits supersedes it: the owner's turn is `Now`, its waiting
/// press no longer holds the claim, and the new press can be taken in the same pass.
#[test]
fn a_press_that_supersedes_a_waiting_one_is_taken_in_its_own_pass() {
    let ctx = Context::default();
    let canvas = Id::unique("claim-canvas");
    let (overlay, field) = (Id::unique("overlay"), Id::unique("field"));
    let at = pos2(100.0, 100.0);
    let mut nr = 0;
    let mut run = |events: Vec<Event>, check: &dyn Fn(&Context)| {
        nr += 1;
        let input = RawInput { screen_rect: Some(screen()), time: Some(f64::from(nr)), events, ..RawInput::default() };
        ctx.run_ui(input, |ui| {
            let response = ui.interact(ScreenRect::from_min_size(pos2(300.0, 300.0), vec2(20.0, 20.0)), field, egui::Sense::click());
            let mut gesture = Gesture::load(ui, field, &0, EditPolicy::default());
            if nr == 1 {
                response.request_focus();
                gesture.begin();
            }
            let _ = gesture.finish();
            check(ui.ctx());
        })
        .drop_without_applying_deltas();
    };
    run(vec![Event::PointerMoved(at)], &|_| {});
    run(vec![button(at, true)], &|ctx| assert!(Claim::take(ctx, canvas, overlay)));
    assert!(Claim::is_owner(&ctx, canvas, overlay));
    run(vec![button(at, false), button(at, true)], &|ctx| {
        assert_eq!(Claim::turn(ctx, canvas, overlay), Turn::Now, "the superseded press's turn is over");
        assert!(!Claim::keeps_waiting(ctx, canvas, overlay, true), "so the overlay takes the new press now");
        assert!(Claim::take(ctx, canvas, overlay), "which it can take in this pass");
    });
    assert!(Claim::is_owner(&ctx, canvas, overlay));
}

/// A text field's typing run beside a canvas under test: begun with the focus in the first pass it
/// is drawn, ended by whatever takes the focus. It records the pass of its Commit and the pass of
/// the overlay's first event, so a test can tell which went out first.
#[derive(Default)]
pub(super) struct TypingField {
    pub on: bool,
    passes: usize,
    began: bool,
    committed: Option<usize>,
    first_event: Option<usize>,
}

impl TypingField {
    /// Draws the field; call it in every pass, before the overlay.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.passes += 1;
        if !self.on {
            return;
        }
        let first = !self.began;
        self.began = true;
        let id = ui.make_persistent_id("typing field");
        let response = ui.interact(ScreenRect::from_min_size(pos2(0.0, 0.0), vec2(20.0, 20.0)), id, egui::Sense::click());
        if first {
            response.request_focus();
        }
        let mut gesture = Gesture::load(ui, id, &0, EditPolicy::default());
        if first {
            gesture.begin();
        }
        if matches!(gesture.finish(), Some(crate::edit::EditEvent::Commit { .. })) {
            self.committed = Some(self.passes);
        }
    }

    /// Records the overlay's result of this pass.
    pub fn note(&mut self, event: bool) {
        if event && self.first_event.is_none() {
            self.first_event = Some(self.passes);
        }
    }

    /// The field's Commit went out in a pass before the overlay's first event.
    pub fn commit_came_first(&self) -> bool {
        matches!((self.committed, self.first_event), (Some(c), Some(e)) if c < e)
    }
}

/// A new press taken in the pass after a waiting press was superseded gets its own turn: the old
/// press's queue entry is not its entry, and the queue is empty once it has run.
#[test]
fn a_new_press_does_not_inherit_the_queue_entry_of_a_superseded_one() {
    let ctx = Context::default();
    let canvas = Id::unique("claim-canvas");
    let (overlay, field) = (Id::unique("overlay"), Id::unique("field"));
    let at = pos2(100.0, 100.0);
    let nr = std::cell::Cell::new(0);
    let committed = std::cell::Cell::new(None);
    let run = |events: Vec<Event>, take: bool, turns: &mut Vec<Turn>| {
        nr.set(nr.get() + 1);
        let input = RawInput { screen_rect: Some(screen()), events, ..RawInput::default() };
        ctx.run_ui(input, |ui| {
            let response = ui.interact(ScreenRect::from_min_size(pos2(300.0, 300.0), vec2(20.0, 20.0)), field, egui::Sense::click());
            if nr.get() <= 2 {
                response.request_focus();
            }
            let mut gesture = Gesture::load(ui, field, &0, EditPolicy::default());
            if nr.get() == 1 {
                gesture.begin();
            }
            if matches!(gesture.finish(), Some(crate::edit::EditEvent::Commit { .. })) {
                committed.set(Some(nr.get()));
            }
            if take {
                assert!(Claim::take(ui.ctx(), canvas, overlay));
            }
            turns.push(Claim::turn(ui.ctx(), canvas, overlay));
        })
        .drop_without_applying_deltas();
    };
    let mut turns = Vec::new();
    run(vec![Event::PointerMoved(at)], false, &mut turns);
    run(vec![button(at, true)], true, &mut turns);
    run(vec![button(at, false)], false, &mut turns);
    assert_eq!(turns.last(), Some(&Turn::Waits));
    // A second press while the first still waits: the first's claim is superseded.
    turns.clear();
    run(vec![button(at, true)], true, &mut turns);
    let mut runs = 0;
    for _ in 0..8 {
        run(vec![], false, &mut turns);
        runs += turns.iter().filter(|&&t| t == Turn::Runs).count();
        turns.clear();
    }
    assert_eq!(runs, 1, "the second press runs once");
    assert!(committed.get().is_some());
    ctx.run_ui(RawInput { screen_rect: Some(screen()), ..RawInput::default() }, |ui| {
        assert_eq!(crate::edit::order::turn(ui.ctx(), Id::unique("probe"), None), Turn::Now, "no entry is left");
    })
    .drop_without_applying_deltas();
}
