//! Tests of the selection overlay (task SE-F7), with a `MaskSource` over a disc that counts its row
//! reads, shown on a plain egui `Context`.

use std::cell::Cell;
use std::time::Duration;

use craft_ui::canvas::CanvasView;
use craft_ui::canvas::kurbo::Point;
use craft_ui::localized::{LocaleNumbers, Unit};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Context, RawInput, Rect, Sense, Shape, TextureId, Ui, vec2};

use super::{OutlineSource, SelectionOverlay, SelectionOverlayResponse};
use crate::selection::{PreviewMode, QuickMaskLook};
use crate::trace::{MaskRect, MaskSource, Segment};

/// A disc of coverage 1 in a canvas of `size`, counting the rows it was asked for.
struct Disc {
    size: [u32; 2],
    centre: [i32; 2],
    radius: i32,
    revision: Cell<u64>,
    reads: Cell<usize>,
}

impl Disc {
    fn new(size: [u32; 2], centre: [i32; 2], radius: i32) -> Self {
        Disc { size, centre, radius, revision: Cell::new(1), reads: Cell::new(0) }
    }
}

impl MaskSource for Disc {
    fn size(&self) -> [u32; 2] {
        self.size
    }
    fn revision(&self) -> u64 {
        self.revision.get()
    }
    fn content_bounds(&self) -> MaskRect {
        let ([cx, cy], r) = (self.centre, self.radius);
        if r <= 0 { MaskRect::default() } else { MaskRect::new(cx - r, cy - r, cx + r, cy + r) }
    }
    fn read_row(&self, y: i32, x0: i32, step: u32, out: &mut [f32]) {
        self.reads.set(self.reads.get() + 1);
        for (i, v) in out.iter_mut().enumerate() {
            let x = x0.saturating_add(i32::try_from(i as u64 * u64::from(step)).unwrap_or(i32::MAX));
            let (dx, dy) = (i64::from(x - self.centre[0]), i64::from(y - self.centre[1]));
            *v = if dx * dx + dy * dy <= i64::from(self.radius).pow(2) { 1.0 } else { 0.0 };
        }
    }
}

/// What one pass drew.
struct Pass {
    response: SelectionOverlayResponse,
    shapes: Vec<Shape>,
    delay: Duration,
    /// Sizes of the textures uploaded this pass.
    uploads: Vec<[usize; 2]>,
    nodes: Vec<(Role, String, String)>,
}

impl Pass {
    /// The shapes that are not a preview texture.
    fn ants(&self) -> Vec<&Shape> {
        self.shapes.iter().filter(|s| !is_preview(s)).collect()
    }
    fn previews(&self) -> usize {
        self.shapes.len() - self.ants().len()
    }
    fn ants_bounds(&self) -> Rect {
        self.ants().iter().fold(Rect::NOTHING, |r, s| r.union(s.visual_bounding_rect()))
    }
}

fn is_preview(shape: &Shape) -> bool {
    matches!(shape, Shape::Mesh(m) if m.texture_id != TextureId::default())
}

fn flatten(shape: Shape, out: &mut Vec<Shape>) {
    match shape {
        Shape::Vec(v) => v.into_iter().for_each(|s| flatten(s, out)),
        s => out.push(s),
    }
}

fn context() -> Context {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    ctx.enable_accesskit();
    ctx
}

/// A view of a 400 × 300 canvas at the top left of the screen showing `centre` at `zoom`.
fn view(centre: [f64; 2], zoom: f64) -> CanvasView {
    CanvasView::new(Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(400.0, 300.0)), Point::new(centre[0], centre[1]), zoom)
}

fn pass(ctx: &Context, view: &CanvasView, show: impl FnOnce(&mut Ui, &egui::Response, &CanvasView) -> SelectionOverlayResponse) -> Pass {
    let input = RawInput { screen_rect: Some(Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(500.0, 400.0))), ..RawInput::default() };
    let mut show = Some(show);
    let mut response = None;
    let mut output = ctx.run_ui(input, |ui| {
        let canvas = ui.allocate_rect(view.rect, Sense::hover());
        if let Some(show) = show.take() {
            response = Some(show(ui, &canvas, view));
        }
    });
    let Some(response) = response else { panic!("the overlay was not shown") };
    let mut shapes = Vec::new();
    for clipped in output.shapes {
        flatten(clipped.shape, &mut shapes);
    }
    let delay = output.viewport_output.values().next().map_or(Duration::MAX, |v| v.repaint_delay);
    let uploads = output.textures_delta.set.values().flat_map(|deltas| deltas.iter()).filter(|d| d.pos.is_none()).map(|d| d.image.size()).collect();
    output.textures_delta.clear();
    let nodes = output
        .platform_output
        .accesskit_update
        .map(|u| {
            u.nodes.iter().map(|(_, n)| (n.role(), n.label().unwrap_or_default().to_owned(), n.value().unwrap_or_default().to_owned())).collect()
        })
        .unwrap_or_default();
    Pass { response, shapes, delay, uploads, nodes }
}

fn show_disc<'a>(ctx: &Context, view: &CanvasView, disc: &'a Disc, build: impl FnOnce(SelectionOverlay<'a>) -> SelectionOverlay<'a>) -> Pass {
    pass(ctx, view, |ui, canvas, view| build(SelectionOverlay::new("sel", disc)).show(ui, canvas, view))
}

#[test]
fn overlay_retraces_only_when_revision_step_or_region_changes() {
    let ctx = context();
    let disc = Disc::new([4000, 4000], [2000, 2000], 120);
    fn same(o: SelectionOverlay<'_>) -> SelectionOverlay<'_> {
        o
    }

    let first = show_disc(&ctx, &view([2000.0, 2000.0], 1.0), &disc, same);
    assert!(first.response.retraced && disc.reads.get() > 0);
    assert!(!first.ants().is_empty(), "the ants are drawn");

    let reads = disc.reads.get();
    let panned = show_disc(&ctx, &view([2010.0, 2000.0], 1.0), &disc, same);
    assert!(!panned.response.retraced, "a small pan reuses the trace");
    assert_eq!(disc.reads.get(), reads, "and reads no rows");
    assert!(!panned.ants().is_empty());

    disc.revision.set(2);
    assert!(show_disc(&ctx, &view([2010.0, 2000.0], 1.0), &disc, same).response.retraced, "a new revision");
    assert!(!show_disc(&ctx, &view([2010.0, 2000.0], 1.0), &disc, same).response.retraced);

    assert!(show_disc(&ctx, &view([2010.0, 2000.0], 0.5), &disc, same).response.retraced, "a zoom that changes the step");
    assert!(show_disc(&ctx, &view([2310.0, 2000.0], 0.5), &disc, same).response.retraced, "a pan past 256 steps");
    assert!(show_disc(&ctx, &view([2310.0, 2000.0], 0.5), &disc, |o| o.reduction(2)).response.retraced, "a new reduction");
}

#[test]
fn overlay_offset_and_mapped_outline_follow_the_piece() {
    let ctx = context();
    let segments: [Segment; 2] = [([100, 100], [160, 100]), ([160, 100], [160, 140])];
    let v = view([200.0, 150.0], 1.0);
    let at = |offset: [i32; 2]| {
        pass(&ctx, &v, |ui, canvas, view| {
            let source = OutlineSource::Segments { segments: &segments, bounds: MaskRect::new(100, 100, 160, 140) };
            SelectionOverlay::new("seg", source).offset(offset).show(ui, canvas, view)
        })
    };
    let base = at([0, 0]).ants_bounds();
    let moved = at([20, 10]).ants_bounds();
    assert!(base.is_positive());
    assert_eq!((moved.min - base.min).round(), vec2(20.0, 10.0), "{base:?} -> {moved:?}");

    // A selection far outside the view is traced whole under a map, and the map brings it into
    // view; without the map nothing of it lies in the traced region.
    let disc = Disc::new([8000, 8000], [6000, 6000], 60);
    let v = view([100.0, 100.0], 1.0);
    let plain = show_disc(&ctx, &v, &disc, |o| o);
    assert!(plain.ants().iter().all(|s| !s.visual_bounding_rect().is_positive()), "outside the view");
    let shift = |p: Point| Point::new(p.x - 5900.0, p.y - 5900.0);
    let mapped = show_disc(&ctx, &v, &disc, |o| o.mapped(&shift));
    assert!(mapped.response.retraced);
    let b = mapped.ants_bounds();
    assert!(b.is_positive() && Rect::from_min_max(egui::pos2(130.0, 80.0), egui::pos2(270.0, 220.0)).contains_rect(b), "{b:?}");
}

#[test]
fn overlay_hides_ants_when_not_visible() {
    let ctx = context();
    let disc = Disc::new([400, 300], [200, 150], 60);
    let v = view([200.0, 150.0], 1.0);
    let look = QuickMaskLook::default();
    // Frames after the first, once the context asks only for what is drawn.
    for _ in 0..3 {
        show_disc(&ctx, &v, &disc, |o| o.preview(PreviewMode::Grayscale, look));
    }
    let shown = show_disc(&ctx, &v, &disc, |o| o.preview(PreviewMode::Grayscale, look));
    assert!(!shown.ants().is_empty() && shown.delay <= Duration::from_millis(100), "ants march and repaint: {:?}", shown.delay);

    let ctx = context();
    for _ in 0..3 {
        show_disc(&ctx, &v, &disc, |o| o.visible(false).preview(PreviewMode::Grayscale, look));
    }
    let hidden = show_disc(&ctx, &v, &disc, |o| o.visible(false).preview(PreviewMode::Grayscale, look));
    assert!(hidden.ants().is_empty(), "no ants");
    assert_eq!(hidden.previews(), 1, "the preview stays");
    assert!(hidden.delay > Duration::from_secs(60), "and no repaint is asked for: {:?}", hidden.delay);
}

#[test]
fn overlay_preview_texture_is_reduced_and_reused() {
    let ctx = context();
    let disc = Disc::new([3000, 1500], [1500, 750], 400);
    let v = view([1500.0, 750.0], 0.1);
    fn gray(o: SelectionOverlay<'_>) -> SelectionOverlay<'_> {
        o.preview(PreviewMode::Grayscale, QuickMaskLook::default())
    }
    let wanted = [1000, 500];
    let first = show_disc(&ctx, &v, &disc, gray);
    assert!(first.uploads.contains(&wanted), "ceil(3000 / 1024) = 3: {:?}", first.uploads);
    assert!(!show_disc(&ctx, &v, &disc, gray).uploads.contains(&wanted), "reused");
    assert!(show_disc(&ctx, &v, &disc, |o| o.preview(PreviewMode::BlackMatte, QuickMaskLook::default())).uploads.contains(&wanted), "a new mode");
    let look = QuickMaskLook { opacity: 0.8, ..QuickMaskLook::default() };
    assert!(show_disc(&ctx, &v, &disc, |o| o.preview(PreviewMode::QuickMask, look)).uploads.contains(&wanted), "a new look");
    assert!(!show_disc(&ctx, &v, &disc, |o| o.preview(PreviewMode::QuickMask, look)).uploads.contains(&wanted));
    disc.revision.set(9);
    assert!(show_disc(&ctx, &v, &disc, |o| o.preview(PreviewMode::QuickMask, look)).uploads.contains(&wanted), "a new revision");
    // Reduced twice, the same source is the same size of texture but a new key.
    assert!(show_disc(&ctx, &v, &disc, |o| o.reduction(2).preview(PreviewMode::QuickMask, look)).uploads.contains(&wanted), "a new reduction");
}

#[test]
fn selection_overlay_accessible_value() {
    let ctx = context();
    let numbers = LocaleNumbers::from_tag("en");
    let unit = Unit::new("{n} mm");
    let value = |disc: &Disc, mode: PreviewMode| {
        let v = view([60.0, 40.0], 1.0);
        let p = show_disc(&ctx, &v, disc, |o| o.value_in(&numbers, &unit).preview(mode, QuickMaskLook::default()));
        let found = p.nodes.iter().find(|(role, label, _)| *role == Role::GraphicsObject && label == "Selection").map(|(_, _, v)| v.clone());
        let Some(found) = found else { panic!("no Selection node in {:?}", p.nodes) };
        found
    };
    let disc = Disc::new([200, 100], [60, 40], 20);
    assert_eq!(value(&disc, PreviewMode::None), "40 mm × 40 mm at 40 mm, 20 mm");
    assert_eq!(value(&disc, PreviewMode::Grayscale), "40 mm × 40 mm at 40 mm, 20 mm, Grayscale");
    let empty = Disc::new([200, 100], [60, 40], 0);
    assert_eq!(value(&empty, PreviewMode::None), "No selection");
}
