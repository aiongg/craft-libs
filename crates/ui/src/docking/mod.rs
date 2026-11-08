//! `SplitArea`, `Splitter` and `DockGroup`.
//!
//! `SplitArea` shows the docked tree of a `Layout` (craft-behavior's dock model): its body shows one `DockGroup`
//! per group at the rects `tree::frames` gives, and its handles resize the splits. A panel dragged from a group's
//! tab carries a `PanelDrag`; `SplitArea` is the drop target that says which group and side it lands on.

mod config;
pub(crate) mod divider;
mod drop;
mod event;
pub mod frame;
mod group;

mod floating;
mod panels;
mod spec;
mod splitter;
pub mod stack;
#[cfg(feature = "testing")]
mod stories;
pub mod targets;
pub mod tree;
mod words;

pub mod desktop;

pub use config::{DockConfig, DockSides};
pub use drop::{DockDrag, DockDropTarget, DockDropped, DragUnit, cancel_on_replacement, float_drop, panel_key, resolve_drop, start_group_drag};
pub use event::{DockEvent, FloatSpace, HidePanels};
pub use group::{DockGroup, DockGroupOutput, DockGroupStyle, GroupGrip, GroupGripOutput, GroupGripStyle, HeaderHook};

pub use floating::{FloatingGroup, FloatingGroupOutput, FloatingGroupStyle};
pub use panels::DockPanels;
pub use spec::{DockHeights, PanelLimits, PanelSpec};
pub use splitter::{Splitter, SplitterStyle};
pub use targets::{DockSizes, DockSpot, DockTargets, GroupDrag, ZoneRule};
pub use words::DockWords;

mod dock_column;
mod flyout;
mod iconic;

pub use dock_column::{
    ColumnGesture, DockColumn, DockColumnOutput, DockColumnStyle, GroupSize, GroupView, fit_widths, group_heights, stacked, width_bounds,
};
pub use flyout::Flyout;
pub use iconic::{IconicColumn, IconicColumnOutput, IconicColumnStyle, IconicSizes};

use crate::contract::{
    Access, Bind, BindMut, BodyCx, Common, Component, Container, ContainerProps, Cx, DropMark, DropOperation, DropTarget, Edit as _, EditResponse,
    Focus as _, Outcome, Pad, PaintUi, PartsCx, Pointer as _, Props as _, Role, Sizing, WordSetCx as _,
};
use crate::runtime::env::Label;
use crate::runtime::focus::KeyUse;
use crate::runtime::memory::sub;
use crate::style::{ComponentStyle, StyleCx};
use craft_behavior::dock::{Layout, ZoneSize, zones};
use craft_behavior::{Axis, DockSide, Flow, Metrics, PartKey, Sense};
use divider::{Divider, Drag};
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

/// The style of a split area: the docking sizes its frames, handles and keys read.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct SplitAreaStyle {
    #[style(metrics)]
    metrics: Metrics,
    /// The docking sizes: the dividers' gap, reach and key steps, and the groups' least extent and strip.
    sizes: DockSizes,
}

/// What a split area reports: the layout's edits and the drop of a panel.
#[derive(Debug)]
#[non_exhaustive]
pub struct SplitAreaOutput<P> {
    /// The resize edits of the layout and the area's response.
    pub edit: EditResponse<Layout<P>>,
    /// A panel dropped on a group this pass.
    pub dropped: Option<DockDrop>,
}

impl<P: Clone + PartialEq + Send + Sync + 'static> Outcome for SplitAreaOutput<P> {
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

/// The groups of the layout with the frames the area gave them this pass, for the body.
#[derive(Clone)]
struct PassFrames<P>(Vec<(P, Rect)>);

impl<P> Default for PassFrames<P> {
    fn default() -> Self {
        PassFrames(Vec::new())
    }
}

const FRAMES: &str = "split_frames";

impl BodyCx<'_> {
    /// The groups of the split area this body belongs to, each as its anchor panel and its rect, with this pass's
    /// resize edits applied: a panel placed at its group's rect sits against the handle in the same pass.
    pub fn group_frames<P: Clone + Send + Sync + 'static>(&self) -> Vec<(P, Rect)> {
        self.memory.recall::<PassFrames<P>>(sub(self.id, FRAMES)).0
    }
}

/// The dividers of the layout's docked tree in `rect`, each with the handle that names its split.
fn handles<P>(layout: &Layout<P>, rect: Rect, sizes: &DockSizes, rules: &FrameRules<'_, P>, rtl: bool) -> Vec<SplitHandle> {
    layout.root.as_ref().map_or_else(Vec::new, |root| tree::frames(root, rect, sizes, rules, rtl).handles)
}

/// The divider of `handle` in reading order. With `ranged`, the range it can travel is the tree's; without, it is
/// where it stands, which is all its part and face need.
fn divider_of<P>(
    layout: &Layout<P>,
    handle: &SplitHandle,
    rect: Rect,
    sizes: &DockSizes,
    rules: &FrameRules<'_, P>,
    rtl: bool,
    ranged: bool,
) -> Divider {
    let flow = Flow::new(rect, rtl);
    let (centre, split) = (flow.to_physical(handle.divider).center(), flow.to_physical(handle.split));
    let at = if handle.axis == Axis::X { centre.x } else { centre.y };
    let range = match layout.root.as_ref() {
        Some(root) if ranged => tree::drag_range(root, rect, sizes, rules, &handle.path),
        _ => (0.0, 0.0),
    };
    Divider::new(handle.axis, at, split, (at + range.0, at + range.1), sizes.reach)
}

/// The layout with the boundary of `handle` moved by `delta` points in reading order; `None` when nothing moves.
fn moved<P: Clone>(
    layout: &Layout<P>,
    rect: Rect,
    sizes: &DockSizes,
    rules: &FrameRules<'_, P>,
    handle: &SplitHandle,
    delta: f32,
) -> Option<Layout<P>> {
    let root = layout.root.as_ref()?;
    let edits = tree::drag_sizes(root, rect, sizes, rules, &handle.path, delta);
    if edits.is_empty() {
        return None;
    }
    let mut next = layout.clone();
    tree::apply_sizes(next.root.as_mut()?, &edits);
    Some(next)
}

impl<P: Clone + Eq + Hash + Debug + Send + Sync + 'static> Component for SplitArea<'_, P> {
    const ROLE: Role = Role::GROUP;
    const KEYS: KeyUse = KeyUse::SPLITTER;
    type Style = SplitAreaStyle;
    type Output = SplitAreaOutput<P>;

    fn style(cx: &StyleCx<'_>, _: SplitAreaStyleKey<P>) -> SplitAreaStyle {
        SplitAreaStyle { metrics: Metrics::new(), sizes: DockSizes::style(cx) }
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
        let (sizes, rtl) = (cx.style.sizes, cx.flow.is_rtl());
        for (at, handle) in handles(layout, rect, &sizes, &self.rules, rtl).iter().enumerate() {
            let divider = divider_of(layout, handle, rect, &sizes, &self.rules, rtl, false);
            divider::declare(cx, rect, HANDLE, at as u64, &divider);
        }
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        let Some(layout) = cx.value().cloned() else {
            cx.core.memory.shown::<PassFrames<P>>(sub(sub(cx.core.id, "body"), FRAMES)).0.clear();
            return;
        };
        let (rect, rtl, sizes, rules) = (cx.rect, cx.flow.is_rtl(), cx.style.sizes, self.rules);
        let handles = handles(&layout, rect, &sizes, &rules, rtl);
        for (at, handle) in handles.iter().enumerate() {
            let divider = divider_of(&layout, handle, rect, &sizes, &rules, rtl, false);
            match divider::pointer(cx, HANDLE, at as u64, &divider) {
                Some(Drag::Start) => cx.begin(),
                Some(Drag::To(coord)) => {
                    if let Some(next) = moved(&layout, rect, &sizes, &rules, handle, coord - divider.at) {
                        cx.propose(next);
                    }
                }
                Some(Drag::Release) => cx.commit(),
                None => {}
            }
        }
        let focused = cx.active_part().filter(|part| part.key == HANDLE).and_then(|part| part.index);
        if let Some(index) = focused
            && let Some(handle) = handles.get(index as usize)
        {
            let divider = divider_of(&layout, handle, rect, &sizes, &rules, rtl, true);
            if let Some((_, coord)) = divider::keys(cx, HANDLE, &[(index, divider)], sizes.step, sizes.jump)
                && let Some(next) = moved(&layout, rect, &sizes, &rules, handle, coord - divider.at)
            {
                cx.set(next);
            }
        }
        let groups = cx.value().and_then(|shown| shown.root.as_ref()).map_or_else(Vec::new, |root| {
            tree::frames(root, rect, &sizes, &rules, rtl).groups.iter().filter_map(|group| Some((group.anchor()?.clone(), group.rect))).collect()
        });
        let keyed = groups.iter().map(|(anchor, rect)| (PanelKey::of(anchor), *rect)).collect();
        cx.core.memory.shown::<PassFrames<P>>(sub(sub(cx.core.id, "body"), FRAMES)).0 = groups;
        let target = GroupDrops { groups: keyed, rtl };
        if let Some(drop) = cx.drop_target::<PanelDrag, _>(&target) {
            self.dropped = Some(drop);
        }
    }

    fn paint_over(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        let Some(layout) = cx.value() else { return };
        let (sizes, rtl) = (cx.style.sizes, cx.flow.is_rtl());
        for (at, handle) in handles(layout, cx.rect, &sizes, &self.rules, rtl).iter().enumerate() {
            ui.face(HANDLE.at(at as u64), Splitter::new(handle.axis));
        }
    }

    fn access(&self, access: &mut Access, cx: &Cx<'_, Self>) {
        let Some(layout) = cx.value() else { return };
        let (rect, rtl, sizes) = (cx.rect, cx.flow.is_rtl(), cx.style.sizes);
        let words = cx.words::<DockWords>();
        let name = words.resize.as_str();
        for (at, handle) in handles(layout, rect, &sizes, &self.rules, rtl).iter().enumerate() {
            let divider = divider_of(layout, handle, rect, &sizes, &self.rules, rtl, true);
            divider::describe(access, HANDLE, at as u64, &divider, name, sizes.step);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> SplitAreaOutput<P> {
        SplitAreaOutput { edit: cx.edit_response(), dropped: self.dropped }
    }
}

impl<P: Clone + Eq + Hash + Debug + Send + Sync + 'static> Container for SplitArea<'_, P> {}
