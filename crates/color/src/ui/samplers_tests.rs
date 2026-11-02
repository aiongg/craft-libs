//! Behaviour tests of `samplers`.

use craft_cms::convert::{CmykConverter, srgb_to_cmyk_naive};
use craft_theme::Srgba;

use craft_ui::composites::Tone;

use super::{ColorSamplerTool, ColorSamplers, MAX_SAMPLERS, SamplerError, SamplerReadout, SamplerReadouts};
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasTool, CanvasView, ToolHost, ToolInput};
use craft_ui::theme::{Base, install};
use egui::{Id, Modifiers, Pos2, Rect, Sense, vec2};
use egui_kittest::Harness;

/// A click places points up to ten and the eleventh reports Full.
#[test]
fn points_stop_at_ten() {
    let mut samplers = ColorSamplers::new();
    for i in 0..MAX_SAMPLERS {
        assert_eq!(samplers.add([i as f64 + 0.7, 3.2], [100, 100]), Ok(i));
    }
    assert_eq!(samplers.add([50.0, 50.0], [100, 100]), Err(SamplerError::Full));
    assert_eq!(samplers.points().len(), 10);
    // A position floors to its pixel; outside the canvas is refused before the limit applies.
    assert_eq!(samplers.points()[0], [0.0, 3.0]);
    assert_eq!(samplers.add([100.0, 5.0], [100, 100]), Err(SamplerError::Outside));
    assert_eq!(samplers.add([-0.5, 5.0], [100, 100]), Err(SamplerError::Outside));
}

/// Moving onto the canvas floors; dropping off it removes; removing renumbers; a hit picks the
/// nearest point within the tolerance.
#[test]
fn points_move_remove_and_hit() {
    let mut samplers = ColorSamplers::new();
    for x in [10.0, 20.0, 30.0] {
        samplers.add([x, 10.0], [100, 100]).unwrap();
    }
    samplers.move_point(1, [22.9, 11.1], [100, 100]).unwrap();
    assert_eq!(samplers.points()[1], [22.0, 11.0]);
    assert_eq!(samplers.hit([21.0, 11.0], 7.0), Some(1));
    assert_eq!(samplers.hit([60.0, 60.0], 7.0), None);
    samplers.move_point(1, [-3.0, 11.0], [100, 100]).unwrap();
    assert_eq!(samplers.points(), [[10.0, 10.0], [30.0, 10.0]]);
    samplers.remove(0).unwrap();
    assert_eq!(samplers.points(), [[30.0, 10.0]]);
    assert_eq!(samplers.remove(3), Err(SamplerError::NoSuchPoint));
    assert_eq!(samplers.move_point(3, [1.0, 1.0], [100, 100]), Err(SamplerError::NoSuchPoint));
}

/// The readout's CMYK comes from the given converter, as the picker's.
#[test]
fn readouts_use_the_documents_converter() {
    let converter = CmykConverter::coated();
    let blue = Srgba { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    let reading = SamplerReadout::new(2, [4.0, 9.0], blue, converter);
    assert_eq!((reading.number, reading.position, reading.rgb), (3, [4, 9], [0, 0, 255]));
    let managed = converter.from_srgb([0.0, 0.0, 1.0]).map(|v| (v * 100.0).round() as u8);
    assert_eq!(reading.cmyk, managed);
    let naive = srgb_to_cmyk_naive([0.0, 0.0, 1.0]).map(|v| (v * 100.0).round() as u8);
    assert_ne!(reading.cmyk, naive);
    // Channels that are not finite read as 0 and the rest is clamped.
    let odd = SamplerReadout::new(0, [0.0, 0.0], Srgba { r: f32::NAN, g: 2.0, b: -1.0, a: 1.0 }, converter);
    assert_eq!(odd.rgb, [0, 255, 0]);
}

/// The cached readouts depend on the document, the revision, the points and the CMYK profile:
/// another document at the same revision, or another CMYK working space, is read again.
#[test]
fn readouts_are_cached_per_document_and_converter() {
    use std::cell::Cell;
    use std::sync::Arc;
    let ctx = egui::Context::default();
    let mut samplers = ColorSamplers::new();
    samplers.add([1.0, 1.0], [10, 10]).unwrap();
    let reads = Cell::new(0);
    let read = |_: [f64; 2]| {
        reads.set(reads.get() + 1);
        Srgba { r: 0.0, g: 0.0, b: 1.0, a: 1.0 }
    };
    let coated = CmykConverter::coated();
    let first = SamplerReadouts::new(&samplers, read).revision(3).document("a").cmyk(coated).get(&ctx);
    let again = SamplerReadouts::new(&samplers, read).revision(3).document("a").cmyk(coated).get(&ctx);
    assert!(Arc::ptr_eq(&first, &again) && reads.get() == 1, "unchanged: from the cache");
    SamplerReadouts::new(&samplers, read).revision(3).document("b").cmyk(coated).get(&ctx);
    assert_eq!(reads.get(), 2, "another document at the same revision reads again");
    let fogra = CmykConverter::new(Arc::new(craft_cms::Builtin::CoatedFogra39.profile().clone())).unwrap();
    let other = SamplerReadouts::new(&samplers, read).revision(3).document("a").cmyk(&fogra).get(&ctx);
    assert_eq!(reads.get(), 3, "another CMYK working space reads again");
    assert_ne!(other[0].cmyk, first[0].cmyk);
}

/// A canvas with a tool host over a 400 x 300 view at zoom 1 whose centre is document (200, 150),
/// so document points equal screen points.
struct Rig<T> {
    tool: T,
    id: craft_ui::canvas::CanvasId,
}

fn harness<T: CanvasTool + 'static>(tool: T) -> Harness<'static, Rig<T>> {
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, rig: &mut Rig<T>| {
            let view = CanvasView::new(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
            let canvas_id = craft_ui::canvas::CanvasId::new(ui, "canvas");
            let canvas = canvas_id.interact(ui, view.rect, Sense::click_and_drag());
            rig.id = ToolHost::new(canvas_id).show(ui, &view, &canvas, &mut rig.tool).id;
        },
        Rig { tool, id: craft_ui::canvas::CanvasId::from_id(Id::NULL) },
    );
    h.step();
    h
}

fn input<T>(h: &mut Harness<'_, Rig<T>>, input: ToolInput) {
    ToolHost::inject(&h.ctx, h.state().id, input);
    h.step();
}

fn point(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn down<T>(h: &mut Harness<'_, Rig<T>>, at: Point, modifiers: Modifiers) {
    input(h, ToolInput::Down { button: egui::PointerButton::Primary, doc: at, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

fn move_to<T>(h: &mut Harness<'_, Rig<T>>, at: Point, modifiers: Modifiers) {
    input(h, ToolInput::Move { doc: at, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

fn up<T>(h: &mut Harness<'_, Rig<T>>, at: Point, modifiers: Modifiers) {
    input(h, ToolInput::Up { doc: at, modifiers });
}

const SIZE: [u32; 2] = [300, 200];

/// A rig whose tool edits `samplers`, which the test keeps by pointer round-trip through the state.
fn sampler_rig(points: &[[f64; 2]]) -> Harness<'static, Rig<ColorSamplerTool<'static>>> {
    let samplers: &'static mut ColorSamplers = Box::leak(Box::new(ColorSamplers::new()));
    for p in points {
        samplers.add(*p, SIZE).unwrap();
    }
    harness(ColorSamplerTool::new(samplers, SIZE))
}

fn points(h: &Harness<'_, Rig<ColorSamplerTool<'static>>>) -> Vec<[f64; 2]> {
    h.state().tool.samplers().points().to_vec()
}

/// A point dragged off the canvas is removed; a release within half a pixel is a click.
#[test]
fn a_point_dragged_off_the_canvas_is_removed() {
    let mut h = sampler_rig(&[[50.0, 50.0], [100.0, 50.0]]);
    // A release within half a pixel of the press does nothing.
    down(&mut h, point(100.0, 50.0), Modifiers::NONE);
    up(&mut h, point(100.3, 50.0), Modifiers::NONE);
    assert_eq!(points(&h), [[50.0, 50.0], [100.0, 50.0]]);
    // Dropped inside the canvas it moves, onto the pixel.
    down(&mut h, point(100.0, 50.0), Modifiers::NONE);
    up(&mut h, point(150.6, 80.2), Modifiers::NONE);
    assert_eq!(points(&h), [[50.0, 50.0], [150.0, 80.0]]);
    // Dropped off the canvas it is removed and the other stays.
    down(&mut h, point(150.0, 80.0), Modifiers::NONE);
    move_to(&mut h, point(350.0, 80.0), Modifiers::NONE);
    up(&mut h, point(350.0, 80.0), Modifiers::NONE);
    assert_eq!(points(&h), [[50.0, 50.0]]);
    // A click on empty canvas places a point; outside the canvas none.
    down(&mut h, point(20.0, 20.0), Modifiers::NONE);
    up(&mut h, point(20.0, 20.0), Modifiers::NONE);
    down(&mut h, point(320.0, 20.0), Modifiers::NONE);
    up(&mut h, point(320.0, 20.0), Modifiers::NONE);
    assert_eq!(points(&h), [[50.0, 50.0], [20.0, 20.0]]);
    let ctx = h.ctx.clone();
    assert_eq!(h.state_mut().tool.take_error(&ctx), None);
}

/// Alt-click on a point removes it; the later points move up one number.
#[test]
fn alt_click_removes_and_renumbers() {
    let mut h = sampler_rig(&[[10.0, 10.0], [60.0, 10.0], [110.0, 10.0]]);
    down(&mut h, point(62.0, 12.0), Modifiers::ALT);
    up(&mut h, point(62.0, 12.0), Modifiers::ALT);
    assert_eq!(points(&h), [[10.0, 10.0], [110.0, 10.0]], "the third point is now number 2");
    // Alt-click on empty space does nothing.
    down(&mut h, point(200.0, 100.0), Modifiers::ALT);
    up(&mut h, point(200.0, 100.0), Modifiers::ALT);
    assert_eq!(points(&h).len(), 2);

    // The eleventh click reports Full.
    for i in 0..MAX_SAMPLERS {
        let at = point(10.0 + 20.0 * i as f64, 150.0);
        down(&mut h, at, Modifiers::NONE);
        up(&mut h, at, Modifiers::NONE);
    }
    assert_eq!(points(&h).len(), MAX_SAMPLERS);
    let ctx = h.ctx.clone();
    let line = h.state_mut().tool.take_error(&ctx).expect("the press past the limit reports");
    assert_eq!((line.text.as_str(), line.tone), ("Color Sampler limit reached: at most 10 points per document", Tone::Bad));
    assert_eq!(h.state_mut().tool.take_error(&ctx), None);
}
