//! Stories of the Layer Style dialog (task ST-AD2): not registered. Registering them (task RG2)
//! adds `layer_style::stories` to the crate's `register!` list, [`COMPONENTS`] to the registry and
//! [`LayerStyleStoryLabels`] to `WORDS`, and regenerates the registry dump.
//!
//! The story is the host: it keeps one [`LayerStyleDraft`] per story instance and applies the
//! dialog's edit events (Esc restores the draft kept at `Begin`), answers the colour requests, the
//! Styles page and the contour library, and draws a fake picture (a rounded shape on the
//! checkerboard) with a rough shadow painted from the draft's Drop Shadow, so a drag on the
//! picture moves the shadow without an engine. Every string goes through story words.
//!
//! Ids start with their level (the gallery requires it), so the spec's `imaging/layer-style/*` are:
//!
//! | Story id | Component | Shows |
//! |---|---|---|
//! | `patterns/layer-style/dialog` | `LayerStyleDialog` | Drop Shadow selected over the fake picture; a drag on the picture moves the shadow |
//! | `patterns/layer-style/blending` | `LayerStyleDialog` | Blending Options with Blend If on Gray, R, G, B; a switch to CMYK |
//! | `patterns/layer-style-effects/<kind>` | `LayerStyleDialog` | One story per effect page, ten ids (`bevel-emboss`, `stroke`, `inner-shadow`, `inner-glow`, `satin`, `color-overlay`, `gradient-overlay`, `pattern-overlay`, `outer-glow`, `drop-shadow`); the shadow kinds with the picture |
//! | `patterns/layer-style/styles` | `LayerStyleDialog` | The Styles page with fake thumbnails; a pick applies a preset |
//! | `composites/contour-field/editor` | `ContourField` | The contour field, its picker with a host library and the Contour Editor |
//! | `composites/blend-if-bar/handles` | `BlendIfBar` | One Blend If bar unsplit and one split, with the keys |

#![expect(dead_code, reason = "task RG2 registers the stories")]

use craft_theme::Srgba;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasView, Checkerboard};
use craft_ui::controls::BlendChoice;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::presets::{PresetRow, PresetState};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, NoArgs, Story, StoryArgs, StoryChoice, lock, shared};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens};
use craft_ui::typed_story;
use egui::{Color32, CornerRadius, Painter, Rect, Sense, Stroke, StrokeKind, Ui, vec2};

use super::{BlendIfBar, ContourAction, ContourField, LayerStyleDialog, LayerStyleLabels, PatternChoice, StylePresets};
use crate::layer_style::{
    BlendIf, BlendIfRange, Blending, Contour, ContourPoint, EffectKind, EffectSpec, LayerStyle, LayerStyleDraft, LayerStylePage, StyleParams,
    StyleValue,
};

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["LayerStyleDialog", "ContourField", "BlendIfBar"];

craft_ui::words! {
    /// The captions and sample data of the Layer Style stories, English by default.
    #[non_exhaustive]
    pub(crate) struct LayerStyleStoryLabels {
        red: "layerStyleStory.red" = "R";
        green: "layerStyleStory.green" = "G";
        blue: "layerStyleStory.blue" = "B";
        cyan: "layerStyleStory.cyan" = "C";
        magenta: "layerStyleStory.magenta" = "M";
        yellow: "layerStyleStory.yellow" = "Y";
        black: "layerStyleStory.black" = "K";
        mode_normal: "layerStyleStory.modeNormal" = "Normal";
        mode_dissolve: "layerStyleStory.modeDissolve" = "Dissolve";
        mode_multiply: "layerStyleStory.modeMultiply" = "Multiply";
        mode_screen: "layerStyleStory.modeScreen" = "Screen";
        mode_overlay: "layerStyleStory.modeOverlay" = "Overlay";
        mode_soft_light: "layerStyleStory.modeSoftLight" = "Soft Light";
        mode_difference: "layerStyleStory.modeDifference" = "Difference";
        pattern_weave: "layerStyleStory.patternWeave" = "Diagonal Weave";
        pattern_dots: "layerStyleStory.patternDots" = "Dots";
        group_basics: "layerStyleStory.groupBasics" = "Basics";
        preset_shadow: "layerStyleStory.presetShadow" = "Soft Shadow";
        preset_outline: "layerStyleStory.presetOutline" = "Outline Glow";
        preset_raised: "layerStyleStory.presetRaised" = "Raised Button";
        picture_hint: "layerStyleStory.pictureHint" = "Drag on the picture to move the shadow.";
        applied_style: "layerStyleStory.appliedStyle" = "Applied style: {name}";
        asked_new_style: "layerStyleStory.askedNewStyle" = "New Style… asked.";
        asked_make_default: "layerStyleStory.askedMakeDefault" = "Make Default asked for {kind}.";
        asked_reset: "layerStyleStory.askedReset" = "Reset to Default asked for {id}.";
        asked_load: "layerStyleStory.askedLoad" = "Load Contours… asked.";
        asked_replace: "layerStyleStory.askedReplace" = "Replace Contours… asked.";
        asked_save: "layerStyleStory.askedSave" = "Save Contours… asked.";
        deleted_contour: "layerStyleStory.deletedContour" = "Deleted {name} from the library.";
        added_contour: "layerStyleStory.addedContour" = "Added {name} to the library.";
        library_ramp: "layerStyleStory.libraryRamp" = "Soft Ramp";
        library_notch: "layerStyleStory.libraryNotch" = "Notch";
        contour_now: "layerStyleStory.contourNow" = "Contour: {name}, {n} points.";
        bars_hint: "layerStyleStory.barsHint" = "Tab to a handle. Arrows move one level, Shift ten, Alt one half.";
        unsplit: "layerStyleStory.unsplit" = "Unsplit pairs";
        split: "layerStyleStory.split" = "Split pairs (Alt-drag splits a pair)";
    }
}

/// The fifteen stories above.
pub(crate) fn stories() -> Vec<Story> {
    macro_rules! effect {
        ($id:literal, $name:literal, $kind:expr) => {
            typed_story!($id, Level::Patterns, "LayerStyleDialog", $name, NoArgs {}, |ui: &mut Ui, _: &mut NoArgs| show_effect(ui, $kind))
        };
    }
    vec![
        typed_story!("patterns/layer-style/dialog", Level::Patterns, "LayerStyleDialog", "Dialog", NoArgs {}, show_dialog),
        typed_story!(
            "patterns/layer-style/blending",
            Level::Patterns,
            "LayerStyleDialog",
            "Blending Options",
            BlendingArgs { space: Space::Rgb },
            show_blending
        ),
        effect!("patterns/layer-style-effects/bevel-emboss", "Bevel & Emboss", EffectKind::BevelEmboss),
        effect!("patterns/layer-style-effects/stroke", "Stroke", EffectKind::Stroke),
        effect!("patterns/layer-style-effects/inner-shadow", "Inner Shadow", EffectKind::InnerShadow),
        effect!("patterns/layer-style-effects/inner-glow", "Inner Glow", EffectKind::InnerGlow),
        effect!("patterns/layer-style-effects/satin", "Satin", EffectKind::Satin),
        effect!("patterns/layer-style-effects/color-overlay", "Color Overlay", EffectKind::ColorOverlay),
        effect!("patterns/layer-style-effects/gradient-overlay", "Gradient Overlay", EffectKind::GradientOverlay),
        effect!("patterns/layer-style-effects/pattern-overlay", "Pattern Overlay", EffectKind::PatternOverlay),
        effect!("patterns/layer-style-effects/outer-glow", "Outer Glow", EffectKind::OuterGlow),
        effect!("patterns/layer-style-effects/drop-shadow", "Drop Shadow", EffectKind::DropShadow),
        typed_story!("patterns/layer-style/styles", Level::Patterns, "LayerStyleDialog", "Styles", NoArgs {}, show_styles),
        typed_story!("composites/contour-field/editor", Level::Composites, "ContourField", "Editor", NoArgs {}, show_contour),
        typed_story!("composites/blend-if-bar/handles", Level::Composites, "BlendIfBar", "Handles", NoArgs {}, show_bars),
    ]
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum Space {
    #[choice(name = "RGB")]
    Rgb,
    #[choice(name = "CMYK")]
    Cmyk,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BlendingArgs {
    /// The document's colour channels.
    space: Space,
}

/// The fake picture's extent in document pixels: a demo document, not a UI size.
const PICTURE: [u32; 2] = [220, 170];
/// The shape's share of the picture's width and height.
const SHAPE_SHARE: f32 = 0.55;
/// The shape's corner radius as a share of its height.
const SHAPE_ROUND: f32 = 0.24;
/// Layers the rough shadow is built from: more look smoother, one is a hard shadow.
const SHADOW_LAYERS: u32 = 6;

/// What a story keeps for one instance.
struct Rig {
    draft: LayerStyleDraft,
    /// The draft at `Begin` of the gesture in progress.
    before: Option<LayerStyleDraft>,
    presets: PresetState<String>,
    rows: Vec<PresetRow<String>>,
    library: Vec<Contour>,
    note: String,
}

impl Rig {
    fn new(draft: LayerStyleDraft, words: &LayerStyleStoryLabels) -> Self {
        let rows = vec![
            PresetRow::group("basics".to_owned(), words.group_basics.as_str(), 0),
            PresetRow::item("0".to_owned(), words.preset_shadow.as_str(), 1),
            PresetRow::item("1".to_owned(), words.preset_outline.as_str(), 1),
            PresetRow::item("2".to_owned(), words.preset_raised.as_str(), 1),
        ];
        Rig { draft, before: None, presets: PresetState::new(), rows, library: library(words), note: String::new() }
    }
}

/// The host's contour library: two contours of the story's own.
fn library(words: &LayerStyleStoryLabels) -> Vec<Contour> {
    let point = |input: f32, output: f32| ContourPoint { input, output, corner: false };
    [
        (words.library_ramp.as_str(), vec![point(0.0, 0.0), point(0.5, 0.25), point(1.0, 1.0)]),
        (words.library_notch.as_str(), vec![point(0.0, 0.0), point(0.4, 0.8), point(0.6, 0.2), point(1.0, 1.0)]),
    ]
    .into_iter()
    .filter_map(|(name, points)| Contour::from_points(name, points))
    .collect()
}

/// Applies an edit of the dialog, a contour or a bar: the value follows `Update` and `Set`, a
/// `Cancel` puts back what the gesture's `Begin` found.
fn apply_edit<T: Clone>(event: Option<EditEvent<T>>, value: &mut T, before: &mut Option<T>) {
    match event {
        Some(EditEvent::Begin { value: first, .. }) => {
            before.get_or_insert_with(|| value.clone());
            if let Some(first) = first {
                *value = first;
            }
        }
        Some(EditEvent::Update { value: next, .. } | EditEvent::Set { value: next, .. }) => *value = next,
        Some(EditEvent::Commit { value: next, .. }) => {
            *value = next;
            *before = None;
        }
        Some(EditEvent::Cancel { .. }) => {
            if let Some(kept) = before.take() {
                *value = kept;
            }
        }
        _ => {}
    }
}

fn channels(words: &LayerStyleStoryLabels, space: Space) -> Vec<String> {
    match space {
        Space::Rgb => vec![words.red.clone(), words.green.clone(), words.blue.clone()],
        Space::Cmyk => vec![words.cyan.clone(), words.magenta.clone(), words.yellow.clone(), words.black.clone()],
    }
}

fn blend_modes(words: &LayerStyleStoryLabels) -> Vec<BlendChoice> {
    let mode = |key: &str, label: &str, separator_before: bool| BlendChoice { key: key.to_owned(), label: label.to_owned(), separator_before };
    vec![
        mode("Normal", &words.mode_normal, false),
        mode("Dissolve", &words.mode_dissolve, false),
        mode("Multiply", &words.mode_multiply, true),
        mode("Screen", &words.mode_screen, true),
        mode("Overlay", &words.mode_overlay, true),
        mode("Soft Light", &words.mode_soft_light, false),
        mode("Difference", &words.mode_difference, true),
    ]
}

fn patterns(words: &LayerStyleStoryLabels) -> Vec<PatternChoice> {
    vec![
        PatternChoice { key: "weave".to_owned(), label: words.pattern_weave.clone() },
        PatternChoice { key: "dots".to_owned(), label: words.pattern_dots.clone() },
    ]
}

/// The style presets the Styles page offers, by row key: the effects each one sets.
fn preset(key: &str) -> Vec<(EffectKind, bool, StyleParams)> {
    let kinds: &[EffectKind] = match key {
        "0" => &[EffectKind::DropShadow],
        "1" => &[EffectKind::Stroke, EffectKind::OuterGlow],
        _ => &[EffectKind::BevelEmboss, EffectKind::InnerShadow],
    };
    kinds.iter().map(|&kind| (kind, true, EffectSpec::photoshop(kind).defaults)).collect()
}

/// A thumbnail of preset `key`: the shape with a shadow, an outline or a light edge.
fn paint_thumb(painter: &Painter, rect: Rect, key: &str, tokens: &UiTokens) {
    let (back, shape, edge) = (tokens.color.surface.sunken.to_egui(), tokens.color.accent.strong.to_egui(), tokens.color.text.muted.to_egui());
    painter.rect_filled(rect, CornerRadius::ZERO, back);
    let body = Rect::from_center_size(rect.center(), rect.size() * SHAPE_SHARE);
    let round = CornerRadius::same((body.height() * SHAPE_ROUND).clamp(0.0, 255.0) as u8);
    match key {
        "0" => {
            painter.rect_filled(body.translate(vec2(body.width() * 0.08, body.height() * 0.1)), round, edge);
            painter.rect_filled(body, round, shape);
        }
        "1" => {
            painter.rect_filled(body, round, shape);
            painter.rect_stroke(body.expand(body.height() * 0.1), round, Stroke::new(body.height() * 0.1, edge), StrokeKind::Outside);
        }
        _ => {
            painter.rect_filled(body, round, shape);
            painter.rect_stroke(body, round, Stroke::new(body.height() * 0.08, edge), StrokeKind::Inside);
        }
    }
}

/// What the dialog needs beside the rig.
struct Env {
    words: LayerStyleStoryLabels,
    channels: Vec<String>,
}

/// Draws the dialog body over the rig's draft and applies what it reports.
fn run_dialog(ui: &mut Ui, rig: &mut Rig, env: &Env) {
    let numbers = LocaleNumbers::of(ui);
    let modes = blend_modes(&env.words);
    let pattern_list = patterns(&env.words);
    let tokens = Tokens::of(ui);
    let out = {
        let Rig { draft, presets, rows, library, .. } = &mut *rig;
        let thumbs = StylePresets::new(presets, rows, |painter, rect, info| paint_thumb(painter, rect, info.key, &tokens));
        LayerStyleDialog::new("layer-style", draft, &env.channels, &numbers)
            .blend_modes(&modes)
            .patterns(&pattern_list)
            .user_contours(library)
            .style_presets(thumbs)
            .show(ui)
    };
    apply_edit(out.edit.event, &mut rig.draft, &mut rig.before);
    if let Some(request) = out.pick_color {
        // The colour picker is the host's: answer with the accent, or the error colour when the
        // swatch already shows the accent.
        let accent = Srgba::from(tokens.color.accent.strong);
        let answer = if request.color.same_rgb8(accent) { Srgba::from(tokens.color.status.bad.fg) } else { accent };
        LayerStyleDialog::set_color(&mut rig.draft, &request.key, answer);
    }
    if let Some(key) = out.apply_style {
        rig.draft.apply_preset(preset(&key));
        let name = rig.rows.iter().find(|row| row.key == key).map(|row| row.label.clone()).unwrap_or_default();
        rig.note = craft_i18n::fmt(&env.words.applied_style, &[("name", name.as_str())]);
    }
    if out.new_style {
        rig.note = env.words.asked_new_style.clone();
    }
    if let Some((kind, _)) = out.make_default {
        rig.note = craft_i18n::fmt(&env.words.asked_make_default, &[("kind", LayerStyleLabels::of(ui).effect(kind))]);
    }
    if let Some(id) = out.reset_to_default {
        rig.note = craft_i18n::fmt(&env.words.asked_reset, &[("id", id.key().as_str())]);
    }
    if let Some(action) = out.contour_action {
        rig.note = contour_note(&env.words, action, &mut rig.library);
    }
}

/// Answers a contour library request and words it.
fn contour_note(words: &LayerStyleStoryLabels, action: ContourAction, library: &mut Vec<Contour>) -> String {
    match action {
        ContourAction::Load => words.asked_load.clone(),
        ContourAction::Replace => words.asked_replace.clone(),
        ContourAction::Save => words.asked_save.clone(),
        ContourAction::Delete(index) if index < library.len() => {
            let gone = library.remove(index);
            craft_i18n::fmt(&words.deleted_contour, &[("name", gone.name.as_str())])
        }
        ContourAction::New(contour) => {
            let text = craft_i18n::fmt(&words.added_contour, &[("name", contour.name.as_str())]);
            library.push(contour);
            text
        }
        _ => String::new(),
    }
}

/// The rough shadow of the first Drop Shadow that is on: colour, opacity, offset and blur read
/// from its parameters over the spec's defaults.
struct RoughShadow {
    colour: Color32,
    opacity: f32,
    /// The offset in document pixels, y down.
    offset: [f64; 2],
    blur: f64,
}

impl RoughShadow {
    fn of(draft: &LayerStyleDraft) -> Option<Self> {
        let instance = draft.style.effects.iter().find(|e| e.on && e.kind == EffectKind::DropShadow)?;
        let params = instance.params.over(&EffectSpec::photoshop(EffectKind::DropShadow).defaults);
        let number = |key: &str| params.number(key).unwrap_or(0.0);
        let colour = match params.get("color") {
            Some(StyleValue::Color(colour)) => Color32::from(*colour),
            _ => Color32::BLACK,
        };
        let angle = number("angle").to_radians();
        let distance = number("distance");
        Some(RoughShadow {
            colour,
            opacity: (number("opacity") / 100.0) as f32,
            offset: [-angle.cos() * distance, angle.sin() * distance],
            blur: number("size"),
        })
    }
}

/// The fake picture with its rough shadow; a drag on it moves the selected shadow page's shadow.
fn picture(ui: &mut Ui, draft: &mut LayerStyleDraft) {
    let ppp = ui.pixels_per_point();
    // Points per document pixel: one document pixel is one device pixel.
    let scale = CanvasView::points_per_unit(1.0, ppp);
    let size = vec2(PICTURE[0] as f32 * scale as f32, PICTURE[1] as f32 * scale as f32);
    let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
    let view = CanvasView::new(rect, Point::new(f64::from(PICTURE[0]) / 2.0, f64::from(PICTURE[1]) / 2.0), scale);
    let painter = ui.painter_at(rect);
    Checkerboard::default().paint(&painter, &view, PICTURE);

    let tokens = Tokens::of(ui);
    let body = Rect::from_center_size(rect.center(), size * SHAPE_SHARE);
    let round = body.height() * SHAPE_ROUND;
    if let Some(shadow) = RoughShadow::of(draft) {
        let offset = vec2(shadow.offset[0] as f32 * scale as f32, shadow.offset[1] as f32 * scale as f32);
        let blur = (shadow.blur * scale) as f32;
        let alpha = shadow.opacity / SHADOW_LAYERS as f32;
        for layer in 0..SHADOW_LAYERS {
            let grow = blur * layer as f32 / SHADOW_LAYERS as f32;
            let radius = CornerRadius::same((round + grow).clamp(0.0, 255.0) as u8);
            painter.rect_filled(body.translate(offset).expand(grow), radius, shadow.colour.gamma_multiply(alpha));
        }
    }
    painter.rect_filled(body, CornerRadius::same(round.clamp(0.0, 255.0) as u8), tokens.color.accent.strong.to_egui());

    if response.drag_started() {
        draft.drag_shadow(true, [0.0, 0.0]);
    } else if response.dragged() {
        let total = ui.input(|i| i.pointer.latest_pos().zip(i.pointer.press_origin()).map(|(now, start)| now - start));
        if let Some(total) = total {
            draft.drag_shadow(false, [f64::from(total.x) / scale, f64::from(total.y) / scale]);
        }
    }
    if response.drag_stopped() {
        draft.shadow_drag = None;
    }
}

/// The dialog beside the picture (when `with_picture`), and the host's last answer under them.
fn host(ui: &mut Ui, rig: &mut Rig, env: &Env, with_picture: bool) {
    ui.horizontal_top(|ui| {
        run_dialog(ui, rig, env);
        if with_picture {
            ui.vertical(|ui| {
                picture(ui, &mut rig.draft);
                ui.add(Text::new(env.words.picture_hint.as_str()).content(Content::Muted));
            });
        }
    });
    if !rig.note.is_empty() {
        ui.add(Text::new(rig.note.as_str()).content(Content::Muted));
    }
}

fn show_dialog(ui: &mut Ui, _args: &mut NoArgs) {
    let words = LayerStyleStoryLabels::of(ui);
    let state = shared(ui, "layer-style", || {
        let mut draft = LayerStyleDraft::default();
        draft.open_on(EffectKind::DropShadow);
        Rig::new(draft, &words)
    });
    let env = Env { channels: channels(&words, Space::Rgb), words };
    host(ui, &mut lock(&state), &env, true);
}

/// The Blending Options draft of a document with `channels` colour channels: a Multiply layer at
/// 80 %, with split Blend If ranges on the Gray entry.
fn blending_of(channels: usize) -> Blending {
    let mut blending = Blending {
        blend: "Multiply".into(),
        opacity: 80.0,
        channels: vec![true; channels],
        blend_if: (0..=channels).map(|channel| BlendIf { channel, ..BlendIf::default() }).collect(),
        ..Blending::default()
    };
    if let Some(gray) = blending.blend_if.first_mut() {
        gray.this_layer = BlendIfRange::ordered([0, 40, 200, 255]);
        gray.underlying = BlendIfRange::ordered([0, 0, 120, 200]);
    }
    blending
}

fn show_blending(ui: &mut Ui, args: &mut BlendingArgs) {
    let words = LayerStyleStoryLabels::of(ui);
    let names = channels(&words, args.space);
    let state = shared(ui, "layer-style", || {
        let style = LayerStyle { blending: blending_of(names.len()), ..LayerStyle::default() };
        Rig::new(LayerStyleDraft::new(style), &words)
    });
    let mut rig = lock(&state);
    if rig.draft.style.blending.channels.len() != names.len() {
        rig.draft.style.blending = blending_of(names.len());
        rig.draft.blend_if_channel = 0;
        rig.draft.page = LayerStylePage::Blending;
    }
    host(ui, &mut rig, &Env { channels: names, words }, false);
}

fn show_effect(ui: &mut Ui, kind: EffectKind) {
    let words = LayerStyleStoryLabels::of(ui);
    let state = shared(ui, "layer-style", || {
        let mut draft = LayerStyleDraft::default();
        draft.open_on(kind);
        Rig::new(draft, &words)
    });
    let env = Env { channels: channels(&words, Space::Rgb), words };
    host(ui, &mut lock(&state), &env, kind.drags_shadow());
}

fn show_styles(ui: &mut Ui, _args: &mut NoArgs) {
    let words = LayerStyleStoryLabels::of(ui);
    let state = shared(ui, "layer-style", || {
        let mut draft = LayerStyleDraft::default();
        draft.open_on(EffectKind::DropShadow);
        draft.page = LayerStylePage::Styles;
        Rig::new(draft, &words)
    });
    let env = Env { channels: channels(&words, Space::Rgb), words };
    host(ui, &mut lock(&state), &env, false);
}

/// The contour story's state.
struct ContourRig {
    contour: Contour,
    before: Option<Contour>,
    anti_alias: bool,
    library: Vec<Contour>,
    note: String,
}

fn show_contour(ui: &mut Ui, _args: &mut NoArgs) {
    let words = LayerStyleStoryLabels::of(ui);
    let labels = LayerStyleLabels::of(ui);
    let state = shared(ui, "contour", || ContourRig {
        contour: Contour::presets().into_iter().nth(1).unwrap_or_else(Contour::linear),
        before: None,
        anti_alias: false,
        library: library(&words),
        note: String::new(),
    });
    let mut guard = lock(&state);
    let rig = &mut *guard;
    let out =
        ContourField::new("contour", labels.contour.as_str(), &rig.contour).anti_alias(Some(rig.anti_alias)).user_contours(&rig.library).show(ui);
    apply_edit(out.edit, &mut rig.contour, &mut rig.before);
    if let Some(on) = out.anti_alias {
        rig.anti_alias = on;
    }
    if let Some(action) = out.action {
        rig.note = contour_note(&words, action, &mut rig.library);
    }
    let points = rig.contour.points.len().to_string();
    let now = craft_i18n::fmt(&words.contour_now, &[("name", rig.contour.name.as_str()), ("n", points.as_str())]);
    ui.add(Text::new(now.as_str()).content(Content::Muted));
    if !rig.note.is_empty() {
        ui.add(Text::new(rig.note.as_str()).content(Content::Muted));
    }
}

/// The bar story's state: an unsplit and a split range, and the value each gesture began from.
struct BarRig {
    ranges: [BlendIfRange; 2],
    before: [Option<BlendIfRange>; 2],
}

fn show_bars(ui: &mut Ui, _args: &mut NoArgs) {
    let words = LayerStyleStoryLabels::of(ui);
    let labels = LayerStyleLabels::of(ui);
    let state = shared(ui, "bars", || BarRig { ranges: [BlendIfRange([30, 30, 220, 220]), BlendIfRange([10, 60, 190, 240])], before: [None, None] });
    let mut guard = lock(&state);
    let rig = &mut *guard;
    ui.add(Text::new(words.bars_hint.as_str()).content(Content::Muted));
    let layer = labels.this_layer.trim_end_matches([':', '\u{ff1a}']).to_owned();
    for (index, caption) in [&words.unsplit, &words.split].into_iter().enumerate() {
        ui.add(Text::new(caption.as_str()).content(Content::Muted));
        let out = BlendIfBar::new(("bar", index), layer.as_str(), rig.ranges[index]).show(ui);
        apply_edit(out.event, &mut rig.ranges[index], &mut rig.before[index]);
    }
}
