//! [`ConnectorBar`]: the connector part of a style bar.

use egui::{AsIdSalt, Color32, IdSalt, Painter, Rect, Stroke, Ui, UiBuilder, pos2, vec2};

use super::ConnectorBarLabels;
use crate::geometry::connector::Routing;
use crate::ui::stroke::{StrokeChoiceRow, StrokeEditor};
use craft_ui::controls::mixed::ApplyRowLabels;
use craft_ui::controls::{Segmented, Select, SelectText};
use craft_ui::icons::Icon;
use craft_ui::localized::Labels as _;
use craft_ui::patterns::{OptionsBar, OptionsBarSlots};
use craft_ui::theme::{Tokens, TokensOf as _};

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

/// What [`ConnectorBar`] reported this pass: one field per control, `Some` when that control was
/// changed. Each change is one undo step (`Set`).
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
#[must_use = "the app applies each choice to the selected connectors"]
pub struct ConnectorBarOutput {
    /// The routing button picked.
    pub routing: Option<Routing>,
    /// The start cap picked: the [`StrokeChoice`](crate::ui::stroke::StrokeChoice) id from the
    /// app's start row.
    pub start_cap: Option<String>,
    /// The end cap picked: the choice's id from the app's end row.
    pub end_cap: Option<String>,
    /// The line style menu's pick, as the whole new line.
    pub line: Option<ConnectorLine>,
}

/// The connector part of a style bar, in FigCraft's order: three routing buttons (Straight, Elbow,
/// Curved, drawn as icons; a radio group, the current one on), the start and end cap menus (the app's
/// [`StrokeChoiceRow`]s from [`caps`](Self::caps), with its arrowheads and their painted previews;
/// the library holds no arrowhead data; no menus without them), and the line style menu (Solid, Dash, then the [weights](ConnectorLine::WEIGHTS)).
/// `add_to` fills the bar's body slot; the choices go to the [`ConnectorBarOutput`] of
/// [`output`](Self::output). Each control is a Tab stop; the menus are keyboard operable as
/// `Select`s.
pub struct ConnectorBar<'a> {
    id_salt: IdSalt,
    routing: Routing,
    line: ConnectorLine,
    caps: Option<(StrokeChoiceRow<'a>, StrokeChoiceRow<'a>)>,
    labels: Option<ConnectorBarLabels>,
    output: Option<&'a mut ConnectorBarOutput>,
}

impl<'a> ConnectorBar<'a> {
    /// The bar for a connector routed `routing` with `line`.
    pub fn new(id_salt: impl AsIdSalt, routing: Routing, line: ConnectorLine) -> Self {
        ConnectorBar { id_salt: IdSalt::new(id_salt), routing, line, caps: None, labels: None, output: None }
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

    /// Where the bar reports its choices.
    pub fn output(mut self, output: &'a mut ConnectorBarOutput) -> Self {
        self.output = Some(output);
        self
    }

    /// Fills the bar's body slot.
    pub fn add_to(self, slots: &mut OptionsBarSlots<'a>) {
        slots.body(move |ui| self.cells(ui));
    }

    fn cells(self, ui: &mut Ui) {
        let Self { id_salt, routing, line, caps, labels, output } = self;
        let labels = labels.unwrap_or_else(|| ConnectorBarLabels::of(ui));
        let tokens = Tokens::of(ui);
        let bar = OptionsBar::style(&tokens, false);
        let ink = StrokeEditor::style(&tokens).sample_color;
        let preview = vec2(bar.fields[1], Select::style(&tokens, OptionsBar::CONTROL_SIZE).field.icon);
        let mixed_word = ApplyRowLabels::of(ui).mixed;
        let mut out = ConnectorBarOutput::default();
        let builder = UiBuilder::new().id_salt(id_salt).layout(*ui.layout());
        ui.scope_builder(builder, |ui| {
            let mut picked = routing;
            let segmented = Segmented::new("routing", labels.routing.as_str(), &mut picked)
                .size(OptionsBar::CONTROL_SIZE)
                .icon_item(Routing::Straight, Icon::RouteStraight, labels.straight.as_str())
                .icon_item(Routing::Elbow, Icon::RouteElbow, labels.elbow.as_str())
                .icon_item(Routing::Curved, Icon::RouteCurved, labels.curved.as_str());
            if segmented.show(ui).changed() {
                out.routing = Some(picked);
            }

            if let Some((start, end)) = &caps {
                OptionsBar::divider(ui);
                for (row, name, salt, slot) in
                    [(start, labels.start.as_str(), "start-cap", &mut out.start_cap), (end, labels.end.as_str(), "end-cap", &mut out.end_cap)]
                {
                    let mixed = row.selected.is_some_and(|m| m.is_mixed());
                    let mut value: Option<String> = row.selected.and_then(|m| m.same()).map(str::to_owned);
                    let mut select = Select::new(salt, name, &mut value)
                        .size(OptionsBar::CONTROL_SIZE)
                        .width(bar.fields[2])
                        .text_shown(SelectText::InList)
                        .placeholder(if mixed { mixed_word.as_str() } else { "" });
                    for choice in row.choices {
                        select = select.option(Some(choice.id.clone()), choice.label.clone());
                    }
                    if let Some(paint) = &row.paint {
                        select = select.row_content(preview, |painter, rect, id: &Option<String>| {
                            if let Some(choice) = row.choices.iter().find(|c| Some(&c.id) == id.as_ref()) {
                                paint(painter, rect, choice);
                            }
                        });
                    }
                    if let Some(id) = select.show(ui).committed().cloned().flatten() {
                        *slot = Some(id);
                    }
                }
            }

            OptionsBar::divider(ui);
            let mut value = LinePick::Style(line.dashed);
            let mut select = Select::new("line-style", labels.line_style.as_str(), &mut value)
                .size(OptionsBar::CONTROL_SIZE)
                .width(bar.fields[1])
                .text_shown(SelectText::InList)
                .option(LinePick::Style(false), labels.solid.as_str())
                .option(LinePick::Style(true), labels.dashed.as_str())
                .separator();
            for (index, weight) in ConnectorLine::WEIGHTS.iter().enumerate() {
                let text = labels.weight.replace("{weight}", &weight.to_string());
                select = select.option(LinePick::Weight(index), text);
            }
            let select = select.row_content(preview, |painter, rect, pick: &LinePick| {
                let (dashed, weight) = match *pick {
                    LinePick::Style(dashed) => (dashed, line.weight),
                    LinePick::Weight(index) => (false, ConnectorLine::WEIGHTS[index]),
                };
                paint_line(painter, rect, ink, weight, dashed);
            });
            if let Some(pick) = select.show(ui).committed().copied() {
                let line = match pick {
                    LinePick::Style(dashed) => ConnectorLine { dashed, ..line },
                    LinePick::Weight(index) => ConnectorLine { weight: ConnectorLine::WEIGHTS[index], ..line },
                };
                out.line = Some(line);
            }
        });
        if let Some(output) = output {
            *output = out;
        }
    }
}

/// A row of the line style menu: the solid or dashed rows set the dashes, a weight row the weight.
#[derive(Clone, Copy, Debug, PartialEq)]
enum LinePick {
    Style(bool),
    Weight(usize),
}

/// A line `weight` thick, dashed or not, across the middle of `rect`; no thicker than half of it.
fn paint_line(painter: &Painter, rect: Rect, ink: Color32, weight: f64, dashed: bool) {
    let stroke = Stroke::new((weight as f32).min(rect.height() / 2.0), ink);
    let ends = [pos2(rect.min.x, rect.center().y), pos2(rect.max.x, rect.center().y)];
    if dashed {
        let [dash, gap] = ConnectorLine::DASH.map(|n| n as f32);
        painter.extend(egui::Shape::dashed_line(&ends, stroke, dash, gap));
    } else {
        painter.line_segment(ends, stroke);
    }
}
