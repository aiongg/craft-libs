//! `SplitArea`, `Splitter` and `DockGroup`.
//!
//! `SplitArea` shows a `DockTree` (craft-behavior's pure geometry): its body shows one `DockGroup` per group at
//! the rects `group_rects` gives, and its handles resize the splits. A panel dragged from a group's tab carries a
//! `PanelDrag`; `SplitArea` is the drop target that says which group and side it lands on.

mod config;
mod event;
mod panels;
mod spec;
mod splitter;
#[cfg(feature = "testing")]
mod stories;
mod tab;
pub mod targets;
mod words;

pub use config::{DockConfig, DockSides};
pub use event::{DockEvent, FloatSpace, HidePanels};
pub use panels::DockPanels;
pub use spec::{DockHeights, PanelLimits, PanelSpec};
pub use splitter::{Splitter, SplitterStyle};
pub use targets::{DockSizes, DockSpot, DockTargets, GroupDrag, ZoneRule};
pub use words::DockWords;

use crate::contract::{
    Access, Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, DropMark, DropOperation, DropTarget, Edit as _, EditResponse,
    Focus as _, HeaderRegion, Name, Numeric, Outcome, Pad, PaintUi, PartsCx, Pointer as _, Props as _, Role, Show as _, ShowContainer as _, Sizing,
    WordSetCx as _,
};
use crate::layout_components::Row;
use crate::runtime::env::Label;
use crate::runtime::focus::KeyUse;
use crate::style::{ComponentStyle, StyleCx, SurfaceKind};
use craft_behavior::nav::{StepResult, ValueAxes, ValueSteps, step_value};
use craft_behavior::{
    Axis, CursorIcon, DockHandle, DockSide, DockTree, Flow, GroupId, HitLevel, HitPriority, Metrics, NavMove, PartKey, PressPhase, Sense,
};
use egui::{Pos2, Rect, Response, Ui};
use tab::PanelTab;

const HANDLE: PartKey = PartKey::new("handle");

/// How far from each edge of a group a drop docks on that side, as a share of the group's width or height.
const EDGE: f32 = 0.25;

/// The stable key of a panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PanelKey(pub u64);

impl PanelKey {
    /// The panel keyed `key`.
    pub const fn new(key: u64) -> Self {
        PanelKey(key)
    }
}

/// A panel as its group's strip shows it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PanelTitle {
    /// The panel.
    pub key: PanelKey,
    /// The tab's text.
    pub title: Label,
}

impl PanelTitle {
    /// A panel `key` titled `title`.
    pub fn new(key: PanelKey, title: impl Into<Label>) -> Self {
        PanelTitle { key, title: title.into() }
    }
}

/// What a dragged tab carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PanelDrag {
    /// The dragged panel.
    pub panel: PanelKey,
    /// The group the tab was in.
    pub from: GroupId,
}

impl PanelDrag {
    pub(crate) const fn new(panel: PanelKey, from: GroupId) -> Self {
        PanelDrag { panel, from }
    }
}

/// Where a dropped panel lands: the group the pointer is over and the side of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DockDrop {
    /// The group the pointer is over.
    pub group: GroupId,
    /// The side of the group: an edge docks beside it, the middle joins its tabs.
    pub side: DockSide,
}

/// Where each group of `tree` sits in `rect`, mirrored under right-to-left.
pub fn group_rects(tree: &DockTree, rect: Rect, rtl: bool) -> Vec<(GroupId, Rect)> {
    let flow = Flow::new(rect, rtl);
    tree.layout(rect).into_iter().map(|(group, cell)| (group, flow.to_physical(cell))).collect()
}

/// The accepting side of a `SplitArea`: groups take a panel on any side, by move only.
struct GroupDrops {
    groups: Vec<(GroupId, Rect)>,
    rtl: bool,
}

impl GroupDrops {
    /// The side of `rect` that `at` selects, read along the reading direction.
    fn side(&self, rect: Rect, at: Pos2) -> DockSide {
        let logical = if self.rtl { Pos2::new(rect.right() - (at.x - rect.left()), at.y) } else { at };
        DockSide::pick(rect, logical, EDGE)
    }
}

impl DropTarget<PanelDrag> for GroupDrops {
    type Drop = DockDrop;

    fn accept(&self, payload: &PanelDrag, at: Pos2, op: DropOperation) -> Option<DockDrop> {
        if !matches!(op, DropOperation::Move) {
            return None;
        }
        let (group, rect) = self.groups.iter().find(|(_, rect)| rect.contains(at))?;
        let side = self.side(*rect, at);
        (payload.from != *group || side != DockSide::Center).then_some(DockDrop { group: *group, side })
    }

    fn mark(&self, drop: &DockDrop) -> DropMark {
        let rect = self.groups.iter().find(|(group, _)| *group == drop.group).map_or(Rect::NOTHING, |(_, rect)| *rect);
        match drop.side {
            DockSide::Center => DropMark::Into(rect),
            side => DropMark::Split(Flow::new(rect, self.rtl).to_physical(side.area(rect))),
        }
    }
}

/// The style of a split area: the sizes its handles and keys read.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SplitAreaStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// How thick a handle is across its boundary.
    reach: f32,
    /// The least extent of a group along a split.
    least: f32,
    /// One arrow press.
    step: f32,
    /// One Shift-arrow or Page press.
    jump: f32,
}

/// What a split area reports: the tree's edits and the drop of a panel.
#[derive(Debug)]
#[non_exhaustive]
pub struct DockOutput {
    /// The resize edits of the tree and the area's response.
    pub edit: EditResponse<DockTree>,
    /// A panel dropped on a group this pass.
    pub dropped: Option<DockDrop>,
}

impl Outcome for DockOutput {
    type Value = DockTree;

    fn response(&self) -> &Response {
        &self.edit.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.edit.story_facts()
    }
}

/// A tree of groups with a handle between neighbours. Its body shows the groups, each at its rect.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct SplitArea<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    tree: Bind<'a, DockTree>,
    dropped: Option<DockDrop>,
}

impl<'a> SplitArea<'a> {
    /// A split area named `label` over `tree`. A mixed tree shows no handles.
    pub fn new(label: impl Into<Label>, tree: Bind<'a, DockTree>) -> Self {
        let props = ContainerProps::new().arena(true).sizing(Sizing::Fill).pad(Pad::None);
        SplitArea { common: Common::new(), props, tree, dropped: None }.label(label)
    }
}

/// `x` as a coordinate from the start edge of `rect`: unchanged for left-to-right, mirrored for right-to-left.
fn logical_x(rect: Rect, rtl: bool, x: f32) -> f32 {
    if rtl { rect.right() - (x - rect.left()) } else { x }
}

/// A vertical move read as the move of a value that grows downwards.
fn along_y(mv: NavMove) -> NavMove {
    match mv {
        NavMove::Up => NavMove::Down,
        NavMove::Down => NavMove::Up,
        NavMove::PageUp => NavMove::PageDown,
        NavMove::PageDown => NavMove::PageUp,
        NavMove::Forward => NavMove::Up,
        NavMove::Back => NavMove::Down,
        other => other,
    }
}

/// Dragging a handle moves its boundary: one gesture from the drag's start to its release.
fn resize_by_pointer(cx: &mut Cx<'_, SplitArea<'_>>, tree: &DockTree, handles: &[DockHandle]) {
    let (rect, rtl, least) = (cx.rect, cx.flow.is_rtl(), cx.style.least);
    for handle in handles {
        let phase = cx.part(HANDLE.at(u64::from(handle.key.0))).press.phase;
        match phase {
            PressPhase::DragStart { .. } => cx.begin(),
            PressPhase::Drag { pos, .. } => {
                let along = if handle.axis == Axis::X { logical_x(rect, rtl, pos.x) } else { pos.y };
                let mut next = tree.clone();
                if next.move_boundary(&handle.path, handle.index, handle.split, along, least) {
                    cx.propose(next);
                }
            }
            PressPhase::Release { .. } => cx.commit(),
            _ => {}
        }
    }
}

/// A handle's boundary position along its axis and the span it can move in: the split's extent less the least
/// size of each side.
fn handle_span(handle: &DockHandle, least: f32) -> (f32, f32, f32) {
    let (at, lo, hi) = match handle.axis {
        Axis::X => (handle.bar.center().x, handle.split.left(), handle.split.right()),
        _ => (handle.bar.center().y, handle.split.top(), handle.split.bottom()),
    };
    (at, lo + least, (hi - least).max(lo + least))
}

/// The arrow keys move the focused handle's boundary by a step, Shift or Page by a jump; AccessKit sets it to a value.
fn resize_by_keys(cx: &mut Cx<'_, SplitArea<'_>>, tree: &DockTree, handles: &[DockHandle]) {
    let style = cx.style;
    let focused = cx.active_part().filter(|part| part.key == HANDLE).and_then(|part| part.index);
    let Some(key) = focused else { return };
    let Some(handle) = handles.iter().find(|handle| u64::from(handle.key.0) == key) else { return };
    let (at, lo, hi) = handle_span(handle, style.least);
    let value = if let Some(set) = cx.intents().take_set_value() {
        set.clamp(f64::from(lo), f64::from(hi))
    } else if let Some((mv, mods)) = cx.intents().take_move() {
        let (axes, mv) = match handle.axis {
            Axis::X => (ValueAxes::Horizontal, mv),
            _ => (ValueAxes::Vertical, along_y(mv)),
        };
        let steps = ValueSteps::new(f64::from(at), f64::from(lo), f64::from(hi), f64::from(style.step), f64::from(style.jump));
        match step_value(&steps, axes, mv, mods) {
            StepResult::Set(value) => value,
            _ => return,
        }
    } else {
        return;
    };
    let mut next = tree.clone();
    if next.move_boundary(&handle.path, handle.index, handle.split, value as f32, style.least) {
        cx.set(next);
    }
}

impl Component for SplitArea<'_> {
    const ROLE: Role = Role::GROUP;
    const KEYS: KeyUse = KeyUse::CANVAS;
    type Style = SplitAreaStyle;
    type Output = DockOutput;

    fn style(cx: &StyleCx<'_>, _: SplitAreaStyleKey) -> SplitAreaStyle {
        let control = cx.control();
        SplitAreaStyle {
            metrics: Metrics::new(),
            reach: cx.tokens.size.hit.target,
            least: cx.tokens.size.panel_header,
            step: control.gap,
            jump: control.height,
        }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn bind(&mut self) -> Option<BindMut<'_, DockTree>> {
        Some(self.tree.reborrow())
    }

    /// The handles: edge parts above the groups, each its own Tab stop that the arrow keys move.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let Some(tree) = cx.value() else { return };
        let flow = Flow::new(rect, cx.flow.is_rtl());
        for handle in tree.handles(rect, cx.style.reach) {
            let cursor = if handle.axis == Axis::X { CursorIcon::ResizeHorizontal } else { CursorIcon::ResizeVertical };
            cx.part(HANDLE.at(u64::from(handle.key.0)), flow.to_physical(handle.bar))
                .sense(Sense::DRAG | Sense::TAKE_FOCUS)
                .priority(HitPriority::new(HitLevel::Edge, 0))
                .cursor(cursor)
                .focusable()
                .over();
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(tree) = cx.value().cloned() else { return };
        let handles = tree.handles(cx.rect, cx.style.reach);
        resize_by_pointer(cx, &tree, &handles);
        resize_by_keys(cx, &tree, &handles);
        let (rect, rtl) = (cx.rect, cx.flow.is_rtl());
        let target = GroupDrops { groups: group_rects(&tree, rect, rtl), rtl };
        if let Some(drop) = cx.drop_target::<PanelDrag, _>(&target) {
            self.dropped = Some(drop);
        }
    }

    fn paint_over(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(tree) = cx.value() else { return };
        for handle in tree.handles(cx.rect, cx.style.reach) {
            ui.face(HANDLE.at(u64::from(handle.key.0)), Splitter::new(handle.axis));
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let Some(tree) = cx.value() else { return };
        let name = cx.words::<DockWords>().resize.as_str().to_owned();
        for handle in tree.handles(cx.rect, cx.style.reach) {
            let (at, lo, hi) = handle_span(&handle, cx.style.least);
            let node = access.node(HANDLE.at(u64::from(handle.key.0)), Role::SPLITTER);
            node.name = Name::Text(name.clone());
            node.numeric = Some(Numeric::new(f64::from(at), f64::from(lo)..=f64::from(hi)).step(f64::from(cx.style.step)));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> DockOutput {
        DockOutput { edit: cx.edit_response(), dropped: self.dropped }
    }
}

impl Container for SplitArea<'_> {}

/// What a dock group reports: its response and the tab the user picked.
#[derive(Debug)]
#[non_exhaustive]
pub struct DockGroupOutput {
    /// The group's response.
    pub response: Response,
    /// The panel whose tab was clicked this pass; the app makes it the group's active panel.
    pub picked: Option<PanelKey>,
}

impl Outcome for DockGroupOutput {
    type Value = crate::contract::NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The style of a dock group: no look of its own.
#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct DockGroupStyle {
    #[style(metrics)]
    metrics: Metrics,
}

/// A group of panels: a strip of tabs over the active panel's body. Show it with `show_at(group_rect)` in the
/// body of a `SplitArea`, with an `IdSource::Id` made from the group's id so its state survives a re-dock.
#[derive(crate::contract::Component)]
#[component(container)]
pub struct DockGroup<'a> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    group: GroupId,
    panels: &'a [PanelTitle],
    active: PanelKey,
    picked: Option<PanelKey>,
}

impl<'a> DockGroup<'a> {
    /// The group `group` with its `panels`, `active` showing. The group is named by the active panel's title.
    pub fn new(group: GroupId, panels: &'a [PanelTitle], active: PanelKey) -> Self {
        let props = ContainerProps::new().surface(SurfaceKind::Panel).sizing(Sizing::Fill).header(HeaderRegion::new().rule(true));
        let title = panels.iter().find(|panel| panel.key == active).map(|panel| panel.title.clone()).unwrap_or_default();
        DockGroup { common: Common::new(), props, group, panels, active, picked: None }.label(title)
    }
}

impl Component for DockGroup<'_> {
    const ROLE: Role = Role::TAB_PANEL;
    const PARTS: bool = false;
    type Style = DockGroupStyle;
    type Output = DockGroupOutput;

    fn style(_: &StyleCx<'_>, _: DockGroupStyleKey) -> DockGroupStyle {
        DockGroupStyle::default()
    }

    fn finish(self, cx: Cx<'_, Self>) -> DockGroupOutput {
        DockGroupOutput { response: cx.response.clone(), picked: self.picked }
    }
}

impl Container for DockGroup<'_> {
    // F8-MERGE: tabs (a `TabStrip` of the panels replaces this row)
    fn header(&mut self, ui: &mut Ui, _: &mut BodyCx<'_>) {
        let (group, active, panels) = (self.group, self.active, self.panels);
        let picked = &mut self.picked;
        Row::new().show(ui, |ui, _| {
            for panel in panels {
                if PanelTab::new(group, panel.key, panel.title.clone(), panel.key == active).id_salt(panel.key.0).show(ui).chosen {
                    *picked = Some(panel.key);
                }
            }
        });
    }
}
