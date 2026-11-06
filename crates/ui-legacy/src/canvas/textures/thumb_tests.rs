//! `Thumbnails`, `ThumbRequest`, `thumb_image` and `fit_rect` over `FakeThumbSource`.

use std::sync::Arc;

use craft_host::ManualJobs;
use egui::{Context, Rect, Ui, pos2, vec2};

use super::{FakeThumbSource, MAX_TEXTURE, Priority, TextureState, ThumbPaint, ThumbRequest, ThumbnailOptions, Thumbnails, fit_rect, thumb_image};

fn thumbnails(ctx: &Context) -> Thumbnails<u32> {
    Thumbnails::in_context(ctx, "test", ThumbnailOptions::default())
}

fn ask(thumbs: &Thumbnails<u32>, ctx: &Context, source: &FakeThumbSource<u32>, key: u32) -> TextureState {
    thumbs.state(ctx, source, &key, &ThumbRequest::for_cell(32.0, 2.0), Priority::VISIBLE)
}

fn pass(ctx: &Context, mut run: impl FnMut(&mut Ui)) {
    ctx.run_ui(egui::RawInput::default(), |ui| run(ui)).drop_without_applying_deltas();
}

// 20 synchronous renders in one pass: 16 land, 4 are Loading, and a repaint is requested.
#[test]
fn thumbnails_defer_renders_over_budget() {
    let ctx = Context::default();
    let thumbs = thumbnails(&ctx);
    let source = FakeThumbSource::<u32>::new();
    let mut states = Vec::new();
    pass(&ctx, |_| states = (0..20).map(|key| ask(&thumbs, &ctx, &source, key)).collect());
    assert_eq!(states.iter().filter(|state| matches!(state, TextureState::Ready(_))).count(), 16);
    assert_eq!(states.iter().filter(|state| matches!(state, TextureState::Loading { stand_in: None })).count(), 4);
    assert!(ctx.has_requested_repaint());
    pass(&ctx, |_| states = (0..20).map(|key| ask(&thumbs, &ctx, &source, key)).collect());
    assert!(states.iter().all(|state| matches!(state, TextureState::Ready(_))));
}

// While held, a new revision returns the old picture as Loading and no render runs.
#[test]
fn thumbnails_hold_keeps_shown_pictures() {
    let ctx = Context::default();
    let thumbs = thumbnails(&ctx);
    let mut source = FakeThumbSource::<u32>::new();
    let mut shown = None;
    pass(&ctx, |_| shown = ask(&thumbs, &ctx, &source, 1).shown());
    let shown = shown.expect("rendered");
    source.bump();
    thumbs.hold(true);
    let mut state = TextureState::Missing;
    pass(&ctx, |_| state = ask(&thumbs, &ctx, &source, 1));
    assert_eq!(state, TextureState::Loading { stand_in: Some(shown) });
    thumbs.hold(false);
    pass(&ctx, |_| state = ask(&thumbs, &ctx, &source, 1));
    let TextureState::Ready(updated) = state else { panic!("expected Ready, got {state:?}") };
    assert_eq!(updated.id, shown.id);
}

// A Later render shows no picture (Loading without a stand-in) until ManualJobs runs it, then Ready.
#[test]
fn thumbnails_later_renders_arrive_through_the_queue() {
    let ctx = Context::default();
    let jobs = Arc::new(ManualJobs::new());
    let thumbs = Thumbnails::<u32>::in_context(&ctx, "later", ThumbnailOptions { lanes: 1, jobs: Some(jobs.clone()), ..ThumbnailOptions::default() });
    let source = FakeThumbSource::<u32>::new().later(true);
    let mut state = TextureState::Missing;
    pass(&ctx, |_| state = ask(&thumbs, &ctx, &source, 1));
    assert_eq!(state, TextureState::Loading { stand_in: None });
    assert_eq!(jobs.waiting(), 1);
    pass(&ctx, |_| state = ask(&thumbs, &ctx, &source, 1));
    assert_eq!(state, TextureState::Loading { stand_in: None });
    assert_eq!(jobs.waiting(), 1);
    assert!(jobs.run_next());
    pass(&ctx, |_| state = ask(&thumbs, &ctx, &source, 1));
    assert!(matches!(state, TextureState::Ready(_)), "got {state:?}");
    assert_eq!(jobs.waiting(), 0);
}

// `paint` returns false for Empty and draws the placeholder for Missing and the icon for Failed.
#[test]
fn thumbnails_paint_each_state() {
    let ctx = Context::default();
    let thumbs = thumbnails(&ctx);
    let source = FakeThumbSource::<u32>::new();
    let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(32.0, 32.0));
    let mut drawn = Vec::new();
    pass(&ctx, |ui| {
        let shapes = |ui: &Ui| ui.ctx().graphics(|graphics| graphics.get(ui.layer_id()).map_or(0, |list| list.all_entries().len()));
        let paint = |state: TextureState| {
            let before = shapes(ui);
            let result = thumbs.paint(ui.painter(), rect, &state, &ThumbPaint::default());
            (result, shapes(ui) - before)
        };
        let ready = ask(&thumbs, &ctx, &source, 1);
        drawn = vec![paint(TextureState::Empty), paint(TextureState::Missing), paint(TextureState::Failed { message: "no".into() }), paint(ready)];
    });
    assert_eq!(drawn[0], (false, 0));
    assert_eq!(drawn[1], (true, 1));
    assert_eq!(drawn[2], (true, 2));
    assert!(drawn[3].0 && drawn[3].1 > 2);
}

// Landscape 400x300 at 64 gives 64x48; portrait 300x400 gives 48x64.
#[test]
fn thumb_image_has_the_frames_aspect() {
    assert_eq!(thumb_image([400, 300], 64, |_, _| [0, 0, 0, 255]).size, [64, 48]);
    assert_eq!(thumb_image([300, 400], 64, |_, _| [0, 0, 0, 255]).size, [48, 64]);
    let image = thumb_image([4, 2], 4, |x, y| [x as u8, y as u8, 0, 255]);
    assert_eq!(image.size, [4, 2]);
    assert_eq!(image.pixels[5], egui::Color32::from_rgb(1, 1, 0));
}

// 28 pt at 2x gives 64; 3 pt at 1x gives 16; past MAX_TEXTURE gives MAX_TEXTURE; `state` lowers to max_side.
#[test]
fn thumb_request_rounds_side_up_to_16() {
    assert_eq!(ThumbRequest::for_cell(28.0, 2.0).side, 64);
    assert_eq!(ThumbRequest::for_cell(3.0, 1.0).side, 16);
    assert_eq!(ThumbRequest::for_cell(1.0e6, 1.0).side, MAX_TEXTURE);

    let ctx = Context::default();
    let thumbs = Thumbnails::<u32>::in_context(&ctx, "small", ThumbnailOptions { max_side: 100, ..ThumbnailOptions::default() });
    let source = FakeThumbSource::<u32>::new();
    let mut texture = None;
    pass(&ctx, |_| {
        let request = ThumbRequest::for_cell(300.0, 1.0);
        texture = thumbs.state(&ctx, &source, &1, &request, Priority::VISIBLE).shown();
    });
    assert_eq!(texture.map(|texture| texture.size), Some([96, 96]));
}

// `fit_rect` of a 4:3 picture in a square cell is centred, full width, three quarters high.
#[test]
fn fit_rect_keeps_the_aspect() {
    let cell = Rect::from_min_size(pos2(0.0, 0.0), vec2(40.0, 40.0));
    assert_eq!(fit_rect(cell, vec2(4.0, 3.0)), Rect::from_min_size(pos2(0.0, 5.0), vec2(40.0, 30.0)));
    assert_eq!(fit_rect(cell, vec2(0.0, 3.0)), cell);
}

// `paint_placeholder` fills the rect with one shape in the placeholder colour.
#[test]
fn thumbnails_paint_the_placeholder_alone() {
    let ctx = Context::default();
    let mut drawn = 0;
    pass(&ctx, |ui| {
        let shapes = |ui: &Ui| ui.ctx().graphics(|graphics| graphics.get(ui.layer_id()).map_or(0, |list| list.all_entries().len()));
        let before = shapes(ui);
        Thumbnails::<u32>::paint_placeholder(ui.painter(), Rect::from_min_size(pos2(10.0, 10.0), vec2(32.0, 32.0)));
        drawn = shapes(ui) - before;
    });
    assert_eq!(drawn, 1);
}

struct Counting {
    inner: FakeThumbSource<u32>,
    renders: std::cell::RefCell<Vec<Option<Rect>>>,
}

impl super::ThumbSource<u32> for Counting {
    fn revision(&self, key: &u32, request: &ThumbRequest) -> super::Revision {
        self.inner.revision(key, request)
    }
    fn render(&self, key: &u32, request: &ThumbRequest) -> super::ThumbRender {
        self.renders.borrow_mut().push(request.region);
        self.inner.render(key, request)
    }
}

// A key holds one picture: asking for another region of it renders again and the source sees the
// region; asking for the same one does not.
#[test]
fn a_region_is_its_own_picture() {
    let ctx = Context::default();
    let thumbs = thumbnails(&ctx);
    let source = Counting { inner: FakeThumbSource::<u32>::new(), renders: Default::default() };
    let whole = ThumbRequest::for_cell(32.0, 2.0);
    let part = whole.region(Rect::from_min_max(pos2(0.25, 0.25), pos2(0.5, 0.5)));
    let other = whole.region(Rect::from_min_max(pos2(0.5, 0.5), pos2(0.75, 0.75)));
    for request in [&whole, &whole, &part, &part, &other] {
        pass(&ctx, |_| {
            assert!(matches!(thumbs.state(&ctx, &source, &1, request, Priority::VISIBLE), TextureState::Ready(_)));
        });
    }
    assert_eq!(*source.renders.borrow(), [None, part.region, other.region]);
}
