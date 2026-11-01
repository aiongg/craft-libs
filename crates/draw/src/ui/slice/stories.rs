//! Stories of the Slice tools (task ST-SH1; task RG2 registers them). A fake document carries user,
//! auto and layer slices, and the tool, its options bar and the dialogs run on it as an app runs
//! them. Sample words are story words; the plain strings are the status lines.
//!
//! - `primitives/slice-tool/slices`: Slice and Select modes in a `ToolHost` with the options bar below: draw a
//!   slice, move and resize a user slice (an auto or layer slice that moves becomes a user slice),
//!   Promote, Delete, Hide Auto Slices, and a lock that refuses edits. Slice Options and Divide
//!   are the dialogs' own stories; here those commands only report.
//! - `patterns/slice-options/bar`: the bar alone in both modes, for each kind of selected slice.
//! - `patterns/slice-options/dialog`: Image, No Image and HTML Text; the Background colour is answered
//!   by the story.
//! - `patterns/divide-slices/dialog`: rows and columns by count or by size.
//!
//! RG2 registers `stories()` and the components `SliceTool`, `SliceBar`, `SliceOptionsDialog` and
//! `DivideSlicesDialog`; RG1 adds [`SliceStoryLabels`] to `WORDS`.

use egui::Ui;
use kurbo::Rect;

use super::{
    DivideBy, DivideSlices, DivideSlicesDialog, SliceBar, SliceCommand, SliceEvent, SliceItem, SliceKind, SliceMode, SliceOptions,
    SliceOptionsDialog, SliceOrigin, SliceRefusal, SliceRules, SliceTool, SliceToolLabels, SliceToolSettings,
};
use craft_theme::Srgba;
use craft_ui::canvas::{Constraint, ToolHost};
use craft_ui::controls::Button;
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface, drive, lock, shared};
use craft_ui::testing::canvas_story::{DOC, ViewArgs, document};
use craft_ui::theme::{Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["SliceTool", "SliceBar", "SliceOptionsDialog", "DivideSlicesDialog"];

craft_ui::words! {
    /// The captions of the slice stories, English by default.
    #[non_exhaustive]
    pub(crate) struct SliceStoryLabels {
        bar: "sliceStory.bar" = "Slice options";
        selected: "sliceStory.selected" = "Selected: slice {number}";
        none: "sliceStory.none" = "No slice selected";
        command: "sliceStory.command" = "Command: {command}";
    }
}

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/slice-tool/slices",
            Level::Primitives,
            "SliceTool",
            "Tool",
            ToolArgs { view: ViewArgs::default(), mode: ModeArg::Slice, rules: RulesArg::Photoshop, auto: true, locked: false },
            show_tool
        )
        .on(Surface::Canvas),
        typed_story!(
            "patterns/slice-options/bar",
            Level::Patterns,
            "SliceBar",
            "Bar",
            BarArgs { mode: ModeArg::Slice, selected: SelectedArg::None, locked: false },
            show_bar
        ),
        typed_story!(
            "patterns/slice-options/dialog",
            Level::Patterns,
            "SliceOptionsDialog",
            "Slice Options",
            OptionsArgs { open: true, kind: KindArg::Image, background_row: true },
            show_options
        ),
        typed_story!(
            "patterns/divide-slices/dialog",
            Level::Patterns,
            "DivideSlicesDialog",
            "Divide Slice",
            DivideArgs { open: true, rows: AxisArg::Count, rows_value: 2, columns: AxisArg::Off, columns_value: 3 },
            show_divide
        ),
    ]
}

// --- shared parts ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum ModeArg {
    Slice,
    Select,
}

impl ModeArg {
    fn mode(self) -> SliceMode {
        match self {
            ModeArg::Slice => SliceMode::Slice,
            ModeArg::Select => SliceMode::Select,
        }
    }
}

/// Photoshop's slice rules or Illustrator's (modifier keys and multiple selection).
#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RulesArg {
    Photoshop,
    Illustrator,
}

impl RulesArg {
    fn rules(self) -> SliceRules {
        match self {
            RulesArg::Photoshop => SliceRules::PHOTOSHOP,
            RulesArg::Illustrator => SliceRules::ILLUSTRATOR,
        }
    }
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

/// The slices of the fake document. The rectangles are a demo layout on the 640 × 400 story page
/// (story-local: no token sizes a document).
fn sample_slices() -> Vec<SliceItem> {
    let item = |number, origin, rect| SliceItem { number, id: (origin != SliceOrigin::Auto).then_some(number as u32), rect, origin };
    vec![
        item(1, SliceOrigin::Auto, Rect::new(0.0, 0.0, DOC.x1, 60.0)),
        item(2, SliceOrigin::Auto, Rect::new(0.0, 60.0, 80.0, DOC.y1)),
        item(3, SliceOrigin::User, Rect::new(80.0, 60.0, 300.0, 200.0)),
        item(4, SliceOrigin::Auto, Rect::new(300.0, 60.0, DOC.x1, 120.0)),
        item(5, SliceOrigin::Layer, Rect::new(360.0, 120.0, 560.0, 300.0)),
        item(6, SliceOrigin::Auto, Rect::new(560.0, 120.0, DOC.x1, 300.0)),
        item(7, SliceOrigin::Auto, Rect::new(80.0, 200.0, 360.0, DOC.y1)),
        item(8, SliceOrigin::Auto, Rect::new(360.0, 300.0, DOC.x1, DOC.y1)),
    ]
}

// --- tool ---

#[derive(StoryArgs, Debug, PartialEq)]
struct ToolArgs {
    #[arg(flatten)]
    view: ViewArgs,
    mode: ModeArg,
    rules: RulesArg,
    /// Shows the auto slices (dashed); Hide Auto Slices in Select mode turns it off too.
    auto: bool,
    /// Refuses every edit, as View › Lock Slices does.
    locked: bool,
}

/// What the tool story keeps: the tool, the slices and the selection, the tool settings, the
/// last command and the last refusal.
struct ToolKept {
    tool: SliceTool,
    items: Vec<SliceItem>,
    selected: Option<usize>,
    settings: SliceToolSettings,
    note: String,
}

impl ToolKept {
    fn new() -> Self {
        Self {
            tool: SliceTool::new(),
            items: sample_slices(),
            selected: None,
            settings: SliceToolSettings::new(Constraint::Free),
            note: String::new(),
        }
    }

    fn next_number(&self) -> usize {
        self.items.iter().map(|i| i.number).max().unwrap_or(0) + 1
    }

    /// The rectangle a finished gesture ends with, if it changed anything.
    fn done(event: &EditEvent<Rect>) -> Option<Rect> {
        match event {
            EditEvent::Commit { value, changed: true, .. } | EditEvent::Set { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Sets a slice's rectangle; a moved auto or layer slice becomes a user slice.
    fn place(&mut self, number: usize, rect: Rect) {
        if let Some(item) = self.items.iter_mut().find(|i| i.number == number) {
            item.rect = rect;
            item.origin = SliceOrigin::User;
        }
    }

    /// Answers one tool event as the app does.
    fn apply(&mut self, event: SliceEvent, words: &SliceStoryLabels, tool_words: &SliceToolLabels) {
        match event {
            SliceEvent::Draw(event) => {
                if let Some(rect) = Self::done(&event) {
                    let number = self.next_number();
                    self.items.push(SliceItem { number, id: Some(number as u32), rect, origin: SliceOrigin::User });
                    self.selected = Some(number);
                }
            }
            SliceEvent::Move { number, event } | SliceEvent::Resize { number, event } => {
                if let Some(rect) = Self::done(&event) {
                    self.place(number, rect);
                }
            }
            SliceEvent::Select { number, .. } => self.selected = number,
            SliceEvent::Command(command) => {
                self.note = craft_i18n::fmt(&words.command, &[("command", &format!("{command:?}"))]);
                match command {
                    SliceCommand::Delete => {
                        let user = |i: &SliceItem| i.origin == SliceOrigin::User && Some(i.number) == self.selected;
                        self.items.retain(|i| !user(i));
                        self.selected = None;
                    }
                    SliceCommand::Promote => {
                        if let Some(item) = self.items.iter_mut().find(|i| Some(i.number) == self.selected) {
                            item.origin = SliceOrigin::User;
                        }
                    }
                    SliceCommand::HideAuto(hide) => self.settings.hide_auto = hide,
                    SliceCommand::Options | SliceCommand::Divide | SliceCommand::FromGuides => {}
                }
            }
            SliceEvent::Refused(SliceRefusal::Locked) => self.note = tool_words.locked.clone(),
        }
    }

    fn origin(&self) -> Option<SliceOrigin> {
        self.items.iter().find(|i| Some(i.number) == self.selected).map(|i| i.origin)
    }
}

fn show_tool(ui: &mut Ui, args: &mut ToolArgs) {
    let words = SliceStoryLabels::of(ui);
    let tool_words = SliceToolLabels::of(ui);
    let story = StoryLabels::of(ui);
    let canvas = document(ui, args.view, &story.document);
    let state = shared(ui, "tool", ToolKept::new);
    let mut kept = lock(&state);
    let kept = &mut *kept;
    kept.tool
        .mode(args.mode.mode())
        .items(&kept.items)
        .selected(kept.selected)
        .locked(args.locked)
        .rules(args.rules.rules())
        .settings(kept.settings)
        .hide_auto(!args.auto || kept.settings.hide_auto);
    let _ = ToolHost::new(canvas.id).document(DOC.size()).show(ui, &canvas.view, &canvas.response, &mut kept.tool);
    for event in kept.tool.take_events() {
        kept.apply(event, &words, &tool_words);
    }

    let mut shown = None;
    OptionsBar::new("slice-tool-options").label(words.bar.clone()).show(ui, |slots| {
        slots.body(|ui| {
            let bar = SliceBar::new("slice-tool-bar", args.mode.mode(), &kept.settings).selected(kept.origin()).locked(args.locked);
            shown = Some(bar.show(ui));
        });
    });
    if let Some(out) = shown {
        if let Some(event) = out.style
            && let Some(style) = event.value()
        {
            kept.settings.style = *style;
        }
        if let Some(command) = out.command {
            kept.apply(SliceEvent::Command(command), &words, &tool_words);
        }
    }
    ui.horizontal(|ui| {
        if ui.add(Button::new(story.reset.as_str())).clicked() {
            *kept = ToolKept::new();
        }
        let selection = match kept.selected {
            Some(number) => craft_i18n::fmt(&words.selected, &[("number", &number.to_string())]),
            None => words.none.clone(),
        };
        status(ui, if kept.note.is_empty() { selection } else { format!("{selection} · {}", kept.note) });
    });
}

// --- bar ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum SelectedArg {
    None,
    User,
    Auto,
    Layer,
}

impl SelectedArg {
    fn origin(self) -> Option<SliceOrigin> {
        match self {
            SelectedArg::None => None,
            SelectedArg::User => Some(SliceOrigin::User),
            SelectedArg::Auto => Some(SliceOrigin::Auto),
            SelectedArg::Layer => Some(SliceOrigin::Layer),
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    mode: ModeArg,
    /// The selected slice's origin, which enables Promote, Divide, Delete and Options.
    selected: SelectedArg,
    locked: bool,
}

/// The bar alone: its style and Hide Auto Slices change the settings kept here, a button reports.
fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let words = SliceStoryLabels::of(ui);
    let state = shared(ui, "bar", || (SliceToolSettings::new(Constraint::Free), String::new()));
    let mut kept = lock(&state);
    let (settings, note) = &mut *kept;
    let mut shown = None;
    OptionsBar::new("slice-bar-options").label(words.bar.clone()).show(ui, |slots| {
        slots.body(|ui| {
            shown = Some(SliceBar::new("slice-bar", args.mode.mode(), settings).selected(args.selected.origin()).locked(args.locked).show(ui));
        });
    });
    if let Some(out) = shown {
        if let Some(event) = out.style
            && let Some(style) = event.value()
        {
            settings.style = *style;
        }
        if let Some(command) = out.command {
            if let SliceCommand::HideAuto(hide) = command {
                settings.hide_auto = hide;
            }
            *note = craft_i18n::fmt(&words.command, &[("command", &format!("{command:?}"))]);
        }
    }
    if !note.is_empty() {
        status(ui, note.clone());
    }
}

// --- options dialog ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum KindArg {
    Image,
    #[choice(name = "No Image")]
    NoImage,
    #[choice(name = "HTML Text")]
    HtmlText,
}

impl KindArg {
    fn kind(self) -> SliceKind {
        match self {
            KindArg::Image => SliceKind::Image,
            KindArg::NoImage => SliceKind::NoImage,
            KindArg::HtmlText => SliceKind::HtmlText,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct OptionsArgs {
    open: bool,
    /// The type the slice has when the dialog opens.
    kind: KindArg,
    /// Shows the Background row, which asks the story for a colour.
    #[arg(name = "background row")]
    background_row: bool,
}

/// The colour the story's picker answers the Background row with.
fn picked_colour(ui: &Ui) -> Srgba {
    Srgba::from(Tokens::of(ui).color.accent.solid)
}

fn show_options(ui: &mut Ui, args: &mut OptionsArgs) {
    let story = StoryLabels::of(ui);
    let (kind, background_row) = (args.kind.kind(), args.background_row);
    let colour = picked_colour(ui);
    drive(
        ui,
        "slice-options",
        &mut args.open,
        (kind, background_row),
        || {
            let mut options = SliceOptions { kind, background_row, ..SliceOptions::default() };
            options.name = story.photo.clone();
            SliceOptionsDialog::new(options).kinds(&[SliceKind::Image, SliceKind::NoImage, SliceKind::HtmlText])
        },
        |dialog| {
            if let Some(request) = dialog.pick_color() {
                dialog.set_color(&request.key, colour);
            }
        },
        |options| {
            let value = format!("{:?} {}", options.kind, options.name);
            craft_i18n::fmt(&story.result_ok, &[("value", &value)])
        },
    );
}

// --- divide dialog ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum AxisArg {
    Off,
    Count,
    Size,
}

impl AxisArg {
    fn divide(self, value: u32) -> Option<DivideBy> {
        match self {
            AxisArg::Off => None,
            AxisArg::Count => Some(DivideBy::Count(value)),
            AxisArg::Size => Some(DivideBy::Size(value)),
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct DivideArgs {
    open: bool,
    /// Divides horizontally: off, into a count of slices, or into slices of a size in pixels.
    rows: AxisArg,
    #[arg(name = "rows value", range = 1..=1000)]
    rows_value: u32,
    columns: AxisArg,
    #[arg(name = "columns value", range = 1..=1000)]
    columns_value: u32,
}

fn show_divide(ui: &mut Ui, args: &mut DivideArgs) {
    let words = StoryLabels::of(ui);
    let divide = DivideSlices { rows: args.rows.divide(args.rows_value), columns: args.columns.divide(args.columns_value) };
    drive(
        ui,
        "divide-slices",
        &mut args.open,
        (args.rows, args.rows_value, args.columns, args.columns_value),
        || DivideSlicesDialog::new(divide),
        |_| {},
        |answer| craft_i18n::fmt(&words.result_ok, &[("value", &format!("{answer:?}"))]),
    );
}
