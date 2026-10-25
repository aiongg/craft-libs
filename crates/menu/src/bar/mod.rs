//! The menu bar drawn with egui, for Windows, Linux and the web (and macOS, for testing): an
//! already laid-out [`MenuBar`] as a row of menu titles whose menus open below them.
//!
//! A press on a title opens its menu (releasing over an item runs it, as in a native bar), a
//! second press or Esc closes it, and while one is open the pointer moving onto another title
//! opens that one. Submenus open on hover and stay open while the pointer heads for them. Tall
//! menus scroll, and the arrow keys, ↩ and Space drive an open menu ([`nav`]). When the titles
//! don't fit, the last ones move into a trailing "⋯" menu, or all of them into one "Menu".

mod nav;

use craft_keymap::{CommandRef, Platform};
use egui::accesskit::{HasPopup, Role, Toggled};
use egui::containers::menu::{MenuConfig, MenuState, SubMenu, SubMenuButton};
use egui::text::LayoutJob;
use egui::{Color32, Id, PopupCloseBehavior, Response, TextFormat, TextStyle, Ui, UiKind, UiStackInfo, WidgetText};

use crate::{Gate, Item, ItemKind, Menu, MenuBar, Node};
use nav::Nav;

pub use nav::is_open;

/// How the bar looks. [`BarStyle::from_style`] takes its defaults from an egui style.
#[derive(Clone, Debug, PartialEq)]
pub struct BarStyle {
    /// Padding around a menu title.
    pub title_padding: egui::Vec2,
    /// The menu titles' text color.
    pub title_color: Color32,
    /// The narrowest a menu is.
    pub menu_width: f32,
    /// Room a separator takes: the line and the space above and below it.
    pub separator_spacing: f32,
    /// Shown before the label of a checked check item.
    pub check_mark: String,
    /// Shown before the label of the selected radio item.
    pub radio_mark: String,
}

impl BarStyle {
    /// Title padding, title color and menu width from `style`.
    pub fn from_style(style: &egui::Style) -> BarStyle {
        BarStyle {
            title_padding: style.spacing.button_padding,
            title_color: style.visuals.widgets.inactive.text_color(),
            menu_width: style.spacing.menu_width,
            separator_spacing: 9.0,
            check_mark: "✓".to_owned(),
            radio_mark: "●".to_owned(),
        }
    }
}

impl Default for BarStyle {
    fn default() -> BarStyle {
        BarStyle::from_style(&egui::Style::default())
    }
}

fn english(label: &'static str) -> String {
    label.to_owned()
}

/// The bar for one frame: `MenuBarUi::new(&bar).gate(gate).show(ui)` returns the commands
/// chosen. Draw it at the top of the window, in a top panel.
pub struct MenuBarUi<'a> {
    bar: &'a MenuBar,
    gate: Gate,
    platform: Platform,
    style: Option<BarStyle>,
    label: &'a dyn Fn(&'static str) -> String,
    id: Id,
}

impl<'a> MenuBarUi<'a> {
    /// The bar for `bar`, laid out for the platform it runs on. Shortcuts show as on
    /// [`Platform::current`] (a web build shows Ctrl names).
    pub fn new(bar: &'a MenuBar) -> MenuBarUi<'a> {
        MenuBarUi { bar, gate: Gate::default(), platform: Platform::current(), style: None, label: &english, id: Id::new("craft-menu-bar") }
    }

    /// What may run this frame: the items `gate` forbids ([`Gate::allows`]) are drawn and
    /// behave as disabled.
    pub fn gate(mut self, gate: Gate) -> MenuBarUi<'a> {
        self.gate = gate;
        self
    }

    /// The platform whose names the shortcut column uses ([`craft_keymap::Chord::display`]).
    pub fn platform(mut self, platform: Platform) -> MenuBarUi<'a> {
        self.platform = platform;
        self
    }

    /// The bar's look; [`BarStyle::from_style`] of the `Ui` by default.
    pub fn style(mut self, style: BarStyle) -> MenuBarUi<'a> {
        self.style = Some(style);
        self
    }

    /// The app's translation of the labels the bar writes itself: `⋯` (the menu of titles that
    /// don't fit), `Menu` (the one menu when none fits), `Scroll menu up` and `Scroll menu down`
    /// (the scroll arrows' accessible names). The same hook as [`crate::LayoutOptions::label`].
    pub fn label(mut self, label: &'a dyn Fn(&'static str) -> String) -> MenuBarUi<'a> {
        self.label = label;
        self
    }

    /// The bar's id, where its state lives in egui memory: tells two bars in one window apart.
    pub fn id(mut self, id: Id) -> MenuBarUi<'a> {
        self.id = id;
        self
    }

    /// Draw the bar and its open menu; returns the commands chosen this frame (at most one),
    /// clicked or run with ↩ or Space. Disabled items, and items the gate forbids, never run.
    pub fn show(self, ui: &mut Ui) -> Vec<CommandRef> {
        let ctx = ui.ctx().clone();
        let style = self.style.clone().unwrap_or_else(|| BarStyle::from_style(ui.style()));
        let nav_id = self.id.with("nav");
        let mut nav = Nav::load(&ctx, nav_id);
        let mut draw = Draw { gate: self.gate, platform: self.platform, style: &style, label: self.label, press: self.id.with("press"), clicked: None };
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            egui::MenuBar::new().ui(ui, |ui| {
                ctx.accesskit_node_builder(ui.unique_id(), |n| n.set_role(Role::MenuBar));
                ui.spacing_mut().button_padding = style.title_padding;
                nav.bar_bottom = Some(ui.max_rect().bottom());
                let tops = self.tops(ui);
                let mut buttons = Vec::with_capacity(tops.len());
                for top in &tops {
                    let title = ui.add(egui::Button::new(egui::RichText::new(top.title()).color(style.title_color)));
                    let open_now = egui::Popup::is_id_open(&ctx, egui::Popup::default_response_id(&title));
                    ctx.accesskit_node_builder(title.id, |n| {
                        n.set_role(Role::MenuItem);
                        n.set_label(top.title());
                        n.set_has_popup(HasPopup::Menu);
                        n.set_expanded(open_now);
                    });
                    // The bar's close behavior and style, as a menu (not bar) config so that
                    // submenus inside draw as submenus.
                    let bar = MenuConfig::find(ui);
                    let config = MenuConfig::new().close_behavior(bar.close_behavior).style(bar.style.clone());
                    let open = title_press(&ctx, &title, draw.press);
                    // The release ending the press that opened this menu is a click "outside"
                    // the popup: it must not close it again.
                    let opening = title.clicked() && ctx.data(|d| d.get_temp::<bool>(draw.press)).unwrap_or(false);
                    let close = if opening { PopupCloseBehavior::IgnoreClicks } else { config.close_behavior };
                    egui::Popup::menu(&title)
                        .open_memory(open)
                        .close_behavior(close)
                        .style(config.style.clone())
                        .info(UiStackInfo::new(UiKind::Menu).with_tag_value(MenuConfig::MENU_CONFIG_TAG, config))
                        .show(|ui| {
                            ctx.accesskit_node_builder(ui.unique_id(), |n| n.set_role(Role::Menu));
                            ui.set_min_width(style.menu_width);
                            let rows = match top {
                                Top::Menu(m) => entries(&m.children),
                                Top::Group(_, menus) => menus.iter().map(|m| Entry::Sub(&m.title, &m.children)).collect(),
                            };
                            draw.level(ui, 1, &rows, &mut nav);
                        });
                    buttons.push(title);
                }
                switch_on_hover(&ctx, &buttons);
                let ids: Vec<Id> = buttons.iter().map(egui::Popup::default_response_id).collect();
                nav.keys(&ctx, &ids, &mut draw.clicked);
            });
        });
        nav.store(&ctx, nav_id);
        // The press-drag gesture ends with the button (its release was handled by the rows).
        if ctx.input(|i| i.pointer.primary_released() || !i.pointer.primary_down()) {
            ctx.data_mut(|d| d.remove::<bool>(draw.press));
        }
        draw.clicked.into_iter().collect()
    }

    /// The titles to draw: every menu when they fit in the row; else the menus that fit with
    /// room for a "⋯" menu holding the rest; else one "Menu" holding them all.
    fn tops(&self, ui: &Ui) -> Vec<Top<'a>> {
        let menus = &self.bar.menus;
        let font = TextStyle::Button.resolve(ui.style());
        let pad = 2.0 * ui.spacing().button_padding.x;
        let width = |t: &str| ui.painter().layout_no_wrap(t.to_owned(), font.clone(), Color32::PLACEHOLDER).size().x + pad;
        let room = ui.available_width();
        let widths: Vec<f32> = menus.iter().map(|m| width(&m.title)).collect();
        if widths.iter().sum::<f32>() <= room {
            return menus.iter().map(Top::Menu).collect();
        }
        let more = (self.label)("⋯");
        let room = room - width(&more);
        let mut used = 0.0;
        let fit = widths
            .iter()
            .take_while(|w| {
                used += **w;
                used <= room
            })
            .count();
        if fit == 0 {
            return vec![Top::Group((self.label)("Menu"), menus)];
        }
        let (shown, rest) = menus.split_at(fit);
        shown.iter().map(Top::Menu).chain([Top::Group(more, rest)]).collect()
    }
}

/// A title of the bar: a menu, or the overflow menu holding whole menus as submenus.
enum Top<'a> {
    Menu(&'a Menu),
    Group(String, &'a [Menu]),
}

impl Top<'_> {
    fn title(&self) -> &str {
        match self {
            Top::Menu(m) => &m.title,
            Top::Group(title, _) => title,
        }
    }
}

/// A row of a menu level.
#[derive(Clone, Copy)]
enum Entry<'a> {
    Item(&'a Item),
    Separator,
    Sub(&'a str, &'a [Node]),
}

/// The rows of `nodes`. AppKit's own items ([`Node::Standard`]) have nothing to run here and
/// are left out; a desktop or web layout has none.
fn entries(nodes: &[Node]) -> Vec<Entry<'_>> {
    nodes
        .iter()
        .filter_map(|n| match n {
            Node::Item(it) => Some(Entry::Item(it)),
            Node::Separator => Some(Entry::Separator),
            Node::Submenu { label, children, .. } => Some(Entry::Sub(label, children)),
            Node::Standard { .. } => None,
        })
        .collect()
}

/// A submenu is enabled only while some item under it (at any depth) can run, so a submenu of
/// disabled items is disabled too instead of opening onto them.
fn any_enabled(nodes: &[Node], gate: Gate) -> bool {
    nodes.iter().any(|n| match n {
        Node::Item(it) => it.enabled && gate.allows(it),
        Node::Submenu { children, .. } => any_enabled(children, gate),
        Node::Separator | Node::Standard { .. } => false,
    })
}

/// What the rows need while drawing, and the command chosen.
struct Draw<'s> {
    gate: Gate,
    platform: Platform,
    style: &'s BarStyle,
    label: &'s dyn Fn(&'static str) -> String,
    /// Where the press-drag gesture's flag lives: a press on a title opened the menus and the
    /// button is still down.
    press: Id,
    clicked: Option<CommandRef>,
}

impl Draw<'_> {
    /// One level of a menu, scrolling when taller than the window.
    fn level(&mut self, ui: &mut Ui, depth: usize, rows: &[Entry<'_>], nav: &mut Nav) {
        let label = self.label;
        nav::level(ui, depth, nav, label, |ui, nav| self.rows(ui, depth, rows, nav));
    }

    fn rows(&mut self, ui: &mut Ui, depth: usize, rows: &[Entry<'_>], nav: &mut Nav) {
        // Items never wrap: the menu widens to its longest label and shortcut.
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        // Rows touch, as in native menus.
        ui.spacing_mut().item_spacing.y = 0.0;
        let marks = Marks::new(ui, self.style);
        let mut last_was_sep = true;
        for (i, row) in rows.iter().enumerate() {
            match *row {
                Entry::Separator => {
                    let more = rows.get(i + 1..).is_some_and(|rest| rest.iter().any(|r| !matches!(r, Entry::Separator)));
                    if !last_was_sep && more {
                        ui.add(egui::Separator::default().spacing(self.style.separator_spacing));
                        last_was_sep = true;
                    }
                }
                Entry::Item(it) => {
                    self.item(ui, depth, it, &marks, nav);
                    last_was_sep = false;
                }
                Entry::Sub(label, children) => {
                    self.submenu(ui, depth, label, children, &marks, nav);
                    last_was_sep = false;
                }
            }
        }
    }

    fn item(&mut self, ui: &mut Ui, depth: usize, it: &Item, marks: &Marks, nav: &mut Nav) {
        let enabled = it.enabled && self.gate.allows(it);
        let (role, mark) = match it.kind {
            ItemKind::Plain => (Role::MenuItem, None),
            ItemKind::Check(on) => (Role::MenuItemCheckBox, on.then_some(self.style.check_mark.as_str())),
            ItemKind::Radio(on) => (Role::MenuItemRadio, on.then_some(self.style.radio_mark.as_str())),
        };
        let shortcut = it.shortcut.map(|c| c.display(self.platform));
        let mut button = egui::Button::new(marks.text(ui, mark, &it.label));
        if let Some(s) = &shortcut {
            button = button.shortcut_text(s.as_str());
        }
        let press = self.press;
        let hit = nav.row(ui, depth - 1, enabled, Some(&it.command), |ui, _| {
            let r = ui.add_enabled(enabled, button);
            access(&r, role, &it.label, it.kind.checked(), shortcut.as_deref());
            let hit = r.clicked() || released_on(ui, &r, press);
            (r, hit)
        });
        if hit {
            self.clicked = Some(it.command.clone());
            ui.close();
        }
    }

    fn submenu(&mut self, ui: &mut Ui, depth: usize, label: &str, children: &[Node], marks: &Marks, nav: &mut Nav) {
        let enabled = any_enabled(children, self.gate);
        let text = marks.text(ui, None, label);
        ui.add_enabled_ui(enabled, |ui| {
            nav.row(ui, depth - 1, enabled, None, |ui, nav| {
                let (response, open) = submenu_row(ui, text, |ui| {
                    ui.ctx().accesskit_node_builder(ui.unique_id(), |n| n.set_role(Role::Menu));
                    self.level(ui, depth + 1, &entries(children), nav);
                });
                if open {
                    // Where the submenu hangs from, to keep it below the bar.
                    nav.set_anchor(depth, response.rect);
                }
                access(&response, Role::MenuItem, label, None, None);
                response.ctx.accesskit_node_builder(response.id, |n| {
                    n.set_has_popup(HasPopup::Menu);
                    n.set_expanded(open);
                });
                (response, ())
            });
        });
    }
}

/// The mark column every row of a level keeps before its label, so that checked, unchecked
/// and plain rows (and submenu rows) start their labels at the same x.
struct Marks {
    font: egui::FontId,
    /// The column's width: the wider mark and a space.
    gutter: f32,
}

impl Marks {
    fn new(ui: &Ui, style: &BarStyle) -> Marks {
        let font = TextStyle::Button.resolve(ui.style());
        let w = |s: &str| ui.painter().layout_no_wrap(format!("{s} "), font.clone(), Color32::PLACEHOLDER).size().x;
        let gutter = w(&style.check_mark).max(w(&style.radio_mark));
        Marks { font, gutter }
    }

    /// A row's text: `mark` in the column, then the label. The text takes the button's color
    /// (dimmed when disabled).
    fn text(&self, ui: &Ui, mark: Option<&str>, label: &str) -> WidgetText {
        let plain = TextFormat { font_id: self.font.clone(), color: Color32::PLACEHOLDER, ..Default::default() };
        let mut job = LayoutJob::default();
        job.wrap.max_width = f32::INFINITY;
        let mut lead = self.gutter;
        if let Some(mark) = mark {
            let w = ui.painter().layout_no_wrap(mark.to_owned(), self.font.clone(), Color32::PLACEHOLDER).size().x;
            job.append(mark, 0.0, plain.clone());
            lead = (self.gutter - w).max(0.0);
        }
        job.append(label, lead, plain);
        job.into()
    }
}

/// Give a row its AccessKit role, its label without the mark, its checked state and its
/// shortcut. egui marks it disabled already.
fn access(r: &Response, role: Role, label: &str, checked: Option<bool>, shortcut: Option<&str>) {
    r.ctx.accesskit_node_builder(r.id, |n| {
        n.set_role(role);
        n.set_label(label);
        match checked {
            Some(true) => n.set_toggled(Toggled::True),
            Some(false) => n.set_toggled(Toggled::False),
            None => n.clear_toggled(),
        }
        if let Some(s) = shortcut {
            n.set_keyboard_shortcut(s);
        }
    });
}

/// A submenu row, drawn as `ui.menu_button` draws it, except that a row scrolled out of view
/// doesn't open its submenu. egui opens a submenu whenever the pointer is over its row, clipped
/// or not: a menu reopened scrolled down has rows hidden above its view, over the bar, and the
/// hidden one under the title would open on the press; the release on the title then counted
/// as a click outside that submenu and closed the whole menu. An open submenu (opened from the
/// keyboard, say) is drawn as usual. Returns the row and whether its submenu is shown.
fn submenu_row(ui: &mut Ui, title: WidgetText, content: impl FnOnce(&mut Ui)) -> (Response, bool) {
    let sub = SubMenu::id_from_widget_id(ui.next_auto_id());
    let open = MenuState::from_ui(ui, |state, _| state.open_item == Some(sub));
    let inactive = ui.style().visuals.widgets.inactive;
    if open {
        ui.style_mut().visuals.widgets.inactive = ui.style().visuals.widgets.open;
    }
    let response = ui.add(egui::Button::new(title).right_text(SubMenuButton::RIGHT_ARROW));
    ui.style_mut().visuals.widgets.inactive = inactive;
    // The rect egui tests the pointer against (`SubMenu::show`), against the part that can be seen.
    let reach = response.rect.expand2(ui.spacing().item_spacing / 2.0);
    let hidden = ui.ctx().pointer_hover_pos().is_some_and(|p| reach.contains(p) && !ui.clip_rect().contains(p));
    if hidden && !open {
        return (response, false);
    }
    let shown = SubMenu::new().show(ui, &response, content).is_some();
    (response, shown)
}

/// A title's open or close command this frame: the press opens a closed menu (and starts a
/// press-drag gesture: releasing on an item runs it) or closes an open one; the release never
/// toggles, so the menu doesn't blink. A keyboard or AccessKit click toggles it.
fn title_press(ctx: &egui::Context, title: &Response, press: Id) -> Option<egui::SetOpenCommand> {
    // A press this frame on the title (still down, or a whole click within one frame).
    let pressed = ctx.input(|i| i.pointer.primary_pressed()) && (title.is_pointer_button_down_on() || title.clicked());
    if !pressed {
        return (title.clicked() && !title.clicked_by(egui::PointerButton::Primary)).then_some(egui::SetOpenCommand::Toggle);
    }
    let open = egui::Popup::is_id_open(ctx, egui::Popup::default_response_id(title));
    if !open {
        ctx.data_mut(|d| d.insert_temp(press, true));
    }
    Some(egui::SetOpenCommand::Bool(!open))
}

/// Did the press-drag gesture that opened the menus end over `item` (released on it)?
fn released_on(ui: &Ui, item: &Response, press: Id) -> bool {
    item.enabled() && item.contains_pointer() && ui.input(|i| i.pointer.primary_released()) && ui.ctx().data(|d| d.get_temp::<bool>(press)).unwrap_or(false)
}

/// Whether the pointer can reach a title: a tall submenu that egui moved up over the bar is on
/// top there, and hovering it must not switch menus. The title's column reaches up to the top
/// of the window: egui lights a widget up to `interact_radius` away, so in a maximized window
/// a pointer pushed against the screen edge above the title lights it and switches to it too.
fn pointer_reaches_title(ctx: &egui::Context, response: &Response, p: egui::Pos2) -> bool {
    let r = response.interact_rect;
    let column = egui::Rect::from_min_max(egui::pos2(r.left(), f32::NEG_INFINITY), r.max);
    column.contains(p) && ctx.layer_id_at(p) == Some(response.layer_id)
}

/// As in a native menu bar: while one menu is open, the pointer moving onto another title opens
/// that menu instead.
fn switch_on_hover(ctx: &egui::Context, buttons: &[Response]) {
    let ids: Vec<Id> = buttons.iter().map(egui::Popup::default_response_id).collect();
    let Some(open) = ids.iter().position(|id| egui::Popup::is_id_open(ctx, *id)) else { return };
    // `Response::hovered` is false while the menu's popup layer is open: hit-test the title,
    // but only where its own layer is on top.
    let Some(p) = ctx.pointer_hover_pos() else { return };
    // Only a moving pointer switches: a pointer resting on a title must not undo ← and →.
    if ctx.input(|i| i.pointer.delta() == egui::Vec2::ZERO) {
        return;
    }
    if let Some(i) = buttons.iter().position(|b| pointer_reaches_title(ctx, b, p))
        && i != open
        && let Some(&id) = ids.get(i)
    {
        egui::Popup::open_id(ctx, id);
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests;
