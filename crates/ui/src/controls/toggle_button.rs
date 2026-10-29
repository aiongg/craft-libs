//! [`ToggleButton`]: a button that stays pressed while on.

use craft_theme::{ButtonPart, ButtonVariant, ControlSize, States, TypeRole};
use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, IdSalt, Response, Ui, Widget};

use super::button::{ButtonStyle, Face, content_color};
use super::{SIZE_NAMES, salt, size_arg};
use crate::icons::Icon;
use crate::primitives::{Corner, Gap, Interactive, Row, StateColors, SurfaceKind};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

/// A toggle button's look, from [`ToggleButton::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToggleButtonStyle {
    /// Off: a secondary button.
    pub off: ButtonStyle,
    /// On: `toggle_button.on.*`, held in every state but disabled, and
    /// `toggle_button.on_disabled.*` when disabled.
    pub on_fill: States<Color32>,
    pub on_border: States<Color32>,
    pub on_text: States<Color32>,
}

/// A secondary-looking button that stays pressed while on:
/// `ui.add(ToggleButton::new("Snap", &mut snap))`.
///
/// On, it draws `toggle_button.on.*` (the `state.selected` fill) and its raised surface draws
/// pressed in, so a bevel theme shows it sunk. Disabled while on, it draws
/// `toggle_button.on_disabled.*`, which still looks on. A click flips the value; AccessKit reports
/// the button toggled, after the click, in the same frame.
#[derive(Debug)]
pub struct ToggleButton<'a> {
    label: String,
    on: &'a mut bool,
    size: ControlSize,
    icon: Option<Icon>,
    role: Option<TypeRole>,
    enabled: bool,
    id_salt: Option<IdSalt>,
}

impl<'a> ToggleButton<'a> {
    pub fn new(label: impl Into<String>, on: &'a mut bool) -> Self {
        Self { label: label.into(), on, size: ControlSize::Md, icon: None, role: None, enabled: true, id_salt: None }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A leading icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The type role of the label, for a path or a value in `Mono`; `button.label` by default.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = Some(role);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Keeps focus and animations when the layout around it changes; else the `Ui`'s next
    /// automatic id.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    pub fn style(tokens: &UiTokens, size: ControlSize) -> ToggleButtonStyle {
        let off = super::Button::style(tokens, ButtonVariant::Secondary, size);
        let on = &tokens.component.toggle_button;
        let held = |part| {
            let on_color = on.on(part).to_egui();
            States::new(on_color, on_color, on_color, on.on_disabled(part).to_egui())
        };
        ToggleButtonStyle { off, on_fill: held(ButtonPart::Fill), on_border: held(ButtonPart::Border), on_text: held(ButtonPart::Text) }
    }
}

impl Widget for ToggleButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let mut style = ToggleButton::style(&tokens, self.size);
        if let Some(role) = self.role {
            style.off.label = role;
        }
        let face = Face { label: &self.label, icon: self.icon, size: self.size, loading: false };
        let laid = face.layout(ui, &tokens, &style.off);
        let id_salt = salt(ui, self.id_salt);
        let on = self.on;
        let response = Interactive::salted(id_salt, Role::Button, self.label.as_str())
            .surface(SurfaceKind::Raised)
            .corner(Corner::Button)
            .border_width(style.off.border_width)
            .colors(style.off.colors())
            .on_colors(StateColors { fill: style.on_fill, border: Some(style.on_border) })
            .enabled(self.enabled)
            .toggle(&mut *on)
            .show(ui, laid.size);
        let text = if *on { &style.on_text } else { &style.off.text };
        face.paint(ui, response.rect, &laid, &style.off, content_color(ui, text, &response, true));
        laid.tooltip(response, &self.label)
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Toggle button"];

pub(crate) fn stories() -> Vec<Story> {
    vec![Story::new(
        "controls/toggle-button/toggle",
        Level::Controls,
        "Toggle button",
        "Toggle",
        || Args::new().with_choice("size", SIZE_NAMES, 1).with_bool("disabled", false).with_text("label", "Snap to grid"),
        show,
    )]
}

/// One toggle off and one on, with and without an icon.
fn show(ui: &mut Ui, args: &mut Args) {
    let size = size_arg(args.choice("size"));
    let enabled = !args.bool("disabled");
    let label = args.text("label").to_owned();
    let state_id = ui.scope_id().with("toggle-button-state");
    let (mut snap, mut guides) = ui.data(|d| d.get_temp::<(bool, bool)>(state_id).unwrap_or((false, true)));
    Row::new(Gap::Inline).show(ui, |ui| {
        ui.add(ToggleButton::new(label, &mut snap).size(size).enabled(enabled).id_salt("toggle-button-snap"));
        ui.add(ToggleButton::new("Guides", &mut guides).icon(Icon::Eye).size(size).enabled(enabled).id_salt("toggle-button-guides"));
    });
    ui.data_mut(|d| d.insert_temp(state_id, (snap, guides)));
}
