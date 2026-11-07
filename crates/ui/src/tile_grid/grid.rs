//! `TileGrid`: a collection of `Tile`s in wrapped cards or justified rows, under optional group headers. The item step
//! does the work: virtualisation, lead and selection rules, arrow, Home, End and page keys over the layout, type-ahead,
//! drag with drop marks, OS file drops, the rename lifecycle and the context-menu request. The grid adds the layout,
//! the pinned group headers with their parts, and the rating keys.

#[cfg(feature = "testing")]
mod stories;

use super::layout::{ArrangeSpacing, GridLayout, TileGroup, Tiling, arrange, aspects_of};
use super::{CaptionPlace, Tile, TilePartEvent};
use crate::contract::{
    Access, ActivateOn, Bind, BindMut, Collection, CollectionProps, CollectionResponse, Common, Component, ContainerProps, Cx, EditResponse,
    Focus as _, ListDrag, NoValue, Outcome, PaintUi, PartsCx, Pointer as _, Props as _, ReorderRule, Role, SelectMode, Show as _, Sizing,
    Styled as _,
};
use crate::controls::disclosure::Disclosure;
use crate::list::{DropTest, RowTest, SourceTest};
use crate::primitives::{PaintShape, TextRun};
use crate::runtime::access::Name;
use crate::runtime::commands::CommandScope;
use crate::runtime::env::Label;
use crate::style::{ComponentStyle, RuleKind, Stateful, States, StyleColor, StyleCx, SurfaceKind, TextLook, TypeRole};
use craft_behavior::{HitLevel, HitPriority, ItemSource, Metrics, Mixed, PartKey, PointerButton, PressPhase, Selection, Sense};
use craft_keymap::Mods;
use egui::{CornerRadius, Id, Pos2, Rect, Response, Ui, pos2, vec2};
use std::cell::RefCell;
use std::hash::Hash;

const GROUP: PartKey = PartKey::new("group");
/// The heading level of a group header: the grid's groups are its top-level sections.
const GROUP_LEVEL: usize = 1;
const TOGGLE: PartKey = PartKey::new("group_toggle");

/// Which keys rate the selected tiles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RatingKeys {
    /// No key rates.
    #[default]
    None,
    /// The digits 1 to 5 rate and 0 clears. Typed text is not matched against tile names while it is on.
    Digits,
}

/// What the grid reports besides the collection's events.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum GridEvent<K> {
    /// A rating key was pressed: the rating (0 clears) for the selected tiles, or the lead when none is selected.
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

/// The look of the group headers.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct TileGridStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The header's title.
    #[style(text)]
    group_title: TextLook,
    /// The header's detail at its trailing end.
    #[style(text)]
    group_detail: TextLook,
    /// The header's wash, which takes the hover.
    group_fill: Stateful<StyleColor>,
    /// A header pinned over its group's tiles.
    pinned: StyleColor,
    /// The side of the disclosure column and the space beside the header's content.
    toggle: f32,
    pad: f32,
}

/// A grid of tiles: one Tab stop, the selection rules of the collection contract, type-ahead on the tiles' names,
/// rename in place, drag to reorder, files dropped from the OS, and the request for the app's context menu.
///
/// The tile size is `zoom` times the medium thumbnail side: the value the app's zoom control holds. Cards are `aspect`
/// wide over high; with `justified` the tiles keep their own aspects (the source's `ItemSource::size_hint` as
/// `SizeHint::Aspect`) and fill rows. `groups` put headers over runs of tiles; a header stays pinned at the top while
/// its group scrolls.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(collection, probe(TileGrid<'static, usize>))]
pub struct TileGrid<'a, K: Clone + Eq + Hash + Send + Sync + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
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
    /// The tile size as a multiple of the medium thumbnail side; 1 is the medium thumbnail.
    #[prop(quantity = "zoom factor")]
    zoom: f32,
    /// Where a caption goes; `CaptionPlace::None` keeps no room for one.
    #[prop]
    caption: CaptionPlace,
    /// The headers over runs of tiles, in display order.
    #[prop]
    groups: &'a [TileGroup],
    /// Group headers show a disclosure that opens and closes their group.
    #[prop(flag)]
    collapsible: bool,
    /// Which keys rate the selected tiles.
    #[prop]
    rating_keys: RatingKeys,
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
            groups: &[],
            collapsible: false,
            rating_keys: RatingKeys::None,
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

    /// What the user can select.
    #[must_use]
    pub fn select(mut self, mode: SelectMode) -> Self {
        self.items = self.items.select(mode);
        self
    }

    /// Tiles reorder by dragging and by Alt with the arrow keys.
    #[must_use]
    pub fn reorder(mut self, rule: ReorderRule) -> Self {
        self.items = self.items.reorder(rule);
        self
    }

    /// Tiles rename in place: a double click on the caption, F2 or the rename command opens the editor.
    #[must_use]
    pub fn rename(mut self, on: bool) -> Self {
        self.items = self.items.rename(on);
        self
    }

    /// A press on the empty space between tiles empties the selection.
    #[must_use]
    pub fn allow_empty(mut self, on: bool) -> Self {
        self.items = self.items.allow_empty(on);
        self
    }

    /// Which gesture activates a tile.
    #[must_use]
    pub fn activate(mut self, on: ActivateOn) -> Self {
        self.items = self.items.activate(on);
        self
    }

    /// The tiles are the set `stamp` names; the lead starts over when the stamp changes.
    #[must_use]
    pub fn rows_stamp(mut self, stamp: u64) -> Self {
        self.items = self.items.rows_stamp(stamp);
        self
    }

    /// The text of the empty state.
    #[must_use]
    pub fn empty(mut self, text: impl Into<Label>) -> Self {
        self.items = self.items.empty(text);
        self
    }

    /// How the grid takes its size: to its content or to the room it has.
    #[must_use]
    pub fn sizing(mut self, sizing: Sizing) -> Self {
        self.props = self.props.sizing(sizing);
        self
    }

    /// Commands of `scope` reach the grid while focus is in it.
    #[must_use]
    pub fn command_scope(mut self, scope: CommandScope) -> Self {
        self.props = self.props.command_scope(scope);
        self
    }

    /// The surface the grid paints behind its tiles.
    #[must_use]
    pub fn surface(mut self, surface: SurfaceKind) -> Self {
        self.props = self.props.surface(surface);
        self
    }

    /// The selected tiles, or the lead when none is selected: what a rating key rates.
    fn rated(&mut self) -> Vec<K> {
        let held = self.selection.as_mut().map(Bind::reborrow).and_then(|bind| bind.get());
        match held {
            Some(Mixed::Same(selection)) if !selection.items.is_empty() => selection.items,
            Some(Mixed::Same(selection)) => selection.lead.into_iter().collect(),
            _ => Vec::new(),
        }
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

/// The spacing of the arrangement, from the tokens.
fn spacing(cx: &StyleCx<'_>) -> ArrangeSpacing {
    ArrangeSpacing::new(cx.tokens.space.gap.stack, cx.tokens.space.gap.inline, cx.tokens.size.panel_header)
}

impl<K: Clone + Eq + Hash + Send + Sync + 'static> Component for TileGrid<'_, K> {
    const ROLE: Role = Role::LIST_BOX;
    const OVERLAYS: bool = false;
    type Style = TileGridStyle;
    type Output = TileGridOutput<K>;

    fn style(cx: &StyleCx<'_>, _: TileGridStyleKey<K>) -> TileGridStyle {
        let colors = &cx.tokens.color;
        let clear = cx.color(craft_theme::Rgba::TRANSPARENT);
        let hover = cx.color(colors.state.hover.faded(HOVER_SHARE));
        TileGridStyle {
            metrics: Metrics::new(),
            group_title: TextLook::new(TypeRole::BodyStrong, Stateful::new(cx.color(colors.text.normal))),
            group_detail: TextLook::new(TypeRole::Caption, Stateful::new(cx.color(colors.text.faint))),
            group_fill: Stateful::new(clear).off(States::new(clear, hover, hover, clear)),
            pinned: cx.color(colors.surface.panel),
            toggle: cx.tokens.size.icon.sm,
            pad: cx.tokens.space.gap.inline,
        }
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
            while let Some(text) = cx.intents().take_text() {
                for rating in text.chars().filter_map(|letter| letter.to_digit(10)).filter(|digit| *digit <= MAX_RATING) {
                    let keys = self.rated();
                    if !keys.is_empty() {
                        self.events.push(GridEvent::Rate { keys, rating: rating as u8 });
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
                    PointerButton::Secondary => GridEvent::GroupMenu { group, at: rect.left_bottom() },
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
        let (style, rtl) = (cx.style, cx.flow.is_rtl());
        for placed in &self.placed {
            let Some(group) = self.groups.get(placed.group) else { continue };
            let rect = placed.rect;
            {
                let draw = ui.draw();
                if placed.pinned {
                    draw.fill(&PaintShape::Rect(rect, CornerRadius::ZERO), style.pinned);
                    draw.rule(RuleKind::Divider, rect.left_bottom(), rect.right_bottom());
                }
                let wash = cx.resolve_part(GROUP.at(placed.group as u64), &style.group_fill);
                draw.fill(&PaintShape::Rect(rect, CornerRadius::ZERO), wash);
            }
            let mut start = rect.shrink2(vec2(style.pad, 0.0));
            if self.collapsible {
                start = if rtl { start.with_max_x(start.max.x - style.toggle) } else { start.with_min_x(start.min.x + style.toggle) };
                let disclosure = Disclosure::new(Bind::given(group.open)).node(false).label(group.title.clone());
                ui.face(TOGGLE.at(placed.group as u64), disclosure);
            }
            let draw = ui.draw();
            let detail = (!group.detail.is_empty())
                .then(|| TextRun::new(group.detail.as_str()).role(style.group_detail.role).color(cx.resolve(&style.group_detail.color)).elide());
            let detail_width = detail.as_ref().map_or(0.0, |run| draw.layout(run, f32::INFINITY).size().x);
            let title = TextRun::new(group.title.as_str()).role(style.group_title.role).color(cx.resolve(&style.group_title.color)).elide();
            let laid = draw.layout(&title, (start.width() - detail_width - style.pad).max(0.0));
            let centre = |height: f32| rect.center().y - height / 2.0;
            let title_x = if rtl { start.max.x - laid.size().x } else { start.min.x };
            draw.galley(pos2(title_x, centre(laid.size().y)), &laid);
            if let Some(run) = detail {
                let shown = draw.layout(&run, detail_width);
                let detail_x = if rtl { start.min.x } else { start.max.x - shown.size().x };
                draw.galley(pos2(detail_x, centre(shown.size().y)), &shown);
            }
        }
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> TileGridOutput<K> {
        let response = cx.collection_response();
        TileGridOutput { response, events: self.events }
    }
}

/// The share of the hover colour a header's wash takes.
const HOVER_SHARE: f32 = 0.5;

/// The highest rating a digit key sets.
const MAX_RATING: u32 = 5;

const HEADER_REASON: &str = "group headers open and close by pointer and by the AccessKit Expand and Collapse actions";

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
        let spacing = spacing(cx);
        let side = cx.tokens.size.thumb.md * self.zoom.max(f32::EPSILON);
        let around = Tile::around_picture(cx, self.caption, false);
        let (start, band) = if self.caption == CaptionPlace::Start { (around.x, 0.0) } else { (0.0, around.y) };
        let layout = if self.justified || !self.groups.is_empty() {
            let tiling = if self.justified { Tiling::justified(side) } else { Tiling::cards(side, self.aspect, band) };
            GridLayout::arranged(arrange(&aspects_of(source), self.groups, width, tiling, spacing, start), width, start)
        } else {
            let aspect = if self.aspect.is_finite() && self.aspect > 0.0 { self.aspect } else { 1.0 };
            let card = vec2(side + around.x, side / aspect + around.y);
            GridLayout::equal(source, width, spacing.inset, card, spacing.gap)
        };
        *self.laid.borrow_mut() = Some(layout.clone());
        layout
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
