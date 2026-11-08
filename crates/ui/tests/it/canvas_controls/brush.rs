//! The brush overlays: the cursor under each preference, an aliased tip's pixel cell under turned and mirrored views,
//! the resize gesture's chord and travel, and the cursor bitmap.

use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_behavior::{CanvasView, Cursor, GesturePhase, PaintedCursor, PointerButton};
use craft_keymap::Mods;
use craft_theme::Rgba;
use craft_ui::canvas::ToolContext;
use craft_ui::canvas_controls::brush::{
    BrushCursor, BrushCursorOptions, BrushDimensions, BrushResize, BrushResizeOptions, BrushTip, BrushTipOutline, CursorBitmap, CursorBitmapSpec,
    PaintingCursor, TipFalloff, TipPose,
};
use craft_ui::testing::trial;
use egui::{Pos2, Rect, pos2, vec2};

/// Screen points per document unit.
const ZOOM: f64 = 2.0;
/// The crosshair arm a cursor bitmap draws, in points.
const REACH: f32 = 4.0;
/// The tip: diameter 40 with its 50 % contour at radius 10 and full reach at radius 20.
const DIAMETER: f64 = 40.0;
const NORMAL: f64 = 10.0;
const MAX_DIAMETER: f64 = 400.0;
/// Rounding slack for lengths computed in f32.
const SLACK: f32 = 0.01;

fn view_at(zoom: f64) -> CanvasView {
    CanvasView::new(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)), Point::new(100.0, 100.0), zoom)
}

fn view() -> CanvasView {
    view_at(ZOOM)
}

/// A tool context with Caps Lock `caps_lock`; the crosshair that replaces a small outline takes its reach from the context.
fn cx(caps_lock: bool) -> ToolContext<'static> {
    ToolContext::new(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)), None).caps_lock(caps_lock)
}

fn options(painting: PaintingCursor) -> BrushCursorOptions {
    BrushCursorOptions::new(painting)
}

fn painted(cursor: Cursor) -> PaintedCursor {
    match cursor {
        Cursor::Painted(painted) => painted,
        other => panic!("expected a painted cursor, got {other:?}"),
    }
}

fn radius(cursor: &PaintedCursor) -> f32 {
    match cursor {
        PaintedCursor::Circle { radius, .. } => *radius,
        other => panic!("expected a circle, got {other:?}"),
    }
}

fn rings(cursor: &PaintedCursor) -> usize {
    match cursor {
        PaintedCursor::Outline { rings, .. } => rings.len(),
        other => panic!("expected an outline, got {other:?}"),
    }
}

#[trial]
fn the_painting_cursor_follows_the_preferences() {
    let tip = BrushTip::round(DIAMETER, NORMAL);
    let brush = BrushCursor::new(&tip);
    let view = view();
    let pick = |cursor: &BrushCursor<'_>, options: &BrushCursorOptions, caps: bool| painted(cursor.cursor(&cx(caps), options, &view));

    assert!(matches!(pick(&brush, &options(PaintingCursor::Standard), false), PaintedCursor::Tool { .. }));
    assert_eq!(pick(&brush, &options(PaintingCursor::Precise), false), PaintedCursor::Crosshair);
    let normal = radius(&pick(&brush, &options(PaintingCursor::NormalBrushTip), false));
    let full = radius(&pick(&brush, &options(PaintingCursor::FullSizeBrushTip), false));
    // The two outlines are in the proportion of the tip's radii, and zooming in scales them.
    assert!((full / normal - (DIAMETER / 2.0 / NORMAL) as f32).abs() < SLACK);
    let zoomed = BrushCursor::new(&tip).cursor(&cx(false), &options(PaintingCursor::NormalBrushTip), &view_at(2.0 * ZOOM));
    assert!((radius(&painted(zoomed)) / normal - 2.0).abs() < SLACK);

    // Caps Lock gives the crosshair over every preference, unless the preference turns it off.
    for painting in [PaintingCursor::Standard, PaintingCursor::NormalBrushTip] {
        assert_eq!(pick(&brush, &options(painting), true), PaintedCursor::Crosshair);
    }
    assert!(matches!(pick(&brush, &options(PaintingCursor::NormalBrushTip).caps_lock_precise(false), true), PaintedCursor::Circle { .. }));

    // Show Only Crosshair While Painting applies to a stroke only.
    let only = options(PaintingCursor::NormalBrushTip).show_only_crosshair_while_painting(true);
    assert_eq!(pick(&BrushCursor::new(&tip).painting(true), &only, false), PaintedCursor::Crosshair);
    assert!(matches!(pick(&brush, &only, false), PaintedCursor::Circle { .. }));
}

#[trial]
fn an_outline_shows_the_pose_the_view_and_the_inner_ring() {
    let tip = BrushTip::round(DIAMETER, NORMAL);
    let normal = options(PaintingCursor::NormalBrushTip);
    // A view with a pixel aspect and a skewed pose make the circle an ellipse.
    let wide = view().aspect(2.0);
    assert_eq!(rings(&painted(BrushCursor::new(&tip).cursor(&cx(false), &normal, &wide))), 1);
    let turned = TipPose::pen(30.0, Some([40.0, 0.0]));
    assert_eq!(rings(&painted(BrushCursor::new(&tip).pose(turned).cursor(&cx(false), &normal, &view()))), 1);
    // An inner ring is a second outline.
    let two = BrushCursor::new(&tip).inner_ring(NORMAL / 2.0).cursor(&cx(false), &normal, &view());
    assert_eq!(rings(&painted(two)), 2);
    // A tip too small to read on screen gives the crosshair.
    let small = BrushTip::round(1.0, 0.25);
    assert_eq!(painted(BrushCursor::new(&small).cursor(&cx(false), &normal, &view())), PaintedCursor::Crosshair);
    // A tip that is not a finite size never gives a non-finite ring.
    let broken = BrushTip::round(f64::NAN, f64::INFINITY);
    assert_eq!(painted(BrushCursor::new(&broken).cursor(&cx(false), &normal, &view())), PaintedCursor::Crosshair);
}

fn cell_tip(cell: DocRect, round: bool) -> BrushTip {
    BrushTip::new(cell.width(), BrushTipOutline::pixels(cell, round))
}

#[trial]
fn an_aliased_tip_hides_the_pointer_unless_it_is_too_small_or_asks_for_the_crosshair() {
    let normal = options(PaintingCursor::NormalBrushTip);
    let big = cell_tip(DocRect::new(10.0, 10.0, 15.0, 15.0), false);
    assert_eq!(painted(BrushCursor::new(&big).cursor(&cx(false), &normal, &view())), PaintedCursor::Hidden);
    assert_eq!(painted(BrushCursor::new(&big).centre(true).cursor(&cx(false), &normal, &view())), PaintedCursor::Crosshair);
    let single = cell_tip(DocRect::new(10.0, 10.0, 11.0, 11.0), false);
    assert_eq!(painted(BrushCursor::new(&single).cursor(&cx(false), &normal, &view())), PaintedCursor::Crosshair);
}

#[trial]
fn a_pixel_cell_follows_a_turned_and_a_mirrored_view_on_the_pixel_grid() {
    // A cell 4 wide and 2 tall to the right of the view's centre.
    let tip = cell_tip(DocRect::new(110.0, 99.0, 114.0, 101.0), false);
    let brush = BrushCursor::new(&tip);
    let base = view();
    let middle = base.rect.center();
    let cell = |view: &CanvasView| brush.pixel_cell(view, 1.0).expect("an aliased tip has a cell");

    let upright = cell(&base);
    assert_eq!(upright.width(), 2.0 * upright.height());
    assert!(upright.center().x > middle.x);

    let mirrored = cell(&base.flip_x(true));
    assert_eq!(mirrored.size(), upright.size());
    assert!(mirrored.center().x < middle.x);

    // Turned a quarter clockwise, the cell lies below the centre and is as tall as it was wide.
    let turned = cell(&base.rotation(90.0));
    assert_eq!(turned.size(), vec2(upright.height(), upright.width()));
    assert!(turned.center().y > middle.y);

    // On a display of two device pixels per point every edge lies on a half point.
    let sharp = brush.pixel_cell(&base.rotation(33.0), 2.0).expect("a cell");
    for edge in [sharp.min.x, sharp.min.y, sharp.max.x, sharp.max.y] {
        assert_eq!(edge, (edge * 2.0).round() / 2.0);
    }
    assert!(BrushCursor::new(&BrushTip::round(DIAMETER, NORMAL)).pixel_cell(&base, 1.0).is_none());
}

fn press(resize: &mut BrushResize, view: &CanvasView, start: BrushDimensions) -> Pos2 {
    let at = view.to_screen(Point::new(100.0, 100.0));
    resize.begin(view, at, start, &BrushResizeOptions::new(MAX_DIAMETER)).expect("a finite press begins a resize");
    at
}

#[trial]
fn a_resize_sets_the_diameter_across_and_the_level_down_in_screen_directions() {
    let view = view();
    let start = BrushDimensions::new(DIAMETER, 0.5);
    let mut resize = BrushResize::default();
    let at = press(&mut resize, &view, start);
    assert!(resize.active());

    // Thirty points right at a zoom of two is fifteen document units of radius.
    let step = resize.track(&view, at + vec2(30.0, 0.0)).expect("the diameter changed");
    assert_eq!(step.phase, GesturePhase::Update);
    assert_eq!(step.dimensions, BrushDimensions::new(DIAMETER + 30.0, 0.5));
    // The same point again changes nothing.
    assert!(resize.track(&view, at + vec2(30.0, 0.0)).is_none());
    // Forty points down is a fifth of the travel, whatever the zoom.
    let step = resize.track(&view, at + vec2(30.0, 40.0)).expect("the level changed");
    assert_eq!(step.dimensions, BrushDimensions::new(DIAMETER + 30.0, 0.7));

    // The values stay in range.
    let far = resize.track(&view, at + vec2(1.0e6, 1.0e6)).expect("clamped values differ");
    assert_eq!(far.dimensions, BrushDimensions::new(MAX_DIAMETER, 1.0));
    let near = resize.track(&view, at + vec2(-1.0e6, -1.0e6)).expect("clamped values differ");
    assert_eq!(near.dimensions, BrushDimensions::new(1.0, 0.0));

    let end = resize.finish(&view, at + vec2(30.0, 0.0)).expect("a resize ends with a commit");
    assert_eq!((end.phase, end.changed), (GesturePhase::Commit, true));
    assert!(!resize.active() && resize.finish(&view, at).is_none());
}

#[trial]
fn a_resize_released_where_it_began_commits_nothing_and_a_cancel_restores_the_start() {
    let view = view();
    let start = BrushDimensions::new(DIAMETER, 0.5);
    let mut resize = BrushResize::default();
    let at = press(&mut resize, &view, start);
    let click = resize.finish(&view, at).expect("commit");
    assert_eq!((click.phase, click.changed, click.dimensions), (GesturePhase::Commit, false, start));

    let at = press(&mut resize, &view, start);
    resize.track(&view, at + vec2(20.0, 0.0));
    let cancel = resize.cancel().expect("cancel");
    assert_eq!((cancel.phase, cancel.dimensions), (GesturePhase::Cancel, start));
    assert!(!resize.active() && resize.cancel().is_none());

    // A press the view cannot map is not a resize.
    assert!(resize.begin(&view, pos2(f32::NAN, 0.0), start, &BrushResizeOptions::new(MAX_DIAMETER)).is_none());
    assert!(!resize.active());
}

#[trial]
fn the_resize_chord_is_alt_with_the_secondary_button_or_control_and_alt_with_the_primary() {
    let alt = Mods::ALT;
    let control_alt = Mods { ctrl: true, alt: true, ..Mods::NONE };
    assert!(BrushResize::chord(PointerButton::Secondary, alt));
    assert!(BrushResize::chord(PointerButton::Primary, control_alt));
    assert!(!BrushResize::chord(PointerButton::Primary, alt));
    assert!(!BrushResize::chord(PointerButton::Secondary, Mods::NONE));
    // The tool takes the secondary button only with Alt, so a plain right click stays the canvas menu's.
    assert!(BrushResize::takes(PointerButton::Primary, Mods::NONE));
    assert!(BrushResize::takes(PointerButton::Secondary, alt));
    assert!(!BrushResize::takes(PointerButton::Secondary, Mods::NONE));
}

#[trial]
fn a_falloff_is_solid_to_its_core_and_empty_at_its_reach() {
    let soft = TipFalloff::from_hardness(20.0, 0.5);
    assert_eq!((soft.core(), soft.reach()), (10.0, 20.0));
    let hard = TipFalloff::hard(20.0);
    assert_eq!((hard.core(), hard.reach()), (20.0, 20.0));
    let empty = TipFalloff::from_hardness(f64::NAN, 0.5);
    assert_eq!((empty.core(), empty.reach()), (0.0, 0.0));
}

#[trial]
fn a_cursor_bitmap_draws_the_ring_around_its_hotspot_and_only_where_it_fits() {
    const SCALE: f32 = 2.0;
    const RADIUS: f32 = 10.0;
    const STROKE: f32 = 1.0;
    const MAX_SIDE: u16 = 255;
    let spec = CursorBitmapSpec::new(SCALE, Rgba([255; 4]), Rgba([0, 0, 0, 255]), STROKE, REACH, MAX_SIDE);
    let bitmap = CursorBitmap::render(&PaintedCursor::circle(RADIUS), &spec).expect("a small circle has a bitmap");
    let [width, height] = bitmap.size;
    assert_eq!(width, height);
    assert_eq!(width % 2, 1, "the hotspot lies on the centre pixel");
    assert_eq!(bitmap.hotspot, [width / 2, height / 2]);
    let alpha = |col: u16, row: u16| bitmap.rgba[(usize::from(row) * usize::from(width) + usize::from(col)) * 4 + 3];
    let (hot_col, hot_row) = (bitmap.hotspot[0], bitmap.hotspot[1]);
    assert_eq!(alpha(hot_col, hot_row), 0, "the centre of a circle is empty");
    assert!(alpha(hot_col + (RADIUS * SCALE) as u16, hot_row) > 0, "the ring is inked");

    // A circle larger than the limit stays painted, and so does a cursor that is not a shape.
    assert!(CursorBitmap::render(&PaintedCursor::circle(RADIUS * 100.0), &spec).is_none());
    assert!(CursorBitmap::render(&PaintedCursor::Hidden, &spec).is_none());
    let crosshair = CursorBitmap::render(&PaintedCursor::Crosshair, &spec).expect("a crosshair has a bitmap");
    assert!(crosshair.size[0] > 0);
}
