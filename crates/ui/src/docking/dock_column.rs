//! `DockColumn`: a column of a side dock with its column bar and stacked groups, and the pure size rules of columns.
//!
//! The column is one container. Its bar is a `TitleRow` (a disclosure that collapses the column to icons and a menu
//! action) over a drag band; the groups below it stack at the heights `group_heights` gives and are shown by the
//! caller's group view. The column registers its edge bands and seams as drop spots of the dock and reports what the
//! user did; the dock decides what a drop changes.

use super::divider::{self, Divider, Drag};
use super::{DockHeights, DockSizes, DockSpot, DockTargets, DockWords, DragUnit, PanelKey, PanelSpec, Splitter, start_group_drag};
use crate::contract::SizeLimits;
use crate::contract::overlay::{Anchor, Popup, Trigger};
use crate::contract::{
    Access, Bind, BodyCx, Common, Component, Container, ContainerProps, Cx, HeaderRegion, Name, NoValue, Outcome, Pad, PaintUi, PartsCx, Pointer,
    Props as _, Role, Show as _, ShowContainer as _, Sizing, WordSetCx as _,
};
use crate::menu::{GroupItem, Menu, MenuWords, PanelMenu, PanelRow};
use crate::runtime::env::Label;
use crate::runtime::focus::KeyUse;
use crate::runtime::memory::sub;
use crate::sections::{HeaderAction, TitleRow};
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::divider::{self as push, Segment};
use craft_behavior::dock::{Column, Group};
use craft_behavior::{Axis, CursorIcon, DockSide, Flow, Metrics, PartKey, PressPhase, Sense, Value};
use egui::{Context, Id, Pos2, Rect, Response, Ui, Vec2};
use std::hash::Hash;

#[cfg(feature = "testing")]
mod stories;

pub(super) const BAR: PartKey = PartKey::new("bar");
const DIVIDER: PartKey = PartKey::new("divider");

/// The caller's view of one group: the group, the rect it takes and the body step's context. The dock passes the
/// group to `DockPanels::group_view` or shows a `DockGroup` in it.
pub type GroupView<'a, P> = Box<dyn FnMut(&Group<P>, Rect, &mut Ui, &mut BodyCx<'_>) + 'a>;

/// The width bounds of an expanded column: no narrower than `least` or the widest minimum of its panels, no wider
/// than the narrowest maximum.
pub fn width_bounds(least: f32, limits: impl IntoIterator<Item = SizeLimits>) -> (f32, f32) {
    let (mut lo, mut hi) = (least, f32::INFINITY);
    for limit in limits {
        lo = lo.max(limit.min.x);
        hi = hi.min(limit.max.map_or(f32::INFINITY, |max| max.x));
    }
    (lo, hi.max(lo))
}

/// Column widths that fit `room`. Each entry is `(width, floor)`: a column with a floor shrinks in proportion, never
/// below its floor; one without keeps its width. A dock saved on a larger screen still fits as far as the floors
/// allow.
pub fn fit_widths(columns: &[(f32, Option<f32>)], room: f32) -> Vec<f32> {
    let mut widths: Vec<f32> = columns.iter().map(|(width, _)| *width).collect();
    for _ in 0..columns.len() {
        let total: f32 = widths.iter().sum();
        if total <= room {
            break;
        }
        let flex: f32 =
            widths.iter().zip(columns).filter(|(width, (_, floor))| floor.is_some_and(|floor| **width > floor)).map(|(width, _)| *width).sum();
        if flex <= 0.0 {
            break;
        }
        let scale = ((room - (total - flex)) / flex).max(0.0);
        for (width, (_, floor)) in widths.iter_mut().zip(columns) {
            if let Some(floor) = *floor
                && *width > floor
            {
                *width = (*width * scale).max(floor);
            }
        }
    }
    widths
}

/// A group of a column as [`group_heights`] sizes it. Heights include the tab strip.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct GroupSize {
    /// Only the strip shows.
    pub minimized: bool,
    /// The height the user set; `None` or not finite when the group was never resized.
    pub saved: Option<f32>,
    /// The least height.
    pub min: f32,
    /// The heights the panels ask for, with the strip added to `fill` only when it is above 0, so 0 still means the
    /// group does not ask to fill.
    pub content: Option<DockHeights>,
}

impl GroupSize {
    /// An open group no shorter than `min` that was never resized and asks for no heights.
    pub fn new(min: f32) -> Self {
        GroupSize { minimized: false, saved: None, min, content: None }
    }
}

/// The heights of stacked groups in a column body `room` high. Minimized groups take `strip`, groups with a saved
/// height keep it, and the groups the user never resized whose content asks to fill share what is left. Without
/// those, the groups without content heights share it, and when every open group has a height the last one takes
/// the rest. Other groups take their default height, or their minimum. Default heights shrink towards their compact
/// heights until the filling groups get their fill height, nearest the filling groups first; then every height
/// shrinks in proportion towards its minimum.
pub fn group_heights(groups: &[GroupSize], strip: f32, room: f32) -> Vec<f32> {
    let saved = |group: &GroupSize| group.saved.filter(|height| height.is_finite());
    let open: Vec<usize> = (0..groups.len()).filter(|at| groups.get(*at).is_some_and(|group| !group.minimized)).collect();
    let unsaved =
        |at: &usize, test: fn(Option<DockHeights>) -> bool| groups.get(*at).is_some_and(|group| saved(group).is_none() && test(group.content));
    let mut shared: Vec<usize> = open.iter().copied().filter(|at| unsaved(at, |content| content.is_some_and(|content| content.fill > 0.0))).collect();
    if shared.is_empty() {
        shared = open.iter().copied().filter(|at| unsaved(at, |content| content.is_none())).collect();
    }
    if shared.is_empty() {
        shared.extend(open.last());
    }
    let sized: Vec<usize> = open.iter().copied().filter(|at| !shared.contains(at)).collect();
    let mut defaulted: Vec<usize> = sized.iter().copied().filter(|at| groups.get(*at).is_some_and(|group| saved(group).is_none())).collect();
    let mut heights: Vec<f32> = groups
        .iter()
        .enumerate()
        .map(|(at, group)| {
            if group.minimized {
                strip
            } else if shared.contains(&at) {
                0.0
            } else {
                saved(group).or(group.content.map(|content| content.default)).unwrap_or(group.min).max(group.min)
            }
        })
        .collect();
    let mins: Vec<f32> = groups.iter().map(|group| if group.minimized { strip } else { group.min }).collect();
    let compacts: Vec<f32> = groups.iter().zip(&mins).map(|(group, min)| group.content.map_or(*min, |content| content.compact).max(*min)).collect();
    let fill: f32 =
        shared.iter().filter_map(|at| groups.get(*at)).map(|group| group.content.map_or(group.min, |content| content.fill).max(group.min)).sum();
    defaulted.sort_by_key(|at| shared.iter().map(|shared| shared.abs_diff(*at)).min());
    let mut deficit = heights.iter().sum::<f32>() + fill - room;
    for at in defaulted {
        if deficit <= 0.0 {
            break;
        }
        if let (Some(height), Some(floor)) = (heights.get_mut(at), compacts.get(at)) {
            let give = (*height - floor).clamp(0.0, deficit);
            *height -= give;
            deficit -= give;
        }
    }
    let shared_min: f32 = shared.iter().filter_map(|at| mins.get(*at)).sum();
    give_way(&mut heights, &sized, &mins, shared_min - room);
    let free = (room - heights.iter().sum::<f32>()).max(0.0);
    // The shared groups split what is left evenly, each at least its minimum: groups whose minimum is above the even
    // share take their minimum and the rest split the remainder.
    let mut floors: Vec<f32> = shared.iter().filter_map(|at| mins.get(*at).copied()).collect();
    floors.sort_by(|a, b| b.total_cmp(a));
    let (mut rest, mut count) = (free, floors.len());
    for floor in floors {
        if count > 1 && floor > rest / count as f32 {
            rest -= floor;
            count -= 1;
        } else {
            break;
        }
    }
    let share = rest / count.max(1) as f32;
    for at in &shared {
        if let (Some(height), Some(min)) = (heights.get_mut(*at), mins.get(*at)) {
            *height = share.max(*min);
        }
    }
    heights
}

/// Shrinks the `which` heights in proportion towards their `floors` until the column's heights plus `extra` fit, or
/// every one is at its floor.
fn give_way(heights: &mut [f32], which: &[usize], floors: &[f32], extra: f32) {
    let deficit = heights.iter().sum::<f32>() + extra;
    let slack: f32 = which.iter().filter_map(|at| Some((heights.get(*at)? - floors.get(*at)?).max(0.0))).sum();
    if deficit > 0.0 && slack > 0.0 {
        let take = deficit.min(slack) / slack;
        for at in which {
            if let (Some(height), Some(floor)) = (heights.get_mut(*at), floors.get(*at)) {
                *height -= (*height - floor).max(0.0) * take;
            }
        }
    }
}

/// The rects of groups `heights` high, stacked from the top of `area`.
pub fn stacked(area: Rect, heights: &[f32]) -> Vec<Rect> {
    let mut top = area.top();
    heights
        .iter()
        .map(|height| {
            let rect = Rect::from_min_max(Pos2::new(area.left(), top), Pos2::new(area.right(), top + height));
            top += height;
            rect
        })
        .collect()
}

/// The sizes of the groups of `column` for [`group_heights`]: the strip added to the tallest minimum of a group's
/// panels, and the largest of each of their [`DockHeights`] with the strip added.
fn group_sizes<P>(column: &Column<P>, spec: &dyn Fn(&P) -> PanelSpec, sizes: &DockSizes) -> Vec<GroupSize> {
    column
        .groups
        .iter()
        .map(|group| {
            let specs: Vec<PanelSpec> = group.panels.iter().map(spec).collect();
            let body = specs.iter().map(|spec| spec.limits.min.y).fold(sizes.least, f32::max);
            let content = specs
                .iter()
                .filter_map(|spec| spec.heights.filter(|heights| heights.valid()))
                .reduce(|a, b| DockHeights::new(a.default.max(b.default)).compact(a.compact.max(b.compact)).fill(a.fill.max(b.fill)))
                .map(|heights| {
                    DockHeights::new(heights.default + sizes.strip)
                        .compact(heights.compact.min(heights.default) + sizes.strip)
                        .fill(if heights.fill > 0.0 { heights.fill + sizes.strip } else { 0.0 })
                });
            let mut size = GroupSize::new(sizes.strip + body);
            size.minimized = group.minimized;
            size.saved = group.height;
            size.content = content;
            size
        })
        .collect()
}

/// What the column bar recorded while the body ran, for the container to read when it finishes.
#[derive(Default)]
pub(super) struct BarRecord {
    /// The user asked to collapse (`true`) or expand (`false`) the column.
    pub(super) iconic: Option<bool>,
    /// The id of the bar's menu button, when it was pressed.
    pub(super) opener: Option<Id>,
    /// The dock row picked in the column menu.
    pub(super) picked: Option<GroupItem>,
}

/// Shows the column bar and its menu, at `rect` or in the flow of `ui`: the disclosure collapses an expanded column and
/// expands an iconic one, the menu button and the context request open the column menu. Returns what the user did.
pub(super) fn show_bar(ui: &mut Ui, body: &mut BodyCx<'_>, rect: Option<Rect>, expanded: bool, auto_collapse: bool, menu_button: bool) -> BarRecord {
    let words = body.words::<DockWords>();
    let menu_words = body.words::<MenuWords>();
    let mut open = expanded;
    let actions: &[HeaderAction] = if menu_button { &[HeaderAction::Menu] } else { &[] };
    let toggle = if expanded { words.collapse_column.as_str() } else { words.expand_column.as_str() };
    let row =
        TitleRow::new(Label::default()).disclosure(Bind::bound(&mut open)).lead_name(Label::from(toggle.to_owned())).actions(actions.iter().copied());
    let shown = match rect {
        Some(rect) => row.show_at(ui, rect),
        None => row.show(ui),
    };
    let opener = shown.edit.response.id;
    let mut record = BarRecord::default();
    if open != expanded {
        record.iconic = Some(!open);
    }
    if shown.action == Some(HeaderAction::Menu) {
        record.opener = Some(opener);
    }
    let rows = vec![
        PanelRow::Item(crate::contract::Entry::new(GroupItem::CollapseToIcons, Label::from(toggle.to_owned()))),
        PanelRow::Item(
            crate::contract::Entry::new(GroupItem::AutoCollapse, Label::from(menu_words.panel_auto_collapse.as_str().to_owned()))
                .checked(auto_collapse),
        ),
    ];
    let mut menu = Menu::new(Label::from(words.panels.as_str().to_owned())).opened_by(&shown.edit.response, Trigger::Manual);
    if let Some(button) = shown.menu_rect {
        menu = menu.anchor(Anchor::Rect(button));
    }
    let picked = menu.opened_by(&shown.edit.response, Trigger::Context).show(ui, |ui, _| PanelMenu::new(Vec::new(), rows.clone()).show(ui).picked);
    record.picked = picked.inner.flatten();
    record
}

/// The name of a column's bar: the labels of its panels.
pub(super) fn bar_name<P>(column: &Column<P>, spec: &dyn Fn(&P) -> PanelSpec, template: &crate::runtime::env::WordTemplate) -> String {
    template.fill(&[("name", panel_names(column, spec).as_str())])
}

/// The labels of a column's panels, in order.
fn panel_names<P>(column: &Column<P>, spec: &dyn Fn(&P) -> PanelSpec) -> String {
    let names: Vec<String> = column.groups.iter().flat_map(|group| group.panels.iter()).map(|panel| spec(panel).label.as_str().to_owned()).collect();
    names.join(", ")
}

/// Starts the move of the whole column when its bar is dragged: the drag session carries the column, named by its
/// first panel, and its ghost is a frame the size of the column.
pub(super) fn drag_column<P, C>(cx: &mut C, column: &Column<P>, spec: &dyn Fn(&P) -> PanelSpec, size: Vec2)
where
    P: Value + Hash,
    C: Pointer,
{
    let Some(first) = column.groups.first() else { return };
    let Some(anchor) = first.panels.first() else { return };
    let active = PanelKey::of(first.active_panel().unwrap_or(anchor));
    start_group_drag(cx, DragUnit::Column(anchor.clone()), active, Some((size, Label::from(panel_names(column, spec)))));
}

/// What a dock column reports.
#[derive(Debug)]
#[non_exhaustive]
pub struct DockColumnOutput<P> {
    /// The column's response.
    pub response: Response,
    /// The rect the column took, for `DockEvent::ColumnPlaced`.
    pub placed: Rect,
    /// The user asked to collapse the column to icons (`true`); the host sets `Column::iconic`.
    pub iconic: Option<bool>,
    /// The dock row the user picked in the column menu.
    pub picked: Option<GroupItem>,
    /// The groups whose height the user changed this pass, each named by its first panel, with the new height.
    pub resized: Vec<(P, f32)>,
}

impl<P> Outcome for DockColumnOutput<P> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The style of a dock column: the sizes its bar, groups and drop bands take.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DockColumnStyle {
    #[style(metrics)]
    metrics: Metrics,
    sizes: DockSizes,
}

/// The rects of a column's groups as this pass's resizes leave them, for the body.
#[derive(Clone, Default)]
struct GroupRects(Vec<Rect>);

const RECTS: &str = "column_rects";

/// One column of a side dock: the column bar over the stacked groups of `column`. Show it with
/// [`show_groups`](DockColumn::show_groups); the closure shows one group in the rect it is given, through
/// `DockPanels::group_view` or a `DockGroup`. A double click on the bar collapses the column to icons; the bar's
/// drag starts the move of the column through the drag session; Enter on the bar's menu button, the Menu key and a right
/// click open the column menu, whose dock rows come back in `picked`. A divider between two open groups resizes
/// them by pointer or with the arrow keys.
///
/// The column registers the drop bands of its two edges (`DockSpot::column`) and of the seams between its groups
/// (`DockSpot::seam`) with the dock `dock` every pass. A dock hidden by the shell shows nothing.
#[derive(crate::contract::Component, crate::contract::Props)]
#[component(container, probe(DockColumn<'static, u8>))]
pub struct DockColumn<'a, P: Value + Hash + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    dock: Id,
    column: &'a Column<P>,
    spec: &'a dyn Fn(&P) -> PanelSpec,
    /// Whether Auto-Collapse Iconic Panels is on, which the column menu shows as checked.
    #[prop]
    auto_collapse: bool,
    bar: BarRecord,
    resized: Vec<(P, f32)>,
    toggled: bool,
}

impl<'a, P: Value + Hash + 'static> DockColumn<'a, P> {
    /// A column named `label` of the dock `dock`, over `column`; `spec` reads each panel's limits and heights.
    pub fn new(label: impl Into<Label>, dock: Id, column: &'a Column<P>, spec: &'a dyn Fn(&P) -> PanelSpec) -> Self {
        let props = ContainerProps::new().surface(SurfaceKind::Panel).sizing(Sizing::Fill).pad(Pad::None).header(HeaderRegion::new());
        DockColumn {
            common: Common::new(),
            props,
            dock,
            column,
            spec,
            auto_collapse: false,
            bar: BarRecord::default(),
            resized: Vec::new(),
            toggled: false,
        }
        .label(label)
    }

    /// Shows the column; `view` shows one group in the rect it gets.
    pub fn show_groups(self, ui: &mut Ui, mut view: impl FnMut(&Group<P>, Rect, &mut Ui, &mut BodyCx<'_>) + 'a) -> DockColumnOutput<P> {
        let column = self.column;
        self.show(ui, move |ui, body| {
            if body.shell().is_some_and(|shell| !shell.dock_visible()) {
                return;
            }
            let rects = body.memory.recall::<GroupRects>(sub(body.id, RECTS)).0;
            for (group, rect) in column.groups.iter().zip(&rects) {
                view(group, *rect, ui, body);
            }
        })
        .output
    }

    /// The bar and body of the column in `rect`, and the groups stacked in the body.
    fn plan(&self, rect: Rect, sizes: &DockSizes) -> Plan {
        let bar = Rect::from_min_max(rect.min, Pos2::new(rect.right(), (rect.top() + sizes.bar).min(rect.bottom())));
        let body = Rect::from_min_max(Pos2::new(rect.left(), bar.bottom()), rect.max);
        let group_sizes = group_sizes(self.column, self.spec, sizes);
        let heights = group_heights(&group_sizes, sizes.strip, body.height());
        let groups = stacked(body, &heights);
        let open = (0..group_sizes.len()).filter(|at| !group_sizes[*at].minimized).collect();
        Plan { bar, body, groups, heights, mins: group_sizes.iter().map(|size| size.min).collect(), open }
    }

    fn anchor(&self, group: usize) -> Option<&P> {
        self.column.groups.get(group).and_then(|group| group.panels.first())
    }

    /// Moves the divider after the `index`-th open group by `delta` points: the heights that change come out as
    /// resizes, and the plan takes them so the groups are shown at their new heights in the same pass.
    fn move_divider(&mut self, plan: &mut Plan, index: usize, delta: f32) {
        let row = plan.row();
        let at: f32 = row.iter().take(index + 1).map(|segment| segment.extent).sum();
        let sizes = push::resize(&row, index, at + delta);
        for ((group, next), segment) in plan.open.clone().into_iter().zip(sizes).zip(&row) {
            if next != segment.extent {
                if let Some(anchor) = self.anchor(group) {
                    self.resized.push((anchor.clone(), next));
                }
                plan.heights[group] = next;
            }
        }
        plan.groups = stacked(plan.body, &plan.heights);
    }
}

/// Where the parts of a column sit.
struct Plan {
    bar: Rect,
    body: Rect,
    groups: Vec<Rect>,
    heights: Vec<f32>,
    mins: Vec<f32>,
    open: Vec<usize>,
}

impl Plan {
    /// The open groups as a row of segments, for the push rule of the dividers between them.
    fn row(&self) -> Vec<Segment> {
        self.open.iter().filter_map(|at| Some(Segment::unbounded(*self.heights.get(*at)?, *self.mins.get(*at)?))).collect()
    }

    /// The divider after each open group but the last: its part index (the group above it), its position among the open
    /// groups and the divider.
    fn dividers(&self, rect: Rect, reach: f32) -> Vec<(u64, usize, Divider)> {
        let row = self.row();
        self.open
            .windows(2)
            .enumerate()
            .filter_map(|(index, pair)| {
                let at = self.groups.get(pair[0])?.bottom();
                let (lo, hi) = push::travel(&row, index);
                Some((pair[0] as u64, index, Divider::new(Axis::Y, at, rect, (at + lo, at + hi), reach)))
            })
            .collect()
    }
}

impl<P: Value + Hash + 'static> Component for DockColumn<'_, P> {
    const ROLE: Role = Role::GROUP;
    const KEYS: KeyUse = KeyUse::SPLITTER;
    type Style = DockColumnStyle;
    type Output = DockColumnOutput<P>;

    fn style(cx: &StyleCx<'_>, _: DockColumnStyleKey<P>) -> DockColumnStyle {
        DockColumnStyle { metrics: Metrics::new(), sizes: DockSizes::style(cx) }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    /// The bar's drag band under the bar's own controls, and a divider between every two open groups.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let sizes = cx.style.sizes;
        let plan = self.plan(rect, &sizes);
        cx.part(BAR, plan.bar)
            .sense(Sense::DRAG)
            .cursor(CursorIcon::Grab)
            .drag_cursor(CursorIcon::Grabbing)
            .pointer_only("Move Panel To in the panel menu moves the panels of the column");
        for (key, _, divider) in plan.dividers(rect, sizes.reach) {
            divider::declare(cx, rect, DIVIDER, key, &divider);
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let sizes = cx.style.sizes;
        let (rect, rtl, ctx) = (cx.rect, cx.flow.is_rtl(), cx.core.ctx.clone());
        let mut plan = self.plan(rect, &sizes);
        let bar = cx.part(BAR).clone();
        if matches!(bar.press.phase, PressPhase::DragStart { start, .. } if start.button == craft_behavior::PointerButton::Primary) {
            drag_column(cx, self.column, self.spec, rect.size());
        }
        self.toggled = bar.press.double_pressed();
        let dividers = plan.dividers(rect, sizes.reach);
        for (key, index, divider) in &dividers {
            if let Some(Drag::To(coord)) = divider::pointer(cx, DIVIDER, *key, divider) {
                self.move_divider(&mut plan, *index, coord - divider.at);
            }
        }
        let keyed: Vec<(u64, Divider)> = dividers.iter().map(|(key, _, divider)| (*key, *divider)).collect();
        if let Some((key, coord)) = divider::keys(cx, DIVIDER, &keyed, sizes.step, sizes.jump)
            && let Some((_, index, divider)) = dividers.iter().find(|(at, ..)| *at == key)
        {
            self.move_divider(&mut plan, *index, coord - divider.at);
        }
        cx.core.memory.shown::<GroupRects>(sub(sub(cx.core.id, "body"), RECTS)).0 = plan.groups.clone();
        self.register(&ctx, rect, rtl, &plan, &sizes);
    }

    fn paint_over(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let plan = self.plan(cx.rect, &cx.style.sizes);
        for pair in plan.open.windows(2) {
            ui.face(DIVIDER.at(pair[0] as u64), Splitter::new(Axis::Y));
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let words = cx.words::<DockWords>();
        let sizes = cx.style.sizes;
        let plan = self.plan(cx.rect, &sizes);
        access.node(BAR, Role::BUTTON).name = Name::Text(bar_name(self.column, self.spec, &words.move_column));
        for (key, _, divider) in plan.dividers(cx.rect, sizes.reach) {
            divider::describe(access, DIVIDER, key, &divider, words.resize.as_str(), sizes.step);
        }
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> DockColumnOutput<P> {
        if let Some(opener) = self.bar.opener {
            Popup::toggle(opener, &mut cx);
        }
        let iconic = self.bar.iconic.or(self.toggled.then_some(!self.column.iconic));
        DockColumnOutput { response: cx.response.clone(), placed: cx.rect, iconic, picked: self.bar.picked, resized: self.resized }
    }
}

/// Registers the drop bands of the two edges of the column in `rect` with the dock `dock`: a drop there makes a new
/// column beside the one that holds `anchor`.
pub(super) fn register_edges<P: Clone + Send + 'static>(ctx: &Context, dock: Id, anchor: &P, rect: Rect, rtl: bool, sizes: &DockSizes) {
    let flow = Flow::new(rect, rtl);
    // A narrow column keeps a third of its width between its two edge bands, so a drop inside it still reaches its
    // groups.
    let band = sizes.column_edge_band.min(rect.width() * 2.0 / 3.0);
    for (side, x) in [(DockSide::Start, rect.left()), (DockSide::End, rect.right())] {
        let edge = Rect::from_x_y_ranges((x - band * 0.5)..=(x + band * 0.5), rect.y_range());
        DockTargets::register(ctx, dock, DockSpot::column(anchor.clone(), side), flow.to_physical(edge));
    }
}

impl<P: Value + Hash + 'static> DockColumn<'_, P> {
    /// Registers the drop bands of the column's two edges and of the seams between its groups.
    fn register(&self, ctx: &Context, rect: Rect, rtl: bool, plan: &Plan, sizes: &DockSizes) {
        let Some(first) = self.anchor(0) else { return };
        register_edges(ctx, self.dock, first, rect, rtl, sizes);
        let (mut before, half) = (first, sizes.seam_band * 0.5);
        for (at, group) in plan.groups.iter().enumerate().skip(1) {
            let Some(after) = self.anchor(at) else { continue };
            let seam = Rect::from_x_y_ranges(rect.x_range(), (group.top() - half)..=(group.top() + half));
            DockTargets::register(ctx, self.dock, DockSpot::seam(before.clone(), after.clone()), seam);
            before = after;
        }
    }
}

impl<P: Value + Hash + 'static> Container for DockColumn<'_, P> {
    fn header(&mut self, ui: &mut Ui, cx: &mut BodyCx<'_>) {
        if cx.shell().is_some_and(|shell| !shell.dock_visible()) {
            return;
        }
        self.bar = show_bar(ui, cx, None, true, self.auto_collapse, true);
    }
}
