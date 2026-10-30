//! [`CommandSearch`]: the ranking behind a command palette and the menu bar's Help search, over
//! [`PaletteEntry`] values.

use crate::controls::Kbd;
use crate::menu::{MenuEntry, MenuItem};

/// One command a palette or a menu search can list: what runs, how it is named and where it lives.
///
/// `PaletteEntry::new(command, label).path(["Image", "Adjustments"]).shortcut(kbd)`; entries for
/// the commands of a menu tree come from [`from_menu`](PaletteEntry::from_menu), and the app adds
/// its tools, panels and styles with a [`kind`](PaletteEntry::kind).
#[derive(Clone, Debug)]
pub struct PaletteEntry<T> {
    command: T,
    label: String,
    english: Option<String>,
    path: Vec<String>,
    path_english: Vec<String>,
    kind: Option<String>,
    shortcut: Option<Kbd>,
    enabled: bool,
}

impl<T> PaletteEntry<T> {
    /// An entry that returns `command` when it runs, named `label` (as shown, translated).
    pub fn new(command: T, label: impl Into<String>) -> Self {
        Self { command, label: label.into(), english: None, path: Vec::new(), path_english: Vec::new(), kind: None, shortcut: None, enabled: true }
    }

    /// The English name, which the search also tries: commands are documented in English.
    pub fn english(mut self, english: impl Into<String>) -> Self {
        self.english = Some(english.into());
        self
    }

    /// The titles of the menus above the command, outermost first.
    pub fn path<S: Into<String>>(mut self, path: impl IntoIterator<Item = S>) -> Self {
        self.path = path.into_iter().map(Into::into).collect();
        self
    }

    /// The English titles of the menus above the command, as [`path`](PaletteEntry::path) gives
    /// them; the search tries them too.
    pub fn path_english<S: Into<String>>(mut self, path: impl IntoIterator<Item = S>) -> Self {
        self.path_english = path.into_iter().map(Into::into).collect();
        self
    }

    /// What the entry is when it is not a menu command: "Tool", "Paragraph Style". Shown where the
    /// path of a menu command would be.
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// The command's shortcut.
    pub fn shortcut(mut self, shortcut: Kbd) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Whether the command can run; a disabled entry is listed but does not run.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn command(&self) -> &T {
        &self.command
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn path_of(&self) -> &[String] {
        &self.path
    }

    pub fn kind_of(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    pub fn shortcut_of(&self) -> Option<&Kbd> {
        self.shortcut.as_ref()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl<T: Clone> PaletteEntry<T> {
    /// The commands under the menus `menus`, depth first, in menu order: the path is the titles of
    /// the menus above a command, and a separator or a submenu's own row is not a command. A
    /// top-level item without a submenu is not listed (a menu bar's titles are menus).
    pub fn from_menu(menus: &[MenuItem<T>]) -> Vec<PaletteEntry<T>> {
        let mut out = Vec::new();
        for menu in menus {
            let mut path = vec![menu];
            Self::collect(menu.children().unwrap_or(&[]), &mut path, &mut out);
        }
        out
    }

    fn collect<'m>(entries: &'m [MenuEntry<T>], path: &mut Vec<&'m MenuItem<T>>, out: &mut Vec<PaletteEntry<T>>) {
        for item in entries.iter().filter_map(MenuEntry::item) {
            if let Some(children) = item.children() {
                path.push(item);
                Self::collect(children, path, out);
                path.pop();
            } else if let Some(command) = item.command() {
                let mut entry = PaletteEntry::new(command.clone(), item.label()).path(path.iter().map(|m| m.label())).enabled(item.is_enabled());
                if let Some(english) = item.english_of() {
                    entry = entry.english(english);
                }
                if path.iter().any(|m| m.english_of().is_some()) {
                    entry = entry.path_english(path.iter().map(|m| m.english_of().unwrap_or(m.label())));
                }
                if let Some(kbd) = item.shortcut_of() {
                    entry = entry.shortcut(kbd.clone());
                }
                out.push(entry);
            }
        }
    }
}

/// A command that matched, with how well.
#[derive(Debug)]
pub struct Match<'a, T> {
    /// The matched entry.
    pub entry: &'a PaletteEntry<T>,
    /// Its place in the entries given.
    pub index: usize,
    /// 0 when the label starts with the query, 1 when a word of it does, 2 when the label contains
    /// it, 3 when a menu title above the command does, 4 for a subsequence match.
    pub rank: u8,
    /// The label (or English label) is the query.
    pub exact: bool,
    /// How well a rank 4 match fits (higher is better); 0 for the other ranks.
    pub score: i32,
}

/// The ranking of commands against a query, as PhotoCraft's palette and the menu bar's Help
/// search do it.
///
/// Rank 0 to 3 look at the label and the English label, then at the menu titles above the
/// command; rank 4 is a subsequence match over the label, the path and the English names. A
/// command whose name is exactly the query comes before every other. Within a rank 0 to 3
/// the entries keep their order; rank 4 is best score first. A command listed twice is listed
/// once, at its best match, where [`same`](CommandSearch::same) or [`dedup`](CommandSearch::dedup)
/// says two entries are the same command.
///
/// ```
/// use craft_ui::behavior::{CommandSearch, PaletteEntry};
/// let entries = [PaletteEntry::new(1, "Levels…"), PaletteEntry::new(2, "Hue/Saturation…")];
/// let found = CommandSearch::new(&entries).query("sat").results(5);
/// assert_eq!(found.iter().map(|m| *m.entry.command()).collect::<Vec<_>>(), [2]);
/// ```
#[derive(Clone)]
#[must_use = "read the matches with `results`"]
pub struct CommandSearch<'a, T> {
    entries: &'a [PaletteEntry<T>],
    query: String,
    same: Option<fn(&T, &T) -> bool>,
    fuzzy: bool,
}

impl<'a, T> CommandSearch<'a, T> {
    pub fn new(entries: &'a [PaletteEntry<T>]) -> Self {
        Self { entries, query: String::new(), same: None, fuzzy: true }
    }

    /// The text typed. Blank matches nothing.
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    /// Whether the subsequence match (rank 4) is tried; on by default. The menu bar's search
    /// stops at rank 3.
    pub fn fuzzy(mut self, fuzzy: bool) -> Self {
        self.fuzzy = fuzzy;
        self
    }

    /// Lists a command that appears in several entries once, where `same` says two commands are
    /// the same.
    pub fn same(mut self, same: fn(&T, &T) -> bool) -> Self {
        self.same = Some(same);
        self
    }

    /// The best matches, at most `max`, best first.
    pub fn results(&self, max: usize) -> Vec<Match<'a, T>> {
        let q = self.query.trim().to_lowercase();
        if q.is_empty() || max == 0 {
            return Vec::new();
        }
        let mut hits: Vec<Match<'a, T>> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| rank(&q, entry, self.fuzzy).map(|(rank, exact, score)| Match { entry, index, rank, exact, score }))
            .collect();
        hits.sort_by_key(|m| (!m.exact, m.rank, -m.score, m.index));
        let mut found: Vec<Match<'a, T>> = Vec::with_capacity(max.min(hits.len()));
        for hit in hits {
            let listed = |g: &Match<'a, T>| self.same.is_some_and(|same| same(hit.entry.command(), g.entry.command()));
            if !found.iter().any(listed) {
                found.push(hit);
            }
            if found.len() == max {
                break;
            }
        }
        found
    }
}

impl<T: PartialEq> CommandSearch<'_, T> {
    /// Lists a command that appears in several entries once.
    pub fn dedup(self) -> Self {
        self.same(T::eq)
    }
}

/// How well `query` (lower case) matches a label: the label starting with it (0), then a word of
/// the label starting with it (1), then anywhere in the label (2).
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

/// Whether `name` is the query, ignoring case, an ellipsis and the space round it.
fn is_exact(query: &str, name: &str) -> bool {
    name.trim_end_matches('…').trim().to_lowercase() == query.trim_end_matches('…').trim()
}

/// Case-insensitive subsequence match score (higher is better); `None` when `query` is not a
/// subsequence of `text`. Runs of consecutive characters and characters at the start of a word score
/// more; a long text loses a little.
fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let mut score = 0;
    let mut at = 0;
    let mut last = None;
    for qc in query.chars() {
        let found = (at..text.len()).find(|&i| text[i] == qc)?;
        score += if last == Some(found.wrapping_sub(1)) { 8 } else { 1 };
        if found == 0 || text[found - 1] == ' ' {
            score += 5;
        }
        last = Some(found);
        at = found + 1;
    }
    Some(score - text.len() as i32 / 8)
}

/// `(rank, exact, score)` of `entry` for `query` (lower case, trimmed).
fn rank<T>(query: &str, entry: &PaletteEntry<T>, fuzzy: bool) -> Option<(u8, bool, i32)> {
    let names = [Some(entry.label.as_str()), entry.english.as_deref()];
    let exact = names.iter().flatten().any(|n| is_exact(query, n));
    let by_label = names.iter().flatten().filter_map(|n| label_rank(query, n)).min();
    let by_path = || entry.path.iter().chain(&entry.path_english).any(|t| t.to_lowercase().contains(query));
    if let Some(r) = by_label.or_else(|| by_path().then_some(3)) {
        return Some((r, exact, 0));
    }
    if !fuzzy {
        return None;
    }
    let text = [entry.label.as_str(), &entry.path.join(" "), entry.english.as_deref().unwrap_or(""), &entry.path_english.join(" ")].join(" ");
    fuzzy_score(query, &text).map(|score| (4, exact, score))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands(found: &[Match<'_, u8>]) -> Vec<u8> {
        found.iter().map(|m| *m.entry.command()).collect()
    }

    fn entries() -> Vec<PaletteEntry<u8>> {
        vec![
            PaletteEntry::new(1, "Select All").path(["Select"]),
            PaletteEntry::new(2, "Hue/Saturation…").path(["Image", "Adjustments"]),
            PaletteEntry::new(3, "Curves…").path(["Image", "Adjustments"]),
            PaletteEntry::new(4, "Levels…").path(["Image", "Adjustments"]),
            PaletteEntry::new(5, "Level Horizon").path(["Select"]),
            PaletteEntry::new(6, "Gaussian Blur").path(["Filter", "Blur"]).english("Gaussian Blur"),
        ]
    }

    #[test]
    fn ranks_run_from_label_start_to_subsequence() {
        let e = entries();
        let rank_of = |q: &str| CommandSearch::new(&e).query(q).results(10).first().map(|m| m.rank);
        assert_eq!(rank_of("lev"), Some(0));
        assert_eq!(rank_of("sat"), Some(1));
        assert_eq!(rank_of("vel"), Some(2));
        assert_eq!(rank_of("adjust"), Some(3));
        assert_eq!(rank_of("gblur"), Some(4));
        assert_eq!(rank_of("zzz"), None);
        assert_eq!(commands(&CommandSearch::new(&e).query(" LEV ").results(10)), [4, 5], "entry order within a rank, case and edge space ignored");
        assert_eq!(commands(&CommandSearch::new(&e).query("adjust").results(10)), [2, 3, 4], "a menu title matches every command under it");
        assert!(CommandSearch::new(&e).query("gblur").fuzzy(false).results(10).is_empty());
    }

    #[test]
    fn an_exact_name_comes_before_everything() {
        let e =
            vec![PaletteEntry::new(1, "Color Settings…"), PaletteEntry::new(2, "Settings Edit").path(["Edit"]), PaletteEntry::new(3, "Settings…")];
        let found = CommandSearch::new(&e).query("settings").results(10);
        assert_eq!(commands(&found), [3, 2, 1], "the exact name, then the label start, then the word start");
        assert!(found[0].exact && !found[1].exact);
    }

    #[test]
    fn the_english_label_matches_and_a_duplicate_is_listed_once_at_its_best() {
        let e = vec![
            PaletteEntry::new(7, "Ebenen").english("Layers").path(["Fenster"]),
            PaletteEntry::new(8, "Rückgängig").path(["Bearbeiten"]).english("Undo"),
            PaletteEntry::new(8, "Undo history").path(["Window"]),
        ];
        assert_eq!(commands(&CommandSearch::new(&e).query("lay").results(10)), [7]);
        let found = CommandSearch::new(&e).query("un").dedup().results(10);
        assert_eq!((commands(&found), found[0].rank, found[0].index), (vec![8], 0, 1), "the first of equal ranks, once");
        assert_eq!(CommandSearch::new(&e).query("un").results(10).len(), 2, "without a same rule both are listed");
        assert_eq!(CommandSearch::new(&e).query("e").results(1).len(), 1, "cut to the limit");
    }

    #[test]
    fn from_menu_keeps_the_path_and_skips_separators_and_submenu_rows() {
        let menus: Vec<MenuItem<u8>> = vec![
            MenuItem::submenu(
                "Image",
                vec![
                    MenuItem::new("Size…", 1).shortcut(Kbd::keys(&["Alt", "I"])).into(),
                    MenuEntry::Separator,
                    MenuItem::submenu("Adjustments", vec![MenuItem::new("Levels…", 2).enabled(false).english("Levels").into()]).into(),
                ],
            ),
            MenuItem::new("Top level", 3),
        ];
        let entries = PaletteEntry::from_menu(&menus);
        assert_eq!(entries.iter().map(|e| *e.command()).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(entries[1].path_of(), ["Image", "Adjustments"]);
        assert!(entries[0].shortcut_of().is_some() && !entries[1].is_enabled());
    }
}
