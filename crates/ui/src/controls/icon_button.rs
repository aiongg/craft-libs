//! [`IconButton`]: a square ghost button that shows only an icon.

use craft_theme::{ButtonPart, ButtonVariant, ControlSize, Srgba, States, Tone};
use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{AsIdSalt, Color32, CornerRadius, Id, IdSalt, Image, ImageSource, Rect, Response, Sense, Stroke, Ui, Widget, pos2, vec2};

use super::TooltipExt as _;
use super::button::content_color;
use super::{SizeDisabledArgs, salt};
use crate::direction;
use crate::icons::{self, Icon, IconSize};
use crate::primitives::{Content, Corner, Gap, Interactive, Row, StateColors, Text};
use crate::story::{Level, SizeArg, Story, StoryArgs, ToneArg};
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

/// The picture on an [`IconButton`]: a library [`Icon`] or an image the app supplies.
#[derive(Clone, Debug)]
pub enum IconSource<'a> {
    /// An icon of the active icon set, drawn with the set's stroke.
    Library(Icon),
    /// An app image, normally an SVG. It is tinted with the button's state colour, so it should
    /// be drawn in a single colour.
    Image(ImageSource<'a>),
}

impl From<Icon> for IconSource<'_> {
    fn from(icon: Icon) -> Self {
        Self::Library(icon)
    }
}

impl<'a> From<ImageSource<'a>> for IconSource<'a> {
    fn from(source: ImageSource<'a>) -> Self {
        Self::Image(source)
    }
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
    icon: IconSource<'a>,
    label: String,
    size: ControlSize,
    icon_size: Option<f32>,
    side: Option<f32>,
    on: Option<&'a mut bool>,
    selected: Option<bool>,
    show_tooltip: bool,
    shortcut: Option<String>,
    badge: Option<Srgba>,
    tone: Tone,
    enabled: bool,
    focusable: bool,
    id_salt: Option<IdSalt>,
    style: Option<IconButtonStyle>,
    id: Option<Id>,
}

impl<'a> IconButton<'a> {
    /// `icon` is a library [`Icon`] or an app image ([`IconSource`]).
    pub fn new(icon: impl Into<IconSource<'a>>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            size: ControlSize::Md,
            icon_size: None,
            side: None,
            on: None,
            selected: None,
            show_tooltip: true,
            shortcut: None,
            badge: None,
            tone: Tone::Neutral,
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

    /// The icon's side in points, in place of the size class's. The button keeps its own side, so
    /// the icon can differ from the hit area.
    pub fn icon_size(mut self, points: f32) -> Self {
        self.icon_size = Some(points);
        self
    }

    /// The button's side in points, in place of the size class's. The icon keeps its side.
    pub fn side(mut self, points: f32) -> Self {
        self.side = Some(points);
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

    /// Draws the icon in the tone's status colour (`notice.<tone>.fg`, the colour a notice of the
    /// tone has), as for a warning the button reports. Neutral, the default, is the button's own
    /// colour; disabled, the icon is dimmed as usual.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
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
        let tokens = Tokens::of(ui);
        let mut style = self.style.unwrap_or_else(|| IconButton::style(&tokens, self.size));
        style.side = self.side.unwrap_or(style.side);
        style.icon = self.icon_size.unwrap_or(style.icon);
        if self.tone != Tone::Neutral {
            let fg = tokens.component.notice.fg(self.tone).to_egui();
            style.text = States::new(fg, fg, fg, style.text.disabled);
            style.selected_text = States::new(fg, fg, fg, style.selected_text.disabled);
        }
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
            match self.icon {
                IconSource::Library(icon) => icons::paint_in(ui, rect, icon, icons::active(ui.ctx()), style.icon, IconSize::Sm.stroke(), color),
                IconSource::Image(source) => Image::new(source).tint(color).paint_at(ui, rect),
            }
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
    tone: ToneArg,
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
            PlainArgs { size: SizeArg::Md, tone: ToneArg::Neutral, disabled: false, show_tooltip: true, badge: false },
            show_plain
        ),
        typed_story!("controls/icon-button/app-icon", Level::Controls, "Icon button", "App icon", SizeDisabledArgs::default(), show_app_icon),
        typed_story!("controls/icon-button/toggle", Level::Controls, "Icon button", "Toggle", SizeDisabledArgs::default(), show_toggle),
    ]
}

/// A toolbar row of icon buttons; clicks count up. A second row is a tool strip whose selected
/// button the host owns.
fn show_plain(ui: &mut Ui, args: &mut PlainArgs) {
    let size = args.size.control();
    let enabled = !args.disabled;
    let (tooltip, badge, tone) = (args.show_tooltip, args.badge, args.tone.tone());
    let state_id = ui.scope_id().with("icon-button-plain-count");
    let (mut count, mut tool) = ui.data(|d| d.get_temp::<(u32, usize)>(state_id).unwrap_or_default());
    Row::new(Gap::Inline).show(ui, |ui| {
        for (icon, label) in SAMPLES {
            let mut button =
                IconButton::new(icon, label).size(size).tone(tone).enabled(enabled).show_tooltip(tooltip).id_salt(("icon-button-plain", label));
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
            let button = IconButton::new(icon, label).size(size).tone(tone).enabled(enabled).selected(tool == i).show_tooltip(tooltip);
            if ui.add(button.id_salt(("icon-button-tool", label))).clicked() {
                tool = i;
            }
        }
    });
    ui.add(Text::new(format!("Selected: {}", SAMPLES[tool].1)).content(Content::Muted));
    ui.data_mut(|d| d.insert_temp(state_id, (count, tool)));
}

const APP_ICON: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="2"><path d="M4 4h16v16H4z"/><path d="M4 14l5-5 4 4 3-3 4 4"/></svg>"##;

/// An app's own SVG on a 24 pt button and on a 34 pt button with a 20 pt icon; the second is a
/// tool strip button the host owns.
fn show_app_icon(ui: &mut Ui, args: &mut SizeDisabledArgs) {
    let enabled = !args.disabled;
    let source = || ImageSource::Bytes { uri: "bytes://craft-ui-story/app-icon.svg".into(), bytes: egui::load::Bytes::Static(APP_ICON) };
    let state_id = ui.scope_id().with("icon-button-app-icon-selected");
    let mut selected = ui.data(|d| d.get_temp::<bool>(state_id).unwrap_or(true));
    Row::new(Gap::Inline).show(ui, |ui| {
        let small = IconButton::new(source(), "Image").side(24.0).icon_size(14.0).enabled(enabled).id_salt("icon-button-app-small");
        ui.add(small);
        let large =
            IconButton::new(source(), "Image").side(34.0).icon_size(20.0).selected(selected).enabled(enabled).id_salt("icon-button-app-large");
        if ui.add(large).clicked() {
            selected = !selected;
        }
    });
    ui.data_mut(|d| d.insert_temp(state_id, selected));
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

#[cfg(test)]
mod tests {
    use egui::{Context, Pos2, RawInput, Rect};

    use super::*;
    use crate::theme::{Base, install};

    const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M2 2h12v12H2z"/></svg>"#;

    fn app_icon<'a>() -> ImageSource<'a> {
        ImageSource::Bytes { uri: "bytes://icon-button-test/box.svg".into(), bytes: egui::load::Bytes::Static(SVG) }
    }

    fn side_of(button: impl FnOnce() -> IconButton<'static>) -> Vec2 {
        let ctx = Context::default();
        install(&ctx, &Base::suite_default());
        let mut size = Vec2::ZERO;
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 200.0))), ..Default::default() };
        let mut button = Some(button);
        ctx.run_ui(input, |ui| size = ui.add(button.take().unwrap()()).rect.size()).textures_delta.clear();
        size
    }

    use egui::Vec2;

    #[test]
    fn an_app_icon_button_keeps_the_size_class_side_and_takes_a_free_side() {
        let md = side_of(|| IconButton::new(Icon::Eye, "Eye"));
        assert_eq!(side_of(|| IconButton::new(app_icon(), "Box")), md, "the source does not change the default side");
        assert_eq!(side_of(|| IconButton::new(app_icon(), "Box").icon_size(18.0)), md, "the icon size does not move the hit area");
        assert_eq!(side_of(|| IconButton::new(app_icon(), "Box").side(34.0).icon_size(18.0)), vec2(34.0, 34.0));
    }
}
