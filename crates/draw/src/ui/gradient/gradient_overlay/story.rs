//! The gradient overlay's story: a gradient line over the picture it colours.

use craft_ui::testing::canvas_story::ViewArgs;
use egui::{Color32, Mesh, Ui};
use kurbo::Point;

use super::{ColorRequest, GradientLineNames, GradientOverlay, GradientOverlayResponse};
use crate::ui::gradient::stops::{StopRules, sample};
use crate::ui::gradient::{ColorStop, GradientEditor, GradientStops, StopRef, StopRow, StopRows};
use crate::ui::gradient::{GradientExtent, GradientLine, LineGradient, gradient_line};
use craft_theme::Srgba;
use craft_ui::canvas::CanvasView;
use craft_ui::edit::{EditEvent, GestureHost, UndoMarks, describe};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::story::{Level, Story, StoryChoice, Surface};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["Gradient line"];

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RowsArg {
    #[choice(name = "Split")]
    Split,
    #[choice(name = "Combined")]
    Combined,
}

impl RowsArg {
    fn rows(self) -> StopRows {
        match self {
            RowsArg::Split => StopRows::Split,
            RowsArg::Combined => StopRows::Combined,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ExtentArg {
    #[choice(name = "None")]
    None,
    #[choice(name = "Circle")]
    Circle,
    #[choice(name = "Square")]
    Square,
    #[choice(name = "Mirror")]
    Mirror,
}

impl ExtentArg {
    fn extent(self) -> GradientExtent {
        match self {
            ExtentArg::None => GradientExtent::None,
            ExtentArg::Circle => GradientExtent::Circle,
            ExtentArg::Square => GradientExtent::Square,
            ExtentArg::Mirror => GradientExtent::Mirror,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SelectedArg {
    #[choice(name = "None")]
    None,
    #[choice(name = "Stop 2")]
    Stop2,
    #[choice(name = "Midpoint 1")]
    Midpoint1,
}

#[derive(craft_ui::story::StoryArgs, Debug, PartialEq)]
struct EditArgs {
    #[arg(flatten)]
    view: ViewArgs,
    rows: RowsArg,
    extent: ExtentArg,
    selected: SelectedArg,
    reverse: bool,
    midpoints: bool,
    #[arg(name = "alt copies")]
    alt_copies: bool,
    editable: bool,
}

impl Default for EditArgs {
    fn default() -> Self {
        Self {
            view: ViewArgs::default(),
            rows: RowsArg::Split,
            extent: ExtentArg::None,
            selected: SelectedArg::Stop2,
            reverse: false,
            midpoints: true,
            alt_copies: false,
            editable: true,
        }
    }
}

#[derive(craft_ui::story::StoryArgs, Debug, PartialEq)]
struct WithEditorArgs {
    #[arg(flatten)]
    view: ViewArgs,
    rows: RowsArg,
    reverse: bool,
    midpoints: bool,
}

impl Default for WithEditorArgs {
    fn default() -> Self {
        Self { view: ViewArgs::default(), rows: RowsArg::Split, reverse: false, midpoints: true }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!("primitives/gradient-line/edit", Level::Primitives, "Gradient line", "Edit", EditArgs::default(), show_edit).on(Surface::Canvas),
        typed_story!(
            "primitives/gradient-line/with-editor",
            Level::Primitives,
            "Gradient line",
            "With editor",
            WithEditorArgs::default(),
            show_with_editor
        )
        .on(Surface::Canvas),
    ]
}

/// The English names the tests use.
#[cfg(test)]
pub(crate) fn english_names() -> GradientLineNames {
    GradientLineNames { group: "Gradient".into(), line: "Gradient line".into(), start: "Start point".into(), end: "End point".into() }
}

/// The story's stops: four colours with a half-transparent last stop, uneven so a midpoint moves.
pub(crate) fn story_stops(rows: StopRows) -> GradientStops {
    let alpha = if rows == StopRows::Combined { 0.45 } else { 1.0 };
    let mut color = vec![
        ColorStop::new(0.0, Srgba::rgb(0.13, 0.35, 0.72)),
        ColorStop { midpoint: 0.35, ..ColorStop::new(0.3, Srgba::rgb(0.95, 0.75, 0.2)) },
        ColorStop::new(0.7, Srgba::rgb(0.85, 0.25, 0.3)),
        ColorStop::new(1.0, Srgba { a: alpha, ..Srgba::rgb(0.9, 0.9, 0.95) }),
    ];
    color.sort_by(|a, b| a.location.total_cmp(&b.location));
    GradientStops { color, opacity: Vec::new() }
}

/// The story's line.
pub(crate) const LINE: GradientLine = GradientLine { start: Point::new(150.0, 230.0), end: Point::new(470.0, 150.0) };

/// A host's gradient, as the story and tests keep it: updates shown as they come.
#[derive(Clone, Debug)]
pub(crate) struct LineHost {
    pub value: LineGradient,
    /// The gradient when a drag began, to restore on a cancel.
    pub before: Option<LineGradient>,
}

impl LineHost {
    pub(crate) fn new(value: LineGradient) -> Self {
        Self { value, before: None }
    }

    pub(crate) fn apply(&mut self, response: &GradientOverlayResponse) {
        match &response.event {
            Some(EditEvent::Begin { .. }) => self.before = Some(self.value.clone()),
            Some(EditEvent::Update { value, .. }) => self.value = value.clone(),
            Some(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => {
                self.before = None;
                self.value = value.clone();
            }
            Some(EditEvent::Cancel { .. }) => {
                if let Some(before) = self.before.take() {
                    self.value = before;
                }
            }
            Some(_) | None => {}
        }
    }
}

/// The story's state: the host's gradient for a row layout, the last event and the "selected"
/// arg last applied.
#[derive(Clone)]
struct StoryState {
    rows: StopRows,
    host: LineHost,
    last: String,
    selected_arg: Option<SelectedArg>,
}

fn show_edit(ui: &mut Ui, args: &mut EditArgs) {
    let rows = args.rows.rows();
    let extent = args.extent.extent();
    let key = ui.scope_id().with("gradient-line-story");
    let fresh =
        || StoryState { rows, host: LineHost::new(LineGradient { line: LINE, stops: story_stops(rows) }), last: "–".into(), selected_arg: None };
    let mut state = ui.data(|d| d.get_temp::<StoryState>(key)).filter(|s| s.rows == rows).unwrap_or_else(fresh);
    let reverse = args.reverse;
    ui.label(format!(
        "Line {:.0}, {:.0} → {:.0}, {:.0}; {} stops. Last event: {}",
        state.host.value.line.start.x,
        state.host.value.line.start.y,
        state.host.value.line.end.x,
        state.host.value.line.end.y,
        state.host.value.stops.color.len(),
        state.last
    ));
    let canvas = craft_ui::testing::canvas_story::document(ui, args.view, "Document");
    let view = &canvas.view;
    paint_picture(ui, view, &state.host.value, rows, reverse);
    let id = ui.make_persistent_id("story-gradient");
    let selected_arg = args.selected;
    if Some(selected_arg) != state.selected_arg {
        let selected = match selected_arg {
            SelectedArg::Stop2 => Some(StopRef::Color(1)),
            SelectedArg::Midpoint1 => Some(StopRef::Midpoint(StopRow::Color, 0)),
            SelectedArg::None => None,
        };
        GradientOverlay::select(ui.ctx(), id, selected);
        state.selected_arg = Some(selected_arg);
    }
    let value = state.host.value.clone();
    let r = GradientOverlay::new("story-gradient", &value.line, &value.stops, GradientLineNames::of(ui))
        .rules(StopRules { rows, ..StopRules::default() })
        .midpoints(args.midpoints)
        .reverse(reverse)
        .extent(extent)
        .alt_copies(args.alt_copies)
        .editable(args.editable)
        .show(ui, &canvas.response, view);
    state.host.apply(&r);
    if let Some(event) = &r.event {
        state.last = describe(&event.clone().map(|_| ()));
    }
    if let Some(ColorRequest { index: i, .. }) = r.color_requested {
        state.last = format!("colour of stop {} requested", i + 1);
    }
    ui.data_mut(|d| d.insert_temp(key, state));
}

/// The gradient on the document: a grid of cells coloured by their location along the line.
fn paint_picture(ui: &Ui, view: &CanvasView, value: &LineGradient, rows: StopRows, reverse: bool) {
    const COLUMNS: u32 = 64;
    const ROWS: u32 = 40;
    let doc = craft_ui::testing::canvas_story::DOC;
    let mut cells = Mesh::default();
    for j in 0..ROWS {
        for i in 0..COLUMNS {
            let (x0, y0) = (doc.x0 + doc.width() * f64::from(i) / f64::from(COLUMNS), doc.y0 + doc.height() * f64::from(j) / f64::from(ROWS));
            let (w, h) = (doc.width() / f64::from(COLUMNS), doc.height() / f64::from(ROWS));
            let centre = Point::new(x0 + w / 2.0, y0 + h / 2.0);
            let (t, _) = gradient_line::project(&value.line, centre, reverse);
            let color = Color32::from(sample(&value.stops, rows, t));
            let base = cells.vertices.len() as u32;
            for p in [(x0, y0), (x0 + w, y0), (x0 + w, y0 + h), (x0, y0 + h)] {
                cells.colored_vertex(view.to_screen(Point::new(p.0, p.1)), color);
            }
            cells.add_triangle(base, base + 1, base + 2);
            cells.add_triangle(base, base + 2, base + 3);
        }
    }
    ui.painter_at(view.rect).add(cells);
}

/// The gradient of the "With editor" story and its undo history of whole snapshots.
#[derive(Clone, Debug)]
struct Doc {
    value: LineGradient,
    undo: Vec<LineGradient>,
}

impl Doc {
    fn apply(&mut self, value: LineGradient) {
        self.undo.push(self.value.clone());
        self.value = value;
    }
}

impl UndoMarks for Doc {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(before) = self.undo.get(mark).cloned() {
            self.value = before;
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

/// The story's host: one gradient, edited by the overlay and the editor through one gesture host.
#[derive(Clone, Debug)]
struct WithEditor {
    rows: StopRows,
    doc: Doc,
    host: GestureHost<Doc>,
}

fn show_with_editor(ui: &mut Ui, args: &mut WithEditorArgs) {
    let rows = args.rows.rows();
    let key = ui.scope_id().with("gradient-line-with-editor");
    let fresh =
        || WithEditor { rows, doc: Doc { value: LineGradient { line: LINE, stops: story_stops(rows) }, undo: Vec::new() }, host: GestureHost::new() };
    let mut state = ui.data(|d| d.get_temp::<WithEditor>(key)).filter(|s| s.rows == rows).unwrap_or_else(fresh);
    let (reverse, ctx) = (args.reverse, ui.ctx().clone());
    let numbers = LocaleNumbers::from_tag("en");
    let rules = StopRules { rows, ..StopRules::default() };
    ui.label(format!("Undo steps: {}", state.doc.undo.len()));
    ui.horizontal_top(|ui| {
        let editor_id = ui
            .vertical(|ui| {
                ui.set_width(300.0);
                let stops = state.doc.value.stops.clone();
                let bar = GradientEditor::new("story-gradient-bar", "Gradient", &stops, &numbers).rows(rows).stops(rules.min, rules.max).show(ui);
                if let Some(event) = bar.edit.event {
                    state.host.handle(&ctx, &mut state.doc, event, |doc: &mut Doc, stops: GradientStops| {
                        let line = doc.value.line;
                        doc.apply(LineGradient { line, stops });
                    });
                }
                bar.edit.response.id
            })
            .inner;
        let canvas = craft_ui::testing::canvas_story::document(ui, args.view, "Document");
        let view = &canvas.view;
        paint_picture(ui, view, &state.doc.value, rows, reverse);
        let value = state.doc.value.clone();
        let r = GradientOverlay::new("story-gradient", &value.line, &value.stops, GradientLineNames::of(ui))
            .rules(rules.clone())
            .midpoints(args.midpoints)
            .reverse(reverse)
            .selection_of(editor_id)
            .show(ui, &canvas.response, view);
        if let Some(event) = r.event {
            state.host.handle(&ctx, &mut state.doc, event, Doc::apply);
        }
    });
    ui.data_mut(|d| d.insert_temp(key, state));
}
