//! Stories of the stroke editor (task ST-PE1; task RG2 registers them, adds `stories()` to the
//! draw stories and `COMPONENTS` to the registry, and regenerates the dump). Every story keeps a
//! fake [`StrokeStyle`] in egui memory and applies the editor's events to it as an app would; the
//! choice rows paint fake arrowheads and profiles.
//!
//! | Story id | Shows |
//! |---|---|
//! | `composites/stroke-editor/panel` | The docked body; args: closed, dashed, mixed, sections (panel, popover), unit (widths, document) |
//! | `composites/stroke-button/popover` | The opener and the popover; args: stroke on or off |
//! | `composites/stroke-editor/choice-rows` | Arrowhead start and end and profile rows with painted previews |

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Ui, vec2};

use super::{
    DashUnit, LineStyle, StrokeChoice, StrokeChoiceRow, StrokeEditor, StrokeEditorStyle, StrokeField, StrokeMixed, StrokeSections, StrokeStyle,
};
use super::{LineCap, LineJoin, StrokeAlign, StrokeButton};
use craft_ui::controls::{Button, MixedValue};
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, lock, shared};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["StrokeEditor", "StrokeButton"];

craft_ui::words! {
    /// The row and choice names of the stroke choice-row story, English by default.
    #[non_exhaustive]
    pub(crate) struct StrokeStoryLabels {
        profile: "strokeStory.profile" = "Profile";
        arrow: "strokeStory.arrow" = "Arrow";
        circle: "strokeStory.circle" = "Circle";
        square: "strokeStory.square" = "Square";
        bar: "strokeStory.bar" = "Bar";
        uniform: "strokeStory.uniform" = "Uniform";
        taper: "strokeStory.taper" = "Taper";
        bulge: "strokeStory.bulge" = "Bulge";
        /// The last-event line after a choice row picks: the row's name and the choice's.
        row_choice: "strokeStory.rowChoice" = "{row}: {choice}";
    }
}

craft_ui::words! {
    /// The names of the edit event kinds the draw stories' last-event lines show, English by
    /// default.
    #[non_exhaustive]
    pub(crate) struct EventKindLabels {
        begin: "drawStory.eventBegin" = "Begin";
        update: "drawStory.eventUpdate" = "Update";
        commit: "drawStory.eventCommit" = "Commit";
        cancel: "drawStory.eventCancel" = "Cancel";
        set: "drawStory.eventSet" = "Set";
    }
}

impl EventKindLabels {
    /// The word for the kind of `event`; `None` for a kind this story has no word for.
    pub(crate) fn kind<T>(&self, event: &EditEvent<T>) -> Option<String> {
        match event {
            EditEvent::Begin { .. } => Some(self.begin.clone()),
            EditEvent::Update { .. } => Some(self.update.clone()),
            EditEvent::Commit { .. } => Some(self.commit.clone()),
            EditEvent::Cancel { .. } => Some(self.cancel.clone()),
            EditEvent::Set { .. } => Some(self.set.clone()),
            _ => None,
        }
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "composites/stroke-editor/panel",
            Level::Composites,
            "StrokeEditor",
            "Panel",
            PanelArgs {
                closed: true,
                dashed: false,
                mixed: false,
                sections: SectionsArg::Panel,
                unit: UnitArg::Widths,
                stroke_width: 3.0,
                width: 272.0,
            },
            show_panel
        ),
        typed_story!(
            "composites/stroke-button/popover",
            Level::Composites,
            "StrokeButton",
            "Popover",
            PopoverArgs { stroke: true, closed: true },
            show_popover
        ),
        typed_story!(
            "composites/stroke-editor/choice-rows",
            Level::Composites,
            "StrokeEditor",
            "Choice rows",
            RowsArgs { mixed: false, width: 272.0 },
            show_rows
        ),
    ]
}

// --- shared parts ---

/// A style that differs from `base` in every field the editor edits, for a mixed selection.
fn other_than(base: &StrokeStyle) -> StrokeStyle {
    StrokeStyle {
        align: StrokeAlign::Inside,
        cap: LineCap::Square,
        join: LineJoin::Bevel,
        miter_limit: base.miter_limit / 2.0,
        dashes: StrokeStyle::DASHED.to_vec(),
        dash_offset: 3.0,
        opacity: base.opacity / 2.0,
        ..base.clone()
    }
}

/// Clears the mixed flags the edit of `field` sets for the whole selection.
fn settle(mixed: &mut StrokeMixed, field: StrokeField) {
    match field {
        StrokeField::Preset => {
            mixed.dashes = false;
            mixed.cap = false;
        }
        StrokeField::Align => mixed.align = false,
        StrokeField::Cap => mixed.cap = false,
        StrokeField::Join => mixed.join = false,
        StrokeField::MiterLimit => mixed.miter_limit = false,
        StrokeField::Opacity => mixed.opacity = false,
        StrokeField::Dashed | StrokeField::Dash(_) | StrokeField::AddPair | StrokeField::RemovePair => mixed.dashes = false,
        StrokeField::DashOffset => mixed.dash_offset = false,
    }
}

/// The value an event carries, if it has one.
fn value_of(event: Option<&EditEvent<StrokeStyle>>) -> Option<&StrokeStyle> {
    event.and_then(|e| e.value())
}

/// The last-event line of a story.
fn last_event(ui: &mut Ui, words: &StoryLabels, last: &Option<String>) {
    let text = match last {
        Some(event) => craft_i18n::fmt(&words.last_event, &[("event", event)]),
        None => words.no_event.clone(),
    };
    ui.add(Text::new(text).content(Content::Muted));
}

// --- panel ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SectionsArg {
    Panel,
    Popover,
}

impl SectionsArg {
    fn sections(self) -> StrokeSections {
        match self {
            SectionsArg::Panel => StrokeSections::PANEL,
            SectionsArg::Popover => StrokeSections::POPOVER,
        }
    }
}

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum UnitArg {
    Widths,
    Document,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PanelArgs {
    /// A closed path enables Alignment.
    closed: bool,
    dashed: bool,
    /// The selection holds two strokes that differ in every field.
    mixed: bool,
    sections: SectionsArg,
    unit: UnitArg,
    #[arg(name = "stroke width", range = 0.5..=24.0)]
    stroke_width: f32,
    #[arg(range = 160.0..=480.0)]
    width: f32,
}

/// What the panel story keeps: the stroke, which fields are mixed, the `dashed` arg last applied
/// and the last event.
#[derive(Clone, Debug)]
struct PanelState {
    style: StrokeStyle,
    mixed: StrokeMixed,
    dashed: bool,
    last: Option<String>,
}

impl PanelState {
    fn new() -> Self {
        let style = StrokeStyle { cap: LineCap::Round, join: LineJoin::Round, ..StrokeStyle::default() };
        let other = other_than(&style);
        let mixed = StrokeMixed::of([&style, &other]).map(|(_, mixed)| mixed).unwrap_or_default();
        Self { style, mixed, dashed: false, last: None }
    }
}

/// The editor as a panel hosts it: edits go to the stroke, and the first edit of a mixed field
/// settles it.
fn show_panel(ui: &mut Ui, args: &mut PanelArgs) {
    let words = StoryLabels::of(ui);
    let kept = shared(ui, "panel", PanelState::new);
    let mut state = lock(&kept);
    state.style.width = args.stroke_width;
    state.style.dash_unit = match args.unit {
        UnitArg::Widths => DashUnit::Widths,
        UnitArg::Document => DashUnit::Document,
    };
    if args.dashed != state.dashed {
        state.style = state.style.with_line_style(if args.dashed { LineStyle::Dashed } else { LineStyle::Solid });
        state.dashed = args.dashed;
    }
    let numbers = LocaleNumbers::of(ui);
    let tokens = Tokens::of(ui);
    let look = StrokeEditorStyle { width: args.width, ..StrokeEditor::style(&tokens) };
    let mixed = if args.mixed { state.mixed } else { StrokeMixed::default() };
    let response = StrokeEditor::new("stroke-editor-panel", &state.style, &numbers)
        .sections(args.sections.sections())
        .closed(args.closed)
        .mixed(&mixed)
        .look(look)
        .show(ui);
    if let Some(value) = value_of(response.event.as_ref()) {
        state.style = value.clone();
        args.dashed = !state.style.dashes.is_empty();
        state.dashed = args.dashed;
    }
    if let (Some(event), Some(field)) = (&response.event, response.field) {
        settle(&mut state.mixed, field);
        state.last = EventKindLabels::of(ui).kind(event);
    }
    ui.horizontal(|ui| {
        if ui.add(Button::new(words.reset.as_str())).clicked() {
            *state = PanelState::new();
            args.dashed = false;
        }
        last_event(ui, &words, &state.last);
    });
}

// --- popover ---

#[derive(StoryArgs, Debug, PartialEq)]
struct PopoverArgs {
    /// Off shows "No Color" on the opener; picking a preset then turns the stroke on.
    stroke: bool,
    closed: bool,
}

#[derive(Clone, Debug)]
struct PopoverState {
    style: StrokeStyle,
    open: bool,
    last: Option<String>,
}

/// The opener of an options bar and its popover.
fn show_popover(ui: &mut Ui, args: &mut PopoverArgs) {
    let words = StoryLabels::of(ui);
    let kept = shared(ui, "popover", || PopoverState { style: StrokeStyle::default(), open: false, last: None });
    let mut guard = lock(&kept);
    let state = &mut *guard;
    let numbers = LocaleNumbers::of(ui);
    let response =
        StrokeButton::new("stroke-editor-popover", &state.style, &numbers).enabled(args.stroke).closed(args.closed).show(ui, &mut state.open);
    if let Some(value) = value_of(response.event.as_ref()) {
        state.style = value.clone();
    }
    if let (Some(event), Some(field)) = (&response.event, response.field) {
        if field == StrokeField::Preset {
            args.stroke = true;
        }
        state.last = EventKindLabels::of(ui).kind(event);
    }
    last_event(ui, &words, &state.last);
}

// --- choice rows ---

#[derive(StoryArgs, Debug, PartialEq)]
struct RowsArgs {
    /// The selection's profiles differ.
    mixed: bool,
    #[arg(range = 160.0..=480.0)]
    width: f32,
}

#[derive(Clone, Debug)]
struct RowsState {
    style: StrokeStyle,
    start: String,
    end: String,
    profile: String,
    last: Option<String>,
}

const START: &str = "arrow-start";
const END: &str = "arrow-end";
const PROFILE: &str = "profile";

/// The arrowhead `id` at one end of the line through `rect`, `pointing` outwards.
fn paint_head(painter: &Painter, rect: Rect, id: &str, at_end: bool, color: Color32, line: f32) {
    let y = rect.center().y;
    let size = rect.height() * 0.35;
    let dir = if at_end { 1.0 } else { -1.0 };
    let tip = Pos2::new(if at_end { rect.right() } else { rect.left() }, y);
    painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(line, color));
    let back = |d: f32| Pos2::new(tip.x - dir * d, y);
    match id {
        "arrow" => {
            painter.add(Shape::convex_polygon(
                vec![tip, back(size * 2.0) + vec2(0.0, -size), back(size * 2.0) + vec2(0.0, size)],
                color,
                Stroke::NONE,
            ));
        }
        "circle" => {
            painter.circle_filled(back(size), size, color);
        }
        "square" => {
            painter.rect_filled(Rect::from_center_size(back(size), vec2(size * 2.0, size * 2.0)), 0.0, color);
        }
        "bar" => {
            painter.line_segment([tip + vec2(0.0, -size), tip + vec2(0.0, size)], Stroke::new(line, color));
        }
        _ => {}
    }
}

/// The width profile `id` along the line through `rect`: half the line's thickness at each point.
fn paint_profile(painter: &Painter, rect: Rect, id: &str, color: Color32) {
    const STEPS: usize = 16;
    let half = rect.height() * 0.3;
    let at = |t: f32| match id {
        "taper" => 1.0 - t,
        "bulge" => (t * std::f32::consts::PI).sin(),
        _ => 0.5,
    };
    let point = |t: f32, side: f32| Pos2::new(rect.left() + rect.width() * t, rect.center().y + side * half * at(t));
    let steps = (0..=STEPS).map(|i| i as f32 / STEPS as f32);
    let mut outline: Vec<Pos2> = steps.clone().map(|t| point(t, -1.0)).collect();
    outline.extend(steps.rev().map(|t| point(t, 1.0)));
    painter.add(Shape::convex_polygon(outline, color, Stroke::NONE));
}

/// The editor with the app's rows: arrowheads at the start and end and a width profile.
fn show_rows(ui: &mut Ui, args: &mut RowsArgs) {
    let words = StoryLabels::of(ui);
    let kept = shared(ui, "rows", || RowsState {
        style: StrokeStyle::default(),
        start: "none".into(),
        end: "arrow".into(),
        profile: "uniform".into(),
        last: None,
    });
    let mut state = lock(&kept);
    let numbers = LocaleNumbers::of(ui);
    let tokens = Tokens::of(ui);
    let look = StrokeEditorStyle { width: args.width, ..StrokeEditor::style(&tokens) };
    let color = tokens.color.text.normal.to_egui();
    let line = look.sample_stroke;
    let own = StrokeStoryLabels::of(ui);
    let none = StrokeChoice::new("none", words.none.as_str());
    let heads = [
        none.clone(),
        StrokeChoice::new("arrow", own.arrow.as_str()),
        StrokeChoice::new("circle", own.circle.as_str()),
        StrokeChoice::new("square", own.square.as_str()),
        StrokeChoice::new("bar", own.bar.as_str()),
    ];
    let profiles = [
        StrokeChoice::new("uniform", own.uniform.as_str()),
        StrokeChoice::new("taper", own.taper.as_str()),
        StrokeChoice::new("bulge", own.bulge.as_str()),
    ];
    let picked = (state.start.clone(), state.end.clone(), state.profile.clone());
    let profile = if args.mixed { MixedValue::Mixed } else { MixedValue::Same(picked.2.as_str()) };
    let rows = [
        StrokeChoiceRow::new(START, words.start.as_str(), &heads)
            .selected(MixedValue::Same(picked.0.as_str()))
            .paint(move |painter, rect, choice| paint_head(painter, rect, &choice.id, false, color, line)),
        StrokeChoiceRow::new(END, words.end.as_str(), &heads)
            .selected(MixedValue::Same(picked.1.as_str()))
            .paint(move |painter, rect, choice| paint_head(painter, rect, &choice.id, true, color, line)),
        StrokeChoiceRow::new(PROFILE, own.profile.as_str(), &profiles)
            .selected(profile)
            .paint(move |painter, rect, choice| paint_profile(painter, rect, &choice.id, color)),
    ];
    let response = StrokeEditor::new("stroke-editor-rows", &state.style, &numbers).look(look).rows(&rows).show(ui);
    if let Some(value) = value_of(response.event.as_ref()) {
        state.style = value.clone();
    }
    if let Some((row, choice)) = response.choice {
        let row_name = match row.as_str() {
            START => words.start.as_str(),
            END => words.end.as_str(),
            _ => own.profile.as_str(),
        };
        let choice_name = heads.iter().chain(&profiles).find(|c| c.id == choice).map_or(choice.as_str(), |c| c.label.as_str());
        state.last = Some(craft_i18n::fmt(&own.row_choice, &[("row", row_name), ("choice", choice_name)]));
        match row.as_str() {
            START => state.start = choice,
            END => state.end = choice,
            _ => {
                state.profile = choice;
                args.mixed = false;
            }
        }
    }
    last_event(ui, &words, &state.last);
}
