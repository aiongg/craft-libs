//! [`Interactive`]: the hit area, AccessKit node and keyboard activation every control builds on.

use egui::accesskit::{Action, Role};
use egui::{AsIdSalt, Color32, Id, IdSalt, InteractOptions, Key, PointerButton, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, vec2};

use super::{Corner, FocusRing, StateColors, StateLayer, Surface, SurfaceEdge, SurfaceKind};
use crate::inspect::{self, RecordKind};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf};

/// An interactive element: allocates its rect, senses the pointer, reports its role, label, value
/// and state to AccessKit, and paints its fill and the [`FocusRing`].
///
/// With a click sense (the default) it takes keyboard focus, and Enter and Space activate it
/// while it has focus: [`Response::clicked`] is true for them as for a pointer click, and for a
/// screen reader's click. A [`Role::Link`] activates on Enter only, as a WAI-ARIA link does.
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
    id_salt: IdSalt,
    sense: Sense,
    enabled: bool,
    checked: Option<State<'a>>,
    selected: Option<State<'a>>,
    report: Report,
    look: Look,
}

/// What [`Sensed::finish`] reports to AccessKit, besides the state.
#[derive(Debug)]
struct Report {
    role: Role,
    label: String,
    value: Option<String>,
    expanded: Option<bool>,
    busy: bool,
}

/// How [`Sensed::finish`] paints.
#[derive(Debug)]
struct Look {
    surface: Option<SurfaceKind>,
    corner: Option<Corner>,
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
        Self {
            id_salt: IdSalt::new(id_salt),
            sense: Sense::click(),
            enabled: true,
            checked: None,
            selected: None,
            report: Report { role, label: label.into(), value: None, expanded: None, busy: false },
            look: Look { surface: None, corner: None, colors: None, on_colors: None, state_layer: true, focus_ring: true },
        }
    }

    /// The id [`show`](Interactive::show) gives an element made with `id_salt` in `ui`, for a
    /// control that needs it first (to read its focus).
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
    pub fn interact(mut self, ui: &mut Ui, size: Vec2) -> Sensed {
        // `id_salt` is already `IdSalt::new(..)` of the caller's salt, as `id_in` takes it.
        let id = ui.make_persistent_id(self.id_salt);
        let (_, rect) = ui.allocate_space(size);
        let options = InteractOptions { enabled: self.enabled, ..Default::default() };
        let mut response = ui.interact_opt(rect, id, self.sense, options);
        if self.report.role == Role::Link {
            ignore_space(ui, &mut response);
        }
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
        Sensed { response, checked, selected, report: self.report, look: self.look }
    }
}

/// Drops the activation by Space from a focused link's `response`: Enter activates a link, Space
/// doesn't (WAI-ARIA). egui counts both as a click of a focused widget.
fn ignore_space(ui: &Ui, response: &mut Response) {
    let id = response.id;
    let by_other_means =
        response.clicked_by(PointerButton::Primary) || ui.input(|i| i.key_pressed(Key::Enter) || i.has_accesskit_action_request(id, Action::Click));
    if response.clicked() && !by_other_means {
        // egui's `TextEdit` drops it the same way.
        response.flags.remove(egui::response::Flags::FAKE_PRIMARY_CLICKED);
    }
}

/// An element that has sensed this frame's input, from [`Interactive::interact`], and is not yet
/// reported or painted.
#[derive(Debug)]
#[must_use = "an element is reported and painted by `finish`"]
pub struct Sensed {
    response: Response,
    checked: Option<bool>,
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
        self
    }

    /// Selected or not, as settled after sensing.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Reports the element to AccessKit and paints its fill, edge and focus ring. The caller
    /// draws the content after it.
    pub fn finish(self, ui: &Ui) -> Response {
        let Sensed { response, checked, selected, report, look } = self;
        let enabled = response.enabled();
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(report.role, enabled, &report.label);
            info.current_text_value.clone_from(&report.value);
            info.selected = checked;
            info
        });
        if selected.is_some() || report.expanded.is_some() || report.busy {
            ui.ctx().accesskit_node_builder(response.id, |node| {
                if let Some(selected) = selected {
                    node.set_selected(selected);
                }
                if let Some(expanded) = report.expanded {
                    node.set_expanded(expanded);
                }
                if report.busy {
                    node.set_busy();
                }
            });
        }

        let tokens = Tokens::of(ui);
        let rect = response.rect;
        let on = checked == Some(true) || selected == Some(true);
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
                        style.edge = SurfaceEdge::Border(Stroke::new(stroke.width, animated(&border)));
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
