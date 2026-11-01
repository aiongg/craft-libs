//! Stories of the live preview (task ST-AD1): not registered. Registering it (task RG2) adds
//! `live_preview::stories` to the crate's `register!` list, [`COMPONENTS`] to its components and
//! [`PreviewStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! The document is `MockImage` and its target is the subject's bounds, so the damage shows
//! without a renderer. Every string goes through words ([`PreviewStoryLabels`]).
//!
//! | Story id | Component | Shows |
//! |---|---|---|
//! | `composites/live-preview/live` | `LivePreview` | A slider changes the settings and so the key; the story outlines the target region, each switch's damage rect and the area `uncovered` returns after the view pans, and reads out the settle state, the proxy factor at the zoom, and a `DraftScale` drag's scale |

#![expect(dead_code, reason = "task RG2 registers the stories")]

use std::time::Duration;

use craft_ui::canvas::kurbo::{Point, Rect};
use craft_ui::canvas::{CanvasView, PixelRect, RasterSource as _};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;
use egui::{Color32, Sense, Shape, Stroke, Ui, vec2};

use super::{DraftScale, LivePreview, PreviewKey, PreviewSession, ProxyPlan, settings_hash};
use crate::ui::mock_image::MockImage;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["LivePreview"];

craft_ui::words! {
    /// The captions of the live preview story, English by default.
    #[non_exhaustive]
    pub(crate) struct PreviewStoryLabels {
        target: "previewStory.target" = "Target region";
        switched: "previewStory.switched" = "Damage when the key switched";
        uncovered: "previewStory.uncovered" = "Uncovered after the view moved";
        covered: "previewStory.covered" = "Covered by the shown key";
        key: "previewStory.key" = "Key {key}";
        no_key: "previewStory.noKey" = "No preview: the committed document";
        settling: "previewStory.settling" = "Settling: secondary views wait";
        settled: "previewStory.settled" = "Settled";
        proxy: "previewStory.proxy" = "Proxy factor {k} at {zoom} % of {megapixels} MP";
        proxy_region: "previewStory.proxyRegion" = "Region on the proxy: {width} × {height}";
        draft: "previewStory.draft" = "Draft scale {scale}";
        no_drag: "previewStory.noDrag" = "No drag: no draft";
    }
}

/// The story above.
pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "composites/live-preview/live",
        Level::Composites,
        "LivePreview",
        "Live preview",
        LiveArgs {
            amount: 40.0,
            preview: true,
            pan: 0.0,
            zoom: 100.0,
            megapixels: 24.0,
            reduced_previews: true,
            drag: false,
            gpu: true,
            render_ms: 10.0
        },
        show_live
    )]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct LiveArgs {
    /// The adjustment's slider; each value is another settings hash and so another key.
    #[arg(range = 0.0..=100.0)]
    amount: f64,
    /// The dialog's Preview check box: off, the host passes `PreviewKey::NONE`.
    preview: bool,
    /// How far the view is panned from the middle, in document pixels.
    #[arg(range = -700.0..=700.0)]
    pan: f64,
    /// The view's zoom, in percent.
    #[arg(range = 10.0..=400.0)]
    zoom: f64,
    /// The document's size for the proxy factor, in megapixels.
    #[arg(range = 1.0..=100.0)]
    megapixels: f64,
    /// Preferences › Performance › Low Resolution Previews.
    reduced_previews: bool,
    /// A slider drag is under way (LightCraft's draft scale).
    drag: bool,
    /// The GPU draws the view.
    gpu: bool,
    /// The fake render time of each draft, in milliseconds.
    #[arg(range = 1.0..=60.0)]
    render_ms: f64,
}

/// What the story keeps between passes: the host's preview state and the damage it last saw.
#[derive(Clone, Default)]
struct LiveState {
    live: LivePreview,
    shown: PreviewKey,
    draft: DraftScale,
    switched: Option<PixelRect>,
    uncovered: Option<PixelRect>,
}

/// The story's document, session and the settle window of the Navigator (PhotoCraft's 0.2 s).
const DOCUMENT: u64 = 1;
const REVISION: u64 = 1;
const TARGET_LAYER: u64 = 1;
const SETTLE_SECONDS: f64 = 0.2;
/// The legend's sample line, in points (a story-local size: no token names a legend).
const LEGEND_LINE: f32 = 24.0;
/// The slider a draft scale times.
const DRAG_CONTROL: &str = "amount";

fn outline(painter: &egui::Painter, view: &CanvasView, rect: PixelRect, stroke: Stroke) {
    let doc = Rect::new(f64::from(rect.x0), f64::from(rect.y0), f64::from(rect.x1), f64::from(rect.y1));
    painter.add(Shape::closed_line(view.quad(doc).to_vec(), stroke));
}

fn fill(text: &str, pairs: &[(&str, &str)]) -> String {
    craft_i18n::fmt(text, pairs)
}

fn show_live(ui: &mut Ui, args: &mut LiveArgs) {
    let labels = PreviewStoryLabels::of(ui);
    let numbers = LocaleNumbers::of(ui);
    let tokens = Tokens::of(ui);
    let colors = &tokens.color;
    let (target_colour, switch_colour, uncovered_colour, covered_colour): (Color32, Color32, Color32, Color32) =
        (colors.text.muted.to_egui(), colors.accent.strong.to_egui(), colors.status.warn.fg.to_egui(), colors.text.faint.to_egui());
    let line = ui.visuals().widgets.active.fg_stroke.width;
    let id = ui.scope_id().with("live-preview-story");
    let mut state = ui.data(|d| d.get_temp::<LiveState>(id)).unwrap_or_default();
    let now = ui.input(|i| i.time);

    // The document and the view: the pan moves the view's middle.
    let image = MockImage::default();
    let size = image.size();
    let subject = image.subject_bounds();
    let region = PixelRect::new(subject.x0, subject.y0, subject.x1, subject.y1);
    let centre = Point::new(f64::from(size[0]) / 2.0 + args.pan, f64::from(size[1]) / 2.0);
    let (_response, view) = image.canvas(ui, "live-preview", ui.available_width(), Some(args.zoom), Some(centre));
    let visible = PixelRect::visible(&view, size);

    // The host's per-pass protocol (see the module docs of `live_preview`).
    state.live.begin(PreviewSession::new(DOCUMENT, REVISION, TARGET_LAYER), region);
    let key = if args.preview {
        let amount = args.amount.round().to_le_bytes();
        state.live.key(settings_hash(&[b"amount".as_slice(), amount.as_slice()]), now)
    } else {
        PreviewKey::NONE
    };
    if key != state.shown {
        state.switched = state.live.switch(state.shown, key, visible);
        state.uncovered = None;
    }
    if let Some(rect) = state.live.uncovered(key, visible) {
        state.uncovered = Some(rect);
    }
    state.shown = key;

    let painter = ui.painter_at(view.rect);
    let stroke = |colour| Stroke::new(line, colour);
    outline(&painter, &view, region, stroke(target_colour));
    if let Some((_, covered)) = state.live.covered {
        outline(&painter, &view, covered, stroke(covered_colour));
    }
    if let Some(rect) = state.switched {
        outline(&painter, &view, rect, stroke(switch_colour));
    }
    if let Some(rect) = state.uncovered {
        outline(&painter, &view, rect, stroke(uncovered_colour));
    }

    // The legend: each outline's colour names what it is.
    let legend = |ui: &mut Ui, caption: &str, colour: Color32, rect: Option<PixelRect>| {
        let text = match rect {
            Some(r) => format!("{caption}: {}, {}, {}, {}", r.x0, r.y0, r.x1, r.y1),
            None => caption.to_owned(),
        };
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(LEGEND_LINE, ui.spacing().interact_size.y), Sense::hover());
            ui.painter().hline(rect.x_range(), rect.center().y, Stroke::new(line, colour));
            ui.add(Text::new(text.as_str()));
        });
    };
    legend(ui, &labels.target, target_colour, Some(region));
    legend(ui, &labels.covered, covered_colour, state.live.covered.map(|(_, rect)| rect));
    legend(ui, &labels.switched, switch_colour, state.switched);
    legend(ui, &labels.uncovered, uncovered_colour, state.uncovered);

    // The readouts.
    let key_text = if key.is_none() { labels.no_key.clone() } else { fill(&labels.key, &[("key", &format!("{:x}", key.raw()))]) };
    ui.add(Text::new(key_text.as_str()).content(Content::Muted));
    let settling = state.live.settling(now, SETTLE_SECONDS);
    ui.add(Text::new(if settling { labels.settling.as_str() } else { labels.settled.as_str() }).content(Content::Muted));
    if settling {
        ui.ctx().request_repaint_after(Duration::from_millis(50));
    }

    // The proxy: a document of `megapixels` at the view's zoom, in physical pixels per document pixel.
    let pixels = (args.megapixels * 1_000_000.0) as u64;
    let device_zoom = (args.zoom / 100.0) as f32 * ui.pixels_per_point();
    let k = ProxyPlan::factor(device_zoom, pixels, args.reduced_previews);
    let proxy_text = fill(
        &labels.proxy,
        &[
            ("k", &numbers.format(f64::from(k), 0..=0)),
            ("zoom", &numbers.format(args.zoom, 0..=0)),
            ("megapixels", &numbers.format(args.megapixels, 0..=0)),
        ],
    );
    ui.add(Text::new(proxy_text.as_str()).content(Content::Muted));
    let scaled = ProxyPlan::scaled(region, k);
    let region_text = fill(
        &labels.proxy_region,
        &[("width", &numbers.format(f64::from(scaled.width()), 0..=0)), ("height", &numbers.format(f64::from(scaled.height()), 0..=0))],
    );
    ui.add(Text::new(region_text.as_str()).content(Content::Muted));

    // LightCraft's draft scale: chosen when a drag starts, timed with the fake render time.
    let drag = args.drag.then_some((DOCUMENT, DRAG_CONTROL));
    let scale = state.draft.scale(drag, args.gpu, Some(key.raw()), now);
    if scale.is_some() {
        state.draft.observe(key.raw(), args.render_ms, None);
    }
    let draft_text = match scale {
        Some(scale) => fill(&labels.draft, &[("scale", &numbers.format(f64::from(scale), 0..=1))]),
        None => labels.no_drag.clone(),
    };
    ui.add(Text::new(draft_text.as_str()).content(Content::Muted));

    ui.data_mut(|d| d.insert_temp(id, state));
}

#[cfg(test)]
mod tests {
    use craft_ui::theme::{Base, install};
    use egui::{Context, RawInput};

    // Every story draws its default state, twice (the second pass reads the kept cell values).
    #[test]
    fn every_story_shows_with_its_default_args() {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        for story in super::stories() {
            let mut args = story.defaults();
            for _ in 0..2 {
                ctx.run_ui(RawInput::default(), |ui| (story.show)(ui, &mut args)).drop_without_applying_deltas();
            }
        }
    }
}
