//! `TileGrid`: a collection of `Tile`s in wrapped cards or justified rows, under optional group headers. The item step
//! does the work: virtualisation, lead and selection rules, arrow, Home, End and page keys over the layout, type-ahead,
//! drag with drop marks, OS file drops, the rename lifecycle and the context-menu request. The grid adds the layout,
//! the pinned group headers with their parts, and the rating keys. It owns what its cards have in common: where the
//! caption goes, whether a detail line is reserved, and the zoom.

#[cfg(feature = "testing")]
mod stories;

use super::layout::{Around, ArrangeSpacing, GridLayout, Recipe, TileGroup, Tiling, arrange, aspects_of};
use super::tile::AroundPicture;
use super::{CaptionPlace, Tile, TilePartEvent};
use crate::contract::{
    Access, Bind, BindMut, Collection, CollectionEvent, CollectionProps, CollectionResponse, ColumnKind, ColumnSpec, Common, Component,
    ContainerProps, Cx, EditResponse, Focus as _, ListDrag, NoValue, Outcome, PaintUi, PartsCx, Pointer as _, Props as _, Role, RowState, SelectMode,
    Show as _, Sizing, SpaceAction, ToggleColumn,
};
use crate::controls::disclosure::Disclosure;
use crate::list::{DropTest, ListRow, RowTest, SourceTest};
use crate::primitives::PaintShape;
use crate::runtime::access::Name;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, RuleKind, StyleColor, StyleCx};
use craft_behavior::{
    ColumnKey, ColumnWidth, EditEvent, HitLevel, HitPriority, Intent, ItemSource, Metrics, Mixed, PartKey, PointerButton, PressPhase, Selection,
    Sense,
};
use craft_keymap::Mods;
use egui::{CornerRadius, Id, Pos2, Rect, Response, Ui, pos2, vec2};
use std::cell::RefCell;
use std::hash::Hash;

const GROUP: PartKey = PartKey::new("group");
/// The heading level of a group header: the grid's groups are its top-level sections.
const GROUP_LEVEL: usize = 1;
const TOGGLE: PartKey = PartKey::new("group_toggle");
/// The toggle column that carries the tiles' check boxes, so Space toggles the lead tile's check.
const CHECKS: ColumnKey = ColumnKey::new("check");

/// Which keys rate the selected tiles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RatingKeys {
    /// No key rates.
    #[default]
    None,
    /// The digits 1 to 5 rate and 0 clears. Other typed text still moves by type-ahead.
    Digits,
}

/// What the grid reports besides the collection's events.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum GridEvent<K> {
    /// A tile's stars were clicked (that tile), or a rating key was pressed (the selected tiles, or the lead when none
    /// is selected): the rating, 0 clears. The one route to a rating; the tile's own `TilePartEvent::Rate` does not
    /// reach the collection's events.
    #[non_exhaustive]
    Rate {
        /// The tiles to rate.
        keys: Vec<K>,
        /// The rating.
        rating: u8,
    },
    /// A group header was clicked.
    #[non_exhaustive]
    GroupPicked {
        /// The group's index in the list the grid was given.
        group: usize,
        /// The modifiers held.
        mods: Mods,
    },
    /// The disclosure of a group header, or AccessKit Expand or Collapse on it, asks for a state.
    #[non_exhaustive]
    GroupOpen {
        /// The group's index.
        group: usize,
        /// The state asked for.
        open: bool,
    },
    /// A group header asks for its context menu.
    #[non_exhaustive]
    GroupMenu {
        /// The group's index.
        group: usize,
        /// Where the menu opens: the header's bottom start corner.
        at: Pos2,
    },
}

/// What a `TileGrid` reports.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the grid's events"]
pub struct TileGridOutput<K> {
    /// The collection's response and events.
    pub response: CollectionResponse<K, TilePartEvent>,
    /// The grid's own events, in the order they happened.
    pub events: Vec<GridEvent<K>>,
}

impl<K> Outcome for TileGridOutput<K> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.response.story_facts()
    }
}

/// The sizes of the grid and the look of the pinned group headers; the headers' own look is the heading row's.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct TileGridStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// A header pinned over its group's tiles.
    pinned: StyleColor,
    /// The side of the disclosure column and the space beside the header's content.
    toggle: f32,
    pad: f32,
    /// The picture side of a card at zoom 1: the medium thumbnail.
    thumb: f32,
    /// What a tile adds around its picture.
    around: AroundPicture,
    /// The gaps of the arrangement and the height of a group header.
    spacing: ArrangeSpacing,
}

impl TileGridStyle {
    fn build(cx: &StyleCx<'_>) -> TileGridStyle {
        let colors = &cx.tokens.color;
        TileGridStyle {
            metrics: Metrics::new(),
            pinned: cx.color(colors.surface.panel),
            toggle: cx.tokens.size.icon.sm,
            pad: cx.tokens.space.gap.inline,
            thumb: cx.tokens.size.thumb.md,
            around: Tile::around(cx),
            spacing: ArrangeSpacing::new(cx.tokens.space.gap.stack, cx.tokens.space.gap.inline, cx.tokens.size.panel_header),
        }
    }

    /// The side of a picture at `zoom`, a factor on the medium thumbnail.
    fn picture_side(&self, zoom: f32) -> f32 {
        self.thumb * zoom.max(f32::EPSILON)
    }

    /// An equal card: a picture `side` wide with the aspect `aspect`, and what the tile adds around it.
    fn card(&self, side: f32, aspect: f32, around: Around) -> egui::Vec2 {
        vec2(side, side / aspect) + around.size()
    }
}

/// A grid of tiles: one Tab stop, the selection rules of the collection contract, type-ahead on the tiles' names,
/// rename in place, drag to reorder, files dropped from the OS, and the request for the app's context menu.
///
/// The picture side is `zoom` times the medium thumbnail side: the factor the app's zoom control holds. A card is the
/// picture plus what the tile adds around it, whatever the layout. Pictures are `aspect` wide over high; with
/// `justified` the tiles keep their own aspects (the source's `ItemSource::size_hint` as `SizeHint::Aspect`) and fill
/// rows. `groups` put headers over runs of tiles; a header stays pinned at the top while its group scrolls. The grid
/// decides where every tile's caption goes and whether the cards hold a detail line.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(collection, probe(TileGrid<'static, usize>))]
pub struct TileGrid<'a, K: Clone + Eq + Hash + Send + Sync + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    #[collection]
    items: CollectionProps,
    /// The selection the caller owns.
    #[prop]
    selection: Option<Bind<'a, Selection<K>>>,
    /// Scrolls the tile of `key` into view once.
    #[prop]
    reveal: Option<K>,
    /// Tiles keep their own aspect and fill rows, instead of equal cards.
    #[prop(flag)]
    justified: bool,
    /// The width of a card over its picture's height.
    #[prop(quantity = "aspect ratio")]
    aspect: f32,
    /// The picture side as a multiple of the medium thumbnail side; 1 is the medium thumbnail.
    #[prop(quantity = "zoom factor")]
    zoom: f32,
    /// Where every tile's caption goes; `CaptionPlace::None` keeps no room for one.
    #[prop]
    caption: CaptionPlace,
    /// Cards keep a line under the name for the tile's detail, whether a tile has one or not.
    #[prop(flag)]
    detail_line: bool,
    /// The headers over runs of tiles, in display order.
    #[prop]
    groups: &'a [TileGroup],
    /// Group headers show a disclosure that opens and closes their group.
    #[prop(flag)]
    collapsible: bool,
    /// Which keys rate the selected tiles.
    rating_keys: RatingKeys,
    /// Tiles show a check box and Space toggles the lead tile's check.
    checks: bool,
    /// Which tiles may be renamed; a rename request on another tile does nothing.
    #[prop(func = "Fn(&K) -> bool")]
    can_rename: Option<RowTest<'a, K>>,
    /// Which tiles may be dragged.
    #[prop(func = "Fn(&K) -> bool")]
    can_drag: Option<RowTest<'a, K>>,
    /// Which collections a drag may come from: the test gets the id of the collection the drag started in.
    #[prop(func = "Fn(Id) -> bool")]
    accept_from: Option<SourceTest<'a>>,
    /// Where a drag may land: the test gets the drag, the tile under the pointer and the position before which the
    /// dragged tiles would go.
    #[prop(func = "Fn(&ListDrag<K>, &K, usize) -> bool")]
    accepts: Option<DropTest<'a, K>>,
    /// Files dragged in from the OS may drop on the grid; a drop reports `CollectionEvent::FilesDropped`.
    #[prop]
    takes_files: bool,
    /// The layout of this pass, kept for the headers' parts and paint.
    laid: RefCell<Option<GridLayout>>,
    /// The headers this pass shows, in the grid's layer space: the group, its rect and whether it is pinned.
    placed: Vec<Placed>,
    events: Vec<GridEvent<K>>,
}

/// A group header as it shows this pass.
#[derive(Clone, Copy, Debug)]
struct Placed {
    group: usize,
    rect: Rect,
    pinned: bool,
}

impl<'a, K: Clone + Eq + Hash + Send + Sync + 'static> TileGrid<'a, K> {
    /// A grid named `label`, with multiple selection and equal square cards of the medium thumbnail.
    pub fn new(label: impl Into<Label>) -> Self {
        TileGrid {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::Fill),
            items: CollectionProps::new().select(SelectMode::Multi),
            selection: None,
            reveal: None,
            justified: false,
            aspect: 1.0,
            zoom: 1.0,
            caption: CaptionPlace::Below,
            detail_line: false,
            groups: &[],
            collapsible: false,
            rating_keys: RatingKeys::None,
            checks: false,
            can_rename: None,
            can_drag: None,
            accept_from: None,
            accepts: None,
            takes_files: false,
            laid: RefCell::new(None),
            placed: Vec::new(),
            events: Vec::new(),
        }
        .label(label)
    }

    /// Which keys rate the selected tiles. Digit keys reach the grid as digits only while this is on.
    #[must_use]
    pub fn rating_keys(mut self, keys: RatingKeys) -> Self {
        self.rating_keys = keys;
        self.items = self.items.digit_keys(keys == RatingKeys::Digits);
        self
    }

    /// Tiles show a check box (`Tile::check`) and Space toggles the lead tile's check. A toggle of a check reports
    /// `CollectionEvent::Item` with `TilePartEvent::Check`, as a click on the check box does.
    #[must_use]
    pub fn checks(mut self, on: bool) -> Self {
        self.checks = on;
        let columns = if on {
            vec![ColumnSpec::new(CHECKS, Label::default(), ColumnWidth::Fixed(0.0)).kind(ColumnKind::Toggle(ToggleColumn::default()))]
        } else {
            Vec::new()
        };
        self.items = self.items.columns(columns).header(!on).space(if on { SpaceAction::ToggleCell } else { SpaceAction::default() });
        self
    }

    /// The selected tiles, or the lead when none is selected: what a rating key rates. The caller's binding when it
    /// holds one, else the selection the item step keeps.
    fn rated(&mut self, cx: &Cx<'_, Self>) -> Vec<K> {
        let held = self.selection.as_mut().map(Bind::reborrow).and_then(|bind| bind.get());
        let selection = match held {
            Some(Mixed::Same(selection)) => Some(selection),
            Some(_) => None,
            None => cx.collection_selection().cloned(),
        };
        match selection {
            Some(selection) if !selection.items.is_empty() => selection.items,
            Some(selection) => selection.lead.into_iter().collect(),
            None => Vec::new(),
        }
    }

    /// The layout of this pass: `previous` when it was made from the same recipe, else a new arrangement. A card is
    /// the picture plus what the tile adds around it in every tiling.
    fn laid_out(&self, previous: Option<GridLayout>, source: &dyn ItemSource<K>, width: f32, cx: &StyleCx<'_>) -> GridLayout {
        let style = TileGridStyle::build(cx);
        let (side, spacing, around) = (style.picture_side(self.zoom), style.spacing, style.around.get(self.caption, self.detail_line));
        let aspect = if self.aspect.is_finite() && self.aspect > 0.0 { self.aspect } else { 1.0 };
        let recipe = self.items.rows_stamp.map(|stamp| Recipe {
            stamp,
            width,
            tiles: source.len(),
            side,
            aspect,
            justified: self.justified,
            around,
            groups: self.groups.iter().map(|group| (group.start, group.count, group.open)).collect(),
        });
        if let (Some(previous), Some(recipe)) = (previous, &recipe)
            && previous.is_made_from(recipe)
        {
            *self.laid.borrow_mut() = Some(previous.clone());
            return previous;
        }
        let layout = if self.justified || !self.groups.is_empty() {
            let tiling = if self.justified { Tiling::justified(side) } else { Tiling::cards(side, aspect) };
            GridLayout::arranged(arrange(&aspects_of(source), self.groups, width, tiling, spacing, around), width, around)
        } else {
            GridLayout::equal(source, width, spacing.inset, style.card(side, aspect, around), spacing.gap)
        };
        let layout = match recipe {
            Some(recipe) => layout.made_from(recipe),
            None => layout,
        };
        *self.laid.borrow_mut() = Some(layout.clone());
        layout
    }

    fn group_rect(rect: Rect, view: Rect, rtl: bool) -> Rect {
        if rtl {
            Rect::from_min_max(pos2(view.min.x + view.max.x - rect.max.x, rect.min.y), pos2(view.min.x + view.max.x - rect.min.x, rect.max.y))
        } else {
            rect
        }
    }

    /// The disclosure column of the header in `rect`, at its reading start.
    fn toggle_rect(rect: Rect, style: &TileGridStyle, rtl: bool) -> Rect {
        let x = if rtl { rect.max.x - style.pad - style.toggle } else { rect.min.x + style.pad };
        Rect::from_min_size(pos2(x, rect.min.y), vec2(style.toggle, rect.height()))
    }
}

impl<K: Clone + Eq + Hash + Send + Sync + 'static> Component for TileGrid<'_, K> {
    const ROLE: Role = Role::LIST_BOX;
    const OVERLAYS: bool = false;
    type Style = TileGridStyle;
    type Output = TileGridOutput<K>;

    fn style(cx: &StyleCx<'_>, _: TileGridStyleKey<K>) -> TileGridStyle {
        TileGridStyle::build(cx)
    }

    /// One part per header in view, over the tiles, and a disclosure part in it when headers collapse.
    fn parts(&mut self, area: Rect, cx: &mut PartsCx<'_, Self>) {
        self.placed.clear();
        let Some(arrangement) = self.laid.borrow().as_ref().and_then(|layout| layout.arrangement().cloned()) else { return };
        let (view, offset, rtl) = (cx.content_rect, cx.offset(), cx.flow.is_rtl());
        let top = offset.y as f32;
        let pinned = arrangement.group_at(top).and_then(|group| {
            let header = *arrangement.headers.get(group)?;
            if header.top() >= top {
                return None;
            }
            let lift = arrangement.headers.get(group + 1).map_or(top, |next| top.min(next.top() - header.height()));
            Some((group, Rect::from_min_size(pos2(header.left(), lift), header.size())))
        });
        let shift = view.min.to_vec2() - vec2(offset.x as f32, offset.y as f32);
        let clip = area;
        for (group, header) in arrangement.headers.iter().enumerate() {
            let (rect, held) = match pinned {
                Some((at, lifted)) if at == group => (lifted, true),
                _ => (*header, false),
            };
            let rect = Self::group_rect(rect.translate(shift), view, rtl);
            if !rect.intersects(clip) {
                continue;
            }
            self.placed.push(Placed { group, rect, pinned: held });
            let priority = HitPriority::new(HitLevel::Edge, 0);
            cx.part(GROUP.at(group as u64), rect).sense(Sense::CLICK).priority(priority).over().pointer_only(HEADER_REASON);
            if self.collapsible {
                let column = Self::toggle_rect(rect, cx.style, rtl);
                cx.part(TOGGLE.at(group as u64), column).sense(Sense::CLICK).priority(priority).over().pointer_only(HEADER_REASON);
            }
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if self.rating_keys == RatingKeys::Digits {
            for rating in 0..=MAX_RATING {
                while cx.intents().take_exact(&Intent::Digit(rating)) {
                    let keys = self.rated(cx);
                    if !keys.is_empty() {
                        self.events.push(GridEvent::Rate { keys, rating });
                    }
                }
            }
        }
        let placed = self.placed.clone();
        for Placed { group, rect, .. } in placed {
            let part = GROUP.at(group as u64);
            let toggle = TOGGLE.at(group as u64);
            let open = self.groups.get(group).is_some_and(|held| held.open);
            if let Some(result) = cx.part_if_declared(part)
                && let PressPhase::Release { start, click: true, .. } = result.press.phase
            {
                let mods = result.press.mods;
                self.events.push(match start.button {
                    PointerButton::Secondary => {
                        GridEvent::GroupMenu { group, at: if cx.flow.is_rtl() { rect.right_bottom() } else { rect.left_bottom() } }
                    }
                    _ => GridEvent::GroupPicked { group, mods },
                });
            }
            if self.collapsible
                && let Some(result) = cx.part_if_declared(toggle)
                && result.press.phase.is_click()
            {
                self.events.push(GridEvent::GroupOpen { group, open: !open });
            }
        }
        if self.collapsible {
            while let Some(request) = cx.intents().take_access() {
                let Some(target) = request.target else { continue };
                let wanted = match request.action {
                    craft_behavior::AccessAction::Expand => true,
                    craft_behavior::AccessAction::Collapse => false,
                    _ => continue,
                };
                let owner = self
                    .placed
                    .iter()
                    .map(|placed| placed.group)
                    .find(|group| GROUP.at(*group as u64) == target || TOGGLE.at(*group as u64) == target);
                if let Some(group) = owner {
                    self.events.push(GridEvent::GroupOpen { group, open: wanted });
                }
            }
        }
    }

    fn access(&self, access: &mut Access, _: &Cx<'_, Self>) {
        for placed in &self.placed {
            let Some(group) = self.groups.get(placed.group) else { continue };
            let node = access.node(GROUP.at(placed.group as u64), Role::HEADING);
            node.level(GROUP_LEVEL);
            node.name = Name::Text(if group.detail.is_empty() { group.title.clone() } else { format!("{}, {}", group.title, group.detail) });
            if self.collapsible {
                node.expanded = Some(group.open);
            }
        }
    }

    fn paint_over(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let style = cx.style;
        for placed in &self.placed {
            let Some(group) = self.groups.get(placed.group) else { continue };
            let rect = placed.rect;
            if placed.pinned {
                let draw = ui.draw();
                draw.fill(&PaintShape::Rect(rect, CornerRadius::ZERO), style.pinned);
                draw.rule(RuleKind::Divider, rect.left_bottom(), rect.right_bottom());
            }
            let mut heading = ListRow::new(group.title.clone()).heading().node(false);
            if !group.detail.is_empty() {
                heading = heading.trailing(group.detail.clone());
            }
            ui.face(GROUP.at(placed.group as u64), heading);
            if self.collapsible {
                let disclosure = Disclosure::new(Bind::given(group.open)).node(false).label(group.title.clone());
                ui.face(TOGGLE.at(placed.group as u64), disclosure);
            }
        }
    }

    /// The grid's events are the digits' and the stars' ratings, in that order, besides the collection's own.
    fn finish(self, mut cx: Cx<'_, Self>) -> TileGridOutput<K> {
        let mut response = cx.collection_response();
        let mut events = self.events;
        let mut kept = Vec::with_capacity(response.events.len());
        for event in std::mem::take(&mut response.events) {
            match event {
                CollectionEvent::Item(key, TilePartEvent::Rate(rating)) => events.push(GridEvent::Rate { keys: vec![key], rating }),
                CollectionEvent::Toggle(edit) if self.checks && matches!(edit, EditEvent::Set { .. } | EditEvent::Commit { .. }) => {
                    if let Some(toggle) = edit.value() {
                        kept.push(CollectionEvent::Item(toggle.key.clone(), TilePartEvent::Check { mods: Mods::NONE }));
                    }
                }
                other => kept.push(other),
            }
        }
        response.events = kept;
        TileGridOutput { response, events }
    }
}

/// The highest rating a digit key sets.
const MAX_RATING: u8 = 5;

const HEADER_REASON: &str = "group headers pick, open their menu, open and close by pointer; AccessKit Expand and Collapse also open and close them";

impl<K: Clone + Eq + Hash + Send + Sync + 'static> Collection for TileGrid<'_, K> {
    type Key = K;
    type ItemLayout = GridLayout;
    type Selection = Selection<K>;
    type Item<'r> = Tile<'r>;
    type ItemEvent = TilePartEvent;

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, Selection<K>>> {
        self.selection.as_mut().map(Bind::reborrow)
    }

    fn reveal(&self) -> Option<&K> {
        self.reveal.as_ref()
    }

    fn item_layout(&self, source: &dyn ItemSource<K>, width: f32, cx: &StyleCx<'_>) -> GridLayout {
        self.laid_out(None, source, width, cx)
    }

    /// The last layout again while the width, the rows stamp and everything the arrangement reads are the same.
    fn item_layout_from(&self, previous: Option<GridLayout>, source: &dyn ItemSource<K>, width: f32, cx: &StyleCx<'_>) -> GridLayout {
        self.laid_out(previous, source, width, cx)
    }

    /// Every tile gets the grid's caption place and detail line, and its row's rename state.
    fn dress<'r>(&self, tile: Self::Item<'r>, row: &RowState) -> Self::Item<'r> {
        tile.caption(self.caption).detail_line(self.detail_line).renaming(row.renaming)
    }

    fn can_rename(&self, key: &K) -> bool {
        self.can_rename.as_ref().is_none_or(|test| test.passes(key))
    }

    fn can_drag(&self, key: &K) -> bool {
        self.can_drag.as_ref().is_none_or(|test| test.passes(key))
    }

    fn accept_from(&self, from: Id) -> bool {
        self.accept_from.as_ref().is_none_or(|test| test.passes(from))
    }

    fn accepts(&self, drag: &ListDrag<K>, over: &K, insert: usize) -> bool {
        self.accepts.as_ref().is_none_or(|test| test.passes(drag, over, insert))
    }

    fn takes_files(&self) -> bool {
        self.takes_files
    }

    fn rename_editor(&mut self, ui: &mut Ui, draft: Bind<'_, String>, label: Rect, name: &str) -> Option<EditResponse<String>> {
        Some(crate::text::TextField::new(name.to_owned(), draft).show_at(ui, label))
    }
}
