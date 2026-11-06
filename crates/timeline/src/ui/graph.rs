//! The graph editor: keyframed values (or their speed) as curves over time (N1.08).

use craft_ui::behavior::{ClaimedKey, KeyClaim, ListSelection, RowInfo, Selection, SelectionChange, SelectionMode, SelectionPick};
use craft_ui::canvas::Orientation;
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::icons::Icon;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::patterns::{Toolbar, ToolbarItem};
use craft_ui::primitives::{Corner, FocusRing};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, UiTokens, role_format, scale};
use egui::accesskit::Role;
use egui::{
    Align2, AsIdSalt, Color32, EventFilter, Id, IdSalt, Key, Modifiers, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui,
    UiBuilder, WidgetInfo, pos2, vec2,
};

use super::keyframes::{KeyframeId, KeyframeInterpolation};
use super::ruler::ruler_ticks;
use super::style::TimelineStyle;
use crate::time::{Tick, TimeView};
use crate::timecode::TimeFormat;

/// The value range shows about this many grid lines (EffectCraft `graph.rs:439`).
const GRID_LINES: f64 = 6.0;
/// The share of the value span the auto range adds each side (`graph.rs:352`).
const AUTO_MARGIN: f64 = 0.08;
/// The share of the span Fit Selection adds each side, and the least it adds in value units
/// (`graph.rs:790`, `:789`).
const FIT_MARGIN: f64 = 0.15;
const FIT_MARGIN_MIN: f64 = 1.0;
/// The share of the plot's height a reference graph leaves free above and below (EffectCraft
/// `graph.rs:318`: it is drawn in 90 % of the height). A proportion of the plot, not a size.
const REFERENCE_PAD: f32 = 0.05;
/// The share of a curve's colour a reference graph keeps (EffectCraft `graph_tools.rs:320`).
const REFERENCE_FADE: f32 = 0.35;

craft_ui::words! {
    /// The bottom bar's tooltips, English by default (EffectCraft `graph.rs:611`).
    #[non_exhaustive]
    pub struct GraphEditorLabels {
        title: "graphEditor.title" = "Graph Editor";
        value_graph: "graphEditor.valueGraph" = "Edit Value Graph";
        speed_graph: "graphEditor.speedGraph" = "Edit Speed Graph";
        show_selected: "graphEditor.showSelected" = "Show Selected Properties (else all animated properties)";
        show_reference: "graphEditor.showReference" = "Show Reference Graph";
        auto_zoom: "graphEditor.autoZoom" = "Auto-zoom graph height";
        fit_selection: "graphEditor.fitSelection" = "Fit selection to view";
        fit_all: "graphEditor.fitAll" = "Fit all graphs to view";
        separate: "graphEditor.separate" = "Separate Dimensions";
        hold: "graphEditor.hold" = "Convert selected keyframes to Hold";
        linear: "graphEditor.linear" = "Convert selected keyframes to Linear";
        auto_bezier: "graphEditor.autoBezier" = "Convert selected keyframes to Auto Bezier";
        easy_ease: "graphEditor.easyEase" = "Easy Ease (F9)";
        easy_ease_in: "graphEditor.easyEaseIn" = "Easy Ease In (Shift+F9)";
        easy_ease_out: "graphEditor.easyEaseOut" = "Easy Ease Out (Ctrl+Shift+F9)";
    }
}

/// Which graph shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GraphMode {
    #[default]
    /// The property's value over time.
    Value,
    /// The rate of change of the value over time.
    Speed,
}

/// A key on a curve, with its Bezier handles in graph units (time, value) when it has them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphKey {
    /// The keyframe's identity.
    pub id: KeyframeId,
    /// The keyframe's time.
    pub time: Tick,
    /// The keyframe's value.
    pub value: f64,
    /// How the curve leaves the keyframe.
    pub interpolation: KeyframeInterpolation,
    /// The incoming Bezier handle as an absolute (time, value) point; `None` without one.
    pub in_handle: Option<(Tick, f64)>,
    /// The outgoing Bezier handle as an absolute (time, value) point; `None` without one.
    pub out_handle: Option<(Tick, f64)>,
}

/// One curve: a property's dimension, sampled by the app (the app evaluates its own
/// interpolation and expressions).
#[derive(Clone, Debug, PartialEq)]
pub struct GraphCurve {
    /// The property and dimension, the app's.
    pub id: u64,
    /// The curve's display name.
    pub name: String,
    /// `None`: the next colour of the per-dimension set (red, green, blue, yellow; EffectCraft
    /// `graph.rs:22`).
    pub color: Option<Color32>,
    /// `(time, value)` in time order, about one per two points of width.
    pub samples: Vec<(Tick, f64)>,
    /// The curve's keyframes, in time order.
    pub keys: Vec<GraphKey>,
}

/// The graph's value range and options; the app keeps it per panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphRange {
    /// The lowest value shown at the bottom of the plot.
    pub low: f64,
    /// The highest value shown at the top of the plot.
    pub high: f64,
    /// Which graph shows.
    pub mode: GraphMode,
    /// The range follows the shown curves (After Effects' Auto-zoom graph height).
    pub auto_zoom: bool,
    /// Show only the selected properties' curves; otherwise all animated ones.
    pub selected_only: bool,
    /// Show each dimension of a property as its own curve.
    pub separate_dimensions: bool,
    /// Show Reference Graph: the other graph type, faint, scaled to the plot (EffectCraft
    /// `graph.rs:301`). The curves come from [`GraphEditor::reference`].
    pub reference: bool,
}

impl Default for GraphRange {
    fn default() -> Self {
        Self { low: 0.0, high: 100.0, mode: GraphMode::Value, auto_zoom: true, selected_only: false, separate_dimensions: false, reference: false }
    }
}

/// A move of the selected keys in time and value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GraphKeyMove {
    /// The shift in time.
    pub delta_time: Tick,
    /// The shift in value, in the curve's own units.
    pub delta_value: f64,
}

/// Which handle of a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandleSide {
    /// The incoming handle.
    In,
    /// The outgoing handle.
    Out,
}

/// A handle dragged to `(time, value)`; with Alt the two handles break apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandleMove {
    /// The keyframe the handle belongs to.
    pub key: KeyframeId,
    /// Which of the key's handles moved.
    pub side: HandleSide,
    /// The handle's new time.
    pub time: Tick,
    /// The handle's new value.
    pub value: f64,
    /// The two handles of the key move independently (Alt held).
    pub broken: bool,
}

/// A bottom-bar button, or a key on the plot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GraphAction {
    /// Delete with keys selected.
    Delete,
    /// Fit the view to the selected keys.
    FitSelection,
    /// Fit the view to every curve.
    FitAll,
    /// Convert the selected keys to Hold.
    Hold,
    /// Convert the selected keys to Linear.
    Linear,
    /// Convert the selected keys to Auto Bezier.
    AutoBezier,
    /// Easy Ease on the selected keys.
    EasyEase,
    /// Easy Ease In on the selected keys.
    EasyEaseIn,
    /// Easy Ease Out on the selected keys.
    EasyEaseOut,
}

/// The value / speed graph editor, After Effects' (EffectCraft).
///
/// - Value grid lines at a nice step ([`nice_step`]) with labels; time lines follow the
///   timeline's ruler (`view`); the playhead as a vertical line.
/// - Curves from the app's samples; keys as squares, selected ones filled with their handles
///   drawn (a line and a dot).
/// - The selection is the app's [`Selection`] of keys (shared with the keyframe lanes): a click
///   replaces, Shift+click toggles (After Effects'), a drag on empty graph box-selects (Shift
///   adds), each reported as a [`SelectionChange`]; dragging selected
///   keys moves them in time and value (Shift constrains to the axis moved most), reported as an
///   `EditEvent<GraphKeyMove>` gesture; dragging a handle reports an `EditEvent<HandleMove>`.
///   Esc cancels a drag.
/// - Ctrl/Cmd+wheel (and a pinch) zooms the value range about its middle (turns auto-zoom off),
///   read as egui's zoom delta; auto-zoom fits the shown curves each frame ([`auto_range`]).
/// - With [`GraphRange::reference`] and [`reference`](Self::reference) curves, the other graph
///   type is drawn faint behind the curves, scaled to the plot.
/// - The bottom bar, a horizontal [`Toolbar`]: Value / Speed graph,
///   selected only, auto-zoom, fit selection, fit all, reference graph, separate dimensions,
///   Hold, Linear, Auto Bezier, Easy Ease, Ease In, Ease Out; icon buttons with the tooltips of
///   [`GraphEditorLabels`].
/// - Keyboard: the plot is one Tab stop; the keys and handles are pointer-only. Arrows move the
///   selected keys (a frame / a value step; a key drag does not snap, as in EffectCraft), F9,
///   Shift+F9 and Ctrl+Shift+F9 report the Easy Ease actions; Delete and Backspace are claimed
///   through [`KeyClaim`] and reported as
///   [`GraphAction::Delete`]. Every chord matches its modifiers exactly. EffectCraft has no
///   keyboard selection of keys, so there is none.
///   AccessKit: the plot is a `Group` named by [`GraphEditorLabels`]' title with the
///   selected keys' count as its value.
///
/// The selection is applied by the editor and reported as a [`SelectionChange`] too.
pub struct GraphEditor<'a> {
    id_salt: IdSalt,
    curves: &'a [GraphCurve],
    reference: &'a [GraphCurve],
    selection: &'a mut Selection<KeyframeId>,
    view: &'a TimeView,
    range: &'a mut GraphRange,
    playhead: Tick,
    labels: Option<GraphEditorLabels>,
    format: Option<TimeFormat>,
}

impl<'a> GraphEditor<'a> {
    /// A graph editor for `curves` with the app's key `selection`, the time `view` of the timeline and the graph `range`, which the editor changes (zoom, mode, options).
    pub fn new(
        id_salt: impl AsIdSalt,
        curves: &'a [GraphCurve],
        selection: &'a mut Selection<KeyframeId>,
        view: &'a TimeView,
        range: &'a mut GraphRange,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), curves, reference: &[], selection, view, range, playhead: Tick::ZERO, labels: None, format: None }
    }

    /// The curves of the other graph type, which [`GraphRange::reference`] shows faint (the
    /// speed curves under the value graph, and the reverse). The app samples them as it does
    /// `curves`.
    pub fn reference(mut self, curves: &'a [GraphCurve]) -> Self {
        self.reference = curves;
        self
    }

    /// The playhead time, drawn as a vertical line.
    pub fn playhead(mut self, t: Tick) -> Self {
        self.playhead = t;
        self
    }

    /// The tooltip and accessibility words; `GraphEditorLabels::of` when not set.
    pub fn labels(mut self, labels: GraphEditorLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The time format of the time lines, and the frame the arrow keys move keys by; without one
    /// there are no time lines and the arrows move a key one point of the view.
    pub fn format(mut self, format: TimeFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Show the editor in `rect` and report what the user did.
    // effectcraft: crates/ui-egui/src/panels/graph.rs:407 `show`, `:356` `box_select`, `:379`
    // `wheel_zoom`, `:611` `button_bar`, `:740` `fit_selection`, `:842` `HandleGrab`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> GraphEditorOutput {
        let Self { id_salt, curves, reference, selection, view, range, playhead, labels, format } = self;
        let labels = labels.unwrap_or_else(|| GraphEditorLabels::of(ui));
        let tokens = Tokens::of(ui);
        let style = TimelineStyle::of(&tokens);
        let enabled = ui.is_enabled();
        let id = ui.make_persistent_id(id_salt);
        let bar = Rect::from_min_max(pos2(rect.left(), (rect.bottom() - tokens.size.control.md).max(rect.top())), rect.max);
        let plot = Rect::from_min_max(pos2(rect.left(), (rect.top() + tokens.space.gap.inline).min(bar.top())), pos2(rect.right(), bar.top()));
        let response = ui.interact(plot, id, Sense::click_and_drag());
        let modifiers = ui.input(|i| i.modifiers);

        // Gestures: the keys' move, a handle's move. Both carry the change from where the
        // pointer went down.
        let (keys_id, handle_id) = (id.with("keys"), id.with("handle"));
        let reported_keys = Gesture::<GraphKeyMove>::reported(ui, keys_id).map_or(GraphKeyMove::default(), |(_, delta)| delta);
        let reported_handle = Gesture::<(Tick, f64)>::reported(ui, handle_id).map_or((Tick::ZERO, 0.0), |(_, delta)| delta);
        let mut moving = Gesture::load(ui, keys_id, &reported_keys, EditPolicy::default()).enabled(enabled);
        let mut pulling = Gesture::load(ui, handle_id, &reported_handle, EditPolicy::default()).enabled(enabled);
        let key_drag_key = id.with("key drag");
        let handle_drag_key = id.with("handle drag");
        let marquee_key = id.with("marquee");
        if !moving.is_active() {
            ui.data_mut(|d| d.remove::<KeyDrag>(key_drag_key));
        }
        if !pulling.is_active() {
            ui.data_mut(|d| d.remove::<HandleDrag>(handle_drag_key));
        }
        let key_drag = ui.data(|d| d.get_temp::<KeyDrag>(key_drag_key));
        let handle_drag = ui.data(|d| d.get_temp::<HandleDrag>(handle_drag_key));
        let dragging = moving.is_active() || pulling.is_active();

        // The value range: fitted to the curves unless the user set it.
        if range.auto_zoom && !dragging {
            let (low, high) = auto_range(curves, range.mode);
            (range.low, range.high) = (low, high);
        }
        let (low, high) = (range.low, range.high);
        let span = (high - low).max(f64::EPSILON);
        let y_of = |value: f64| plot.bottom() - ((value - low) / span) as f32 * plot.height();
        let value_at = |y: f32| low + f64::from(plot.bottom() - y) / f64::from(plot.height().max(f32::EPSILON)) * span;
        let x_of = |time: Tick| view.x_of(time, plot.left());
        let time_at = |x: f32| view.time_at(x, plot.left());

        let mut out = GraphEditorOutput { response, selection: None, keys_moved: None, handle_moved: None, actions: Vec::new() };
        let step = nice_step(span / GRID_LINES);
        let frame = format
            .as_ref()
            .map_or_else(|| Tick::from_seconds_f64(1.0 / view.points_per_second.max(f64::EPSILON)), |format| format.rate.frame_duration());
        let order = key_order(curves);
        let rules = ListSelection::new(SelectionMode::Multiple);
        let position = |key: KeyframeId| order.iter().position(|row| row.key == key);

        // The keys as the gestures show them.
        let (moved, pulled) = (*moving.shown(), *pulling.shown());
        let key_at = |key: &GraphKey| {
            let shifted = key_drag.as_ref().is_some_and(|drag| drag.group.contains(&key.id));
            let (dt, dv) = if shifted { (moved.delta_time, moved.delta_value) } else { (Tick::ZERO, 0.0) };
            pos2(x_of(key.time + dt), y_of(key.value + dv))
        };
        let handle_at = |key: &GraphKey, side: HandleSide, at: (Tick, f64)| match handle_drag {
            Some(grab) if grab.key == key.id && grab.side == side => pos2(x_of(grab.time + pulled.0), y_of(grab.value + pulled.1)),
            _ => {
                let shifted = key_drag.as_ref().is_some_and(|drag| drag.group.contains(&key.id));
                let (dt, dv) = if shifted { (moved.delta_time, moved.delta_value) } else { (Tick::ZERO, 0.0) };
                pos2(x_of(at.0 + dt), y_of(at.1 + dv))
            }
        };

        if enabled {
            // Keys and handles take the pointer before the plot's own drag.
            for (index, curve) in curves.iter().enumerate() {
                for key in &curve.keys {
                    let on = selection.contains(&key.id);
                    for (side, handle) in [(HandleSide::In, key.in_handle), (HandleSide::Out, key.out_handle)] {
                        let Some(at) = handle.filter(|_| on) else { continue };
                        let hit = Rect::from_center_size(handle_at(key, side, at), vec2(style.graph_handle_hit, style.graph_handle_hit));
                        let handle_response = ui.interact(hit, id.with(("handle", index, key.id.0, side as u8)), Sense::DRAG);
                        if handle_response.drag_started_by(PointerButton::Primary)
                            && let Some(press) = ui.input(|i| i.pointer.press_origin())
                        {
                            out.response.request_focus();
                            let grab = HandleDrag { press, key: key.id, side, time: at.0, value: at.1, broken: modifiers.alt };
                            ui.data_mut(|d| d.insert_temp(handle_drag_key, grab));
                            pulling.begin();
                        }
                        if let Some(grab) =
                            ui.data(|d| d.get_temp::<HandleDrag>(handle_drag_key)).filter(|grab| grab.key == key.id && grab.side == side)
                            && handle_response.dragged_by(PointerButton::Primary)
                            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
                        {
                            let (time_ok, value_ok) = axes(modifiers.shift, pos.x - grab.press.x, pos.y - grab.press.y);
                            let dt = if time_ok { time_at(pos.x) - time_at(grab.press.x) } else { Tick::ZERO };
                            let dv = if value_ok { value_at(pos.y) - value_at(grab.press.y) } else { 0.0 };
                            pulling.update((dt, dv), modifiers);
                        }
                        if handle_response.drag_stopped() {
                            pulling.commit();
                        }
                    }
                    let center = key_at(key);
                    if !plot.expand(style.graph_reach).contains(center) && !key_drag.as_ref().is_some_and(|drag| drag.group.contains(&key.id)) {
                        continue;
                    }
                    let hit = Rect::from_center_size(center, vec2(style.graph_key_hit, style.graph_key_hit));
                    let key_response = ui.interact(hit, id.with(("key", index, key.id.0)), Sense::CLICK | Sense::DRAG);
                    if key_response.drag_started_by(PointerButton::Primary)
                        && let Some(press) = ui.input(|i| i.pointer.press_origin())
                    {
                        out.response.request_focus();
                        let additive = modifiers.shift || modifiers.command;
                        let mut group = selection.items.clone();
                        if !selection.contains(&key.id) {
                            let mut next = if additive { Selection { items: group, ..Selection::new() } } else { Selection::new() };
                            next.items.push(key.id);
                            next.lead = Some(key.id);
                            next.anchor = Some(key.id);
                            group = next.items.clone();
                            let pick = if additive { SelectionPick::Toggle } else { SelectionPick::Replace };
                            out.selection = Some(SelectionChange { pick, row: Some(key.id), selection: next });
                        }
                        ui.data_mut(|d| d.insert_temp(key_drag_key, KeyDrag { press, group }));
                        moving.begin();
                    }
                    if let Some(grab) = ui.data(|d| d.get_temp::<KeyDrag>(key_drag_key))
                        && key_response.dragged_by(PointerButton::Primary)
                        && grab.group.contains(&key.id)
                        && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
                    {
                        let (time_ok, value_ok) = axes(modifiers.shift, pos.x - grab.press.x, pos.y - grab.press.y);
                        let dt = if time_ok { time_at(pos.x) - time_at(grab.press.x) } else { Tick::ZERO };
                        let dv = if value_ok { value_at(pos.y) - value_at(grab.press.y) } else { 0.0 };
                        moving.update(GraphKeyMove { delta_time: dt, delta_value: dv }, modifiers);
                    }
                    if key_response.drag_stopped() {
                        moving.commit();
                    }
                    if key_response.clicked_by(PointerButton::Primary) {
                        out.response.request_focus();
                        // Shift toggles a key as Cmd/Ctrl does (After Effects').
                        let toggles = Modifiers { command: modifiers.command || modifiers.shift, ..Modifiers::NONE };
                        out.selection = position(key.id).and_then(|at| rules.click(selection, order.as_slice(), at, toggles));
                    }
                }
            }

            // The plot itself: a click on empty graph clears, a drag box-selects.
            let marquee = MarqueeCx { marquee_key, plot, rules: &rules, dragging, modifiers };
            marquee_select(ui, &mut out, &marquee, &*selection, curves, &key_at);

            // The wheel with Ctrl zooms the value range about its middle and ends auto-zoom.
            wheel_zoom(ui, plot, range);

            // Keyboard: arrows move the selected keys, F9 the Easy Ease family, Delete deletes.
            keyboard(ui, id, &mut out, selection, (step, frame), &mut moving);
        }

        // This pass's selection change shows in this pass's paint.
        if let Some(change) = &out.selection {
            selection.clone_from(&change.selection);
        }

        out.keys_moved = moving.finish();
        out.handle_moved = match handle_drag {
            Some(grab) => pulling.finish_as(|(dt, dv)| HandleMove {
                key: grab.key,
                side: grab.side,
                time: grab.time + dt,
                value: grab.value + dv,
                broken: grab.broken,
            }),
            None => {
                let _ = pulling.finish();
                None
            }
        };

        let reference = if range.reference { reference } else { &[] };
        let frame_of_plot = PlotFrame { rect, plot, low, high, step, playhead, marquee_key, reference };
        paint_plot(ui, &tokens, &style, &frame_of_plot, (curves, &*selection, view, format.as_ref()), (&x_of, &y_of, &key_at, &handle_at));
        FocusRing::paint_radius(ui, plot, Corner::Control.radius(&tokens), &out.response);
        out.response.widget_info(|| WidgetInfo::labeled(Role::Group, enabled, labels.title.as_str()));
        let count = selection.items.len().to_string();
        ui.ctx().accesskit_node_builder(out.response.id, |node| node.set_value(count.as_str()));

        bottom_bar(ui, (id, bar), &tokens, &labels, range, (curves, &*selection), &mut out);
        out
    }
}

/// What the plot's box selection needs besides the selection itself.
struct MarqueeCx<'a> {
    marquee_key: Id,
    plot: Rect,
    rules: &'a ListSelection,
    dragging: bool,
    modifiers: Modifiers,
}

/// The plot itself: a click on empty graph clears, a drag box-selects (Shift adds, Esc cancels).
fn marquee_select(
    ui: &mut Ui,
    out: &mut GraphEditorOutput,
    cx: &MarqueeCx<'_>,
    selection: &Selection<KeyframeId>,
    curves: &[GraphCurve],
    key_at: &dyn Fn(&GraphKey) -> Pos2,
) {
    let MarqueeCx { marquee_key, plot, rules, dragging, modifiers } = *cx;
    if out.response.drag_started_by(PointerButton::Primary)
        && !dragging
        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
    {
        out.response.request_focus();
        let base = if modifiers.shift { selection.clone() } else { Selection::new() };
        ui.data_mut(|d| d.insert_temp(marquee_key, Marquee { origin, base }));
    }
    if out.response.drag_stopped() {
        ui.data_mut(|d| d.remove::<Marquee>(marquee_key));
    }
    if let Some(marquee) = ui.data(|d| d.get_temp::<Marquee>(marquee_key)) {
        let cancelled = pressed(ui, Key::Escape);
        if cancelled {
            ui.data_mut(|d| d.remove::<Marquee>(marquee_key));
        }
        let next = if cancelled {
            Some(marquee.base.clone())
        } else {
            ui.input(|i| i.pointer.interact_pos())
                .map(|pos| marquee.extended(Rect::from_two_pos(marquee.origin, pos).intersect(plot), curves, &key_at))
        };
        if let Some(next) = next.filter(|next| next != selection) {
            out.selection = Some(SelectionChange {
                pick: if next.is_empty() { SelectionPick::Clear } else { SelectionPick::Replace },
                row: next.lead,
                selection: next,
            });
        }
    }
    if out.response.clicked_by(PointerButton::Primary) {
        out.response.request_focus();
        if !modifiers.shift && !modifiers.command {
            out.selection = rules.click_empty(selection);
        }
    }
}

/// Ctrl/Cmd+wheel (and a pinch) over the plot zooms the value range about its middle and ends
/// auto-zoom. egui turns the wheel into its zoom delta (a factor of e per 200 points of travel,
/// as EffectCraft `graph.rs:386` reads it), so the range scales by the delta's inverse.
fn wheel_zoom(ui: &Ui, plot: Rect, range: &mut GraphRange) {
    if !ui.rect_contains_pointer(plot) {
        return;
    }
    let zoom = ui.input(|i| i.zoom_delta());
    if zoom != 1.0 && zoom.is_finite() && zoom > 0.0 {
        let factor = 1.0 / f64::from(zoom);
        let (low, high) = (range.low, range.high);
        let middle = (low + high) / 2.0;
        (range.low, range.high) = (middle - (middle - low) * factor, middle + (high - middle) * factor);
        range.auto_zoom = false;
    }
}

/// Whether `key` was pressed with no modifier this pass; an extra Shift or Alt is another chord.
fn pressed(ui: &Ui, key: Key) -> bool {
    chord(ui, Modifiers::NONE, key)
}

/// Whether `key` was pressed with exactly `modifiers` this pass, and consumes it.
fn chord(ui: &Ui, modifiers: Modifiers, key: Key) -> bool {
    ui.input_mut(|i| ClaimedKey::exact(modifiers, key).consume(i))
}

/// The plot's keys: arrows move the selected keys, F9 the Easy Ease family, Delete deletes.
fn keyboard(
    ui: &Ui,
    id: Id,
    out: &mut GraphEditorOutput,
    selection: &Selection<KeyframeId>,
    (step, frame): (f64, Tick),
    moving: &mut Gesture<GraphKeyMove>,
) {
    if out.response.has_focus() {
        ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { horizontal_arrows: true, vertical_arrows: true, ..EventFilter::default() }));
        KeyClaim::claim(ui.ctx(), id, &[ClaimedKey::exact(Modifiers::NONE, Key::Delete), ClaimedKey::exact(Modifiers::NONE, Key::Backspace)]);
        let mut nudge = GraphKeyMove::default();
        if pressed(ui, Key::ArrowLeft) {
            nudge.delta_time = Tick(-frame.0);
        }
        if pressed(ui, Key::ArrowRight) {
            nudge.delta_time = frame;
        }
        if pressed(ui, Key::ArrowUp) {
            nudge.delta_value = step;
        }
        if pressed(ui, Key::ArrowDown) {
            nudge.delta_value = -step;
        }
        if nudge != GraphKeyMove::default() && !selection.is_empty() {
            moving.set(nudge);
        }
        if chord(ui, Modifiers::CTRL | Modifiers::SHIFT, Key::F9) {
            out.actions.push(GraphAction::EasyEaseOut);
        }
        if chord(ui, Modifiers::SHIFT, Key::F9) {
            out.actions.push(GraphAction::EasyEaseIn);
        }
        if pressed(ui, Key::F9) {
            out.actions.push(GraphAction::EasyEase);
        }
        if !selection.is_empty() && (pressed(ui, Key::Delete) || pressed(ui, Key::Backspace)) {
            out.actions.push(GraphAction::Delete);
        }
    }
}

/// The plot's place and range, and what it marks.
struct PlotFrame<'a> {
    rect: Rect,
    plot: Rect,
    low: f64,
    high: f64,
    step: f64,
    playhead: Tick,
    marquee_key: Id,
    /// The other graph type's curves, empty unless they show.
    reference: &'a [GraphCurve],
}

/// What the plot shows: the curves, the selection, the time view and the time format.
type PlotData<'a> = (&'a [GraphCurve], &'a Selection<KeyframeId>, &'a TimeView, Option<&'a TimeFormat>);
/// How the plot places things: time to x, value to y, a key's centre, a handle's centre.
type PlotMaps<'a> = (&'a dyn Fn(Tick) -> f32, &'a dyn Fn(f64) -> f32, KeyAt<'a>, HandleAt<'a>);
type KeyAt<'a> = &'a dyn Fn(&GraphKey) -> Pos2;
type HandleAt<'a> = &'a dyn Fn(&GraphKey, HandleSide, (Tick, f64)) -> Pos2;

/// The grid, curves, keys, handles, box selection and playhead.
fn paint_plot(
    ui: &Ui,
    tokens: &UiTokens,
    style: &TimelineStyle,
    frame: &PlotFrame<'_>,
    (curves, selection, view, format): PlotData<'_>,
    (x_of, y_of, key_at, handle_at): PlotMaps<'_>,
) {
    let PlotFrame { rect, plot, low, high, step, playhead, marquee_key, reference } = *frame;
    let color = &tokens.color;
    let ppp = ui.pixels_per_point();
    let hairline = scale::Stroke::Hairline.width(ppp);
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(plot, egui::CornerRadius::ZERO, color.surface.sunken.to_egui());
    let plot_painter = ui.painter().with_clip_rect(plot);
    let grid = Stroke::new(hairline, color.line.divider.to_egui());
    if let Some(format) = &format {
        for tick in ruler_ticks(format, view, plot.width(), style).iter().filter(|tick| tick.major) {
            let x = x_of(tick.at);
            plot_painter.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], grid);
        }
    }
    let text = role_format(tokens, style.graph_text, color.text.faint.to_egui());
    let numbers = LocaleNumbers::of(ui);
    let mut line = (low / step).ceil() * step;
    while line <= high {
        let y = y_of(line);
        plot_painter.line_segment([pos2(plot.left(), y), pos2(plot.right(), y)], grid);
        plot_painter.text(
            pos2(plot.left() + tokens.space.gap.inline, y - hairline),
            Align2::LEFT_BOTTOM,
            format_value(&numbers, line, step),
            text.font_id.clone(),
            text.color,
        );
        line += step;
    }
    let dimension_colors = [color.data.red, color.data.green, color.data.blue, color.data.yellow];
    paint_reference(&plot_painter, reference, plot, &dimension_colors.map(|color| color.to_egui()), &x_of, ppp);
    let mut next_color = 0;
    let curve_width = scale::Stroke::W1_5.width(ppp);
    let line_width = scale::Stroke::W1.width(ppp);
    let handle_color = color.data.yellow.to_egui();
    for (index, curve) in curves.iter().enumerate() {
        let stroke_color = curve.color.unwrap_or_else(|| {
            let picked = dimension_colors[next_color % dimension_colors.len()].to_egui();
            next_color += 1;
            picked
        });
        let points: Vec<Pos2> = curve.samples.iter().map(|&(time, value)| pos2(x_of(time), y_of(value))).collect();
        plot_painter.add(Shape::line(points, Stroke::new(curve_width, stroke_color)));
        for key in &curve.keys {
            let center = key_at(key);
            if !plot.expand(style.graph_reach).contains(center) {
                continue;
            }
            let on = selection.contains(&key.id);
            if on {
                for (side, handle) in [(HandleSide::In, key.in_handle), (HandleSide::Out, key.out_handle)] {
                    let Some(at) = handle else { continue };
                    let end = handle_at(key, side, at);
                    plot_painter.line_segment([center, end], Stroke::new(line_width, handle_color));
                    plot_painter.circle_filled(end, style.graph_handle, handle_color);
                }
            }
            let square = Rect::from_center_size(center, vec2(style.graph_key, style.graph_key));
            let fill = if on { color.accent.solid } else { color.text.normal }.to_egui();
            plot_painter.rect_filled(square, egui::CornerRadius::ZERO, fill);
            plot_painter.rect_stroke(square, egui::CornerRadius::ZERO, Stroke::new(line_width, stroke_color), StrokeKind::Outside);
        }
        let _ = index;
    }
    if let Some(marquee) = ui.data(|d| d.get_temp::<Marquee>(marquee_key))
        && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
    {
        let area = Rect::from_two_pos(marquee.origin, pos).intersect(plot);
        plot_painter.rect_filled(area, egui::CornerRadius::ZERO, color.accent.soft.to_egui());
        plot_painter.rect_stroke(area, egui::CornerRadius::ZERO, Stroke::new(hairline, color.accent.border.to_egui()), StrokeKind::Inside);
    }
    let x = x_of(playhead);
    plot_painter.line_segment([pos2(x, plot.top()), pos2(x, plot.bottom())], Stroke::new(line_width, color.accent.solid.to_egui()));
}

/// Show Reference Graph: the other graph type's curves, faint, each scaled so the curves together
/// fill the plot less [`REFERENCE_PAD`] above and below (EffectCraft `graph.rs:301-323`).
fn paint_reference(painter: &egui::Painter, curves: &[GraphCurve], plot: Rect, dimensions: &[Color32], x_of: &dyn Fn(Tick) -> f32, ppp: f32) {
    let values = || curves.iter().flat_map(|curve| curve.samples.iter().map(|sample| sample.1));
    let (low, high) = values().fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), value| (low.min(value), high.max(value)));
    if !low.is_finite() {
        return;
    }
    let span = (high - low).max(1e-9);
    let free = plot.height() * REFERENCE_PAD;
    let stroke_width = scale::Stroke::W1.width(ppp);
    for (index, curve) in curves.iter().enumerate() {
        let stroke_color = curve.color.unwrap_or(dimensions[index % dimensions.len()]);
        let points: Vec<Pos2> = curve
            .samples
            .iter()
            .map(|&(time, value)| pos2(x_of(time), plot.bottom() - ((value - low) / span) as f32 * (plot.height() - 2.0 * free) - free))
            .collect();
        painter.add(Shape::line(points, Stroke::new(stroke_width, stroke_color.gamma_multiply(REFERENCE_FADE))));
    }
}

/// The bottom bar: the graph type, display toggles, fits, separate dimensions and the keyframe
/// interpolation buttons.
fn bottom_bar(
    ui: &mut Ui,
    (id, bar): (Id, Rect),
    tokens: &UiTokens,
    labels: &GraphEditorLabels,
    range: &mut GraphRange,
    (curves, selection): (&[GraphCurve], &Selection<KeyframeId>),
    out: &mut GraphEditorOutput,
) {
    let color = &tokens.color;
    let hairline = scale::Stroke::Hairline.width(ui.pixels_per_point());
    let painter = ui.painter().with_clip_rect(bar);
    painter.rect_filled(bar, egui::CornerRadius::ZERO, color.surface.panel.to_egui());
    painter.line_segment([bar.left_top(), bar.right_top()], Stroke::new(hairline, color.line.divider.to_egui()));
    let (mut value_on, mut speed_on) = (range.mode == GraphMode::Value, range.mode == GraphMode::Speed);
    let (mut selected_on, mut zoom_on, mut separate_on) = (range.selected_only, range.auto_zoom, range.separate_dimensions);
    let mut reference_on = range.reference;
    let mut bar_ui = ui.new_child(UiBuilder::new().max_rect(bar));
    let toolbar = Toolbar::new(id.with("bar"), Orientation::Horizontal)
        .group([
            ToolbarItem::new(Icon::ChartLine, labels.value_graph.as_str()).toggle(&mut value_on),
            ToolbarItem::new(Icon::Gauge, labels.speed_graph.as_str()).toggle(&mut speed_on),
        ])
        .group([
            ToolbarItem::new(Icon::Pointer, labels.show_selected.as_str()).toggle(&mut selected_on),
            ToolbarItem::new(Icon::Scaling, labels.auto_zoom.as_str()).toggle(&mut zoom_on),
            ToolbarItem::new(Icon::Scan, labels.fit_selection.as_str()),
            ToolbarItem::new(Icon::Maximize2, labels.fit_all.as_str()),
        ])
        .group([ToolbarItem::new(Icon::Layers, labels.show_reference.as_str()).toggle(&mut reference_on)])
        .group([ToolbarItem::new(Icon::Split, labels.separate.as_str()).toggle(&mut separate_on)])
        .group([
            ToolbarItem::new(Icon::KeyHold, labels.hold.as_str()),
            ToolbarItem::new(Icon::KeyLinear, labels.linear.as_str()),
            ToolbarItem::new(Icon::KeyAutoBezier, labels.auto_bezier.as_str()),
            ToolbarItem::new(Icon::KeyframeEasyEase, labels.easy_ease.as_str()),
            ToolbarItem::new(Icon::KeyEaseIn, labels.easy_ease_in.as_str()),
            ToolbarItem::new(Icon::KeyEaseOut, labels.easy_ease_out.as_str()),
        ])
        .overflow(true)
        .show(&mut bar_ui);
    match toolbar.clicked {
        Some((0, 0)) => range.mode = GraphMode::Value,
        Some((0, 1)) => range.mode = GraphMode::Speed,
        Some((1, 0)) => range.selected_only = !range.selected_only,
        Some((1, 1)) => range.auto_zoom = !range.auto_zoom,
        Some((1, 2)) => {
            if let Some((low, high)) = fit_selection(curves, selection, range.mode) {
                (range.low, range.high, range.auto_zoom) = (low, high, false);
            }
            out.actions.push(GraphAction::FitSelection);
        }
        Some((1, 3)) => {
            range.auto_zoom = true;
            out.actions.push(GraphAction::FitAll);
        }
        Some((2, 0)) => range.reference = !range.reference,
        Some((3, 0)) => range.separate_dimensions = !range.separate_dimensions,
        Some((4, item)) => {
            out.actions.push(
                [
                    GraphAction::Hold,
                    GraphAction::Linear,
                    GraphAction::AutoBezier,
                    GraphAction::EasyEase,
                    GraphAction::EasyEaseIn,
                    GraphAction::EasyEaseOut,
                ][item],
            );
        }
        _ => {}
    }
}

/// The unique keys of `curves` in order of appearance, for the kernel's selection rules.
fn key_order(curves: &[GraphCurve]) -> Vec<RowInfo<KeyframeId>> {
    let mut seen: Vec<KeyframeId> = Vec::new();
    for key in curves.iter().flat_map(|curve| &curve.keys) {
        if !seen.contains(&key.id) {
            seen.push(key.id);
        }
    }
    seen.into_iter().map(RowInfo::leaf).collect()
}

/// Which axes a drag may move on (time, value): both, or with Shift the one the pointer moved
/// more along (EffectCraft `graph.rs:853`).
fn axes(shift: bool, dx: f32, dy: f32) -> (bool, bool) {
    if shift { (dx.abs() >= dy.abs(), dy.abs() > dx.abs()) } else { (true, true) }
}

/// A key drag under way: where the pointer went down and the keys that move.
#[derive(Clone)]
struct KeyDrag {
    press: Pos2,
    group: Vec<KeyframeId>,
}

/// A handle drag under way: where the pointer went down and the handle grabbed, with its place
/// then.
#[derive(Clone, Copy)]
struct HandleDrag {
    press: Pos2,
    key: KeyframeId,
    side: HandleSide,
    time: Tick,
    value: f64,
    broken: bool,
}

/// A box selection under way: where it began and the selection it adds to.
#[derive(Clone)]
struct Marquee {
    origin: Pos2,
    base: Selection<KeyframeId>,
}

impl Marquee {
    /// The base selection with the keys whose centres are inside `area`.
    fn extended(&self, area: Rect, curves: &[GraphCurve], center_of: &impl Fn(&GraphKey) -> Pos2) -> Selection<KeyframeId> {
        let mut next = self.base.clone();
        for key in curves.iter().flat_map(|curve| &curve.keys).filter(|key| area.contains(center_of(key))) {
            if !next.contains(&key.id) {
                next.items.push(key.id);
            }
            next.lead = Some(key.id);
            next.anchor = Some(key.id);
        }
        next
    }
}

/// The value range around the selected keys (and their handles), in Fit Selection to View: a
/// lone key also frames its neighbours, and the speed graph frames the curve between the first
/// and last selected key (EffectCraft `graph.rs:740`).
fn fit_selection(curves: &[GraphCurve], selection: &Selection<KeyframeId>, mode: GraphMode) -> Option<(f64, f64)> {
    let mut bounds: Option<(f64, f64)> = None;
    let mut include = |value: f64| {
        bounds = Some(bounds.map_or((value, value), |(low, high)| (low.min(value), high.max(value))));
    };
    for curve in curves {
        let chosen: Vec<usize> = (0..curve.keys.len()).filter(|&at| selection.contains(&curve.keys[at].id)).collect();
        for &at in &chosen {
            let key = &curve.keys[at];
            include(key.value);
            for (_, value) in key.in_handle.iter().chain(key.out_handle.iter()) {
                include(*value);
            }
        }
        let (Some(&first), Some(&last)) = (chosen.first(), chosen.last()) else { continue };
        let (from, to) = if first == last { (first.saturating_sub(1), (last + 1).min(curve.keys.len() - 1)) } else { (first, last) };
        if first == last {
            include(curve.keys[from].value);
            include(curve.keys[to].value);
        }
        if mode == GraphMode::Speed {
            let (start, end) = (curve.keys[from].time, curve.keys[to].time);
            for &(time, value) in curve.samples.iter().filter(|(time, _)| *time >= start && *time <= end) {
                let _ = time;
                include(value);
            }
        }
    }
    let (low, high) = bounds?;
    let margin = ((high - low) * FIT_MARGIN).max(FIT_MARGIN_MIN);
    Some((low - margin, high + margin))
}

/// A grid line's label: as many decimals as the step needs, no negative zero.
// effectcraft: crates/ui-egui/src/panels/graph.rs:826 `fmt`.
fn format_value(numbers: &LocaleNumbers, value: f64, step: f64) -> String {
    let decimals = if step >= 1.0 { 0 } else { (-step.log10()).ceil() as usize };
    // `LocaleNumbers::format` drops negative zero; the minimum keeps the step's digits.
    numbers.format(value, decimals..=decimals)
}
/// What the graph editor did this pass.
#[must_use]
pub struct GraphEditorOutput {
    /// The plot's response, for hover and focus.
    pub response: Response,
    /// The selection change a click or box selection made.
    pub selection: Option<SelectionChange<KeyframeId>>,
    /// The key move gesture, one update per frame and a commit on release.
    pub keys_moved: Option<EditEvent<GraphKeyMove>>,
    /// The handle move gesture, one update per frame and a commit on release.
    pub handle_moved: Option<EditEvent<HandleMove>>,
    /// This pass's actions, in order.
    pub actions: Vec<GraphAction>,
}

/// A grid step near `raw`: 1, 2, 5 or 10 times the power of ten below `|raw|` (cut-offs 1.5,
/// 3.5, 7.5); `|raw|` at least 1e-9.
// effectcraft: crates/ui-egui/src/panels/graph.rs:805 `nice_step`.
pub fn nice_step(raw: f64) -> f64 {
    let raw = raw.abs().max(1e-9);
    let magnitude = 10f64.powf(raw.log10().floor());
    let scaled = raw / magnitude;
    magnitude
        * if scaled < 1.5 {
            1.0
        } else if scaled < 3.5 {
            2.0
        } else if scaled < 7.5 {
            5.0
        } else {
            10.0
        }
}

/// The value range that shows `curves` (samples, keys and handles) with an 8 % margin each side:
/// 0…100 without values, at least 0 at the bottom for `mode` Speed, a flat range one unit high.
// effectcraft: crates/ui-egui/src/panels/graph.rs:328 `auto_range`.
pub fn auto_range(curves: &[GraphCurve], mode: GraphMode) -> (f64, f64) {
    let mut values = curves.iter().flat_map(|curve| {
        let keys = curve.keys.iter().flat_map(|key| [Some(key.value), key.in_handle.map(|handle| handle.1), key.out_handle.map(|handle| handle.1)]);
        curve.samples.iter().map(|sample| sample.1).chain(keys.flatten())
    });
    let Some(first) = values.next() else { return (0.0, 100.0) };
    let (mut low, mut high) = values.fold((first, first), |(low, high), value| (low.min(value), high.max(value)));
    if mode == GraphMode::Speed {
        low = low.min(0.0);
    }
    if (high - low).abs() < 1e-9 {
        high = low + 1.0;
    }
    let margin = (high - low) * AUTO_MARGIN;
    (low - margin, high + margin)
}
