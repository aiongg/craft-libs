//! The Connector tool ([`ConnectorTool`]: hover attach points, drag a new connector), the handles of
//! a selected connector ([`ConnectorHandles`]: reattach an end, reshape a run or a curve) and the
//! connector part of an options bar ([`ConnectorBar`]). Routes come from the UI-free
//! [`geometry::connector`](crate::geometry::connector), which an app's engine also calls.
//!
//! The tool and the handles are two parts of the canvas: the tool takes a press on empty canvas or
//! on an object, the handles (a canvas control) take a press on the selected connector's ends and
//! bends before the tool sees it.

mod bar;
mod handles;
mod overlay;
mod place;
mod tool;
mod words;

#[cfg(feature = "screen-testing")]
mod stories;

use craft_behavior::EditEvent;
use kurbo::{Point, Rect};

use crate::geometry::connector::{ConnectionSide, ConnectorPath, Routing};

pub use bar::{ConnectorBar, ConnectorBarOutput, ConnectorLine};
pub use handles::{ConnectorHandles, ConnectorHandlesOutput};
pub use tool::ConnectorTool;
pub use words::{ConnectorBarWords, ConnectorWords};

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

/// An object a connector may glue to, as the app gives it this pass. The app passes the selected
/// connector's end objects among the targets, or the handles have no route for it.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectionTarget {
    /// The app's id of the object.
    pub id: u64,
    /// The object's name for screen readers ("Rectangle 2"); `None` reads the number.
    pub name: Option<String>,
    /// Its bounds, document units.
    pub bounds: Rect,
    /// Its sites; empty means the four side middles ([`ConnectionTarget::side_sites`]).
    pub sites: Vec<ConnectionSite>,
}

impl ConnectionTarget {
    /// The target `id` with the four side middles of `bounds` as its sites.
    pub fn new(id: u64, bounds: Rect) -> Self {
        ConnectionTarget { id, name: None, bounds, sites: Vec::new() }
    }

    /// The target named `name` for screen readers.
    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
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
    Glued {
        /// Id of the object glued to.
        target: u64,
        /// The side the end leaves by.
        side: ConnectionSide,
        /// The numbered connection site, when the product has them.
        site: Option<u32>,
        /// A hand-placed spot as fractions of the target's box.
        at: Option<[f64; 2]>,
    },
}

/// Which end of a connector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConnectorEndKind {
    /// The `from` end.
    Start,
    /// The `to` end.
    End,
}

/// A connector as the handles and the app see it: the value `ConnectorHandles` edits.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectorItem {
    /// The app's id for the connector.
    pub id: u64,
    /// The start end.
    pub from: ConnectorEnd,
    /// The finish end.
    pub to: ConnectorEnd,
    /// How the connector runs between its ends.
    pub routing: Routing,
    /// The hand-reshaped path, `None` when it routes by itself.
    pub path: Option<ConnectorPath>,
}

/// The connector being drawn: its ends, routing and route, for the app's own preview
/// ([`ConnectorTool::custom_preview`]) and the commit.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnectorDraft {
    /// The start end.
    pub from: ConnectorEnd,
    /// The finish end.
    pub to: ConnectorEnd,
    /// How the connector runs between its ends.
    pub routing: Routing,
    /// The route's points ([`route`](crate::geometry::connector::route)).
    pub points: Vec<Point>,
}

/// What the tool and the handles report.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ConnectorEvent {
    /// A new connector, from [`ConnectorTool::take_events`]: `Begin` past the click slop, `Update`,
    /// `Commit`, `Cancel`. A drag that ends on its start object is cancelled as `Dismissed`.
    Draw(EditEvent<ConnectorDraft>),
    /// An end of the selected connector dragged onto another object or onto empty canvas, or moved
    /// to the next object with the arrow keys, from [`ConnectorHandlesOutput::event`]. The gesture's
    /// phases are in the output's `edit.event`.
    Reattach {
        /// Which end moved.
        end: ConnectorEndKind,
        /// The end's new attachment.
        to: ConnectorEnd,
    },
    /// A bend of the selected connector dragged or nudged: the new path, from
    /// [`ConnectorHandlesOutput::event`].
    Reshape(ConnectorPath),
}
