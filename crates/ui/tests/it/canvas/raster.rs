//! `RasterDisplay` and `Thumbnails`: what is read again, the detail plan, the fit maths and the request rules.

use craft_behavior::kurbo::Point;
use craft_behavior::{CanvasView, PixelSize};
use craft_ui::canvas::{Canvas, CanvasSpace, DetailPlan, PixelRect, RasterDisplay, RasterSource, ToolPath, snap_to_texels};
use craft_ui::contract::{Show as _, ShowContainer as _};
use craft_ui::primitives::textures::{FakeThumbSource, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails, fit_rect, thumb_image};
use craft_ui::primitives::{Priority, Revision, TextureState, texture_factor, texture_side};
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Color32, ColorImage, Pos2, Rect, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The canvas the display is shown in, in points.
const CANVAS: Vec2 = Vec2::new(800.0, 600.0);

/// A document that records every read.
struct Document {
    size: PixelSize,
    revision: Cell<u64>,
    reads: RefCell<Vec<(PixelRect, u32)>>,
}

impl Document {
    fn new(width: u32, height: u32) -> Rc<Self> {
        Rc::new(Document { size: PixelSize::new(width, height), revision: Cell::new(1), reads: RefCell::new(Vec::new()) })
    }
}

impl RasterSource for Document {
    fn size(&self) -> PixelSize {
        self.size
    }

    fn revision(&self) -> Revision {
        Revision::new(self.revision.get())
    }

    fn read(&self, region: PixelRect, factor: u32, out: &mut ColorImage) {
        self.reads.borrow_mut().push((region, factor));
        out.pixels.fill(Color32::from_gray(40));
    }
}

/// What a display reported in a pass: the pixels it read and the detail plan it used.
type Reported = (Option<PixelRect>, Option<DetailPlan>);

/// A display of `document` in a canvas looking at the document's centre at `zoom`.
struct Rig {
    fixture: Fixture,
    document: Rc<Document>,
    damage: Rc<Cell<PixelRect>>,
    reported: Rc<RefCell<Option<Reported>>>,
    space: Rc<RefCell<Option<CanvasSpace>>>,
}

fn rig(document: Rc<Document>, zoom: f64) -> Rig {
    let center = Point::new(f64::from(document.size.width) / 2.0, f64::from(document.size.height) / 2.0);
    let view = CanvasView::new(Rect::from_min_size(Pos2::ZERO, CANVAS), center, zoom);
    let damage = Rc::new(Cell::new(PixelRect::default()));
    let reported = Rc::new(RefCell::new(None));
    let space = Rc::new(RefCell::new(None));
    let (shown_document, shown_damage, shown_reported, shown_space) =
        (Rc::clone(&document), Rc::clone(&damage), Rc::clone(&reported), Rc::clone(&space));
    let mut fixture = Fixture::new(move |ui| {
        let canvas = Canvas::new("Document", ToolPath::Freehand).view(view).show(ui, |ui, body| {
            let output = RasterDisplay::new("Picture", &*shown_document).damaged(shown_damage.get()).checkerboard().show_at(ui, body.rect);
            *shown_reported.borrow_mut() = Some((output.read, output.detail));
        });
        *shown_space.borrow_mut() = canvas.output.space;
    });
    fixture.room(CANVAS);
    fixture.settle();
    Rig { fixture, document, damage, reported, space }
}

impl Rig {
    /// One frame with the damage given, and the reads it made.
    fn frame(&mut self, revision: u64, damage: PixelRect) -> Vec<(PixelRect, u32)> {
        self.document.revision.set(revision);
        self.damage.set(damage);
        self.document.reads.borrow_mut().clear();
        self.fixture.frame(Vec::new());
        self.damage.set(PixelRect::default());
        self.document.reads.borrow().clone()
    }
}

#[trial]
fn only_damaged_pixels_are_read_again() {
    let size = PixelSize::new(400, 300);
    let mut rig = rig(Document::new(size.width, size.height), 1.0);
    assert!(rig.frame(1, PixelRect::default()).is_empty(), "nothing changed");

    let rect = PixelRect::new(10, 20, 50, 60);
    let reads = rig.frame(2, rect);
    assert!(!reads.is_empty() && reads.iter().all(|read| *read == (rect, 1)), "damage is read alone: {reads:?}");

    let reads = rig.frame(3, PixelRect::default());
    let whole = PixelRect::of_size(size);
    assert!(!reads.is_empty() && reads.iter().all(|read| *read == (whole, 1)), "a new revision with no damage reads everything: {reads:?}");

    assert!(rig.frame(3, PixelRect::default()).is_empty(), "the same revision reads nothing");
}

#[trial]
fn zoomed_in_a_reduced_document_gets_a_detail_texture_that_zooming_out_drops() {
    let size = PixelSize::new(10_000, 3000);
    let mut rig = rig(Document::new(size.width, size.height), 1.0);
    let ctx = rig.fixture.ctx().clone();
    let side = texture_side(&ctx);
    let space = rig.space.borrow().expect("the canvas has a view");
    let plan = DetailPlan::in_view(&space.view, size, side, space.pixels_per_point).expect("100 percent over a reduced document needs detail");
    assert_eq!(plan.factor, 1);
    assert_eq!(rig.reported.borrow().map(|(_, detail)| detail), Some(Some(plan)));

    let reads = rig.frame(2, PixelRect::default());
    assert!(reads.contains(&(PixelRect::of_size(size), texture_factor(size, side))), "the base texture is the whole document, reduced");
    assert!(reads.contains(&(plan.region, 1)), "the detail texture is the part in view at full resolution");

    // A view at 10 percent is no finer than the reduced texture.
    let mut far = space.view;
    far.zoom = 0.1;
    assert_eq!(DetailPlan::in_view(&far, size, side, space.pixels_per_point), None);
}

#[trial]
fn the_detail_plan_snaps_to_256_texels_and_only_applies_to_a_reduced_document() {
    let size = PixelSize::new(10_000, 3000);
    let visible = PixelRect::new(4100, 1000, 4900, 1600);
    let plan = DetailPlan::of(size, 4096, 1.0, visible).expect("100 percent needs a detail texture");
    assert_eq!((plan.factor, plan.region), (1, PixelRect::new(4096, 768, 5120, 1792)));
    assert_eq!(DetailPlan::of(size, 4096, 0.1, visible), None);
    assert_eq!(DetailPlan::of(PixelSize::new(3000, 2000), 4096, 1.0, visible), None, "a document that fits needs none");
    assert_eq!(texture_factor(size, 4096), 3);
    assert_eq!(texture_factor(PixelSize::new(4096, 4096), 4096), 1);
}

#[trial]
fn damage_snaps_to_the_texel_grid_of_the_region_it_updates() {
    // A detail region that fell back to the visible rect starts off the factor's grid.
    let region = PixelRect::new(5, 3, 45, 43);
    let snapped = snap_to_texels(PixelRect::new(14, 10, 17, 12), [region.x0, region.y0], 4);
    assert_eq!(snapped, PixelRect::new(13, 7, 17, 15));
    assert_eq!(snap_to_texels(PixelRect::new(5, 5, 9, 9), [0, 0], 4), PixelRect::new(4, 4, 12, 12));
}

/// Runs `show` in one frame of `frames`.
fn once(frames: &mut Frames, show: impl FnMut(&mut egui::Ui)) {
    frames.idle(1, show);
}

fn ask(thumbs: &Thumbnails<u32>, source: &dyn ThumbSource<u32>, ctx: &egui::Context, key: u32) -> TextureState {
    thumbs.state(ctx, source, &key, &ThumbRequest::for_cell(32.0, 2.0), Priority::VISIBLE)
}

#[trial]
fn thumbnails_render_a_few_per_pass_and_the_rest_wait_with_no_picture() {
    const KEYS: u32 = 20;
    const BUDGET: usize = 16;
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let thumbs = Thumbnails::<u32>::in_context(&ctx, "budget", ThumbnailOptions::new());
    let source = FakeThumbSource::<u32>::new();
    let mut states = Vec::new();
    once(&mut frames, |_| states = (0..KEYS).map(|key| ask(&thumbs, &source, &ctx, key)).collect());
    assert_eq!(states.iter().filter(|state| matches!(state, TextureState::Ready(_))).count(), BUDGET);
    assert_eq!(
        states.iter().filter(|state| matches!(state, TextureState::Loading { .. }) && state.shown().is_none()).count(),
        KEYS as usize - BUDGET
    );
    once(&mut frames, |_| states = (0..KEYS).map(|key| ask(&thumbs, &source, &ctx, key)).collect());
    assert!(states.iter().all(|state| matches!(state, TextureState::Ready(_))), "the rest land in the next pass");
}

#[trial]
fn a_held_thumbnail_keeps_its_picture_until_released() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let thumbs = Thumbnails::<u32>::in_context(&ctx, "hold", ThumbnailOptions::new());
    let mut source = FakeThumbSource::<u32>::new();
    let mut shown = None;
    once(&mut frames, |_| shown = ask(&thumbs, &source, &ctx, 1).shown());
    let shown = shown.expect("the first pass renders");
    source.bump();
    thumbs.hold(true);
    let mut state = TextureState::Missing;
    once(&mut frames, |_| state = ask(&thumbs, &source, &ctx, 1));
    assert!(matches!(state, TextureState::Loading { .. }) && state.shown() == Some(shown), "the old picture stands in: {state:?}");
    thumbs.hold(false);
    once(&mut frames, |_| state = ask(&thumbs, &source, &ctx, 1));
    let TextureState::Ready(updated) = state else { panic!("expected Ready, got {state:?}") };
    assert_eq!(updated.id, shown.id, "the new revision replaces the picture in place");
}

/// A fake source that records the region of every render.
struct Counting {
    inner: FakeThumbSource<u32>,
    renders: RefCell<Vec<Option<Rect>>>,
}

impl ThumbSource<u32> for Counting {
    fn revision(&self, key: &u32, request: &ThumbRequest) -> Revision {
        self.inner.revision(key, request)
    }

    fn render(&self, key: &u32, request: &ThumbRequest) -> ThumbRender {
        self.renders.borrow_mut().push(request.region);
        self.inner.render(key, request)
    }
}

#[trial]
fn a_region_of_a_frame_is_its_own_picture() {
    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let thumbs = Thumbnails::<u32>::in_context(&ctx, "regions", ThumbnailOptions::new());
    let source = Counting { inner: FakeThumbSource::new(), renders: RefCell::default() };
    let whole = ThumbRequest::for_cell(32.0, 2.0);
    let part = whole.region(Rect::from_min_max(Pos2::new(0.25, 0.25), Pos2::new(0.5, 0.5)));
    let other = whole.region(Rect::from_min_max(Pos2::new(0.5, 0.5), Pos2::new(0.75, 0.75)));
    for request in [&whole, &whole, &part, &part, &other] {
        once(&mut frames, |_| {
            assert!(matches!(thumbs.state(&ctx, &source, &1, request, Priority::VISIBLE), TextureState::Ready(_)));
        });
    }
    assert_eq!(*source.renders.borrow(), [None, part.region, other.region], "the same request renders once");
}

#[trial]
fn requests_step_by_16_pixels_and_are_lowered_to_the_largest_side() {
    const SMALL_SIDE: u32 = 100;
    assert_eq!(ThumbRequest::for_cell(28.0, 2.0).side, 64);
    assert_eq!(ThumbRequest::for_cell(3.0, 1.0).side, 16);
    assert_eq!(ThumbRequest::for_cell(1.0e6, 1.0).side % 16, 0);

    let mut frames = Frames::new();
    let ctx = frames.ctx.clone();
    let thumbs = Thumbnails::<u32>::in_context(&ctx, "small", ThumbnailOptions::new().max_side(SMALL_SIDE));
    let source = FakeThumbSource::<u32>::new();
    let mut size = None;
    once(&mut frames, |_| {
        let request = ThumbRequest::for_cell(300.0, 1.0);
        size = thumbs.state(&ctx, &source, &1, &request, Priority::VISIBLE).shown().map(|texture| texture.size);
    });
    assert_eq!(size, Some(PixelSize::new(SMALL_SIDE / 16 * 16, SMALL_SIDE / 16 * 16)), "a side past the largest is lowered to a multiple of 16");
}

#[trial]
fn a_thumbnail_keeps_the_frames_aspect_and_fits_its_cell_whole() {
    assert_eq!(thumb_image(PixelSize::new(400, 300), 64, |_, _| [0, 0, 0, 255]).size, [64, 48]);
    assert_eq!(thumb_image(PixelSize::new(300, 400), 64, |_, _| [0, 0, 0, 255]).size, [48, 64]);
    let image = thumb_image(PixelSize::new(4, 2), 4, |x, y| [x as u8, y as u8, 0, 255]);
    assert_eq!(image.size, [4, 2]);
    assert_eq!(image.pixels[5], Color32::from_rgb(1, 1, 0), "each texel samples the pixel under its centre");

    let cell = Rect::from_min_size(Pos2::ZERO, Vec2::splat(40.0));
    assert_eq!(fit_rect(cell, Vec2::new(4.0, 3.0)), Rect::from_min_size(Pos2::new(0.0, 5.0), Vec2::new(40.0, 30.0)));
    assert_eq!(fit_rect(cell, Vec2::new(0.0, 3.0)), cell, "a picture with no size fills the cell");
}
