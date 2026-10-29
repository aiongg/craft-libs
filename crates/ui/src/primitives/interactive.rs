//! [`Interactive`]: the hit area, AccessKit node and keyboard activation every control builds on.

use std::ops::RangeInclusive;

use egui::accesskit::{Action, Role};
use egui::{AsIdSalt, Color32, Id, IdSalt, InteractOptions, Rect, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, vec2};

use super::{Corner, FocusRing, StateColors, StateLayer, Surface, SurfaceEdge, SurfaceKind};
use crate::inspect::{self, RecordKind};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf};

/// An interactive element: allocates its rect, senses the pointer, reports its role, label, value
/// and state to AccessKit, and paints its fill and the [`FocusRing`].
///
/// With a click sense (the default) it takes keyboard focus, and Enter and Space activate it
/// while it has focus: [`Response::clicked`] is true for them as for a pointer click, and for a
/// screen reader's click.
///
/// [`show`](Interactive::show) paints, in order: the fill, which is the [`surface`](Interactive::surface)'s
/// fill (transparent without one) changing to the hover and pressed fills ([`StateLayer`]); the
/// surface's border or bevel over it; and the focus ring on a layer above. The caller draws the
/// content after `show`, between the edge and the ring.
///
/// A control with its own colour tokens passes them with [`colors`](Interactive::colors): they
/// take the place of the surface's fill, the `state.*` fills and the border colour, while the
/// surface keeps the shape and the edge style (a bevel stays a bevel). A raised surface draws as
/// [`SurfaceKind::Pressed`] while pressed and while the element is checked or selected, so a
/// bevelled button presses in.
///
/// A group whose items' states depend on each other (a radio group, where a click on one item
/// unchecks another) senses every item with [`interact`](Interactive::interact) first, settles
/// the states, then reports and paints each item with [`Sensed::finish`]; `show` does both steps
/// for a single element.
#[derive(Debug)]
pub struct Interactive<'a> {
    id: IdSource,
    sense: Sense,
    enabled: bool,
    checked: Option<State<'a>>,
    mixed: bool,
    selected: Option<State<'a>>,
    report: Report,
    look: Look,
}

/// Where an element's id comes from.
#[derive(Debug)]
enum IdSource {
    /// A salt in the `Ui` the element is placed in.
    Salt(IdSalt),
    /// The id itself.
    Id(Id),
}

/// What [`Sensed::finish`] reports to AccessKit, besides the state.
#[derive(Debug)]
struct Report {
    role: Role,
    label: String,
    value: Option<String>,
    numeric: Option<Numeric>,
    expanded: Option<bool>,
    busy: bool,
    /// Zero-based, for an item of a group whose node reports the set's size.
    position: Option<usize>,
}

/// A numeric value with its bounds, for sliders.
#[derive(Clone, Debug)]
struct Numeric {
    /// `None` for a mixed value.
    value: Option<f64>,
    range: RangeInclusive<f64>,
    step: f64,
}

/// How [`Sensed::finish`] paints.
#[derive(Debug)]
struct Look {
    surface: Option<SurfaceKind>,
    corner: Option<Corner>,
    border_width: Option<f32>,
    colors: Option<StateColors>,
    on_colors: Option<StateColors>,
    state_layer: bool,
    focus_ring: bool,
}

/// A checked or selected state: given, or bound to the caller's value and changed on click.
#[derive(Debug)]
enum State<'a> {
    Fixed(bool),
    /// Flips on click.
    Toggle(&'a mut bool),
    /// Becomes true on click.
    Select(&'a mut bool),
}

impl State<'_> {
    /// Applies a click, if any, and returns the state after it and whether it changed.
    fn update(&mut self, clicked: bool) -> (bool, bool) {
        match self {
            State::Fixed(on) => (*on, false),
            State::Toggle(on) => {
                if clicked {
                    **on = !**on;
                }
                (**on, clicked)
            }
            State::Select(on) => {
                let changed = clicked && !**on;
                **on |= clicked;
                (**on, changed)
            }
        }
    }
}

impl<'a> Interactive<'a> {
    /// An element with the AccessKit `role` and accessible name `label`; `id_salt` keeps its
    /// focus and animations when the layout around it changes.
    pub fn new(id_salt: impl AsIdSalt, role: Role, label: impl Into<String>) -> Self {
        Self::salted(IdSalt::new(id_salt), role, label.into())
    }

    /// As [`new`](Interactive::new) for a control that holds `salt` already made by
    /// `IdSalt::new`: the element has the id `ui.make_persistent_id(salt)`, the one
    /// [`id_in`](Interactive::id_in) gives the salt's source. Passing the held salt to `new` would
    /// hash it a second time.
    pub fn salted(salt: IdSalt, role: Role, label: impl Into<String>) -> Self {
        Self::with_id_source(IdSource::Salt(salt), role, label.into())
    }

    /// An element with the id `id` wherever it is placed: for an element whose id is needed
    /// before the `Ui` it sits in exists, e.g. an option of a
    /// [`RadioSet`](crate::controls::RadioSet).
    pub fn with_id(id: Id, role: Role, label: impl Into<String>) -> Self {
        Self::with_id_source(IdSource::Id(id), role, label.into())
    }

    fn with_id_source(id: IdSource, role: Role, label: String) -> Self {
        Self {
            id,
            sense: Sense::click(),
            enabled: true,
            checked: None,
            mixed: false,
            selected: None,
            report: Report { role, label, value: None, numeric: None, expanded: None, busy: false, position: None },
            look: Look { surface: None, corner: None, border_width: None, colors: None, on_colors: None, state_layer: true, focus_ring: true },
        }
    }

    /// The id [`show`](Interactive::show) gives an element made with `id_salt` in `ui`, for a
    /// control that needs it first (to read its focus). A control that holds the salt as an
    /// `IdSalt` uses `ui.make_persistent_id(salt)`, which is the same id.
    pub fn id_in(ui: &Ui, id_salt: impl AsIdSalt) -> Id {
        ui.make_persistent_id(IdSalt::new(id_salt))
    }

    /// What the element responds to; [`Sense::click`] by default.
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// The value a screen reader reads after the label, e.g. a slider's `"40 %"`.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.report.value = Some(value.into());
        self
    }

    /// Checked or not, for a check box, radio button, switch or toggle button whose state a
    /// click doesn't change directly. Use [`toggle`](Interactive::toggle) for one that a click
    /// flips.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(State::Fixed(checked));
        self
    }

    /// Checked or not, flipped on click, for check boxes, switches and toggle buttons. The
    /// element reports the state after the click in the same frame, and the response is
    /// [changed](Response::changed) on a click.
    pub fn toggle(mut self, checked: &'a mut bool) -> Self {
        self.checked = Some(State::Toggle(checked));
        self
    }

    /// Neither checked nor unchecked, for a check box over a selection where some items have the
    /// option and some don't: AccessKit toggled mixed, and the [`on_colors`](Interactive::on_colors).
    /// It takes the place of [`checked`](Interactive::checked).
    pub fn mixed(mut self) -> Self {
        self.mixed = true;
        self
    }

    /// Selected or not, for tabs, list items and segments whose selection a click doesn't set
    /// directly. Use [`select`](Interactive::select) for one that a click selects.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(State::Fixed(selected));
        self
    }

    /// Selected or not, set on click, for tabs, list items and segments. The element reports the
    /// state after the click in the same frame, and the response is
    /// [changed](Response::changed) when a click selects it.
    pub fn select(mut self, selected: &'a mut bool) -> Self {
        self.selected = Some(State::Select(selected));
        self
    }

    /// Open or closed, for menu buttons, combo boxes and disclosure headers.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.report.expanded = Some(expanded);
        self
    }

    /// The surface the element is drawn on, e.g. [`SurfaceKind::Raised`] for a push button: its
    /// fill is the rest fill, and its border or bevel goes over the state fills. Without one the
    /// element rests transparent, like a toolbar item.
    pub fn surface(mut self, kind: SurfaceKind) -> Self {
        self.look.surface = Some(kind);
        self
    }

    /// The corners of the fill and the focus ring: the surface's corners by default, and
    /// [`Corner::Control`] without a surface.
    pub fn corner(mut self, corner: Corner) -> Self {
        self.look.corner = Some(corner);
        self
    }

    /// The width of a flat surface's border when [`colors`](Interactive::colors) give it a colour,
    /// in points, in place of the surface's one point. A bevel keeps its own width.
    pub fn border_width(mut self, width: f32) -> Self {
        self.look.border_width = Some(width);
        self
    }

    /// The element's own fill and border colours per state, from its component's tokens, in
    /// place of the surface's fill, `state.hover`, `state.pressed` and the surface's border colour.
    /// The disabled colour applies when the element is disabled.
    pub fn colors(mut self, colors: impl Into<StateColors>) -> Self {
        self.look.colors = Some(colors.into());
        self
    }

    /// The colours while the element is checked or selected, after this frame's click: a toggle
    /// button's pressed look, a selected segment. Without them the element keeps
    /// [`colors`](Interactive::colors).
    pub fn on_colors(mut self, colors: impl Into<StateColors>) -> Self {
        self.look.on_colors = Some(colors.into());
        self
    }

    /// Whether the element takes input; on by default, and off in a disabled `Ui`. A disabled
    /// element senses nothing, reports itself disabled and paints its disabled colours. Unlike a
    /// disabled `Ui`, it fades nothing: a control with disabled tokens draws its disabled state in
    /// an enabled `Ui`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The element's place in its group, from 0, for a radio button, segment or tab: a screen
    /// reader reads it with the set's size, which the group's own node reports
    /// ([`RadioSelection::report_size`](crate::behavior::RadioSelection::report_size)).
    pub fn position_in_set(mut self, index: usize) -> Self {
        self.report.position = Some(index);
        self
    }

    /// Busy: the element is working (a loading button) and a screen reader says so.
    pub fn busy(mut self, busy: bool) -> Self {
        self.report.busy = busy;
        self
    }

    /// Whether the fill changes on hover and press; on by default.
    pub fn state_layer(mut self, paint: bool) -> Self {
        self.look.state_layer = paint;
        self
    }

    /// Whether to paint the [`FocusRing`]; on by default.
    pub fn focus_ring(mut self, paint: bool) -> Self {
        self.look.focus_ring = paint;
        self
    }

    /// Allocates `size` in the layout, interacts with it, reports the element and paints it.
    pub fn show(self, ui: &mut Ui, size: Vec2) -> Response {
        self.interact(ui, size).finish(ui)
    }

    /// Allocates `size` in the layout and interacts with it: a click changes bound states. The
    /// element is reported and painted by [`Sensed::finish`].
    pub fn interact(self, ui: &mut Ui, size: Vec2) -> Sensed {
        let (_, rect) = ui.allocate_space(size);
        self.interact_at(ui, rect)
    }

    /// Interacts with `rect` without allocating it in the layout, for an element placed inside a
    /// control's own rect (a slider's thumb); otherwise as [`interact`](Interactive::interact).
    ///
    /// Since `rect` isn't allocated, a part of it outside the space the control allocated doesn't
    /// grow the `Ui` (a scroll area around it doesn't scroll to it), and it is clipped to the
    /// `Ui`'s clip rect like any widget: neither painted nor hit outside it. An element that
    /// senses clicks or drags over an earlier widget takes that widget's hits (the later widget
    /// wins); one that senses only focus, as a slider's thumb, leaves them to the control.
    pub fn interact_at(mut self, ui: &Ui, rect: Rect) -> Sensed {
        let id = match self.id {
            // Already `IdSalt::new(..)` of the caller's salt, as `id_in` takes it.
            IdSource::Salt(salt) => ui.make_persistent_id(salt),
            IdSource::Id(id) => id,
        };
        let options = InteractOptions { enabled: self.enabled, ..Default::default() };
        let mut response = ui.interact_opt(rect, id, self.sense, options);
        inspect::record(ui, id, RecordKind::Interactive(self.report.role), rect, rect);
        // A click changes bound states before they are reported, as egui's check box does.
        let clicked = response.clicked();
        let mut changed = false;
        let mut update = |state: &mut Option<State<'_>>| {
            state.as_mut().map(|s| {
                let (on, change) = s.update(clicked);
                changed |= change;
                on
            })
        };
        let checked = update(&mut self.checked);
        let selected = update(&mut self.selected);
        if changed {
            response.mark_changed();
        }
        Sensed { response, checked, mixed: self.mixed, selected, report: self.report, look: self.look }
    }
}

/// An element that has sensed this frame's input, from [`Interactive::interact`], and is not yet
/// reported or painted.
#[derive(Debug)]
#[must_use = "an element is reported and painted by `finish`"]
pub struct Sensed {
    response: Response,
    checked: Option<bool>,
    mixed: bool,
    selected: Option<bool>,
    report: Report,
    look: Look,
}

impl Sensed {
    /// This frame's interaction, e.g. whether the element was clicked.
    pub fn response(&self) -> &Response {
        &self.response
    }

    /// Checked or not, as settled after sensing, e.g. the radio button a group selected this
    /// frame.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self.mixed = false;
        self
    }

    /// Mixed, as settled after sensing: a check box that stayed mixed. See
    /// [`Interactive::mixed`].
    pub fn mixed(mut self) -> Self {
        self.mixed = true;
        self
    }

    /// Selected or not, as settled after sensing.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// The value text, as settled after sensing: what a screen reader reads as the value of an
    /// element whose value is not one number (a colour area's two components).
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.report.value = Some(value.into());
        self
    }

    /// A numeric value, as settled after sensing, with the bounds it may take and the step of one
    /// increment, for a [`Role::Slider`]. A screen reader reads them, and may ask for an
    /// increment, a decrement (offered while the value is inside its bounds) or a value; the
    /// control reads those requests from the input.
    pub fn numeric(mut self, value: f64, range: RangeInclusive<f64>, step: f64) -> Self {
        self.report.numeric = Some(Numeric { value: Some(value), range, step });
        self
    }

    /// As [`numeric`](Sensed::numeric) for a value that is mixed (a selection whose items differ):
    /// no value, the bounds and step, and every request offered.
    pub fn numeric_mixed(mut self, range: RangeInclusive<f64>, step: f64) -> Self {
        self.report.numeric = Some(Numeric { value: None, range, step });
        self
    }

    /// Reports the element to AccessKit and paints its fill, edge and focus ring. The caller
    /// draws the content after it.
    pub fn finish(self, ui: &Ui) -> Response {
        let Sensed { response, checked, mixed, selected, report, look } = self;
        let enabled = response.enabled();
        let checked = if mixed { None } else { checked };
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(report.role, enabled, &report.label);
            info.current_text_value.clone_from(&report.value);
            info.value = report.numeric.as_ref().and_then(|n| n.value);
            info.selected = checked;
            info
        });
        if mixed || selected.is_some() || report.numeric.is_some() || report.expanded.is_some() || report.busy || report.position.is_some() {
            ui.ctx().accesskit_node_builder(response.id, |node| {
                if mixed {
                    node.set_toggled(egui::accesskit::Toggled::Mixed);
                }
                if let Some(Numeric { value, range, step }) = &report.numeric {
                    node.set_min_numeric_value(*range.start());
                    node.set_max_numeric_value(*range.end());
                    node.set_numeric_value_step(*step);
                    if value.is_none() {
                        node.clear_numeric_value();
                    }
                    if enabled {
                        node.add_action(Action::SetValue);
                        if value.is_none_or(|v| v < *range.end()) {
                            node.add_action(Action::Increment);
                        }
                        if value.is_none_or(|v| v > *range.start()) {
                            node.add_action(Action::Decrement);
                        }
                    }
                }
                if let Some(selected) = selected {
                    node.set_selected(selected);
                }
                if let Some(expanded) = report.expanded {
                    node.set_expanded(expanded);
                }
                if report.busy {
                    node.set_busy();
                }
                if let Some(position) = report.position {
                    node.set_position_in_set(position);
                }
            });
        }

        let tokens = Tokens::of(ui);
        let rect = response.rect;
        let on = mixed || checked == Some(true) || selected == Some(true);
        let pressed = look.state_layer && enabled && response.is_pointer_button_down_on();
        let kind = match look.surface {
            Some(SurfaceKind::Raised) if pressed || on => Some(SurfaceKind::Pressed),
            kind => kind,
        };
        let surface = kind.map(|kind| Surface::style(&tokens, kind));
        let radius = match (look.corner, look.surface) {
            (Some(corner), _) => corner.radius(&tokens),
            (None, Some(kind)) => Surface::style(&tokens, kind).radius,
            (None, None) => Corner::Control.radius(&tokens),
        };
        if ui.is_rect_visible(rect) {
            let colors = if on { look.on_colors.or(look.colors) } else { look.colors };
            let fills = colors.map_or_else(|| StateLayer::fills(&tokens, look.surface), |c| c.fill);
            let still = |states: &craft_theme::States<Color32>| if enabled { states.rest } else { states.disabled };
            let animated = |states: &craft_theme::States<Color32>| {
                if look.state_layer { StateLayer::color(ui, states, &response) } else { still(states) }
            };
            let fill = animated(&fills);
            match surface {
                Some(mut style) => {
                    style.fill = fill;
                    style.radius = radius;
                    if let (SurfaceEdge::Border(stroke), Some(border)) = (style.edge, colors.and_then(|c| c.border)) {
                        style.edge = SurfaceEdge::Border(Stroke::new(look.border_width.unwrap_or(stroke.width), animated(&border)));
                    }
                    ui.painter().extend(style.shapes(rect, ui.pixels_per_point()));
                }
                None if fill != Color32::TRANSPARENT => {
                    ui.painter().rect_filled(rect, radius, fill);
                }
                None => {}
            }
        }
        if look.focus_ring {
            FocusRing::paint_radius(ui, rect, radius, &response);
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Interactive"];

const ROLE_NAMES: &[&str] = &["button", "check box", "tab"];

const SURFACE_NAMES: &[&str] = &["none", "raised", "sunken", "panel"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "primitives/interactive/element",
        level: Level::Primitives,
        component: "Interactive",
        name: "Element",
        args: || {
            Args::new()
                .with_choice("role", ROLE_NAMES, 0)
                .with_choice("surface", SURFACE_NAMES, 0)
                .with_text("label", "Activate")
                .with_bool("disabled", false)
        },
        show,
    }]
}

/// One element (a check box toggles, a tab sits beside a second one); clicks, Enter and Space
/// count up.
fn show(ui: &mut Ui, args: &mut Args) {
    let tokens = Tokens::of(ui);
    let role = match args.choice("role") {
        "check box" => Role::CheckBox,
        "tab" => Role::Tab,
        _ => Role::Button,
    };
    let surface = SurfaceKind::ALL.iter().copied().find(|k| k.name() == args.choice("surface"));
    let label = args.text("label").to_owned();
    let state_id = ui.scope_id().with("interactive-story-state");
    let (mut count, mut checked, mut tab) = ui.data(|d| d.get_temp::<(u32, bool, usize)>(state_id).unwrap_or_default());
    let size = vec2(120.0, tokens.size.control.md);
    let text = |ui: &Ui, response: &Response, label: &str| {
        let color = tokens.color.text.normal.to_egui();
        ui.painter().text(response.rect.center(), egui::Align2::CENTER_CENTER, label, tokens.font(craft_theme::TypeRole::Label), color);
    };
    ui.add_enabled_ui(!args.bool("disabled"), |ui| {
        super::Row::new(super::Gap::Inline).show(ui, |ui| {
            let labels = if role == Role::Tab { vec![label.clone(), "Other".to_owned()] } else { vec![label.clone()] };
            for (i, label) in labels.iter().enumerate() {
                let mut element = Interactive::new(("interactive-story", i), role, label);
                if let Some(kind) = surface {
                    element = element.surface(kind);
                }
                let mut selected = tab == i;
                element = match role {
                    Role::CheckBox => element.toggle(&mut checked),
                    Role::Tab => element.select(&mut selected),
                    _ => element,
                };
                let response = element.show(ui, size);
                text(ui, &response, label);
                if response.clicked() {
                    count += 1;
                }
                if selected {
                    tab = i;
                }
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(state_id, (count, checked, tab)));
    ui.add(super::Text::new(format!("Activated {count} times")).content(super::Content::Muted));
}
