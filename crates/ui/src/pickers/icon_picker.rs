//! `IconPicker`: a search field over a grid of icon tiles in named groups.
//!
//! The pattern puts a `TextField::search` over a `TileGrid`. The field drives the grid: Up and Down move into the
//! tiles, Enter picks the active tile, and focus stays in the field. A click on a tile picks it.

#[cfg(feature = "testing")]
mod stories;

use super::words::PickerWords;
use crate::contract::{ActivateOn, Bind, CollectionEvent, NoValue, Outcome, Pattern, Props, SelectMode, Show as _, ShowCollection as _};
use crate::list::ThumbAspect;
use crate::primitives::ContentPainterCx;
use crate::runtime::env::{Env, Label};
use crate::text::TextField;
use crate::tile_grid::{CaptionPlace, Tile, TileGrid, TileGroup};
use craft_behavior::{DisplayOrder, EditEvent, GestureId, ItemSource, Mixed, Selection};
use craft_i18n::matcher::Matcher;
use egui::{Rect, Response, Ui};
use std::borrow::Cow;
use std::hash::Hash;
use std::sync::Arc;

/// One cell: what it picks, its name (tooltip, accessible name, search) and more words it is found by.
#[derive(Clone, Debug, PartialEq, Eq, Props)]
#[non_exhaustive]
pub struct IconPickerItem<K> {
    /// What the cell picks.
    pub key: K,
    /// The cell's name.
    pub label: String,
    /// Words the search also matches ("arrow", "direction").
    #[prop]
    pub keywords: Vec<String>,
}

impl<K> IconPickerItem<K> {
    /// A cell that picks `key`, named `label`, found by no other words.
    pub fn new(key: K, label: impl Into<String>) -> Self {
        Self { key, label: label.into(), keywords: Vec::new() }
    }

    /// Whether the name or a keyword matches `matcher`.
    fn matches(&self, matcher: &Matcher) -> bool {
        matcher.score(&self.label).is_some() || self.keywords.iter().any(|word| matcher.score(word).is_some())
    }
}

/// A titled group of cells; with a search typed the groups with no match are hidden.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct IconPickerGroup<K> {
    /// The group's title.
    pub name: String,
    /// The group's cells, in display order.
    pub items: Vec<IconPickerItem<K>>,
}

impl<K> IconPickerGroup<K> {
    /// A group titled `name` holding `items`.
    pub fn new(name: impl Into<String>, items: Vec<IconPickerItem<K>>) -> Self {
        Self { name: name.into(), items }
    }
}

/// What the picker reported this pass.
#[derive(Debug)]
#[non_exhaustive]
pub struct IconPickerOutput<K> {
    /// The grid's response.
    pub response: Response,
    /// The cell clicked or activated with Enter this pass. A host showing the picker in a popover closes it.
    pub picked: Option<K>,
    /// The search text changed.
    pub search_changed: bool,
}

impl<K> Outcome for IconPickerOutput<K> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        Outcome::story_facts(&self.response)
    }
}

/// The host's painter of a cell's icon: it paints the cell of a key into a rect.
pub struct CellPaint<'a, K>(Box<dyn Fn(&ContentPainterCx, Rect, &K) + 'a>);

impl<'a, K> CellPaint<'a, K> {
    /// The painter that runs `paint` for each cell, given the cell's key.
    pub fn new(paint: impl Fn(&ContentPainterCx, Rect, &K) + 'a) -> Self {
        CellPaint(Box::new(paint))
    }
}

/// A searchable grid of icons in named groups. The cell of the bound key is selected; a pick writes its key to the
/// binding and is reported in `picked`. The host paints each icon with `paint`; the cell's name is its tooltip and
/// accessible name, and the search matches it and its keywords.
///
/// ```ignore
/// let out = IconPicker::new("Tool", Bind::bound(&mut tool), &groups).paint(|cx, rect, key| draw_tool(cx, rect, key)).show(ui);
/// ```
#[derive(Pattern, Props)]
#[pattern(probe(IconPicker<'static, u8>))]
pub struct IconPicker<'a, K: Clone + Eq + Hash + Send + Sync + 'static> {
    label: Label,
    value: Bind<'a, Option<K>>,
    groups: &'a [IconPickerGroup<K>],
    /// Paints the icon of a cell.
    #[prop(func = "Fn(&ContentPainterCx, Rect, &K)")]
    paint: Option<CellPaint<'a, K>>,
}

/// The visible cells as the grid's source, keyed by position.
struct Cells<'a, K> {
    items: Vec<&'a IconPickerItem<K>>,
}

impl<K> DisplayOrder<usize> for Cells<'_, K> {
    fn len(&self) -> usize {
        self.items.len()
    }

    fn key(&self, at: usize) -> usize {
        at
    }

    fn index_of(&self, key: &usize) -> Option<usize> {
        (*key < self.items.len()).then_some(*key)
    }

    fn is_section(&self, _: usize) -> bool {
        false
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.items[at].label.as_str()))
    }
}

impl<K> ItemSource<usize> for Cells<'_, K> {}

impl<'a, K: Clone + Eq + Hash + Send + Sync + 'static> IconPicker<'a, K> {
    /// A picker named `label` over `value`, choosing among the cells of `groups`.
    pub fn new(label: impl Into<Label>, value: Bind<'a, Option<K>>, groups: &'a [IconPickerGroup<K>]) -> Self {
        IconPicker { label: label.into(), value, groups, paint: None }
    }

    /// Shows the picker and returns what the user did this pass.
    pub fn show(self, ui: &mut Ui) -> IconPickerOutput<K> {
        let IconPicker { label, mut value, groups, paint } = self;
        let env = Env::of(ui);
        let words = env.words::<PickerWords>();
        let grid_id = ui.scope_id().with("icons");
        let search = TextField::search(words.icon_search.clone(), Bind::kept()).drives(grid_id).show(ui);
        let search_changed = search.event.is_some();
        let query = match &search.shown {
            Some(Mixed::Same(shown)) => shown.trim().to_owned(),
            _ => String::new(),
        };
        let matcher = (!query.is_empty()).then(|| Matcher::new(&env.folder(), &query));

        let mut items = Vec::new();
        let mut shown_groups = Vec::new();
        for group in groups {
            let kept: Vec<&IconPickerItem<K>> =
                group.items.iter().filter(|item| matcher.as_ref().is_none_or(|matcher| item.matches(matcher))).collect();
            if kept.is_empty() {
                continue;
            }
            if !group.name.is_empty() {
                shown_groups.push(TileGroup::new(items.len(), kept.len(), group.name.clone()));
            }
            items.extend(kept);
        }
        let held: Option<K> = match value.reborrow().get() {
            Some(Mixed::Same(Some(key))) => Some(key),
            _ => None,
        };
        let selection =
            held.as_ref().and_then(|held| items.iter().position(|item| item.key == *held)).map_or_else(Selection::default, Selection::single);
        let source = Cells { items };
        let painter = paint.as_ref();
        let grid = TileGrid::<usize>::new(label.clone())
            .id(grid_id)
            .select(SelectMode::Single)
            .activate(ActivateOn::Click)
            .caption(CaptionPlace::None)
            .groups(&shown_groups)
            .selection(Bind::given(selection))
            .rows_stamp(query_stamp(&query))
            .empty(words.icon_no_match.clone())
            .show_items(ui, &source, &mut |at, _| {
                let item = source.items[*at];
                let tile = Tile::new(item.label.clone());
                match painter {
                    Some(painter) => tile.thumb(ThumbAspect::SQUARE, move |cx, rect| (painter.0)(cx, rect, &item.key)),
                    None => tile,
                }
            });
        let picked = grid.response.events.iter().find_map(|event| match event {
            CollectionEvent::Activate(at) => source.items.get(*at).map(|item| item.key.clone()),
            _ => None,
        });
        if let Some(key) = &picked
            && held.as_ref() != Some(key)
        {
            let name: Arc<str> = Arc::from(label.as_str());
            value.reborrow().apply(&EditEvent::set(GestureId::default(), Mixed::Same(held), Some(key.clone()), name, None));
        }
        IconPickerOutput { response: grid.response.response, picked, search_changed }
    }
}

/// The rows stamp of a search text: the same text keeps the layout, another one rebuilds it.
fn query_stamp(query: &str) -> u64 {
    use std::hash::Hasher as _;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    query.hash(&mut hasher);
    hasher.finish()
}
