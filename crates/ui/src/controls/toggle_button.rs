//! [`ToggleButton`]: a button that stays pressed while on.

use craft_theme::{ButtonVariant, ControlSize, Rgba, States, TypeRole};
use egui::accesskit::Role;
use egui::{AsIdSalt, Color32, IdSalt, Response, Ui, Widget};

use super::button::{ButtonStyle, Face, content_color};
use super::common_stories::ControlStoryLabels;
use super::salt;
use crate::icons::Icon;
use crate::localized::Labels as _;
use crate::primitives::{Corner, Gap, Interactive, Row, StateColors, SurfaceKind};
use crate::story::{Level, SizeArg, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// A toggle button's look, from [`ToggleButton::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToggleButtonStyle {
    /// Off: a secondary button.
    pub off: ButtonStyle,
    /// On: the accent colours, held in every state but disabled, and
    /// the disabled colours when disabled.
    pub on_fill: States<Color32>,
    #[allow(missing_docs)]
    pub on_border: States<Color32>,
    #[allow(missing_docs)]
    pub on_text: States<Color32>,
}

/// A secondary-looking button that stays pressed while on:
/// `ui.add(ToggleButton::new("Snap", &mut snap))`.
///
/// On, it draws the accent colours and its raised surface draws
/// pressed in, so a bevel theme shows it sunk. Disabled while on, it draws
/// its disabled colours, which still looks on. A click flips the value; AccessKit reports
/// the button toggled, after the click, in the same frame.
#[derive(Debug)]
pub struct ToggleButton<'a> {
    label: String,
    on: &'a mut bool,
    size: ControlSize,
    icon: Option<Icon>,
    trailing: Option<Icon>,
    role: Option<TypeRole>,
    enabled: bool,
    id_salt: Option<IdSalt>,
}

impl<'a> ToggleButton<'a> {
    /// A button named `label` that edits `on`.
    pub fn new(label: impl Into<String>, on: &'a mut bool) -> Self {
        Self { label: label.into(), on, size: ControlSize::Md, icon: None, trailing: None, role: None, enabled: true, id_salt: None }
    }

    /// Sets the control size (default [`ControlSize::Md`]).
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A leading icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// An icon at the reading end, mirrored right to left.
    pub fn trailing_icon(mut self, icon: Icon) -> Self {
        self.trailing = Some(icon);
        self
    }

    /// The type role of the label, for a path or a value in `Mono`; `button.label` by default.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Whether the button takes input (default `true`).
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

    /// The button's sizes and colours at `size` from `tokens`.
    pub fn style(tokens: &UiTokens, size: ControlSize) -> ToggleButtonStyle {
        let off = super::Button::style(tokens, ButtonVariant::Secondary, size);
        let on = on_colors(tokens);
        ToggleButtonStyle { off, on_fill: on.fill, on_border: on.border, on_text: on.text }
    }
}

/// The colours of a button that is on, per state: shared with the icon button.
pub(crate) struct OnColors {
    pub fill: States<Color32>,
    pub border: States<Color32>,
    pub text: States<Color32>,
}

/// `accent.soft` inside an `accent.border` outline with `accent.text`, the same in every state
/// but disabled, which keeps the fill and drops the outline and text to the disabled colours.
pub(crate) fn on_colors(tokens: &UiTokens) -> OnColors {
    let c = &tokens.color;
    let held = |on: Rgba, disabled: Rgba| {
        let (on, disabled) = (on.to_egui(), disabled.to_egui());
        States::new(on, on, on, disabled)
    };
    OnColors {
        fill: held(c.accent.soft, c.accent.soft),
        border: held(c.accent.border, c.line.divider),
        text: held(c.accent.text, c.text.disabled),
    }
}

impl Widget for ToggleButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let mut style = ToggleButton::style(&tokens, self.size);
        if let Some(role) = self.role {
            style.off.label = role;
        }
        let face = Face { label: &self.label, icon: self.icon, trailing: self.trailing, size: self.size, loading: false };
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
pub const COMPONENTS: &[&str] = &["ToggleButton"];

#[derive(StoryArgs, Debug, PartialEq)]
struct ToggleArgs {
    size: SizeArg,
    disabled: bool,
    label: String,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!(
        "controls/toggle-button/toggle",
        Level::Controls,
        "ToggleButton",
        "Toggle",
        ToggleArgs { size: SizeArg::Md, disabled: false, label: "Snap to grid".into() },
        show
    )]
}

/// One toggle off and one on, with and without an icon.
fn show(ui: &mut Ui, args: &mut ToggleArgs) {
    let size = args.size.control();
    let enabled = !args.disabled;
    let label = args.label.clone();
    let state_id = ui.scope_id().with("toggle-button-state");
    let (mut snap, mut guides) = ui.data(|d| d.get_temp::<(bool, bool)>(state_id).unwrap_or((false, true)));
    Row::new(Gap::Inline).show(ui, |ui| {
        ui.add(ToggleButton::new(label, &mut snap).size(size).enabled(enabled).id_salt("toggle-button-snap"));
        ui.add(
            ToggleButton::new(ControlStoryLabels::of(ui).guides.as_str(), &mut guides)
                .icon(Icon::Eye)
                .size(size)
                .enabled(enabled)
                .id_salt("toggle-button-guides"),
        );
    });
    ui.data_mut(|d| d.insert_temp(state_id, (snap, guides)));
}
