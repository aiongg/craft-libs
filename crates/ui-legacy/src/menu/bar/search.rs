//! The Help menu's search field, as PhotoCraft's: it finds any command of the bar by its label or
//! its menu path, and lists up to [`MAX_RESULTS`] matches above the Help menu's own rows, each
//! named by its path ("Image › Adjustments › Levels…").

use craft_theme::ControlSize;
use std::sync::Arc;

use egui::{Align, EventFilter, Id, Layout, Margin, Ui};

use super::MenuBarLabels;
use crate::behavior::{CommandSearch, PaletteEntry};
use crate::controls::SearchField;
use crate::menu::item::{MenuEntry, MenuItem};
use crate::menu::row::{MenuStyle, RowPainter};
use crate::primitives::{Content, Interactive, Text};
use crate::theme::UiTokens;

/// Most matches the field lists.
pub(super) const MAX_RESULTS: usize = 15;

/// Between menu names in a match's path.
const PATH_SEPARATOR: &str = " › ";

/// The field's state between passes.
#[derive(Clone, Debug, Default)]
struct State {
    query: String,
    /// Give the field the focus once it can take it (the menu just opened).
    focus: bool,
}

/// The search field of one pass.
pub(super) struct Search<T> {
    id: Id,
    bar: Id,
    same: Same<T>,
    entries: Entries<T>,
    state: State,
    found: Vec<PaletteEntry<T>>,
    submitted: bool,
}

/// Whether two commands are the same command.
pub(super) type Same<T> = fn(&T, &T) -> bool;

/// The entries of the menus of the bar `bar`, as of the last pass: see [`cached_entries`].
pub(super) type Entries<T> = fn(&egui::Context, Id, &[MenuItem<T>]) -> Arc<Vec<PaletteEntry<T>>>;

/// The menu tree and the entries built from it.
struct Cached<T> {
    menus: Vec<MenuItem<T>>,
    entries: Arc<Vec<PaletteEntry<T>>>,
}

/// The palette entries of `menus`, built again only when the tree differs from the one they were
/// built from, so a pass with a query does not clone every label of the bar.
pub(super) fn cached_entries<T: Clone + PartialEq + Send + Sync + 'static>(
    ctx: &egui::Context,
    bar: Id,
    menus: &[MenuItem<T>],
) -> Arc<Vec<PaletteEntry<T>>> {
    let id = bar.with("search-entries");
    if let Some(cached) = ctx.data(|d| d.get_temp::<Arc<Cached<T>>>(id)).filter(|c| c.menus == menus) {
        return Arc::clone(&cached.entries);
    }
    let entries = Arc::new(PaletteEntry::from_menu(menus));
    ctx.data_mut(|d| d.insert_temp(id, Arc::new(Cached { menus: menus.to_vec(), entries: Arc::clone(&entries) })));
    entries
}

/// The commands among `entries` matching `query`, best first and in menu order within a rank, at
/// most [`MAX_RESULTS`]. A command in several menus is listed once, at its best match.
fn matches<T: Clone>(entries: &[PaletteEntry<T>], query: &str, same: Same<T>) -> Vec<PaletteEntry<T>> {
    let found = CommandSearch::new(entries).query(query).fuzzy(false).same(same).results(MAX_RESULTS);
    found.into_iter().map(|m| m.entry.clone()).collect()
}

/// The keys the field leaves to the menu: ↑ / ↓ move its highlight, Esc closes it.
fn keys() -> EventFilter {
    EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false }
}

impl<T: Clone> Search<T> {
    /// The field of the bar `bar`, as the last pass left it; `same` tells commands apart.
    pub fn load(ctx: &egui::Context, bar: Id, same: Same<T>, entries: Entries<T>) -> Self {
        let id = bar.with("search");
        Self { id, bar, same, entries, state: ctx.data(|d| d.get_temp(id)).unwrap_or_default(), found: Vec::new(), submitted: false }
    }

    /// The Help menu `popup` shows this pass: a fresh opening starts empty, with the field
    /// focused; the matches are those of the query so far.
    pub fn begin(&mut self, ctx: &egui::Context, popup: Id, menus: &[MenuItem<T>]) {
        if ctx.read_response(popup).is_none() {
            self.state = State { query: String::new(), focus: true };
        }
        if !self.state.query.trim().is_empty() {
            self.found = matches(&(self.entries)(ctx, self.bar, menus), &self.state.query, self.same);
        } else {
            self.found.clear();
        }
    }

    /// The Help menu's entries: the matches, a separator, then `help`. `None` without matches.
    pub fn entries(&self, help: &[MenuEntry<T>]) -> Option<Vec<MenuEntry<T>>> {
        if self.found.is_empty() {
            return None;
        }
        let rows = self.found.iter().map(|f| {
            let trail = f.path_of().iter().map(String::as_str).chain([f.label()]).collect::<Vec<_>>();
            let mut row = MenuItem::new(trail.join(PATH_SEPARATOR), f.command().clone()).enabled(f.is_enabled()).without_mnemonic();
            if let Some(kbd) = f.shortcut_of() {
                row = row.shortcut(kbd.clone());
            }
            MenuEntry::Item(row)
        });
        Some(rows.chain([MenuEntry::Separator]).chain(help.iter().cloned()).collect())
    }

    /// Draws the field at the top of the menu (with "No matching commands" when nothing
    /// matches, and the separator before the menu's rows when no match is listed). Returns whether
    /// the field has the keyboard focus.
    pub fn field(&mut self, ui: &mut Ui, style: &MenuStyle, tokens: &UiTokens, size: ControlSize, labels: &MenuBarLabels) -> bool {
        let pad = style.rows.pad_x;
        let salt = "craft-ui-menu-bar-search";
        let before = self.state.query.clone();
        let mut typing = false;
        egui::Frame::NONE.inner_margin(Margin::symmetric(pad.round() as i8, style.rows.gap.round() as i8)).show(ui, |ui| {
            ui.set_width((style.min_width - 2.0 * pad).max(0.0));
            ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                let field = Interactive::id_in(ui, salt);
                // egui drops a focus given in a pass that sizes the popup, or with a click outside
                // the field (the click on the title that opened the menu).
                if self.state.focus && !ui.is_sizing_pass() && !ui.input(|i| i.pointer.any_click()) {
                    ui.memory_mut(|m| m.request_focus_with_filter(field, keys()));
                    self.state.focus = false;
                }
                let edit = SearchField::new(salt, labels.search.as_str(), &self.state.query, labels.clear.as_str())
                    .placeholder(labels.search.as_str())
                    .size(size)
                    .keep_focus(true)
                    .edit_menu(false)
                    .keys(keys())
                    .show(ui);
                if let Some(text) = edit.event.as_ref().and_then(|e| e.value()) {
                    self.state.query.clone_from(text);
                }
                self.submitted = edit.submitted;
                typing = edit.response.has_focus();
                if self.found.is_empty() && !self.state.query.trim().is_empty() {
                    ui.add(Text::new(labels.no_matches.as_str()).content(Content::Muted));
                }
            });
        });
        if self.found.is_empty() {
            RowPainter { style, tokens, size }.separator(ui, style.min_width);
        }
        if self.state.query != before {
            ui.ctx().request_repaint();
        }
        typing
    }

    /// Enter was pressed in the field this pass.
    pub fn submitted(&self) -> bool {
        self.submitted
    }

    /// The command of the first enabled match.
    pub fn first_enabled(&self) -> Option<T> {
        self.found.iter().find(|f| f.is_enabled()).map(|f| f.command().clone())
    }

    pub fn store(self, ctx: &egui::Context) {
        ctx.data_mut(|d| d.insert_temp(self.id, self.state));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_list_menu_commands_by_path_and_stop_at_the_limit() {
        let menus: Vec<MenuItem<u8>> = vec![
            MenuItem::submenu(
                "Image",
                vec![MenuItem::submenu("Adjustments", vec![MenuItem::new("Curves…", 1).into(), MenuItem::new("Levels…", 2).into()]).into()],
            ),
            MenuItem::submenu("Select", vec![MenuItem::new("All", 3).into(), MenuItem::new("Level Horizon", 4).into()]),
        ];
        let found: Vec<(String, u8)> =
            matches(&PaletteEntry::from_menu(&menus), " LEV ", u8::eq).iter().map(|f| (f.path_of().join(PATH_SEPARATOR), *f.command())).collect();
        assert_eq!(found, [("Image › Adjustments".to_owned(), 2), ("Select".to_owned(), 4)]);
        let many: Vec<MenuItem<usize>> = vec![MenuItem::submenu("Many", (0..40).map(|i| MenuItem::new(format!("Row {i}"), i).into()).collect())];
        assert_eq!(matches(&PaletteEntry::from_menu(&many), "row", usize::eq).len(), MAX_RESULTS);
    }

    #[test]
    fn entries_are_reused_until_the_menus_change() {
        let ctx = egui::Context::default();
        let bar = Id::unique("bar");
        let menus: Vec<MenuItem<u8>> = vec![MenuItem::submenu("Select", vec![MenuItem::new("All", 1).into()])];
        let first = cached_entries(&ctx, bar, &menus);
        assert!(Arc::ptr_eq(&first, &cached_entries(&ctx, bar, &menus)));
        let edited: Vec<MenuItem<u8>> = vec![MenuItem::submenu("Select", vec![MenuItem::new("All", 1).enabled(false).into()])];
        let second = cached_entries(&ctx, bar, &edited);
        assert!(!Arc::ptr_eq(&first, &second));
        assert!(!second[0].is_enabled());
        assert!(Arc::ptr_eq(&second, &cached_entries(&ctx, bar, &edited)));
    }
}
