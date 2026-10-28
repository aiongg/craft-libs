//! [`Tooltip`] and [`TooltipExt`]: tooltips on the popup surface.

use egui::accesskit::Role;
use egui::{Frame, KeyboardShortcut, Margin, Response, Shadow, Stroke, Ui};

use super::Kbd;
use crate::primitives::{Content, Gap, Row, Text};
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, UiTokens};

/// A tooltip's frame and text, from [`Tooltip::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TooltipStyle {
    /// Widest the text grows: `tooltip.max_width`.
    pub max_width: f32,
    /// Between the frame and the text: `space.pad.control`.
    pub margin: Margin,
    /// `Corner::Popup`: `shape.radius.popup`.
    pub radius: egui::CornerRadius,
    /// The popup shadow.
    pub shadow: Shadow,
    /// The popup surface: `surface.raised`.
    pub fill: egui::Color32,
    /// `line.border`, one point wide.
    pub border: Stroke,
    /// `text.normal`, in the body type role.
    pub text: egui::Color32,
}

/// The look of every tooltip, stock egui's `on_hover_text` and the [`TooltipExt`] helpers alike:
/// the popup surface with the body type role and `text.normal`. egui draws tooltips with the
/// `Style` that [`style_for`](crate::theme::style_for) generates, which popups and menus share, so
/// this look is theirs and a tooltip has no colours or shape of its own.
#[derive(Clone, Copy, Debug)]
pub struct Tooltip;

impl Tooltip {
    pub fn style(tokens: &UiTokens) -> TooltipStyle {
        let frame = Frame::popup(&tokens.style);
        TooltipStyle {
            max_width: tokens.component.tooltip.max_width,
            margin: frame.inner_margin,
            radius: frame.corner_radius,
            shadow: frame.shadow,
            fill: frame.fill,
            border: frame.stroke,
            text: tokens.color.text.normal.to_egui(),
        }
    }
}

/// Tooltips for a [`Response`], enabled or disabled: a disabled control keeps its tooltip.
///
/// Egui's timing and placement apply. Each helper also describes the widget for a screen reader
/// (the tooltip text is its description, and its name when it has none, as egui's
/// `on_hover_text` does).
pub trait TooltipExt: Sized {
    /// A tooltip with `text`.
    fn tooltip(self, text: impl Into<String>) -> Self;
    /// A tooltip with `text` and its keyboard shortcut after it in a [`Kbd`].
    fn tooltip_with(self, text: impl Into<String>, shortcut: &KeyboardShortcut) -> Self;
    /// A tooltip with any contents.
    fn tooltip_ui(self, add_contents: impl FnOnce(&mut Ui)) -> Self;
}

impl TooltipExt for Response {
    fn tooltip(self, text: impl Into<String>) -> Self {
        let text = text.into();
        describe(&self, &text);
        self.tooltip_ui(|ui| {
            ui.add(Text::new(text));
        })
    }

    fn tooltip_with(self, text: impl Into<String>, shortcut: &KeyboardShortcut) -> Self {
        let text = text.into();
        let spoken = format!("{text} ({})", shortcut.format(&egui::ModifierNames::NAMES, super::kbd::is_mac(self.ctx.os())));
        describe(&self, &spoken);
        self.tooltip_ui(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add(Text::new(text));
                ui.add(Kbd::new(shortcut));
            });
        })
    }

    fn tooltip_ui(self, add_contents: impl FnOnce(&mut Ui)) -> Self {
        let add = |ui: &mut Ui| {
            // Stops the area shrinking tooltips whose contents change, as egui's text tooltips do.
            ui.set_max_width(ui.spacing().tooltip_width);
            add_contents(ui);
        };
        if self.enabled() { self.on_hover_ui(add) } else { self.on_disabled_hover_ui(add) }
    }
}

/// The description egui's `on_hover_text` gives: only for a widget that already has a node.
fn describe(response: &Response, text: &str) {
    let ctx = &response.ctx;
    if ctx.has_accesskit_node(response.id) {
        ctx.accesskit_node_builder(response.id, |node| {
            if node.description().is_none() {
                node.set_description(text);
            }
            if node.role() != Role::Label && node.labelled_by().is_empty() && node.label().is_none_or(str::is_empty) {
                node.set_label(text);
            }
        });
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

/// Hover targets: an enabled and a disabled button, and a stock egui tooltip beside them.
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
