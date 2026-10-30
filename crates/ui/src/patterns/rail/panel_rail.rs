//! `PanelRail`: the rail of icons that open panels, drawn with the `rail.panel_*`,
//! `rail.icon.panel`, `rail.selected*` and `rail.marker*` tokens.

use std::fmt::Debug;
use std::hash::Hash;

use egui::accesskit::{Orientation, Role};
use egui::emath::GuiRounding as _;
use egui::{
    Align, AsIdSalt, Color32, CornerRadius, EventFilter, Frame, Id, IdSalt, Key, Layout, Margin, Modifiers, Rect, Response, ScrollArea, Sense,
    Stroke, Ui, UiBuilder, pos2, vec2,
};

use crate::controls::{IconButton, IconButtonStyle};
use crate::direction;
use crate::icons::Icon;
use crate::panes::Edge;
use crate::story::{Args, Level, Story, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use craft_theme::{ControlSize, States};

/// A panel rail's sizes and colours, from [`PanelRail::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PanelRailStyle {
    /// Width of the rail: `rail.panel_width`.
    pub width: f32,
    /// Side of a button: `rail.panel_button`.
    pub button: f32,
    /// Between buttons: `rail.panel_gap`.
    pub gap: f32,
    /// Above and below the buttons: `rail.pad_y`.
    pub pad_y: f32,
    /// Radius of the marker dot: `rail.marker_radius`.
    pub marker_radius: f32,
    /// Behind the rail: `rail.fill`.
    pub fill: Color32,
    /// The marker dot: `rail.marker`.
    pub marker: Color32,
    /// The line on the canvas side: `rail.edge`.
    pub edge: Color32,
    /// What each button is drawn with: glyph `rail.panel_icon` in `rail.icon.panel`, `rail.icon.hover`
    /// on `rail.hover`; open: `rail.selected` inside `rail.selected.border`, glyph `rail.selected.text`.
    pub icon_button: IconButtonStyle,
}

/// One icon of the rail, standing for the panel it opens.
#[derive(Clone, Debug)]
pub struct PanelRailItem<P> {
    pub panel: P,
    pub icon: Icon,
    /// Tooltip and accessible name. Translate it in the app.
    pub label: String,
    /// `false`: drawn disabled, not focusable, and a click asks for nothing.
    pub enabled: bool,
    /// A small dot telling that the panel has content (comments exist, the document has
    /// bookmarks). Drawn only while the panel is not open.
    pub marker: bool,
    /// The item's accessible description while the marker shows ("Has comments"). Translate it
    /// in the app.
    pub marker_label: Option<String>,
}

impl<P> PanelRailItem<P> {
    /// An enabled item without a marker.
    pub fn new(panel: P, icon: Icon, label: impl Into<String>) -> Self {
        Self { panel, icon, label: label.into(), enabled: true, marker: false, marker_label: None }
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn marker(mut self, marker: bool) -> Self {
        self.marker = marker;
        self
    }

    #[must_use]
    pub fn marker_label(mut self, label: impl Into<String>) -> Self {
        self.marker_label = Some(label.into());
        self
    }
}

/// What a click on an item asks the host to do.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum PanelRailRequest<P> {
    /// Show this panel.
    Open(P),
    /// Hide this panel, which is shown.
    Close(P),
}

/// What [`PanelRail::show`] returns.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PanelRailOutput<P> {
    /// The click of this frame, if any.
    pub request: Option<PanelRailRequest<P>>,
    /// The item holding keyboard focus.
    pub focused: Option<P>,
    /// Each drawn item's response, in order, so the host can attach a context menu or start a dock
    /// drag. The rail does neither.
    pub responses: Vec<(P, Response)>,
    /// Where the rail was drawn.
    pub rect: Rect,
}

/// The rail of icons that open panels, as a side panel at one edge of the window.
///
/// ```
/// use craft_ui::icons::Icon;
/// use craft_ui::patterns::{PanelRail, PanelRailItem};
/// # egui::__run_test_ui(|ui| {
/// let items = [PanelRailItem::new("layers", Icon::Layers, "Layers"), PanelRailItem::new("history", Icon::Clock, "History").marker(true)];
/// let out = PanelRail::new("panels").label("Panels").show(ui, &items, &["layers"], |_| {});
/// # let _ = out;
/// # });
/// ```
///
/// - **Look:** `rail.panel_width` wide in `rail.fill`, `rail.panel_button` squares with
///   `rail.panel_icon` glyphs, `rail.pad_y` above and below, one hairline in `rail.edge` along the
///   canvas side. A button whose panel is open has the `rail.selected` fill inside a
///   `rail.selected.border` outline; one under the pointer has `rail.hover`.
/// - **Marker:** a `rail.marker` dot at the button's corner away from the canvas, only while the
///   panel is not open.
/// - **Requests:** a click on a closed panel's icon asks to open it, on an open one to close it.
///   The host changes its layout; the rail stores nothing.
/// - **Layout:** call it before the panel host, so the rail is the outermost panel on its edge.
///   `items` scroll in a short window; `foot` stays below them. Edges and the marker corner mirror
///   in a right-to-left interface.
/// - **Keyboard and AccessKit:** each enabled icon is a Tab stop; Up and Down move between icons
///   and stop at the ends; Enter and Space activate. A disabled focused icon hands focus to the
///   nearest enabled one. The rail is a vertical `Toolbar` named by [`label`](PanelRail::label);
///   an icon is a `Button` with `selected` while its panel is open, described by its marker label
///   while the marker shows.
pub struct PanelRail {
    id_salt: IdSalt,
    edge: Edge,
    label: String,
    scroll_bar_label: String,
}

impl PanelRail {
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt), edge: Edge::End, label: "Panels".to_owned(), scroll_bar_label: "Scroll bar".to_owned() }
    }

    /// The edge the rail sits at (default [`Edge::End`]). The rail is vertical, so
    /// [`Edge::Bottom`] draws it at the end edge.
    #[must_use]
    pub fn edge(mut self, edge: Edge) -> Self {
        self.edge = edge;
        self
    }

    /// The toolbar's translated accessible name (default "Panels").
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// The scroll bar's translated accessible name (default "Scroll bar").
    #[must_use]
    pub fn scroll_bar_label(mut self, label: impl Into<String>) -> Self {
        self.scroll_bar_label = label.into();
        self
    }

    pub fn style(tokens: &UiTokens) -> PanelRailStyle {
        let r = &tokens.component.rail;
        let clear = Color32::TRANSPARENT;
        let hover = r.hover.to_egui();
        let icon_hover = r.icon_hover.to_egui();
        let icon = r.panel_icon_color.to_egui();
        let disabled = IconButton::style(tokens, ControlSize::Md);
        PanelRailStyle {
            width: r.panel_width,
            button: r.panel_button,
            gap: r.panel_gap,
            pad_y: r.pad_y,
            marker_radius: r.marker_radius,
            fill: r.fill.to_egui(),
            marker: r.marker.to_egui(),
            edge: r.edge.to_egui(),
            icon_button: IconButtonStyle {
                side: r.panel_button,
                icon: r.panel_icon,
                radius: CornerRadius::same(r.radius.round().clamp(0.0, 255.0) as u8),
                fill: States::new(clear, hover, hover, clear),
                text: States::new(icon, icon_hover, icon_hover, disabled.text.disabled),
                selected_fill: States::new(r.selected.to_egui(), r.selected.to_egui(), r.selected.to_egui(), disabled.selected_fill.disabled),
                selected_border: States::new(
                    r.selected_border.to_egui(),
                    r.selected_border.to_egui(),
                    r.selected_border.to_egui(),
                    disabled.selected_border.disabled,
                ),
                selected_text: States::new(
                    r.selected_text.to_egui(),
                    r.selected_text.to_egui(),
                    r.selected_text.to_egui(),
                    disabled.selected_text.disabled,
                ),
                selected_border_width: Some(r.selected_border_width),
                ..disabled
            },
        }
    }

    /// The widget id of `panel`'s icon, to give it keyboard focus (`Memory::request_focus`), for
    /// example when the host's F6 cycle reaches the rail. `ui` is the one the rail is shown in.
    pub fn item_id<P: Hash + Debug>(&self, ui: &Ui, panel: &P) -> Id {
        Self::item_id_in(ui.make_persistent_id(self.id_salt), panel)
    }

    fn item_id_in<P: Hash + Debug>(id: Id, panel: &P) -> Id {
        id.with(("panel-rail-item", panel))
    }

    /// True when the rail sits against the left edge of the window.
    fn left(&self, ui: &Ui) -> bool {
        matches!((self.edge, direction::is_rtl(ui)), (Edge::Start, false) | (Edge::End | Edge::Bottom, true))
    }

    /// Draws the rail as the outermost panel on its edge. `items` are drawn top-down; `open` lists
    /// the panels the host shows; `foot` draws below the icons, always visible (page navigation,
    /// zoom), and may draw nothing.
    pub fn show<P: Clone + Hash + Debug>(
        self,
        ui: &mut Ui,
        items: &[PanelRailItem<P>],
        open: &[P],
        foot: impl FnOnce(&mut Ui),
    ) -> PanelRailOutput<P> {
        let style = PanelRail::style(&Tokens::of(ui));
        let id = ui.make_persistent_id(self.id_salt);
        let left = self.left(ui);
        let panel = if left { egui::Panel::left(id.with("panel")) } else { egui::Panel::right(id.with("panel")) };
        let frame = Frame::NONE.fill(style.fill).inner_margin(Margin::symmetric(0, style.pad_y.round() as i8));
        let shown = panel.exact_size(style.width).resizable(false).show_separator_line(false).frame(frame).show(ui, |ui| {
            // The foot is a panel of its own, so that in a short window the icons scroll instead
            // of running into it.
            egui::Panel::bottom(id.with("foot")).frame(Frame::NONE).show_separator_line(false).resizable(false).show(ui, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), foot);
            });
            let scroll =
                ScrollArea::vertical().id_salt(id.with("items")).auto_shrink([false, false]).vertical_scroll_bar_label(&self.scroll_bar_label);
            scroll
                .show(ui, |ui| {
                    let toolbar = UiBuilder::new().id_salt("toolbar").accessibility_role(Role::Toolbar).accessibility_label(self.label.as_str());
                    ui.scope_builder(toolbar, |ui| {
                        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| node.set_orientation(Orientation::Vertical));
                        ui.with_layout(Layout::top_down(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.y = style.gap;
                            Self::items(ui, id, left, &style, items, open)
                        })
                        .inner
                    })
                    .inner
                })
                .inner
        });
        let rect = shown.response.rect;
        // The hairline runs the rail's full height along the canvas side.
        let x = if left { rect.right() } else { rect.left() };
        let x = x.round_to_pixels(ui.pixels_per_point()) + if left { -0.5 } else { 0.5 };
        ui.painter().line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(1.0, style.edge));
        let (request, focused, responses) = shown.inner;
        PanelRailOutput { request, focused, responses, rect }
    }

    #[allow(clippy::type_complexity)]
    fn items<P: Clone + Hash + Debug>(
        ui: &mut Ui,
        rail: Id,
        left: bool,
        style: &PanelRailStyle,
        items: &[PanelRailItem<P>],
        open: &[P],
    ) -> (Option<PanelRailRequest<P>>, Option<P>, Vec<(P, Response)>) {
        let mut request = None;
        let mut focused = None;
        let mut responses = Vec::new();
        let open: Vec<Id> = open.iter().map(|panel| Self::item_id_in(rail, panel)).collect();
        let mut drawn: Vec<Id> = Vec::new();
        // Focus order for Up and Down: the enabled items with their place in `drawn`.
        let mut enabled: Vec<(usize, Id, &P)> = Vec::new();
        let mut focus_index = None;
        // An item disabled while focused, by its place. egui drops a disabled widget's focus as
        // it draws it, so this is read before.
        let mut lost_focus = None;
        for item in items {
            let id = Self::item_id_in(rail, &item.panel);
            // A panel listed twice would give two widgets one id.
            if drawn.contains(&id) {
                continue;
            }
            let place = drawn.len();
            drawn.push(id);
            let selected = open.contains(&id);
            let had_focus = ui.memory(|m| m.has_focus(id));
            let (rect, _) = ui.allocate_exact_size(vec2(style.button, style.button), Sense::hover());
            if item.marker
                && !selected
                && let Some(label) = &item.marker_label
            {
                ui.ctx().accesskit_node_builder(id, |node| node.set_description(label.as_str()));
            }
            let response = IconButton::new(item.icon, item.label.as_str())
                .with_id(id)
                .with_style(style.icon_button)
                .selected(selected)
                .enabled(item.enabled)
                .show_at(ui, rect);
            Self::paint_marker(ui, rect, item, selected, left, style);
            responses.push((item.panel.clone(), response.clone()));
            if !item.enabled {
                if had_focus {
                    lost_focus = Some(place);
                }
                continue;
            }
            if response.gained_focus() {
                response.scroll_to_me(None);
            }
            if response.has_focus() {
                focus_index = Some(enabled.len());
                focused = Some(item.panel.clone());
            }
            enabled.push((place, id, &item.panel));
            if response.clicked() {
                request = Some(if selected { PanelRailRequest::Close(item.panel.clone()) } else { PanelRailRequest::Open(item.panel.clone()) });
            }
        }
        if let Some(place) = lost_focus
            && focus_index.is_none()
        {
            // Focus moves to the nearest enabled item below, else above, else leaves the rail.
            let next = enabled.iter().find(|(p, ..)| *p > place).or_else(|| enabled.iter().rev().find(|(p, ..)| *p < place));
            if let Some(&(_, next, panel)) = next {
                ui.memory_mut(|m| m.request_focus_with_filter(next, EventFilter { vertical_arrows: true, ..Default::default() }));
                focused = Some(panel.clone());
            }
        }
        if let Some(index) = focus_index
            && let Some(&(_, id, _)) = enabled.get(index)
        {
            // Up and Down move between the items, never out of the rail by egui's geometric focus.
            ui.memory_mut(|m| m.set_focus_lock_filter(id, EventFilter { vertical_arrows: true, ..Default::default() }));
            let next = ui.input_mut(|input| {
                if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    Some(index.checked_add(1))
                } else if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    Some(index.checked_sub(1))
                } else {
                    None
                }
            });
            if let Some(&(_, next, _)) = next.flatten().and_then(|i| enabled.get(i)) {
                ui.memory_mut(|m| m.request_focus_with_filter(next, EventFilter { vertical_arrows: true, ..Default::default() }));
            }
        }
        (request, focused, responses)
    }

    fn paint_marker<P>(ui: &Ui, rect: Rect, item: &PanelRailItem<P>, selected: bool, left: bool, style: &PanelRailStyle) {
        if item.marker && !selected && ui.is_rect_visible(rect) {
            // The corner facing away from the canvas.
            let inset = 2.0 * style.marker_radius;
            let corner = if left { rect.left_top() + vec2(inset, inset) } else { rect.right_top() + vec2(-inset, inset) };
            ui.painter().circle_filled(corner, style.marker_radius, style.marker);
        }
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["PanelRail"];

const PANELS: [(&str, Icon, &str); 6] = [
    ("properties", Icon::Info, "Properties"),
    ("navigator", Icon::Navigation, "Navigator"),
    ("color", Icon::Palette, "Color & Swatches"),
    ("layers", Icon::Layers, "Layers"),
    ("history", Icon::Clock, "History"),
    ("brushes", Icon::Brush, "Brushes"),
];

const OPEN: &[&str] = &["none", "properties", "navigator", "color", "layers", "history", "brushes"];

/// PhotoCraft's panel rail: six panel icons, markers on History and Brushes.
fn show_story(ui: &mut Ui, args: &mut Args) {
    let open = args.choice("open");
    let height = args.number("height") as f32;
    let items: Vec<PanelRailItem<&'static str>> = PANELS
        .iter()
        .map(|(panel, icon, label)| {
            PanelRailItem::new(*panel, *icon, *label).marker(matches!(*panel, "history" | "brushes")).marker_label("Has content")
        })
        .collect();
    let open: Vec<&str> = OPEN.iter().copied().filter(|panel| *panel == open && *panel != "none").collect();
    ui.allocate_ui(vec2(120.0, height), |ui| {
        PanelRail::new("story").label("Panels").show(ui, &items, &open, |_| {});
    });
}

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_choice("open", OPEN, 4).with_number("height", 320.0, 120.0, 600.0);
    vec![Story::new("patterns/rail/panel", Level::Patterns, "PanelRail", "Panel rail", args, show_story).on(Surface::Chrome)]
}
