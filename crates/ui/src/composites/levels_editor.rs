//! [`LevelsEditor`]: a levels adjustment's histogram, input thumbs and fields, output bar, thumbs
//! and fields, editing one channel's [`Levels`].

use std::collections::VecDeque;
use std::ops::RangeInclusive;

use craft_theme::{ControlSize, Rgba, States, TypeRole};
use egui::accesskit::Role;
use egui::{Align, AsIdSalt, Color32, Context, Id, IdSalt, Key, Layout, Modifiers, Rect, Sense, Ui, UiBuilder, pos2, vec2};

use super::curve_editor::{gradient, story_bins};
use super::histogram::{ChannelColor, Histogram, HistogramData};
use crate::controls::NumberField;
use crate::controls::thumb_row::{Follow, Order, Thumb, ThumbRow, ThumbRowStyle};
use crate::edit::{self, CancelReason, EditEvent, EditPolicy, EditResponse, Gesture, GestureHost, UndoMarks, parts};
use crate::localized::{LocaleNumbers, Unit};
use crate::primitives::{Content, Gap, Stack, Surface, SurfaceKind, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// One channel's levels: the input black and white points with the gamma between them, and the
/// output black and white points.
#[derive(Clone, Copy, Debug)]
pub struct Levels {
    pub in_black: f64,
    /// The midtone gamma: above 1 brightens.
    pub gamma: f64,
    pub in_white: f64,
    /// Output black; above `out_white` it inverts.
    pub out_black: f64,
    pub out_white: f64,
}

/// Levels are equal when every value is, an unset (NaN) value equal to another: the edit
/// machinery compares values to tell a change from none, and a value that is never equal to
/// itself reads as an outside change of every gesture.
impl PartialEq for Levels {
    fn eq(&self, other: &Self) -> bool {
        let same = |a: f64, b: f64| a == b || (a.is_nan() && b.is_nan());
        same(self.in_black, other.in_black)
            && same(self.gamma, other.gamma)
            && same(self.in_white, other.in_white)
            && same(self.out_black, other.out_black)
            && same(self.out_white, other.out_white)
    }
}

impl Levels {
    /// The levels that change nothing on `range`: the input and output ends at its ends, gamma 1.
    pub fn identity(range: RangeInclusive<f64>) -> Self {
        let (start, end) = range.into_inner();
        Self { in_black: start, gamma: 1.0, in_white: end, out_black: start, out_white: end }
    }

    fn input(&self) -> [f64; 3] {
        [self.in_black, self.gamma, self.in_white]
    }

    fn output(&self) -> [f64; 2] {
        [self.out_black, self.out_white]
    }

    fn with_input(self, [in_black, gamma, in_white]: [f64; 3]) -> Self {
        Self { in_black, gamma, in_white, ..self }
    }

    fn with_output(self, [out_black, out_white]: [f64; 2]) -> Self {
        Self { out_black, out_white, ..self }
    }
}

/// Which input end an Alt (Option) drag is moving, for the host's clipping view
/// ([`LevelsEditor::clipping`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LevelsClip {
    /// The input black thumb: show what clips to black.
    Shadows,
    /// The input white thumb: show what clips to white.
    Highlights,
}

/// The editor's texts, whole phrases the host translates. Each thumb and the field under it share
/// a name: one value, two ways to set it.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelsLabels {
    /// The histogram's accessible name.
    pub histogram: String,
    /// The caption over the histogram and the input row.
    pub input: String,
    /// The caption over the output bar.
    pub output: String,
    pub in_black: String,
    pub gamma: String,
    pub in_white: String,
    pub out_black: String,
    pub out_white: String,
}

impl Default for LevelsLabels {
    fn default() -> Self {
        Self {
            histogram: "Histogram".to_owned(),
            input: "Input Levels".to_owned(),
            output: "Output Levels".to_owned(),
            in_black: "Input black".to_owned(),
            gamma: "Gamma".to_owned(),
            in_white: "Input white".to_owned(),
            out_black: "Output black".to_owned(),
            out_white: "Output white".to_owned(),
        }
    }
}

/// A levels editor's sizes and colours, from [`LevelsEditor::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LevelsStyle {
    /// The width range the editor takes of the available width: `levels.min_width`,
    /// `levels.max_width`.
    pub min_width: f32,
    pub max_width: f32,
    /// Height of a row of thumbs: `levels.row`.
    pub row: f32,
    /// A triangle thumb: `levels.thumb_width` × `levels.thumb_height`.
    pub thumb_width: f32,
    pub thumb_height: f32,
    /// Distance from a thumb's centre within which a press grabs it: `levels.hit`.
    pub hit: f32,
    /// Height of the output bar: `levels.bar`.
    pub bar: f32,
    /// A thumb's edge, and a disabled one's: `levels.thumb_edge`, `levels.disabled_edge`.
    pub thumb_edge: Color32,
    pub disabled_edge: Color32,
    /// The ring round a hovered or pressed thumb: `levels.ring`, `slider.ring_width` wide.
    pub ring: States<Color32>,
    pub ring_width: f32,
    /// The captions' role; they are muted text.
    pub caption_role: TypeRole,
    /// Between the fields under a row: `gap.inline`.
    pub field_gap: f32,
}

impl From<&LevelsStyle> for ThumbRowStyle {
    fn from(style: &LevelsStyle) -> Self {
        Self {
            height: style.row,
            thumb: vec2(style.thumb_width, style.thumb_height),
            hit: style.hit,
            edge: style.thumb_edge,
            disabled_edge: style.disabled_edge,
            ring: style.ring,
            ring_width: style.ring_width,
        }
    }
}

/// The thumbs' faces: image tones, the same in every theme.
const FACES: [Color32; 3] = [Color32::BLACK, Color32::from_gray(128), Color32::WHITE];

/// Gamma's step, on its thumb and in its field.
const GAMMA_STEP: f64 = 0.01;

/// How far inside the black and white thumbs a gamma drag keeps its thumb, as a fraction of the
/// span between them: it never passes them, and the gamma stays finite.
const GAMMA_T: f64 = 1e-6;

const CLIP: &str = "craft_ui::levels_clip";
const STATE: &str = "craft_ui::levels_editor";
const EDIT: &str = "craft_ui::levels_editor::edit";

type Edits = parts::Edits<Levels>;

/// The input level the gamma thumb sits at: the level that maps to 50 % grey. `values` are the
/// input row's (black, gamma, white).
pub(crate) fn gamma_at(values: &[f64], gamma: f64) -> f64 {
    let (black, white) = (values[0], values[2]);
    black + (white - black) * 0.5f64.powf(gamma)
}

/// The gamma that puts its thumb at input level `x`.
pub(crate) fn gamma_from(values: &[f64], x: f64) -> f64 {
    let (black, white) = (values[0], values[2]);
    let t = ((x - black) / (white - black).max(f64::MIN_POSITIVE)).clamp(GAMMA_T, 1.0 - GAMMA_T);
    t.ln() / 0.5f64.ln()
}

/// Edits one channel's [`Levels`] through the edit lifecycle, as PhotoCraft's Levels:
/// `LevelsEditor::new("levels", "Levels", &levels, &numbers).show(ui)` returns an
/// [`EditResponse<Levels>`].
///
/// Top to bottom: the "Input Levels" caption; the channel's [`Histogram`] with the input row under
/// it (black, gamma and white triangles); the three input fields (black at the start, gamma
/// centred, white at the end); the "Output Levels" caption; a black-to-channel bar with the output
/// row under it (two triangles); the two output fields. The histogram and the bar span the thumbs'
/// travel, so a bin lines up with the thumb at its level. The editor keeps this geometry right to
/// left; the captions follow the reading order.
///
/// - The gamma thumb sits at `black + (white − black) · 0.5^gamma` and follows the black and white
///   thumbs; a gamma drag sets `ln t / ln 0.5` of where it is between them, in steps of 0.01 within
///   the [gamma range](LevelsEditor::gamma_range). The arrows move it the way they point (Right
///   lowers the gamma); screen-reader Increment raises the gamma.
/// - The input ends stay [`min_gap`](LevelsEditor::min_gap) steps apart; the output ends may
///   cross (output black above white inverts).
/// - Each thumb is a Tab stop and an AccessKit `Slider`; a press grabs a thumb where it is, a
///   press off the thumbs moves the nearest one there, and overlapping thumbs follow the drag's
///   direction. A double-click or Cmd/Ctrl+Backspace sets a thumb's default; Esc cancels a drag.
/// - Each field shows its thumb's value, the host's as it is when it lies outside the range, and
///   takes values within the thumb's bounds. A press on a row or on another field (a scrub) while
///   a field's typing run has the focus waits for the run's commit, and edits the levels it
///   leaves.
/// - While Alt is held during an input black or white drag, [`clipping`](LevelsEditor::clipping)
///   says which, for the host's clipping view.
///
/// Events: a row's press-drag-release is Begin, Updates and Commit; taps, keys, the wheel,
/// screen-reader requests and resets are Sets; a field's scrub or typing run is its own gesture.
/// The parts report as one edit ([`edit::parts`]): a part's event while another part's gesture is
/// under way waits for its ending.
///
/// AccessKit: a `Group` named by the label, valued by [`value`](LevelsEditor::value) (the
/// channel); the histogram's `Image`, five `Slider`s and five `SpinButton`s inside it.
#[must_use = "show the editor with `show`"]
pub struct LevelsEditor<'a> {
    id_salt: IdSalt,
    label: String,
    levels: &'a Levels,
    numbers: &'a LocaleNumbers,
    range: RangeInclusive<f64>,
    step: f64,
    decimals: usize,
    gamma_range: RangeInclusive<f64>,
    min_gap: f64,
    channel: ChannelColor,
    value: Option<String>,
    histogram: Option<&'a HistogramData>,
    stale: bool,
    captions: bool,
    width: Option<f32>,
    policy: EditPolicy,
    labels: LevelsLabels,
    enabled: bool,
}

impl<'a> LevelsEditor<'a> {
    /// An editor named `label` ("Levels") of the host's `levels`; `id_salt` keeps its gestures and
    /// focus, and `numbers` writes the field values.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, levels: &'a Levels, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            levels,
            numbers,
            range: 0.0..=255.0,
            step: 1.0,
            decimals: 0,
            gamma_range: 0.01..=9.99,
            min_gap: 2.0,
            channel: ChannelColor::Neutral,
            value: None,
            histogram: None,
            stale: false,
            captions: true,
            width: None,
            policy: EditPolicy::default(),
            labels: LevelsLabels::default(),
            enabled: true,
        }
    }

    /// The levels' range, ascending; `0.0..=255.0` by default.
    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        self.range = range;
        self
    }

    /// The step levels snap to and the fields step by; 1 by default.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Digits after the decimal mark in the level fields; 0 by default.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// The gamma's range; `0.01..=9.99` by default.
    pub fn gamma_range(mut self, range: RangeInclusive<f64>) -> Self {
        self.gamma_range = range;
        self
    }

    /// How many steps apart the input ends stay; 2 by default.
    pub fn min_gap(mut self, steps: f64) -> Self {
        self.min_gap = steps;
        self
    }

    /// The edited channel's colour: the light end of the output bar.
    pub fn channel(mut self, channel: ChannelColor) -> Self {
        self.channel = channel;
        self
    }

    /// What a screen reader reads after the label: the channel ("Red channel").
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// The channel's histogram; without it the plot is empty.
    pub fn histogram(mut self, data: &'a HistogramData) -> Self {
        self.histogram = Some(data);
        self
    }

    /// Marks the histogram as older than the image ([`Histogram::stale`]).
    pub fn stale(mut self, stale: bool) -> Self {
        self.stale = stale;
        self
    }

    /// Whether the "Input Levels" and "Output Levels" captions show; on by default.
    pub fn captions(mut self, captions: bool) -> Self {
        self.captions = captions;
        self
    }

    /// The editor's width; by default the available width within `levels.min_width ..=
    /// levels.max_width`.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// How the gestures report ([`EditPolicy`]).
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn labels(mut self, labels: LevelsLabels) -> Self {
        self.labels = labels;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The id of an editor shown in `ui` with `id_salt`, for [`clipping`](LevelsEditor::clipping).
    pub fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// Which input end editor `id` is dragging while Alt is held, as of its last pass: the host
    /// shows the image's clipping for it.
    pub fn clipping(ctx: &Context, id: Id) -> Option<LevelsClip> {
        let (clip, pass) = ctx.data(|d| d.get_temp::<(Option<LevelsClip>, u64)>(id.with(CLIP)))?;
        // An editor that isn't drawn any more (hidden during the drag) reports nothing.
        clip.filter(|_| pass + 1 >= ctx.cumulative_pass_nr())
    }

    pub fn style(tokens: &UiTokens) -> LevelsStyle {
        let l = &tokens.component.levels;
        LevelsStyle {
            min_width: l.min_width,
            max_width: l.max_width,
            row: l.row,
            thumb_width: l.thumb_width,
            thumb_height: l.thumb_height,
            hit: l.hit,
            bar: l.bar,
            thumb_edge: l.thumb_edge.to_egui(),
            disabled_edge: l.disabled_edge.to_egui(),
            ring: l.ring.to_egui(),
            ring_width: tokens.component.slider.ring_width,
            caption_role: TypeRole::Caption,
            field_gap: tokens.space.gap.inline,
        }
    }

    /// Shows the editor and returns this frame's edit. The response is the group's.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Levels> {
        let tokens = Tokens::of(ui);
        let style = LevelsEditor::style(&tokens);
        let id = ui.make_persistent_id(self.id_salt);
        let width = self.width.unwrap_or_else(|| ui.available_width().clamp(style.min_width, style.max_width));
        let enabled = self.enabled && ui.is_enabled();
        let mut builder = UiBuilder::new().scope_id(id).accessibility_role(Role::Group).accessibility_label(&self.label);
        if !enabled {
            builder = builder.disabled();
        }
        let group = ui
            .allocate_ui(vec2(width, ui.available_height()), |ui| {
                ui.set_max_width(width);
                ui.scope_builder(builder, |ui| Stack::new(Gap::Stack).show(ui, |ui| self.parts(ui, id, &tokens, &style, width, enabled)).inner)
            })
            .inner;
        let rect = group.response.rect;
        // A `Ui`'s node gets no bounds of its own from egui.
        let bounds = egui::accesskit::Rect { x0: rect.min.x.into(), y0: rect.min.y.into(), x1: rect.max.x.into(), y1: rect.max.y.into() };
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_bounds(bounds);
            if let Some(value) = &self.value {
                node.set_value(value.as_str());
            }
        });
        let (event, submitted, escaped) = group.inner;
        EditResponse { response: group.response, event, submitted, escaped }
    }

    /// The parts top to bottom. Returns the pass's event and whether a field was submitted or
    /// passed on an Esc press.
    fn parts(
        &self,
        ui: &mut Ui,
        id: Id,
        tokens: &UiTokens,
        style: &LevelsStyle,
        width: f32,
        enabled: bool,
    ) -> (Option<EditEvent<Levels>>, bool, bool) {
        let rows = ThumbRowStyle::from(style);
        let (start, end) = (self.range.start().min(*self.range.end()), self.range.start().max(*self.range.end()));
        let gap = self.min_gap * self.step;
        let mut state = ui.data(|d| d.get_temp::<parts::State<()>>(id.with(STATE))).unwrap_or_default();
        // Esc is the parts': each cancels its own gesture, which cancels this one.
        let outer = Gesture::load(ui, id.with(EDIT), self.levels, self.policy.cancellable(false)).enabled(enabled);
        // Every part draws from the composite's shown value; the parts report each update and the
        // composite's gesture applies the policy.
        let levels = *outer.shown();
        let inner = EditPolicy::default().cancellable(self.policy.cancellable).cost(self.policy.cost);
        let mut edits = Edits::new(id, &state);
        let stack_gap = ui.spacing().item_spacing.y;

        if self.captions {
            self.caption(ui, style, &self.labels.input);
        }
        // The histogram touches the input row, and the bar the output row.
        ui.spacing_mut().item_spacing.y = 0.0;
        let (plot, _) = ui.allocate_exact_size(vec2(width, Histogram::style(tokens).height), Sense::hover());
        let plot = plot.shrink2(vec2(rows.inset(), 0.0));
        let mut histogram = Histogram::new("histogram", self.labels.histogram.as_str()).height(plot.height()).stale(self.stale);
        if let Some(data) = self.histogram {
            histogram = histogram.data(data.clone());
        }
        histogram.show(&mut ui.new_child(UiBuilder::new().id_salt("histogram-plot").max_rect(plot)));
        ui.spacing_mut().item_spacing.y = stack_gap;

        let ordered = |below, above| Order::Between { below, above, gap };
        let thumbs = [
            Thumb::new(self.labels.in_black.as_str(), FACES[0]).order(ordered(None, Some(2))).default_value(start),
            Thumb::new(self.labels.gamma.as_str(), FACES[1])
                .follows(&self.gamma_range, GAMMA_STEP, Follow { at: gamma_at, value_at: gamma_from })
                .default_value(1.0)
                .unset_at(1.0)
                .falling(),
            Thumb::new(self.labels.in_white.as_str(), FACES[2]).order(ordered(Some(0), None)).default_value(end),
        ];
        let input = ThumbRow::new("input", levels.input(), &self.range, Some(self.step), thumbs)
            .width(width)
            .policy(inner)
            .enabled(enabled)
            .part(true)
            .show(ui, &rows);
        let alt = ui.input(|i| i.modifiers.alt);
        let clip = match input.dragged {
            Some(0) if alt => Some(LevelsClip::Shadows),
            Some(2) if alt => Some(LevelsClip::Highlights),
            _ => None,
        };
        let pass = ui.ctx().cumulative_pass_nr();
        ui.data_mut(|d| d.insert_temp(id.with(CLIP), (clip, pass)));

        let level = |field: NumberField<'a>| field.step(self.step).decimals(self.decimals..=self.decimals);
        let [black, gamma, white] = input.values;
        let [black_at, _, white_at] = input.shown;
        let full = self.field_width(ui, start..=end, self.decimals);
        let fields = FieldStrip::new(ui, width, style.field_gap);
        let w3 = fields.share(3);
        let in_fields = [
            (
                fields.at_start(full.min(w3)),
                "in-black",
                level(self.field(inner, "in-black", &self.labels.in_black, black, start..=(white_at - gap).clamp(start, end))),
            ),
            (
                fields.centred(self.field_width(ui, self.gamma_range.clone(), 2).min(w3)),
                "gamma",
                self.field(inner, "gamma", &self.labels.gamma, gamma, self.gamma_range.clone()).step(GAMMA_STEP).decimals(2..=2),
            ),
            (
                fields.at_end(full.min(w3)),
                "in-white",
                level(self.field(inner, "in-white", &self.labels.in_white, white, (black_at + gap).clamp(start, end)..=end)),
            ),
        ];
        let in_edits = in_fields.map(|(rect, salt, field)| show_field(ui, rect, salt, field.part(true)));

        if self.captions {
            self.caption(ui, style, &self.labels.output);
        }
        ui.spacing_mut().item_spacing.y = 0.0;
        let (bar, _) = ui.allocate_exact_size(vec2(width, style.bar), Sense::hover());
        ui.spacing_mut().item_spacing.y = stack_gap;
        // The bar spans the thumbs' travel and is painted first: a thumb's ring reaches over it.
        let bar = Rect::from_x_y_ranges((bar.left() + rows.inset())..=(bar.right() - rows.inset()), bar.y_range());
        if ui.is_rect_visible(bar) {
            ui.painter().add(gradient(bar, Rgba::BLACK.to_egui(), self.channel.bar(tokens), false));
        }
        let thumbs = [
            Thumb::new(self.labels.out_black.as_str(), FACES[0]).default_value(start),
            Thumb::new(self.labels.out_white.as_str(), FACES[2]).default_value(end),
        ];
        let output = ThumbRow::new("output", levels.output(), &self.range, Some(self.step), thumbs)
            .width(width)
            .policy(inner)
            .enabled(enabled)
            .part(true)
            .show(ui, &rows);
        let [out_black, out_white] = output.values;
        let fields = FieldStrip::new(ui, width, style.field_gap);
        let w2 = full.min(fields.share(2));
        let out_fields = [
            (fields.at_start(w2), "out-black", level(self.field(inner, "out-black", &self.labels.out_black, out_black, start..=end))),
            (fields.at_end(w2), "out-white", level(self.field(inner, "out-white", &self.labels.out_white, out_white, start..=end))),
        ];
        let out_edits = out_fields.map(|(rect, salt, field)| show_field(ui, rect, salt, field.part(true)));

        let [b, g, w] = in_edits;
        let [ob, ow] = out_edits;
        let submitted = [&b, &g, &w, &ob, &ow].iter().any(|e| e.submitted);
        let escaped = [&b, &g, &w, &ob, &ow].iter().any(|e| e.escaped);
        edits.push_from(&input.edit, &levels.input(), |v| levels.with_input(v));
        edits.typing(ui.ctx(), &b, |v| Levels { in_black: v, ..levels });
        edits.typing(ui.ctx(), &g, |v| Levels { gamma: v, ..levels });
        edits.typing(ui.ctx(), &w, |v| Levels { in_white: v, ..levels });
        edits.push_from(&output.edit, &levels.output(), |v| levels.with_output(v));
        edits.typing(ui.ctx(), &ob, |v| Levels { out_black: v, ..levels });
        edits.typing(ui.ctx(), &ow, |v| Levels { out_white: v, ..levels });
        let outer = edits.feed(outer, &mut state, edit::in_scope(ui));
        ui.data_mut(|d| d.insert_temp(id.with(STATE), state));
        (outer.finish(), submitted, escaped)
    }

    fn caption(&self, ui: &mut Ui, style: &LevelsStyle, text: &str) {
        ui.add(Text::new(text).role(style.caption_role).content(Content::Muted));
    }

    fn field(&self, policy: EditPolicy, salt: &'static str, label: &str, value: f64, range: RangeInclusive<f64>) -> NumberField<'a> {
        NumberField::new(salt, label, value, self.numbers).range(range).policy(policy)
    }

    /// The width that shows every value of `range` with `decimals` digits.
    fn field_width(&self, ui: &Ui, range: RangeInclusive<f64>, decimals: usize) -> f32 {
        NumberField::fit_width(ui, ControlSize::Md, self.numbers, range, decimals, &Unit::default())
    }
}

/// A strip of fields under a row, placed left to right in either direction (the fields line up
/// with the thumbs).
struct FieldStrip {
    rect: Rect,
    gap: f32,
}

impl FieldStrip {
    fn new(ui: &mut Ui, width: f32, gap: f32) -> Self {
        let height = NumberField::style(&Tokens::of(ui), ControlSize::Md).field.height;
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        Self { rect, gap }
    }

    /// The width each of `n` fields gets at most.
    fn share(&self, n: usize) -> f32 {
        ((self.rect.width() - self.gap * (n - 1) as f32) / n as f32).max(0.0)
    }

    fn at_start(&self, width: f32) -> Rect {
        Rect::from_min_size(self.rect.min, vec2(width, self.rect.height()))
    }

    fn centred(&self, width: f32) -> Rect {
        Rect::from_center_size(self.rect.center(), vec2(width, self.rect.height()))
    }

    fn at_end(&self, width: f32) -> Rect {
        Rect::from_min_size(pos2(self.rect.right() - width, self.rect.top()), vec2(width, self.rect.height()))
    }
}

fn show_field(ui: &mut Ui, rect: Rect, salt: &str, field: NumberField<'_>) -> EditResponse<f64> {
    let layout = Layout::left_to_right(Align::Center).with_main_justify(true);
    field.show(&mut ui.new_child(UiBuilder::new().id_salt(salt).max_rect(rect).layout(layout)))
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["LevelsEditor"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "composites/levels-editor/levels",
            level: Level::Composites,
            component: "LevelsEditor",
            name: "Levels",
            args: || {
                Args::new()
                    .with_choice("channel", &["RGB", "Red", "Green", "Blue"], 0)
                    .with_bool("histogram", true)
                    .with_bool("stale", false)
                    .with_choice("range", &["0 to 255", "0 to 1"], 0)
                    .with_bool("captions", true)
                    .with_bool("disabled", false)
                    .with_number("width", 300.0, 120.0, 400.0)
            },
            show: show_levels,
        },
        Story {
            id: "composites/levels-editor/event-log",
            level: Level::Composites,
            component: "LevelsEditor",
            name: "Event log",
            args: || Args::new().with_choice("update rate", edit::RATES, 0).with_bool("cancellable", true).with_bool("on release only", false),
            show: show_event_log,
        },
    ]
}

const STORY_CHANNELS: [(&str, ChannelColor); 4] =
    [("RGB", ChannelColor::Neutral), ("Red", ChannelColor::Red), ("Green", ChannelColor::Green), ("Blue", ChannelColor::Blue)];

/// A host's levels per channel and range, and the "before" of the gesture under way.
#[derive(Clone, Debug, Default)]
struct StoryLevels {
    range: String,
    channels: Vec<Levels>,
    before: Option<Levels>,
}

/// PhotoCraft's Levels on an RGB image, a [`Levels`] per channel; or EffectCraft's on 0 to 1.
fn show_levels(ui: &mut Ui, args: &mut Args) {
    let numbers = LocaleNumbers::from_tag("en");
    let channel = STORY_CHANNELS.iter().position(|(name, _)| *name == args.choice("channel")).unwrap_or(0);
    let (name, color) = STORY_CHANNELS[channel];
    let unit = args.choice("range") == "0 to 1";
    let range = if unit { 0.0..=1.0 } else { 0.0..=255.0 };
    let id = ui.scope_id().with("levels-editor-levels");
    let mut host = ui.data(|d| d.get_temp::<StoryLevels>(id)).unwrap_or_default();
    if host.range != args.choice("range") || host.channels.len() != STORY_CHANNELS.len() {
        host = StoryLevels {
            range: args.choice("range").to_owned(),
            channels: vec![Levels::identity(range.clone()); STORY_CHANNELS.len()],
            before: None,
        };
    }
    let bins = story_bins([0.0, 0.0, 12.0, -12.0][channel]);
    let data = HistogramData::tone(&bins, color);
    let levels = host.channels[channel];
    let mut editor = LevelsEditor::new("levels", "Levels", &levels, &numbers)
        .channel(color)
        .value(format!("{name} channel"))
        .stale(args.bool("stale"))
        .captions(args.bool("captions"))
        .enabled(!args.bool("disabled"))
        .width(args.number("width") as f32)
        .range(range);
    if unit {
        editor = editor.step(0.001).decimals(3).gamma_range(0.1..=10.0);
    }
    if args.bool("histogram") {
        editor = editor.histogram(&data);
    }
    let edit = editor.show(ui);
    let StoryLevels { channels, before, .. } = &mut host;
    match edit.event {
        Some(EditEvent::Begin { .. }) => *before = Some(channels[channel]),
        Some(EditEvent::Cancel { .. }) => {
            if let Some(b) = before.take() {
                channels[channel] = b;
            }
        }
        Some(event) => {
            if let Some(value) = event.value() {
                channels[channel] = *value;
            }
        }
        None => {}
    }
    let l = host.channels[channel];
    let text = format!("Input {:.3} / {:.2} / {:.3}, output {:.3} / {:.3}", l.in_black, l.gamma, l.in_white, l.out_black, l.out_white);
    ui.add(Text::new(text).content(Content::Muted));
    ui.data_mut(|d| d.insert_temp(id, host));
}

/// Events kept in the event log.
const LOG_LEN: usize = 20;

/// The event log's document: one channel's levels and an undo history of whole values.
#[derive(Clone, Debug)]
struct LogDoc {
    levels: Levels,
    undo: Vec<Levels>,
}

impl UndoMarks for LogDoc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(&levels) = self.undo.get(mark) {
            self.levels = levels;
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

#[derive(Clone, Debug)]
struct LogState {
    doc: LogDoc,
    host: GestureHost<LogDoc>,
    /// Newest first.
    log: VecDeque<String>,
}

impl Default for LogState {
    fn default() -> Self {
        Self { doc: LogDoc { levels: Levels::identity(0.0..=255.0), undo: Vec::new() }, host: GestureHost::new(), log: VecDeque::new() }
    }
}

impl LogState {
    fn log(&mut self, line: String) {
        self.log.push_front(line);
        self.log.truncate(LOG_LEN);
    }
}

/// A levels editor under a [`GestureHost`], its events newest first and the undo steps counted.
/// Cmd/Ctrl+Z undoes a step, or cancels the gesture under way.
fn show_event_log(ui: &mut Ui, args: &mut Args) {
    let policy = edit::policy(args);
    let numbers = LocaleNumbers::from_tag("en");
    let state_id = ui.scope_id().with("levels-editor-event-log");
    let mut state = ui.data(|d| d.get_temp::<LogState>(state_id)).unwrap_or_default();
    if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
        match state.host.active() {
            Some(gesture) => {
                state.host.cancel(&mut state.doc, gesture);
                edit::cancel_all(ui.ctx(), CancelReason::UndoDuringGesture);
                state.log(format!("undo: cancel_all({:?})", CancelReason::UndoDuringGesture));
            }
            None => {
                if let Some(levels) = state.doc.undo.pop() {
                    state.doc.levels = levels;
                    state.log("undo: one step".to_owned());
                }
            }
        }
    }
    Stack::new(Gap::Stack).show(ui, |ui| {
        let levels = state.doc.levels;
        let edit = LevelsEditor::new("levels-log", "Levels", &levels, &numbers).policy(policy).width(300.0).show(ui);
        if let Some(event) = edit.event {
            state.log(format!("levels: {}", edit::describe(&event)));
            state.host.handle(ui.ctx(), &mut state.doc, event, |doc, value| {
                doc.undo.push(doc.levels);
                doc.levels = value;
            });
        }
        if let Some(event) = state.host.end_frame(ui.ctx(), &mut state.doc) {
            let line = format!("host: {}", edit::describe(&event));
            state.log(line);
        }
        ui.add(Text::new(format!("{} undo steps", state.doc.undo.len())).content(Content::Muted));
        Surface::new(SurfaceKind::Sunken).show(ui, |ui| {
            ui.set_min_width(LOG_WIDTH);
            if state.log.is_empty() {
                ui.add(Text::new("No events yet").content(Content::Faint));
            }
            for line in &state.log {
                ui.add(Text::new(line.as_str()).role(TypeRole::Mono));
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, state));
}

const LOG_WIDTH: f32 = 420.0;
