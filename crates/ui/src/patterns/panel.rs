//! `Panel`: the body of a docked panel and the floating panel over the canvas, drawn with the
//! `panel.*` tokens. This module holds the embedded panel; the floating panel shares its body
//! ([`Panel::show_body`]).

use egui::accesskit::{Orientation, Role};
use egui::{AsIdSalt, Frame, IdSalt, Margin, Rect, ScrollArea, Sense, Ui, UiBuilder, pos2, vec2};

use crate::composites::PanelFooter;
use crate::docking::DockLook;
use crate::icons::Icon;
use crate::story::{Args, Level, Story, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};

type FooterContent<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A panel's padding and fill, from [`Panel::style`]. The footer's own sizes are
/// [`PanelFooter::style`](crate::composites::PanelFooter::style).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PanelStyle {
    /// Padding round the body inside the scroll area: `panel.pad`.
    pub pad: f32,
    /// Fill of the panel when [`Panel::fill`] is on: `panel.fill`.
    pub fill: egui::Color32,
    /// Height of the footer, rule and bar, for a one-pixel-wide rule of `pixels_per_point`.
    pub footer_height: f32,
}

/// What [`Panel::show`] returns.
#[derive(Clone, Debug)]
pub struct PanelOutput<R> {
    /// The body closure's result.
    pub inner: R,
    /// The footer's rect, the rule and the bar, when the panel has a footer.
    pub footer: Option<Rect>,
}

/// A panel body: padding, a vertical scroll area and an optional footer fixed under it.
///
/// ```
/// use craft_ui::icons::Icon;
/// use craft_ui::patterns::Panel;
/// # egui::__run_test_ui(|ui| {
/// Panel::new("layers")
///     .footer_label("Layer actions")
///     .footer(|ui| {
///         craft_ui::composites::PanelFooter::button(ui, Icon::Trash, "Delete layer");
///     })
///     .show(ui, |ui| {
///         ui.label("Background");
///     });
/// # });
/// ```
///
/// - The panel takes the whole rect `ui` has left. Top to bottom: the scroll body, with `panel.pad`
///   on every side inside the scroll area (the scroll bar sits at the panel's edge), then the
///   footer. The body shrinks by the footer's height; the host reserves nothing.
/// - The footer is a [`PanelFooter`]: a rule in `panel.footer.line` from edge to edge and a bar of
///   `panel_footer.height` with `panel_footer.button` squares, the first at the reading end.
/// - **Keyboard and AccessKit:** the host's widgets are the Tab stops. With a title the panel is a
///   `Group` of that name; the footer is a horizontal `Toolbar` named by `footer_label`.
pub struct Panel<'a> {
    id_salt: IdSalt,
    title: Option<String>,
    scroll: bool,
    fill: bool,
    footer: Option<FooterContent<'a>>,
    footer_label: Option<String>,
    scroll_bar_label: String,
}

impl<'a> Panel<'a> {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            title: None,
            scroll: true,
            fill: false,
            footer: None,
            footer_label: None,
            scroll_bar_label: "Scroll bar".to_owned(),
        }
    }

    /// The panel's accessible name. An embedded panel shows no title: the dock's tab does.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Whether the body scrolls vertically (default on). Off for bodies that scroll themselves.
    #[must_use]
    pub fn scroll(mut self, scroll: bool) -> Self {
        self.scroll = scroll;
        self
    }

    /// Paints `panel.fill` behind the panel (default off: a dock group paints its own fill).
    #[must_use]
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// A footer fixed under the body. `add` shows its widgets in the bar; see [`PanelFooter`].
    #[must_use]
    pub fn footer(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.footer = Some(Box::new(add));
        self
    }

    /// The footer toolbar's translated accessible name.
    #[must_use]
    pub fn footer_label(mut self, label: impl Into<String>) -> Self {
        self.footer_label = Some(label.into());
        self
    }

    /// The scroll bar's translated accessible name (default "Scroll bar").
    #[must_use]
    pub fn scroll_bar_label(mut self, label: impl Into<String>) -> Self {
        self.scroll_bar_label = label.into();
        self
    }

    /// The panel's padding, fill and footer height.
    pub fn style(tokens: &UiTokens, pixels_per_point: f32) -> PanelStyle {
        let panel = &tokens.component.panel;
        PanelStyle { pad: panel.pad, fill: panel.fill.to_egui(), footer_height: Panel::footer_height(tokens, pixels_per_point) }
    }

    /// The height a footer takes: the rule and the bar. A host adds it to a dock group's minimum
    /// height so the footer is never clipped.
    pub fn footer_height(tokens: &UiTokens, pixels_per_point: f32) -> f32 {
        let footer = PanelFooter::style(tokens, pixels_per_point);
        footer.rule.width + footer.height
    }

    /// Shows the panel over the rect `ui` has left and runs `add` in its body.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> PanelOutput<R> {
        let tokens = Tokens::of(ui);
        let ppp = ui.pixels_per_point();
        let style = Panel::style(&tokens, ppp);
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let id = ui.make_persistent_id(self.id_salt);
        let mut builder = UiBuilder::new().id_salt(id).max_rect(rect);
        if let Some(title) = &self.title {
            builder = builder.accessibility_role(Role::Group).accessibility_label(title.as_str());
        }
        let mut panel = ui.new_child(builder);
        panel.set_clip_rect(rect.intersect(ui.clip_rect()));
        if self.fill {
            panel.painter().rect_filled(rect, 0.0, style.fill);
        }
        let footer_height = if self.footer.is_some() { style.footer_height.min(rect.height()) } else { 0.0 };
        let body = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - footer_height));
        let inner = Panel::show_body(&mut panel, body, &style, self.scroll.then_some(self.scroll_bar_label.as_str()), add);
        let footer = self.footer.map(|add| {
            let footer = Rect::from_min_max(pos2(rect.left(), rect.bottom() - footer_height), rect.max);
            Panel::show_footer(&mut panel, footer, &tokens, self.footer_label.as_deref(), add);
            footer
        });
        PanelOutput { inner, footer }
    }

    /// The padded, optionally scrolling body in `body`; the floating panel shows its content the
    /// same way.
    pub(crate) fn show_body<R>(parent: &mut Ui, body: Rect, style: &PanelStyle, scroll_bar_label: Option<&str>, add: impl FnOnce(&mut Ui) -> R) -> R {
        let mut ui = parent.new_child(UiBuilder::new().id_salt("body").max_rect(body));
        ui.set_clip_rect(body.intersect(parent.clip_rect()));
        let body_gap = DockLook::of(&ui).body_gap;
        let content = move |ui: &mut Ui| {
            Frame::new()
                .inner_margin(Margin::same(style.pad.round() as i8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = body_gap;
                    add(ui)
                })
                .inner
        };
        if let Some(label) = scroll_bar_label {
            ScrollArea::vertical().id_salt("scroll").vertical_scroll_bar_label(label).auto_shrink([false, false]).show(&mut ui, content).inner
        } else {
            content(&mut ui)
        }
    }

    fn show_footer(parent: &mut Ui, footer: Rect, tokens: &UiTokens, label: Option<&str>, add: FooterContent<'_>) {
        let style = PanelFooter::style(tokens, parent.pixels_per_point());
        let reserved = PanelFooter::reserved_height(&style);
        let area =
            Rect::from_min_size(pos2(footer.left() + style.margin, footer.top()), vec2((footer.width() - 2.0 * style.margin).max(0.0), reserved));
        let mut builder = UiBuilder::new().id_salt("footer").max_rect(area).accessibility_role(Role::Toolbar);
        if let Some(label) = label {
            builder = builder.accessibility_label(label);
        }
        let mut ui = parent.new_child(builder);
        ui.set_clip_rect(parent.clip_rect());
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| node.set_orientation(Orientation::Horizontal));
        PanelFooter::show(&mut ui, add);
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Panel"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || {
        Args::new()
            .with_bool("scroll", true)
            .with_bool("footer", true)
            .with_number("height", 240.0, 60.0, 480.0)
            .with_number("width", 260.0, 120.0, 480.0)
    };
    vec![Story::new("patterns/panel/embedded", Level::Patterns, "Panel", "Embedded", args, show_embedded).on(Surface::Own)]
}

/// A Layers-like panel: rows in the body and three footer buttons.
fn show_embedded(ui: &mut Ui, args: &mut Args) {
    let size = vec2(args.number("width") as f32, args.number("height") as f32);
    let (scroll, footer) = (args.bool("scroll"), args.bool("footer"));
    ui.allocate_ui(size, |ui| {
        let mut panel = Panel::new("story").title("Layers").scroll(scroll).fill(true);
        if footer {
            panel = panel.footer_label("Layer actions").footer(|ui| {
                PanelFooter::button(ui, Icon::Trash, "Delete layer");
                PanelFooter::button(ui, Icon::Plus, "New layer");
                PanelFooter::button(ui, Icon::Settings, "Layer settings");
            });
        }
        panel.show(ui, |ui| {
            for row in 1..=24 {
                ui.label(format!("Layer {row}"));
            }
        });
    });
}
