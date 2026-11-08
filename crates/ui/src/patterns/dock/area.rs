//! `DockArea`: the docking pattern. It shows a workspace's docked tree, dock columns, flyouts and floating groups from
//! the parts of `docking`, takes the drops of tab and group drags, and reports what the host applies.

use super::hide::{self, Shown};
use super::style::DockAreaStyle;
use crate::contract::overlay::PlacementChange;
use crate::contract::{
    Bind, BodyCx, Common, Component, Container, ContainerProps, Cx, Entry, NoValue, Outcome, Pad, Pointer as _, Props as _, Role, ShowContainer as _,
    Sizing, WordSetCx as _,
};
use crate::docking::stack::{StackSection, section_heights};
use crate::docking::tree::{self, FrameRules};
use crate::docking::{
    DockColumn, DockConfig, DockDropTarget, DockDropped, DockEvent, DockGroup, DockPanels, DockSizes, DockSpot, DockTargets, DockWords, DragUnit,
    FloatSpace, FloatingGroup, GroupDrag, HidePanels, IconicColumn, PanelDrag, PanelKey, PanelSpec, PanelTitle, SplitArea, cancel_on_replacement,
    fit_widths, frame, start_group_drag,
};
use crate::docking::{DockScene, DockTo, cross_window, desktop, float_drop, move_actions, native};
use crate::layout_components::Stack;
use crate::menu::{GroupItem, MenuWords, PanelMenu, PanelRow};
use crate::runtime::env::{Env, Label, RevealEdge, ShellCx};
use crate::runtime::memory::{Remember as _, sub};
use crate::runtime::windows::WindowHost;
use crate::sections::HeaderAction;
use crate::style::StyleCx;
use craft_behavior::dock::{Action, Column, DockEdge, DockError, Group, Layout, Node, Permissions, Placement, Target, Workspace, node_at};
use craft_behavior::{CancelReason, DockSide, EditEvent, Flow, GestureId, GesturePhase, Metrics, Mixed, Sense};
use egui::{Context, Id, Pos2, Rect, Response, Ui, Vec2};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

#[cfg(feature = "testing")]
mod stories;

/// A layout the dock cannot show: the workspace failed `Workspace::validate`. The area draws nothing for it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct InvalidGeometry {
    /// Why the model refused the workspace.
    pub error: DockError,
}

/// What a dock area reports in a pass. The host applies `actions` to the workspace (its undo step) and handles
/// `events`.
#[derive(Debug)]
#[non_exhaustive]
pub struct DockAreaOutput<P> {
    /// The changes the user made through the area: tab picks, closes, moves, drops, floats and menu rows.
    pub actions: Vec<Action<P>>,
    /// Requests for host state and reports of where things ended up.
    pub events: Vec<DockEvent<P>>,
    /// The panel of the group the user clicked or picked this pass.
    pub focused: Option<P>,
    /// Why nothing was drawn, when the workspace is invalid.
    pub error: Option<InvalidGeometry>,
    /// The size edit of a split divider this pass, as the edit of the whole workspace. A workspace bound by
    /// reference takes it itself; a given one needs the host to apply it.
    pub edit: Option<EditEvent<Workspace<P>>>,
    /// Where commands float a panel, group or column: the area's float bounds and the sizes of what it drew.
    pub float_space: Option<FloatSpace<P>>,
    /// The area's response.
    pub response: Response,
}

impl<P> Outcome for DockAreaOutput<P> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The docking pattern over a workspace.
///
/// - Shape: the docked tree is a `SplitArea`; a dock's columns are `DockColumn`s, or `IconicColumn`s with their
///   flyouts; a group is a `DockGroup`; a floating window is a `FloatingGroup`. The host's `DockPanels` supplies the
///   panels.
/// - Edits: a divider drag writes the workspace's layout through its binding. Every other change is an
///   [`Action`] in the output, for the host to apply to its workspace set.
/// - Hiding: [`hide`](DockArea::hide), the shell's panel visibility and [`maximized`](DockArea::maximized) prune
///   what shows; a pruned panel keeps its place for restore.
/// - Drops: the area is the drop target of tab and group drags. Holding Ctrl or Cmd floats the drop.
/// - State: the open flyout (unless the host binds it with [`flyout`](DockArea::flyout)) and the previous layout
///   live in the area's root container.
#[derive(crate::contract::Pattern, crate::contract::Props)]
#[pattern(probe(DockArea<'static, usize>))]
pub struct DockArea<'a, P: Clone + Eq + Hash + Debug + Send + Sync + 'static> {
    id: Id,
    workspace: Bind<'a, Workspace<P>>,
    panels: &'a mut dyn DockPanels<P>,
    config: DockConfig,
    /// Which panels the host hides.
    #[prop]
    hide: HidePanels<'a, P>,
    /// The panel whose frame shows alone.
    #[prop]
    maximized: Option<P>,
    /// Auto-Collapse Iconic Panels: a flyout closes when the focus leaves it.
    #[prop(flag)]
    auto_collapse: bool,
    /// The workspace is locked: nothing moves, floats or closes.
    #[prop(flag)]
    locked: bool,
    /// The panel open in an iconic column's flyout, owned by the host. Without it the area keeps the value.
    #[prop]
    flyout: Option<Bind<'a, Option<P>>>,
}

impl<'a, P: Clone + Eq + Hash + Debug + Send + Sync + 'static> DockArea<'a, P> {
    /// A dock area `id` over `workspace`, with its panels from `panels` and its behaviour from `config`.
    pub fn new(id: Id, workspace: Bind<'a, Workspace<P>>, panels: &'a mut dyn DockPanels<P>, config: DockConfig) -> Self {
        DockArea { id, workspace, panels, config, hide: HidePanels::None, maximized: None, auto_collapse: false, locked: false, flyout: None }
    }

    /// A dock area over a layout that is only shown: a given workspace without hidden panels.
    pub fn read_only(id: Id, layout: &Layout<P>, panels: &'a mut dyn DockPanels<P>, config: DockConfig) -> Self {
        Self::new(id, Bind::given(Workspace::new(layout.clone())), panels, config)
    }
}

/// What the area keeps in its root container.
struct AreaMemory<P> {
    /// The open flyout when the host binds none.
    open: Option<P>,
    /// The flyout as the last pass left it, for `FlyoutChanged`.
    reported: Option<P>,
    /// The layout of the previous pass, for cancelling a drag when another arrangement replaces it.
    previous: Layout<P>,
    /// Where each column's first panel was placed, for `ColumnPlaced`.
    columns: Vec<(P, Rect)>,
    /// The running move or resize of a floating window.
    moving: Option<Moving<P>>,
}

/// A floating window's move or resize as one workspace edit: its gesture and the workspace when it began.
#[derive(Clone)]
struct Moving<P> {
    id: GestureId,
    before: Workspace<P>,
}

impl<P> Default for AreaMemory<P> {
    fn default() -> Self {
        AreaMemory { open: None, reported: None, previous: Layout::default(), columns: Vec::new(), moving: None }
    }
}

impl<P: Clone> Clone for AreaMemory<P> {
    fn clone(&self) -> Self {
        AreaMemory {
            open: self.open.clone(),
            reported: self.reported.clone(),
            previous: self.previous.clone(),
            columns: self.columns.clone(),
            moving: self.moving.clone(),
        }
    }
}

/// The sizes the root container resolved this pass, for its body.
#[derive(Clone, Copy, Default)]
struct PassSizes(Option<DockSizes>);

const SIZES: &str = "dock_sizes";

/// A drag a part reported and the root container starts, since only a component's context starts a drag.
struct Start<P> {
    unit: DragUnit<P>,
    active: PanelKey,
    ghost: Option<(Vec2, Label)>,
}

type Starts<P> = RefCell<Vec<Start<P>>>;

/// What the root container reports.
struct SurfaceOutput<P> {
    response: Response,
    dropped: Option<DockDropped<P>>,
}

impl<P> Outcome for SurfaceOutput<P> {
    type Value = NoValue;

    fn response(&self) -> &Response {
        &self.response
    }
}

/// The root container of a dock area: it resolves the dock's sizes, takes the drops and starts the drags the parts
/// report.
#[derive(crate::contract::Component)]
#[component(container, probe(Surface<'static, usize>))]
struct Surface<'a, P: Clone + Eq + Hash + Debug + Send + Sync + 'static> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    target: DockDropTarget<'a, P>,
    starts: &'a Starts<P>,
    dropped: Option<DockDropped<P>>,
}

impl<P: Clone + Eq + Hash + Debug + Send + Sync + 'static> Component for Surface<'_, P> {
    const ROLE: Role = Role::GROUP;
    type Style = DockAreaStyle;
    type Output = SurfaceOutput<P>;

    fn style(cx: &StyleCx<'_>, _: SurfaceStyleKey<P>) -> DockAreaStyle {
        DockAreaStyle { metrics: Metrics::new(), sizes: DockSizes::style(cx) }
    }

    fn sense(&self) -> Sense {
        Sense::empty()
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        cx.core.memory.shown::<PassSizes>(sub(sub(cx.core.id, "body"), SIZES)).0 = Some(cx.style.sizes);
        self.dropped = cx.drop_target::<GroupDrag<P>, _>(&self.target).or_else(|| cx.drop_target::<PanelDrag, _>(&self.target));
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> SurfaceOutput<P> {
        for start in self.starts.borrow_mut().drain(..) {
            start_group_drag(&mut cx, start.unit, start.active, start.ghost);
        }
        SurfaceOutput { response: cx.response.clone(), dropped: self.dropped }
    }
}

impl<P: Clone + Eq + Hash + Debug + Send + Sync + 'static> Container for Surface<'_, P> {}

/// What the area collects while it shows its parts.
struct Report<P> {
    actions: Vec<Action<P>>,
    events: Vec<DockEvent<P>>,
    focused: Option<P>,
    flyout: Vec<EditEvent<Option<P>>>,
    placed: Vec<(P, Rect)>,
}

/// What every part of one pass reads.
struct Scene<'s, P> {
    dock: Id,
    sizes: DockSizes,
    config: &'s DockConfig,
    layout: &'s Layout<P>,
    specs: &'s HashMap<P, PanelSpec>,
    permissions: &'s HashMap<P, Permissions>,
    keys: &'s HashMap<PanelKey, P>,
    space: &'s FloatSpace<P>,
    bounds: Rect,
    auto_collapse: bool,
    rtl: bool,
}

impl<P: Clone + Eq + Hash> Scene<'_, P> {
    fn spec(&self, panel: &P) -> PanelSpec {
        self.specs.get(panel).cloned().unwrap_or_else(|| PanelSpec::new(Label::default()))
    }

    fn permissions(&self, panel: &P) -> Permissions {
        self.permissions.get(panel).copied().unwrap_or(Permissions::ALL)
    }

    fn panel(&self, key: PanelKey) -> Option<&P> {
        self.keys.get(&key)
    }

    /// The size a command floats `panel` at.
    fn float_size(&self, panel: &P) -> Vec2 {
        self.space.panel(panel).or(self.spec(panel).float_size).unwrap_or(self.space.default_size).min(self.bounds.size())
    }

    /// The rect a command floats `panel` at.
    fn float_rect(&self, panel: &P) -> Rect {
        self.space.place(self.float_size(panel))
    }
}

/// What the area reports from one pass.
struct Passed<P> {
    actions: Vec<Action<P>>,
    events: Vec<DockEvent<P>>,
    focused: Option<P>,
    edit: Option<EditEvent<Workspace<P>>>,
    space: Option<FloatSpace<P>>,
}

/// How a group is shown: minimized, and whether it is an accordion section (with its open state).
#[derive(Clone, Copy)]
struct Look {
    minimized: bool,
    section: Option<bool>,
}

/// A dock to show: its window edge, its columns and its rect.
struct DockPlan<'p, P> {
    edge: DockEdge,
    columns: &'p [Column<P>],
    rect: Rect,
    flow: Flow,
    open: Option<P>,
}

/// What a column reported.
struct ColumnWork<P> {
    iconic: Option<bool>,
    picked: Option<GroupItem>,
    resized: Vec<(P, f32)>,
}

/// The parts of one pass with the host's panels and what they reported.
struct Pass<'s, P> {
    scene: &'s Scene<'s, P>,
    panels: &'s mut dyn DockPanels<P>,
    report: Report<P>,
    /// The dock whose registry the groups shown now register their drop spots in: the area's, or a floating window's
    /// own, which takes the drops over that window on the window's layer.
    dock: Id,
}

impl<'a, P: Clone + Eq + Hash + Debug + Send + Sync + 'static> DockArea<'a, P> {
    /// Shows the area across the room offered.
    pub fn show(self, ui: &mut Ui) -> DockAreaOutput<P> {
        let DockArea { id, mut workspace, panels, config, hide, maximized, auto_collapse, locked, mut flyout } = self;
        let ctx = ui.ctx().clone();
        let rtl = Env::of(ui).direction.is_rtl();
        let held = match workspace.reborrow().get() {
            Some(Mixed::Same(held)) => held,
            _ => Workspace::default(),
        };
        let (current, error) = match held.validate() {
            Ok(()) => (held, None),
            Err(error) => (Workspace::default(), Some(InvalidGeometry { error })),
        };
        let rights: HashMap<P, Permissions> = current
            .layout
            .panels()
            .into_iter()
            .map(|panel| {
                let rights = panels.permissions(panel);
                (panel.clone(), if locked { rights.locked() } else { rights })
            })
            .collect();
        let policy = |panel: &P| rights.get(panel).copied().unwrap_or(Permissions::ALL);
        let mut target = DockDropTarget::new(&ctx, id, &current.layout, &policy);
        target.rtl = rtl;
        let starts: Starts<P> = RefCell::new(Vec::new());
        let surface = Surface {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::Fill).pad(Pad::None),
            target,
            starts: &starts,
            dropped: None,
        }
        .id(id);
        let mut passed = None;
        let shown = surface.show(ui, |ui, body| {
            let Some(sizes) = body.memory.recall::<PassSizes>(sub(body.id, SIZES)).0 else { return };
            let input = Input { id, ctx: &ctx, current: &current, config: &config, hide, maximized: maximized.as_ref(), auto_collapse, sizes, rtl };
            let result = show_area(ui, body, &input, &rights, &mut *panels, flyout.as_mut());
            if let Some(edit) = result.edit.as_ref() {
                workspace.reborrow().write(body, edit);
            }
            passed = Some(result);
        });
        let Passed { mut actions, events, focused, edit, space } =
            passed.unwrap_or_else(|| Passed { actions: Vec::new(), events: Vec::new(), focused: None, edit: None, space: None });
        if let Some(dropped) = shown.output.dropped {
            cross_window::accept_cross_window(&ctx);
            actions.push(dropped.action);
        }
        DockAreaOutput { actions, events, focused, error, edit, float_space: space, response: shown.output.response }
    }
}

/// What the pattern gives one pass.
struct Input<'i, P> {
    id: Id,
    ctx: &'i Context,
    current: &'i Workspace<P>,
    config: &'i DockConfig,
    hide: HidePanels<'i, P>,
    maximized: Option<&'i P>,
    auto_collapse: bool,
    sizes: DockSizes,
    rtl: bool,
}

fn open_of<P: Clone + Send + Sync + 'static + PartialEq + Debug>(
    flyout: &mut Option<&mut Bind<'_, Option<P>>>,
    body: &mut BodyCx<'_>,
    memory: &AreaMemory<P>,
) -> Option<P> {
    match flyout.as_mut() {
        Some(bind) => match bind.reborrow().read(body) {
            Some(Mixed::Same(open)) => open,
            _ => None,
        },
        None => memory.open.clone(),
    }
}

fn show_area<P: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
    ui: &mut Ui,
    body: &mut BodyCx<'_>,
    input: &Input<'_, P>,
    rights: &HashMap<P, Permissions>,
    panels: &mut dyn DockPanels<P>,
    mut flyout: Option<&mut Bind<'_, Option<P>>>,
) -> Passed<P> {
    let Input { id, ctx, current, config, hide, maximized, auto_collapse, sizes, rtl } = *input;
    let shell = body.shell();
    let mut memory = body.memory::<AreaMemory<P>>().clone();
    cancel_on_replacement(ctx, &memory.previous, &current.layout);
    let specs: HashMap<P, PanelSpec> = current.layout.panels().into_iter().map(|panel| (panel.clone(), panels.spec(panel))).collect();
    let keys: HashMap<PanelKey, P> = current.layout.panels().into_iter().map(|panel| (PanelKey::of(panel), panel.clone())).collect();
    let protected = |panel: &P| rights.get(panel).is_some_and(|rights| rights.is_protected());
    let Shown { view, floating } = hide::shown(&current.layout, maximized, hide, shell.as_ref(), &protected);
    let layout: &Layout<P> = &view.layout;
    let bounds = shell.as_ref().map_or(body.rect, ShellCx::float_bounds);
    let mut space = FloatSpace::new(bounds, &sizes).with_side_edge_zones(config.side_edge_zones);
    for (panel, spec) in &specs {
        if let Some(size) = spec.float_size {
            space = space.with_panel_size(panel.clone(), size);
        }
    }
    let scene = Scene { dock: id, sizes, config, layout, specs: &specs, permissions: rights, keys: &keys, space: &space, bounds, auto_collapse, rtl };
    let open = open_of(&mut flyout, body, &memory);
    let mut pass = Pass {
        scene: &scene,
        panels,
        report: Report { actions: Vec::new(), events: Vec::new(), focused: None, flyout: Vec::new(), placed: Vec::new() },
        dock: id,
    };

    let spec_of = |panel: &P| scene.spec(panel);
    let limits = |panel: &P| scene.spec(panel).limits;
    let dock_width = |columns: &[Column<P>]| scene.sizes.natural(columns, &spec_of).iter().map(|(width, _)| *width).sum::<f32>();
    let stripless = |panel: &P| scene.spec(panel).canvas;
    let rules = FrameRules::new(&limits).dock_width(&dock_width).stripless(&stripless);
    let label = Label::from(body.words::<DockWords>().panels.as_str().to_owned());
    let area = SplitArea::new(label, Bind::given(layout.clone()), rules).show(ui, |ui, split| {
        for (anchor, rect) in split.group_frames::<P>() {
            let Some(node) = scene.layout.place(&anchor).and_then(|place| scene.layout.node_of(&place)) else { continue };
            pass.leaf(ui, split, node, rect);
        }
        let flow = Flow::new(split.rect, rtl);
        for (edge, rect) in split.dock_frames() {
            let Some(path) = scene.layout.dock_path(edge) else { continue };
            let Some(Node::Dock { columns, .. }) = scene.layout.root.as_ref().and_then(|root| node_at(root, &path)) else { continue };
            if let Some(shell) = shell.as_ref().filter(|shell| shell.revealed()) {
                shell.reveal_rect(if edge == DockEdge::Start { RevealEdge::Start } else { RevealEdge::End }, rect);
            }
            pass.dock(ui, split, DockPlan { edge, columns, rect, flow, open: open.clone() });
        }
    });
    let edit = area.output.edit.event.map(|event| {
        event.map(|edited| {
            let mut next = current.clone();
            view.write_back(&edited, &mut next.layout);
            next
        })
    });
    register_bands(&scene, ctx, body.rect);
    let moved = if floating { pass.windows(ui, body) } else { None };
    let mut report = pass.report;
    let label: Arc<str> = Arc::from(body.words::<DockWords>().move_window.as_str());
    let edit = edit.or_else(|| moved.and_then(|(anchor, change)| window_edit(ctx, current, rights, &mut memory.moving, &anchor, &change, label)));

    for event in report.flyout.drain(..) {
        match flyout.as_mut() {
            Some(bind) => {
                bind.reborrow().write(body, &event);
            }
            None => {
                if let Some(value) = event.value() {
                    memory.open = value.clone();
                }
            }
        }
    }
    let now = open_of(&mut flyout, body, &memory);
    if memory.reported != now {
        report.events.push(DockEvent::FlyoutChanged { panel: now.clone() });
        memory.reported = now;
    }
    for (anchor, rect) in &report.placed {
        if !memory.columns.iter().any(|(held, at)| held == anchor && at == rect) {
            report.events.push(DockEvent::ColumnPlaced { anchor: anchor.clone(), rect: *rect });
        }
    }
    memory.columns = std::mem::take(&mut report.placed);
    memory.previous = current.layout.clone();
    *body.memory::<AreaMemory<P>>() = memory;
    Passed { actions: report.actions, events: report.events, focused: report.focused, edit, space: Some(space) }
}

/// Registers the outer-edge bands and the new-dock bands as the dock's drop spots for the next pass.
fn register_bands<P: Clone + Eq + Hash + Send + 'static>(scene: &Scene<'_, P>, ctx: &Context, area: Rect) {
    let Some(root) = scene.layout.root.as_ref() else { return };
    let limits = |panel: &P| scene.spec(panel).limits;
    let rules = FrameRules::new(&limits);
    let frames = tree::frames(root, area, &scene.sizes, &rules, scene.rtl);
    let canvas = |panel: &P| scene.spec(panel).canvas;
    let bands = frame::edge_bands(scene.layout, &frames, &canvas, scene.config, &scene.sizes, scene.rtl);
    frame::register_edge_bands::<P>(ctx, scene.dock, &bands);
    if scene.config.new_dock_zones {
        let flow = Flow::new(area, scene.rtl);
        let band = scene.sizes.new_dock_band.min(area.width() / 4.0);
        let start = Rect::from_min_size(area.min, Vec2::new(band, area.height()));
        let end = Rect::from_min_max(Pos2::new(area.right() - band, area.top()), area.max);
        for (side, rect) in [(DockSide::Start, start), (DockSide::End, end)] {
            DockTargets::<P>::register(ctx, scene.dock, DockSpot::NewDock(side), flow.to_physical(rect));
        }
    }
}

/// A region of `ui` at `rect` that holds one container.
fn region<R>(ui: &mut Ui, rect: Rect, mut content: impl FnMut(&mut Ui) -> R) -> Option<R> {
    Stack::new().fill().pad(Pad::None).show_at(ui, rect, |ui, _| content(ui)).inner
}

impl<P: Clone + Eq + Hash + Debug + Send + Sync + 'static> Pass<'_, P> {
    /// One group of the docked tree or of a floating window: a tab group, or the sections of an accordion.
    fn leaf(&mut self, ui: &mut Ui, body: &mut BodyCx<'_>, node: &Node<P>, rect: Rect) {
        let scene = self.scene;
        match node {
            Node::Tabs { panels: members, active, .. } => self.group(ui, body, members, *active, Look { minimized: false, section: None }, rect),
            Node::Stack { entries, .. } => {
                let sections: Vec<StackSection> = entries
                    .iter()
                    .map(|entry| {
                        let limits = scene.spec(&entry.panel).limits;
                        StackSection::new(entry.open).height(entry.height).limits(limits.min.y, limits.max.map_or(f32::INFINITY, |max| max.y))
                    })
                    .collect();
                let heights = section_heights(&sections, rect.height(), scene.sizes.strip);
                let mut top = rect.top();
                for (entry, height) in entries.iter().zip(heights) {
                    let section = Rect::from_min_size(Pos2::new(rect.left(), top), Vec2::new(rect.width(), height));
                    top += height;
                    let look = Look { minimized: !entry.open, section: Some(entry.open) };
                    self.group(ui, body, std::slice::from_ref(&entry.panel), 0, look, section);
                }
            }
            _ => {}
        }
    }

    /// The rows of the menu of `panel`'s group: the host's rows, then the dock's.
    fn menu_rows(&mut self, body: &BodyCx<'_>, panel: &P) -> Vec<PanelRow> {
        let scene = self.scene;
        let words = body.words::<MenuWords>();
        let rights = scene.permissions(panel);
        let targets: Vec<Label> =
            scene.layout.move_targets(panel, |other| scene.permissions(other)).into_iter().map(|target| scene.spec(target).label).collect();
        let mut group = PanelMenu::group_rows(rights.movable, rights.floatable, rights.closable, scene.config.close_menu_items, &targets, &words);
        let item = |item: GroupItem, label: &str| PanelRow::Item(Entry::new(item, Label::from(label.to_owned())));
        if rights.closable && scene.config.close_menu_items {
            group.push(item(GroupItem::CloseGroup, words.panel_close_group.as_str()));
        }
        if rights.movable && rights.floatable {
            group.push(item(GroupItem::FloatGroup, words.panel_float_group.as_str()));
        }
        if scene.config.close_others_item {
            group.push(item(GroupItem::CloseOthers, words.panel_close_others.as_str()));
        }
        if scene.config.maximize_item {
            group.push(item(GroupItem::ToggleMaximize, words.panel_maximize.as_str()));
        }
        let mut rows = self.panels.panel_rows(panel);
        if !rows.is_empty() && !group.is_empty() {
            rows.push(PanelRow::Separator);
        }
        rows.extend(group);
        rows
    }

    /// The action or event a dock row of the menu of `panel` makes.
    fn choose(&mut self, panel: &P, item: GroupItem) {
        let scene = self.scene;
        let action = match item {
            GroupItem::Float => Some(Action::float(panel.clone(), scene.float_rect(panel))),
            GroupItem::FloatGroup => Some(Action::move_group(panel.clone(), Target::float(scene.float_rect(panel)))),
            GroupItem::Close => Some(Action::close(panel.clone())),
            GroupItem::CloseGroup => Some(Action::close_group(panel.clone())),
            GroupItem::CloseOthers => Some(Action::close_others(panel.clone())),
            GroupItem::CollapseToIcons => Some(Action::set_iconic(panel.clone(), true)),
            GroupItem::StepGroup(up) => Some(Action::step_group(panel.clone(), up)),
            GroupItem::MoveTo(at) => scene
                .layout
                .move_targets(panel, |other| scene.permissions(other))
                .get(at)
                .map(|target| Action::move_beside(panel.clone(), (*target).clone(), Placement::tab(None))),
            _ => None,
        };
        let report = &mut self.report;
        match (action, item) {
            (Some(action), _) => report.actions.push(action),
            (None, GroupItem::AutoCollapse) => report.events.push(DockEvent::ToggleAutoCollapse),
            (None, GroupItem::ToggleMaximize) => report.events.push(DockEvent::ToggleMaximize { panel: panel.clone() }),
            (None, command) => report.events.push(DockEvent::PanelCommand { panel: panel.clone(), command }),
        }
    }

    /// One group in `rect`: the host's `group_view` when it takes the group, else a `DockGroup` over the host's
    /// panels.
    fn group(&mut self, ui: &mut Ui, body: &mut BodyCx<'_>, members: &[P], active: usize, look: Look, rect: Rect) {
        let scene = self.scene;
        let Some(anchor) = members.first() else { return };
        let active = active.min(members.len() - 1);
        if self.panels.group_view(members, active, ui, rect, body) {
            return;
        }
        let titles: Vec<PanelTitle> = members.iter().map(|panel| PanelTitle::new(PanelKey::of(panel), scene.spec(panel).label)).collect();
        let shown = &members[active];
        let menu = self.menu_rows(body, shown);
        let mut group = DockGroup::new(anchor.clone(), &titles, PanelKey::of(shown))
            .dock(self.dock)
            .zones(scene.config.zone_rule())
            .movable(scene.permissions(shown).movable)
            .menu(menu);
        if look.minimized {
            group = group.minimized();
        }
        if scene.config.tab_close_buttons {
            group = group.closable();
        }
        let panels = &mut *self.panels;
        let output = group.show_at(ui, rect, |ui, body| panels.body(shown, ui, body)).output;
        if let Some(panel) = output.picked.and_then(|key| scene.panel(key)) {
            self.report.actions.push(Action::activate(panel.clone()));
            self.report.focused = Some(panel.clone());
        }
        for panel in output.closed.into_iter().filter_map(|key| scene.panel(key)) {
            self.report.actions.push(Action::close(panel.clone()));
        }
        if let Some(item) = output.item {
            self.choose(shown, item);
        }
        if output.toggle_minimized {
            self.report.actions.push(match look.section {
                Some(open) => Action::set_stack_open(shown.clone(), !open),
                None => Action::set_minimized(shown.clone(), !look.minimized),
            });
        }
        if let Some(torn) = output.torn_off
            && let Some(panel) = scene.panel(torn.grabbed)
        {
            self.report.actions.push(float_drop(&DragUnit::Panel(panel.clone()), scene.space, torn.pointer).action);
        }
    }

    fn group_of(&mut self, ui: &mut Ui, body: &mut BodyCx<'_>, group: &Group<P>, rect: Rect) {
        let look = Look { minimized: group.minimized, section: None };
        self.group(ui, body, &group.panels, group.active, look, rect);
    }

    /// The columns of a dock, from the window edge inwards.
    fn dock(&mut self, ui: &mut Ui, body: &mut BodyCx<'_>, plan: DockPlan<'_, P>) {
        let scene = self.scene;
        let spec = |panel: &P| scene.spec(panel);
        let widths = fit_widths(&scene.sizes.natural(plan.columns, &spec), plan.rect.width());
        let reading = plan.flow.to_physical(plan.rect);
        let start = plan.edge == DockEdge::Start;
        let mut cursor = if start { reading.left() } else { reading.right() };
        let name = Label::from(body.words::<DockWords>().panels.as_str().to_owned());
        for (column, width) in plan.columns.iter().zip(widths) {
            let span = if start { (cursor, cursor + width) } else { (cursor - width, cursor) };
            cursor = if start { span.1 } else { span.0 };
            let cell = plan.flow.to_physical(Rect::from_x_y_ranges(span.0..=span.1, reading.y_range()));
            let Some(first) = column.groups.first().and_then(|group| group.panels.first()) else { continue };
            self.report.placed.push((first.clone(), cell));
            if column.iconic {
                let open = plan.open.clone();
                let Some(output) = region(ui, cell, |ui| {
                    IconicColumn::new(name.clone(), scene.dock, column, Bind::given(open.clone()), &spec)
                        .edge(plan.edge)
                        .auto_collapse(scene.auto_collapse)
                        .show_flyouts(ui, |group, rect, ui, body| self.group_of(ui, body, group, rect))
                }) else {
                    continue;
                };
                if let Some(event) = output.edit.event {
                    self.report.flyout.push(event);
                }
                let work = ColumnWork { iconic: output.iconic, picked: output.picked, resized: Vec::new() };
                self.column_work(column, work);
            } else {
                let Some(output) = region(ui, cell, |ui| {
                    DockColumn::new(name.clone(), scene.dock, column, &spec)
                        .auto_collapse(scene.auto_collapse)
                        .show_groups(ui, |group, rect, ui, body| self.group_of(ui, body, group, rect))
                }) else {
                    continue;
                };
                let work = ColumnWork { iconic: output.iconic, picked: output.picked, resized: output.resized };
                self.column_work(column, work);
            }
        }
    }

    /// Turns what a column reported into actions and drags.
    fn column_work(&mut self, column: &Column<P>, work: ColumnWork<P>) {
        let Some(first) = column.groups.first().and_then(|group| group.panels.first()) else { return };
        if let Some(iconic) = work.iconic {
            self.report.actions.push(Action::set_iconic(first.clone(), iconic));
        }
        if let Some(item) = work.picked {
            self.choose(first, item);
        }
        for (anchor, height) in work.resized {
            self.report.actions.push(Action::resize_group(anchor, Some(height)));
        }
    }

    /// The floating windows of the pruned layout, each a `FloatingGroup` holding its tree's groups, in an OS window
    /// where the config, the platform and its panels allow one. A window takes the drops over it on its own layer,
    /// from the spots its groups register in its own registry. Returns the move or resize of a window in the app window
    /// this pass, which the area reports as a workspace edit.
    fn windows(&mut self, ui: &mut Ui, body: &BodyCx<'_>) -> Option<(P, PlacementChange)> {
        let scene = self.scene;
        let ctx = ui.ctx().clone();
        let policy = |panel: &P| scene.permissions(panel);
        let mut moved = None;
        for window in &scene.layout.floating {
            let Some(anchor) = window.panels().next().cloned() else { continue };
            let active = match &window.root {
                Node::Tabs { panels, active, .. } => panels.get(*active).cloned().unwrap_or_else(|| anchor.clone()),
                _ => anchor.clone(),
            };
            let specs: Vec<PanelSpec> = window.panels().map(|panel| scene.spec(panel)).collect();
            let rights: Vec<Permissions> = window.panels().map(|panel| scene.permissions(panel)).collect();
            let opening = native::opening(&ctx, scene.config, window, &specs, scene.space.title_row);
            let mut actions = vec![HeaderAction::Menu];
            if rights.iter().all(|rights| rights.movable) {
                actions.push(HeaderAction::Float);
            }
            if scene.config.close_button && rights.iter().all(|rights| rights.closable) {
                actions.push(HeaderAction::Close);
            }
            let rows = self.menu_rows(body, &active);
            let picked = Cell::new(None);
            let dropped = RefCell::new(None);
            let dock = scene.dock.with(("floating", window.id));
            let mut target = DockDropTarget::new(&ctx, dock, scene.layout, &policy);
            target.rtl = scene.rtl;
            let spec = scene.spec(&active);
            let mut group = FloatingGroup::new(spec.label, opening.rect)
                .limits(scene.spec(&anchor).limits)
                .sized(Sizing::Fill)
                .minimized(window.minimized)
                .actions(actions)
                .window_of(anchor.clone())
                .take_drops(|cx| {
                    let taken = cx.drop_target::<GroupDrag<P>, _>(&target).or_else(|| cx.drop_target::<PanelDrag, _>(&target));
                    if taken.is_some() {
                        *dropped.borrow_mut() = taken;
                    }
                })
                .id_salt(("floating", window.id));
            group = match opening.spec.clone() {
                Some(window_spec) => group.native(window_spec),
                None => group.bounds(scene.bounds),
            };
            if !rows.is_empty() {
                let picked = &picked;
                group = group.menu(move |ui, _| {
                    if let Some(item) = PanelMenu::new(Vec::new(), rows.clone()).show(ui).picked {
                        picked.set(Some(item));
                    }
                });
            }
            let area_dock = std::mem::replace(&mut self.dock, dock);
            let shown = group.show_group(ui, |ui, body| {
                let limits = |panel: &P| scene.spec(panel).limits;
                let rules = FrameRules::new(&limits);
                let frames = tree::frames(&window.root, body.rect, &scene.sizes, &rules, scene.rtl);
                for frame in &frames.groups {
                    self.leaf(ui, body, frame.node, frame.rect);
                }
            });
            self.dock = area_dock;
            let output = shown.output;
            if let Some(dropped) = dropped.into_inner() {
                cross_window::accept_cross_window(&ctx);
                self.report.actions.push(dropped.action);
            }
            if opening.spec.is_some() {
                let host = WindowHost::of(&ctx);
                if let Some(facts) = host.viewport_of(output.response.id).and_then(|viewport| host.facts(viewport)) {
                    let monitors = desktop::monitors(&ctx);
                    self.report.actions.extend(native::reported(window, facts.outer.unwrap_or(facts.inner), &monitors));
                }
            } else if let Some(change) = output.placed {
                if change.phase == GesturePhase::Commit {
                    self.report.events.push(DockEvent::FloatPlaced { panel: anchor.clone(), rect: change.rect });
                }
                moved = Some((anchor.clone(), change));
            }
            match output.action {
                Some(HeaderAction::Close) => self.report.actions.push(Action::close_window(anchor.clone())),
                Some(HeaderAction::Float) => {
                    let commands = DockScene::new(scene.layout, scene.space, false, &policy);
                    if let Ok(docked) = move_actions(&commands, &DragUnit::Window(anchor.clone()), DockTo::Docked) {
                        self.report.actions.extend(docked);
                    }
                }
                _ => {}
            }
            if let Some(item) = picked.get() {
                self.choose(&active, item);
            }
            if output.minimized_toggled {
                self.report.actions.push(Action::set_minimized(anchor.clone(), !window.minimized));
            }
        }
        moved
    }
}

/// The workspace edit of a floating window's move or resize: the gesture runs from the window's `Begin` to its
/// `Commit`, so one drag is one undo step. `moving` holds the gesture and the workspace it began on; `rights` are the
/// panels' permissions with the lock applied, which refuse the move of a locked workspace.
fn window_edit<P: Clone + Eq + Hash>(
    ctx: &Context,
    current: &Workspace<P>,
    rights: &HashMap<P, Permissions>,
    moving: &mut Option<Moving<P>>,
    anchor: &P,
    change: &PlacementChange,
    label: Arc<str>,
) -> Option<EditEvent<Workspace<P>>> {
    let moved = || {
        let mut next = current.clone();
        let policy = |panel: &P| rights.get(panel).copied().unwrap_or(Permissions::ALL);
        next.apply(Action::move_floating(anchor.clone(), change.rect), false, policy).ok().map(|_| next)
    };
    let begin = |moving: &mut Option<Moving<P>>| {
        let id = crate::runtime::edit::next_gesture_id(ctx);
        *moving = Some(Moving { id, before: current.clone() });
        EditEvent::begin(id, Mixed::Same(current.clone()), Arc::clone(&label), None)
    };
    match change.phase {
        GesturePhase::Begin => Some(begin(moving)),
        GesturePhase::Update => match moving.as_ref() {
            Some(held) => Some(EditEvent::update(held.id, moved()?)),
            None => Some(begin(moving)),
        },
        GesturePhase::Commit => match moving.take() {
            Some(held) => Some(EditEvent::commit(held.id, Mixed::Same(held.before), moved()?)),
            None => Some(EditEvent::set(crate::runtime::edit::next_gesture_id(ctx), Mixed::Same(current.clone()), moved()?, label, None)),
        },
        _ => moving.take().map(|held| EditEvent::cancel(held.id, CancelReason::Dismissed, Some(Mixed::Same(held.before)))),
    }
}
