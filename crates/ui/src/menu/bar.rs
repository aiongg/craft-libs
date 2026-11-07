//! `MenuBar`: a row of menu titles, each opening a menu the caller writes.
//!
//! The titles are parts of one component in one roving group, so the bar is one Tab stop. A press on a title opens
//! its menu and a press on the open title closes it; while a menu is open the pointer moving onto another title
//! switches to that menu. Titles that do not fit go into a trailing "…" menu that holds them as submenus, or all of
//! them into one "Menu". Under `MenuPlacement::ScreenTop` the bar shows nothing: the OS menu bar takes the same
//! items.
//!
//! Keys follow spec 9.7: Alt and a letter open the menu with that mnemonic (also while another is open), F10 or a
//! second lone Alt closes an open menu. A `&` in a title's words row marks its mnemonic; without one the title takes
//! the first letter no earlier title holds.
//!
//! The Help menu can start with a search field over a list of `HelpEntry` commands: typing lists up to `MAX_RESULTS`
//! matches by label, English label or menu path, each as a command row with its path as detail.

#[cfg(feature = "testing")]
mod stories;

use super::words::MenuBarWords;
use super::MenuItem;
use crate::buttons::ToggleButton;
use crate::contract::overlay::Anchor;
use crate::contract::{
    Access, Bind, Common, Component, Cx, Focus as _, MeasureCx, MenuKey, Name, NoValue, Outcome, Overlay, OverlayUi, Overlays as _, PaintUi, PartsCx,
    Pointer as _, Props as _, Role, SetPosition, Show as _, ShowContainer as _,
};
use crate::display::{Divider, Text};
use crate::runtime::commands::CommandRef;
use crate::runtime::env::{EnvView, Label};
use crate::runtime::memory::{Facts, Remember as _};
use crate::style::{ComponentStyle, StyleCx};
use crate::text::TextField;
use craft_behavior::{Axis, Flow, Metrics, NavMove, NavShape, PartKey, PartRef, Roving, Sense};
use craft_host::target::MenuPlacement;
use craft_keymap::mnemonic;
use egui::{Context, Pos2, Rect, Response, Ui, Vec2};

const TITLE: PartKey = PartKey::new("title");
const MORE: PartKey = PartKey::new("more");

/// The most matches the help search lists.
pub const MAX_RESULTS: usize = 15;

/// Between the menu names of a match's path.
const PATH_SEPARATOR: &str = " › ";

/// The style of a menu bar: no look of its own; the titles wear toggle-button faces.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct MenuBarStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A menu of the bar: its title and the body that fills the menu when it opens.
pub struct MenuTitle<'a> {
    label: Label,
    body: Box<dyn FnMut(&mut Ui) + 'a>,
}

impl<'a> MenuTitle<'a> {
    /// A menu titled `label` (a `&` marks the mnemonic) whose rows `body` shows.
    pub fn new(label: impl Into<Label>, body: impl FnMut(&mut Ui) + 'a) -> Self {
        MenuTitle { label: label.into(), body: Box::new(body) }
    }
}

/// A command the help search can find.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct HelpEntry {
    command: CommandRef,
    label: Label,
    english: Option<Label>,
    path: Vec<Label>,
}

impl HelpEntry {
    /// A command found by `label`, the label its menu row shows.
    pub fn new(command: impl Into<CommandRef>, label: impl Into<Label>) -> Self {
        HelpEntry { command: command.into(), label: label.into(), english: None, path: Vec::new() }
    }

    /// The English label, which matches too.
    #[must_use]
    pub fn english(mut self, english: impl Into<Label>) -> Self {
        self.english = Some(english.into());
        self
    }

    /// The menu names above the command, outermost first ("Image", "Adjustments").
    #[must_use]
    pub fn path(mut self, path: impl IntoIterator<Item = impl Into<Label>>) -> Self {
        self.path = path.into_iter().map(Into::into).collect();
        self
    }
}

/// The Help menu's search: the index of the menu that starts with the field, and the commands it searches.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct MenuHelp<'a> {
    menu: usize,
    entries: &'a [HelpEntry],
}

impl<'a> MenuHelp<'a> {
    /// A search field at the top of the menu `menu` (its position among the titles) over `entries`.
    pub fn new(menu: usize, entries: &'a [HelpEntry]) -> Self {
        MenuHelp { menu, entries }
    }
}

/// What a menu bar reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct MenuBarOutput {
    /// The bar's response.
    pub response: Response,
    /// A menu of the bar is open.
    pub open: bool,
    /// The titles alone (with "…" when it shows), from the first title's reading start to the last title's end.
    pub titles: Rect,
}

impl Outcome for MenuBarOutput {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// A title of the bar: a menu, or the overflow menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Open {
    Title(usize),
    More,
}

/// Bar state between passes.
#[derive(Clone, Debug, Default)]
struct BarState {
    /// The menu shown, or about to open.
    open: Option<Open>,
    /// Where the pointer last was over a title, so that only a moving pointer switches menus.
    hover: Option<Pos2>,
}

/// The help field's text.
#[derive(Clone, Debug, Default)]
struct Query(String);

/// What the bar published for `is_open` and `is_focused`.
#[derive(Clone, Copy, Debug)]
struct BarFact {
    open: bool,
    focused: bool,
}

/// Whether a menu of a bar is open. The app leaves the keys to it meanwhile: its shortcuts wait until the menu
/// closes.
pub fn is_open(ctx: &Context) -> bool {
    Facts::<BarFact>::of_last_pass(ctx).iter().any(|fact| fact.open)
}

/// Whether the keyboard is on a menu bar's titles.
pub fn is_focused(ctx: &Context) -> bool {
    Facts::<BarFact>::of_last_pass(ctx).iter().any(|fact| fact.focused)
}

/// The titles a bar shows in a given room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TitleFit {
    /// How many titles show, from the first.
    pub shown: usize,
    /// The rest are in an overflow menu: "…" after the shown titles, or "Menu" when none is shown.
    pub overflow: bool,
}

/// The titles to show in `room`, given each title's `widths` and the width of the "…" title: every title when they
/// fit; else the titles that fit beside "…"; else none, with one "Menu" holding them all.
pub fn fit_titles(widths: &[f32], more: f32, room: f32) -> TitleFit {
    if widths.iter().sum::<f32>() <= room {
        return TitleFit { shown: widths.len(), overflow: false };
    }
    let room = room - more;
    let mut used = 0.0;
    let shown = widths
        .iter()
        .take_while(|width| {
            used += **width;
            used <= room
        })
        .count();
    TitleFit { shown, overflow: true }
}

/// The entries matching `query`, best first and in menu order within a rank, at most `MAX_RESULTS`. Each entry is
/// its `[label, english label, menu path]` texts. Every word of the query must occur in one of them, in any case.
/// Rank 0 is a label named exactly as the query, 1 one that starts with it, 2 one that contains it, 3 a match
/// found only through the words or the path.
pub fn search_order(query: &str, texts: &[[&str; 3]]) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let words: Vec<&str> = query.split_whitespace().collect();
    let mut ranked: Vec<(u8, usize)> = texts
        .iter()
        .enumerate()
        .filter_map(|(at, [label, english, path])| {
            let (label, english, path) = (label.to_lowercase(), english.to_lowercase(), path.to_lowercase());
            let all = format!("{label} {english} {path}");
            if !words.iter().all(|word| all.contains(word)) {
                return None;
            }
            let names = [label.as_str(), english.as_str()];
            let rank = if names.contains(&query.as_str()) {
                0
            } else if names.iter().any(|name| name.starts_with(&query)) {
                1
            } else if names.iter().any(|name| name.contains(&query)) {
                2
            } else {
                3
            };
            Some((rank, at))
        })
        .collect();
    ranked.sort();
    ranked.into_iter().take(MAX_RESULTS).map(|(_, at)| at).collect()
}

/// Which titles a pass declared as parts.
#[derive(Clone, Copy, Debug)]
struct Declared {
    shown: usize,
    more: bool,
}

fn declared(cx: &impl crate::contract::Pointer) -> Declared {
    let shown = (0..).take_while(|at| cx.part_if_declared(TITLE.at(*at)).is_some()).count();
    Declared { shown, more: cx.part_if_declared(MORE).is_some() }
}

/// A menu bar. Titles are the caller's `MenuTitle`s; the bar itself shows no menu rows.
///
/// AccessKit: the bar is a `MenuBar`; titles are `MenuItem`s that report `expanded` while their menu is open.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(overlay, probe(MenuBar<'static>))]
pub struct MenuBar<'a> {
    #[common]
    common: Common,
    titles: Vec<MenuTitle<'a>>,
    /// The Help menu's search.
    #[prop]
    help: Option<MenuHelp<'a>>,
    /// Whether the bar takes F10, Alt and Alt+letter. Off while a dialog or the shortcut recorder owns the keys.
    #[prop]
    keyboard: bool,
    open: bool,
}

impl<'a> MenuBar<'a> {
    /// A bar named `label` with one title per menu, in reading order.
    pub fn new(label: impl Into<Label>, titles: Vec<MenuTitle<'a>>) -> Self {
        MenuBar { common: Common::new(), titles, help: None, keyboard: true, open: false }.label(label)
    }

    fn in_window(env: &EnvView<'_>) -> bool {
        env.shell().menu == MenuPlacement::InWindow
    }

    /// Each title's text without its marker, and its mnemonic.
    fn texts(&self, env: &EnvView<'_>) -> Vec<(String, Option<char>)> {
        let labels: Vec<Label> = self.titles.iter().map(|title| env.resolve(&title.label)).collect();
        let refs: Vec<&str> = labels.iter().map(Label::as_str).collect();
        let letters = mnemonic::assign(&refs);
        refs.iter().zip(letters).map(|(label, letter)| (mnemonic::marked(label).0, letter)).collect()
    }

    fn face(text: &str, lit: bool) -> ToggleButton<'static> {
        ToggleButton::new(Label::from(text.to_owned()), Bind::given(lit))
    }

    /// The text of the overflow title: "…" beside shown titles, "Menu" when it holds them all.
    fn more_text(words: &MenuBarWords, shown: usize) -> String {
        if shown == 0 { words.all.as_str().to_owned() } else { words.more.as_str().to_owned() }
    }

    /// Each title's width as its face measures, the face height, and the width of the "…" title.
    fn widths(&self, cx: &MeasureCx<'_, Self>) -> (Vec<f32>, f32, f32) {
        let texts = self.texts(&cx.env);
        let sizes: Vec<Vec2> = texts.iter().map(|(text, _)| cx.face(Self::face(text, false))).collect();
        let more = cx.face(Self::face(&Self::more_text(&cx.env.words::<MenuBarWords>(), 1), false));
        let height = sizes.iter().map(|size| size.y).fold(more.y, f32::max);
        (sizes.iter().map(|size| size.x).collect(), height, more.x)
    }

    fn slot_part(open: Open) -> PartRef {
        match open {
            Open::Title(at) => TITLE.at(at as u64),
            Open::More => MORE.into(),
        }
    }

    /// The parts a pass declared, as menus.
    fn slots(declared: Declared) -> Vec<Open> {
        (0..declared.shown).map(Open::Title).chain(declared.more.then_some(Open::More)).collect()
    }
}

impl Component for MenuBar<'_> {
    const ROLE: Role = Role::MENU_BAR;
    type Style = MenuBarStyle;
    type Output = MenuBarOutput;

    fn style(_: &StyleCx<'_>, _: MenuBarStyleKey) -> MenuBarStyle {
        MenuBarStyle::default()
    }

    /// Every title's width when the room allows it; the bar fills the row, so what is left over is room for the
    /// overflow.
    fn measure(&mut self, cx: &MeasureCx<'_, Self>) -> Vec2 {
        if !Self::in_window(&cx.env) {
            return Vec2::ZERO;
        }
        let (widths, height, _) = self.widths(cx);
        Vec2::new(widths.iter().sum(), height)
    }

    fn fills(&self) -> Option<Axis> {
        Some(Axis::Horizontal)
    }

    fn sense(&self) -> Sense {
        Sense::HOVER
    }

    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        if !Self::in_window(&cx.env) {
            return;
        }
        let (widths, _, more) = self.widths(cx);
        let fit = fit_titles(&widths, more, rect.width());
        let mut flow = Flow::new(rect, cx.flow.is_rtl());
        for (at, width) in widths.iter().take(fit.shown).enumerate() {
            let (cell, rest) = flow.take_start(*width);
            flow = rest;
            cx.part(TITLE.at(at as u64), cell).sense(Sense::CLICK | Sense::FOCUS).focusable();
        }
        if fit.overflow {
            let (cell, _) = flow.take_start(more);
            cx.part(MORE, cell).sense(Sense::CLICK | Sense::FOCUS).focusable();
        }
        cx.roving(Roving::new(NavShape::Flat));
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("menu");
        let found = declared(cx);
        let slots = Self::slots(found);
        let texts = self.texts(&cx.env);
        let mut state = cx.memory::<BarState>().clone();
        let open = popup.is_open(cx);
        if !open {
            state.open = None;
        }
        let keys = if self.keyboard && Self::in_window(&cx.env) {
            cx.register_menu_bar();
            cx.take_menu_keys()
        } else {
            Vec::new()
        };
        for key in keys {
            match key {
                MenuKey::Letter(letter) => {
                    if let Some(at) = texts.iter().take(found.shown).position(|(_, held)| *held == Some(letter)) {
                        state.open = Some(Open::Title(at));
                        popup.open(cx);
                    }
                }
                MenuKey::Focus => {
                    if open {
                        popup.close(cx);
                        state.open = None;
                    }
                }
            }
        }
        let by_key = cx.activation().filter(|activation| activation.by_key).and(cx.active_part());
        let mut asked = None;
        while let Some((step, _)) = cx.intents().take_move() {
            if matches!(step, NavMove::Down) {
                asked = cx.active_part();
            }
        }
        let seen: Vec<(Open, PartRef, bool, Option<Pos2>)> = slots
            .iter()
            .map(|slot| {
                let part = Self::slot_part(*slot);
                let result = cx.part(part);
                (*slot, part, result.press.phase.is_press(), result.hover_pos)
            })
            .collect();
        for (slot, part, pressed, _) in &seen {
            if *pressed || by_key == Some(*part) || asked == Some(*part) {
                if open && state.open == Some(*slot) {
                    popup.close(cx);
                    state.open = None;
                } else {
                    state.open = Some(*slot);
                    popup.open(cx);
                }
            }
        }
        match seen.iter().find_map(|(slot, _, _, hover)| hover.map(|pos| (*slot, pos))) {
            Some((slot, pos)) => {
                if open && state.hover.is_some_and(|before| before != pos) && state.open != Some(slot) {
                    state.open = Some(slot);
                }
                state.hover = Some(pos);
            }
            None => state.hover = None,
        }
        self.open = state.open.is_some();
        Facts::<BarFact>::register(&cx.ctx, BarFact { open: self.open, focused: cx.is_focused() });
        *cx.memory::<BarState>() = state;
    }

    /// The open title's menu below it: the caller's body, with the help search above it in the Help menu. The
    /// overflow menu lists each hidden title as a submenu.
    fn overlay(&mut self, ui: &mut OverlayUi<'_>, cx: &mut Cx<'_, Self>) {
        let popup = cx.popup("menu");
        let found = declared(cx);
        let Some(open) = cx.memory::<BarState>().open else { return };
        let Some(rect) = cx.part_if_declared(Self::slot_part(open)).map(|result| result.rect) else { return };
        let words = cx.env.words::<MenuBarWords>();
        let mut query = cx.memory::<Query>().clone();
        let helped = match (self.help, open) {
            (Some(help), Open::Title(at)) if help.menu == at => Some(help.entries),
            _ => None,
        };
        let listed: Vec<usize> = match helped {
            Some(entries) => {
                let texts: Vec<[String; 3]> = entries
                    .iter()
                    .map(|entry| {
                        let path: Vec<String> = entry.path.iter().map(|label| cx.env.resolve(label).as_str().to_owned()).collect();
                        let english = entry.english.as_ref().map(|label| cx.env.resolve(label).as_str().to_owned());
                        [cx.env.resolve(&entry.label).as_str().to_owned(), english.unwrap_or_default(), path.join(PATH_SEPARATOR)]
                    })
                    .collect();
                let refs: Vec<[&str; 3]> = texts.iter().map(|[label, english, path]| [label.as_str(), english.as_str(), path.as_str()]).collect();
                let mut seen: Vec<&CommandRef> = Vec::new();
                search_order(&query.0, &refs)
                    .into_iter()
                    .filter(|at| {
                        let command = &entries[*at].command;
                        let fresh = !seen.contains(&command);
                        seen.push(command);
                        fresh
                    })
                    .collect()
            }
            None => Vec::new(),
        };
        let (titles, from) = (&mut self.titles, found.shown);
        let overlay = Overlay::menu().anchor(Anchor::Rect(rect));
        let typed = !query.0.trim().is_empty();
        ui.show(popup, overlay, |ui, cx| match open {
            Open::Title(at) => {
                if let Some(entries) = helped {
                    TextField::search(words.search.as_str(), Bind::bound(&mut query.0)).show(ui);
                    for at in &listed {
                        let entry = &entries[*at];
                        let path: Vec<String> = entry.path.iter().map(|label| cx.env.resolve(label).as_str().to_owned()).collect();
                        let row = MenuItem::command(entry.command.clone());
                        let row = if path.is_empty() { row } else { row.detail(path.join(PATH_SEPARATOR)) };
                        row.show(ui);
                    }
                    if listed.is_empty() && typed {
                        Text::new(words.no_matches.as_str()).show(ui);
                    }
                    Divider::horizontal().show(ui);
                }
                (titles[at].body)(ui);
            }
            Open::More => {
                for title in titles.iter_mut().skip(from) {
                    super::popup::submenu_rows(ui, title.label.clone(), |ui| (title.body)(ui));
                }
            }
        });
        *cx.memory::<Query>() = query;
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let found = declared(cx);
        let texts = self.texts(&cx.env);
        let words = cx.env.words::<MenuBarWords>();
        let state = cx.recall::<BarState>();
        let total = found.shown + usize::from(found.more);
        for (at, slot) in Self::slots(found).into_iter().enumerate() {
            let name = match slot {
                Open::Title(title) => texts[title].0.clone(),
                Open::More => Self::more_text(&words, found.shown),
            };
            let node = access.node(Self::slot_part(slot), Role::MENU_ITEM);
            node.name = Name::Text(name);
            node.expanded = Some(state.open == Some(slot));
            node.position = Some(SetPosition::new(at).of(total));
        }
    }

    fn paint(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let found = declared(cx);
        let texts = self.texts(&cx.env);
        let words = cx.env.words::<MenuBarWords>();
        let state = cx.recall::<BarState>();
        for slot in Self::slots(found) {
            let text = match slot {
                Open::Title(at) => texts[at].0.clone(),
                Open::More => Self::more_text(&words, found.shown),
            };
            ui.face(Self::slot_part(slot), Self::face(&text, state.open == Some(slot)));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> MenuBarOutput {
        let found = declared(&cx);
        let rects = Self::slots(found).into_iter().filter_map(|slot| cx.part_if_declared(Self::slot_part(slot)).map(|result| result.rect));
        let start = if cx.flow.is_rtl() { cx.rect.right_top() } else { cx.rect.left_top() };
        let titles = rects.reduce(Rect::union).unwrap_or_else(|| Rect::from_min_size(start, Vec2::ZERO));
        MenuBarOutput { response: cx.response.clone(), open: self.open, titles }
    }
}
