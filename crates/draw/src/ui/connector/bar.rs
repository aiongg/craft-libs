//! [`ConnectorBar`]: the connector part of a style bar (task SH-T11).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Ui};

use super::ConnectorBarLabels;
use crate::geometry::connector::Routing;
use crate::ui::stroke::StrokeChoiceRow;

/// A connector's line style as the bar's menu offers it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectorLine {
    /// Dashed with [`DASH`](Self::DASH), else solid.
    pub dashed: bool,
    /// The stroke weight in points.
    pub weight: f64,
}

impl ConnectorLine {
    /// The weights the menu offers (FigCraft's `WEIGHTS`).
    pub const WEIGHTS: [f64; 4] = [1.0, 2.0, 4.0, 6.0];
    /// A dashed connector's dash and gap (FigCraft's `DASHES`).
    pub const DASH: [f64; 2] = [8.0, 6.0];
}

impl Default for ConnectorLine {
    /// Solid, 2 points.
    fn default() -> Self {
        ConnectorLine { dashed: false, weight: 2.0 }
    }
}

/// One choice made in the bar; each is one undo step (`Set`).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ConnectorBarEvent {
    Routing(Routing),
    /// The start cap picked: the [`StrokeChoice`](crate::ui::stroke::StrokeChoice) id from the
    /// app's start row.
    StartCap(String),
    /// The end cap picked: the choice's id from the app's end row.
    EndCap(String),
    Line(ConnectorLine),
}

/// What [`ConnectorBar::show`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app applies each choice to the selected connectors"]
pub struct ConnectorBarOutput {
    pub events: Vec<ConnectorBarEvent>,
}

/// The connector part of a style bar, in FigCraft's order: three routing buttons (Straight, Elbow,
/// Curved; a radio group, the current one on), the start and end cap menus (the app's
/// [`StrokeChoiceRow`]s from [`caps`](Self::caps), with its arrowheads and their painted previews;
/// the library holds no arrowhead data; no menus without them), and the line style menu (Solid, Dash, then the [weights](ConnectorLine::WEIGHTS)).
/// Each control is a Tab stop; the menus are keyboard operable as `Select`s.
pub struct ConnectorBar<'a> {
    id_salt: IdSalt,
    routing: Routing,
    line: ConnectorLine,
    caps: Option<(StrokeChoiceRow<'a>, StrokeChoiceRow<'a>)>,
    labels: Option<ConnectorBarLabels>,
}

impl<'a> ConnectorBar<'a> {
    /// The bar for a connector routed `routing` with `line`.
    pub fn new(id_salt: impl AsIdSalt, routing: Routing, line: ConnectorLine) -> Self {
        ConnectorBar { id_salt: IdSalt::new(id_salt), routing, line, caps: None, labels: None }
    }

    /// The start and end cap menus: the app's rows of arrowheads, each with its picked choice
    /// ([`StrokeChoiceRow::selected`], mixed across a selection) and its preview painter.
    pub fn caps(mut self, start: StrokeChoiceRow<'a>, end: StrokeChoiceRow<'a>) -> Self {
        self.caps = Some((start, end));
        self
    }

    /// The words; the installed ones by default.
    pub fn labels(mut self, labels: ConnectorBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Adds the controls to the bar's row `ui`.
    pub fn show(self, ui: &mut Ui) -> ConnectorBarOutput {
        // inv-figcraft: crates/ui-egui/src/canvas/board_bar.rs:350-371 `line_part` (routings :351-354),
        // :375-397 `cap_menu` ("Start point", "End point"), :411-445 `style_menu`, :343 `WEIGHTS`,
        // :345 `DASHES`.
        // The cap menus are `Select`s over each row's choices with its `paint` previews, as
        // `StrokeEditor` shows its rows.
        todo!()
    }
}
