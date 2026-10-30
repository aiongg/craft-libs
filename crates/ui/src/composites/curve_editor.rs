//! [`CurveEditor`]: a tone curve's graph with its points, the Input/Output row of the selected
//! point and an optional hint line.

use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{
    AsIdSalt, Color32, Context, CornerRadius, Id, IdSalt, Key, Mesh, Modifiers, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui,
    UiBuilder, pos2, vec2,
};

use craft_theme::{Rgba, TypeRole};

use super::Histogram;
use super::NoArgs;
use super::curve::{Curve, CurveOp, PointKey, Rules, follow_corners};
use super::histogram::{ChannelColor, HistogramData, HistogramLook};
use super::{PropertyLabel, PropertyRow};
use crate::behavior::{ClaimedKey, KeyClaim};
use crate::controls::number_field::{self, FieldRule};
use crate::controls::{Checkbox, NumberField};
use crate::edit::ops::{self, ScalarOp};
use crate::edit::order::{self, Turn};
use crate::edit::parts::{self, Pushed};
use crate::edit::{self, EditPolicy, EditResponse, Gesture};
use crate::localized::{LocaleNumbers, Unit};
use crate::primitives::{Content, FocusRing, Gap, Handle, HandleShape, HandleStyle, Stack, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// One channel of a curve dialog: its name ("Red"; the host's text) and colour.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveChannel {
    pub label: String,
    pub color: ChannelColor,
}

impl CurveChannel {
    pub fn new(label: impl Into<String>, color: ChannelColor) -> Self {
        Self { label: label.into(), color }
    }
}

/// The editor's words; translate them in the app. Templates take `{i}`, `{n}`, `{x}`, `{y}` and
/// `{channel}`.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveLabels {
    /// The selected point's input field: "Input".
    pub input: String,
    /// The selected point's output field: "Output".
    pub output: String,
    /// A point's accessible name: "Point {i} of {n}".
    pub point: String,
    /// A point's accessible value: "Input {x}, Output {y}".
    pub point_value: String,
    /// The selected point's checkbox that makes it a corner point ([`CurveEditor::corners`]): "Corner".
    pub corner: String,
    /// A corner point's accessible value: "Input {x}, Output {y}, corner".
    pub corner_value: String,
    /// The graph's accessible value: "{channel}, {n} points".
    pub graph_value: String,
    /// The graph's accessible value without channels: "{n} points".
    pub graph_points: String,
    /// The accessible value of a drawn curve: "{channel}, drawn".
    pub graph_drawn: String,
    /// The accessible value of a drawn curve without channels: "Drawn".
    pub drawn: String,
    /// The unit of a value in percent ([`CurveEditor::percent`]): "{n}%".
    pub percent: String,
}

impl Default for CurveLabels {
    fn default() -> Self {
        Self {
            input: "Input".into(),
            output: "Output".into(),
            point: "Point {i} of {n}".into(),
            point_value: "Input {x}, Output {y}".into(),
            corner: "Corner".into(),
            corner_value: "Input {x}, Output {y}, corner".into(),
            graph_value: "{channel}, {n} points".into(),
            graph_points: "{n} points".into(),
            graph_drawn: "{channel}, drawn".into(),
            drawn: "Drawn".into(),
            percent: "{n}%".into(),
        }
    }
}

/// `template` with each `{name}` replaced by its value.
pub(crate) fn fill(template: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(template.to_owned(), |text, (name, value)| text.replace(&format!("{{{name}}}"), value))
}

/// A curve editor's look, from [`CurveEditor::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CurveStyle {
    /// Least and greatest side of the graph, gradient bars included: `curve.min_side`,
    /// `curve.max_side`.
    pub min_side: f32,
    pub max_side: f32,
    /// Inset of the value area in the plot: `curve.pad`.
    pub pad: f32,
    /// Thickness of the gradient bars and their gap to the plot: `curve.bar`, `curve.bar_gap`.
    pub bar: f32,
    pub bar_gap: f32,
    /// Radius around a point that takes a press: `curve.hit`.
    pub hit: f32,
    /// How far outside the plot a dragged interior point is removed: `curve.drag_off`.
    pub drag_off: f32,
    /// The plot: `curve.fill`, a one-point `curve.border` outside it.
    pub fill: Color32,
    pub border: Stroke,
    /// The quarter grid: one point of `curve.grid`.
    pub grid: Stroke,
    /// The diagonal: one point of `curve.baseline`.
    pub baseline: Stroke,
    /// Width of the edited curve: `curve.width`.
    pub width: f32,
    /// Width and opacity of the other channels' curves: `curve.other_width`, `curve.other_alpha`.
    pub other_width: f32,
    pub other_alpha: f32,
    /// An unselected point: `handle.size` square of `curve.point_fill` with a `handle.stroke`
    /// outline of `curve.point_stroke` inside it.
    pub point: HandleStyle,
    /// The selected point: `curve.point_stroke` with a `handle.stroke` outline of
    /// `curve.point_halo` outside it.
    pub selected: HandleStyle,
    /// The hint line: `caption` in `text.faint`.
    pub hint_role: TypeRole,
    pub hint: Color32,
}

/// What the editor keeps between frames, per editor.
#[derive(Clone, Debug, Default)]
struct State {
    /// The channel the selection belongs to; `None` after the host selected a point.
    channel: Option<usize>,
    selected: Option<usize>,
    drag: Option<Drag>,
    /// A press that waits for its turn (`edit::order`), and the last pass that held it.
    held: Option<(u64, Press)>,
    /// The last level a pencil stroke under way drew to.
    stroke: Option<[f32; 2]>,
    /// A pencil press that waits for its turn (`edit::order`), and the last pass that held it.
    stroke_held: Option<(u64, StrokePress)>,
    /// Where the previous pencil stroke ended: Shift+press continues from it.
    end: Option<StrokeEnd>,
    /// The graph's and the fields' edits as the editor's one gesture.
    parts: parts::State<EditOp>,
}

/// An edit of the graph or of a field, as it waits for its turn (`edit::parts`).
#[derive(Clone, Debug)]
enum EditOp {
    Graph(CurveOp),
    /// A field's operation on the point it names, in the field's units.
    Field(Axis, PointKey, ScalarOp),
}

/// A field's axis: the point's input or output.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Axis {
    Input,
    Output,
}

impl Axis {
    fn index(self) -> usize {
        match self {
            Axis::Input => 0,
            Axis::Output => 1,
        }
    }
}

/// A press on the graph: where, which button, and whether Cmd/Ctrl was held.
#[derive(Clone, Copy, Debug)]
struct Press {
    pos: Pos2,
    secondary: bool,
    command: bool,
}

/// A pencil press: where, and whether Shift was held.
#[derive(Clone, Copy, Debug)]
struct StrokePress {
    pos: Pos2,
    shift: bool,
}

/// The end of the previous pencil stroke and the curve it left; the end goes once the host's
/// curve is another (an undo, a reset, Smooth).
#[derive(Clone, Debug)]
struct StrokeEnd {
    at: [f32; 2],
    curve: Curve,
}

/// The point a press captured.
#[derive(Clone, Copy, Debug)]
struct Drag {
    /// The point dragged; `None` while it is off the graph.
    index: Option<usize>,
    /// From the pointer to the point, in levels, so a press doesn't make the point jump.
    offset: [f32; 2],
    /// The selection before the press, restored when the gesture is cancelled.
    before: Option<usize>,
    /// Whether the point is a corner point, kept while it is off the graph so it comes back as one.
    corner: bool,
}

const STATE: &str = "craft_ui::curve_editor";
/// The editor's gesture, made of the graph's and the fields' (`edit::parts`).
const EDIT: &str = "craft_ui::curve_editor::edit";

/// The keys a focused graph keeps from the app's shortcuts. Only `+` is matched logically: many
/// keyboards type it with Shift.
const CLAIMED: [ClaimedKey; 7] = [
    ClaimedKey::exact(Modifiers::NONE, Key::Delete),
    ClaimedKey::exact(Modifiers::NONE, Key::Backspace),
    ClaimedKey::logical(Modifiers::NONE, Key::Plus),
    ClaimedKey::exact(Modifiers::NONE, Key::Equals),
    ClaimedKey::exact(Modifiers::NONE, Key::Minus),
    ClaimedKey::exact(Modifiers::NONE, Key::Insert),
    ClaimedKey::exact(Modifiers::COMMAND, Key::Enter),
];

/// The keys a focused graph keeps from focus navigation: the arrows nudge its point.
const KEY_FILTER: egui::EventFilter = egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false };

/// A host's interpolation: the output level of every input level `0..=levels` through the points.
pub type Sampler = dyn Fn(&[[f32; 2]]) -> Vec<f32>;

/// The curve through points and corner flags, as [`CurveEditor`]'s samples give it.
type CornerSamples<'a> = dyn Fn(&[[f32; 2]], &[bool]) -> Vec<f32> + 'a;

/// Edits one tone curve through the edit lifecycle: `CurveEditor::new("curves", "Curve", &curve,
/// &numbers).show(ui)` returns an [`EditResponse<Curve>`].
///
/// The graph is square: the plot (`curve.fill`, `curve.border`), a quarter grid, the diagonal of
/// the unchanged curve, the curve in the active channel's colour, its points, and gradient bars
/// under it and at its left. Under the graph the Input/Output row edits the selected point, and a
/// hint line can follow. The graph keeps its geometry right to left
/// ([`Unmirrored::Graph`](crate::direction::Unmirrored::Graph)); the row follows the reading
/// order.
///
/// Points follow PhotoCraft's rules: whole levels `0..=levels`, at most
/// [`max_points`](CurveEditor::max_points), a new point within two levels of another's input is
/// that point, the end points stay (they move in both axes), and a point stays strictly between
/// its neighbours' inputs.
///
/// Pointer: a press on a point selects and drags it; a press on the empty graph adds a point there
/// and drags it, as one gesture; a press within two levels of a point's input selects that point
/// and takes it to the pointer, as one gesture (PhotoCraft). Dragged more than `curve.drag_off`
/// off the plot an interior point is removed, and comes back on re-entry within the drag. A
/// right-click or a Cmd/Ctrl press on a point deletes it. A press gives the graph keyboard focus;
/// while a typing run has it (a field's), the press waits for the run's commit to go out and then
/// edits the curve the run leaves (`edit::order`).
///
/// Keys while the graph has focus: `+` (or `=`) and `-` select the next and previous point (the
/// first and last with nothing selected; they stop at the ends); arrows move the selected point one
/// level, ten with Shift; Delete and Backspace remove it; Insert, and Cmd/Ctrl+Enter on every
/// platform, add a point on the curve halfway between the selected point and the next (across the
/// widest gap with nothing selected) and select it; Esc cancels a drag and is otherwise left to
/// the host. Every key but `+` needs exactly its modifiers. The graph claims Delete, Backspace,
/// `+`, `=`, `-`, Insert and Cmd/Ctrl+Enter from the app's shortcuts ([`KeyClaim`]).
///
/// Corners ([`corners`](CurveEditor::corners), the contour editor): a Corner checkbox under the
/// fields makes the selected point a corner point (Photoshop's and PhotoCraft's Contour Editor). The curve
/// bends at a corner point: each run of points from one corner to the next (both included) is
/// interpolated by the sampler on its own, as PhotoCraft's `contour_curve_lut` does. A new point is smooth, and a point keeps its flag when it moves; the flags
/// are the [`Curve::Points`] value's. With corners on, smooth points are round and corner points
/// square; without, every point is square.
///
/// Events: a press-drag-release is Begin, Updates and Commit (unchanged for a press without
/// movement); deletes, nudges and keyboard additions are Sets; the fields report their own
/// gestures as curves. The graph and the fields make one edit of the curve, one gesture at a time
/// (`edit::parts`): a screen reader's step on a field during a graph drag waits for the release
/// and goes out as its own Set. A waiting edit applies on its turn to the point it was made on,
/// found by its input or else as the one point between its neighbours' inputs, and is dropped
/// when that point is gone; an Insert selects its point then, and a Delete clears the selection.
/// The [`policy`](CurveEditor::policy) applies to the editor's events; the graph and the fields
/// show every change as it happens. The selection is view state: the host reads and sets it with
/// [`selected`](CurveEditor::selected) and [`select`](CurveEditor::select), and it clears when the
/// [`channel`](CurveEditor::channel) changes.
///
/// AccessKit: the graph is a focusable `Group` named by the label, valued "{channel}, {n}
/// points", whose active descendant is the selected point; each point is a `GraphicsObject` child
/// named "Point {i} of {n}" and valued "Input {x}, Output {y}" ("Input {x}, Output {y}, corner" for
/// a corner point), selected when it is.
#[must_use = "show the editor with `show`"]
pub struct CurveEditor<'a> {
    id_salt: IdSalt,
    label: String,
    curve: &'a Curve,
    numbers: &'a LocaleNumbers,
    channels: &'a [CurveChannel],
    channel: usize,
    levels: f32,
    max_points: usize,
    sampler: Option<&'a Sampler>,
    side: Option<f32>,
    fields: bool,
    percent: bool,
    hint: Option<String>,
    policy: EditPolicy,
    labels: CurveLabels,
    enabled: bool,
    pencil: bool,
    corners: bool,
    histogram: Option<&'a HistogramData>,
    overlays: &'a [Curve],
}

impl<'a> CurveEditor<'a> {
    /// An editor named `label` ("Curve") of the host's `curve`; `id_salt` keeps its selection and
    /// gesture, and `numbers` writes the field values and the accessible values.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, curve: &'a Curve, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            curve,
            numbers,
            channels: &[],
            channel: 0,
            levels: 255.0,
            max_points: 16,
            sampler: None,
            side: None,
            fields: true,
            percent: false,
            hint: None,
            policy: EditPolicy::default(),
            labels: CurveLabels::default(),
            enabled: true,
            pencil: false,
            corners: false,
            histogram: None,
            overlays: &[],
        }
    }

    /// The dialog's channels; the curve is the [`channel`](CurveEditor::channel)'s. Without
    /// channels the curve is neutral and unnamed.
    pub fn channels(mut self, channels: &'a [CurveChannel]) -> Self {
        self.channels = channels;
        self
    }

    /// The active channel, an index into [`channels`](CurveEditor::channels): the host's, since
    /// a click on the image adds a point to it.
    pub fn channel(mut self, index: usize) -> Self {
        self.channel = index;
        self
    }

    /// The top level of input and output; 255 by default.
    pub fn levels(mut self, levels: u32) -> Self {
        self.levels = levels.max(1) as f32;
        self
    }

    /// The most points the curve holds; 16 by default (Photoshop's).
    pub fn max_points(mut self, max: usize) -> Self {
        self.max_points = max.max(2);
        self
    }

    /// The host's interpolation: the output level of every input level `0..=levels` through the
    /// points. A monotone cubic ([`Curve::monotone`]) by default.
    pub fn sampler(mut self, sampler: &'a Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    /// The graph's side, bars included, in place of the available width clamped to
    /// `curve.min_side..=curve.max_side`.
    pub fn side(mut self, side: f32) -> Self {
        self.side = Some(side);
        self
    }

    /// Whether the Input/Output row shows; on by default.
    pub fn fields(mut self, fields: bool) -> Self {
        self.fields = fields;
        self
    }

    /// The fields and accessible values in percent of the levels (the contour editor).
    pub fn percent(mut self) -> Self {
        self.percent = true;
        self
    }

    /// A line under the editor; the host writes it, since it names shortcut keys.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// How the editor reports gestures; [`EditPolicy::default`] by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The editor's words in the interface language.
    pub fn labels(mut self, labels: CurveLabels) -> Self {
        self.labels = labels;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The host's pencil mode: a press-drag-release draws a [`Curve::Map`] with straight segments
    /// between pointer samples, and Shift+press continues in a straight line from the previous
    /// stroke's end. The points are hidden and no key but Esc (which cancels a stroke) does
    /// anything. A [`Curve::Points`] value is drawn from its samples.
    pub fn pencil(mut self, pencil: bool) -> Self {
        self.pencil = pencil;
        self
    }

    /// Points can be corner points (the contour editor): the curve bends at a corner, each run of
    /// points from one corner to the next being interpolated on its own (PhotoCraft's
    /// `contour_curve_lut`). The "Corner" checkbox of the fields row toggles the selected point;
    /// with [`fields(false)`](Self::fields) there is no checkbox and so no way to toggle a
    /// corner (PhotoCraft has no other path either): the host sets the flags in the
    /// [`Curve::Points`] value.
    pub fn corners(mut self, on: bool) -> Self {
        self.corners = on;
        self
    }

    /// The active channel's histogram, drawn behind the curve in its backdrop look across the
    /// value area.
    pub fn histogram(mut self, data: &'a HistogramData) -> Self {
        self.histogram = Some(data);
        self
    }

    /// The curve of every channel, in the order of [`channels`](CurveEditor::channels): the other
    /// channels' curves draw behind the active one in their channel colours. Identity curves are
    /// skipped, and the host passes them only for the composite channel, as Photoshop shows them.
    pub fn overlays(mut self, curves: &'a [Curve]) -> Self {
        self.overlays = curves;
        self
    }

    /// The graph's widget id for `id_salt` in `ui`, as [`selected`](CurveEditor::selected) and
    /// [`select`](CurveEditor::select) take it; also the response's id.
    pub fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// The selected point of editor `id`.
    pub fn selected(ctx: &Context, id: Id) -> Option<usize> {
        ctx.data(|d| d.get_temp::<State>(id.with(STATE))).and_then(|s| s.selected)
    }

    /// Selects point `index` of editor `id` (the host's click on the image), in the channel the
    /// editor shows next.
    pub fn select(ctx: &Context, id: Id, index: Option<usize>) {
        ctx.data_mut(|d| {
            let state = d.get_temp_mut_or_default::<State>(id.with(STATE));
            state.channel = None;
            state.selected = index;
        });
    }

    pub fn style(tokens: &UiTokens) -> CurveStyle {
        let c = &tokens.component.curve;
        let h = &tokens.component.handle;
        let line = |color: Rgba| Stroke::new(craft_theme::Stroke::W1.width(1.0), color.to_egui());
        let mut point = Handle::style(tokens, HandleShape::Square);
        point.fill = c.point_fill.to_egui();
        point.stroke = Stroke::new(h.stroke, c.point_stroke.to_egui());
        point.halo = Stroke::NONE;
        let mut selected = point;
        selected.fill = c.point_stroke.to_egui();
        selected.stroke = Stroke::NONE;
        selected.halo = Stroke::new(h.stroke, c.point_halo.to_egui());
        CurveStyle {
            min_side: c.min_side,
            max_side: c.max_side,
            pad: c.pad,
            bar: c.bar,
            bar_gap: c.bar_gap,
            hit: c.hit,
            drag_off: c.drag_off,
            fill: c.fill.to_egui(),
            border: line(c.border),
            grid: line(c.grid),
            baseline: line(c.baseline),
            width: c.width,
            other_width: c.other_width,
            other_alpha: c.other_alpha,
            point,
            selected,
            hint_role: TypeRole::Caption,
            hint: tokens.color.text.faint.to_egui(),
        }
    }

    /// Shows the editor and returns this frame's edit event. The response is the graph's.
    pub fn show(self, ui: &mut Ui) -> EditResponse<Curve> {
        let tokens = Tokens::of(ui);
        let style = CurveEditor::style(&tokens);
        let id = ui.make_persistent_id(self.id_salt);
        let mut state = ui.data(|d| d.get_temp::<State>(id.with(STATE))).unwrap_or_default();
        // Esc is the parts': each cancels its own gesture, which cancels this one.
        let outer = Gesture::load(ui, id.with(EDIT), self.curve, self.policy.cancellable(false)).enabled(self.enabled && ui.is_enabled());
        let policy = EditPolicy::default().cancellable(self.policy.cancellable).cost(self.policy.cost);
        let mut edits = parts::Edits::<Curve, EditOp>::new(id, &state.parts, &outer, edit::in_scope(ui));
        edits.replay(ui.ctx(), |op, now| self.apply(op, now, &mut state.selected));
        // The parts keep the selection of their own events as they make them.
        let apply = |op: &EditOp, now: &Curve| self.apply(op, now, &mut None);
        let graph = Stack::new(Gap::Stack)
            .show(ui, |ui| {
                let curve = edits.value().clone();
                let (graph, graph_ops, before_ops) = self.graph(ui, &style, id, &curve, policy, &mut state);
                // A graph edit that waits for another part's gesture isn't in the editor's value,
                // and the selection stays where it was before its first operation until its turn:
                // a selection key pressed ahead of it in the frame keeps its effect.
                if edits.push(&graph, graph_ops.into_iter().map(EditOp::Graph), apply, |curve, _| curve) == Pushed::Waits
                    && let Some(selected) = before_ops
                {
                    state.selected = selected;
                }
                if self.fields && !self.pencil && edits.value().points().is_some() {
                    self.fields_row(ui, id, state.selected, policy, &mut edits, apply);
                }
                if let Some(hint) = &self.hint {
                    ui.add(Text::new(hint.as_str()).role(style.hint_role).content(Content::Faint));
                }
                graph
            })
            .inner;
        let outer = edits.feed(outer, &mut state.parts);
        ui.data_mut(|d| d.insert_temp(id.with(STATE), state));
        EditResponse::new(graph.response, outer.finish())
    }

    /// The graph, a part of the editor's edit drawn from `curve` with `policy`: its interaction,
    /// look and AccessKit nodes. Returns its edit, the operations its one-shot event made and the
    /// selection before the first of them (`None` when it made none).
    fn graph(
        &self,
        ui: &mut Ui,
        style: &CurveStyle,
        id: Id,
        curve: &Curve,
        policy: EditPolicy,
        state: &mut State,
    ) -> (EditResponse<Curve>, Vec<CurveOp>, Option<Option<usize>>) {
        let side = self.side.unwrap_or_else(|| ui.available_width().clamp(style.min_side, style.max_side));
        let (outer, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
        let edge = style.bar + style.bar_gap;
        let plot = Rect::from_min_size(outer.min + vec2(edge, 0.0), vec2(side - edge, side - edge).max(vec2(1.0, 1.0)))
            .round_to_pixels(ui.pixels_per_point());
        let map = Map { area: plot.shrink(style.pad), levels: self.levels };

        let mut builder = UiBuilder::new()
            .scope_id(id)
            .max_rect(plot)
            .sense(Sense::click_and_drag())
            .accessibility_role(Role::Group)
            .accessibility_label(&self.label);
        if !self.enabled {
            builder = builder.disabled();
        }
        let mut graph_ui = ui.new_child(builder);
        graph_ui.expand_to_include_rect(plot);
        let response = graph_ui.response();
        let enabled = response.enabled();

        let filter = if self.pencil { egui::EventFilter::default() } else { KEY_FILTER };
        let mut gesture = Gesture::load(ui, id, curve, policy).event_filter(filter).enabled(enabled);
        if state.channel.is_some_and(|c| c != self.channel) {
            state.selected = None;
            state.drag = None;
            state.stroke = None;
            state.end = None;
        }
        state.channel = Some(self.channel);
        if !self.pencil {
            state.end = None;
        }
        if !gesture.is_active() {
            // Cancelled (Esc, an undo, an external change): the selection goes back too.
            if let Some(drag) = state.drag.take() {
                state.selected = drag.before;
            }
            state.stroke = None;
        }

        let rules = Rules { levels: self.levels, max_points: self.max_points };
        let mut points = gesture.shown().points().map(<[_]>::to_vec).filter(|_| !self.pencil);
        let mut corner_flags = gesture.shown().corner_flags();
        let mut ops = Vec::new();
        let mut before_ops = None;
        if self.pencil {
            state.selected = None;
            state.drag = None;
            state.held = None;
            if !enabled {
                state.stroke_held = None;
            }
            if !gesture.is_active() && state.end.as_ref().is_some_and(|end| end.curve != *curve) {
                state.end = None;
            }
            if enabled {
                let outputs = Curve::drawn(gesture.shown(), |points, corners| self.samples(points, corners), self.levels);
                StrokeEdit { gesture: &mut gesture, state, outputs, map }.pointer(ui, &response);
            }
        } else if let Some(points) = points.as_mut() {
            state.selected = state.selected.filter(|&i| i < points.len());
            state.stroke_held = None;
            if enabled {
                let samples = |points: &[[f32; 2]], corners: &[bool]| self.samples(points, corners);
                let mut edit = PointEdit {
                    gesture: &mut gesture,
                    points,
                    corner_flags: &mut corner_flags,
                    state,
                    rules,
                    map,
                    style,
                    samples: &samples,
                    ops: Vec::new(),
                    before_ops: None,
                };
                edit.pointer(ui, &response);
                if response.has_focus() {
                    KeyClaim::claim(ui.ctx(), id, &CLAIMED);
                    edit.keys(ui);
                }
                ops = edit.ops;
                before_ops = edit.before_ops;
            }
        } else {
            state.selected = None;
            state.drag = None;
            state.held = None;
            state.stroke_held = None;
        }

        let shown = gesture.shown().clone();
        let event = gesture.finish();
        let selected = state.selected;

        let tokens = Tokens::of(ui);
        let channel = self.channels.get(self.channel);
        let color = channel.map_or(ChannelColor::Neutral, |c| c.color);
        if ui.is_rect_visible(outer) {
            self.paint_backdrop(&graph_ui, style, plot, map, color, &tokens);
            self.paint_curve(&graph_ui, &shown, map, color.curve(&tokens), style.width);
        }
        let point_ids = match shown.points() {
            Some(points) if !self.pencil => self.points(&mut graph_ui, style, map, points, &shown.corner_flags(), selected),
            _ => Vec::new(),
        };
        drop(graph_ui);
        FocusRing::paint_radius(ui, plot, CornerRadius::ZERO, &response);

        let n = shown.points().map_or(0, <[_]>::len);
        let drawn = self.pencil || shown.points().is_none();
        let value = match channel {
            Some(c) if drawn => fill(&self.labels.graph_drawn, &[("channel", &c.label)]),
            None if drawn => self.labels.drawn.clone(),
            Some(c) => fill(&self.labels.graph_value, &[("channel", &c.label), ("n", &self.numbers.format(n as f64, 0..=0))]),
            None => fill(&self.labels.graph_points, &[("n", &self.numbers.format(n as f64, 0..=0))]),
        };
        let active = selected.and_then(|i| point_ids.get(i).copied());
        // A `Ui`'s node gets no bounds of its own from egui.
        let bounds = egui::accesskit::Rect { x0: plot.min.x.into(), y0: plot.min.y.into(), x1: plot.max.x.into(), y1: plot.max.y.into() };
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_bounds(bounds);
            node.set_value(value);
            match active {
                Some(point) => node.set_active_descendant(point.accesskit_id()),
                None => node.clear_active_descendant(),
            }
        });
        (EditResponse::new(response, event), ops, before_ops)
    }

    /// The output of every input level through `points`: the host's sampler or a monotone cubic.
    fn samples(&self, points: &[[f32; 2]], corners: &[bool]) -> Vec<f32> {
        Curve::through(points, corners, |points| match self.sampler {
            Some(sampler) => sampler(points),
            None => Curve::monotone(points, self.levels),
        })
    }

    /// The indices of the channels whose curves draw behind the active one: every channel with a
    /// curve in the overlays but the active one, unless it is the identity.
    pub(super) fn other_channels(&self) -> Vec<usize> {
        (0..self.overlays.len().min(self.channels.len())).filter(|&i| i != self.channel && !self.overlays[i].is_identity(self.levels)).collect()
    }

    /// The plot, the histogram, the other channels' curves, the grid, baseline, border and
    /// gradient bars.
    fn paint_backdrop(&self, ui: &Ui, style: &CurveStyle, plot: Rect, map: Map, color: ChannelColor, tokens: &UiTokens) {
        let painter = ui.painter().with_clip_rect(ui.clip_rect());
        painter.rect_filled(plot, 0.0, style.fill);
        let area = map.area;
        if let Some(data) = self.histogram {
            Histogram::paint(&painter, area, data, HistogramLook::Backdrop, tokens);
        }
        for i in self.other_channels() {
            let color = self.channels[i].color.curve(tokens).gamma_multiply(style.other_alpha);
            self.paint_curve(ui, &self.overlays[i], map, color, style.other_width);
        }
        for i in 1..4 {
            let f = i as f32 / 4.0;
            let x = area.left() + f * area.width();
            let y = area.top() + f * area.height();
            painter.line_segment([pos2(x, area.top()), pos2(x, area.bottom())], style.grid);
            painter.line_segment([pos2(area.left(), y), pos2(area.right(), y)], style.grid);
        }
        painter.line_segment([area.left_bottom(), area.right_top()], style.baseline);
        painter.rect_stroke(plot, 0.0, style.border, StrokeKind::Outside);
        let (dark, light) = (Rgba::BLACK.to_egui(), color.bar(tokens));
        let under =
            Rect::from_min_max(pos2(area.left(), plot.bottom() + style.bar_gap), pos2(area.right(), plot.bottom() + style.bar_gap + style.bar));
        let left = Rect::from_min_max(pos2(plot.left() - style.bar_gap - style.bar, area.top()), pos2(plot.left() - style.bar_gap, area.bottom()));
        ui.painter().add(gradient(under, dark, light, false));
        ui.painter().add(gradient(left, dark, light, true));
    }

    /// The curve through its samples, or a drawn curve's straight segments.
    fn paint_curve(&self, ui: &Ui, curve: &Curve, map: Map, color: Color32, width: f32) {
        let outputs = match curve {
            Curve::Points { points, corners } => self.samples(points, corners),
            Curve::Map(outputs) => outputs.clone(),
        };
        let last = outputs.len().saturating_sub(1).max(1) as f32;
        let line: Vec<Pos2> = outputs.iter().enumerate().map(|(k, &o)| map.screen([k as f32 / last * self.levels, o])).collect();
        if line.len() >= 2 {
            ui.painter().with_clip_rect(map.area.expand(width)).add(Shape::line(line, Stroke::new(width, color)));
        }
    }

    /// The points as handles, children of the graph's node. Returns their ids.
    fn points(&self, ui: &mut Ui, style: &CurveStyle, map: Map, points: &[[f32; 2]], corners: &[bool], selected: Option<usize>) -> Vec<Id> {
        let n = points.len();
        let count = self.numbers.format(n as f64, 0..=0);
        points
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                let name = fill(&self.labels.point, &[("i", &self.numbers.format((i + 1) as f64, 0..=0)), ("n", &count)]);
                let is_corner = corners.get(i).copied().unwrap_or(false);
                let template = if is_corner { &self.labels.corner_value } else { &self.labels.point_value };
                let value = fill(template, &[("x", &self.value_text(p[0])), ("y", &self.value_text(p[1]))]);
                let is_selected = selected == Some(i);
                let shape = if self.corners && !is_corner { HandleShape::Round } else { HandleShape::Square };
                Handle::new(shape, name)
                    .value(value)
                    .id_salt(("point", i))
                    .sense(Sense::hover())
                    .look(if is_selected { style.selected } else { style.point })
                    .selected(is_selected)
                    .show(ui, map.screen(p))
                    .id
            })
            .collect()
    }

    /// A level as the fields write it: whole levels, or a whole percent with its unit.
    fn value_text(&self, level: f32) -> String {
        match self.percent {
            true => fill(&self.labels.percent, &[("n", &self.numbers.format(f64::from(self.field_value(level)), 0..=0))]),
            false => self.numbers.format(f64::from(level), 0..=0),
        }
    }

    fn field_value(&self, level: f32) -> f32 {
        match self.percent {
            true => (level / self.levels * 100.0).round(),
            false => level,
        }
    }

    fn field_level(&self, value: f64) -> f32 {
        match self.percent {
            true => (value as f32 / 100.0 * self.levels).round(),
            false => value as f32,
        }
    }

    /// The Input/Output row of point `selected` of the editor's value, disabled without one: two
    /// parts of the editor's edit drawn with `policy`, whose operations name their point.
    fn fields_row(
        &self,
        ui: &mut Ui,
        id: Id,
        selected: Option<usize>,
        policy: EditPolicy,
        edits: &mut parts::Edits<Curve, EditOp>,
        apply: impl FnMut(&EditOp, &Curve) -> Option<Curve> + Copy,
    ) {
        let points = edits.value().points().map(<[_]>::to_vec).unwrap_or_default();
        let at = selected.filter(|&i| i < points.len());
        let key = at.and_then(|i| PointKey::of(&points, i));
        let enabled = self.enabled && at.is_some();
        let field = |salt: &'static str, label: &str, axis: Axis| {
            let value = at.map_or(0.0, |i| points[i][axis.index()]);
            let rule = self.field_rule(&points, at, axis);
            let mut field = NumberField::new(salt, label, f64::from(self.field_value(value)), self.numbers)
                .range(rule.bounds.0..=rule.bounds.1)
                .step(rule.step)
                .decimals(0..=rule.places)
                .policy(policy)
                .part(true)
                .enabled(enabled);
            if self.percent {
                field = field.unit(Unit::new(&self.labels.percent));
            }
            field
        };
        let row = ui.push_id((id, "fields"), |ui| {
            PropertyRow::pair(PropertyLabel::word(self.labels.input.as_str()), PropertyLabel::word(self.labels.output.as_str())).show(
                ui,
                |ui, _| field("input", &self.labels.input, Axis::Input).show(ui),
                |ui, _| field("output", &self.labels.output, Axis::Output).show(ui),
            )
        });
        let (input, output) = row.inner.inner;
        for (axis, edit) in [(Axis::Input, &input), (Axis::Output, &output)] {
            let reported = ops::take::<ScalarOp>(ui.ctx(), edit.response.id);
            let field_ops = reported.into_iter().filter_map(|op| Some(EditOp::Field(axis, key?, op)));
            let map = |v: f64, now: &Curve| {
                key.and_then(|key| self.apply(&EditOp::Field(axis, key, ScalarOp::Set(v)), now, &mut None)).unwrap_or_else(|| now.clone())
            };
            edits.typing(edit, field_ops, apply, map);
        }
        if self.corners {
            let mut corner = at.is_some_and(|i| edits.value().is_corner(i));
            let edit = Checkbox::bool(self.labels.corner.as_str(), &mut corner).id_salt((id, "corner")).enabled(enabled).show(ui);
            let ops = edit.event.as_ref().and_then(|_| Some(EditOp::Graph(CurveOp::Corner(key?, corner))));
            let map = |on: bool, now: &Curve| {
                key.and_then(|key| self.apply(&EditOp::Graph(CurveOp::Corner(key, on)), now, &mut None)).unwrap_or_else(|| now.clone())
            };
            edits.push(&edit, ops, apply, map);
        }
    }

    /// The rule of the `axis` field of point `at` of `points` (none: the row without a point), in
    /// the field's units. The input stays strictly between the neighbours' inputs, so the field
    /// never shows what the curve can't take.
    fn field_rule(&self, points: &[[f32; 2]], at: Option<usize>, axis: Axis) -> FieldRule {
        let (lo, hi) = match (axis, at) {
            (Axis::Input, Some(i)) => {
                let lo = i.checked_sub(1).map_or(0.0, |j| points[j][0] + 1.0);
                let hi = points.get(i + 1).map_or(self.levels, |p| p[0] - 1.0);
                (lo, hi.max(lo))
            }
            _ => (0.0, self.levels),
        };
        FieldRule::new(&(f64::from(self.field_value(lo))..=f64::from(self.field_value(hi))), 1.0, 0)
    }

    /// What `op` makes of `curve` ([`Rules::apply`], which keeps `selected` on its point). A
    /// field's operation applies to its point with that field's rule on `curve`.
    fn apply(&self, op: &EditOp, curve: &Curve, selected: &mut Option<usize>) -> Option<Curve> {
        let rules = Rules { levels: self.levels, max_points: self.max_points };
        match op {
            EditOp::Graph(op) => rules.apply(op, curve, selected, |points, corners| self.samples(points, corners)),
            EditOp::Field(axis, at, op) => {
                let mut points = curve.points()?.to_vec();
                let i = at.find(&points)?;
                let from = f64::from(self.field_value(points[i][axis.index()]));
                let value = number_field::apply(op, from, &self.field_rule(&points, Some(i), *axis));
                let mut to = points[i];
                to[axis.index()] = self.field_level(value);
                rules.move_to(&mut points, i, to);
                Some(Curve::from_parts(points, curve.corner_flags()))
            }
        }
    }
}

/// Levels to screen and back, over the value area.
#[derive(Clone, Copy, Debug)]
struct Map {
    area: Rect,
    levels: f32,
}

impl Map {
    fn screen(&self, q: [f32; 2]) -> Pos2 {
        let a = self.area;
        pos2(a.left() + q[0] / self.levels * a.width(), a.bottom() - q[1] / self.levels * a.height())
    }

    /// The levels under `p`, clamped to the curve's range.
    fn value_at(&self, p: Pos2) -> [f32; 2] {
        let a = self.area;
        let x = (p.x - a.left()) / a.width().max(1.0) * self.levels;
        let y = (a.bottom() - p.y) / a.height().max(1.0) * self.levels;
        [x.clamp(0.0, self.levels), y.clamp(0.0, self.levels)]
    }

    /// The point of `points` nearest `pos` within `radius`.
    fn hit(&self, points: &[[f32; 2]], pos: Pos2, radius: f32) -> Option<usize> {
        points
            .iter()
            .enumerate()
            .map(|(i, &q)| (i, self.screen(q).distance(pos)))
            .filter(|(_, d)| *d <= radius)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
}

/// One frame's edit of the points: the pointer and the keys feed the gesture.
struct PointEdit<'g, 'p> {
    gesture: &'g mut Gesture<Curve>,
    points: &'p mut Vec<[f32; 2]>,
    /// Whether each point is a corner point.
    corner_flags: &'p mut Vec<bool>,
    state: &'g mut State,
    rules: Rules,
    map: Map,
    style: &'g CurveStyle,
    /// The output of every input level through the points and corner flags.
    samples: &'g CornerSamples<'g>,
    /// The operations of this frame's one-shot changes, in order.
    ops: Vec<CurveOp>,
    /// The selection before the first of `ops`.
    before_ops: Option<Option<usize>>,
}

impl PointEdit<'_, '_> {
    fn curve(&self) -> Curve {
        Curve::from_parts(self.points.clone(), self.corner_flags.clone())
    }

    /// Takes the flags along after the pointer inserted, removed or moved points that were
    /// `before`.
    fn follow(&mut self, before: &[[f32; 2]]) {
        follow_corners(before, self.points, self.corner_flags);
    }

    /// A one-shot change of the points: `op` applied to them and to the selection as its replay
    /// would apply it ([`Rules::apply`]). One that changes nothing sends nothing.
    fn apply(&mut self, op: CurveOp) {
        let before = self.state.selected;
        let Some(curve) = self.rules.apply(&op, &self.curve(), &mut self.state.selected, self.samples) else { return };
        self.before_ops.get_or_insert(before);
        if let Some(points) = curve.points() {
            *self.points = points.to_vec();
            *self.corner_flags = curve.corner_flags();
        }
        self.ops.push(op);
        self.gesture.set(curve);
    }

    /// The press, drag and release of PhotoCraft's `point_curve`, and the right-click and
    /// Cmd/Ctrl-click deletes. A press gives the graph the focus and takes its turn from the edit
    /// order (`edit::order`): while a typing run has the focus (the Input field's), the press
    /// waits, keeping its button and Cmd/Ctrl, until the run's ending event has gone out, and
    /// then edits the curve the run leaves. A press released while it waited still runs.
    fn pointer(&mut self, ui: &Ui, response: &Response) {
        // Hit-test where the button went down, not where egui first recognises a drag.
        let press = response.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_pressed());
        let command = ui.input(|i| i.modifiers.command);
        let new = if press {
            ui.input(|i| i.pointer.press_origin()).map(|pos| Press { pos, secondary: false, command })
        } else if response.secondary_clicked() && self.state.drag.is_none() {
            response.interact_pointer_pos().map(|pos| Press { pos, secondary: true, command })
        } else {
            None
        };
        if new.is_some() {
            response.request_focus();
        }
        let pass = ui.ctx().cumulative_pass_nr();
        let held = self.state.held.take().filter(|(last, _)| last + 1 == pass).map(|(_, press)| press);
        if let Some(press) = held.or(new) {
            match order::turn(ui.ctx(), response.id, self.gesture.id()) {
                Turn::Waits => self.state.held = Some((pass, press)),
                Turn::Now | Turn::Runs => self.press(ui, press),
            }
        }
        let released = ui.input(|i| i.pointer.primary_released());
        if !press
            && (response.is_pointer_button_down_on() || released)
            && let Some(drag) = self.state.drag
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        {
            self.drag_to(ui, drag, pos);
        }
        if self.state.drag.is_some() && !response.is_pointer_button_down_on() {
            self.state.drag = None;
            self.gesture.commit();
        }
    }

    fn press(&mut self, ui: &Ui, press: Press) {
        let Press { pos, secondary, command } = press;
        let before = self.state.selected;
        match self.map.hit(self.points, pos, self.style.hit) {
            Some(i) if secondary || command => {
                if let Some(at) = PointKey::of(self.points, i) {
                    self.apply(CurveOp::Delete(at));
                }
                self.state.selected = None;
            }
            Some(i) => {
                let at = self.map.value_at(pos);
                let p = self.points[i];
                self.state.selected = Some(i);
                self.state.drag = Some(Drag { index: Some(i), offset: [p[0] - at[0], p[1] - at[1]], before, corner: false });
                self.gesture.begin();
            }
            None if secondary => self.state.selected = None,
            // A press within two levels of a point's input takes that point to the press, as
            // PhotoCraft's `point_curve` does.
            None if !command => {
                let count = self.points.len();
                let was = self.points.clone();
                let Some(i) = self.rules.insert(self.points, self.map.value_at(pos)) else { return };
                self.follow(&was);
                self.state.selected = Some(i);
                self.state.drag = Some(Drag { index: Some(i), offset: [0.0, 0.0], before, corner: false });
                self.gesture.begin();
                if self.points.len() != count {
                    let curve = self.curve();
                    self.gesture.update(curve, ui.input(|i| i.modifiers));
                }
            }
            None => {}
        }
    }

    /// Moves the captured point to follow the pointer; far off the plot an interior point goes,
    /// and comes back on re-entry.
    fn drag_to(&mut self, ui: &Ui, drag: Drag, pos: Pos2) {
        let outside = !self.map.area.expand(self.style.pad + self.style.drag_off).contains(pos);
        let at = self.map.value_at(pos);
        let target = [at[0] + drag.offset[0], at[1] + drag.offset[1]];
        let before = self.points.clone();
        let changed = match drag.index {
            Some(i) if outside && Rules::can_delete(self.points, i) => {
                let corner = self.corner_flags.get(i).copied().unwrap_or(false);
                Rules::delete(self.points, i);
                self.state.drag = Some(Drag { index: None, corner, ..drag });
                self.state.selected = None;
                true
            }
            Some(i) => self.rules.move_to(self.points, i, target),
            None if !outside => {
                let count = self.points.len();
                let index = self.rules.insert(self.points, target);
                self.state.selected = index;
                self.state.drag = Some(Drag { index, ..drag });
                self.points.len() != count
            }
            None => false,
        };
        self.follow(&before);
        if let (None, Some(i)) = (drag.index, self.state.drag.and_then(|d| d.index))
            && drag.corner
            && changed
            && let Some(flag) = self.corner_flags.get_mut(i)
        {
            *flag = true;
        }
        if changed {
            let curve = self.curve();
            self.gesture.update(curve, ui.input(|i| i.modifiers));
        }
    }

    /// Selection, nudges, deletes and additions while the graph has focus and no drag is under
    /// way. Each change is one Set. Only `+` takes extra modifiers (many keyboards type it with
    /// Shift); every other key needs exactly its own, so Shift+Insert (paste) and Alt+arrows stay
    /// the app's.
    fn keys(&mut self, ui: &Ui) {
        if self.state.drag.is_some() {
            return;
        }
        let n = self.points.len();
        let take = |modifiers, key| take_exact(ui, modifiers, key);
        let plus = ui.input_mut(|i| i.count_and_consume_key(Modifiers::NONE, Key::Plus));
        for _ in 0..plus + take(Modifiers::NONE, Key::Equals) {
            self.state.selected = Some(self.state.selected.map_or(0, |i| (i + 1).min(n - 1)));
        }
        for _ in 0..take(Modifiers::NONE, Key::Minus) {
            self.state.selected = Some(self.state.selected.map_or(n - 1, |i| i.saturating_sub(1)));
        }
        if take(Modifiers::NONE, Key::Insert) + take(Modifiers::COMMAND, Key::Enter) > 0 {
            // A gap too narrow for a point (within two levels of a neighbour) adds nothing and
            // keeps the selection.
            let after = self.state.selected.and_then(|i| PointKey::of(self.points, i));
            self.apply(CurveOp::Insert { after });
        }
        let Some(at) = self.state.selected.and_then(|i| PointKey::of(self.points, i)) else { return };
        if take(Modifiers::NONE, Key::Delete) + take(Modifiers::NONE, Key::Backspace) > 0 {
            self.apply(CurveOp::Delete(at));
            return;
        }
        let mut d = [0.0f32; 2];
        for (key, dx, dy) in [(Key::ArrowLeft, -1.0, 0.0), (Key::ArrowRight, 1.0, 0.0), (Key::ArrowUp, 0.0, 1.0), (Key::ArrowDown, 0.0, -1.0)] {
            let steps = 10 * take(Modifiers::SHIFT, key) + take(Modifiers::NONE, key);
            d = [d[0] + dx * steps as f32, d[1] + dy * steps as f32];
        }
        if d != [0.0, 0.0] {
            self.apply(CurveOp::Nudge { at, by: d });
        }
    }
}

/// Takes the presses of `key` with exactly `modifiers` held (Ctrl and Cmd as egui's
/// [`Modifiers::matches_exact`] counts them); returns how many.
pub(crate) fn take_exact(ui: &Ui, modifiers: Modifiers, key: Key) -> usize {
    ui.input_mut(|i| {
        let before = i.events.len();
        i.events.retain(
            |event| !matches!(event, egui::Event::Key { key: k, pressed: true, modifiers: m, .. } if *k == key && m.matches_exact(modifiers)),
        );
        before - i.events.len()
    })
}

/// One frame's pencil stroke: the pointer draws straight segments into a map of outputs.
struct StrokeEdit<'g> {
    gesture: &'g mut Gesture<Curve>,
    state: &'g mut State,
    outputs: Vec<f32>,
    map: Map,
}

impl StrokeEdit<'_> {
    /// Press, drag and release as one gesture; a Shift press starts at the previous stroke's end.
    /// A press gives the graph the focus and takes its turn from the edit order (`edit::order`):
    /// while a typing run has the focus (an app's field of the same curve), the press waits,
    /// keeping its Shift, until the run's ending event has gone out, and then draws on the curve
    /// the run leaves. A press released while it waited still draws its point.
    fn pointer(&mut self, ui: &Ui, response: &Response) {
        let down = response.is_pointer_button_down_on();
        let press = down && ui.input(|i| i.pointer.primary_pressed());
        let new =
            press.then(|| ui.input(|i| i.pointer.press_origin())).flatten().map(|pos| StrokePress { pos, shift: ui.input(|i| i.modifiers.shift) });
        if new.is_some() {
            response.request_focus();
        }
        let pass = ui.ctx().cumulative_pass_nr();
        let held = self.state.stroke_held.take().filter(|(last, _)| last + 1 == pass).map(|(_, press)| press);
        if let Some(stroke) = held.or(new) {
            match order::turn(ui.ctx(), response.id, self.gesture.id()) {
                Turn::Waits => self.state.stroke_held = Some((pass, stroke)),
                Turn::Now | Turn::Runs => self.start(ui, stroke),
            }
        }
        let released = ui.input(|i| i.pointer.primary_released());
        if !press
            && (down || released)
            && let Some(last) = self.state.stroke
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        {
            let at = self.map.value_at(pos);
            self.state.stroke = Some(at);
            self.draw(ui, last, at);
        }
        if let Some(at) = self.state.stroke.filter(|_| !down) {
            self.state.stroke = None;
            self.state.end = Some(StrokeEnd { at, curve: self.gesture.shown().clone() });
            self.gesture.commit();
        }
    }

    fn start(&mut self, ui: &Ui, press: StrokePress) {
        let at = self.map.value_at(press.pos);
        let from = match press.shift {
            true => self.state.end.as_ref().map_or(at, |end| end.at),
            false => at,
        };
        self.gesture.begin();
        self.state.stroke = Some(at);
        self.draw(ui, from, at);
    }

    fn draw(&mut self, ui: &Ui, from: [f32; 2], to: [f32; 2]) {
        if Curve::pencil_line(&mut self.outputs, from, to, self.map.levels) {
            self.gesture.update(Curve::Map(self.outputs.clone()), ui.input(|i| i.modifiers));
        }
    }
}

/// A gradient bar from `dark` to `light`: left to right, or bottom to top when `vertical`.
pub(super) fn gradient(rect: Rect, dark: Color32, light: Color32, vertical: bool) -> Shape {
    let mut mesh = Mesh::default();
    let (a, b, c, d) = match vertical {
        true => (rect.left_bottom(), rect.right_bottom(), rect.right_top(), rect.left_top()),
        false => (rect.left_top(), rect.left_bottom(), rect.right_bottom(), rect.right_top()),
    };
    mesh.colored_vertex(a, dark);
    mesh.colored_vertex(b, dark);
    mesh.colored_vertex(c, light);
    mesh.colored_vertex(d, light);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    Shape::mesh(mesh)
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["CurveEditor"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story::new(
            "composites/curve-editor/curves",
            Level::Composites,
            "CurveEditor",
            "Curves",
            || {
                Args::new()
                    .with_choice("channel", &["RGB", "Red", "Green", "Blue"], 0)
                    .with_bool("pencil", false)
                    .with_bool("histogram", true)
                    .with_bool("fields", true)
                    .with_number("width", 300.0, 120.0, 400.0)
                    .with_choice("values", &["levels", "percent"], 0)
                    .with_choice("update rate", edit::RATES, 0)
                    .with_bool("cancellable", true)
                    .with_bool("on release only", false)
            },
            show_curves,
        ),
        typed_story!("composites/curve-editor/small", Level::Composites, "CurveEditor", "Small", NoArgs {}, show_small),
        typed_story!("composites/curve-editor/corners", Level::Composites, "CurveEditor", "Corners", NoArgs {}, show_corners),
    ]
}

/// A host's curves and the "before" of the gesture under way, kept between frames.
#[derive(Clone, Default)]
struct StoryCurves {
    curves: Vec<Curve>,
    before: Option<Curve>,
}

/// Applies `edit` to `curve` as a host without undo does, restoring the before value on a cancel.
fn apply(edit: &EditResponse<Curve>, curve: &mut Curve, before: &mut Option<Curve>) {
    use crate::edit::EditEvent;
    match &edit.event {
        Some(EditEvent::Begin { .. }) => *before = Some(curve.clone()),
        Some(EditEvent::Cancel { .. }) => {
            if let Some(b) = before.take() {
                *curve = b;
            }
        }
        Some(event) => {
            if let Some(value) = event.value() {
                *curve = value.clone();
            }
            if matches!(event, EditEvent::Commit { .. } | EditEvent::Set { .. }) {
                *before = None;
            }
        }
        None => {}
    }
}

fn story_curves(ui: &Ui, key: &str, initial: impl FnOnce() -> Vec<Curve>) -> (Id, StoryCurves) {
    let id = ui.scope_id().with(key);
    let curves = ui.data(|d| d.get_temp::<StoryCurves>(id)).unwrap_or_else(|| StoryCurves { curves: initial(), before: None });
    (id, curves)
}

/// Sample bins of an image's channel: two hills, the channel's mean shifted by `shift`.
pub(super) fn story_bins(shift: f32) -> Vec<u32> {
    (0..256)
        .map(|k| {
            let x = k as f32;
            let hill = |mean: f32, spread: f32, height: f32| height * (-((x - mean) / spread).powi(2)).exp();
            (hill(70.0 + shift, 28.0, 900.0) + hill(180.0 - shift, 40.0, 600.0) + 20.0) as u32
        })
        .collect()
}

/// A drawn curve as the story host's points again: every 32nd level and the end.
fn story_points(map: &[f32]) -> Curve {
    let last = map.len().saturating_sub(1);
    let mut points: Vec<[f32; 2]> = (0..last).step_by(32).map(|k| [k as f32, map[k]]).collect();
    points.push([last as f32, map.get(last).copied().unwrap_or(255.0)]);
    Curve::smooth(points)
}

/// PhotoCraft's Curves on an RGB image: a channel per arg, the other channels' curves and a
/// histogram behind the RGB curve, a pencil mode, a hint, the fields in levels or percent, and
/// the host's edit policy.
fn show_curves(ui: &mut Ui, args: &mut Args) {
    let channels = [
        CurveChannel::new("RGB", ChannelColor::Neutral),
        CurveChannel::new("Red", ChannelColor::Red),
        CurveChannel::new("Green", ChannelColor::Green),
        CurveChannel::new("Blue", ChannelColor::Blue),
    ];
    let channel = channels.iter().position(|c| c.label == args.choice("channel")).unwrap_or(0);
    let numbers = LocaleNumbers::from_tag("en");
    let (id, mut host) = story_curves(ui, "curve-editor-curves", || {
        let mut curves = vec![Curve::identity(255.0); 4];
        curves[0] = Curve::smooth(vec![[0.0, 0.0], [64.0, 48.0], [192.0, 214.0], [255.0, 255.0]]);
        curves[1] = Curve::smooth(vec![[0.0, 12.0], [128.0, 140.0], [255.0, 255.0]]);
        curves
    });
    let width = args.number("width") as f32;
    let pencil = args.bool("pencil");
    if !pencil && let Curve::Map(map) = &host.curves[channel] {
        host.curves[channel] = story_points(map);
    }
    let bins = story_bins([0.0, 0.0, 12.0, -12.0][channel]);
    let data = HistogramData::tone(&bins, channels[channel].color);
    let overlays = if channel == 0 { host.curves.clone() } else { Vec::new() };
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let mut editor = CurveEditor::new("curves", "Curve", &host.curves[channel], &numbers)
            .channels(&channels)
            .channel(channel)
            .overlays(&overlays)
            .pencil(pencil)
            .fields(args.bool("fields"))
            .policy(edit::policy(args))
            .hint(match pencil {
                true => "Drag to draw · Shift-click for a straight line from the last stroke · Esc cancels",
                false => "Click to add a point · drag off or Ctrl-click or Delete to remove",
            });
        if args.bool("histogram") {
            editor = editor.histogram(&data);
        }
        if args.choice("values") == "percent" {
            editor = editor.percent();
        }
        let edit = editor.show(ui);
        let StoryCurves { curves, before } = &mut host;
        apply(&edit, &mut curves[channel], before);
    });
    ui.data_mut(|d| d.insert_temp(id, host));
}

/// Camera Raw's tone curve: 260 square, no fields, no channels.
fn show_small(ui: &mut Ui, _: &mut NoArgs) {
    let numbers = LocaleNumbers::from_tag("en");
    let (id, mut host) = story_curves(ui, "curve-editor-small", || vec![Curve::smooth(vec![[0.0, 0.0], [96.0, 80.0], [255.0, 255.0]])]);
    let edit = CurveEditor::new("tone-curve", "Tone curve", &host.curves[0], &numbers).side(260.0).fields(false).show(ui);
    let StoryCurves { curves, before } = &mut host;
    apply(&edit, &mut curves[0], before);
    ui.data_mut(|d| d.insert_temp(id, host));
}

/// The contour editor: 220 square, percent fields, the Corner checkbox toggles the selected point.
fn show_corners(ui: &mut Ui, _: &mut NoArgs) {
    let numbers = LocaleNumbers::from_tag("en");
    let (id, mut host) = story_curves(ui, "curve-editor-corners", || {
        vec![Curve::from_parts(
            vec![[0.0, 0.0], [64.0, 160.0], [128.0, 96.0], [192.0, 224.0], [255.0, 255.0]],
            vec![false, false, true, false, false],
        )]
    });
    let edit = CurveEditor::new("contour", "Contour", &host.curves[0], &numbers).side(220.0).percent().corners(true).show(ui);
    let StoryCurves { curves, before } = &mut host;
    apply(&edit, &mut curves[0], before);
    ui.data_mut(|d| d.insert_temp(id, host));
}
