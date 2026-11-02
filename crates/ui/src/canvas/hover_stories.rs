//! Stories of the hover outline (task KS1): not registered. Registering them (task RG2)
//! adds `stories()` to the canvas stories, `COMPONENTS` to the component list and
//! [`HoverStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! Every story uses fakes: a document drawn by the story (shapes on a light and a dark image) and a
//! [`ManualJobs`](craft_host::ManualJobs) behind the lookup with a "Finish next" button, so each
//! lookup state shows one step at a time. Every string goes through story words.
//!
//! | Story id | Shows |
//! |---|---|
//! | `primitives/hover/outline` | `HoverShape::Rect`, a `Path` in a layer colour and a `Mask` tint, on a light and a dark image |
//! | `primitives/hover/lookup` | A fake finder whose job takes 300 ms: debounce, pending, hit, miss, a cached hit inside a found object, cleared by a press, the empty-layer hint |

use std::sync::{Arc, Mutex};

use craft_host::ManualJobs;
use craft_host::queue::Cancel;
use egui::{Color32, Pos2, Shape, Stroke, Ui};
use kurbo::{Ellipse, Point, Rect, Shape as _};

use super::CanvasView;
use super::hover::{HoverAnswer, HoverHint, HoverLookup, HoverMask, HoverOutline, HoverQuery, HoverShape};
use super::story::{DOC, ViewArgs, document};
use crate::controls::Button;
use crate::localized::Labels as _;
use crate::primitives::{Content, Text};
use crate::story::{Level, Story, StoryArgs, StoryLabels, Surface};
use crate::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};
use crate::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["HoverOutline", "HoverLookup"];

crate::words! {
    /// The captions of the hover stories, English by default.
    #[non_exhaustive]
    pub(crate) struct HoverStoryLabels {
        rect: "hoverStory.rect" = "Rectangle";
        path: "hoverStory.path" = "Path in a layer color";
        mask: "hoverStory.mask" = "Mask tint";
        finish: "hoverStory.finish" = "Finish next";
        nothing_asked: "hoverStory.nothingAsked" = "No lookup yet.";
        waiting: "hoverStory.waiting" = "Looking up… ({n} waiting)";
        found: "hoverStory.found" = "Found: {name}";
        miss: "hoverStory.miss" = "Nothing found here.";
        empty_layer: "hoverStory.emptyLayer" = "The sampled layer has no pixels here.";
        first: "hoverStory.first" = "Box";
        second: "hoverStory.second" = "Ellipse";
        third: "hoverStory.third" = "Soft ellipse";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/hover/outline",
            Level::Primitives,
            "HoverOutline",
            "Outline",
            OutlineArgs { view: ViewArgs::default(), dark: false },
            show_outline
        )
        .on(Surface::Canvas),
        typed_story!(
            "primitives/hover/lookup",
            Level::Primitives,
            "HoverLookup",
            "Lookup",
            LookupArgs { view: ViewArgs::default(), empty_layer: false },
            show_lookup
        )
        .on(Surface::Canvas),
    ]
}

/// Tone of the fake picture under the objects: a light and a dark photo, which only this story
/// needs.
const LIGHT_PICTURE: u8 = 235;
const DARK_PICTURE: u8 = 40;
/// The fake objects, in document units (the story document is 640 × 400).
const BOX: Rect = Rect::new(60.0, 120.0, 200.0, 280.0);
const ROUND: ([f64; 2], [f64; 2]) = ([330.0, 200.0], [80.0, 70.0]);
const SOFT: ([f64; 2], [f64; 2]) = ([520.0, 200.0], [80.0, 70.0]);
/// The part of the document where the sampled layer is empty, when the story says so.
const EMPTY_FROM_X: f64 = 560.0;
/// Fraction of an ellipse's radius where the soft mask starts to fall off, and where it ends.
const SOFT_FROM: f64 = 0.8;
const SOFT_TO: f64 = 1.0;

fn ellipse(spec: ([f64; 2], [f64; 2])) -> Ellipse {
    Ellipse::new(Point::new(spec.0[0], spec.0[1]), (spec.1[0], spec.1[1]), 0.0)
}

fn soft_mask() -> HoverMask {
    let e = ellipse(SOFT);
    HoverMask::sampled(e.bounding_box(), move |x, y| {
        let d = (((x - SOFT.0[0]) / SOFT.1[0]).powi(2) + ((y - SOFT.0[1]) / SOFT.1[1]).powi(2)).sqrt();
        ((SOFT_TO - d) / (SOFT_TO - SOFT_FROM)).clamp(0.0, 1.0) as f32
    })
}

/// A fake object: its outline shape and the area that answers a lookup.
#[derive(Clone, Debug)]
struct Fake {
    shape: HoverShape,
    area: Area,
}

#[derive(Clone, Copy, Debug)]
enum Area {
    Box(Rect),
    Round(Ellipse),
}

impl Area {
    fn contains(&self, p: Point) -> bool {
        match self {
            Area::Box(r) => r.contains(p),
            Area::Round(e) => e.contains(p),
        }
    }

    fn polygon(&self, view: &CanvasView) -> Vec<Pos2> {
        match self {
            Area::Box(r) => view.quad(*r).to_vec(),
            Area::Round(e) => {
                const STEPS: usize = 64;
                (0..STEPS)
                    .map(|i| {
                        let a = std::f64::consts::TAU * i as f64 / STEPS as f64;
                        let r = e.radii();
                        view.to_screen(e.center() + kurbo::Vec2::new(r.x * a.cos(), r.y * a.sin()))
                    })
                    .collect()
            }
        }
    }
}

fn fakes() -> [Fake; 3] {
    let round = ellipse(ROUND);
    let soft = ellipse(SOFT);
    [
        Fake { shape: HoverShape::Rect(BOX), area: Area::Box(BOX) },
        Fake { shape: HoverShape::Path(round.to_path(0.1)), area: Area::Round(round) },
        Fake { shape: HoverShape::Mask(soft_mask()), area: Area::Round(soft) },
    ]
}

/// Paints the fake picture over the document and the fake objects over it.
fn paint_objects(ui: &Ui, view: &CanvasView, tokens: &UiTokens, tone: Option<u8>, fakes: &[Fake]) {
    let painter = ui.painter_at(view.rect);
    if let Some(tone) = tone {
        painter.add(Shape::convex_polygon(view.quad(DOC).to_vec(), Color32::from_gray(tone), Stroke::NONE));
    }
    let fill = tokens.color.accent.solid.to_egui().gamma_multiply(0.35);
    for fake in fakes {
        painter.add(Shape::convex_polygon(fake.area.polygon(view), fill, Stroke::NONE));
    }
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

// --- outline ---

#[derive(StoryArgs, Debug, PartialEq)]
struct OutlineArgs {
    #[arg(flatten)]
    view: ViewArgs,
    #[arg(name = "dark picture")]
    dark: bool,
}

/// The three shapes on a light or dark picture, each outlined as the hover would: a rectangle in
/// the style's colour, a path in the layer colour and a mask tint.
fn show_outline(ui: &mut Ui, args: &mut OutlineArgs) {
    let words = StoryLabels::of(ui);
    let own = HoverStoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let tokens = Tokens::of(ui);
    let fakes = fakes();
    paint_objects(ui, &canvas.view, &tokens, Some(if args.dark { DARK_PICTURE } else { LIGHT_PICTURE }), &fakes);
    let layer = tokens.color.status.good.solid.to_egui();
    HoverOutline::new("hover-story-rect", Some(fakes[0].shape.clone())).show(ui, &canvas.view);
    HoverOutline::new("hover-story-path", Some(fakes[1].shape.clone())).color(layer).show(ui, &canvas.view);
    HoverOutline::new("hover-story-mask", Some(fakes[2].shape.clone())).color(layer).show(ui, &canvas.view);
    status(ui, format!("{} · {} · {}", own.rect, own.path, own.mask));
}

// --- lookup ---

#[derive(StoryArgs, Debug, PartialEq)]
struct LookupArgs {
    #[arg(flatten)]
    view: ViewArgs,
    #[arg(name = "empty layer at right")]
    empty_layer: bool,
}

/// What the lookup story keeps between passes.
struct LookupState {
    lookup: HoverLookup<usize>,
    jobs: Arc<ManualJobs>,
    fakes: [Fake; 3],
}

type Shared = Arc<Mutex<LookupState>>;

/// The pointer rests over the fake objects: the lookup waits out the debounce, then its job waits
/// for "Finish next". A found object is outlined and stays shown while the pointer moves inside
/// it; a miss is remembered; a press clears.
fn show_lookup(ui: &mut Ui, args: &mut LookupArgs) {
    let words = StoryLabels::of(ui);
    let own = HoverStoryLabels::of(ui);
    let canvas = document(ui, args.view, &words.document);
    let tokens = Tokens::of(ui);
    let ctx = ui.ctx().clone();
    let key = ui.make_persistent_id("hover-lookup-story");
    let shared: Shared = ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with(key, || {
            let jobs = Arc::new(ManualJobs::new());
            let lookup = HoverLookup::new(&ctx, Some(jobs.clone()));
            Arc::new(Mutex::new(LookupState { lookup, jobs, fakes: fakes() }))
        })
        .clone()
    });
    let Ok(mut guard) = shared.lock() else { return };
    let LookupState { lookup, jobs, fakes } = &mut *guard;
    paint_objects(ui, &canvas.view, &tokens, None, fakes);

    let at = canvas.response.hover_pos().map(|p| canvas.view.to_doc(p)).filter(|p| DOC.contains(*p));
    let empty_here = args.empty_layer && at.is_some_and(|p| p.x >= EMPTY_FROM_X);
    let names = [own.first.as_str(), own.second.as_str(), own.third.as_str()];
    let areas: Vec<Area> = fakes.iter().map(|f| f.area).collect();
    let covers = |found: &usize, p: Point| areas[*found].contains(p);
    let query = HoverQuery { at, revision: 1, key: u64::from(args.empty_layer), enabled: true, empty_here, covers: &covers };
    let update = lookup.update(&ctx, query, |point| {
        let areas = areas.clone();
        move |_: &Cancel| match areas.iter().position(|a| a.contains(point)) {
            Some(found) => HoverAnswer::Found(found),
            None => HoverAnswer::Nothing,
        }
    });
    let (shown, pending, hint) = (update.shown.copied(), update.pending, update.hint);
    if let Some(found) = shown {
        HoverOutline::new("hover-lookup-outline", Some(fakes[found].shape.clone())).show(ui, &canvas.view);
    }
    let waiting = jobs.waiting();
    ui.horizontal(|ui| {
        if ui.add(Button::new(own.finish.as_str())).clicked() {
            jobs.run_next();
            ctx.request_repaint();
        }
        let line = match (shown, hint, pending) {
            (_, Some(HoverHint::EmptySampledLayer), _) => own.empty_layer.clone(),
            (Some(found), _, _) => own.found.replace("{name}", names[found]),
            (None, None, true) => own.waiting.replace("{n}", &waiting.to_string()),
            (None, None, false) if at.is_some() => own.miss.clone(),
            _ => own.nothing_asked.clone(),
        };
        status(ui, line);
    });
}
