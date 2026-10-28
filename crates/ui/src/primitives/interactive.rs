//! [`Interactive`]: the hit area, AccessKit node and keyboard activation every control builds on.

use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, IdSalt, Response, Sense, Ui, Vec2, WidgetInfo, vec2};

use super::{Corner, FocusRing, StateLayer, Surface, SurfaceKind};
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
#[derive(Debug)]
pub struct Interactive<'a> {
    id_salt: IdSalt,
    role: Role,
    label: String,
    sense: Sense,
    value: Option<String>,
    checked: Option<State<'a>>,
    selected: Option<State<'a>>,
    expanded: Option<bool>,
    surface: Option<SurfaceKind>,
    corner: Option<Corner>,
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
            role,
            label: label.into(),
            sense: Sense::click(),
            value: None,
            checked: None,
            selected: None,
            expanded: None,
            surface: None,
            corner: None,
            state_layer: true,
            focus_ring: true,
        }
    }

    /// What the element responds to; [`Sense::click`] by default.
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// The value a screen reader reads after the label, e.g. a slider's `"40 %"`.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
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
        self.expanded = Some(expanded);
        self
    }

    /// The surface the element is drawn on, e.g. [`SurfaceKind::Raised`] for a push button: its
    /// fill is the rest fill, and its border or bevel goes over the state fills. Without one the
    /// element rests transparent, like a toolbar item.
    pub fn surface(mut self, kind: SurfaceKind) -> Self {
        self.surface = Some(kind);
        self
    }

    /// The corners of the fill and the focus ring: the surface's corners by default, and
    /// [`Corner::Control`] without a surface.
    pub fn corner(mut self, corner: Corner) -> Self {
        self.corner = Some(corner);
        self
    }

    /// Whether the fill changes on hover and press; on by default.
    pub fn state_layer(mut self, paint: bool) -> Self {
        self.state_layer = paint;
        self
    }

    /// Whether to paint the [`FocusRing`]; on by default.
    pub fn focus_ring(mut self, paint: bool) -> Self {
        self.focus_ring = paint;
        self
    }

    /// Allocates `size` in the layout and interacts with it.
    pub fn show(mut self, ui: &mut Ui, size: Vec2) -> Response {
        let id = ui.make_persistent_id(self.id_salt);
        let (_, rect) = ui.allocate_space(size);
        let mut response = ui.interact(rect, id, self.sense);
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
        let enabled = ui.is_enabled();
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(self.role, enabled, &self.label);
            info.current_text_value.clone_from(&self.value);
            info.selected = checked;
            info
        });
        if selected.is_some() || self.expanded.is_some() {
            ui.ctx().accesskit_node_builder(id, |node| {
                if let Some(selected) = selected {
                    node.set_selected(selected);
                }
                if let Some(expanded) = self.expanded {
                    node.set_expanded(expanded);
                }
            });
        }

        let tokens = Tokens::of(ui);
        let surface = self.surface.map(|kind| Surface::style(&tokens, kind));
        let radius = match (self.corner, surface) {
            (Some(corner), _) => corner.radius(&tokens),
            (None, Some(style)) => style.radius,
            (None, None) => Corner::Control.radius(&tokens),
        };
        if ui.is_rect_visible(rect) {
            let fill =
                if self.state_layer { StateLayer::fill(ui, self.surface, &response) } else { surface.map_or(Color32::TRANSPARENT, |s| s.fill) };
            match surface {
                Some(mut style) => {
                    style.fill = fill;
                    style.radius = radius;
                    ui.painter().extend(style.shapes(rect, ui.pixels_per_point()));
                }
                None if fill != Color32::TRANSPARENT => {
                    ui.painter().rect_filled(rect, radius, fill);
                }
                None => {}
            }
        }
        if self.focus_ring {
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
