//! Tests of the styled transform preview.

use std::cell::Cell;

use egui::{Color32, ColorImage, Context};
use kurbo::Point;

use craft_ui::canvas::PixelRect;
use craft_ui::canvas::quad::Homography;

use super::styled::{StyledFrame, resample};
use super::{PreviewDisplay, PreviewPixels, PreviewTextures, StylePlan, StyledBudget, StyledContent, StyledPreview, StyledSource};

fn square(x: f64, y: f64, size: f64) -> [Point; 4] {
    [Point::new(x, y), Point::new(x + size, y), Point::new(x + size, y + size), Point::new(x, y + size)]
}

/// A source with a fixed reach (in work pixels, whatever the scale) that counts its compositions.
struct Fake {
    revision: Cell<u64>,
    reach: i32,
    composed: Cell<u32>,
}

impl Fake {
    fn new(reach: i32) -> Self {
        Self { revision: Cell::new(1), reach, composed: Cell::new(0) }
    }
}

impl StyledSource for Fake {
    fn revision(&self) -> u64 {
        self.revision.get()
    }
    fn pixel_cost_ns(&self) -> f64 {
        20.0
    }
    fn reach(&self, _scale: f64) -> i32 {
        self.reach
    }
    fn compose(&self, plan: &StylePlan, _content: &StyledContent) -> Option<ColorImage> {
        self.composed.set(self.composed.get() + 1);
        let size = [plan.shown.width() as usize, plan.shown.height() as usize];
        Some(ColorImage::new(size, vec![Color32::WHITE; size[0] * size[1]]))
    }
}

fn frame<'a>(textures: &'a PreviewTextures, quad: [Point; 4], zoom: f32) -> StyledFrame<'a> {
    StyledFrame { textures, quad, quad_scale: 1.0, nearest: false, zoom, dragging: false, canvas: [100, 100], budget: StyledBudget::default() }
}

fn textures(ctx: &Context) -> PreviewTextures {
    let image = ColorImage::new([4, 4], vec![Color32::from_rgb(200, 100, 50); 16]);
    PreviewTextures::new(ctx, "styled", PreviewPixels::new(image, [1.0, 1.0]))
}

// Two passes with the same box, zoom, revision, display key, level and filter build once; a
// change of any one builds again.
#[test]
fn the_styled_frame_is_rebuilt_only_when_its_key_changes() {
    let ctx = Context::default();
    let textures = textures(&ctx);
    let source = Fake::new(2);
    let preview = StyledPreview::new("styled-frame");
    let box_a = square(10.0, 10.0, 40.0);
    assert!(preview.texture(&ctx, &frame(&textures, box_a, 1.0), &source, None).is_some());
    assert!(preview.texture(&ctx, &frame(&textures, box_a, 1.0), &source, None).is_some());
    assert_eq!(preview.builds(), 1);
    assert_eq!(source.composed.get(), 1);

    let mut builds = 1;
    let mut expect_build = |preview: &StyledPreview, f: &StyledFrame<'_>, source: &Fake, display: Option<&PreviewDisplay>, what: &str| {
        preview.texture(&ctx, f, source, display);
        builds += 1;
        assert_eq!(preview.builds(), builds, "{what}");
    };
    expect_build(&preview, &frame(&textures, square(11.0, 10.0, 40.0), 1.0), &source, None, "moved box");
    expect_build(&preview, &frame(&textures, square(11.0, 10.0, 40.0), 0.5), &source, None, "zoom");
    source.revision.set(2);
    expect_build(&preview, &frame(&textures, square(11.0, 10.0, 40.0), 0.5), &source, None, "revision");
    let display = PreviewDisplay { key: 7, convert: std::sync::Arc::new(Clone::clone) };
    expect_build(&preview, &frame(&textures, square(11.0, 10.0, 40.0), 0.5), &source, Some(&display), "display key");
    let mut nearest = frame(&textures, square(11.0, 10.0, 40.0), 0.5);
    nearest.nearest = true;
    expect_build(&preview, &nearest, &source, Some(&display), "filter");
}

// A composite of the wrong size, or none, is asked for once per key; a new key asks again.
#[test]
fn a_failed_composite_is_not_asked_for_again_until_the_key_changes() {
    struct Wrong(Cell<u32>);
    impl StyledSource for Wrong {
        fn revision(&self) -> u64 {
            1
        }
        fn pixel_cost_ns(&self) -> f64 {
            20.0
        }
        fn reach(&self, _scale: f64) -> i32 {
            2
        }
        fn compose(&self, _plan: &StylePlan, _content: &StyledContent) -> Option<ColorImage> {
            self.0.set(self.0.get() + 1);
            Some(ColorImage::new([1, 1], vec![Color32::WHITE]))
        }
    }
    let ctx = Context::default();
    let textures = textures(&ctx);
    let source = Wrong(Cell::new(0));
    let preview = StyledPreview::new("styled-wrong");
    for _ in 0..3 {
        assert!(preview.texture(&ctx, &frame(&textures, square(10.0, 10.0, 40.0), 1.0), &source, None).is_none());
    }
    assert_eq!(source.0.get(), 1);
    assert!(preview.texture(&ctx, &frame(&textures, square(11.0, 10.0, 40.0), 1.0), &source, None).is_none());
    assert_eq!(source.0.get(), 2);
    assert_eq!(preview.builds(), 0);
}

// A costly style shrinks the work scale until the content's area fits the budget, in up to 6
// rounds; a box off the canvas gives None.
#[test]
fn the_budget_shrinks_the_work_scale_until_the_composite_fits() {
    let canvas = [4000, 4000];
    let big = square(0.0, 0.0, 4000.0);
    let plan = StylePlan::new(big, canvas, 1.0, 1.0e6, |_| 10).unwrap_or_else(|| panic!("a plan"));
    assert!(plan.scale < 1.0);
    let area = f64::from(plan.content.width()) * f64::from(plan.content.height());
    assert!(area <= 1.0e6, "{area}");
    assert!(plan.shown.x0 >= 0 && plan.shown.x1 <= (4000.0 * plan.scale).ceil() as i32);
    assert!(plan.content.x0 <= plan.shown.x0 - 20 || plan.shown.x0 == 0 && plan.content.x0 == -20);

    let small = StylePlan::new(square(10.0, 10.0, 20.0), [100, 100], 4.0, 1.0e6, |_| 3).unwrap_or_else(|| panic!("a plan"));
    assert_eq!(small.scale, 1.0, "the scale never exceeds the document's");
    assert_eq!(small.bounds, PixelRect::new(10, 10, 30, 30));
    assert_eq!(small.shown, PixelRect::new(7, 7, 33, 33));
    assert_eq!(small.content, PixelRect::new(1, 1, 39, 39));
    assert_eq!(small.document_rect(), kurbo::Rect::new(7.0, 7.0, 33.0, 33.0));

    assert_eq!(StylePlan::new(square(500.0, 500.0, 20.0), [100, 100], 1.0, 1.0e6, |_| 3), None);
    assert_eq!(StylePlan::new(square(f64::NAN, 0.0, 20.0), [100, 100], 1.0, 1.0e6, |_| 3), None);
    // A reach that grows with the scale and never fits gives up after 6 rounds.
    assert_eq!(StylePlan::new(big, canvas, 1.0, 1.0e6, |_| 100_000), None);
}

// The budget is 30 ms dragging and 120 ms at rest over the cost per pixel plus the resample's,
// clamped to 0.25 to 8 million pixels.
#[test]
fn the_budget_follows_what_the_style_costs() {
    let dear = StyledBudget::default();
    assert_eq!(dear.pixels(45.0, true), 0.5e6, "the resample's cost is in the sum");
    assert!(dear.pixels(60.0, true) < 0.5e6);
    let budget = StyledBudget { resample_ns: 0.0, ..StyledBudget::default() };
    assert_eq!(budget.pixels(60.0, true), 0.5e6);
    assert_eq!(budget.pixels(60.0, false), 2.0e6);
    assert_eq!(budget.pixels(1000.0, true), StyledBudget::MIN_PIXELS);
    assert_eq!(budget.pixels(1.0, false), StyledBudget::MAX_PIXELS);
}

// Resampling a texel image through a box keeps texels whole with nearest, fades the source's
// edges over one texel, and leaves empty texels transparent.
#[test]
fn resampling_handles_empty_texels_and_nearest_neighbor() {
    // A 2 x 1 image, red then blue, shown at 4 times its size on a 10 x 10 work rectangle.
    let image = ColorImage::new([2, 1], vec![Color32::from_rgb(255, 0, 0), Color32::from_rgb(0, 0, 255)]);
    let quad = [Point::new(1.0, 1.0), Point::new(9.0, 1.0), Point::new(9.0, 5.0), Point::new(1.0, 5.0)];
    let Some(to_unit) = Homography::unit_to_quad(quad).and_then(|h| h.inverse()) else { panic!("invertible") };
    let inside = Point::new(5.0, 3.0);
    let rect = PixelRect::new(0, 0, 10, 6);
    let at = |c: &StyledContent, x: usize, y: usize| c.texels[y * 10 + x];

    let crisp = resample(&image, [1.0, 1.0], &to_unit, inside, rect, true);
    assert_eq!(at(&crisp, 2, 2), [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(at(&crisp, 7, 2), [0.0, 0.0, 1.0, 1.0]);
    assert_eq!(at(&crisp, 0, 0), [0.0; 4], "outside the box");

    let smooth = resample(&image, [1.0, 1.0], &to_unit, inside, rect, false);
    let edge = at(&smooth, 1, 3);
    assert!(edge[3] > 0.0 && edge[3] < 1.0, "the source's left edge fades over a texel: {edge:?}");
    assert!(at(&smooth, 0, 5)[3] < 0.5, "a texel outside the box fades out");

    let empty = resample(&ColorImage::new([0, 0], vec![]), [1.0, 1.0], &to_unit, inside, rect, false);
    assert!(empty.texels.iter().all(|t| *t == [0.0; 4]));
    assert_eq!(empty.texels.len(), 60);
}
