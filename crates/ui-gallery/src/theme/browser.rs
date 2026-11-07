//! The token browser: every colour, component token and size of the subject theme as a list of groups, with filters
//! over it.

use super::chip::chip;
use super::rows::{Filters, Row, TokenRows};
use super::subject::Pick;
use super::words::{layer_words, lengths_words, value_words};
use craft_behavior::{Selection, SelectionChange};
use craft_theme::{Explained, Mode, Rgba, TokenRef};
use craft_ui::choice::Checkbox;
use craft_ui::contract::{Bind, CollectionEvent, Pattern, Props, SelectMode, Show as _, ShowCollection as _, ShowContainer as _};
use craft_ui::display::{Shade, Text};
use craft_ui::layout_components::{Row as Line, Stack};
use craft_ui::list::{List, ListRow, OpenSet, ThumbSize};
use craft_ui::runtime::memory::Remember as _;
use craft_ui::style::TypeRole;
use craft_ui::text::TextField;
use egui::Ui;

/// What the browser keeps between passes: the filters, which groups are open and the selected row.
#[derive(Clone, Default)]
struct Session {
    filters: Filters,
    open: OpenSet<Row>,
    selection: Selection<Row>,
    /// The pick the host gave last, so a new one selects its row and a pick the user made does not echo back.
    given: Option<Pick>,
}

/// What the browser reports: the row the user picked in this pass.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct BrowserOutput {
    /// The token or size the user selected or activated.
    pub picked: Option<Pick>,
}

/// The list of tokens of an explanation, in groups that open and close, with a filter by path and by pins, moves
/// and failing pairs.
///
/// - Selecting or activating a token or size row reports it as `picked`. A pick the host gives with
///   [`pick`](TokenBrowser::pick) selects its row, opens its group and scrolls it into view.
/// - A filter on shows every group's matches, open.
#[derive(Pattern, Props)]
pub struct TokenBrowser<'a> {
    explained: &'a Explained,
    /// The row the host has picked, which the list selects.
    #[prop]
    pick: Option<&'a Pick>,
}

impl<'a> TokenBrowser<'a> {
    /// A browser over `explained`.
    pub fn new(explained: &'a Explained) -> Self {
        TokenBrowser { explained, pick: None }
    }

    /// Shows the browser across the room offered.
    pub fn show(self, ui: &mut Ui) -> BrowserOutput {
        let TokenBrowser { explained, pick } = self;
        let mut out = BrowserOutput::default();
        Stack::new()
            .fill()
            .header(|ui, cx| {
                let filters = &mut cx.memory::<Session>().filters;
                Text::new(overview(explained)).role(TypeRole::Caption).shade(Shade::Muted).show(ui);
                TextField::search("Filter by path", Bind::bound(&mut filters.query)).placeholder("Filter by path").show(ui);
                Line::new().wrap(true).show(ui, |ui, _| {
                    Checkbox::new(Bind::bound(&mut filters.pinned)).label("Pinned").show(ui);
                    Checkbox::new(Bind::bound(&mut filters.moved)).label("Moved by contrast").show(ui);
                    Checkbox::new(Bind::bound(&mut filters.failing)).label("Failing pairs only").show(ui);
                });
            })
            .show(ui, |ui, cx| {
                let session = cx.memory::<Session>();
                let mut reveal = None;
                if session.given.as_ref() != pick {
                    session.given = pick.cloned();
                    reveal = pick.and_then(|pick| Row::of(pick, explained));
                    session.selection = reveal.map_or_else(Selection::default, Selection::single);
                    if let Some(group) = reveal.and_then(|row| row.group(explained)) {
                        session.open.set(&Row::Group(group), true);
                    }
                }
                let rows = TokenRows::of(explained, &session.filters, &session.open);
                let mut list = List::<Row>::new("Tokens")
                    .select(SelectMode::Single)
                    .tree(true)
                    .empty("No token matches the filters")
                    .selection(Bind::bound(&mut session.selection));
                if let Some(row) = reveal.filter(|row| rows.contains(row)) {
                    list = list.reveal(row);
                }
                let events = list.show_items(ui, &rows, &mut |key, _| row_of(explained, &rows, key)).events;
                let session = cx.memory::<Session>();
                for event in events {
                    match event {
                        CollectionEvent::Selection(SelectionChange::Set(keys)) => out.picked = keys.last().and_then(|row| row.pick(explained)),
                        CollectionEvent::Activate(row) => match row.pick(explained) {
                            Some(pick) => out.picked = Some(pick),
                            None => session.open.set(&row, !rows.is_open(&row)),
                        },
                        CollectionEvent::Open(change) => session.open.apply(&change.key, change.open, change.all, rows.headings()),
                        _ => {}
                    }
                }
                if out.picked.is_some() {
                    session.given.clone_from(&out.picked);
                }
            });
        out
    }
}

/// The line over the list: how many pairs the explanation measured and how many fall short.
fn overview(explained: &Explained) -> String {
    let variant = if explained.resolved.variant_used == Mode::Dark { "dark" } else { "light" };
    let failing = explained.pairs.iter().filter(|pair| !pair.pair.passes()).count();
    format!(
        "{variant} variant: {} pairs measured, {failing} below their minimum, {} reported",
        explained.pairs.len(),
        explained.resolved.warnings.len()
    )
}

/// The colour a token resolved to; a component token has none.
pub(super) fn value_of(explained: &Explained, token: TokenRef) -> Rgba {
    match token {
        TokenRef::Color(token) => explained.resolved.tokens.color(token),
        TokenRef::Component(_) => Rgba::TRANSPARENT,
    }
}

/// The list row of `key`: a heading, or a token with its colour chip and value, or a size with its value and layer.
fn row_of(explained: &Explained, rows: &TokenRows, key: &Row) -> ListRow<'static> {
    let depth = rows.depth_of(key);
    match key {
        Row::Group(name) => ListRow::new(*name).heading().depth(depth).group(rows.is_open(key)),
        Row::Token(TokenRef::Component(field)) => {
            ListRow::new(field.path()).depth(depth).trailing(lengths_words(&explained.resolved.tokens.component, *field))
        }
        Row::Token(token) => {
            let value = value_of(explained, *token);
            ListRow::new(token.path()).depth(depth).thumb(ThumbSize::Small, chip(value)).trailing(value.to_hex())
        }
        Row::Size(at) => match explained.sizes.get(*at) {
            Some(origin) => {
                ListRow::new(origin.path.clone()).depth(depth).trailing(format!("{} · {}", value_words(origin.value), layer_words(origin.layer)))
            }
            None => ListRow::spacer(),
        },
    }
}
