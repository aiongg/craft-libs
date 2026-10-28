//! [`Menu`]: a popup of rows, separators and submenus, opened by its owner (a menu bar's title, a
//! button, a secondary click) from an egui [`Popup`] the owner anchors and opens.

use std::borrow::Cow;

use craft_theme::ControlSize;
use egui::containers::menu::{MenuConfig, MenuState, SubMenu};
use egui::style::StyleModifier;
use egui::{Align, Id, Layout, Popup, PopupCloseBehavior, PopupKind, Response, Sense, Ui, UiKind, UiStackInfo, Vec2, vec2};

use super::item::{MenuEntry, MenuItem, at_path};
use super::mnemonic;
use super::nav::{MenuStep, Nav, Row, Strips};
use super::row::{self, Columns, MenuStyle, RowContent, RowPainter, RowState};
use crate::popup_list;
use crate::primitives::Interactive;
use crate::theme::{Tokens, TokensOf as _, UiTokens};

/// A menu: rows, separators and submenus in a popup, driven by the pointer and the keyboard.
///
/// ```
/// use craft_ui::menu::{Menu, MenuEntry, MenuItem};
/// use egui::Popup;
/// # egui::__run_test_ui(|ui| {
/// let entries: Vec<MenuEntry<&str>> = vec![MenuItem::new("Copy", "copy").into(), MenuItem::new("Paste", "paste").into()];
/// let button = ui.button("Edit");
/// let shown = Menu::new(&entries).show(ui, Popup::menu(&button));
/// if let Some(command) = shown.chosen {
///     // Run it.
/// }
/// # });
/// ```
///
/// The owner makes the egui [`Popup`]: its id, where it hangs and when it opens (egui's popup
/// memory, [`Popup::open_memory`]). The menu sets the rest (kind, layout, look, close behaviour)
/// and draws in it, so a context menu, a press menu and a menu bar each open one from their own
/// anchor and gesture. Its state (the highlighted row per level, the pointer heading for a
/// submenu) lives in egui memory under the popup's id, and starts fresh each time it opens.
///
/// Look: list box rows at the menu's size (`listbox.*`), touching, as wide as the widest row and
/// at least `menu.min_width`, never wrapping; a mark column at the reading start on every row of a
/// level that has a check or radio item, an icon column on every row of a level that has an icon;
/// shortcuts right-aligned in [`Kbd`](crate::controls::Kbd)'s text form in `menu.shortcut`; the
/// popup frame is `surface.raised` with `radius.popup` and `elevation.popup`. The active row (the
/// pointer's or the keyboard's) has `menu.active.fill` with everything on it in
/// `menu.active.text`: the menu keeps the keyboard focus away from its rows, so that fill is the
/// visible focus. A level taller than the window scrolls, with ▲ / ▼ strips that scroll while the
/// pointer rests on them.
///
/// Pointer: a click on an enabled row runs it and closes every level; a click on a disabled row or
/// a separator leaves the menu open; a click outside closes it. Submenus open on hover with no
/// delay and stay open while the pointer heads for them (PhotoCraft's aim triangle, 24 pt of
/// slack, let go after 0.3 s at rest). With [`pressing`](Menu::pressing), the release of the
/// press that opened the menu runs the enabled row it ends on.
///
/// Keys: ↑ / ↓ move the highlight over enabled rows and wrap; → opens a submenu and ← closes one
/// (mirrored right to left); Enter and Space run the highlighted row or open its submenu; Esc
/// closes every level. Where menus have [`mnemonics`](super::mnemonics), a letter runs the row
/// with that mnemonic in the level the keys are in (or opens its submenu); the mnemonics are
/// underlined while Alt is held or after the menu was opened from the keyboard. Keys that would
/// leave the menu at its top level (← there, → on a row without a submenu) are reported in
/// [`MenuResponse::step`] for a menu bar.
///
/// AccessKit: the popup is a `Menu`; rows are `MenuItem`, `MenuItemCheckBox` or `MenuItemRadio`,
/// named by their label without the mark, with `toggled`, the mnemonic as access key and the
/// shortcut as `keyboard_shortcut`; submenu rows report a menu popup and whether it is
/// `expanded`. While open, the menu is recorded with the other open popup lists, so FieldTab
/// and other key readers leave its keys alone.
#[must_use = "You should call .show"]
pub struct Menu<'a, T> {
    entries: &'a [MenuEntry<T>],
    size: ControlSize,
    pressing: bool,
    keyboard: bool,
    top_limit: Option<f32>,
    scroll_up: Cow<'a, str>,
    scroll_down: Cow<'a, str>,
    header: Option<&'a mut dyn FnMut(&mut Ui) -> bool>,
}

/// What a menu did in a pass, from [`Menu::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MenuResponse<T> {
    /// The command of the row run this pass (clicked, released on, Enter, Space or its
    /// mnemonic). The menu has closed.
    pub chosen: Option<T>,
    /// The menu is open after this pass.
    pub open: bool,
    /// A key asked for the neighbouring menu of a bar: ← at the top level, or → on a row without
    /// a submenu. The menu stays open; a bar opens the other one.
    pub step: Option<MenuStep>,
    /// The highlighted row in the level the keys move in, for the owner's AccessKit
    /// `active_descendant`.
    pub active: Option<Id>,
    /// The popup's response while it is shown.
    pub popup: Option<Response>,
}

impl<'a, T: Clone> Menu<'a, T> {
    pub fn new(entries: &'a [MenuEntry<T>]) -> Self {
        Self {
            entries,
            size: ControlSize::Md,
            pressing: false,
            keyboard: false,
            top_limit: None,
            scroll_up: Cow::Borrowed("Scroll menu up"),
            scroll_down: Cow::Borrowed("Scroll menu down"),
            header: None,
        }
    }

    /// The rows' size; [`ControlSize::Md`] by default.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// The press that opened the menu is still down, or ends in this pass: its release on an
    /// enabled row runs that row (press-drag-release). The owner tracks the press.
    pub fn pressing(mut self, pressing: bool) -> Self {
        self.pressing = pressing;
        self
    }

    /// The menu opens from the keyboard in this pass: its first enabled row is highlighted and
    /// its mnemonics underlined. Read only on the pass the menu opens.
    pub fn opened_by_keyboard(mut self, keyboard: bool) -> Self {
        self.keyboard = keyboard;
        self
    }

    /// No level of the menu goes above `y`: a menu bar's bottom, which a tall submenu would
    /// otherwise cover. Levels that don't fit scroll instead.
    pub fn below(mut self, y: f32) -> Self {
        self.top_limit = Some(y);
        self
    }

    /// The accessible names of the scroll strips, translated; "Scroll menu up" and "Scroll menu
    /// down" by default.
    pub fn scroll_labels(mut self, up: impl Into<Cow<'a, str>>, down: impl Into<Cow<'a, str>>) -> Self {
        self.scroll_up = up.into();
        self.scroll_down = down.into();
        self
    }

    /// Content above the top level's rows, scrolling with them (a menu bar's search field).
    /// `header` returns whether it holds the keyboard focus: the letters, ← / → and Space are then
    /// its own, not the menu's.
    pub(crate) fn header(mut self, header: &'a mut dyn FnMut(&mut Ui) -> bool) -> Self {
        self.header = Some(header);
        self
    }

    /// Draws the menu in `popup` while the popup is open, and handles its keys.
    pub fn show(self, ui: &Ui, popup: Popup<'_>) -> MenuResponse<T> {
        let ctx = ui.ctx().clone();
        let id = popup.get_id();
        let fresh = ctx.read_response(id).is_none();
        let tokens = Tokens::of(ui);
        let style = MenuStyle::new(&tokens, self.size);
        let modifier = popup_style(ui);
        let config = MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside).style(modifier.clone());
        let mnemonics = mnemonic::mnemonics(&ctx);
        let mut nav = Nav::load(&ctx, id, fresh);
        if fresh && self.keyboard {
            nav.highlight_first(0);
            nav.underline = true;
        }
        nav.top_limit = self.top_limit;
        let underline = mnemonics && (nav.underline || ctx.input(|i| i.modifiers.alt));
        let released = ctx.input(|i| i.pointer.primary_released());
        let mut draw = Draw {
            painter: RowPainter { style: &style, tokens: &tokens, size: self.size },
            pressing: self.pressing && released,
            mnemonics,
            underline,
            strips: Strips {
                height: style.scroll_strip,
                live: style.rows.text,
                dead: style.rows.text_disabled,
                up: &self.scroll_up,
                down: &self.scroll_down,
            },
            chosen: None,
            header: self.header.map(|h| h as &mut dyn FnMut(&mut Ui) -> bool),
            typing: false,
        };
        let shown = popup
            .kind(PopupKind::Menu)
            .layout(Layout::top_down(Align::Min))
            .style(modifier)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .info(UiStackInfo::new(UiKind::Menu).with_tag_value(MenuConfig::MENU_CONFIG_TAG, config))
            .show(|ui| draw.level(ui, 1, id, &[], self.entries, &mut nav));
        let mut chosen = draw.chosen;
        let mut open = shown.as_ref().is_some_and(|r| !r.response.should_close());
        let mut step = None;
        if open {
            let keys = nav.keys(&ctx, crate::direction::is_rtl(&ctx), mnemonics, draw.typing);
            chosen = chosen.or(keys.run);
            step = keys.step;
            if keys.close || chosen.is_some() {
                Popup::close_id(&ctx, id);
                open = false;
            }
        }
        popup_list::record_open(&ctx, id, open);
        let active = nav.active().filter(|_| open);
        if open {
            nav.store(&ctx, id);
        } else {
            Nav::forget(&ctx, id);
        }
        MenuResponse {
            chosen: chosen.and_then(|path| at_path(self.entries, &path)?.command().cloned()),
            open,
            step,
            active,
            popup: shown.map(|r| r.response),
        }
    }
}

impl Menu<'static, ()> {
    /// A menu's sizes and colours at `size`.
    pub fn style(tokens: &UiTokens, size: ControlSize) -> MenuStyle {
        MenuStyle::new(tokens, size)
    }
}

/// The style of a menu's popups, submenus included: the theme scope the menu opens in (a popup's
/// `Ui` starts from the global style), with no wrapping, so rows widen the menu.
fn popup_style(ui: &Ui) -> StyleModifier {
    let scoped = crate::theme::scoped_style(ui);
    StyleModifier::new(move |s: &mut egui::Style| {
        if let Some(scoped) = &scoped {
            *s = (**scoped).clone();
        }
        s.wrap_mode = Some(egui::TextWrapMode::Extend);
    })
}

/// What the rows need while drawing, and the row run.
struct Draw<'s> {
    painter: RowPainter<'s>,
    /// The press that opened the menu ends in this pass.
    pressing: bool,
    mnemonics: bool,
    underline: bool,
    strips: Strips<'s>,
    /// The path of the row clicked or released on.
    chosen: Option<Vec<usize>>,
    /// Drawn above the top level's rows; returns whether it has the keyboard focus.
    header: Option<&'s mut dyn FnMut(&mut Ui) -> bool>,
    /// The header has the keyboard focus.
    typing: bool,
}

impl Draw<'_> {
    /// One level of the menu, scrolling when taller than the window. `scope` is the popup's id or
    /// the submenu row's, `path` the submenu row's entry path.
    fn level<T>(&mut self, ui: &mut Ui, depth: usize, scope: Id, path: &[usize], entries: &[MenuEntry<T>], nav: &mut Nav) {
        let strips = Strips { up: self.strips.up, down: self.strips.down, ..self.strips };
        super::nav::level(ui, depth, nav, &strips, |ui, nav| {
            if depth == 1
                && let Some(header) = self.header.as_mut()
            {
                self.typing = header(ui);
            }
            self.rows(ui, depth, scope, path, entries, nav);
        });
    }

    fn rows<T>(&mut self, ui: &mut Ui, depth: usize, scope: Id, path: &[usize], entries: &[MenuEntry<T>], nav: &mut Nav) {
        // Rows touch, as in native menus.
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let items: Vec<(usize, &MenuItem<T>)> = entries.iter().enumerate().filter_map(|(i, e)| Some((i, e.item()?))).collect();
        let labels: Vec<&str> = items.iter().map(|(_, it)| it.label()).collect();
        // Mnemonics are unique within the level, separators and rows without one aside.
        let mut shown: Vec<String> = labels.iter().map(|l| (*l).to_owned()).collect();
        let mut keys = vec![None; items.len()];
        if self.mnemonics {
            let keyed: Vec<usize> = (0..items.len()).filter(|&k| items.get(k).is_some_and(|(_, it)| it.takes_mnemonic())).collect();
            let of = |k: &usize| items.get(*k).map(|(_, it)| *it);
            let labels: Vec<&str> = keyed.iter().filter_map(of).map(MenuItem::label).collect();
            let english: Vec<Option<&str>> = keyed.iter().filter_map(of).map(MenuItem::english_of).collect();
            let preferred: Vec<Option<char>> = keyed.iter().filter_map(of).map(MenuItem::preferred).collect();
            let (keyed_shown, keyed_keys) = mnemonic::localized(&labels, &english, &preferred);
            for ((k, text), key) in keyed.iter().zip(keyed_shown).zip(keyed_keys) {
                if let (Some(s), Some(m)) = (shown.get_mut(*k), keys.get_mut(*k)) {
                    *s = text;
                    *m = key;
                }
            }
        }
        let content = |k: usize, it: &'_ MenuItem<T>| -> (String, Option<char>) {
            (shown.get(k).cloned().unwrap_or_else(|| it.label().to_owned()), keys.get(k).copied().flatten())
        };
        let mut cols = Columns {
            marks: items.iter().any(|(_, it)| it.item_kind().checked().is_some()),
            icons: items.iter().any(|(_, it)| it.icon_of().is_some()),
            width: self.painter.style.min_width,
        };
        for (k, (_, it)) in items.iter().enumerate() {
            let (label, _) = content(k, it);
            let row = row_content(it, &label, None);
            cols.width = cols.width.max(self.painter.width(ui, &row, &cols));
        }
        let mut k = 0;
        let mut last_was_sep = true;
        for (i, entry) in entries.iter().enumerate() {
            match entry {
                MenuEntry::Separator => {
                    let more = entries.get(i + 1..).is_some_and(|rest| rest.iter().any(|e| e.item().is_some()));
                    if !last_was_sep && more {
                        self.painter.separator(ui, cols.width);
                        last_was_sep = true;
                    }
                }
                MenuEntry::Item(it) => {
                    let (label, mnemonic) = content(k, it);
                    k += 1;
                    self.item(ui, depth, scope, path, i, it, (&label, mnemonic), &cols, nav);
                    last_was_sep = false;
                }
            }
        }
    }

    /// The row of `it`, entry `index` of its level; `shown` is its label as shown and its
    /// mnemonic.
    #[allow(clippy::too_many_arguments)]
    fn item<T>(
        &mut self,
        ui: &mut Ui,
        depth: usize,
        scope: Id,
        path: &[usize],
        index: usize,
        it: &MenuItem<T>,
        shown: (&str, Option<char>),
        cols: &Columns,
        nav: &mut Nav,
    ) {
        let (label, mnemonic) = shown;
        let enabled = it.is_enabled();
        let widget = scope.with(("menu-row", index));
        let row_path: Vec<usize> = path.iter().copied().chain([index]).collect();
        let children = it.children();
        let underline = mnemonic.filter(|_| self.underline);
        let content = row_content(it, label, underline);
        let row = Row { widget, enabled, path: row_path.clone(), submenu: children.is_some(), mnemonic };
        let height = self.painter.style.rows.height;
        nav.row(ui, depth - 1, row, |ui, nav, lit| {
            let r = Interactive::with_id(widget, row::role(it.item_kind()), label)
                .sense(Sense::CLICK)
                .enabled(enabled)
                .state_layer(false)
                .focus_ring(false)
                .interact(ui, vec2(cols.width, height))
                .finish(ui);
            let open = match children {
                Some(children) => {
                    let open = submenu_row(ui, &r, |ui| self.level(ui, depth + 1, widget, &row_path, children, nav));
                    if open {
                        // Where the submenu hangs from, to keep it below a bar.
                        nav.set_anchor(depth, r.rect);
                    }
                    open
                }
                None => {
                    let released_on = self.pressing && r.contains_pointer();
                    if enabled && (r.clicked() || released_on) {
                        self.chosen = Some(row_path.clone());
                        ui.close();
                    }
                    false
                }
            };
            self.painter.paint(ui, r.rect, &content, cols, RowState { enabled, active: lit || open });
            row::access(&r, &RowContent { mnemonic, ..content }, open);
            (r, ())
        });
    }
}

fn row_content<'r, T>(it: &'r MenuItem<T>, label: &'r str, mnemonic: Option<char>) -> RowContent<'r> {
    RowContent {
        label,
        mnemonic,
        icon: it.icon_of(),
        kind: it.item_kind(),
        shortcut: it.shortcut_of().filter(|_| it.children().is_none()),
        submenu: it.children().is_some(),
    }
}

/// Shows the submenu of the row `row` with `content` as egui's submenu button does, except that a
/// row scrolled out of view doesn't open it. egui opens a submenu whenever the pointer is over its
/// row, clipped or not: a menu reopened scrolled down has rows hidden above its view, over the
/// bar, and the hidden one under the title would open on the press; the release on the title then
/// counted as a click outside that submenu and closed the whole menu. An open submenu (opened
/// from the keyboard, say) is drawn as usual. Returns whether the submenu is shown.
fn submenu_row(ui: &Ui, row: &Response, content: impl FnOnce(&mut Ui)) -> bool {
    let sub = SubMenu::id_from_widget_id(row.id);
    let open = MenuState::from_ui(ui, |state, _| state.open_item == Some(sub));
    // The rect egui tests the pointer against (`SubMenu::show`), against the part that can be seen.
    let reach = row.rect.expand2(ui.spacing().item_spacing / 2.0);
    let hidden = ui.ctx().pointer_hover_pos().is_some_and(|p| reach.contains(p) && !ui.clip_rect().contains(p));
    if hidden && !open {
        return false;
    }
    SubMenu::new().show(ui, row, content).is_some()
}
