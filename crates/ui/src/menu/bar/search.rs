//! The Help menu's search field, as PhotoCraft's: it finds any command of the bar by its label or
//! its menu path, and lists up to [`MAX_RESULTS`] matches above the Help menu's own rows, each
//! named by its path ("Image › Adjustments › Levels…").

use craft_theme::ControlSize;
use egui::{Align, EventFilter, Id, Layout, Margin, Ui};

use super::MenuBarLabels;
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

/// A match: the command's row and its path of menu titles.
struct Found<'m, T> {
    path: Vec<&'m str>,
    item: &'m MenuItem<T>,
}

/// The search field of one pass.
pub(super) struct Search<'m, T> {
    id: Id,
    state: State,
    found: Vec<Found<'m, T>>,
    submitted: bool,
}

/// How well `query` (lower case) matches a label: the label starting with it, then a word of the
/// label starting with it, then anywhere in the label. `None` when it doesn't match.
fn label_rank(query: &str, label: &str) -> Option<u8> {
    let l = label.to_lowercase();
    if l.starts_with(query) {
        Some(0)
    } else if l.split(|c: char| !c.is_alphanumeric()).any(|w| w.starts_with(query)) {
        Some(1)
    } else if l.contains(query) {
        Some(2)
    } else {
        None
    }
}

/// How well `query` (lower case) matches a command: by its label or English label
/// ([`label_rank`]), else anywhere in one of the menu titles above it (3).
fn rank<T>(query: &str, item: &MenuItem<T>, path: &[&MenuItem<T>]) -> Option<u8> {
    let names = |it: &MenuItem<T>| [Some(it.label()), it.english_of()].into_iter().flatten().map(str::to_owned).collect::<Vec<_>>();
    let by_label = names(item).iter().filter_map(|l| label_rank(query, l)).min();
    by_label.or_else(|| path.iter().flat_map(|m| names(m)).any(|t| t.to_lowercase().contains(query)).then_some(3))
}

/// Every command under `entries`, depth first, with the menus above it.
fn commands<'m, T>(entries: &'m [MenuEntry<T>], path: &mut Vec<&'m MenuItem<T>>, out: &mut Vec<(Vec<&'m MenuItem<T>>, &'m MenuItem<T>)>) {
    for item in entries.iter().filter_map(MenuEntry::item) {
        match item.children() {
            Some(children) => {
                path.push(item);
                commands(children, path, out);
                path.pop();
            }
            None => out.push((path.clone(), item)),
        }
    }
}

/// The commands under `menus` matching `query`, best first and in menu order within a rank, at
/// most [`MAX_RESULTS`].
fn matches<'m, T>(menus: &'m [MenuItem<T>], query: &str) -> Vec<Found<'m, T>> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut all = Vec::new();
    for menu in menus {
        let mut path = vec![menu];
        commands(menu.children().unwrap_or(&[]), &mut path, &mut all);
    }
    let mut hits: Vec<(u8, usize, Found<'m, T>)> = all
        .into_iter()
        .enumerate()
        .filter_map(|(i, (path, item))| {
            let r = rank(&q, item, &path)?;
            Some((r, i, Found { path: path.iter().map(|m| m.label()).collect(), item }))
        })
        .collect();
    hits.sort_by_key(|(r, i, _)| (*r, *i));
    hits.into_iter().take(MAX_RESULTS).map(|(_, _, f)| f).collect()
}

/// The keys the field leaves to the menu: ↑ / ↓ move its highlight, Esc closes it.
fn keys() -> EventFilter {
    EventFilter { horizontal_arrows: true, vertical_arrows: true, tab: false, escape: false }
}

impl<'m, T: Clone> Search<'m, T> {
    /// The field of the bar `bar`, as the last pass left it.
    pub fn load(ctx: &egui::Context, bar: Id) -> Self {
        let id = bar.with("search");
        Self { id, state: ctx.data(|d| d.get_temp(id)).unwrap_or_default(), found: Vec::new(), submitted: false }
    }

    /// The Help menu `popup` shows this pass: a fresh opening starts empty, with the field
    /// focused; the matches are those of the query so far.
    pub fn begin(&mut self, ctx: &egui::Context, popup: Id, menus: &'m [MenuItem<T>]) {
        if ctx.read_response(popup).is_none() {
            self.state = State { query: String::new(), focus: true };
        }
        self.found = matches(menus, &self.state.query);
    }

    /// The Help menu's entries: the matches, a separator, then `help`. `None` without matches.
    pub fn entries(&self, help: &[MenuEntry<T>]) -> Option<Vec<MenuEntry<T>>> {
        if self.found.is_empty() {
            return None;
        }
        let rows = self.found.iter().filter_map(|f| {
            let mut trail = f.path.clone();
            trail.push(f.item.label());
            let mut row = MenuItem::new(trail.join(PATH_SEPARATOR), f.item.command()?.clone()).enabled(f.item.is_enabled());
            if let Some(kbd) = f.item.shortcut_of() {
                row = row.shortcut(kbd.clone());
            }
            Some(MenuEntry::Item(row))
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
                let field = Interactive::id_in(ui, egui::IdSalt::new(salt));
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
        self.found.iter().find(|f| f.item.is_enabled()).and_then(|f| f.item.command().cloned())
    }

    pub fn store(self, ctx: &egui::Context) {
        ctx.data_mut(|d| d.insert_temp(self.id, self.state));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_rank_label_starts_then_word_starts_then_anywhere_then_the_path() {
        assert_eq!(label_rank("lev", "Levels…"), Some(0));
        assert_eq!(label_rank("sat", "Hue/Saturation…"), Some(1));
        assert_eq!(label_rank("vel", "Levels…"), Some(2));
        assert_eq!(label_rank("zz", "Levels…"), None);
        let menus: Vec<MenuItem<u8>> = vec![
            MenuItem::submenu(
                "Image",
                vec![MenuItem::submenu("Adjustments", vec![MenuItem::new("Curves…", 1).into(), MenuItem::new("Levels…", 2).into()]).into()],
            ),
            MenuItem::submenu("Select", vec![MenuItem::new("All", 3).into(), MenuItem::new("Level Horizon", 4).into()]),
        ];
        let found: Vec<(String, u8)> = matches(&menus, " LEV ").iter().map(|f| (f.path.join(PATH_SEPARATOR), *f.item.command().unwrap())).collect();
        assert_eq!(found, [("Image › Adjustments".to_owned(), 2), ("Select".to_owned(), 4)]);
        let by_path: Vec<u8> = matches(&menus, "adjust").iter().map(|f| *f.item.command().unwrap()).collect();
        assert_eq!(by_path, [1, 2], "a menu title matches every command under it");
        let many: Vec<MenuItem<usize>> = vec![MenuItem::submenu("Many", (0..40).map(|i| MenuItem::new(format!("Row {i}"), i).into()).collect())];
        assert_eq!(matches(&many, "row").len(), MAX_RESULTS);
    }
}
