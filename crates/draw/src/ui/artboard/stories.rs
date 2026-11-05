//! Stories of the Artboard tool, bar and Properties section. Fake boards on a `Pasteboard` stand for the document; the tool, the bar and the section
//! run on them as an app runs them. Sample words are story words; the plain strings are the status
//! lines.
//!
//! - `primitives/artboard-tool/boards`: the boards on the pasteboard (`PageFrame::active` marks the selected
//!   one) with the tool in a `ToolHost`: draw, move, resize, the "+" buttons (Photoshop rules), the
//!   Illustrator rules (multiple selection, ⌥ copies, Esc leaves), a locked board. The pasteboard's
//!   page names select a board.
//! - `patterns/artboard-bar/bar`: the bar with and without a selected board, over a preset list.
//! - `patterns/artboard-properties/section`: the Properties section: preset, W and H, X and Y, background;
//!   Other… is answered by the story.

use egui::{Color32, Ui};
use kurbo::{Point, Rect, Size, Vec2};

use super::{
    ArtboardBackground, ArtboardBar, ArtboardBarOutput, ArtboardEvent, ArtboardItem, ArtboardPreset, ArtboardProperties, ArtboardRules, ArtboardTool,
};
use crate::ui::add_beside::AddSide;
use craft_ui::canvas::{PageFrame, Pasteboard, ToolHost};
use craft_ui::controls::Button;
use craft_ui::edit::EditEvent;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::patterns::OptionsBar;
use craft_ui::primitives::{Content, Text};
use craft_ui::story::{Level, Story, StoryArgs, StoryChoice, StoryLabels, Surface, lock, reset_on, shared};
use craft_ui::testing::canvas_story::{ViewArgs, document_at};
use craft_ui::theme::{Tokens, TokensOf as _};
use craft_ui::typed_story;

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["ArtboardTool", "ArtboardBar", "ArtboardProperties"];

craft_ui::words! {
    /// The captions of the artboard stories, English by default.
    #[non_exhaustive]
    pub(crate) struct ArtboardStoryLabels {
        bar: "artboardStory.bar" = "Artboard options";
        board: "artboardStory.board" = "Artboard {number}";
        letter: "artboardStory.letter" = "Letter";
        tablet: "artboardStory.tablet" = "Tablet";
        phone: "artboardStory.phone" = "Phone";
        added: "artboardStory.added" = "Added {name}";
        opened: "artboardStory.opened" = "Options of {name}";
        fit: "artboardStory.fit" = "Fit to the object at {x}, {y}";
        selected: "artboardStory.selected" = "Selected: {names}";
        none: "artboardStory.none" = "No artboard selected";
        size: "artboardStory.size" = "Size {width} × {height}";
    }
}

/// The story page, the boards' size and the gap the "+" buttons leave (story-local: no token sizes
/// a document).
const BOARD: Size = Size::new(180.0, 240.0);
const GAP: f64 = 60.0;

/// The presets' sizes in pixels (story-local sample data).
const PRESET_SIZES: [(f64, f64); 3] = [(612.0, 792.0), (768.0, 1024.0), (390.0, 844.0)];

pub(crate) fn stories() -> Vec<Story> {
    vec![
        typed_story!(
            "primitives/artboard-tool/boards",
            Level::Primitives,
            "ArtboardTool",
            "Tool",
            ToolArgs { view: ViewArgs { zoom: 75.0, ..ViewArgs::default() }, rules: RulesArg::Photoshop, locked_board: true },
            show_tool
        )
        .on(Surface::Canvas),
        typed_story!("patterns/artboard-bar/bar", Level::Patterns, "ArtboardBar", "Bar", BarArgs { selection: true, wide: true }, show_bar),
        typed_story!(
            "patterns/artboard-properties/section",
            Level::Patterns,
            "ArtboardProperties",
            "Properties",
            PropertiesArgs { background: BackgroundArg::White, x: 0.0, y: 0.0 },
            show_properties
        ),
    ]
}

// --- shared parts ---

fn presets(words: &ArtboardStoryLabels) -> Vec<ArtboardPreset> {
    [&words.letter, &words.tablet, &words.phone]
        .into_iter()
        .zip(PRESET_SIZES)
        .map(|(name, (width, height))| ArtboardPreset { name: name.clone(), size: Size::new(width, height) })
        .collect()
}

fn board_name(words: &ArtboardStoryLabels, number: usize) -> String {
    craft_i18n::fmt(&words.board, &[("number", &number.to_string())])
}

fn fill(background: ArtboardBackground) -> Option<Color32> {
    match background {
        ArtboardBackground::White => Some(Color32::WHITE),
        ArtboardBackground::Black => Some(Color32::BLACK),
        ArtboardBackground::Transparent => None,
        ArtboardBackground::Custom(color) => Some(Color32::from(color)),
    }
}

/// A muted status line.
fn status(ui: &mut Ui, text: String) {
    ui.add(Text::new(text).content(Content::Muted));
}

// --- tool ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum RulesArg {
    Photoshop,
    Illustrator,
}

impl RulesArg {
    fn rules(self) -> ArtboardRules {
        match self {
            RulesArg::Photoshop => ArtboardRules::PHOTOSHOP,
            RulesArg::Illustrator => ArtboardRules::ILLUSTRATOR,
        }
    }
}

#[derive(StoryArgs, Debug, PartialEq)]
struct ToolArgs {
    #[arg(flatten)]
    view: ViewArgs,
    /// Photoshop shows the "+" buttons round the active board; Illustrator selects several boards,
    /// copies with ⌥, leaves with Esc and fits a board to an object.
    rules: RulesArg,
    /// Locks the third board: it has no handles and refuses a move or resize.
    #[arg(name = "locked board")]
    locked_board: bool,
}

/// What the tool story keeps: the tool, the boards, the selection and the last result.
struct ToolKept {
    tool: ArtboardTool,
    boards: Vec<ArtboardItem>,
    selected: Vec<usize>,
    note: String,
}

impl ToolKept {
    fn new(words: &ArtboardStoryLabels) -> Self {
        let board = |number: usize, locked: bool| {
            let x = (number - 1) as f64 * (BOARD.width + GAP);
            ArtboardItem {
                name: board_name(words, number),
                rect: Rect::new(x, 0.0, x + BOARD.width, BOARD.height),
                locked,
                background: ArtboardBackground::White,
            }
        };
        Self { tool: ArtboardTool::new(), boards: vec![board(1, false), board(2, false), board(3, true)], selected: vec![0], note: String::new() }
    }

    /// Adds a board beside `index` on `side`, the same size.
    fn add_beside(&mut self, index: usize, side: AddSide, words: &ArtboardStoryLabels) {
        let Some(from) = self.boards.get(index).map(|b| b.rect.abs()) else { return };
        let by = match side {
            AddSide::Top => Vec2::new(0.0, -(from.height() + GAP)),
            AddSide::Right => Vec2::new(from.width() + GAP, 0.0),
            AddSide::Bottom => Vec2::new(0.0, from.height() + GAP),
            AddSide::Left => Vec2::new(-(from.width() + GAP), 0.0),
        };
        self.push(from + by, words);
    }

    fn push(&mut self, rect: Rect, words: &ArtboardStoryLabels) {
        let name = board_name(words, self.boards.len() + 1);
        self.note = craft_i18n::fmt(&words.added, &[("name", &name)]);
        self.boards.push(ArtboardItem { name, rect, locked: false, background: ArtboardBackground::White });
        self.selected = vec![self.boards.len() - 1];
    }

    fn done<T: Copy>(event: &EditEvent<T>) -> Option<T> {
        match event {
            EditEvent::Commit { value, changed: true, .. } | EditEvent::Set { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Answers one tool event as the app does.
    fn apply(&mut self, event: ArtboardEvent, words: &ArtboardStoryLabels, numbers: &LocaleNumbers) {
        match event {
            ArtboardEvent::Draw(event) => {
                if let Some(rect) = Self::done(&event) {
                    self.push(rect.abs(), words);
                }
            }
            ArtboardEvent::Move { indices, event, copy } => {
                if let Some(by) = Self::done(&event) {
                    for index in indices {
                        let Some(from) = self.boards.get(index).map(|b| b.rect) else { continue };
                        if copy {
                            self.push(from + by, words);
                        } else if let Some(board) = self.boards.get_mut(index) {
                            board.rect = from + by;
                        }
                    }
                }
            }
            ArtboardEvent::Resize { index, event } => {
                if let (Some(rect), Some(board)) = (Self::done(&event), self.boards.get_mut(index)) {
                    board.rect = rect.abs();
                }
            }
            ArtboardEvent::AddBeside { index, side, .. } => self.add_beside(index, side, words),
            ArtboardEvent::Select { indices, .. } => self.selected = indices,
            ArtboardEvent::Delete { mut indices } => {
                indices.sort_unstable_by(|a, b| b.cmp(a));
                for index in indices {
                    if index < self.boards.len() {
                        self.boards.remove(index);
                    }
                }
                self.selected.clear();
            }
            ArtboardEvent::OpenOptions { index } => {
                let name = self.boards.get(index).map(|b| b.name.clone()).unwrap_or_default();
                self.note = craft_i18n::fmt(&words.opened, &[("name", &name)]);
            }
            ArtboardEvent::FitObject { at } => {
                self.note = craft_i18n::fmt(&words.fit, &[("x", &numbers.format(at.x, 0..=0)), ("y", &numbers.format(at.y, 0..=0))]);
            }
            ArtboardEvent::Leave => self.selected.clear(),
        }
    }
}

fn show_tool(ui: &mut Ui, args: &mut ToolArgs) {
    let words = ArtboardStoryLabels::of(ui);
    let story = StoryLabels::of(ui);
    let state = shared(ui, "tool", || ToolKept::new(&words));
    let mut kept = lock(&state);
    let kept = &mut *kept;
    if let Some(third) = kept.boards.get_mut(2) {
        third.locked = args.locked_board;
    }
    let bounds = kept.boards.iter().map(|b| b.rect.abs()).reduce(|a, b| a.union(b)).unwrap_or(Rect::from_origin_size(Point::ORIGIN, BOARD));
    let canvas = document_at(ui, args.view, &story.document, 0.0, bounds.center());

    let pages: Vec<PageFrame> = kept
        .boards
        .iter()
        .enumerate()
        .map(|(i, board)| {
            let page = PageFrame::new(board.rect.abs()).name(board.name.clone()).active(kept.selected.contains(&i));
            fill(board.background).map_or(page.clone(), |color| page.fill(color))
        })
        .collect();
    let sensed = Pasteboard::new("artboard-pasteboard", &pages).canvas(canvas.id).show(ui, &canvas.view);
    if let Some(index) = sensed.name_clicked {
        kept.selected = vec![index];
    }

    let numbers = LocaleNumbers::of(ui);
    kept.tool.items(&kept.boards).selected(&kept.selected).rules(args.rules.rules());
    let _ = ToolHost::new(canvas.id).show(ui, &canvas.view, &canvas.response, &mut kept.tool);
    for event in kept.tool.take_events() {
        kept.apply(event, &words, &numbers);
    }

    ui.horizontal(|ui| {
        if ui.add(Button::new(story.reset.as_str())).clicked() {
            *kept = ToolKept::new(&words);
        }
        let names: Vec<&str> = kept.selected.iter().filter_map(|&i| kept.boards.get(i)).map(|b| b.name.as_str()).collect();
        let selection = if names.is_empty() { words.none.clone() } else { craft_i18n::fmt(&words.selected, &[("names", &names.join(", "))]) };
        status(ui, if kept.note.is_empty() { selection } else { format!("{selection} · {}", kept.note) });
    });
}

// --- bar ---

#[derive(StoryArgs, Debug, PartialEq)]
struct BarArgs {
    /// A board is selected; without one the size cells are disabled.
    selection: bool,
    /// The selected board is wider than tall (Make Portrait), else taller (Make Landscape).
    wide: bool,
}

/// The board the bar story edits, and how many Add New Artboard has added.
#[derive(Clone, Debug)]
struct BarKept {
    size: Size,
    added: usize,
}

fn show_bar(ui: &mut Ui, args: &mut BarArgs) {
    let words = ArtboardStoryLabels::of(ui);
    let sample = if args.wide { Size::new(BOARD.height, BOARD.width) } else { BOARD };
    reset_on(ui, "bar", &args.wide);
    let state = shared(ui, "bar", || BarKept { size: sample, added: 0 });
    let mut kept = lock(&state);
    let list = presets(&words);
    let board = ArtboardItem {
        name: board_name(&words, 1),
        rect: Rect::from_origin_size(Point::ORIGIN, kept.size),
        locked: false,
        background: ArtboardBackground::White,
    };
    let mut out = ArtboardBarOutput::default();
    OptionsBar::new("artboard-bar-options")
        .label(words.bar.clone())
        .show(ui, |slots| ArtboardBar::new("artboard-bar", args.selection.then_some(&board), &list).output(&mut out).add_to(slots));
    if let Some(size) = out.size.as_ref().and_then(EditEvent::value).copied() {
        kept.size = size;
    }
    kept.added += usize::from(out.add_new);
    let numbers = LocaleNumbers::of(ui);
    let size =
        craft_i18n::fmt(&words.size, &[("width", &numbers.format(kept.size.width, 0..=0)), ("height", &numbers.format(kept.size.height, 0..=0))]);
    status(
        ui,
        if kept.added > 0 { format!("{size} · {}", craft_i18n::fmt(&words.added, &[("name", &board_name(&words, 1 + kept.added))])) } else { size },
    );
}

// --- properties ---

#[derive(StoryChoice, Clone, Copy, Debug, PartialEq)]
enum BackgroundArg {
    White,
    Black,
    Transparent,
    Custom,
}

#[derive(StoryArgs, Debug, PartialEq)]
struct PropertiesArgs {
    /// The background the board starts with.
    background: BackgroundArg,
    #[arg(range = -500.0..=500.0)]
    x: f64,
    #[arg(range = -500.0..=500.0)]
    y: f64,
}

fn show_properties(ui: &mut Ui, args: &mut PropertiesArgs) {
    let words = ArtboardStoryLabels::of(ui);
    let list = presets(&words);
    let accent = craft_theme::Srgba::from(Tokens::of(ui).color.accent.solid);
    let background = match args.background {
        BackgroundArg::White => ArtboardBackground::White,
        BackgroundArg::Black => ArtboardBackground::Black,
        BackgroundArg::Transparent => ArtboardBackground::Transparent,
        BackgroundArg::Custom => ArtboardBackground::Custom(accent),
    };
    let seed = (args.background, args.x.to_bits(), args.y.to_bits());
    let fresh =
        || ArtboardItem { name: board_name(&words, 1), rect: Rect::from_origin_size(Point::new(args.x, args.y), BOARD), locked: false, background };
    reset_on(ui, "properties", &seed);
    let state = shared(ui, "properties", fresh);
    let mut kept = lock(&state);
    let out = ArtboardProperties::new("artboard-properties", &kept, &list).show(ui);
    if let Some(rect) = out.rect.as_ref().and_then(ToolKept::done) {
        kept.rect = rect;
    }
    if let Some(background) = out.background {
        kept.background = background;
    }
    if out.pick_color.is_some() {
        kept.background = ArtboardBackground::Custom(accent);
    }
}
