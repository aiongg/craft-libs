//! [`GradientEditor`]: a gradient's stop bar, edited by pointer and keys through the edit
//! lifecycle.

use std::any::Any;
use std::marker::PhantomData;
use std::ops::RangeInclusive;
use std::sync::Arc;

use craft_theme::{Rgba, Srgba};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{
    AsIdSalt, Color32, Context, CornerRadius, CursorIcon, EventFilter, Id, IdSalt, Key, Modifiers, Pos2, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::ramp::{GradientRamp, GradientRampStyle};
use super::stops::{self, Sampler, StopRules, default_sampler};
use super::value::{GradientStops, StopRow, StopRows};
use crate::behavior::{ClaimedKey, KeyClaim};
use crate::composites::curve_editor::{fill, take_exact};
use crate::controls::slider::PRESS_WAIT;
use crate::edit::{self, EditEvent, EditPolicy, EditResponse, Gesture, parts};
use crate::localized::{self, LocaleNumbers};
use crate::primitives::{FocusRing, Gap, Stack};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

mod fields;

/// A marker of the stop bar: a colour stop, an opacity stop, or the midpoint after stop `i` of a
/// row. Indices are into the stops as the bar shows them ([`GradientStops::opacity_stops`] for
/// the opacity row).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StopRef {
    Color(usize),
    Opacity(usize),
    Midpoint(StopRow, usize),
}

impl StopRef {
    /// The row the marker belongs to.
    pub fn row(self) -> StopRow {
        match self {
            StopRef::Color(_) => StopRow::Color,
            StopRef::Opacity(_) => StopRow::Opacity,
            StopRef::Midpoint(row, _) => row,
        }
    }

    /// Stop `i` of `row`.
    fn stop(row: StopRow, i: usize) -> Self {
        match row {
            StopRow::Color => StopRef::Color(i),
            StopRow::Opacity => StopRef::Opacity(i),
        }
    }

    /// The stop's index, or for a midpoint the index of the stop before it.
    fn index(self) -> usize {
        match self {
            StopRef::Color(i) | StopRef::Opacity(i) | StopRef::Midpoint(_, i) => i,
        }
    }

    fn is_stop(self) -> bool {
        !matches!(self, StopRef::Midpoint(..))
    }
}

/// Where a payload dropped on the bar landed ([`GradientEditor::drop_target`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StopDrop {
    /// On colour stop `i` (DesignCraft and VectorCraft recolour it).
    OnStop(usize),
    /// At a location of the bar, 0..=1, away from the colour stops (they add a stop there).
    At(f32),
}

/// The editor's words; translate them in the app. Templates take `{i}`, `{n}`, `{m}`,
/// `{location}`, `{hex}` and `{opacity}`.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientLabels {
    /// A colour stop's accessible name: "Color stop {i} of {n}".
    pub color_stop: String,
    /// A colour stop's accessible value: "{location}, {hex}".
    pub color_value: String,
    /// A colour stop's accessible value under [`StopRows::Combined`]: "{location}, {hex}, opacity
    /// {opacity}".
    pub color_opacity_value: String,
    /// An opacity stop's accessible name: "Opacity stop {i} of {n}".
    pub opacity_stop: String,
    /// An opacity stop's accessible value: "{location}, opacity {opacity}".
    pub opacity_value: String,
    /// A colour midpoint's accessible name: "Midpoint {i}".
    pub midpoint: String,
    /// An opacity midpoint's accessible name: "Opacity midpoint {i}".
    pub opacity_midpoint: String,
    /// The bar's accessible value: "{n} color stops, {m} opacity stops".
    pub bar_value: String,
    /// The bar's accessible value under [`StopRows::Combined`]: "{n} stops".
    pub bar_stops: String,
    /// A value in percent: "{n}%".
    pub percent: String,
    /// The fields row's labels, which name their fields: "Color", "Opacity", "Location",
    /// "Midpoint".
    pub color: String,
    pub opacity: String,
    pub location: String,
    pub midpoint_field: String,
    /// The Delete button's accessible name and tooltip: "Delete stop".
    pub delete: String,
}

impl Default for GradientLabels {
    fn default() -> Self {
        Self {
            color_stop: "Color stop {i} of {n}".into(),
            color_value: "{location}, {hex}".into(),
            color_opacity_value: "{location}, {hex}, opacity {opacity}".into(),
            opacity_stop: "Opacity stop {i} of {n}".into(),
            opacity_value: "{location}, opacity {opacity}".into(),
            midpoint: "Midpoint {i}".into(),
            opacity_midpoint: "Opacity midpoint {i}".into(),
            bar_value: "{n} color stops, {m} opacity stops".into(),
            bar_stops: "{n} stops".into(),
            percent: "{n}%".into(),
            color: "Color".into(),
            opacity: "Opacity".into(),
            location: "Location".into(),
            midpoint_field: "Midpoint".into(),
            delete: "Delete stop".into(),
        }
    }
}

/// A gradient editor's look, from [`GradientEditor::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GradientEditorStyle {
    /// Height of the ramp and its inset at both ends of the bar: `gradient_editor.ramp_height`,
    /// `gradient_editor.inset`.
    pub ramp_height: f32,
    pub inset: f32,
    /// Side of a stop marker's square and height of its pointer: `gradient_editor.marker`,
    /// `gradient_editor.pointer`.
    pub marker: f32,
    pub pointer: f32,
    /// Radius of a midpoint diamond: `gradient_editor.diamond`.
    pub diamond: f32,
    /// How far from a marker and a diamond a press takes it: `gradient_editor.hit`,
    /// `gradient_editor.diamond_hit`.
    pub hit: f32,
    pub diamond_hit: f32,
    /// How far above or below the ramp a dragged stop is removed: `gradient_editor.tear_off`.
    pub tear_off: f32,
    /// Width of the line that marks a drop place: `gradient_editor.drop_line`.
    pub drop_line: f32,
    /// The ramp's checkerboard and its one-point edge (`gradient_editor.edge`).
    pub ramp: GradientRampStyle,
    /// Marker outlines, pointers and diamonds: `gradient_editor.marker_edge`; hovered:
    /// `text.normal`; selected, with the selected diamond's fill: `gradient_editor.selected`.
    pub marker_edge: Color32,
    pub hovered: Color32,
    pub selected: Color32,
    /// The drop place line: `accent.solid`.
    pub drop: Color32,
}

/// What the editor keeps between frames, per editor.
#[derive(Clone, Debug)]
struct State {
    selected: Option<StopRef>,
    drag: Option<Drag>,
    /// Pointer input that waits while another control's gesture with the focus ends.
    held: Option<Held>,
    parts: parts::State<()>,
    /// The selection as the editor's gesture began, restored when it is cancelled.
    start_selected: Option<StopRef>,
    /// The anchor of the colour popover while it is open ([`GradientEditor::color_popover`]).
    popover: Option<Id>,
}

impl Default for State {
    fn default() -> Self {
        Self { selected: Some(StopRef::Color(0)), drag: None, held: None, parts: parts::State::default(), start_selected: None, popover: None }
    }
}

/// The marker a press captured.
#[derive(Clone, Debug)]
struct Drag {
    marker: StopRef,
    /// The stops at the press: every frame of the drag edits them afresh, so a stop torn off
    /// comes back as it was.
    base: GradientStops,
    /// Where the press was and the marker's location then, so the marker doesn't jump.
    press: Pos2,
    grab: f32,
    /// Alt was held at the press: the drag moves a copy of the stop.
    copy: bool,
    /// The selection before the press, restored when the gesture is cancelled.
    before: Option<StopRef>,
}

/// Pointer input the bar runs once no other control's gesture has the focus.
#[derive(Clone, Copy, Debug)]
enum Input {
    Press { pos: Pos2, alt: bool },
    Secondary(Pos2),
    Click(Pos2),
}

/// Inputs held since pass `since`; `last` is the last pass that held them.
#[derive(Clone, Debug)]
struct Held {
    since: u64,
    last: u64,
    inputs: Vec<Input>,
}

const STATE: &str = "craft_ui::gradient_editor";
const EDIT: &str = "craft_ui::gradient_editor::edit";

/// The keys a focused bar keeps from the app's shortcuts. Only `+` is matched logically: many
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

/// Stop locations as markers show them, closer than this, have no midpoint diamond between them.
const MIN_GAP: f32 = 0.001;

type Edits = parts::Edits<GradientStops>;

/// What [`GradientEditor::show`] returns: the edit of the stops, whose response is the bar's, a
/// colour stop whose colour the host is asked to edit (a double-click on it, Enter or Space while
/// it is selected, or a click on the Color well; never with
/// [`color_popover`](GradientEditor::color_popover)), and a payload dropped on the bar
/// ([`GradientEditor::drop_target`]).
pub struct GradientEditorResponse<T = ()> {
    pub edit: EditResponse<GradientStops>,
    pub color_requested: Option<usize>,
    pub dropped: Option<(StopDrop, Arc<T>)>,
}

/// Edits a gradient's stops through the edit lifecycle: `GradientEditor::new("fill", "Gradient",
/// &stops, &numbers).show(ui)` returns a [`GradientEditorResponse`].
///
/// The bar fills the available width. [`StopRows::Split`] rows, top to bottom: opacity markers
/// (their midpoints just above the ramp), the ramp, the colour midpoints, colour markers;
/// [`StopRows::Combined`]: the midpoints, the ramp, colour markers. The ramp is drawn without the
/// host's Reverse, and the bar keeps its geometry right to left
/// ([`Unmirrored::ColourRamp`](crate::direction::Unmirrored::ColourRamp)).
///
/// Pointer: a press on a marker or diamond selects and captures it without a jump; a drag moves
/// it (stops pass their neighbours and the selection follows) and is one gesture. A stop dragged
/// more than `gradient_editor.tear_off` above or below the ramp is removed while its row is above
/// the minimum, and comes back on re-entry within the drag. Alt held at the press drags a copy,
/// which can't be torn off. A click on empty space adds a stop there (under `Split`, above the
/// ramp's centre an opacity stop) as one Set; a press and drag on empty space does nothing. A
/// double-click on a colour stop asks the host to edit its colour; a right-click on a marker
/// selects it, and the host may attach a context menu to the response. A press gives the bar
/// keyboard focus; while another control's gesture has it (a field's typing run), the press waits
/// until that gesture's ending event has gone out, and edits the stops it leaves.
///
/// Keys while the bar has focus (one Tab stop): `+` (or `=`) and `-` select the next and
/// previous marker of the selected row in location order, midpoints between their stops (the
/// first and last with nothing selected; they stop at the ends); under `Split`, Up and Down select
/// the opacity or colour stop nearest the selected location; Left and Right move the selected stop
/// or midpoint by 1 %, 10 % with Shift; Delete and Backspace remove the selected stop and select
/// the one that takes its index; Insert, and Cmd/Ctrl+Enter, add a stop halfway in the selected
/// row ([`stops::halfway`]) and select it; Enter and Space on a colour stop ask for its colour; Esc
/// cancels a drag and is otherwise left to the host. Every key but `+` needs exactly its
/// modifiers. The bar claims Delete, Backspace, `+`, `=`, `-`, Insert and Cmd/Ctrl+Enter from the
/// app's shortcuts ([`KeyClaim`]).
///
/// Events: a press-drag-release is Begin, Updates and Commit (unchanged for a press without
/// movement); clicks that add, keys that change the stops are Sets. Inside [`edit::scope`] only
/// Updates go out. The selection is view state: [`selected`](GradientEditor::selected) and
/// [`select`](GradientEditor::select), the first colour stop until set.
///
/// Fields ([`fields`](GradientEditor::fields), on by default): under the bar, one
/// [`PropertyRow`](crate::composites::PropertyRow) per field of the selected marker, as
/// PhotoCraft's: a colour stop's "Color" well (a [`Swatch`](crate::controls::Swatch); its click
/// asks for the colour, or opens the popover) and, under [`StopRows::Combined`], its "Opacity";
/// an opacity stop's "Opacity"; a stop's "Location" and a Delete button, disabled at the row's
/// minimum; a midpoint's "Midpoint" in the host's range. The number fields are percent, as wide
/// as the widest value, at the start of the control column; their labels scrub them. Enter or
/// Space on an opacity stop or a midpoint focuses its first field. Every field is a part of the
/// editor's edit: a scrub or a typing run is one gesture of the stops, a Location run that passes
/// a neighbour keeps the selection on its stop, and a press on the bar or the Delete button during
/// a typing run waits for the run's Commit. With [`color_popover`](GradientEditor::color_popover)
/// the popover's gestures are the editor's, mapped to the stop.
///
/// AccessKit: the bar is a focusable `Group` named by the label, valued "{n} color stops, {m}
/// opacity stops", whose active descendant is the selected marker; each marker is a
/// `GraphicsObject` child ("Color stop {i} of {n}", valued "{location}, {hex}"), selected when it
/// is. The fields are named by their labels; the Color well is a `Button` valued by its hex
/// colour, and the Delete button is "Delete stop".
#[must_use = "show the editor with `show`"]
pub struct GradientEditor<'a, T = ()> {
    id_salt: IdSalt,
    label: String,
    stops: &'a GradientStops,
    numbers: &'a LocaleNumbers,
    rules: StopRules,
    midpoints: bool,
    sampler: Option<&'a Sampler>,
    location_decimals: usize,
    policy: EditPolicy,
    labels: GradientLabels,
    enabled: bool,
    fields: bool,
    color_popover: bool,
    drop_target: bool,
    payload: PhantomData<fn() -> T>,
}

impl<'a> GradientEditor<'a> {
    /// An editor named `label` ("Gradient") of the host's `stops`; `id_salt` keeps its selection
    /// and gesture, and `numbers` writes the accessible values.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, stops: &'a GradientStops, numbers: &'a LocaleNumbers) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            stops,
            numbers,
            rules: StopRules::default(),
            midpoints: true,
            sampler: None,
            location_decimals: 0,
            policy: EditPolicy::default(),
            labels: GradientLabels::default(),
            enabled: true,
            fields: true,
            color_popover: false,
            drop_target: false,
            payload: PhantomData,
        }
    }

    /// The bar's widget id for `id_salt` in `ui`, as [`selected`](GradientEditor::selected) and
    /// [`select`](GradientEditor::select) take it; also the response's id.
    pub fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// The selected marker of editor `id`.
    pub fn selected(ctx: &Context, id: Id) -> Option<StopRef> {
        ctx.data(|d| d.get_temp::<State>(id.with(STATE))).unwrap_or_default().selected
    }

    /// Selects `marker` of editor `id`.
    pub fn select(ctx: &Context, id: Id, marker: Option<StopRef>) {
        ctx.data_mut(|d| d.get_temp_mut_or_default::<State>(id.with(STATE)).selected = marker);
    }

    pub fn style(tokens: &UiTokens) -> GradientEditorStyle {
        let g = &tokens.component.gradient_editor;
        GradientEditorStyle {
            ramp_height: g.ramp_height,
            inset: g.inset,
            marker: g.marker,
            pointer: g.pointer,
            diamond: g.diamond,
            hit: g.hit,
            diamond_hit: g.diamond_hit,
            tear_off: g.tear_off,
            drop_line: g.drop_line,
            ramp: GradientRamp::style(tokens),
            marker_edge: g.marker_edge.to_egui(),
            hovered: tokens.color.text.normal.to_egui(),
            selected: g.selected.to_egui(),
            drop: tokens.color.accent.solid.to_egui(),
        }
    }
}

impl<'a, T: Any + Send + Sync> GradientEditor<'a, T> {
    /// The row layout; [`StopRows::Split`] by default.
    pub fn rows(mut self, rows: StopRows) -> Self {
        self.rules.rows = rows;
        self
    }

    /// The fewest and most stops a row keeps; 2 and 1024 by default (PhotoCraft).
    pub fn stops(mut self, min: usize, max: usize) -> Self {
        self.rules.min = min;
        self.rules.max = max.max(min);
        self
    }

    /// The range a midpoint may take; 0.05..=0.95 by default (PhotoCraft).
    pub fn midpoint_range(mut self, range: RangeInclusive<f32>) -> Self {
        self.rules.midpoint = range;
        self
    }

    /// Whether the midpoint diamonds show and edit; on by default.
    pub fn midpoints(mut self, midpoints: bool) -> Self {
        self.midpoints = midpoints;
        self
    }

    /// Samples the gradient for the ramp and new stops instead of [`default_sampler`].
    pub fn sampler(mut self, sampler: &'a Sampler) -> Self {
        self.sampler = Some(sampler);
        self
    }

    /// Decimals of a location in percent; 0 by default.
    pub fn location_decimals(mut self, decimals: usize) -> Self {
        self.location_decimals = decimals;
        self
    }

    /// How the editor reports gestures; [`EditPolicy::default`] by default.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The editor's words in the interface language.
    pub fn labels(mut self, labels: GradientLabels) -> Self {
        self.labels = labels;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether the selected marker's fields show under the bar; on by default. DesignCraft shows
    /// the location in its panel header instead.
    pub fn fields(mut self, fields: bool) -> Self {
        self.fields = fields;
        self
    }

    /// Whether the editor edits a colour stop's colour in a [`ColorPopover`](crate::composites::ColorPopover)
    /// itself, anchored on the Color well (on the bar without the fields), as VectorCraft does;
    /// off by default, when a request goes to the host in
    /// [`GradientEditorResponse::color_requested`] (PhotoCraft opens its Color Picker dialog).
    pub fn color_popover(mut self, popover: bool) -> Self {
        self.color_popover = popover;
        self
    }

    /// Takes drops of `P` payloads (egui's drag and drop): while one hovers, a line marks the
    /// place; the release is reported in [`GradientEditorResponse::dropped`] and the host
    /// applies it.
    pub fn drop_target<P: Any + Send + Sync>(self) -> GradientEditor<'a, P> {
        let Self {
            id_salt, label, stops, numbers, rules, midpoints, sampler, location_decimals, policy, labels, enabled, fields, color_popover, ..
        } = self;
        GradientEditor {
            id_salt,
            label,
            stops,
            numbers,
            rules,
            midpoints,
            sampler,
            location_decimals,
            policy,
            labels,
            enabled,
            fields,
            color_popover,
            drop_target: true,
            payload: PhantomData,
        }
    }

    /// Shows the editor and returns this frame's edit.
    pub fn show(self, ui: &mut Ui) -> GradientEditorResponse<T> {
        let style = GradientEditor::style(&Tokens::of(ui));
        let id = GradientEditor::id_in(ui, self.id_salt);
        Stack::new(Gap::Stack).show(ui, |ui| self.show_inside(ui, id, &style)).inner
    }

    fn show_inside(&self, ui: &mut Ui, id: Id, style: &GradientEditorStyle) -> GradientEditorResponse<T> {
        let enabled = self.enabled && ui.is_enabled();
        let mut state = ui.data(|d| d.get_temp::<State>(id.with(STATE))).unwrap_or_default();
        // A typing run had the focus as the pass began: the Delete button's click waits for it.
        let waits = edit::focused_gesture(ui.ctx()).is_some();
        // Esc is the parts': each cancels its own gesture, which cancels this one.
        let outer = Gesture::load(ui, id.with(EDIT), self.stops, self.policy.cancellable(false)).enabled(enabled);
        let active = outer.is_active();
        let before = state.selected;
        let shown = outer.shown().clone();
        let mut edits = Edits::new(&state.parts);
        let bar = self.bar(ui, id, style, &shown, &mut state, enabled);
        edits.push(&bar.edit, |stops| stops);
        // The one place a colour request is made: the popover takes it, or the host does.
        let mut requested = bar.color_requested;
        if self.fields {
            requested = self.fields_row(ui, id, &bar.value, &mut state, &mut edits, enabled, requested);
        } else if self.color_popover {
            self.popover(ui, bar.edit.response.clone(), &bar.value, &mut state, &mut edits, enabled, requested);
        }
        for () in edits.clicks(ui.ctx(), &mut state.parts, waits) {
            if let Some(m) = state.selected.filter(|m| m.is_stop())
                && let Some((stops, j)) = stops::remove(&bar.value, m.row(), m.index(), &self.rules)
            {
                state.selected = Some(StopRef::stop(m.row(), j));
                edits.set(stops);
            }
        }
        let outer = edits.feed(outer, &mut state.parts, edit::in_scope(ui));
        if !active && outer.is_active() {
            state.start_selected = before;
        }
        let event = outer.finish();
        if matches!(event, Some(EditEvent::Cancel { .. })) {
            state.selected = state.start_selected;
        }
        ui.data_mut(|d| d.insert_temp(id.with(STATE), state));
        let color_requested = requested.filter(|_| !self.color_popover);
        GradientEditorResponse { edit: EditResponse::new(bar.edit.response, event), color_requested, dropped: bar.dropped }
    }

    /// The bar: its interaction, look and AccessKit nodes, as a part of the editor's edit.
    fn bar(&self, ui: &mut Ui, id: Id, style: &GradientEditorStyle, host: &GradientStops, state: &mut State, enabled: bool) -> Bar<T> {
        let rows = self.rules.rows;
        let top = match rows {
            StopRows::Split => style.pointer + style.marker + 1.0,
            StopRows::Combined if self.midpoints => 3.0 * style.diamond + 1.0,
            StopRows::Combined => 1.0,
        };
        let bottom = style.pointer + style.marker + 1.0;
        let width = ui.available_width();
        let (outer, _) = ui.allocate_exact_size(vec2(width, top + style.ramp_height + bottom), Sense::hover());
        let ramp =
            Rect::from_min_size(pos2(outer.left() + style.inset, outer.top() + top), vec2((width - 2.0 * style.inset).max(1.0), style.ramp_height))
                .round_to_pixels(ui.pixels_per_point());
        let geo = Geometry { ramp, rows, midpoints: self.midpoints, style: *style };

        let mut builder = UiBuilder::new()
            .scope_id(id)
            .max_rect(outer)
            .sense(Sense::click_and_drag())
            .accessibility_role(Role::Group)
            .accessibility_label(&self.label);
        if !enabled {
            builder = builder.disabled();
        }
        let mut bar_ui = ui.new_child(builder);
        bar_ui.expand_to_include_rect(outer);
        let response = bar_ui.response();
        let enabled = response.enabled();

        let filter = EventFilter { horizontal_arrows: true, vertical_arrows: rows == StopRows::Split, tab: false, escape: false };
        let policy = EditPolicy::default().cancellable(self.policy.cancellable).cost(self.policy.cost);
        let mut gesture = Gesture::load(ui, id, host, policy).event_filter(filter).enabled(enabled);
        if !gesture.is_active() {
            // Cancelled (Esc, an undo, an external change): the selection goes back too.
            if let Some(drag) = state.drag.take() {
                state.selected = drag.before;
            }
        }
        let waits = edit::focused_gesture(ui.ctx()).is_some_and(|g| gesture.id() != Some(g));
        state.selected = state.selected.filter(|&s| geo.valid(gesture.shown(), s));

        let default = default_sampler(rows);
        let sampler: &Sampler = self.sampler.unwrap_or(&default);
        let mut color_requested = None;
        let mut dropped = None;
        let mut drop_at = None;
        if enabled {
            let mut edit = BarEdit { gesture: &mut gesture, state, geo, rules: &self.rules, sampler };
            edit.pointer(ui, &response, waits);
            if response.double_clicked()
                && let Some(pos) = response.interact_pointer_pos()
                && let Some(StopRef::Color(i)) = geo.hit(edit.gesture.shown(), pos, edit.state.selected)
            {
                edit.state.selected = Some(StopRef::Color(i));
                color_requested = Some(i);
            }
            if response.has_focus() {
                KeyClaim::claim(ui.ctx(), id, &CLAIMED);
                match edit.keys(ui) {
                    Some(StopRef::Color(i)) => color_requested = Some(i),
                    // The fields row draws the marker's first field next.
                    Some(_) if self.fields => localized::focus_first_field(ui.ctx(), true),
                    _ => {}
                }
            }
            if self.drop_target {
                if response.dnd_hover_payload::<T>().is_some() {
                    drop_at = ui.input(|i| i.pointer.hover_pos()).map(|pos| geo.drop_place(edit.gesture.shown(), pos));
                }
                if let Some(payload) = response.dnd_release_payload::<T>()
                    && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
                {
                    dropped = Some((geo.drop_place(edit.gesture.shown(), pos), payload));
                }
            }
        }

        let shown = gesture.shown().clone();
        let event = gesture.finish();
        let selected = state.selected;
        let dragging = state.drag.is_some();
        let hovered = match (enabled && !dragging && response.contains_pointer(), ui.input(|i| i.pointer.hover_pos())) {
            (true, Some(pos)) => geo.hit(&shown, pos, selected),
            _ => None,
        };
        if hovered.is_some() || dragging {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        if ui.is_rect_visible(outer) {
            let look = Look { selected, hovered, enabled };
            self.paint(&bar_ui, &geo, &shown, sampler, &look, drop_at);
        }
        let nodes = self.nodes(&mut bar_ui, &geo, &shown, selected);
        drop(bar_ui);
        FocusRing::paint_radius(ui, outer, CornerRadius::ZERO, &response);

        let count = |n: usize| self.numbers.format(n as f64, 0..=0);
        let value = match rows {
            StopRows::Split => fill(&self.labels.bar_value, &[("n", &count(shown.color.len())), ("m", &count(shown.opacity_stops().len()))]),
            StopRows::Combined => fill(&self.labels.bar_stops, &[("n", &count(shown.color.len()))]),
        };
        let active = selected.and_then(|s| nodes.iter().find(|(m, _)| *m == s)).map(|(_, id)| *id);
        // A `Ui`'s node gets no bounds of its own from egui.
        let bounds = egui::accesskit::Rect { x0: outer.min.x.into(), y0: outer.min.y.into(), x1: outer.max.x.into(), y1: outer.max.y.into() };
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_bounds(bounds);
            node.set_value(value);
            match active {
                Some(marker) => node.set_active_descendant(marker.accesskit_id()),
                None => node.clear_active_descendant(),
            }
        });
        Bar { edit: EditResponse::new(response, event), value: shown, color_requested, dropped }
    }

    /// The ramp, its edge, the midpoints and markers, and the drop place.
    fn paint(&self, ui: &Ui, geo: &Geometry, stops: &GradientStops, sampler: &Sampler, look: &Look, drop_at: Option<StopDrop>) {
        let style = &geo.style;
        let painter = ui.painter();
        let fade = |c: Color32| if look.enabled { c } else { c.gamma_multiply(0.5) };
        GradientRamp::paint(painter, geo.ramp, stops, sampler, &Tokens::of(ui));
        painter.rect_stroke(geo.ramp, 0.0, Stroke::new(1.0, fade(style.ramp.edge)), StrokeKind::Outside);
        let edge = |m: StopRef| {
            fade(match m {
                m if look.selected == Some(m) => style.selected,
                m if look.hovered == Some(m) => style.hovered,
                _ => style.marker_edge,
            })
        };
        let mut shapes = Vec::new();
        for row in geo.rows() {
            for (i, at) in geo.diamonds(stops, row) {
                let m = StopRef::Midpoint(row, i);
                let fill = (look.selected == Some(m)).then(|| fade(style.selected));
                shapes.push((m, vec![diamond(geo.diamond_center(row, at), style.diamond, fill, edge(m))]));
            }
            for (i, at) in locations(stops, row).into_iter().enumerate() {
                let m = StopRef::stop(row, i);
                let content = match row {
                    StopRow::Color => Color32::from(Srgba { a: 1.0, ..stops.color[i].color }),
                    StopRow::Opacity => Color32::from_gray((stops.opacity_stops()[i].opacity.clamp(0.0, 1.0) * 255.0).round() as u8),
                };
                shapes.push((m, geo.marker(row, at, content, edge(m))));
            }
        }
        // The selected marker on top of the others.
        shapes.sort_by_key(|(m, _)| look.selected == Some(*m));
        painter.extend(shapes.into_iter().flat_map(|(_, s)| s));
        if let Some(place) = drop_at {
            let x = match place {
                StopDrop::OnStop(i) => geo.x(stops.color[i].location),
                StopDrop::At(at) => geo.x(at),
            };
            painter.line_segment([pos2(x, geo.ramp.top()), pos2(x, geo.ramp.bottom())], Stroke::new(style.drop_line, style.drop));
        }
    }

    /// The markers as AccessKit children of the bar, in `+` order per row. Returns them with
    /// their ids.
    fn nodes(&self, ui: &mut Ui, geo: &Geometry, stops: &GradientStops, selected: Option<StopRef>) -> Vec<(StopRef, Id)> {
        let percent = |v: f32, decimals: usize| fill(&self.labels.percent, &[("n", &self.numbers.format(f64::from(v) * 100.0, decimals..=decimals))]);
        let location = |at: f32| percent(at, self.location_decimals);
        let int = |n: usize| self.numbers.format(n as f64, 0..=0);
        let mut out = Vec::new();
        for row in geo.rows() {
            let at = locations(stops, row);
            let n = int(at.len());
            for m in geo.order(stops, row) {
                let (name, value, rect) = match m {
                    StopRef::Color(i) => {
                        let stop = stops.color[i];
                        let hex = Rgba::from(Srgba { a: 1.0, ..stop.color }).to_hex().to_uppercase();
                        let values = [("location", location(stop.location)), ("hex", hex), ("opacity", percent(stop.color.a, 0))];
                        let template = match geo.rows {
                            StopRows::Split => &self.labels.color_value,
                            StopRows::Combined => &self.labels.color_opacity_value,
                        };
                        let value = fill(template, &values.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>());
                        (fill(&self.labels.color_stop, &[("i", &int(i + 1)), ("n", &n)]), value, geo.marker_rect(row, stop.location))
                    }
                    StopRef::Opacity(i) => {
                        let stop = stops.opacity_stops()[i];
                        let value =
                            fill(&self.labels.opacity_value, &[("location", &location(stop.location)), ("opacity", &percent(stop.opacity, 0))]);
                        (fill(&self.labels.opacity_stop, &[("i", &int(i + 1)), ("n", &n)]), value, geo.marker_rect(row, stop.location))
                    }
                    StopRef::Midpoint(row, i) => {
                        let template = match row {
                            StopRow::Color => &self.labels.midpoint,
                            StopRow::Opacity => &self.labels.opacity_midpoint,
                        };
                        let r = geo.style.diamond;
                        let rect = Rect::from_center_size(geo.diamond_center(row, midpoint_at(stops, row, i)), vec2(2.0 * r, 2.0 * r));
                        (fill(template, &[("i", &int(i + 1))]), percent(midpoint(stops, row, i), 0), rect)
                    }
                };
                let node = ui.interact(rect, ui.scope_id().with(("marker", m)), Sense::hover());
                let enabled = ui.is_enabled();
                node.widget_info(|| {
                    let mut info = WidgetInfo::labeled(Role::GraphicsObject, enabled, &name);
                    info.current_text_value = Some(value.clone());
                    info
                });
                ui.ctx().accesskit_node_builder(node.id, |b| b.set_selected(selected == Some(m)));
                out.push((m, node.id));
            }
        }
        out
    }
}

/// The bar's part of a pass: its edit, the stops it leaves, and what it reports beside it.
struct Bar<T> {
    edit: EditResponse<GradientStops>,
    value: GradientStops,
    color_requested: Option<usize>,
    dropped: Option<(StopDrop, Arc<T>)>,
}

/// How the markers are drawn this frame.
struct Look {
    selected: Option<StopRef>,
    hovered: Option<StopRef>,
    enabled: bool,
}

/// The locations of `row`'s stops as the bar shows them.
fn locations(stops: &GradientStops, row: StopRow) -> Vec<f32> {
    match row {
        StopRow::Color => stops.color.iter().map(|s| s.location).collect(),
        StopRow::Opacity => stops.opacity_stops().iter().map(|s| s.location).collect(),
    }
}

/// The midpoint after stop `i` of `row`, 0..=1 of its segment.
fn midpoint(stops: &GradientStops, row: StopRow, i: usize) -> f32 {
    match row {
        StopRow::Color => stops.color[i].midpoint,
        StopRow::Opacity => stops.opacity_stops()[i].midpoint,
    }
}

/// Where the midpoint after stop `i` of `row` is on the bar.
fn midpoint_at(stops: &GradientStops, row: StopRow, i: usize) -> f32 {
    let at = locations(stops, row);
    at[i] + (at[i + 1] - at[i]) * midpoint(stops, row, i)
}

/// A diamond of radius `r` round `center`, outlined in `edge`, filled when `fill` is given.
fn diamond(center: Pos2, r: f32, fill: Option<Color32>, edge: Color32) -> Shape {
    let points = vec![center + vec2(0.0, -r), center + vec2(r, 0.0), center + vec2(0.0, r), center + vec2(-r, 0.0)];
    Shape::convex_polygon(points, fill.unwrap_or(Color32::TRANSPARENT), Stroke::new(1.0, edge))
}

/// Where the bar's parts are this frame.
#[derive(Clone, Copy, Debug)]
struct Geometry {
    ramp: Rect,
    rows: StopRows,
    midpoints: bool,
    style: GradientEditorStyle,
}

impl Geometry {
    /// The rows the bar shows, top first.
    fn rows(&self) -> Vec<StopRow> {
        match self.rows {
            StopRows::Split => vec![StopRow::Opacity, StopRow::Color],
            StopRows::Combined => vec![StopRow::Color],
        }
    }

    fn x(&self, location: f32) -> f32 {
        self.ramp.left() + location.clamp(0.0, 1.0) * self.ramp.width()
    }

    /// The location under `x`, unclamped.
    fn location(&self, x: f32) -> f32 {
        (x - self.ramp.left()) / self.ramp.width().max(1.0)
    }

    /// Whether `m` is a marker of `stops` on this bar.
    fn valid(&self, stops: &GradientStops, m: StopRef) -> bool {
        let has = |row| row == StopRow::Color || self.rows == StopRows::Split;
        match m {
            StopRef::Midpoint(row, i) => self.midpoints && has(row) && i + 1 < stops.len(row),
            m => has(m.row()) && m.index() < stops.len(m.row()),
        }
    }

    /// The midpoints of `row` the bar shows, with their locations: between stops more than
    /// [`MIN_GAP`] apart.
    fn diamonds(&self, stops: &GradientStops, row: StopRow) -> Vec<(usize, f32)> {
        if !self.midpoints {
            return Vec::new();
        }
        let at = locations(stops, row);
        (0..at.len().saturating_sub(1)).filter(|&i| at[i + 1] - at[i] > MIN_GAP).map(|i| (i, midpoint_at(stops, row, i))).collect()
    }

    /// The markers of `row` in location order, each midpoint between its stops.
    fn order(&self, stops: &GradientStops, row: StopRow) -> Vec<StopRef> {
        let diamonds = self.diamonds(stops, row);
        let mut out = Vec::new();
        for i in 0..stops.len(row) {
            out.push(StopRef::stop(row, i));
            if diamonds.iter().any(|(k, _)| *k == i) {
                out.push(StopRef::Midpoint(row, i));
            }
        }
        out
    }

    /// Where marker `m` is, 0..=1.
    fn location_of(&self, stops: &GradientStops, m: StopRef) -> f32 {
        match m {
            StopRef::Midpoint(row, i) => midpoint_at(stops, row, i),
            m => locations(stops, m.row())[m.index()],
        }
    }

    fn diamond_center(&self, row: StopRow, at: f32) -> Pos2 {
        let d = 2.0 * self.style.diamond;
        let y = match (row, self.rows) {
            (StopRow::Color, StopRows::Split) => self.ramp.bottom() + d,
            _ => self.ramp.top() - d,
        };
        pos2(self.x(at), y)
    }

    /// The square of a stop marker of `row` at `at`. Colour markers hang below the ramp, opacity
    /// markers stand above it.
    fn body(&self, row: StopRow, at: f32) -> Rect {
        let (x, s) = (self.x(at), &self.style);
        let top = match row {
            StopRow::Color => self.ramp.bottom() + s.pointer,
            StopRow::Opacity => self.ramp.top() - s.pointer - s.marker,
        };
        Rect::from_min_size(pos2(x - s.marker / 2.0, top), vec2(s.marker, s.marker))
    }

    /// A marker's square and pointer.
    fn marker_rect(&self, row: StopRow, at: f32) -> Rect {
        let body = self.body(row, at);
        let tip = match row {
            StopRow::Color => self.ramp.bottom(),
            StopRow::Opacity => self.ramp.top(),
        };
        body.union(Rect::from_x_y_ranges(body.x_range(), tip..=tip))
    }

    /// A stop marker: the pointer from the ramp's edge, the square in `content` with a one-point
    /// outline outside it, pointer and outline in `edge`.
    fn marker(&self, row: StopRow, at: f32, content: Color32, edge: Color32) -> Vec<Shape> {
        let (x, half) = (self.x(at), self.style.marker / 2.0);
        let body = self.body(row, at);
        let (tip, base) = match row {
            StopRow::Color => (self.ramp.bottom() + 1.0, body.top()),
            StopRow::Opacity => (self.ramp.top() - 1.0, body.bottom()),
        };
        vec![
            Shape::convex_polygon(vec![pos2(x, tip), pos2(x + half, base), pos2(x - half, base)], edge, Stroke::NONE),
            Shape::rect_filled(body, 1.0, content),
            Shape::rect_stroke(body, 1.0, Stroke::new(1.0, edge), StrokeKind::Outside),
        ]
    }

    /// The marker under `pos`: off the ramp, a stop within `gradient_editor.hit` across (away
    /// from the ramp's edge by a diamond's radius), else a midpoint within
    /// `gradient_editor.diamond_hit`; the nearest, the selected one among equals.
    fn hit(&self, stops: &GradientStops, pos: Pos2, selected: Option<StopRef>) -> Option<StopRef> {
        let s = &self.style;
        let (row, beyond, midpoints) = if pos.y > self.ramp.bottom() {
            (StopRow::Color, pos.y >= self.ramp.bottom() + s.diamond, self.rows == StopRows::Split)
        } else if pos.y < self.ramp.top() {
            match self.rows {
                StopRows::Split => (StopRow::Opacity, pos.y <= self.ramp.top() - s.diamond, true),
                StopRows::Combined => (StopRow::Color, false, true),
            }
        } else {
            return None;
        };
        let nearest = |candidates: Vec<(StopRef, f32)>, within: f32| {
            candidates
                .into_iter()
                .map(|(m, at)| (m, (self.x(at) - pos.x).abs()))
                .filter(|(_, d)| *d <= within)
                .min_by(|a, b| a.1.total_cmp(&b.1).then((selected != Some(a.0)).cmp(&(selected != Some(b.0)))))
                .map(|(m, _)| m)
        };
        let stop = match beyond {
            true => nearest(locations(stops, row).into_iter().enumerate().map(|(i, at)| (StopRef::stop(row, i), at)).collect(), s.hit),
            false => None,
        };
        stop.or_else(|| match midpoints {
            true => nearest(self.diamonds(stops, row).into_iter().map(|(i, at)| (StopRef::Midpoint(row, i), at)).collect(), s.diamond_hit),
            false => None,
        })
    }

    /// The colour stop or location a payload dropped at `pos` lands on.
    fn drop_place(&self, stops: &GradientStops, pos: Pos2) -> StopDrop {
        let near = stops
            .color
            .iter()
            .enumerate()
            .map(|(i, s)| (i, (self.x(s.location) - pos.x).abs()))
            .filter(|(_, d)| *d <= self.style.hit)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match near {
            Some((i, _)) => StopDrop::OnStop(i),
            None => StopDrop::At(stops::round_location(self.location(pos.x))),
        }
    }
}

/// One frame's edit of the stops: the pointer and the keys feed the bar's gesture.
struct BarEdit<'g> {
    gesture: &'g mut Gesture<GradientStops>,
    state: &'g mut State,
    geo: Geometry,
    rules: &'g StopRules,
    sampler: &'g Sampler,
}

impl BarEdit<'_> {
    /// Press, drag and release of a marker; a click on empty space; a right-click. A press gives
    /// the bar the focus and, while another control's gesture has it, waits for that gesture's
    /// ending event ([`turn`](BarEdit::turn)).
    fn pointer(&mut self, ui: &Ui, response: &Response, waits: bool) {
        // Hit-test where the button went down, not where egui first recognises a drag.
        let down = response.is_pointer_button_down_on();
        let press = down && ui.input(|i| i.pointer.primary_pressed());
        let mut new = Vec::new();
        if press && let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
            new.push(Input::Press { pos, alt: ui.input(|i| i.modifiers.alt) });
        }
        if response.secondary_clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            new.push(Input::Secondary(pos));
        }
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            new.push(Input::Click(pos));
        }
        if new.iter().any(|i| !matches!(i, Input::Click(_))) {
            response.request_focus();
        }
        for input in self.turn(ui.ctx(), waits, new) {
            match input {
                Input::Press { pos, alt } => self.press(pos, alt),
                Input::Secondary(pos) => {
                    if let Some(m) = self.geo.hit(self.gesture.shown(), pos, self.state.selected) {
                        self.state.selected = Some(m);
                    }
                }
                Input::Click(pos) => self.add_at(pos),
            }
        }
        let released = ui.input(|i| i.pointer.primary_released());
        if !press
            && (down || released)
            && let Some(drag) = self.state.drag.clone()
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        {
            self.drag_to(ui, &drag, pos);
        }
        if self.state.drag.is_some() && !down {
            self.state.drag = None;
            self.gesture.commit();
        }
    }

    /// The inputs that run in this pass: those held from earlier passes, then `new`. While
    /// `waits`, they wait as a composite's press does (`controls::slider::press_turn`), at most
    /// [`PRESS_WAIT`] passes.
    fn turn(&mut self, ctx: &Context, waits: bool, new: Vec<Input>) -> Vec<Input> {
        let pass = ctx.cumulative_pass_nr();
        let held = self.state.held.take().filter(|h| h.last + 1 == pass);
        let (since, mut inputs) = held.map_or((pass, Vec::new()), |h| (h.since, h.inputs));
        inputs.extend(new);
        if waits && !inputs.is_empty() && pass < since + PRESS_WAIT {
            self.state.held = Some(Held { since, last: pass, inputs });
            ctx.request_repaint();
            return Vec::new();
        }
        inputs
    }

    fn press(&mut self, pos: Pos2, alt: bool) {
        let stops = self.gesture.shown().clone();
        let Some(m) = self.geo.hit(&stops, pos, self.state.selected) else { return };
        let before = self.state.selected;
        self.state.selected = Some(m);
        let grab = self.geo.location_of(&stops, m);
        self.state.drag = Some(Drag { marker: m, base: stops, press: pos, grab, copy: alt && m.is_stop(), before });
        self.gesture.begin();
    }

    /// A stop added where a click on empty space was: above the ramp's centre in the opacity row
    /// of a split bar, else in the colour row.
    fn add_at(&mut self, pos: Pos2) {
        let stops = self.gesture.shown();
        if self.state.drag.is_some() || self.geo.hit(stops, pos, self.state.selected).is_some() {
            return;
        }
        let row = match self.geo.rows {
            StopRows::Split if pos.y < self.geo.ramp.center().y => StopRow::Opacity,
            _ => StopRow::Color,
        };
        if let Some((stops, i)) = stops::add(stops, row, self.geo.location(pos.x), self.sampler, self.rules) {
            self.state.selected = Some(StopRef::stop(row, i));
            self.gesture.set(stops);
        }
    }

    /// The drag's stops with the marker at the pointer: moved, copied, or torn off.
    fn drag_to(&mut self, ui: &Ui, drag: &Drag, pos: Pos2) {
        let Drag { marker, ref base, press, grab, copy, .. } = *drag;
        let to = grab + self.geo.location(pos.x) - self.geo.location(press.x);
        let moved = pos.x != press.x;
        let ramp = self.geo.ramp;
        let off = pos.y > ramp.bottom() + self.geo.style.tear_off || pos.y < ramp.top() - self.geo.style.tear_off;
        let (stops, selected) = match marker {
            StopRef::Midpoint(row, i) => {
                let at = locations(base, row);
                let m = (to - at[i]) / (at[i + 1] - at[i]).max(MIN_GAP);
                match moved {
                    true => stops::set_midpoint(base, row, i, m, self.rules).map_or((base.clone(), Some(marker)), |(s, _)| (s, Some(marker))),
                    false => (base.clone(), Some(marker)),
                }
            }
            m => {
                let (row, i) = (m.row(), m.index());
                // At the row's minimum a stop dragged off the bar stays and moves.
                let torn = (off && !copy).then(|| stops::remove(base, row, i, self.rules)).flatten().map(|(s, _)| (s, None));
                let edited = torn.or_else(|| match (moved, copy) {
                    (false, _) => None,
                    (true, true) => stops::copy(base, row, i, to, self.rules).map(|(s, j)| (s, Some(StopRef::stop(row, j)))),
                    (true, false) => stops::move_to(base, row, i, to, self.rules).map(|(s, j)| (s, Some(StopRef::stop(row, j)))),
                });
                edited.unwrap_or_else(|| (base.clone(), Some(marker)))
            }
        };
        self.state.selected = selected;
        if stops != *self.gesture.shown() {
            self.gesture.update(stops, ui.input(|i| i.modifiers));
        }
    }

    /// Selection, moves, deletes and additions while the bar has focus and no drag is under way.
    /// Each change is one Set. Only `+` takes extra modifiers; every other key needs exactly its
    /// own, so Shift+Insert (paste) and Alt+arrows stay the app's. Returns the marker Enter or
    /// Space activated.
    fn keys(&mut self, ui: &Ui) -> Option<StopRef> {
        if self.state.drag.is_some() {
            return None;
        }
        let take = |modifiers, key| take_exact(ui, modifiers, key);
        let mut stops = self.gesture.shown().clone();
        let plus = ui.input_mut(|i| i.count_and_consume_key(Modifiers::NONE, Key::Plus)) + take(Modifiers::NONE, Key::Equals);
        for _ in 0..plus {
            self.step(&stops, true);
        }
        for _ in 0..take(Modifiers::NONE, Key::Minus) {
            self.step(&stops, false);
        }
        if self.geo.rows == StopRows::Split {
            for _ in 0..take(Modifiers::NONE, Key::ArrowUp) {
                self.switch(&stops, StopRow::Opacity);
            }
            for _ in 0..take(Modifiers::NONE, Key::ArrowDown) {
                self.switch(&stops, StopRow::Color);
            }
        }
        if take(Modifiers::NONE, Key::Insert) + take(Modifiers::COMMAND, Key::Enter) > 0 {
            let row = self.state.selected.map_or(StopRow::Color, StopRef::row);
            let at = stops::halfway(&stops, row, self.state.selected.map(StopRef::index));
            if let Some((added, i)) = stops::add(&stops, row, at, self.sampler, self.rules) {
                stops = added;
                self.state.selected = Some(StopRef::stop(row, i));
            }
        }
        let activated = (take(Modifiers::NONE, Key::Enter) + take(Modifiers::NONE, Key::Space) > 0).then_some(self.state.selected).flatten();
        if let Some(m) = self.state.selected {
            let (row, i) = (m.row(), m.index());
            if m.is_stop()
                && take(Modifiers::NONE, Key::Delete) + take(Modifiers::NONE, Key::Backspace) > 0
                && let Some((removed, j)) = stops::remove(&stops, row, i, self.rules)
            {
                stops = removed;
                self.state.selected = Some(StopRef::stop(row, j));
            } else {
                let steps = |key| 10 * take(Modifiers::SHIFT, key) as i32 + take(Modifiers::NONE, key) as i32;
                let d = (steps(Key::ArrowRight) - steps(Key::ArrowLeft)) as f32 / 100.0;
                let moved = match m {
                    _ if d == 0.0 => None,
                    StopRef::Midpoint(..) => stops::set_midpoint(&stops, row, i, midpoint(&stops, row, i) + d, self.rules).map(|(s, _)| (s, m)),
                    _ => stops::move_to(&stops, row, i, locations(&stops, row)[i] + d, self.rules).map(|(s, j)| (s, StopRef::stop(row, j))),
                };
                if let Some((s, m)) = moved {
                    stops = s;
                    self.state.selected = Some(m);
                }
            }
        }
        if stops != *self.gesture.shown() {
            self.gesture.set(stops);
        }
        activated
    }

    /// Selects the next or previous marker of the selected row; the first or last with nothing
    /// selected.
    fn step(&mut self, stops: &GradientStops, forward: bool) {
        let row = self.state.selected.map_or(StopRow::Color, StopRef::row);
        let order = self.geo.order(stops, row);
        let Some(last) = order.len().checked_sub(1) else { return };
        let k = match self.state.selected.and_then(|s| order.iter().position(|m| *m == s)) {
            None if forward => 0,
            None => last,
            Some(k) if forward => (k + 1).min(last),
            Some(k) => k.saturating_sub(1),
        };
        self.state.selected = Some(order[k]);
    }

    /// Selects the stop of `row` nearest the selected marker's location.
    fn switch(&mut self, stops: &GradientStops, row: StopRow) {
        if self.state.selected.is_some_and(|s| s.row() == row) {
            return;
        }
        let at = self.state.selected.map_or(0.0, |s| self.geo.location_of(stops, s));
        let nearest = locations(stops, row).into_iter().enumerate().min_by(|a, b| (a.1 - at).abs().total_cmp(&(b.1 - at).abs())).map(|(i, _)| i);
        if let Some(i) = nearest {
            self.state.selected = Some(StopRef::stop(row, i));
        }
    }
}
