//! [`Tooltip`] and [`TooltipExt`]: tooltips with a surface of their own.

use egui::text::LayoutJob;
use egui::{Frame, KeyboardShortcut, Label, Margin, Response, Shadow, Stroke, Ui};

use super::Kbd;
use crate::direction;
use crate::primitives::{Content, Gap, Row, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, TokensOf, UiTokens, apply_popup_style, corner_radius, current, stroke};
use craft_theme::TypeRole;

/// A tooltip's frame and text, from [`Tooltip::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TooltipStyle {
    /// Widest the text grows: `tooltip.max_width`.
    pub max_width: f32,
    /// Between the frame and the text: `space.pad.control`.
    pub margin: Margin,
    /// `tooltip.radius`: `shape.radius.popup`.
    pub radius: egui::CornerRadius,
    /// The popup shadow.
    pub shadow: Shadow,
    /// `tooltip.fill`: `surface.panel`.
    pub fill: egui::Color32,
    /// `tooltip.border`, one point wide: `line.border`.
    pub border: Stroke,
    /// `tooltip.text`, in the body type role: `text.normal`.
    pub text: egui::Color32,
}

impl TooltipStyle {
    /// The tooltip's frame.
    pub fn frame(&self) -> Frame {
        Frame::new().inner_margin(self.margin).corner_radius(self.radius).shadow(self.shadow).fill(self.fill).stroke(self.border)
    }
}

/// The look of every tooltip the [`TooltipExt`] helpers show: the popup surface and the body type
/// role by default, with the `tooltip` tokens (fill, text, border, radius) setting a look of their
/// own. egui draws a tooltip with the `Style` shared with popups and menus unless the caller gives
/// the popup a frame, so the helpers are the one place tooltips get that frame: use them, not
/// egui's `on_hover_text`, `on_hover_ui` and `show_tooltip_text`. A tooltip shows after the
/// pointer rests [`Tooltip::DELAY`] on its control.
#[derive(Clone, Copy, Debug)]
pub struct Tooltip;

impl Tooltip {
    /// Seconds the pointer rests on a control before its tooltip shows, as in PhotoCraft:
    /// `interaction.tooltip_delay` in [`style_for`](crate::theme::style_for).
    pub const DELAY: f32 = 0.35;

    pub fn style(tokens: &UiTokens) -> TooltipStyle {
        let t = &tokens.component.tooltip;
        let popup = Frame::popup(&tokens.style);
        TooltipStyle {
            max_width: t.max_width,
            margin: popup.inner_margin,
            radius: corner_radius(t.radius),
            shadow: popup.shadow,
            fill: t.fill.to_egui(),
            border: stroke(1.0, t.border),
            text: t.text.to_egui(),
        }
    }
}

/// Tooltips for a [`Response`], enabled or disabled: a disabled control keeps its tooltip.
///
/// Egui's timing and placement apply. Each helper also describes the widget for a screen reader
/// (the tooltip text is its description, and its name when it has none, as egui's
/// `on_hover_text` does); [`tooltip_with`](TooltipExt::tooltip_with) gives it the keyboard
/// shortcut as well.
pub trait TooltipExt: Sized {
    /// A tooltip with `text`.
    fn tooltip(self, text: impl Into<String>) -> Self;
    /// A tooltip with `text` and its keyboard shortcut after it in reading order, in a [`Kbd`].
    fn tooltip_with(self, text: impl Into<String>, shortcut: &KeyboardShortcut) -> Self;
    /// A tooltip with any contents.
    fn tooltip_ui(self, add_contents: impl FnOnce(&mut Ui)) -> Self;
    /// A tooltip with `text` next to the pointer instead of the control.
    fn tooltip_at_pointer(self, text: impl Into<String>) -> Self;
    /// A tooltip with any contents next to the pointer instead of the control.
    fn tooltip_ui_at_pointer(self, add_contents: impl FnOnce(&mut Ui)) -> Self;
    /// Shows a tooltip with `text` now, whether or not the pointer rests on the control: for a
    /// message about the control's state, such as why its input was refused.
    fn show_tooltip(&self, text: impl Into<String>);
    /// Shows a tooltip with any contents now, whether or not the pointer rests on the control.
    fn show_tooltip_ui(&self, add_contents: impl FnOnce(&mut Ui));
}

impl TooltipExt for Response {
    fn tooltip(self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.describe_for_accessibility(&text);
        self.tooltip_ui(|ui| {
            tip_text(ui, text);
        })
    }

    fn tooltip_with(self, text: impl Into<String>, shortcut: &KeyboardShortcut) -> Self {
        let text = text.into();
        let keys = Kbd::new(shortcut).spoken(self.ctx.os());
        self.describe_for_accessibility(&text);
        set_shortcut(&self, &keys);
        self.tooltip_ui(|ui| {
            ui.with_layout(direction::row(ui).with_main_wrap(true), |ui| {
                tip_text(ui, text);
                ui.add(Kbd::new(shortcut));
            });
        })
    }

    fn tooltip_ui(self, add_contents: impl FnOnce(&mut Ui)) -> Self {
        let tooltip = if self.enabled() { egui::Tooltip::for_enabled(&self) } else { egui::Tooltip::for_disabled(&self) };
        show_on(&self, tooltip, add_contents);
        self
    }

    fn tooltip_at_pointer(self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.describe_for_accessibility(&text);
        self.tooltip_ui_at_pointer(|ui| {
            tip_text(ui, text);
        })
    }

    fn tooltip_ui_at_pointer(self, add_contents: impl FnOnce(&mut Ui)) -> Self {
        let tooltip = if self.enabled() { egui::Tooltip::for_enabled(&self) } else { egui::Tooltip::for_disabled(&self) };
        show_on(&self, tooltip.at_pointer().gap(POINTER_GAP), add_contents);
        self
    }

    fn show_tooltip(&self, text: impl Into<String>) {
        let text = text.into();
        self.show_tooltip_ui(|ui| tip_text(ui, text));
    }

    fn show_tooltip_ui(&self, add_contents: impl FnOnce(&mut Ui)) {
        let style = Tooltip::style(&current(&self.ctx));
        egui::Popup::from_response(self).kind(egui::PopupKind::Tooltip).frame(style.frame()).show(|ui| {
            contents(ui, &style, add_contents);
        });
    }
}

/// Points between the pointer and a tooltip placed at it, as egui's `on_hover_ui_at_pointer`.
const POINTER_GAP: f32 = 12.0;

/// Shows `tooltip` for `response` with the frame and contents style of the tokens in effect.
fn show_on(response: &Response, mut tooltip: egui::Tooltip<'_>, add_contents: impl FnOnce(&mut Ui)) {
    let style = Tooltip::style(&current(&response.ctx));
    tooltip.popup = tooltip.popup.frame(style.frame());
    tooltip.show(|ui| contents(ui, &style, add_contents));
}

/// The tooltip's contents in the scope's style, with `style.text` as the colour of egui labels.
fn contents(ui: &mut Ui, style: &TooltipStyle, add_contents: impl FnOnce(&mut Ui)) {
    apply_popup_style(ui);
    ui.visuals_mut().override_text_color = Some(style.text);
    // Stops the area shrinking tooltips whose contents change, as egui's text tooltips do.
    ui.set_max_width(ui.spacing().tooltip_width);
    add_contents(ui);
}

/// Body text in the tooltip's text colour.
fn tip_text(ui: &mut Ui, text: String) {
    let tokens = crate::theme::Tokens::of(ui);
    let mut format = Text::format(&tokens, TypeRole::Body, Content::Normal);
    format.color = tokens.component.tooltip.text.to_egui();
    format.valign = ui.text_valign();
    ui.add(Label::new(LayoutJob::single_section(text, format)));
}

/// Sets the keyboard `shortcut`, in its spoken names, on a widget that already has a node.
fn set_shortcut(response: &Response, shortcut: &str) {
    let ctx = &response.ctx;
    if ctx.has_accesskit_node(response.id) {
        ctx.accesskit_node_builder(response.id, |node| node.set_keyboard_shortcut(shortcut));
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Tooltip"];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "controls/tooltip/text",
            level: Level::Controls,
            component: "Tooltip",
            name: "Text",
            args: || Args::new().with_text("tip", "Moves the selection to the next layer"),
            show: show_text,
        },
        Story {
            id: "controls/tooltip/shortcut",
            level: Level::Controls,
            component: "Tooltip",
            name: "Shortcut",
            args: || Args::new(),
            show: show_shortcut,
        },
        Story {
            id: "controls/tooltip/custom",
            level: Level::Controls,
            component: "Tooltip",
            name: "Custom",
            args: || Args::new(),
            show: show_custom,
        },
    ]
}

/// Hover targets: an enabled and a disabled button, and a stock egui tooltip beside them, which
/// keeps the popup look.
fn show_text(ui: &mut Ui, args: &mut Args) {
    use super::Button;
    let tip = args.text("tip").to_owned();
    Row::new(Gap::Inline).show(ui, |ui| {
        ui.add(Button::new("Next layer").id_salt("tooltip-text-enabled")).tooltip(&tip);
        ui.add(Button::new("Disabled").enabled(false).id_salt("tooltip-text-disabled")).tooltip(&tip);
        ui.add(Button::new("Stock egui").id_salt("tooltip-text-stock")).on_hover_text(&tip);
    });
}

fn show_shortcut(ui: &mut Ui, _: &mut Args) {
    use super::Button;
    use egui::{Key, Modifiers};
    Row::new(Gap::Inline).show(ui, |ui| {
        let undo = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
        let redo = KeyboardShortcut::new(Modifiers { command: true, shift: true, ..Modifiers::NONE }, Key::Z);
        ui.add(Button::new("Undo").id_salt("tooltip-undo")).tooltip_with("Undo the last change", &undo);
        ui.add(Button::new("Redo").id_salt("tooltip-redo")).tooltip_with("Redo the change you undid", &redo);
    });
}

fn show_custom(ui: &mut Ui, _: &mut Args) {
    use super::Button;
    ui.add(Button::new("Colour").id_salt("tooltip-custom")).tooltip_ui(|ui| {
        ui.add(Text::new("Document colour").role(craft_theme::TypeRole::BodyStrong));
        ui.add(Text::new("Shown in the sRGB space").content(Content::Muted));
    });
}
