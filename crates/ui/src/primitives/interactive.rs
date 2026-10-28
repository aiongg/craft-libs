//! [`Interactive`]: the hit area, AccessKit node and keyboard activation every control builds on.

use egui::accesskit::Role;
use egui::{AsIdSalt, CornerRadius, IdSalt, Response, Sense, Ui, Vec2, WidgetInfo, vec2};

use super::{FocusRing, StateLayer};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf};

/// An interactive element: allocates its rect, senses the pointer, reports its role, label, value
/// and state to AccessKit, and paints the [`StateLayer`] and [`FocusRing`].
///
/// With a click sense (the default) it takes keyboard focus, and Enter and Space activate it
/// while it has focus: [`Response::clicked`] is true for them as for a pointer click, and for a
/// screen reader's click. The caller draws the content over the state layer after
/// [`show`](Interactive::show).
#[derive(Clone, Debug)]
pub struct Interactive {
    id_salt: IdSalt,
    role: Role,
    label: String,
    sense: Sense,
    value: Option<String>,
    checked: Option<bool>,
    selected: Option<bool>,
    expanded: Option<bool>,
    radius: Option<CornerRadius>,
    state_layer: bool,
    focus_ring: bool,
}

impl Interactive {
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
            radius: None,
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

    /// Checked or not, for check boxes, radio buttons, switches and toggle buttons.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    /// Selected or not, for tabs, list items and segments.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Open or closed, for menu buttons, combo boxes and disclosure headers.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }

    /// The corners of the state layer and focus ring: a token radius such as
    /// `tokens.radius.button`. `radius.control` by default.
    pub fn radius(mut self, radius: CornerRadius) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Whether to paint the [`StateLayer`]; on by default.
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
    pub fn show(self, ui: &mut Ui, size: Vec2) -> Response {
        let id = ui.make_persistent_id(self.id_salt);
        let (_, rect) = ui.allocate_space(size);
        let response = ui.interact(rect, id, self.sense);
        let enabled = ui.is_enabled();
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(self.role, enabled, &self.label);
            info.current_text_value.clone_from(&self.value);
            info.selected = self.checked;
            info
        });
        if self.selected.is_some() || self.expanded.is_some() {
            ui.ctx().accesskit_node_builder(id, |node| {
                if let Some(selected) = self.selected {
                    node.set_selected(selected);
                }
                if let Some(expanded) = self.expanded {
                    node.set_expanded(expanded);
                }
            });
        }
        let radius = self.radius.unwrap_or_else(|| Tokens::of(ui).radius.control);
        if self.state_layer {
            StateLayer::paint(ui, rect, radius, &response);
        }
        if self.focus_ring {
            FocusRing::paint(ui, rect, radius, &response);
        }
        response
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Interactive"];

const ROLE_NAMES: &[&str] = &["button", "check box", "tab"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story {
        id: "primitives/interactive/element",
        level: Level::Primitives,
        component: "Interactive",
        name: "Element",
        args: || Args::new().with_choice("role", ROLE_NAMES, 0).with_text("label", "Activate").with_bool("disabled", false),
        show,
    }]
}

/// One element; clicks, Enter and Space count up.
fn show(ui: &mut Ui, args: &mut Args) {
    let tokens = Tokens::of(ui);
    let role = match args.choice("role") {
        "check box" => Role::CheckBox,
        "tab" => Role::Tab,
        _ => Role::Button,
    };
    let label = args.text("label").to_owned();
    let count_id = ui.scope_id().with("interactive-story-count");
    let count = ui.data(|d| d.get_temp::<u32>(count_id).unwrap_or(0));
    ui.add_enabled_ui(!args.bool("disabled"), |ui| {
        let mut element = Interactive::new("interactive-story", role, &label);
        element = match role {
            Role::CheckBox => element.checked(count % 2 == 1),
            Role::Tab => element.selected(count % 2 == 1),
            _ => element,
        };
        let response = element.show(ui, vec2(120.0, tokens.size.control.md));
        let color = tokens.color.text.normal.to_egui();
        ui.painter().text(response.rect.center(), egui::Align2::CENTER_CENTER, &label, tokens.font(craft_theme::TypeRole::Label), color);
        if response.clicked() {
            ui.data_mut(|d| d.insert_temp(count_id, count + 1));
        }
    });
    ui.add(super::Text::new(format!("Activated {count} times")).content(super::Content::Muted));
}
