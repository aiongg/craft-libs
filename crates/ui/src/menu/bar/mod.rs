//! [`MenuBar`]: a row of menu titles, each opening a [`Menu`](super::Menu), driven as PhotoCraft's
//! bar is.
//!
//! A press on a title opens its menu (releasing over a row runs it), a second press or Esc closes
//! it, and while one is open the pointer moving onto another title opens that one. ← and → at a
//! menu's top level open the neighbouring menu. F10 puts the keyboard on the bar, and where menus
//! have [`mnemonics`](super::mnemonics) a tap on Alt does too and Alt+letter opens a menu at once
//! ([`keys`]). Titles that don't fit move into a trailing "⋯" menu, or all of
//! them into one "Menu". The Help menu can start with a search field over every command of the bar
//! ([`search`]).

mod keys;
mod search;
#[cfg(test)]
mod tests;

pub use keys::{is_focused, leave_for_shortcut};

use std::sync::Arc;

use craft_theme::ControlSize;
use egui::accesskit::{HasPopup, Role};
use egui::emath::GuiRounding as _;
use egui::text::LayoutJob;
use egui::{
    Color32, Galley, Id, Key, KeyboardShortcut, Modifiers, PointerButton, Popup, Pos2, Rect, Response, Sense, SetOpenCommand, Ui, pos2, vec2,
};

use super::item::{MenuEntry, MenuItem};
use super::{Menu, MenuStep, MenuStyle, mnemonic};
use crate::controls::Kbd;
use crate::direction;
use crate::primitives::{Content, Interactive, Text};
use crate::story::{Level, SizeArg, Story, StoryArgs, Surface};
use crate::theme::{Tokens, TokensOf as _, UiTokens, role_format};
use crate::typed_story;
use keys::Keys;
use search::Search;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["MenuBar"];

/// The labels a menu bar writes itself, translated by the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuBarLabels {
    /// The menu of the titles that don't fit: "⋯".
    pub more: String,
    /// The one menu that holds every title when none fits: "Menu".
    pub all: String,
    /// The Help menu's search field: its name and placeholder, "Search menus".
    pub search: String,
    /// The search field's clear button: "Clear".
    pub clear: String,
    /// Shown below the search field when no command matches: "No matching commands".
    pub no_matches: String,
    /// The accessible names of a tall menu's scroll strips.
    pub scroll_up: String,
    pub scroll_down: String,
}

impl Default for MenuBarLabels {
    fn default() -> Self {
        Self {
            more: "⋯".into(),
            all: "Menu".into(),
            search: "Search menus".into(),
            clear: "Clear".into(),
            no_matches: "No matching commands".into(),
            scroll_up: "Scroll menu up".into(),
            scroll_down: "Scroll menu down".into(),
        }
    }
}

/// A menu bar: a row of titles, each opening a [`Menu`](super::Menu) of its entries.
///
/// ```
/// use craft_ui::menu::{MenuBar, MenuItem};
/// # egui::__run_test_ui(|ui| {
/// let menus = vec![
///     MenuItem::submenu("File", vec![MenuItem::new("New", "new").into(), MenuItem::new("Open…", "open").into()]),
///     MenuItem::submenu("Edit", vec![MenuItem::new("Undo", "undo").into()]),
/// ];
/// if let Some(command) = MenuBar::new(&menus).show(ui).chosen {
///     // Run it.
/// }
/// # });
/// ```
///
/// Each menu is a [`MenuItem::submenu`]: its label is the title, with its preferred
/// [`mnemonic`](MenuItem::mnemonic) and [`english`](MenuItem::english) label, and its entries are
/// the menu. Titles stay enabled while every row under them is disabled, as in a native bar.
///
/// Pointer: a press on a title opens its menu and a press on the open title closes it; the release
/// never toggles. Releasing the press over an enabled row runs it; releasing over the title or
/// nothing leaves the menu open. While a menu is open, the pointer moving onto another title opens
/// that one (a pointer at rest doesn't, so it doesn't undo ← and →).
///
/// Keys: an open menu has the [`Menu`](super::Menu)'s keys, and ← / → at its top level open the
/// neighbouring menu, wrapping (mirrored right to left, where the titles run from the right).
///
/// - F10 puts the keyboard on the bar: the first title is lit, ← / → move, ↓, Enter and Space
///   open the lit menu, Esc, F10 again or a click leaves. A menu opened from the keyboard lights
///   its first enabled row. This works on every platform, a browser on a Mac included: the bar is
///   the app's only menu bar wherever it is drawn. An app that draws it beside the native macOS
///   bar turns its keys off with [`keyboard`](MenuBar::keyboard).
///
/// Where menus have [`mnemonics`](super::mnemonics) (not on macOS, where Option types):
///
/// - a tap on Alt puts the keyboard on the bar as F10 does, and a second tap leaves;
/// - a title's letter on the focused bar opens it, and Alt+letter opens the menu with that
///   mnemonic at once, also while another one is open;
/// - the mnemonics are underlined while Alt is held, the bar has the keyboard or a menu was
///   opened from it;
/// - titles take their preferred letter where the label has it, else the first unused one; a
///   title without a Latin letter shows its English label's letter after it, "文件(F)".
///
/// While the bar has the keyboard or a menu is open, the app leaves the keys to it ([`is_open`],
/// [`is_focused`]); [`leave_for_shortcut`] lets a command shortcut still run.
///
/// Titles that don't fit move into a trailing "⋯" menu that holds them as submenus, or all of
/// them into one "Menu"; [`shrink_to_fit`](MenuBar::shrink_to_fit) leaves the rest of the row to
/// other controls.
///
/// Look: titles at the menu row height with its padding and text; the title under the pointer
/// takes the list box's active fill, and the open or keyboard-lit title the menu's
/// `menu.active.fill` with `menu.active.text`.
///
/// AccessKit: the bar is a `MenuBar`; titles are `MenuItem`s with a menu popup, `expanded` while
/// open, and their mnemonic as access key.
#[must_use = "You should call .show"]
pub struct MenuBar<'a, T> {
    menus: &'a [MenuItem<T>],
    id: Id,
    size: ControlSize,
    labels: Option<&'a MenuBarLabels>,
    fit: Option<f32>,
    /// The Help menu's index, and how its search tells commands apart.
    search: Option<(usize, search::Same<T>)>,
    keyboard: bool,
    text_editing: bool,
}

/// What a menu bar did in a pass, from [`MenuBar::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct MenuBarResponse<T> {
    /// The command run this pass (clicked, released on, Enter, Space, its mnemonic or a search
    /// result). The menu has closed.
    pub chosen: Option<T>,
    /// A menu of the bar is open after this pass.
    pub open: bool,
    /// The keyboard is on the bar with no menu open (Alt tap, F10).
    pub focused: bool,
    /// The bar.
    pub response: Response,
    /// The menu titles alone (with "⋯" when it shows), from the first title's reading start to the
    /// last title's inner edge. With no titles, an empty rect at the bar's reading start.
    pub titles: Rect,
}

/// Bar state between passes.
#[derive(Clone, Debug, Default)]
struct BarState {
    keys: Keys,
    /// A press on a title opened a menu and the button is still down (or comes up in this pass).
    pressing: bool,
    /// The menu that opens next was opened from the keyboard: it lights its first row.
    keyboard: bool,
}

/// A title of the bar: a menu, or the overflow menu holding the menus from `from` on as submenus
/// ("Menu" when `from` is 0, else "⋯").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Top {
    Menu(usize),
    Rest(usize),
}

fn open_id() -> Id {
    Id::unique("craft-ui-menu-bar-open")
}

/// The bars with a menu open, each with the last pass it was seen open in.
#[derive(Clone, Debug, Default)]
struct OpenBars(Vec<(Id, u64)>);

/// Whether a menu of a bar is open. The app leaves the keys to it meanwhile: its shortcuts wait
/// until the menu closes. Only while the bar is still drawn: an app that hides it with a menu open
/// (full screen) gets its keys back. [`any_open`](super::any_open) also covers context and press
/// menus.
pub fn is_open(ctx: &egui::Context) -> bool {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<OpenBars>(open_id())).is_some_and(|bars| bars.0.iter().any(|&(_, seen)| seen + 2 >= pass))
}

/// Records whether the bar `bar` has a menu open after this pass, beside the other bars.
fn record_open(ctx: &egui::Context, bar: Id, open: bool) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        let bars = &mut d.get_temp_mut_or_default::<OpenBars>(open_id()).0;
        bars.retain(|&(id, seen)| id != bar && seen + 2 >= pass);
        if open {
            bars.push((bar, pass));
        }
    });
}

impl<'a, T: Clone> MenuBar<'a, T> {
    /// A bar of `menus`, each a [`MenuItem::submenu`] (a command item is a title with an empty
    /// menu).
    pub fn new(menus: &'a [MenuItem<T>]) -> Self {
        Self {
            menus,
            id: Id::unique("craft-ui-menu-bar"),
            size: ControlSize::Md,
            labels: None,
            fit: None,
            search: None,
            keyboard: true,
            text_editing: false,
        }
    }

    /// The bar's id, where its state lives in egui memory: tells two bars in one window apart.
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    /// The titles' and menus' size; [`ControlSize::Md`] by default.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// The labels the bar writes itself, translated; [`MenuBarLabels::default`] otherwise.
    pub fn labels(mut self, labels: &'a MenuBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Take only the width the titles need, at most `max_width` (`f32::INFINITY` for no limit)
    /// and the room the `Ui` has, so that other controls can share the row; past that the menus
    /// overflow into "⋯". By default the bar fills the row.
    pub fn shrink_to_fit(mut self, max_width: f32) -> Self {
        self.fit = Some(max_width);
        self
    }

    /// The menu at `index` (the Help menu) starts with a search field over every command of the
    /// bar, as PhotoCraft's Help menu: typing lists up to 15 matches by label or menu path, each
    /// shown with its path ("Image › Adjustments › Levels…"); a click, ↓ then Enter, or Enter in
    /// the field runs one. The field has the keys while focused, so letters type into it. A menu
    /// moved into "⋯" has no field.
    ///
    /// A command in several menus is listed once, at its best match: hence `T: PartialEq`, the
    /// only place the bar compares commands.
    pub fn help_search(mut self, index: usize) -> Self
    where
        T: PartialEq,
    {
        self.search = Some((index, T::eq));
        self
    }

    /// Whether the bar takes F10, Alt and Alt+letter; on by default. Off while a dialog or a
    /// shortcut recorder has the keys, and where the native macOS bar runs beside this one. Menus
    /// open from the pointer either way.
    pub fn keyboard(mut self, on: bool) -> Self {
        self.keyboard = on;
        self
    }

    /// A text field has the keys (also one egui doesn't know of): a letter typed while the bar has
    /// the keyboard goes on to the field and takes the keyboard off the bar. A focused egui text
    /// field counts without this.
    pub fn text_editing(mut self, editing: bool) -> Self {
        self.text_editing = editing;
        self
    }

    /// Draws the bar and its open menu, and handles their keys.
    pub fn show(self, ui: &mut Ui) -> MenuBarResponse<T> {
        let ctx = ui.ctx().clone();
        let tokens = Tokens::of(ui);
        let style = Menu::style(&tokens, self.size);
        let default_labels = MenuBarLabels::default();
        let labels = self.labels.unwrap_or(&default_labels);
        let mut state: BarState = ctx.data(|d| d.get_temp(self.id)).unwrap_or_default();
        state.keys.take_leave(&ctx);
        let mnemonics = mnemonic::mnemonics(&ctx);
        let keys_on = self.keyboard;
        let titles = Titles { menus: self.menus, labels, tokens: &tokens, style: &style, mnemonics };

        let room = self.fit.map_or(ui.available_width(), |max| max.min(ui.available_width())).max(0.0);
        let width = match self.fit {
            // The half point keeps rounding from pushing the last title into "⋯".
            Some(_) => (titles.widths(ui, &titles.all()).iter().sum::<f32>() + 0.5).min(room),
            None => room,
        };
        let tops = titles.fit(ui, width);
        let (shown, letters) = titles.texts(&tops);
        let title_ids: Vec<Id> = (0..tops.len()).map(|k| self.id.with(("title", k))).collect();
        let popups: Vec<Id> = title_ids.iter().map(|id| id.with("popup")).collect();
        let open_before = popups.iter().any(|id| Popup::is_id_open(&ctx, *id));
        if keys_on {
            state.keys.begin(open_before, state.keyboard);
        } else {
            state.keys.off();
        }
        let underline = keys_on && mnemonics && state.keys.show_underlines(&ctx);
        let mut search = self.search.map(|(_, same)| Search::load(&ctx, self.id, same));
        let help = self.search.map(|(i, _)| i);

        let height = style.rows.height;
        let mut chosen = None;
        let mut step = None;
        let mut shown_open = None;
        let mut responses = Vec::with_capacity(tops.len());
        let bar = ui.allocate_ui_with_layout(vec2(width, height), direction::row(ui), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ctx.accesskit_node_builder(ui.unique_id(), |n| n.set_role(Role::MenuBar));
            let bottom = ui.max_rect().bottom();
            for (k, &top) in tops.iter().enumerate() {
                let text = shown.get(k).map_or("", String::as_str);
                let letter = letters.get(k).copied().flatten();
                let w = titles.text_width(ui, text) + 2.0 * style.rows.pad_x;
                let id = title_ids.get(k).copied().unwrap_or(self.id);
                let title =
                    Interactive::with_id(id, Role::MenuItem, text).sense(Sense::CLICK).state_layer(false).focus_ring(false).show(ui, vec2(w, height));
                let popup_id = Popup::default_response_id(&title);
                let was_open = Popup::is_id_open(&ctx, popup_id);
                let command = title_press(&ctx, &title, &mut state.pressing);
                if command == Some(SetOpenCommand::Toggle) {
                    // An AccessKit click: the menu opens as from the keyboard.
                    state.keyboard = true;
                }
                let shows = match command {
                    Some(SetOpenCommand::Bool(open)) => open,
                    Some(SetOpenCommand::Toggle) => !was_open,
                    None => was_open,
                };
                let popup = Popup::from_response(&title).open_memory(command).align(direction::below(ui)).gap(0.0);
                // Owned entries (the overflow menu's, the Help menu's with its search results) are
                // built only while that menu shows.
                let owned: Option<Vec<MenuEntry<T>>>;
                let mut header = None;
                let entries: &[MenuEntry<T>] = match top {
                    Top::Menu(i) => {
                        let children = self.menus.get(i).and_then(MenuItem::children).unwrap_or(&[]);
                        owned = match search.as_mut().filter(|_| shows && help == Some(i)) {
                            Some(search) => {
                                search.begin(&ctx, popup_id, self.menus);
                                let entries = search.entries(children);
                                header = Some(search);
                                entries
                            }
                            None => None,
                        };
                        owned.as_deref().unwrap_or(children)
                    }
                    Top::Rest(from) if shows => {
                        owned = Some(self.menus.get(from..).unwrap_or(&[]).iter().cloned().map(MenuEntry::Item).collect());
                        owned.as_deref().unwrap_or(&[])
                    }
                    Top::Rest(_) => &[],
                };
                let menu = Menu::new(entries)
                    .size(self.size)
                    .pressing(state.pressing)
                    .opened_by_keyboard(state.keyboard)
                    .below(bottom)
                    .scroll_labels(labels.scroll_up.as_str(), labels.scroll_down.as_str());
                let out = match header {
                    Some(search) => {
                        let mut draw = |ui: &mut Ui| search.field(ui, &style, &tokens, self.size, labels);
                        let out = menu.header(&mut draw).show(ui, popup);
                        if out.chosen.is_none() && out.open && search.submitted() {
                            // Enter in the field with no row lit runs the first match.
                            let first = search.first_enabled();
                            if first.is_some() {
                                Popup::close_id(&ctx, popup_id);
                            }
                            if chosen.is_none() {
                                chosen = first;
                            }
                        }
                        out
                    }
                    None => menu.show(ui, popup),
                };
                let open = out.open && Popup::is_id_open(&ctx, popup_id);
                if chosen.is_none() {
                    chosen = out.chosen;
                }
                if open {
                    shown_open = Some(k);
                    step = step.or(out.step.map(|s| (k, s)));
                }
                let lit = open || state.keys.focus == Some(k);
                paint_title(ui, &titles, &title, text, letter.filter(|_| underline), lit);
                ctx.accesskit_node_builder(title.id, |n| {
                    n.set_has_popup(HasPopup::Menu);
                    n.set_expanded(open);
                    if let Some(m) = letter {
                        n.set_access_key(m.to_string());
                    }
                });
                responses.push(title);
            }
        });
        if let Some(search) = search {
            search.store(&ctx);
        }
        if shown_open.is_some() {
            state.keyboard = false;
        }
        if let Some((k, s)) = step {
            let n = popups.len();
            let next = match s {
                MenuStep::Next => (k + 1) % n,
                MenuStep::Previous => (k + n - 1) % n,
            };
            if let Some(&id) = popups.get(next) {
                Popup::open_id(&ctx, id);
                state.keyboard = true;
                ctx.request_repaint();
            }
        } else {
            switch_on_hover(&ctx, &responses, &popups);
        }
        if keys_on {
            let typing = self.text_editing || ctx.text_edit_focused();
            state.keys.keys(&ctx, &popups, &letters, mnemonics, shown_open.is_some(), typing, &mut state.keyboard);
        }
        // The keyboard on a title while no widget has egui's focus (Alt or F10): the title is
        // AccessKit's focus. An open menu's lit row takes it instead.
        if shown_open.is_none()
            && ctx.memory(|m| m.focused()).is_none()
            && let Some(title) = state.keys.focus.and_then(|k| responses.get(k))
        {
            ctx.set_accesskit_focus(title.id);
        }
        // The press-drag gesture ends with the button (its release was handled by the rows).
        if ctx.input(|i| i.pointer.primary_released() || !i.pointer.primary_down()) {
            state.pressing = false;
        }
        let open = popups.iter().any(|id| Popup::is_id_open(&ctx, *id));
        let focused = state.keys.focus.is_some();
        state.keys.store_focus(&ctx);
        record_open(&ctx, self.id, open);
        ctx.data_mut(|d| d.insert_temp(self.id, state));
        let titles_rect = responses.iter().map(|r| r.rect).reduce(Rect::union).unwrap_or_else(|| {
            let edge = direction::start_x(ui, bar.response.rect);
            Rect::from_min_max(pos2(edge, bar.response.rect.top()), pos2(edge, bar.response.rect.bottom()))
        });
        MenuBarResponse { chosen, open, focused, response: bar.response, titles: titles_rect }
    }
}

/// What the titles need for measuring and drawing.
struct Titles<'s, T> {
    menus: &'s [MenuItem<T>],
    labels: &'s MenuBarLabels,
    tokens: &'s UiTokens,
    style: &'s MenuStyle,
    mnemonics: bool,
}

impl<T> Titles<'_, T> {
    fn all(&self) -> Vec<Top> {
        (0..self.menus.len()).map(Top::Menu).collect()
    }

    fn galley(&self, ui: &Ui, text: &str, mnemonic: Option<char>, color: Color32) -> Arc<Galley> {
        let format = role_format(self.tokens, self.style.rows.label, color);
        let mut job = LayoutJob::default();
        job.wrap.max_width = f32::INFINITY;
        mnemonic::append_underlined(&mut job, text, mnemonic, &format);
        ui.painter().layout_job(job)
    }

    fn text_width(&self, ui: &Ui, text: &str) -> f32 {
        self.galley(ui, text, None, Color32::PLACEHOLDER).size().x
    }

    /// The titles of `tops` as shown, with any "(F)" they take, and their mnemonics.
    fn texts(&self, tops: &[Top]) -> (Vec<String>, Vec<Option<char>>) {
        let name = |top: &Top| -> (&str, Option<&str>, Option<char>) {
            match *top {
                Top::Menu(i) => self.menus.get(i).map_or(("", None, None), |m| (m.label(), m.english_of(), m.preferred())),
                Top::Rest(0) => (&self.labels.all, Some("Menu"), None),
                Top::Rest(_) => (&self.labels.more, Some("⋯"), None),
            }
        };
        let names: Vec<_> = tops.iter().map(name).collect();
        let labels: Vec<&str> = names.iter().map(|n| n.0).collect();
        if !self.mnemonics {
            return (labels.iter().map(|l| (*l).to_owned()).collect(), vec![None; labels.len()]);
        }
        let english: Vec<Option<&str>> = names.iter().map(|n| n.1).collect();
        let preferred: Vec<Option<char>> = names.iter().map(|n| n.2).collect();
        mnemonic::localized(&labels, &english, &preferred)
    }

    /// Each title's width as shown, with its padding.
    fn widths(&self, ui: &Ui, tops: &[Top]) -> Vec<f32> {
        self.texts(tops).0.iter().map(|t| self.text_width(ui, t) + 2.0 * self.style.rows.pad_x).collect()
    }

    /// The titles to draw in `room`: every menu when they fit; else the menus that fit with room
    /// for a "⋯" menu holding the rest; else one "Menu" holding them all.
    fn fit(&self, ui: &Ui, room: f32) -> Vec<Top> {
        let all = self.all();
        let widths = self.widths(ui, &all);
        if widths.iter().sum::<f32>() <= room {
            return all;
        }
        let room = room - self.widths(ui, &[Top::Rest(1)]).iter().sum::<f32>();
        let mut used = 0.0;
        let fit = widths
            .iter()
            .take_while(|w| {
                used += **w;
                used <= room
            })
            .count();
        if fit == 0 {
            return vec![Top::Rest(0)];
        }
        (0..fit).map(Top::Menu).chain([Top::Rest(fit)]).collect()
    }
}

/// Paints a title over its response: `lit` (open, or the keyboard is on it) in the menu's active
/// colours, under the pointer in the list box's active fill.
fn paint_title<T>(ui: &Ui, titles: &Titles<'_, T>, title: &Response, text: &str, mnemonic: Option<char>, lit: bool) {
    let rows = &titles.style.rows;
    let (fill, color) = if lit {
        (Some(titles.style.active_fill), titles.style.active_text)
    } else if title.hovered() {
        (Some(rows.active_fill), rows.active_text)
    } else {
        (None, rows.text)
    };
    let ppp = ui.pixels_per_point();
    if let Some(fill) = fill {
        ui.painter().rect_filled(title.rect.round_to_pixels(ppp), rows.radius, fill);
    }
    let galley = titles.galley(ui, text, mnemonic, color);
    let at = title.rect.center() - galley.size() / 2.0;
    ui.painter().galley(at.round_to_pixels(ppp), galley, color);
}

/// A title's open or close command this pass: the press opens a closed menu (and starts a
/// press-drag gesture: releasing on a row runs it) or closes an open one; the release never
/// toggles, so the menu doesn't blink. An AccessKit click toggles it.
fn title_press(ctx: &egui::Context, title: &Response, pressing: &mut bool) -> Option<SetOpenCommand> {
    // A press this pass on the title (still down, or a whole click within one pass).
    let pressed = ctx.input(|i| i.pointer.primary_pressed()) && (title.is_pointer_button_down_on() || title.clicked());
    if !pressed {
        return (title.clicked() && !title.clicked_by(PointerButton::Primary)).then_some(SetOpenCommand::Toggle);
    }
    let open = Popup::is_id_open(ctx, Popup::default_response_id(title));
    if !open {
        *pressing = true;
    }
    Some(SetOpenCommand::Bool(!open))
}

/// Whether the pointer at `p` can reach a title: a tall submenu over the bar is on top there, and
/// hovering it must not switch menus. The title's column reaches up to the top of the window, so
/// a pointer pushed against the screen edge above a title in a maximized window reaches it too.
fn pointer_reaches_title(ctx: &egui::Context, title: &Response, p: Pos2) -> bool {
    let r = title.interact_rect;
    let column = egui::Rect::from_min_max(egui::pos2(r.left(), f32::NEG_INFINITY), r.max);
    column.contains(p) && ctx.layer_id_at(p) == Some(title.layer_id)
}

/// As in a native menu bar: while one menu is open, the pointer moving onto another title opens
/// that menu instead.
fn switch_on_hover(ctx: &egui::Context, titles: &[Response], popups: &[Id]) {
    let Some(open) = popups.iter().position(|id| Popup::is_id_open(ctx, *id)) else { return };
    // `Response::hovered` is false while the menu's popup layer is open: hit-test the title, but
    // only where its own layer is on top.
    let Some(p) = ctx.pointer_hover_pos() else { return };
    // Only a moving pointer switches: a pointer resting on a title must not undo ← and →.
    if ctx.input(|i| i.pointer.delta() == egui::Vec2::ZERO) {
        return;
    }
    if let Some(i) = titles.iter().position(|t| pointer_reaches_title(ctx, t, p))
        && i != open
        && let Some(&id) = popups.get(i)
    {
        Popup::open_id(ctx, id);
        ctx.request_repaint();
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    size: SizeArg,
    search: bool,
    #[arg(range = 60.0..=1200.0)]
    width: f64,
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/menu/menu-bar",
            Level::Composites,
            "MenuBar",
            "Menu bar",
            BarArgs { size: SizeArg::Md, search: true, width: 640.0 },
            show_bar
        )
        .on(Surface::Chrome),
    ]
}

/// The story's bar: File, Edit (the menus story's), View, Image, Window and Help.
pub(crate) fn story_bar() -> Vec<MenuItem<super::Command>> {
    use super::Command;
    let key = |modifiers: Modifiers, key: Key| Kbd::new(&KeyboardShortcut::new(modifiers, key));
    let row = |label: &str, n: usize| -> MenuEntry<Command> { MenuItem::new(label, Command::Row(n)).into() };
    vec![
        MenuItem::submenu(
            "File",
            vec![
                MenuItem::new("New…", Command::Row(1)).shortcut(key(Modifiers::COMMAND, Key::N)).into(),
                MenuItem::new("Open…", Command::Row(2)).shortcut(key(Modifiers::COMMAND, Key::O)).into(),
                MenuItem::submenu("Open Recent", vec![row("poster.png", 3), row("logo.svg", 4)]).into(),
                MenuEntry::Separator,
                MenuItem::new("Save", Command::Row(5)).shortcut(key(Modifiers::COMMAND, Key::S)).into(),
                MenuItem::new("Close", Command::Row(6)).shortcut(key(Modifiers::COMMAND, Key::W)).into(),
            ],
        ),
        MenuItem::submenu("Edit", super::edit_menu(true)),
        MenuItem::submenu(
            "View",
            vec![
                MenuItem::new("Rulers", Command::Rulers).check(true).into(),
                MenuItem::new("Grid", Command::Grid).check(false).into(),
                MenuEntry::Separator,
                MenuItem::new("Fit on Screen", Command::Fit).shortcut(key(Modifiers::COMMAND, Key::Num0)).into(),
            ],
        ),
        MenuItem::submenu(
            "Image",
            vec![
                MenuItem::submenu("Adjustments", vec![row("Levels…", 7), row("Curves…", 8), row("Hue/Saturation…", 9)]).into(),
                row("Image Size…", 10),
                row("Canvas Size…", 11),
            ],
        ),
        MenuItem::submenu("Window", vec![row("Workspace", 12), row("Layers", 13)]).mnemonic('W'),
        MenuItem::submenu("Help", vec![row("Keyboard Shortcuts", 14), row("About", 15)]),
    ]
}

/// The bar in a row as wide as `width`, past which titles overflow; the command chosen last shows
/// below it.
fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let size = args.size.control();
    let menus = story_bar();
    let last = Id::unique("craft-ui-menu-bar-story-last");
    let mut bar = MenuBar::new(&menus).id(last.with("bar")).size(size).shrink_to_fit(args.width as f32);
    if args.search {
        bar = bar.help_search(menus.len() - 1);
    }
    let shown = bar.show(ui);
    if let Some(command) = shown.chosen {
        ui.data_mut(|d| d.insert_temp(last, format!("{command:?}")));
    }
    let chosen = ui.data(|d| d.get_temp::<String>(last)).unwrap_or_else(|| "Nothing chosen".to_owned());
    ui.add(Text::new(chosen).content(Content::Muted));
}
