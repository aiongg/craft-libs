//! `SplitArea`, `Splitter` and `DockGroup`.
//!
//! `SplitArea` shows the docked tree of a `Layout` (craft-behavior's dock model): its body shows one `DockGroup`
//! per group at the rects `tree::frames` gives, and its handles resize the splits. A panel dragged from a group's
//! tab carries a `PanelDrag`; `SplitArea` is the drop target that says which group and side it lands on.

mod config;
mod event;
pub mod frame;
mod group;
mod panels;
mod spec;
mod splitter;
pub mod stack;
#[cfg(feature = "testing")]
mod stories;
pub mod targets;
pub mod tree;
mod words;

pub use config::{DockConfig, DockSides};
pub use event::{DockEvent, FloatSpace, HidePanels};
pub use group::{DockGroup, DockGroupOutput, DockGroupStyle, GroupGrip, GroupGripOutput, GroupGripStyle, HeaderHook};
pub use panels::DockPanels;
pub use spec::{DockHeights, PanelLimits, PanelSpec};
pub use splitter::{Splitter, SplitterStyle};
pub use targets::{DockSizes, DockSpot, DockTargets, GroupDrag, ZoneRule};
pub use words::DockWords;

use crate::contract::{
    Access, Bind, BindMut, Common, Component, Container, ContainerProps, Cx, DropMark, DropOperation, DropTarget, Edit as _, EditResponse,
    Focus as _, Name, Numeric, Outcome, Pad, PaintUi, PartsCx, Pointer as _, Props as _, Role, Sizing, WordSetCx as _,
};
use crate::runtime::env::Label;
use crate::runtime::focus::KeyUse;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::dock::{Layout, ZoneSize, zones};
use craft_behavior::nav::{StepResult, ValueAxes, ValueSteps, step_value};
use craft_behavior::{Axis, CursorIcon, DockSide, Flow, HitLevel, HitPriority, Metrics, NavMove, PartKey, PressPhase, Sense};
use egui::{Pos2, Rect, Response};
use std::fmt::Debug;
use std::hash::{DefaultHasher, Hash, Hasher};
use tree::{FrameRules, SplitHandle};

const HANDLE: PartKey = PartKey::new("handle");

/// The stable key of a panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct PanelKey(pub u64);

impl PanelKey {
    /// The panel keyed `key`.
    pub const fn new(key: u64) -> Self {
        PanelKey(key)
    }

    /// The key of the panel `panel`, from its hash. Equal panels have equal keys.
    pub fn of<P: Hash + ?Sized>(panel: &P) -> Self {
        let mut hasher = DefaultHasher::new();
        panel.hash(&mut hasher);
        PanelKey(hasher.finish())
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
    /// The group the tab was in, named by its anchor panel.
    pub from: PanelKey,
}

/// Where a dropped panel lands: the group the pointer is over and the side of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DockDrop {
    /// The group the pointer is over, named by its anchor panel.
    pub group: PanelKey,
    /// The side of the group: an edge docks beside it, the middle joins its tabs.
    pub side: DockSide,
}

/// The accepting side of a `SplitArea`: groups take a panel on any side, by move only.
struct GroupDrops {
    groups: Vec<(PanelKey, Rect)>,
    rtl: bool,
}

impl GroupDrops {
    /// The side of `rect` that `at` selects, by the dock's zone function.
    fn side(&self, rect: Rect, at: Pos2) -> DockSide {
        zones(rect, ZoneSize::Fraction, self.rtl).into_iter().find(|(_, zone)| zone.contains(at)).map_or(DockSide::Center, |(side, _)| side)
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
    /// One arrow press.
    step: f32,
    /// One Shift-arrow or Page press.
    jump: f32,
}

/// What a split area reports: the layout's edits and the drop of a panel.
#[derive(Debug)]
#[non_exhaustive]
pub struct DockOutput<P> {
    /// The resize edits of the layout and the area's response.
    pub edit: EditResponse<Layout<P>>,
    /// A panel dropped on a group this pass.
    pub dropped: Option<DockDrop>,
}

impl<P: Clone + PartialEq + Send + Sync + 'static> Outcome for DockOutput<P> {
    type Value = Layout<P>;

    fn response(&self) -> &Response {
        &self.edit.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        self.edit.story_facts()
    }
}

/// The docked tree of a layout, with a handle between neighbouring groups. Its body shows the groups, each at its
/// frame (`tree::frames` with the same rules).
#[derive(crate::contract::Component)]
#[component(container, probe(SplitArea<'static, usize>))]
pub struct SplitArea<'a, P: Clone + Eq + Hash + Debug + Send + Sync + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    layout: Bind<'a, Layout<P>>,
    rules: FrameRules<'a, P>,
    dropped: Option<DockDrop>,
}

impl<'a, P: Clone + Eq + Hash + Debug + Send + Sync + 'static> SplitArea<'a, P> {
    /// A split area named `label` over `layout`, sized by `rules`. A mixed layout shows no handles.
    pub fn new(label: impl Into<Label>, layout: Bind<'a, Layout<P>>, rules: FrameRules<'a, P>) -> Self {
        let props = ContainerProps::new().arena(true).sizing(Sizing::Fill).pad(Pad::None);
        SplitArea { common: Common::new(), props, layout, rules, dropped: None }.label(label)
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

/// The dividers of the layout's docked tree in `rect`.
fn dividers<P>(layout: &Layout<P>, rect: Rect, rules: &FrameRules<'_, P>, rtl: bool) -> Vec<SplitHandle> {
    layout.root.as_ref().map_or_else(Vec::new, |root| tree::frames(root, rect, rules, rtl).handles)
}

/// The handle's strip across its divider, as thick as `reach`.
fn bar(handle: &SplitHandle, reach: f32) -> Rect {
    let (centre, split) = (handle.divider.center(), handle.split);
    match handle.axis {
        Axis::X => Rect::from_min_max(Pos2::new(centre.x - reach / 2.0, split.top()), Pos2::new(centre.x + reach / 2.0, split.bottom())),
        _ => Rect::from_min_max(Pos2::new(split.left(), centre.y - reach / 2.0), Pos2::new(split.right(), centre.y + reach / 2.0)),
    }
}

/// The boundary's coordinate along its axis, in reading order, and the least and most it can take.
fn boundary<P>(layout: &Layout<P>, handle: &SplitHandle, rect: Rect, rules: &FrameRules<'_, P>, rtl: bool) -> (f32, f32, f32) {
    let centre = handle.divider.center();
    let at = match handle.axis {
        Axis::X => logical_x(rect, rtl, centre.x),
        _ => centre.y,
    };
    let (lo, hi) = layout.root.as_ref().map_or((0.0, 0.0), |root| tree::drag_range(root, rect, rules, &handle.path));
    (at, at + lo, at + hi)
}

/// The layout with the boundary of `handle` moved by `delta` points in reading order; `None` when nothing moves.
fn moved<P: Clone>(layout: &Layout<P>, rect: Rect, rules: &FrameRules<'_, P>, handle: &SplitHandle, delta: f32) -> Option<Layout<P>> {
    let root = layout.root.as_ref()?;
    let sizes = tree::drag_sizes(root, rect, rules, &handle.path, delta);
    if sizes.is_empty() {
        return None;
    }
    let mut next = layout.clone();
    tree::apply_sizes(next.root.as_mut()?, &sizes);
    Some(next)
}

/// Dragging a handle moves its boundary: one gesture from the drag's start to its release.
fn resize_by_pointer<P: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
    cx: &mut Cx<'_, SplitArea<'_, P>>,
    layout: &Layout<P>,
    handles: &[SplitHandle],
    rules: &FrameRules<'_, P>,
) {
    let (rect, rtl) = (cx.rect, cx.flow.is_rtl());
    for (at, handle) in handles.iter().enumerate() {
        let phase = cx.part(HANDLE.at(at as u64)).press.phase;
        match phase {
            PressPhase::DragStart { .. } => cx.begin(),
            PressPhase::Drag { pos, .. } => {
                let centre = handle.divider.center();
                let delta = match handle.axis {
                    Axis::X => logical_x(rect, rtl, pos.x) - logical_x(rect, rtl, centre.x),
                    _ => pos.y - centre.y,
                };
                if let Some(next) = moved(layout, rect, rules, handle, delta) {
                    cx.propose(next);
                }
            }
            PressPhase::Release { .. } => cx.commit(),
            _ => {}
        }
    }
}

/// The arrow keys move the focused handle's boundary by a step, Shift or Page by a jump; AccessKit sets it to a value.
fn resize_by_keys<P: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
    cx: &mut Cx<'_, SplitArea<'_, P>>,
    layout: &Layout<P>,
    handles: &[SplitHandle],
    rules: &FrameRules<'_, P>,
) {
    let (rect, rtl) = (cx.rect, cx.flow.is_rtl());
    let (step, jump) = (cx.style.step, cx.style.jump);
    let focused = cx.active_part().filter(|part| part.key == HANDLE).and_then(|part| part.index);
    let Some(handle) = focused.and_then(|key| handles.get(key as usize)) else { return };
    let (at, lo, hi) = boundary(layout, handle, rect, rules, rtl);
    let value = if let Some(set) = cx.intents().take_set_value() {
        set.clamp(f64::from(lo), f64::from(hi))
    } else if let Some((mv, mods)) = cx.intents().take_move() {
        let (axes, mv) = match handle.axis {
            Axis::X => (ValueAxes::Horizontal, mv),
            _ => (ValueAxes::Vertical, along_y(mv)),
        };
        let steps = ValueSteps::new(f64::from(at), f64::from(lo), f64::from(hi), f64::from(step), f64::from(jump));
        match step_value(&steps, axes, mv, mods) {
            StepResult::Set(value) => value,
            _ => return,
        }
    } else {
        return;
    };
    if let Some(next) = moved(layout, rect, rules, handle, value as f32 - at) {
        cx.set(next);
    }
}

impl<P: Clone + Eq + Hash + Debug + Send + Sync + 'static> Component for SplitArea<'_, P> {
    const ROLE: Role = Role::GROUP;
    const KEYS: KeyUse = KeyUse::CANVAS;
    type Style = SplitAreaStyle;
    type Output = DockOutput<P>;

    fn style(cx: &StyleCx<'_>, _: SplitAreaStyleKey<P>) -> SplitAreaStyle {
        let control = cx.control();
        SplitAreaStyle { metrics: Metrics::new(), reach: cx.tokens.size.hit.target, step: control.gap, jump: control.height }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn bind(&mut self) -> Option<BindMut<'_, Layout<P>>> {
        Some(self.layout.reborrow())
    }

    /// The handles: edge parts above the groups, each its own Tab stop that the arrow keys move.
    fn parts(&mut self, rect: Rect, cx: &mut PartsCx<'_, Self>) {
        let Some(layout) = cx.value() else { return };
        for (at, handle) in dividers(layout, rect, &self.rules, cx.flow.is_rtl()).iter().enumerate() {
            let cursor = if handle.axis == Axis::X { CursorIcon::ResizeHorizontal } else { CursorIcon::ResizeVertical };
            cx.part(HANDLE.at(at as u64), bar(handle, cx.style.reach))
                .sense(Sense::DRAG | Sense::TAKE_FOCUS)
                .priority(HitPriority::new(HitLevel::Edge, 0))
                .cursor(cursor)
                .focusable()
                .over();
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(layout) = cx.value().cloned() else { return };
        let (rect, rtl) = (cx.rect, cx.flow.is_rtl());
        let rules = self.rules;
        let handles = dividers(&layout, rect, &rules, rtl);
        resize_by_pointer(cx, &layout, &handles, &rules);
        resize_by_keys(cx, &layout, &handles, &rules);
        let groups = layout.root.as_ref().map_or_else(Vec::new, |root| {
            tree::frames(root, rect, &rules, rtl).groups.iter().filter_map(|group| Some((PanelKey::of(group.anchor()?), group.rect))).collect()
        });
        let target = GroupDrops { groups, rtl };
        if let Some(drop) = cx.drop_target::<PanelDrag, _>(&target) {
            self.dropped = Some(drop);
        }
    }

    fn paint_over(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(layout) = cx.value() else { return };
        for (at, handle) in dividers(layout, cx.rect, &self.rules, cx.flow.is_rtl()).iter().enumerate() {
            ui.face(HANDLE.at(at as u64), Splitter::new(handle.axis));
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let Some(layout) = cx.value() else { return };
        let (rect, rtl) = (cx.rect, cx.flow.is_rtl());
        let name = cx.words::<DockWords>().resize.as_str().to_owned();
        for (at, handle) in dividers(layout, rect, &self.rules, rtl).iter().enumerate() {
            let (value, lo, hi) = boundary(layout, handle, rect, &self.rules, rtl);
            let node = access.node(HANDLE.at(at as u64), Role::SPLITTER);
            node.name = Name::Text(name.clone());
            node.numeric = Some(Numeric::new(f64::from(value), f64::from(lo)..=f64::from(hi)).step(f64::from(cx.style.step)));
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> DockOutput<P> {
        DockOutput { edit: cx.edit_response(), dropped: self.dropped }
    }
}

impl<P: Clone + Eq + Hash + Debug + Send + Sync + 'static> Container for SplitArea<'_, P> {}
