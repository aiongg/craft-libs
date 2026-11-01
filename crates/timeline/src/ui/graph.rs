//! The graph editor: keyframed values (or their speed) as curves over time (N1.08).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::{Selection, SelectionChange};
use craft_ui::edit::EditEvent;
use egui::{AsIdSalt, Color32, IdSalt, Rect, Response, Ui};

use super::keyframes::{KeyframeId, KeyframeInterpolation};
use crate::time::{Tick, TimeView};

craft_ui::words! {
    /// The bottom bar's tooltips, English by default (EffectCraft `graph.rs:611`).
    #[non_exhaustive]
    pub struct GraphEditorLabels {
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
    Value,
    Speed,
}

/// A key on a curve, with its Bezier handles in graph units (time, value) when it has them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphKey {
    pub id: KeyframeId,
    pub time: Tick,
    pub value: f64,
    pub interpolation: KeyframeInterpolation,
    pub in_handle: Option<(Tick, f64)>,
    pub out_handle: Option<(Tick, f64)>,
}

/// One curve: a property's dimension, sampled by the app (the app evaluates its own
/// interpolation and expressions).
#[derive(Clone, Debug, PartialEq)]
pub struct GraphCurve {
    /// The property and dimension, the app's.
    pub id: u64,
    pub name: String,
    /// `None`: the next colour of the per-dimension set (red, green, blue, yellow; EffectCraft
    /// `graph.rs:22`).
    pub color: Option<Color32>,
    /// `(time, value)` in time order, about one per two points of width.
    pub samples: Vec<(Tick, f64)>,
    pub keys: Vec<GraphKey>,
}

/// The graph's value range and options; the app keeps it per panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphRange {
    pub low: f64,
    pub high: f64,
    pub mode: GraphMode,
    /// The range follows the shown curves (After Effects' Auto-zoom graph height).
    pub auto_zoom: bool,
    pub selected_only: bool,
    pub separate_dimensions: bool,
}

impl Default for GraphRange {
    fn default() -> Self {
        Self { low: 0.0, high: 100.0, mode: GraphMode::Value, auto_zoom: true, selected_only: false, separate_dimensions: false }
    }
}

/// A move of the selected keys in time and value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GraphKeyMove {
    pub dt: Tick,
    pub dv: f64,
}

/// Which handle of a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandleSide {
    In,
    Out,
}

/// A handle dragged to `(time, value)`; with Alt the two handles break apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandleMove {
    pub key: KeyframeId,
    pub side: HandleSide,
    pub time: Tick,
    pub value: f64,
    pub broken: bool,
}

/// A bottom-bar button, or a key on the plot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphAction {
    /// Delete with keys selected.
    Delete,
    FitSelection,
    FitAll,
    Hold,
    Linear,
    AutoBezier,
    EasyEase,
    EasyEaseIn,
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
/// - Wheel zooms the value range about the pointer (turns auto-zoom off); auto-zoom fits the
///   shown curves each frame ([`auto_range`]).
/// - The bottom bar, a horizontal [`Toolbar`](craft_ui::patterns::Toolbar): Value / Speed graph,
///   selected only, auto-zoom, fit selection, fit all, separate dimensions, Hold, Linear, Auto
///   Bezier, Easy Ease, Ease In, Ease Out; buttons with the tooltips of [`GraphEditorLabels`].
/// - Keyboard: the plot is one Tab stop; arrows move the selected keys (a frame / a value step),
///   F9, Shift+F9 and Ctrl+Shift+F9 report the Easy Ease actions; Delete is claimed through
///   [`KeyClaim`](craft_ui::behavior::KeyClaim) and reported as [`GraphAction::Delete`].
///   AccessKit: the plot is a `Group` named by [`GraphEditorLabels`]' title with the
///   selected keys' count as its value.
pub struct GraphEditor<'a> {
    id_salt: IdSalt,
    curves: &'a [GraphCurve],
    selection: &'a Selection<KeyframeId>,
    view: &'a TimeView,
    range: &'a mut GraphRange,
    playhead: Tick,
    labels: Option<GraphEditorLabels>,
}

impl<'a> GraphEditor<'a> {
    pub fn new(
        id_salt: impl AsIdSalt,
        curves: &'a [GraphCurve],
        selection: &'a Selection<KeyframeId>,
        view: &'a TimeView,
        range: &'a mut GraphRange,
    ) -> Self {
        Self { id_salt: IdSalt::new(id_salt), curves, selection, view, range, playhead: Tick::ZERO, labels: None }
    }

    pub fn playhead(mut self, t: Tick) -> Self {
        self.playhead = t;
        self
    }

    pub fn labels(mut self, labels: GraphEditorLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // effectcraft: crates/ui-egui/src/panels/graph.rs:407 `show`, `:356` `box_select`, `:379`
    // `wheel_zoom`, `:611` `button_bar`, `:740` `fit_selection`, `:842` `HandleGrab`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> GraphEditorOutput {
        todo!()
    }
}

/// What the graph editor did this pass.
#[must_use]
pub struct GraphEditorOutput {
    pub response: Response,
    pub selection: Option<SelectionChange<KeyframeId>>,
    pub keys_moved: Option<EditEvent<GraphKeyMove>>,
    pub handle_moved: Option<EditEvent<HandleMove>>,
    pub action: Option<GraphAction>,
}

/// A grid step near `raw`: 1, 2, 5 or 10 times the power of ten below `|raw|` (cut-offs 1.5,
/// 3.5, 7.5); `|raw|` at least 1e-9.
// effectcraft: crates/ui-egui/src/panels/graph.rs:805 `nice_step`.
pub fn nice_step(raw: f64) -> f64 {
    todo!()
}

/// The value range that shows `curves` (samples, keys and handles) with an 8 % margin each side:
/// 0…100 without values, at least 0 at the bottom for `mode` Speed, a flat range one unit high.
// effectcraft: crates/ui-egui/src/panels/graph.rs:328 `auto_range`.
pub fn auto_range(curves: &[GraphCurve], mode: GraphMode) -> (f64, f64) {
    todo!()
}
