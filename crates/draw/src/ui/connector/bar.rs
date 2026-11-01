//! [`ConnectorBar`]: the connector part of a style bar (task SH-T11).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Ui};

use super::ConnectorBarLabels;
use crate::geometry::connector::Routing;

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
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum ConnectorBarEvent {
    Routing(Routing),
    /// The start cap picked: its index in the caps the app gave.
    StartCap(usize),
    /// The end cap picked: its index in the caps the app gave.
    EndCap(usize),
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
/// Curved; a radio group, the current one on), the start and end cap menus (S7-pen's
/// `StrokeChoiceRow`s with the app's arrowheads and their painted previews; the library holds no
/// arrowhead data), and the line style menu (Solid, Dash, then the [weights](ConnectorLine::WEIGHTS)).
/// Each control is a Tab stop; the menus are keyboard operable as `Select`s.
pub struct ConnectorBar<'a> {
    id_salt: IdSalt,
    routing: Routing,
    line: ConnectorLine,
    // needs: PE craft_draw::ui::stroke::StrokeChoice: `caps: (&'a [StrokeChoice], &'a [StrokeChoice])`
    // and `current_caps: (Option<usize>, Option<usize>)`, set by a `caps(start, end, current)`
    // builder.
    labels: Option<ConnectorBarLabels>,
    lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> ConnectorBar<'a> {
    /// The bar for a connector routed `routing` with `line`.
    pub fn new(id_salt: impl AsIdSalt, routing: Routing, line: ConnectorLine) -> Self {
        ConnectorBar { id_salt: IdSalt::new(id_salt), routing, line, labels: None, lifetime: std::marker::PhantomData }
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
        // needs: PE craft_draw::ui::stroke::StrokeChoiceRow (the cap menus).
        todo!()
    }
}
