//! Docking commands: the panel, arrange and workspace command sets, and the pure functions that turn a command into
//! the dock model's `Action`s or the host's next step.
//!
//! The commands carry no payload. A panel command acts on the shown panel of the dock group it reaches: `DockArea`
//! gives each group the `dockGroup` scope of [`DockCommands`], publishes [`dock_states`] there and runs [`actions`]
//! for the commands it takes, so a menu row, a key and the command palette run the same code. Nothing here reads
//! `Ui` or egui memory; a host that shows its own dock calls the same functions and applies the result with
//! `Workspace::apply_all`.

use super::drop::{beside, members, moving_panels, to_target};
use super::{DockEvent, DragUnit, FloatSpace};
use crate::runtime::commands::CommandState;
use craft_behavior::DockSide;
use craft_behavior::dock::{
    Action, Arrangement, DockEdge, DockError, DockPlacement, DocumentWindows, Layout, Node, PanelPlace, Permissions, Target, Workspace, WorkspaceSet,
    arrange_rects, node_at,
};
use craft_behavior::placement::cascade_rects;
use egui::{Rect, Vec2};
use std::hash::Hash;

/// The commands of a docked or floating panel. A panel command acts on one panel or on its tab group: the shown
/// panel of the dock group whose scope instance takes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, craft_ui_macros::CommandSet)]
#[command(prefix = "dock", scope = "dockGroup")]
#[non_exhaustive]
pub enum DockCommands {
    /// Float the panel into its own frame.
    #[command(label = "Float")]
    FloatPanel,
    /// Float the panel's tab group into its own frame.
    #[command(label = "Float Tab Group")]
    FloatGroup,
    /// Ask for a group to move the panel to; the host lists [`Layout::move_targets`] and calls [`move_actions`].
    #[command(label = "Move Panel To…")]
    MovePanelTo,
    /// Ask for a group to move the panel's tab group to.
    #[command(label = "Move Group To…")]
    MoveGroupTo,
    /// Close the panel.
    #[command(label = "Close")]
    ClosePanel,
    /// Close the other panels of the panel's group.
    #[command(label = "Close Other Panels in Group")]
    CloseOthers,
    /// Close the panel's tab group.
    #[command(label = "Close Tab Group")]
    CloseGroup,
    /// Move the panel's group up its column.
    #[command(label = "Move Group Up")]
    StepGroupUp,
    /// Move the panel's group down its column.
    #[command(label = "Move Group Down")]
    StepGroupDown,
    /// Minimize the panel's group to its tab strip, or expand it.
    #[command(label = "Minimize", kind = "toggle")]
    ToggleMinimized,
    /// Collapse the panel's column to icons, or expand it.
    #[command(label = "Collapse to Icons", kind = "toggle")]
    CollapseToIcons,
    /// Maximize the panel's frame, or restore it.
    #[command(label = "Maximize Frame", kind = "toggle")]
    ToggleMaximize,
    /// Collapse iconic panels when focus leaves them, or stop doing so.
    #[command(label = "Auto-Collapse Iconic Panels", kind = "toggle")]
    AutoCollapse,
}

impl DockCommands {
    /// The unit a command that asks for a destination moves, `None` for the other commands.
    pub fn moves<P>(self, panel: &P) -> Option<DragUnit<P>>
    where
        P: Clone,
    {
        match self {
            DockCommands::MovePanelTo => Some(DragUnit::Panel(panel.clone())),
            DockCommands::MoveGroupTo => Some(DragUnit::Group(panel.clone())),
            _ => None,
        }
    }

    /// The event a command that only the host can carry out reports: Maximize Frame and Auto-Collapse.
    pub fn event<P>(self, panel: &P) -> Option<DockEvent<P>>
    where
        P: Clone,
    {
        match self {
            DockCommands::ToggleMaximize => Some(DockEvent::ToggleMaximize { panel: panel.clone() }),
            DockCommands::AutoCollapse => Some(DockEvent::ToggleAutoCollapse),
            _ => None,
        }
    }
}

/// The layout a command reads, with what it needs to decide what a drag would also decide: the float space, whether
/// the workspace is locked and what each panel allows.
pub struct DockScene<'a, P> {
    layout: &'a Layout<P>,
    space: &'a FloatSpace<P>,
    locked: bool,
    permissions: &'a dyn Fn(&P) -> Permissions,
}

impl<'a, P> DockScene<'a, P> {
    /// A scene over `layout`. `locked` is `WorkspaceSet::locked`; `permissions` is what the host passes to
    /// `Workspace::apply_all`.
    pub fn new(layout: &'a Layout<P>, space: &'a FloatSpace<P>, locked: bool, permissions: &'a dyn Fn(&P) -> Permissions) -> Self {
        DockScene { layout, space, locked, permissions }
    }

    fn policy(&self, panel: &P) -> Permissions {
        let permissions = (self.permissions)(panel);
        if self.locked { permissions.locked() } else { permissions }
    }
}

/// Where a moved unit goes.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DockTo<P> {
    /// Into the group of `anchor` as tabs, before `before`, or at the end.
    #[non_exhaustive]
    Tabs {
        /// A panel of the destination group.
        anchor: P,
        /// The tab to go before.
        before: Option<P>,
    },
    /// Beside the group of `anchor`; [`DockSide::Center`] joins its tabs.
    #[non_exhaustive]
    Beside {
        /// A panel of the destination group.
        anchor: P,
        /// The side of the group.
        side: DockSide,
    },
    /// At the outer edge of the docked area on this side.
    Edge(DockSide),
    /// As a new last column of the dock on this window edge, or in a new dock there.
    Dock(DockEdge),
    /// Back to the docked layout: a tab of the first docked panel outside the unit. Nothing for a docked unit.
    Docked,
}

impl<P> DockTo<P> {
    /// Into the group of `anchor` as tabs, before `before` or at the end.
    pub fn tabs(anchor: P, before: Option<P>) -> Self {
        DockTo::Tabs { anchor, before }
    }

    /// Beside the group of `anchor`, on `side`.
    pub fn beside(anchor: P, side: DockSide) -> Self {
        DockTo::Beside { anchor, side }
    }
}

/// The actions that run a panel command, empty for a command the host carries out ([`DockCommands::event`],
/// [`DockCommands::moves`]).
///
/// # Errors
///
/// A [`DockError`] when the panel is not in the layout or the command does not apply to where it is.
pub fn actions<P: Clone + Eq + Hash>(scene: &DockScene<'_, P>, command: DockCommands, panel: &P) -> Result<Vec<Action<P>>, DockError> {
    let layout = scene.layout;
    let shown = || if layout.contains(panel) { Ok(()) } else { Err(DockError::MissingPanel) };
    let one = |action: Action<P>| Ok(vec![action]);
    match command {
        DockCommands::FloatPanel => {
            shown()?;
            Ok(float_actions(scene, &DragUnit::Panel(panel.clone())))
        }
        DockCommands::FloatGroup => {
            shown()?;
            Ok(float_actions(scene, &DragUnit::Group(panel.clone())))
        }
        DockCommands::ClosePanel => one(Action::close(panel.clone())),
        DockCommands::CloseOthers => one(Action::close_others(panel.clone())),
        DockCommands::CloseGroup => one(Action::close_group(panel.clone())),
        DockCommands::StepGroupUp => one(Action::step_group(panel.clone(), true)),
        DockCommands::StepGroupDown => one(Action::step_group(panel.clone(), false)),
        DockCommands::ToggleMinimized => {
            let place = layout.place(panel).ok_or(DockError::MissingPanel)?;
            // An accordion entry is minimized while it is closed.
            let current = match &place {
                PanelPlace::Node(_) => match layout.node_of(&place) {
                    Some(Node::Stack { entries, .. }) => entries.iter().any(|entry| &entry.panel == panel && !entry.open),
                    _ => false,
                },
                PanelPlace::Floating { window, .. } => layout.floating.get(*window).is_some_and(|window| window.minimized),
                _ => layout.group(&place).is_some_and(|group| group.minimized),
            };
            one(Action::set_minimized(panel.clone(), !current))
        }
        DockCommands::CollapseToIcons => {
            let place = layout.place(panel).ok_or(DockError::MissingPanel)?;
            let column = layout.column(&place).ok_or(DockError::InvalidPlacement)?;
            one(Action::set_iconic(panel.clone(), !column.iconic))
        }
        DockCommands::MovePanelTo | DockCommands::MoveGroupTo | DockCommands::ToggleMaximize | DockCommands::AutoCollapse => Ok(Vec::new()),
    }
}

/// The actions that move `unit` to `to`.
///
/// # Errors
///
/// A [`DockError`] when the unit's panel is not in the layout, or the destination has no anchor outside the unit.
pub fn move_actions<P: Clone + Eq + Hash>(scene: &DockScene<'_, P>, unit: &DragUnit<P>, to: DockTo<P>) -> Result<Vec<Action<P>>, DockError> {
    let layout = scene.layout;
    let panel = unit.lead();
    if !layout.contains(panel) {
        return Err(DockError::MissingPanel);
    }
    let moved = moving_panels(layout, unit);
    let one = |action: Action<P>| Ok(vec![action]);
    match to {
        DockTo::Tabs { anchor, before } => one(beside(unit, anchor, DockPlacement::tab(before))),
        DockTo::Beside { anchor, side } => {
            let placement = DockPlacement::for_zone(layout, &anchor, side)?;
            one(beside(unit, anchor, placement))
        }
        DockTo::Edge(side) => {
            if side == DockSide::Center {
                return Err(DockError::InvalidPlacement);
            }
            let protected = |p: &P| scene.policy(p).is_protected();
            let skip = |p: &P| moved.contains(p);
            let anchor = layout.edge_anchor(layout.edge_narrows(side, &protected), &protected, &skip).ok_or(DockError::MissingTarget)?;
            one(beside(unit, anchor, DockPlacement::Edge(side)))
        }
        DockTo::Dock(edge) => match last_column(layout, edge) {
            Some(column) => match column.into_iter().find(|p| !moved.contains(p)) {
                Some(anchor) => one(beside(unit, anchor, DockPlacement::column(true))),
                // What moves already is that dock's last column.
                None => Ok(Vec::new()),
            },
            None => {
                let alone = matches!(unit, DragUnit::Panel(_)) && members(layout, panel).len() == 1;
                let unit = if alone { DragUnit::Group(panel.clone()) } else { unit.clone() };
                one(to_target(&unit, Target::new_dock(edge)))
            }
        },
        DockTo::Docked => {
            if !matches!(layout.place(panel), Some(PanelPlace::Floating { .. })) {
                return Ok(Vec::new());
            }
            let panels = layout.panels();
            let free = || panels.iter().copied().filter(|p| !moved.contains(p));
            let docked = free().find(|p| matches!(layout.place(p), Some(PanelPlace::Dock { .. })));
            let framed = || free().find(|p| matches!(layout.place(p), Some(PanelPlace::Node(_))));
            let anchor = docked.or_else(framed).cloned().ok_or(DockError::MissingTarget)?;
            one(beside(unit, anchor, DockPlacement::tab(None)))
        }
    }
}

/// The state of each panel command for `panel`: enabled as its permissions and place allow, Collapse to Icons
/// checked while its column is iconic. Maximize Frame applies to a frame of the docked layout, never to a floating
/// one. `DockArea` publishes them in the panel's group scope.
pub fn dock_states<P: Clone + Eq + Hash>(scene: &DockScene<'_, P>, panel: &P) -> Vec<(DockCommands, CommandState)> {
    let layout = scene.layout;
    let policy = |p: &P| scene.policy(p);
    let allowed = scene.policy(panel);
    let place = layout.place(panel);
    let floatable = place.is_some() && allowed.movable && allowed.floatable;
    let iconic = place.as_ref().and_then(|place| layout.column(place)).map(|column| column.iconic);
    let enabled = |on: bool| CommandState::new().enabled(on);
    let shown = place.is_some();
    let floating = matches!(place, Some(PanelPlace::Floating { .. }));
    let movable = allowed.movable && !layout.move_targets(panel, policy).is_empty();
    vec![
        (DockCommands::FloatPanel, enabled(floatable)),
        (DockCommands::FloatGroup, enabled(floatable)),
        (DockCommands::MovePanelTo, enabled(movable)),
        (DockCommands::MoveGroupTo, enabled(movable)),
        (DockCommands::ClosePanel, enabled(shown && allowed.closable)),
        (DockCommands::CloseOthers, enabled(allowed.closable && members(layout, panel).len() > 1)),
        (DockCommands::CloseGroup, enabled(shown && allowed.closable)),
        (DockCommands::StepGroupUp, enabled(layout.can_step_group(panel, true, policy))),
        (DockCommands::StepGroupDown, enabled(layout.can_step_group(panel, false, policy))),
        (DockCommands::ToggleMinimized, enabled(shown)),
        (DockCommands::CollapseToIcons, CommandState::new().enabled(iconic.is_some()).checked(iconic.unwrap_or(false))),
        (DockCommands::ToggleMaximize, enabled(shown && !floating)),
        (DockCommands::AutoCollapse, enabled(true)),
    ]
}

/// The actions that float `unit` at the rect a drag would float it at, or keep its window where it is.
fn float_actions<P: Clone + Eq + Hash>(scene: &DockScene<'_, P>, unit: &DragUnit<P>) -> Vec<Action<P>> {
    let layout = scene.layout;
    let panel = unit.lead();
    let window = layout.floating.iter().find(|window| window.contains(panel));
    let lone = window.is_some_and(|window| window.panels().count() == 1);
    let whole = window.is_some_and(|window| matches!(window.root, Node::Tabs { .. } | Node::Stack { .. }));
    let kept = window.map(|window| window.rect);
    let action = match (unit, kept) {
        (DragUnit::Panel(_), Some(rect)) if lone => Action::move_floating(panel.clone(), rect),
        (DragUnit::Group(_), Some(rect)) if whole => to_target(unit, Target::float(rect)),
        (DragUnit::Panel(_), _) => Action::float(panel.clone(), placed(scene, unit)),
        _ => to_target(unit, Target::float(placed(scene, unit))),
    };
    vec![action]
}

/// The rect a drag would float `unit` at, inside the float area.
fn placed<P: Clone + Eq + Hash>(scene: &DockScene<'_, P>, unit: &DragUnit<P>) -> Rect {
    let (layout, space) = (scene.layout, scene.space);
    let panel = unit.lead();
    let docked = layout.place(panel).and_then(|place| layout.column(&place)).map(|column| column.width.unwrap_or(scene.sizes.column));
    let remembered = match unit {
        DragUnit::Panel(_) => space.panel(panel),
        DragUnit::Group(_) | DragUnit::Window(_) => space.group(panel),
        DragUnit::Column(_) => space.column(panel),
    };
    let width = docked.filter(|_| !matches!(unit, DragUnit::Panel(_))).unwrap_or(space.default_size.x);
    let size = remembered.unwrap_or_else(|| Vec2::new(width, space.default_size.y));
    space.place(size)
}

/// The panels of the last column of the dock on `edge`.
fn last_column<P: Clone + Eq + Hash>(layout: &Layout<P>, edge: DockEdge) -> Option<Vec<P>> {
    let path = layout.dock_path(edge)?;
    match node_at(layout.root.as_ref()?, &path)? {
        Node::Dock { columns, .. } => Some(columns.last()?.groups.iter().flat_map(|group| group.panels.iter().cloned()).collect()),
        _ => None,
    }
}

/// The Window › Arrange commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, craft_ui_macros::CommandSet)]
#[command(prefix = "arrange")]
#[non_exhaustive]
pub enum ArrangeCommands {
    /// Tile every document window across the document area, side by side.
    #[command(label = "Tile All Vertically")]
    TileAllVertically,
    /// Tile every document window across the document area, stacked.
    #[command(label = "Tile All Horizontally")]
    TileAllHorizontally,
    /// Two document groups, side by side.
    #[command(label = "2-up Vertical")]
    TwoUpVertical,
    /// Two document groups, stacked.
    #[command(label = "2-up Horizontal")]
    TwoUpHorizontal,
    /// Three document groups in a row.
    #[command(label = "3-up Vertical")]
    ThreeUpVertical,
    /// Three document groups in a column.
    #[command(label = "3-up Horizontal")]
    ThreeUpHorizontal,
    /// One large document group beside two stacked ones.
    #[command(label = "3-up Stacked")]
    ThreeUpStacked,
    /// Four document groups in a 2 by 2 grid.
    #[command(label = "4-up")]
    FourUp,
    /// Six document groups in a 3 by 2 grid.
    #[command(label = "6-up")]
    SixUp,
    /// Every document back into one tab group.
    #[command(label = "Consolidate All to Tabs")]
    ConsolidateAllToTabs,
    /// Floating windows stepped from the top left.
    #[command(label = "Cascade")]
    Cascade,
    /// Floating windows in an even grid.
    #[command(label = "Tile")]
    Tile,
    /// The active document into a floating window.
    #[command(label = "Float in Window")]
    FloatInWindow,
    /// Every document into its own floating window.
    #[command(label = "Float All in Windows")]
    FloatAllInWindows,
    /// A second view of the active document.
    #[command(label = "New Window for Document")]
    NewWindowForDocument,
    /// Set every document view to the active view's zoom.
    #[command(label = "Match Zoom")]
    MatchZoom,
    /// Scroll every document view to the active view's location.
    #[command(label = "Match Location")]
    MatchLocation,
    /// Set every document view to the active view's rotation.
    #[command(label = "Match Rotation")]
    MatchRotation,
    /// Match zoom, location and rotation together.
    #[command(label = "Match All")]
    MatchAll,
}

impl ArrangeCommands {
    /// The data entry this command runs.
    pub const fn arrangement(self) -> Arrangement {
        match self {
            ArrangeCommands::TileAllVertically => Arrangement::TileAllVertically,
            ArrangeCommands::TileAllHorizontally => Arrangement::TileAllHorizontally,
            ArrangeCommands::TwoUpVertical => Arrangement::TwoUpVertical,
            ArrangeCommands::TwoUpHorizontal => Arrangement::TwoUpHorizontal,
            ArrangeCommands::ThreeUpVertical => Arrangement::ThreeUpVertical,
            ArrangeCommands::ThreeUpHorizontal => Arrangement::ThreeUpHorizontal,
            ArrangeCommands::ThreeUpStacked => Arrangement::ThreeUpStacked,
            ArrangeCommands::FourUp => Arrangement::FourUp,
            ArrangeCommands::SixUp => Arrangement::SixUp,
            ArrangeCommands::ConsolidateAllToTabs => Arrangement::ConsolidateAllToTabs,
            ArrangeCommands::Cascade => Arrangement::Cascade,
            ArrangeCommands::Tile => Arrangement::Tile,
            ArrangeCommands::FloatInWindow => Arrangement::FloatInWindow,
            ArrangeCommands::FloatAllInWindows => Arrangement::FloatAllInWindows,
            ArrangeCommands::NewWindowForDocument => Arrangement::NewWindowForDocument,
            ArrangeCommands::MatchZoom => Arrangement::MatchZoom,
            ArrangeCommands::MatchLocation => Arrangement::MatchLocation,
            ArrangeCommands::MatchRotation => Arrangement::MatchRotation,
            ArrangeCommands::MatchAll => Arrangement::MatchAll,
        }
    }
}

/// The state of each Arrange command with `documents` open, in `groups` docked groups and `floating` floating
/// windows.
pub fn arrange_states(documents: usize, groups: usize, floating: usize) -> Vec<(ArrangeCommands, CommandState)> {
    ALL_ARRANGE
        .into_iter()
        .map(|command| (command, CommandState::new().enabled(command.arrangement().enabled(documents, groups, floating))))
        .collect()
}

const ALL_ARRANGE: [ArrangeCommands; 19] = [
    ArrangeCommands::TileAllVertically,
    ArrangeCommands::TileAllHorizontally,
    ArrangeCommands::TwoUpVertical,
    ArrangeCommands::TwoUpHorizontal,
    ArrangeCommands::ThreeUpVertical,
    ArrangeCommands::ThreeUpHorizontal,
    ArrangeCommands::ThreeUpStacked,
    ArrangeCommands::FourUp,
    ArrangeCommands::SixUp,
    ArrangeCommands::ConsolidateAllToTabs,
    ArrangeCommands::Cascade,
    ArrangeCommands::Tile,
    ArrangeCommands::FloatInWindow,
    ArrangeCommands::FloatAllInWindows,
    ArrangeCommands::NewWindowForDocument,
    ArrangeCommands::MatchZoom,
    ArrangeCommands::MatchLocation,
    ArrangeCommands::MatchRotation,
    ArrangeCommands::MatchAll,
];

/// The document area an arrangement works in.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ArrangeSpace<D> {
    /// The document area.
    pub area: Rect,
    /// The size of a new document window.
    pub size: Vec2,
    /// The cascade step (`size.window.cascade`).
    pub step: f32,
    /// The active document.
    pub active: Option<D>,
}

impl<D> ArrangeSpace<D> {
    /// The document area `area`, new windows of `size` stepped by `step`, and the `active` document.
    pub fn new(area: Rect, size: Vec2, step: f32, active: Option<D>) -> Self {
        ArrangeSpace { area, size, step, active }
    }
}

/// What the host does after an arrangement changed the windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Arranged {
    /// The document windows changed: the host applies `DocumentWindows::to_actions`, and lays the docked groups out
    /// with `arrange_rects` for an n-up arrangement.
    Windows,
    /// Every document view takes the active view's zoom, location and rotation, as flagged.
    #[non_exhaustive]
    Views {
        /// Match the zoom.
        zoom: bool,
        /// Match the scroll location.
        location: bool,
        /// Match the rotation.
        rotation: bool,
    },
}

/// Runs an Arrange command on the document windows.
pub fn arrange<D: Clone + Eq + Hash>(windows: &mut DocumentWindows<D>, command: ArrangeCommands, space: &ArrangeSpace<D>) -> Arranged {
    let arrangement = command.arrangement();
    let ArrangeSpace { area, size, step, active } = space;
    let next_slot = |windows: &DocumentWindows<D>| cascade_rects(*area, *size, windows.floating.len() + 1, *step).pop();
    match arrangement {
        Arrangement::TileAllVertically | Arrangement::TileAllHorizontally => {
            windows.float_all(*area, *size, *step);
            tile_floating(windows, arrangement, *area);
        }
        Arrangement::Tile => tile_floating(windows, arrangement, *area),
        Arrangement::TwoUpVertical
        | Arrangement::TwoUpHorizontal
        | Arrangement::ThreeUpVertical
        | Arrangement::ThreeUpHorizontal
        | Arrangement::ThreeUpStacked
        | Arrangement::FourUp
        | Arrangement::SixUp => {
            windows.consolidate(active.as_ref());
            windows.split_into(arrangement.group_count(0).unwrap_or(0));
        }
        Arrangement::ConsolidateAllToTabs => windows.consolidate(active.as_ref()),
        Arrangement::Cascade => {
            let rects = cascade_rects(*area, *size, windows.floating.len(), *step);
            for ((_, rect), cascaded) in windows.floating.iter_mut().zip(rects) {
                *rect = cascaded;
            }
        }
        Arrangement::FloatInWindow => {
            if let (Some(document), Some(rect)) = (active, next_slot(windows)) {
                windows.float(document, rect);
            }
        }
        Arrangement::FloatAllInWindows => windows.float_all(*area, *size, *step),
        Arrangement::NewWindowForDocument => {
            if let (Some(document), Some(rect)) = (active, next_slot(windows)) {
                windows.new_window_for(document, rect);
            }
        }
        Arrangement::MatchZoom => return Arranged::Views { zoom: true, location: false, rotation: false },
        Arrangement::MatchLocation => return Arranged::Views { zoom: false, location: true, rotation: false },
        Arrangement::MatchRotation => return Arranged::Views { zoom: false, location: false, rotation: true },
        Arrangement::MatchAll => return Arranged::Views { zoom: true, location: true, rotation: true },
        _ => {}
    }
    Arranged::Windows
}

fn tile_floating<D: Clone + Eq + Hash>(windows: &mut DocumentWindows<D>, arrangement: Arrangement, area: Rect) {
    let Some(rects) = arrange_rects(arrangement, area, windows.floating.len()) else { return };
    for ((_, rect), tiled) in windows.floating.iter_mut().zip(rects) {
        *rect = tiled;
    }
}

/// The Window › Workspace commands over the workspace set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, craft_ui_macros::CommandSet)]
#[command(prefix = "workspace")]
#[non_exhaustive]
pub enum WorkspaceCommands {
    /// Return the current workspace to its stored layout.
    #[command(label = "Reset Workspace")]
    Reset,
    /// Ask for a name and save the live layout under it.
    #[command(label = "New Workspace…")]
    New,
    /// Ask which saved workspace to delete.
    #[command(label = "Delete Workspace…")]
    Delete,
    /// Refuse moves and closes until it is unlocked; resizing still works.
    #[command(label = "Lock Workspace", kind = "toggle")]
    Lock,
}

/// What the host asks the user after a workspace command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkspaceAsk {
    /// A name for `WorkspaceSet::save`.
    Name,
    /// A saved workspace for `WorkspaceSet::delete`.
    Which,
}

/// Runs a workspace command: Reset and Lock take effect, New and Delete say what to ask.
pub fn run_workspace<P: Clone + Eq + Hash>(set: &mut WorkspaceSet<P>, live: &mut Workspace<P>, command: WorkspaceCommands) -> Option<WorkspaceAsk> {
    match command {
        WorkspaceCommands::Reset => {
            if let Some(stored) = set.reset() {
                *live = stored.clone();
            }
            None
        }
        WorkspaceCommands::Lock => {
            set.locked = !set.locked;
            None
        }
        WorkspaceCommands::New => Some(WorkspaceAsk::Name),
        WorkspaceCommands::Delete => Some(WorkspaceAsk::Which),
    }
}

/// Switches to the workspace `id`: the live layout is kept under the current id first, and the target's layout
/// becomes the live one.
///
/// # Errors
///
/// `WorkspaceError::UnknownId` when no workspace has the id.
pub fn select_workspace<P: Clone + Eq + Hash>(
    set: &mut WorkspaceSet<P>,
    live: &mut Workspace<P>,
    id: &str,
) -> Result<(), craft_behavior::dock::WorkspaceError> {
    set.select(id, live.clone())?;
    if let Some(layout) = set.layout() {
        *live = layout.clone();
    }
    Ok(())
}

/// The state of each workspace command: Reset when the current workspace has edits, Delete while a saved workspace
/// exists, Lock checked while the set is locked.
pub fn workspace_states<P: Clone + Eq + Hash>(set: &WorkspaceSet<P>) -> Vec<(WorkspaceCommands, CommandState)> {
    vec![
        (WorkspaceCommands::Reset, CommandState::new().enabled(set.edited.contains_key(&set.current))),
        (WorkspaceCommands::New, CommandState::new()),
        (WorkspaceCommands::Delete, CommandState::new().enabled(!set.saved.is_empty())),
        (WorkspaceCommands::Lock, CommandState::new().checked(set.locked)),
    ]
}
