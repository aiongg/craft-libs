//! [`IconButton`]: a square ghost button that shows only an icon.

use craft_theme::{ButtonPart, ButtonVariant, ControlSize, Srgba, States};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{AsIdSalt, Color32, CornerRadius, Id, IdSalt, Rect, Response, Sense, Stroke, Ui, Widget, pos2, vec2};

use super::TooltipExt as _;
use super::button::content_color;
use super::{SizeDisabledArgs, salt};
use crate::direction;
use crate::icons::{self, Icon, IconSize};
use crate::primitives::{Content, Corner, Gap, Interactive, Row, StateColors, Text};
use crate::story::{Level, SizeArg, Story, StoryArgs};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// An icon button's sizes and colours, from [`IconButton::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct IconButtonStyle {
    /// Side of the square: `icon_button.size`.
    pub side: f32,
    /// Side of the icon: `icon_button.icon`.
    pub icon: f32,
    /// `Corner::Control`: `radius.control`.
    pub radius: CornerRadius,
    /// Off: a ghost button's fill, `button.ghost.fill`.
    pub fill: States<Color32>,
    /// Off: `button.ghost.text`.
    pub text: States<Color32>,
    /// Toggled on: `icon_button.selected.fill`, held in every state but disabled.
    pub selected_fill: States<Color32>,
    /// Toggled on: the 1 pt outline inside the edge, `icon_button.selected.border`.
    pub selected_border: States<Color32>,
    /// Toggled on: `icon_button.selected.text`.
    pub selected_text: States<Color32>,
    /// Width of the toggled-on outline in points; `None`: the surface's, one point.
    pub selected_border_width: Option<f32>,
    /// Diameter of the badge dot: `icon_button.badge`.
    pub badge: f32,
    /// The ring around the badge dot: `line.mark`.
    pub badge_edge: Stroke,
}

/// A square button showing only an icon: `ui.add(IconButton::new(Icon::Eye, "Show layer"))`.
///
/// It looks like a ghost button: transparent until hovered, no edge. The required `label` is its
/// accessible name and its hover tooltip, since the icon alone names nothing to a screen reader
/// and may be unclear to anyone. [`toggle`](IconButton::toggle) makes it a toggle button: on, it
/// shows `accent.soft` inside a 1 pt `accent.border` outline and tints the icon `accent.text`, and
/// AccessKit reports it toggled. [`selected`](IconButton::selected) shows the same look for a state the host owns.
#[derive(Debug)]
pub struct IconButton<'a> {
    icon: Icon,
    label: String,
    size: ControlSize,
    on: Option<&'a mut bool>,
    selected: Option<bool>,
    show_tooltip: bool,
    shortcut: Option<String>,
    badge: Option<Srgba>,
    enabled: bool,
    focusable: bool,
    id_salt: Option<IdSalt>,
    style: Option<IconButtonStyle>,
    id: Option<Id>,
}

impl<'a> IconButton<'a> {
    pub fn new(icon: Icon, label: impl Into<String>) -> Self {
        Self {
            icon,
            label: label.into(),
            size: ControlSize::Md,
            on: None,
            selected: None,
            show_tooltip: true,
            shortcut: None,
            badge: None,
            enabled: true,
            focusable: true,
            id_salt: None,
            style: None,
            id: None,
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A toggle: a click flips `on`, and the button reports the new state in the same frame.
    pub fn toggle(mut self, on: &'a mut bool) -> Self {
        self.on = Some(on);
        self
    }

    /// Shows the selected look when `on`. The host owns the state: a click is reported in the
    /// response and changes nothing here. Not valid together with [`toggle`](IconButton::toggle).
    pub fn selected(mut self, on: bool) -> Self {
        self.selected = Some(on);
        self
    }

    /// Whether the hover tooltip shows (default true). The label stays the accessible name and
    /// the screen reader description.
    pub fn show_tooltip(mut self, show: bool) -> Self {
        self.show_tooltip = show;
        self
    }

    /// The key that selects the same thing elsewhere ("B"). The tooltip reads "Label  (B)"; the
    /// label stays the accessible name and the shortcut is the description.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        let shortcut = shortcut.into();
        self.shortcut = (!shortcut.is_empty()).then_some(shortcut);
        self
    }

    /// A small dot of `color` at the end-bottom corner of the icon, ringed in `line.mark`
    /// so it shows on any fill. The colour is the user's content (an eyedropper's black, grey or
    /// white point), so it has no contrast pair. The label names the button, not the dot.
    pub fn badge(mut self, color: impl Into<Srgba>) -> Self {
        self.badge = Some(color.into());
        self
    }

    /// Whether it takes input; disabled, it draws the disabled colours and keeps its tooltip.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Whether it takes keyboard focus (a Tab stop); on by default. Off for a button that repeats
    /// a key of the control it sits in, as a search field's clear button repeats Esc.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// Keeps focus and animations when the layout around it changes; else the `Ui`'s next
    /// automatic id.
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    /// The exact widget id, wherever the button is placed, for a host that needs it to move
    /// keyboard focus. Takes the place of [`id_salt`](IconButton::id_salt).
    pub fn with_id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Draws with `style` in place of [`IconButton::style`], for a component whose buttons take
    /// their colours and corners from its own tokens (a rail). The size class then only picks the
    /// badge and the layout side of [`show`](Widget::ui); [`show_at`](IconButton::show_at) fits
    /// the rect it is given.
    pub fn with_style(mut self, style: IconButtonStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn style(tokens: &UiTokens, size: ControlSize) -> IconButtonStyle {
        let c = &tokens.component;
        let ghost = c.button.colors(ButtonVariant::Ghost);
        let selected = &c.icon_button.selected;
        let held = |part| {
            let on = selected.on(part).to_egui();
            States::new(on, on, on, selected.on_disabled(part).to_egui())
        };
        IconButtonStyle {
            side: c.icon_button.size.get(size),
            icon: c.icon_button.icon.get(size),
            radius: Corner::Control.radius(tokens),
            fill: ghost.fill.to_egui(),
            text: ghost.text.to_egui(),
            selected_fill: held(ButtonPart::Fill),
            selected_border: held(ButtonPart::Border),
            selected_text: held(ButtonPart::Text),
            selected_border_width: None,
            badge: c.icon_button.badge.get(size),
            badge_edge: Stroke::new(craft_theme::Stroke::W1.width(1.0), tokens.color.line.mark.to_egui()),
        }
    }
}

impl IconButton<'_> {
    /// Shows the button over `rect` inside a control's own rect, without allocating it in the
    /// layout ([`Interactive::interact_at`]).
    pub fn show_at(self, ui: &mut Ui, rect: Rect) -> Response {
        self.show_in(ui, Some(rect))
    }

    fn show_in(self, ui: &mut Ui, rect: Option<Rect>) -> Response {
        let style = self.style.unwrap_or_else(|| IconButton::style(&Tokens::of(ui), self.size));
        debug_assert!(!(self.selected.is_some() && self.on.is_some()), "IconButton: `selected` and `toggle` are exclusive");
        let mut on = self.on;
        let sense = if self.focusable { Sense::click() } else { Sense::CLICK };
        let mut element = match self.id {
            Some(id) => Interactive::with_id(id, Role::Button, self.label.as_str()),
            None => Interactive::salted(salt(ui, self.id_salt), Role::Button, self.label.as_str()),
        }
        .sense(sense)
        .colors(StateColors { fill: style.fill, border: None })
        .on_colors(StateColors { fill: style.selected_fill, border: Some(style.selected_border) })
        .outline()
        .corner_radius(style.radius)
        .enabled(self.enabled);
        if let Some(width) = style.selected_border_width {
            element = element.border_width(width);
        }
        if let Some(on) = on.as_deref_mut() {
            element = element.toggle(on);
        } else if let Some(selected) = self.selected {
            element = element.checked(selected);
        }
        let response = match rect {
            Some(rect) => element.interact_at(ui, rect).finish(ui),
            None => element.show(ui, vec2(style.side, style.side)),
        };
        // The state after this frame's click, which the fill shows too.
        let on = self.selected.unwrap_or(false) || on.is_some_and(|on| *on);
        let color = content_color(ui, if on { &style.selected_text } else { &style.text }, &response, true);
        if ui.is_rect_visible(response.rect) {
            let rect = Rect::from_center_size(response.rect.center(), vec2(style.icon, style.icon)).round_to_pixels(ui.pixels_per_point());
            icons::paint_in(ui, rect, self.icon, icons::active(ui.ctx()), style.icon, IconSize::Sm.stroke(), color);
            if let Some(badge) = self.badge {
                let r = style.badge / 2.0;
                let center = pos2(direction::end_x(ui, rect) - direction::forward(ui) * r, rect.bottom() - r);
                ui.painter().circle(center, r, Color32::from(badge), style.badge_edge);
            }
        }
        let tip = match &self.shortcut {
            Some(shortcut) => {
                if response.ctx.has_accesskit_node(response.id) {
                    response.ctx.accesskit_node_builder(response.id, |node| node.set_description(shortcut.as_str()));
                }
                format!("{}  ({shortcut})", self.label)
            }
            None => self.label.clone(),
        };
        if self.show_tooltip {
            response.tooltip(tip)
        } else {
            response.describe_for_accessibility(&tip);
            response
        }
    }
}

impl Widget for IconButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show_in(ui, None)
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Icon button"];

const SAMPLES: [(Icon, &str); 5] =
    [(Icon::Undo, "Undo"), (Icon::Redo, "Redo"), (Icon::Copy, "Copy"), (Icon::Trash, "Delete"), (Icon::Settings, "Settings")];

#[derive(StoryArgs, Debug, PartialEq)]
struct PlainArgs {
    size: SizeArg,
    disabled: bool,
    #[arg(name = "show tooltip")]
    show_tooltip: bool,
    badge: bool,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "controls/icon-button/plain",
            Level::Controls,
            "Icon button",
            "Plain",
            PlainArgs { size: SizeArg::Md, disabled: false, show_tooltip: true, badge: false },
            show_plain
        ),
        typed_story!("controls/icon-button/toggle", Level::Controls, "Icon button", "Toggle", SizeDisabledArgs::default(), show_toggle),
    ]
}

/// A toolbar row of icon buttons; clicks count up. A second row is a tool strip whose selected
/// button the host owns.
fn show_plain(ui: &mut Ui, args: &mut PlainArgs) {
    let size = args.size.control();
    let enabled = !args.disabled;
    let (tooltip, badge) = (args.show_tooltip, args.badge);
    let state_id = ui.scope_id().with("icon-button-plain-count");
    let (mut count, mut tool) = ui.data(|d| d.get_temp::<(u32, usize)>(state_id).unwrap_or_default());
    Row::new(Gap::Inline).show(ui, |ui| {
        for (icon, label) in SAMPLES {
            let mut button = IconButton::new(icon, label).size(size).enabled(enabled).show_tooltip(tooltip).id_salt(("icon-button-plain", label));
            if badge {
                button = button.badge(Srgba::new(0.9, 0.2, 0.2, 1.0));
            }
            if ui.add(button).clicked() {
                count += 1;
            }
        }
    });
    ui.add(Text::new(format!("Clicked {count} times")).content(Content::Muted));
    Row::new(Gap::Inline).show(ui, |ui| {
        for (i, (icon, label)) in SAMPLES.into_iter().enumerate() {
            let button = IconButton::new(icon, label).size(size).enabled(enabled).selected(tool == i).show_tooltip(tooltip);
            if ui.add(button.id_salt(("icon-button-tool", label))).clicked() {
                tool = i;
            }
        }
    });
    ui.add(Text::new(format!("Selected: {}", SAMPLES[tool].1)).content(Content::Muted));
    ui.data_mut(|d| d.insert_temp(state_id, (count, tool)));
}

/// Visibility and lock toggles, one of each on.
fn show_toggle(ui: &mut Ui, args: &mut SizeDisabledArgs) {
    let size = args.size.control();
    let enabled = !args.disabled;
    let state_id = ui.scope_id().with("icon-button-toggle-state");
    let (mut visible, mut locked) = ui.data(|d| d.get_temp::<(bool, bool)>(state_id).unwrap_or((true, false)));
    Row::new(Gap::Inline).show(ui, |ui| {
        let eye = if visible { Icon::Eye } else { Icon::EyeOff };
        ui.add(IconButton::new(eye, "Visible").size(size).toggle(&mut visible).enabled(enabled).id_salt("icon-button-visible"));
        let lock = if locked { Icon::Lock } else { Icon::LockOpen };
        ui.add(IconButton::new(lock, "Locked").size(size).toggle(&mut locked).enabled(enabled).id_salt("icon-button-locked"));
    });
    ui.data_mut(|d| d.insert_temp(state_id, (visible, locked)));
}
