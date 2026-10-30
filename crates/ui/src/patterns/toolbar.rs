//! [`Toolbar`]: a row or column of icon-button groups with shortcut hints.

use egui::accesskit::{self, Role};
use egui::{AsIdSalt, IdSalt, KeyboardShortcut, Rect, Response, ScrollArea, Sense, Ui, UiBuilder, pos2, vec2};

use crate::canvas::Orientation;
use crate::controls::{IconButton, Kbd};
use crate::direction;
use crate::icons::Icon;
use crate::layout::fit_toolbar;
use crate::menu::{MenuEntry, MenuItem, PressMenu};
use crate::primitives::Divider;
use crate::story::{Level, NoArgs, Story, StoryArgs, Surface};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens};
use crate::typed_story;

/// A toolbar's sizes and colours, from [`Toolbar::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToolbarStyle {
    /// Side of a horizontal toolbar's button: `toolbar.button`.
    pub button: f32,
    /// Side of a vertical strip's button: `toolbar.button_large`.
    pub button_large: f32,
    /// Between the buttons of a group: `toolbar.gap`.
    pub gap: f32,
    /// Added between the groups of a vertical strip: `toolbar.group_gap`.
    pub group_gap: f32,
    /// Inset of a vertical strip, across and along: `toolbar.pad`.
    pub pad: [f32; 2],
    /// Width of a vertical strip: `toolbar.width`.
    pub width: f32,
    /// Width of the overflow button: `toolbar.overflow`.
    pub overflow: f32,
    /// Space between the groups of a horizontal toolbar, which holds a divider: the button side
    /// less two points (24 at Compact, 30 at Comfortable).
    pub spacing: f32,
    /// Fill of a vertical strip: `surface.panel`.
    pub fill: egui::Color32,
}

/// One button of a [`Toolbar`].
pub struct ToolbarItem<'a> {
    icon: Icon,
    label: String,
    shortcut: Option<KeyboardShortcut>,
    toggle: Option<&'a mut bool>,
    enabled: bool,
}

impl<'a> ToolbarItem<'a> {
    /// A button with `icon`; `label` is its translated name, tooltip text and menu row.
    pub fn new(icon: Icon, label: impl Into<String>) -> Self {
        Self { icon, label: label.into(), shortcut: None, toggle: None, enabled: true }
    }

    /// The shortcut shown after the label in the tooltip and in the overflow menu row. The host
    /// handles the key.
    pub fn shortcut(mut self, shortcut: &KeyboardShortcut) -> Self {
        self.shortcut = Some(*shortcut);
        self
    }

    /// A toggle button: a click flips `on`, and the button shows the toggled look while it is on.
    pub fn toggle(mut self, on: &'a mut bool) -> Self {
        self.toggle = Some(on);
        self
    }

    /// Whether the button reacts; enabled by default.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// What a [`Toolbar`] reports for a pass.
#[derive(Debug, Default)]
pub struct ToolbarResponse {
    /// The `(group, item)` that was clicked or chosen from the overflow menu this pass.
    pub clicked: Option<(usize, usize)>,
    /// The responses of the drawn buttons, in item order across groups. Items in the overflow menu
    /// have none; the overflow button is not included.
    pub items: Vec<Response>,
}

/// A row or column of icon-button groups, as in PhotoCraft's Liquify strip and the horizontal tool
/// strips of the other apps. A click is a command, not an edit: the host handles `clicked`.
///
/// ```
/// use craft_ui::canvas::Orientation;
/// use craft_ui::icons::Icon;
/// use craft_ui::patterns::{Toolbar, ToolbarItem};
/// # egui::__run_test_ui(|ui| {
/// let out = Toolbar::new("tools", Orientation::Horizontal)
///     .label("Tools")
///     .group([ToolbarItem::new(Icon::Brush, "Brush"), ToolbarItem::new(Icon::Eraser, "Eraser")])
///     .selected(Some((0, 0)))
///     .overflow(true)
///     .show(ui);
/// if let Some((_group, _item)) = out.clicked {}
/// # });
/// ```
///
/// - **Vertical:** a strip `toolbar.width` wide and as tall as the space left, in `surface.panel`
///   with a hairline on its inner edge; buttons are `toolbar.button_large` squares. The strip sits
///   at the reading start of its container, which the host arranges. If it is too short it scrolls.
/// - **Horizontal:** a row of `toolbar.button` squares with a divider between groups. With
///   [`overflow`](Toolbar::overflow) the groups that do not fit move to a menu behind a "More"
///   button at the end ([`fit_toolbar`]); without it, or when even the button does not fit, the row
///   scrolls. Every item stays reachable.
/// - **Selection:** [`selected`](Toolbar::selected) marks one item, given as `(group, item)` like [`ToolbarResponse::clicked`], with the
///   toggled look; toggle items keep their own state.
/// - **Keyboard:** every drawn button is a Tab stop in reading order; a hidden item is reached
///   through the overflow menu. A horizontal toolbar mirrors in a right-to-left UI.
/// - **AccessKit:** a `Toolbar` named by [`label`](Toolbar::label) with its orientation; buttons
///   are named by their labels, with the shortcut as description and `toggled` on the selected
///   one; the overflow button is a `Button` "More" with `has_popup` and `expanded`.
#[must_use = "You should call .show"]
pub struct Toolbar<'a> {
    id_salt: IdSalt,
    orientation: Orientation,
    groups: Vec<Vec<ToolbarItem<'a>>>,
    selected: Option<(usize, usize)>,
    overflow: bool,
    label: String,
    more_label: String,
}

impl<'a> Toolbar<'a> {
    /// An empty toolbar; `id_salt` keeps the buttons' identity.
    pub fn new(id_salt: impl AsIdSalt, orientation: Orientation) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            orientation,
            groups: Vec::new(),
            selected: None,
            overflow: false,
            label: "Toolbar".into(),
            more_label: "More".into(),
        }
    }

    /// The toolbar's accessible name, translated by the host (default "Toolbar").
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// The overflow button's name, tooltip and menu label, translated (default "More").
    pub fn more_label(mut self, label: impl Into<String>) -> Self {
        self.more_label = label.into();
        self
    }

    /// Adds a group. An empty group is ignored.
    pub fn group(mut self, items: impl IntoIterator<Item = ToolbarItem<'a>>) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        if !items.is_empty() {
            self.groups.push(items);
        }
        self
    }

    /// The exclusive mode: the item at `(group, item)`, both indices counted from 0 within the
    /// groups added with [`group`](Self::group) (empty groups are not kept), has the toggled look
    /// and every other item is plain and not toggled. `None` (the default) leaves items plain.
    pub fn selected(mut self, selected: Option<(usize, usize)>) -> Self {
        self.selected = selected;
        self
    }

    /// Horizontal only: move the groups that do not fit into a menu (default off, where a too
    /// narrow row scrolls).
    pub fn overflow(mut self, overflow: bool) -> Self {
        self.overflow = overflow;
        self
    }

    pub fn style(tokens: &UiTokens) -> ToolbarStyle {
        let t = &tokens.component.toolbar;
        ToolbarStyle {
            button: t.button,
            button_large: t.button_large,
            gap: t.gap,
            group_gap: t.group_gap,
            pad: t.pad,
            width: t.width,
            overflow: t.overflow,
            spacing: t.spacing,
            fill: tokens.color.surface.panel.to_egui(),
        }
    }
}

/// How the buttons are placed: where the content starts and which way it runs.
struct Flow {
    vertical: bool,
    rtl: bool,
    side: f32,
    gap: f32,
    group_gap: f32,
}

impl Flow {
    /// The square for a button `along` points from the start of `area`.
    fn cell(&self, area: Rect, along: f32) -> Rect {
        if self.vertical {
            Rect::from_min_size(pos2(area.left(), area.top() + along), vec2(self.side, self.side))
        } else if self.rtl {
            Rect::from_min_size(pos2(area.right() - along - self.side, area.top()), vec2(self.side, self.side))
        } else {
            Rect::from_min_size(pos2(area.left() + along, area.top()), vec2(self.side, self.side))
        }
    }

    /// Distance between the starts of two successive buttons of a group.
    fn step(&self) -> f32 {
        self.side + self.gap
    }

    fn group_width(&self, count: usize) -> f32 {
        count as f32 * self.side + count.saturating_sub(1) as f32 * self.gap
    }
}

struct Pass<'p> {
    selected: Option<(usize, usize)>,
    out: &'p mut ToolbarResponse,
}

impl<'a> Toolbar<'a> {
    /// Shows the toolbar. A horizontal one spans the available width; a vertical one is
    /// `toolbar.width` wide and spans the available height.
    pub fn show(mut self, ui: &mut Ui) -> ToolbarResponse {
        let tokens = Tokens::of(ui);
        let style = Toolbar::style(&tokens);
        let vertical = self.orientation == Orientation::Vertical;
        let rtl = direction::is_rtl(ui);
        let flow = Flow { vertical, rtl, side: if vertical { style.button_large } else { style.button }, gap: style.gap, group_gap: style.group_gap };
        let size = if vertical { vec2(style.width, ui.available_height()) } else { vec2(ui.available_width(), style.button) };
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let id = ui.make_persistent_id(self.id_salt);
        let builder = UiBuilder::new().id_salt(id).max_rect(rect).accessibility_role(Role::Toolbar).accessibility_label(self.label.clone());
        let mut bar = ui.new_child(builder);
        let orientation = if vertical { accesskit::Orientation::Vertical } else { accesskit::Orientation::Horizontal };
        bar.ctx().accesskit_node_builder(bar.unique_id(), |node| node.set_orientation(orientation));

        let mut out = ToolbarResponse::default();

        if vertical {
            self.paint_strip(&bar, rect, &tokens, &style);
            let height: f32 = self.groups.iter().map(|g| flow.group_width(g.len())).sum::<f32>()
                + self.groups.len().saturating_sub(1) as f32 * (flow.gap + flow.group_gap);
            let scrolls = height > rect.height() - 2.0 * style.pad[1];
            // A scrolling strip gives the bar the strip's edge, which egui puts at the reading end
            // of the area (the left in a right-to-left UI), and centres the buttons in the rest.
            let inner = if scrolls { rect.shrink2(vec2(0.0, style.pad[1])) } else { rect.shrink2(vec2(style.pad[0], style.pad[1])) };
            let mut inner_ui = bar.new_child(UiBuilder::new().max_rect(inner));
            if scrolls {
                let scroll = &mut inner_ui.spacing_mut().scroll;
                if scroll.floating {
                    scroll.floating_allocated_width = scroll.bar_width;
                }
            }
            let scroll = ScrollArea::vertical().id_salt("scroll").auto_shrink([false, false]).scroll_bar_visibility(if scrolls {
                egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded
            } else {
                egui::scroll_area::ScrollBarVisibility::AlwaysHidden
            });
            scroll.show(&mut inner_ui, |ui| {
                let (column, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                // The button column is centred, so it mirrors by itself in a right-to-left UI.
                let content = Rect::from_min_size(pos2(column.center().x - flow.side / 2.0, column.top()), vec2(flow.side, height));
                let mut along = 0.0;
                for (g, items) in self.groups.iter_mut().enumerate() {
                    along = draw_group(ui, &flow, content, along, g, items, &mut Pass { selected: self.selected, out: &mut out });
                    along += flow.group_gap;
                }
            });
            return out;
        }

        let natural: Vec<f32> = self.groups.iter().map(|g| flow.group_width(g.len())).collect();
        let total = natural.iter().sum::<f32>() + natural.len().saturating_sub(1) as f32 * style.spacing;
        let fit = if self.overflow { fit_toolbar(&natural, rect.width(), style.spacing, style.overflow) } else { None };
        match fit {
            Some(fit) if fit.overflow.is_empty() => {
                self.draw_row(&mut bar, &flow, rect, &style, &mut out, self.groups.len());
            }
            Some(fit) => {
                let end = self.draw_row(&mut bar, &flow, rect, &style, &mut out, fit.shown.len());
                let along = if fit.shown.is_empty() { 0.0 } else { end + style.spacing };
                self.draw_overflow(&mut bar, &flow, rect, &style, along, &fit.overflow, &mut out);
            }
            None if total <= rect.width() => {
                self.draw_row(&mut bar, &flow, rect, &style, &mut out, self.groups.len());
            }
            None => {
                let mut scroll = ScrollArea::horizontal().id_salt("scroll").auto_shrink([false, false]);
                // The first item is at the reading start; in a right-to-left UI that is the
                // far end, so a scroll area without saved state opens there.
                if rtl && egui::scroll_area::State::load(bar.ctx(), bar.make_persistent_id("scroll")).is_none() {
                    scroll = scroll.scroll_offset(vec2((total - rect.width()).max(0.0), 0.0));
                }
                scroll.show(&mut bar, |ui| {
                    let (content, _) = ui.allocate_exact_size(vec2(total, style.button), Sense::hover());
                    let mut bar = ui.new_child(UiBuilder::new().max_rect(content));
                    self.draw_row(&mut bar, &flow, content, &style, &mut out, self.groups.len());
                });
            }
        }
        out
    }

    fn paint_strip(&self, ui: &Ui, rect: Rect, tokens: &UiTokens, style: &ToolbarStyle) {
        ui.painter().rect_filled(rect, 0.0, style.fill);
        let edge = if direction::is_rtl(ui) { rect.left() } else { rect.right() };
        let sign = if direction::is_rtl(ui) { 1.0 } else { -1.0 };
        Divider::paint(ui.painter(), pos2(edge + sign * 0.5, rect.top()), pos2(edge + sign * 0.5, rect.bottom()), tokens);
    }

    /// Draws the first `shown` groups with dividers between them and returns the length they use.
    #[allow(clippy::too_many_arguments)]
    fn draw_row(&mut self, ui: &mut Ui, flow: &Flow, area: Rect, style: &ToolbarStyle, out: &mut ToolbarResponse, shown: usize) -> f32 {
        let tokens = Tokens::of(ui);
        let mut along = 0.0;
        let selected = self.selected;
        for (g, items) in self.groups.iter_mut().enumerate().take(shown) {
            if g > 0 {
                // `along` includes the gap after the previous group's last button.
                along -= flow.gap;
                let mid = flow.cell(area, along + style.spacing / 2.0 - flow.side / 2.0).center().x;
                if ui.is_rect_visible(area) {
                    Divider::paint(ui.painter(), pos2(mid, area.top() + flow.gap), pos2(mid, area.bottom() - flow.gap), &tokens);
                }
                along += style.spacing;
            }
            along = draw_group(ui, flow, area, along, g, items, &mut Pass { selected, out });
        }
        along - flow.gap
    }

    /// The "More" button at `along` and its menu of the hidden groups' items.
    #[allow(clippy::too_many_arguments)]
    fn draw_overflow(&mut self, ui: &mut Ui, flow: &Flow, area: Rect, style: &ToolbarStyle, along: f32, hidden: &[usize], out: &mut ToolbarResponse) {
        let cell = flow.cell(area, along);
        let rect =
            Rect::from_min_size(if flow.rtl { pos2(cell.right() - style.overflow, cell.top()) } else { cell.min }, vec2(style.overflow, flow.side));
        let id = ui.make_persistent_id("more");
        let button = IconButton::new(Icon::More, self.more_label.clone()).with_id(id).show_at(ui, rect);
        let mut entries: Vec<MenuEntry<(usize, usize)>> = Vec::new();
        for &g in hidden {
            if !entries.is_empty() {
                entries.push(MenuEntry::Separator);
            }
            for (i, item) in self.groups[g].iter().enumerate() {
                let on = match &item.toggle {
                    Some(on) => Some(**on),
                    None => self.selected.map(|s| s == (g, i)),
                };
                let mut row = MenuItem::new(item.label.clone(), (g, i)).icon(item.icon).enabled(item.enabled);
                if let Some(on) = on {
                    row = row.check(on);
                }
                if let Some(shortcut) = &item.shortcut {
                    row = row.shortcut(Kbd::new(shortcut));
                }
                entries.push(row.into());
            }
        }
        if let Some((g, i)) = PressMenu::new(&entries).show(ui, &button).chosen {
            if let Some(on) = self.groups[g][i].toggle.as_deref_mut() {
                *on = !*on;
            }
            out.clicked = Some((g, i));
        }
    }
}

/// Draws one group's buttons from `along` and returns the length where the next button would
/// start.
fn draw_group(ui: &mut Ui, flow: &Flow, area: Rect, mut along: f32, g: usize, items: &mut [ToolbarItem<'_>], pass: &mut Pass<'_>) -> f32 {
    for (i, item) in items.iter_mut().enumerate() {
        let rect = flow.cell(area, along);
        let os = ui.ctx().os();
        let mut button = IconButton::new(item.icon, item.label.clone()).with_id(ui.make_persistent_id(("item", g, i))).enabled(item.enabled);
        if let Some(shortcut) = &item.shortcut {
            button = button.shortcut(Kbd::new(shortcut).text_in(os, &crate::localized::Words::of(ui)));
        }
        button = match item.toggle.as_deref_mut() {
            Some(on) => button.toggle(on),
            None => match pass.selected {
                Some(s) => button.selected(s == (g, i)),
                None => button,
            },
        };
        let response = button.show_at(ui, rect);
        if response.clicked() {
            pass.out.clicked = Some((g, i));
        }
        pass.out.items.push(response);
        along += flow.step();
    }
    along
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Toolbar"];

const LIQUIFY: &[&[(Icon, &str)]] = &[
    &[(Icon::Pointer, "Forward Warp"), (Icon::Droplet, "Reconstruct"), (Icon::Wand, "Smooth"), (Icon::Scaling, "Twirl Clockwise")],
    &[(Icon::Move, "Pucker"), (Icon::Target, "Bloat"), (Icon::Hand, "Push Left")],
    &[(Icon::Lasso, "Freeze Mask"), (Icon::Eraser, "Thaw Mask"), (Icon::Eyedropper, "Face"), (Icon::Crop, "Hand")],
];

fn shortcut_for(index: usize) -> KeyboardShortcut {
    let keys = [
        egui::Key::W,
        egui::Key::R,
        egui::Key::E,
        egui::Key::C,
        egui::Key::S,
        egui::Key::B,
        egui::Key::O,
        egui::Key::F,
        egui::Key::T,
        egui::Key::A,
        egui::Key::H,
    ];
    KeyboardShortcut::new(egui::Modifiers::NONE, keys[index % keys.len()])
}

fn show_vertical(ui: &mut Ui, _: &mut NoArgs) {
    let id = ui.scope_id().with("toolbar-vertical");
    let mut selected = ui.data(|d| d.get_temp::<(usize, usize)>(id)).unwrap_or((0, 0));
    ui.allocate_ui(vec2(ui.available_width(), 480.0), |ui| {
        let mut toolbar = Toolbar::new("liquify-tools", Orientation::Vertical).label("Liquify tools").selected(Some(selected));
        let mut index = 0;
        for group in LIQUIFY {
            let items: Vec<_> = group
                .iter()
                .map(|&(icon, label)| {
                    let item = ToolbarItem::new(icon, label).shortcut(&shortcut_for(index));
                    index += 1;
                    item
                })
                .collect();
            toolbar = toolbar.group(items);
        }
        if let Some((g, i)) = toolbar.show(ui).clicked {
            selected = (g, i);
        }
    });
    ui.data_mut(|d| d.insert_temp(id, selected));
}

fn show_horizontal(ui: &mut Ui, args: &mut WidthArgs) {
    let id = ui.scope_id().with("toolbar-horizontal");
    let mut state = ui.data(|d| d.get_temp::<((usize, usize), bool)>(id)).unwrap_or(((0, 0), false));
    let width = args.width.min(ui.available_width());
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.set_max_width(width);
        let mut grid = state.1;
        let mut toolbar = Toolbar::new("horizontal-tools", Orientation::Horizontal).label("Tools").selected(Some(state.0)).overflow(true);
        let mut index = 0;
        for group in &LIQUIFY[..2] {
            let items: Vec<_> = group
                .iter()
                .map(|&(icon, label)| {
                    let item = ToolbarItem::new(icon, label).shortcut(&shortcut_for(index));
                    index += 1;
                    item
                })
                .collect();
            toolbar = toolbar.group(items);
        }
        toolbar = toolbar.group([ToolbarItem::new(Icon::Grid, "Show Grid").toggle(&mut grid)]);
        if let Some((g, i)) = toolbar.show(ui).clicked
            && g < 2
        {
            state.0 = (g, i);
        }
        state.1 = grid;
    });
    ui.data_mut(|d| d.insert_temp(id, state));
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/toolbar/vertical-liquify", Level::Patterns, "Toolbar", "Vertical", NoArgs {}, show_vertical).on(Surface::Own),
        typed_story!("patterns/toolbar/horizontal-overflow", Level::Patterns, "Toolbar", "Overflow", WidthArgs { width: 260.0 }, show_horizontal)
            .on(Surface::Chrome),
    ]
}

#[derive(StoryArgs, Debug, PartialEq)]
struct WidthArgs {
    #[arg(range = 60.0..=640.0)]
    width: f32,
}
