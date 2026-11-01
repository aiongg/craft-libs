//! The Connector tool ([`ConnectorTool`]: hover attach points, drag a connector, reattach an end,
//! reshape a run or a curve), the overlay of a selected connector ([`ConnectorOverlay`]) and the
//! connector part of a style bar ([`ConnectorBar`]). Routes come from the UI-free
//! [`geometry::connector`](crate::geometry::connector), which an app's engine also calls.
//!
//! ```no_run
//! use craft_draw::geometry::connector::Routing;
//! use craft_draw::ui::connector::{ConnectionTarget, ConnectorEvent, ConnectorTool};
//! use craft_ui::canvas::{CanvasId, CanvasView, ToolHost};
//! use craft_ui::edit::EditEvent;
//! # fn demo(ui: &mut egui::Ui, view: &CanvasView, canvas: &egui::Response, id: CanvasId, tool: &mut ConnectorTool, targets: &[ConnectionTarget]) {
//! tool.targets(targets).routing(Routing::Elbow);
//! let _ = ToolHost::new(id).show(ui, view, canvas, tool);
//! for event in tool.take_events() {
//!     if let ConnectorEvent::Draw(EditEvent::Commit { value, changed: true, .. }) = event {
//!         // board.connector from value.from to value.to
//!         let _ = value;
//!     }
//! }
//! # }
//! ```

use kurbo::{Point, Rect};

use super::add_beside::AddSide;
use crate::geometry::connector::{ConnectionSide, ConnectorPath, Routing};
use craft_ui::edit::EditEvent;

mod bar;
#[cfg(test)]
mod bar_tests;
mod labels;
mod overlay;
pub(crate) mod stories;
mod style;
mod tool;
#[cfg(test)]
mod tool_tests;

pub use bar::{ConnectorBar, ConnectorBarEvent, ConnectorBarOutput, ConnectorLine};
pub use labels::{ConnectorBarLabels, ConnectorNames};
pub use overlay::ConnectorOverlay;
pub use style::ConnectorStyle;
pub use tool::ConnectorTool;

/// A place on an object a connector end may glue to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectionSite {
    /// In document units.
    pub point: Point,
    /// The side the route leaves by from here.
    pub side: ConnectionSide,
    /// The site's number on its object (DeckCraft's connection sites); a side middle's index in
    /// [`ConnectionSide::ALL`] for the default sites.
    pub index: u32,
}

/// An object a connector may glue to, as the app gives it this pass.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionTarget {
    /// The app's id of the object.
    pub id: u64,
    /// Its bounds, document units.
    pub bounds: Rect,
    /// Its sites; empty means the four side middles ([`ConnectionTarget::side_sites`]).
    pub sites: Vec<ConnectionSite>,
}

impl ConnectionTarget {
    /// The target `id` with the four side middles of `bounds` as its sites.
    pub fn new(id: u64, bounds: Rect) -> Self {
        ConnectionTarget { id, bounds, sites: Vec::new() }
    }

    /// The sites a connector may glue to: [`sites`](Self::sites), or the four side middles.
    pub fn side_sites(&self) -> Vec<ConnectionSite> {
        if !self.sites.is_empty() {
            return self.sites.clone();
        }
        ConnectionSide::ALL.iter().zip(0..).map(|(&side, index)| ConnectionSite { point: side.point(self.bounds), side, index }).collect()
    }
}

/// One end of a connector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectorEnd {
    /// Not glued: a point in the document.
    Free(Point),
    /// Glued to object `target`: by `side` (FigCraft), at numbered `site` (DeckCraft), or at `at`
    /// as fractions of its box placed by hand (FigCraft's Cmd/Ctrl drag).
    Glued { target: u64, side: ConnectionSide, site: Option<u32>, at: Option<[f64; 2]> },
}

/// Which end of a connector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConnectorEndKind {
    Start,
    End,
}

/// The selected connector as the tool and the overlay see it this pass: a snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectorItem {
    pub id: u64,
    pub from: ConnectorEnd,
    pub to: ConnectorEnd,
    pub routing: Routing,
    /// The hand-reshaped path, `None` when it routes by itself.
    pub path: Option<ConnectorPath>,
}

/// The connector being drawn: its ends, routing and route, for the app's own preview
/// ([`ConnectorTool::custom_preview`]) and the commit.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectorDraft {
    pub from: ConnectorEnd,
    pub to: ConnectorEnd,
    pub routing: Routing,
    /// The route's points ([`route`](crate::geometry::connector::route)).
    pub points: Vec<Point>,
}

/// What [`ConnectorTool`] reports, drained with [`ConnectorTool::take_events`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ConnectorEvent {
    /// A new connector: `Begin` past the click slop, `Update`, `Commit` (`changed: false` when it
    /// ends on its start object), `Cancel`.
    Draw(EditEvent<ConnectorDraft>),
    /// An end of the selected connector dragged onto another object or onto empty canvas.
    Reattach { end: ConnectorEndKind, event: EditEvent<ConnectorEnd> },
    /// A path handle of the selected connector dragged: the new path.
    Reshape(EditEvent<ConnectorPath>),
    /// A "+" button beside the selected object: add a connected object on that side.
    AddConnected { side: AddSide },
}
