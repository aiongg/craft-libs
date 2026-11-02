//! Stories of the compare tile: `patterns/compare-tile/pair` and
//! `patterns/compare-tile/survey` (the spec's `browse/compare-*` as `<level>/<component>/<story>`). Two tiles
//! with one shared zoom and pan, the render pills and the rejected veil; twelve tiles at 3:2 in
//! the survey arrangement.
//!
//! The pictures are generated in the story (a gradient with a grid, so zoom and pan
//! show). A click makes a pane active and a double click toggles Fit and 100 %, as an app would.

use craft_theme::Rgba;
use craft_ui::canvas::CanvasView;
use craft_ui::canvas::textures::Revision;
use craft_ui::localized::Labels as _;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, lock, shared};
use craft_ui::theme::{Tokens, TokensOf as _};
use craft_ui::typed_story;
use egui::ecolor::Hsva;
use egui::{Color32, ColorImage, Rect, Sense, Ui, vec2};
use kurbo::Point;

use super::{CompareCaption, CompareImage, ComparePair, CompareStyle, CompareTile, RenderState, survey_columns};
use crate::ui::grid::TileFlag;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["CompareTile"];

craft_ui::words! {
    /// The sample text of the compare stories, English by default.
    #[non_exhaustive]
    pub struct CompareStoryLabels {
        select: "compareStory.select" = "Select";
        candidate: "compareStory.candidate" = "Candidate";
        render_failed: "compareStory.renderFailed" = "The renderer stopped";
        active: "compareStory.active" = "Active pane: {label}";
    }
}

/// The stories above.
pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "patterns/compare-tile/pair",
            Level::Patterns,
            "CompareTile",
            "Pair",
            PairArgs { right: RightState::Ready, rejected: false, flagged: true, rtl: false, height: 340.0 },
            show_pair
        ),
        typed_story!("patterns/compare-tile/survey", Level::Patterns, "CompareTile", "Survey", SurveyArgs { count: 12, height: 360.0 }, show_survey),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RightState {
    Ready,
    #[choice(name = "Rendering")]
    Rendering,
    Failed,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PairArgs {
    /// The right pane's render state: the Rendering and Render failed pills.
    right: RightState,
    /// The rejected veil on the right pane.
    rejected: bool,
    /// Rating, pick flag and colour label in the captions.
    flagged: bool,
    /// Right to left: the panes swap sides.
    rtl: bool,
    #[arg(range = 200.0..=600.0)]
    height: f32,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct SurveyArgs {
    #[arg(range = 2..=12)]
    count: usize,
    #[arg(range = 200.0..=600.0)]
    height: f32,
}

const SIZE: [u32; 2] = [900, 600];

/// A gradient of the seed's hue with a grid every 100 pixels, so zoom and pan show.
fn picture(seed: u64) -> ColorImage {
    let hue = (seed % 12) as f32 / 12.0;
    let [width, height] = SIZE;
    let mut image = ColorImage::filled([width as usize, height as usize], Color32::GRAY);
    for y in 0..height {
        for x in 0..width {
            let t = (x as f32 / width as f32 + y as f32 / height as f32) / 2.0;
            let line = x % 100 < 2 || y % 100 < 2;
            let color: Color32 = if line { Hsva::new(hue, 0.2, 0.95, 1.0).into() } else { Hsva::new((hue + 0.3 * t).fract(), 0.55, 0.8, 1.0).into() };
            image.pixels[(y * width + x) as usize] = color;
        }
    }
    image
}

fn image(seed: u64) -> CompareImage<'static> {
    CompareImage::Picture { key: seed, revision: Revision(1), size: SIZE, make: Box::new(move || picture(seed)) }
}

struct Demo {
    view: CanvasView,
    active: usize,
}

fn show_pair(ui: &mut Ui, args: &mut PairArgs) {
    let words = CompareStoryLabels::of(ui);
    let story = StoryLabels::of(ui);
    let state = shared(ui, "pair", || Demo { view: CanvasView::new(Rect::ZERO, Point::ZERO, 0.0), active: 0 });
    let mut demo = lock(&state);
    let style = CompareStyle::of(&Tokens::of(ui));
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), args.height), Sense::hover());
    let pair = ComparePair::new().rtl(args.rtl);
    let rects = pair.rects(area, &style);
    ui.painter().rect_filled(pair.divider(area, &style).expand2(vec2(0.5, 0.0)), 0.0, style.divider);

    let labels = [words.select.clone(), words.candidate.clone()];
    let names = [craft_i18n::fmt(&story.photo_n, &[("n", "1")]), craft_i18n::fmt(&story.photo_n, &[("n", "2")])];
    let active = demo.active;
    let mut clicked = None;
    for (n, rect) in rects.into_iter().enumerate() {
        let mut caption = CompareCaption::new(labels[n].clone(), names[n].clone());
        if args.flagged {
            caption = caption
                .rating(Some(if n == 0 { 4 } else { 2 }))
                .flag((n == 0).then_some(TileFlag::Picked))
                .label_color((n == 1).then(|| Rgba::rgb(230, 170, 40)));
        }
        let mut tile = CompareTile::new(("compare-pair", n), &mut demo.view, image(n as u64 * 5 + 1)).caption(caption).active(active == n);
        if n == 1 {
            tile = tile.rejected(args.rejected).state(match args.right {
                RightState::Ready => RenderState::Ready,
                RightState::Rendering => RenderState::Pending { overdue: true },
                RightState::Failed => RenderState::Failed(words.render_failed.clone()),
            });
        }
        let shown = tile.show(ui, rect);
        if shown.clicked {
            clicked = Some(n);
        }
        if shown.double_clicked {
            let image_area = Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.bottom() - style.caption));
            CompareTile::toggle_zoom(&mut demo.view, SIZE, image_area, ui.pixels_per_point());
        }
    }
    if let Some(n) = clicked {
        demo.active = n;
    }
    ui.add(Text::new(craft_i18n::fmt(&words.active, &[("label", labels[demo.active].as_str())])).content(Content::Muted));
}

fn show_survey(ui: &mut Ui, args: &mut SurveyArgs) {
    let story = StoryLabels::of(ui);
    let state = shared(ui, "survey", || 0usize);
    let active = *lock(&state);
    let style = CompareStyle::of(&Tokens::of(ui));
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), args.height), Sense::hover());
    let inner = area.shrink(style.margin);
    let count = args.count;
    let columns = survey_columns(count, inner);
    let rows = count.div_ceil(columns);
    let cell =
        vec2((inner.width() - style.gap * (columns - 1) as f32) / columns as f32, (inner.height() - style.gap * (rows - 1) as f32) / rows as f32);
    let mut clicked = None;
    for n in 0..count {
        let (column, row) = (n % columns, n / columns);
        let min = inner.min + vec2(column as f32 * (cell.x + style.gap), row as f32 * (cell.y + style.gap));
        let name = craft_i18n::fmt(&story.photo_n, &[("n", &(n + 1).to_string())]);
        let mut view = CanvasView::new(Rect::ZERO, Point::ZERO, 0.0);
        let shown = CompareTile::new(("compare-survey", n), &mut view, image(n as u64))
            .caption(CompareCaption::new((n + 1).to_string(), name).rating(Some((n % 5 + 1) as u8)))
            .survey(true)
            .active(active == n)
            .show(ui, Rect::from_min_size(min, cell));
        if shown.clicked {
            clicked = Some(n);
        }
    }
    if let Some(n) = clicked {
        *lock(&state) = n;
    }
}
