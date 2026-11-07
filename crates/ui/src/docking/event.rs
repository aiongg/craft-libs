//! `DockEvent`, `FloatSpace` and `HidePanels`: what a dock reports to the host and the host tells a dock.

use crate::menu::GroupItem;
use craft_behavior::GroupId;
use egui::{Rect, Vec2};

/// Gap between the top start corner of the dock area and a unit a command floats without a rect, so it stays clear
/// of the app's menu bar.
pub const FLOAT_INSET: f32 = 24.0;

/// Width a command floats a panel at when the area never drew it.
pub const FLOAT_WIDTH: f32 = 260.0;

/// Height a command floats a panel at when the area never drew it.
pub const FLOAT_HEIGHT: f32 = 320.0;

/// A request for host state outside the layout, or a report of where something ended up.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DockEvent<P> {
    /// Maximize Frame: maximize the frame holding `panel`, or restore it when it is the one the host passed as
    /// maximized.
    ToggleMaximize {
        /// A panel of the frame to maximize or restore.
        panel: P,
    },
    /// Auto-Collapse Iconic Panels was chosen in a column menu. The host flips its preference and passes it back.
    ToggleAutoCollapse,
    /// The area's flyout opened, switched to another panel or closed, whatever changed it. It comes once per
    /// change, in the pass that shows the change.
    FlyoutChanged {
        /// The panel now in the flyout; `None` when it closed.
        panel: Option<P>,
    },
    /// A dock row of `panel`'s menu was chosen. The host's own rows are commands and run by themselves.
    PanelCommand {
        /// The panel whose menu the row belongs to.
        panel: P,
        /// The chosen row.
        command: GroupItem,
    },
    /// A dock column was placed: the column that holds `group` first took `rect` this pass. Hosts that anchor
    /// their own surfaces to a column read it.
    ColumnPlaced {
        /// The first group of the column.
        group: GroupId,
        /// The column's rect in the area's layer space.
        rect: Rect,
    },
    /// A floating frame holding `panel` was moved or resized to `rect`.
    FloatPlaced {
        /// A panel of the frame.
        panel: P,
        /// The frame's rect in the area's layer space.
        rect: Rect,
    },
}

/// Which panels the host hides. Protected panels always stay.
#[non_exhaustive]
pub enum HidePanels<'a, P> {
    /// Hides none.
    None,
    /// Hides every panel.
    All,
    /// Every panel except those for which the function returns true.
    Except(&'a dyn Fn(&P) -> bool),
}

impl<P> Clone for HidePanels<'_, P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P> Copy for HidePanels<'_, P> {}

/// How much shorter than its body content must be to count as not filling it.
const FILL_TOLERANCE: f32 = 1.0;

/// Where a docking command floats a panel, group or column when it gives no rect. A dock records one each pass;
/// the host reads it to turn a command into placements.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FloatSpace<P> {
    /// The rect the area drew in. A floated unit goes inside it, `inset` from its top-left corner.
    pub area: Rect,
    /// Room between the area's top-left corner and a unit a command floats.
    pub inset: f32,
    /// The size a command floats a panel at when the area never drew it.
    pub default_size: Vec2,
    /// What a drag floats from each drawn panel: the panel alone, its group and its dock column.
    panels: Vec<(P, Vec2)>,
    groups: Vec<(P, Vec2)>,
    columns: Vec<(P, Vec2)>,
    /// The height each panel's content used when it was last drawn, which fits a floated group to its content;
    /// `None` for content that filled its body.
    contents: Vec<(P, Option<f32>)>,
    /// The area draws with `DockConfig::side_edge_zones`: an outer edge drop wraps the part between the docks.
    side_edge_zones: bool,
}

impl<P: PartialEq> FloatSpace<P> {
    /// An area with no recorded sizes: commands float a panel at `FLOAT_WIDTH` by `FLOAT_HEIGHT`, `FLOAT_INSET`
    /// inside the area.
    pub fn new(area: Rect) -> Self {
        FloatSpace {
            area,
            inset: FLOAT_INSET,
            default_size: Vec2::new(FLOAT_WIDTH, FLOAT_HEIGHT),
            panels: Vec::new(),
            groups: Vec::new(),
            columns: Vec::new(),
            contents: Vec::new(),
            side_edge_zones: false,
        }
    }

    /// For an area drawn with `DockConfig::side_edge_zones` when the area has not drawn yet (a headless run).
    #[must_use]
    pub fn with_side_edge_zones(mut self, on: bool) -> Self {
        self.side_edge_zones = on;
        self
    }

    /// Whether the area draws with `DockConfig::side_edge_zones`.
    pub fn side_edge_zones(&self) -> bool {
        self.side_edge_zones
    }

    /// Adds the size a command floats `panel` at alone when the area has not drawn it, such as a hidden panel or a
    /// headless run. A size the area recorded for the panel takes precedence.
    #[must_use]
    pub fn with_panel_size(mut self, panel: P, size: Vec2) -> Self {
        self.panels.push((panel, size));
        self
    }

    /// Records what a drag floats from `panel`: the panel alone, its group and its dock column. The first record
    /// of a panel wins: an iconic panel is recorded at its icon before its flyout.
    pub fn record_sizes(&mut self, panel: &P, alone: Vec2, group: Vec2, column: Vec2)
    where
        P: Clone,
    {
        for (sizes, size) in [(&mut self.panels, alone), (&mut self.groups, group), (&mut self.columns, column)] {
            if !sizes.iter().any(|(held, _)| held == panel) {
                sizes.push((panel.clone(), size));
            }
        }
    }

    /// The size a command floats `panel` at alone, when recorded.
    pub fn panel(&self, panel: &P) -> Option<Vec2> {
        Self::find(&self.panels, panel)
    }

    /// The size a command floats the group of `panel` at, when recorded.
    pub fn group(&self, panel: &P) -> Option<Vec2> {
        Self::find(&self.groups, panel)
    }

    /// The size a command floats the dock column of `panel` at, when recorded.
    pub fn column(&self, panel: &P) -> Option<Vec2> {
        Self::find(&self.columns, panel)
    }

    fn find(sizes: &[(P, Vec2)], panel: &P) -> Option<Vec2> {
        sizes.iter().find(|(held, _)| held == panel).map(|(_, size)| *size)
    }

    /// `[x, y, width, height]` of `size` inside the area, inset from its top-left corner and shrunk to fit.
    pub fn place(&self, size: Vec2) -> [f32; 4] {
        let size = size.min(self.area.size());
        let min = (self.area.min + Vec2::splat(self.inset)).min(self.area.max - size).max(self.area.min);
        [min.x, min.y, size.x, size.y]
    }

    /// Records the height `panel`'s content used in a body `available` tall. Content that reaches the body's bottom
    /// filled it and has no height of its own, unless the body was sized to the height recorded before (a fitted
    /// window).
    pub fn record(&mut self, panel: P, used: f32, available: f32) {
        let used = if used.is_finite() { used.max(0.0) } else { 0.0 };
        let at = self.contents.iter().position(|(held, _)| *held == panel);
        let fitted = at.and_then(|at| self.contents[at].1).is_some_and(|height| height >= available - FILL_TOLERANCE);
        let height = (used < available - FILL_TOLERANCE || fitted).then_some(used);
        match at {
            Some(at) => self.contents[at].1 = height,
            None => self.contents.push((panel, height)),
        }
    }

    /// The height `panel`'s content used when it was last recorded; `None` for content that filled its body or was
    /// never recorded.
    pub fn content_height(&self, panel: &P) -> Option<f32> {
        self.contents.iter().find(|(held, _)| held == panel).and_then(|(_, height)| *height)
    }
}
