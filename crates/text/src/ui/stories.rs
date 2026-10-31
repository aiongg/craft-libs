//! Stories of the text editor, the Character and Paragraph controls, the styles panel and the
//! Check Spelling dialog. The editor draws a monospace text of its own over the canvas; the
//! other stories show the controls over sample values and apply the changes they report.

use std::ops::Range;
use std::sync::{Arc, Mutex, PoisonError};

use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasView, ToolHost};
use craft_ui::controls::Button;
use craft_ui::dialog::{DialogHost, DialogResult, StackDialog};
use craft_ui::primitives::Content;
use craft_ui::story::{Level, NoArgs, Story, StoryArgs};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _};
use craft_ui::typed_story;
use egui::epaint::text::FontId;
use egui::{Color32, Context, Event, Id, ImeEvent, Pos2, Sense, Ui, vec2};

use super::character::{
    BaselinePosition, Caps, CharacterChange, CharacterControls, CharacterValues, Kerning, Leading, ParagraphChange, ParagraphControls,
    ParagraphDirection, ParagraphValues, TextAlign,
};
use super::editor::{CaretLine, EditKind, TextEditor, TextEditorHost, TextEditorTool, TextSelection};
use super::spelling::{CheckSpellingDialog, SUGGESTION_COUNT, SpellingAction, SpellingItem};
use super::style_panel::{TextStyleAction, TextStyleKind, TextStyleRow, TextStylesPanel};
use crate::quotes::QuoteStyle;
use crate::spell::{Dictionary, UserWords};

#[derive(StoryArgs, Debug, PartialEq)]
struct EditorArgs {
    #[arg(name = "marked text")]
    marked: bool,
}

/// `patterns/text-editor/canvas`, `patterns/character/controls`, `patterns/character/paragraph`,
/// `patterns/text-styles/panel`, `patterns/check-spelling/dialog`.
pub fn stories() -> Vec<Story> {
    vec![
        typed_story!("patterns/text-editor/canvas", Level::Patterns, "Text editor", "Canvas", EditorArgs { marked: false }, text_editor),
        typed_story!("patterns/character/controls", Level::Patterns, "Character", "Controls", NoArgs {}, character_controls),
        typed_story!("patterns/character/paragraph", Level::Patterns, "Character", "Paragraph", NoArgs {}, paragraph_controls),
        typed_story!("patterns/text-styles/panel", Level::Patterns, "Text styles", "Panel", NoArgs {}, text_styles),
        typed_story!("patterns/check-spelling/dialog", Level::Patterns, "Check spelling", "Dialog", NoArgs {}, check_spelling),
    ]
}

/// A story's state, kept in egui memory under the story's own id.
fn state<T: Send + 'static>(ui: &Ui, name: &str, make: impl FnOnce() -> T) -> Arc<Mutex<T>> {
    let key = ui.scope_id().with(("text-story", name));
    ui.data_mut(|d| d.get_temp_mut_or_insert_with(key, || Arc::new(Mutex::new(make()))).clone())
}

// --- The editor ---

const CELL_WIDTH: f64 = 10.0;
const LINE_HEIGHT: f64 = 24.0;
const FONT_SIZE: f32 = 16.0;

/// A text of lines at `\n` in a monospace layout: char `c` of a line sits at x = 10 c, line `l`
/// at y = 24 l, in document units that are screen points at zoom 1.
struct Lines {
    text: String,
}

impl Lines {
    fn starts(&self) -> Vec<usize> {
        let mut starts = vec![0];
        starts.extend(self.text.chars().enumerate().filter(|(_, c)| *c == '\n').map(|(i, _)| i + 1));
        starts
    }

    fn chars(&self) -> usize {
        self.text.chars().count()
    }

    fn line_of(&self, offset: usize) -> usize {
        self.starts().iter().rposition(|&s| s <= offset).unwrap_or(0)
    }

    /// The pieces of `range`, one per line: (line, first column, end column).
    fn pieces(&self, range: Range<usize>) -> Vec<(usize, usize, usize)> {
        let starts = self.starts();
        (0..starts.len())
            .filter_map(|line| {
                let r = self.range_of(line);
                let (a, b) = (range.start.max(r.start), range.end.min(r.end));
                (a < b).then(|| (line, a - r.start, b - r.start))
            })
            .collect()
    }

    fn range_of(&self, line: usize) -> Range<usize> {
        let starts = self.starts();
        starts[line]..starts.get(line + 1).map_or(self.chars(), |next| next - 1)
    }
}

fn byte(text: &str, chars: usize) -> usize {
    text.char_indices().nth(chars).map_or(text.len(), |(i, _)| i)
}

impl TextEditorHost for Lines {
    fn text(&self) -> &str {
        &self.text
    }

    fn replace(&mut self, range: Range<usize>, text: &str, _: EditKind) -> bool {
        let (a, b) = (byte(&self.text, range.start), byte(&self.text, range.end));
        self.text.replace_range(a..b, text);
        true
    }

    fn contains(&mut self, at: Point) -> bool {
        let width = self.starts().iter().enumerate().map(|(l, _)| self.range_of(l).len()).max().unwrap_or(0);
        at.x >= 0.0 && at.y >= 0.0 && at.x <= (width as f64 + 1.0) * CELL_WIDTH && at.y <= self.starts().len() as f64 * LINE_HEIGHT
    }

    fn hit(&mut self, at: Point) -> (usize, usize) {
        let starts = self.starts();
        let line = ((at.y / LINE_HEIGHT).floor().max(0.0) as usize).min(starts.len() - 1);
        let range = self.range_of(line);
        ((range.start + (at.x / CELL_WIDTH).round().max(0.0) as usize).min(range.end), line)
    }

    fn caret(&mut self, offset: usize, line: Option<usize>) -> Option<CaretLine> {
        let line = line.unwrap_or_else(|| self.line_of(offset));
        let x = (offset - self.starts()[line]) as f64 * CELL_WIDTH;
        Some(CaretLine { top: Point::new(x, line as f64 * LINE_HEIGHT), bottom: Point::new(x, (line + 1) as f64 * LINE_HEIGHT) })
    }

    fn selection_quads(&mut self, range: Range<usize>) -> Vec<[Point; 4]> {
        self.pieces(range)
            .into_iter()
            .map(|(line, from, to)| {
                let (x0, x1) = (from as f64 * CELL_WIDTH, to as f64 * CELL_WIDTH);
                let (y0, y1) = (line as f64 * LINE_HEIGHT, (line + 1) as f64 * LINE_HEIGHT);
                [Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)]
            })
            .collect()
    }

    fn underline_segments(&mut self, range: Range<usize>) -> Vec<[Point; 2]> {
        self.pieces(range)
            .into_iter()
            .map(|(line, from, to)| {
                let y = (line + 1) as f64 * LINE_HEIGHT - 3.0;
                [Point::new(from as f64 * CELL_WIDTH, y), Point::new(to as f64 * CELL_WIDTH, y)]
            })
            .collect()
    }

    fn line_range(&mut self, line: usize) -> Range<usize> {
        self.range_of(line)
    }

    fn line_step(&mut self, offset: usize, lines: i32) -> usize {
        let starts = self.starts();
        let line = self.line_of(offset) as i32 + lines;
        if line < 0 {
            return 0;
        }
        if line as usize >= starts.len() {
            return self.chars();
        }
        let range = self.range_of(line as usize);
        (range.start + offset - starts[self.line_of(offset)]).min(range.end)
    }

    fn line_edge(&mut self, offset: usize, end: bool) -> usize {
        let range = self.range_of(self.line_of(offset));
        if end { range.end } else { range.start }
    }
}

struct Session {
    host: Lines,
    editor: TextEditor,
    /// Whether the marked text has been sent to the editor.
    composed: bool,
}

const SAMPLE: &str = "The quick brown fox\njumps over the lazy dog";

/// A canvas with a two-line text being edited: the "quick" selected, or with `marked text` on a
/// Japanese composition after the first line, underlined. Click, drag and type in it.
fn text_editor(ui: &mut Ui, args: &mut EditorArgs) {
    let marked = args.marked;
    let selection = if marked { TextSelection::caret_at(19) } else { TextSelection { anchor: 4, caret: 9 } };
    let shared = state(ui, if marked { "editor-marked" } else { "editor" }, || Session {
        host: Lines { text: SAMPLE.to_owned() },
        editor: TextEditor::new(Id::unique(("text-story-editor", marked)), selection).smart_quotes(Some(QuoteStyle::ENGLISH)),
        composed: false,
    });
    let mut guard = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let Session { host, editor, composed } = &mut *guard;

    let size = vec2(ui.available_width().min(520.0), 120.0);
    let response = ui.allocate_response(size, Sense::click_and_drag());
    let tokens = Tokens::of(ui);
    ui.painter().rect_filled(response.rect, 0.0, tokens.color.surface.canvas.to_egui());
    let view = CanvasView::new(response.rect, Point::new(f64::from(size.x) / 2.0, f64::from(size.y) / 2.0), 1.0);
    let ctx = ui.ctx().clone();
    if marked && !*composed {
        *composed = true;
        ctx.input_mut(|i| i.events.push(Event::Ime(ImeEvent::Preedit { text: "かな".to_owned(), active_range_chars: None })));
    }
    if response.has_focus() || (marked && ctx.input(|i| i.events.iter().any(|e| matches!(e, Event::Ime(_))))) {
        let _ = editor.update(&ctx, host);
    }
    paint_text(ui, &view, host);
    let mut tool = TextEditorTool::new(editor, host);
    let _ = ToolHost::new(craft_ui::canvas::CanvasId::of(&response)).show(ui, &view, &response, &mut tool);
    editor.finish_pass(&ctx, &view, host);
}

fn paint_text(ui: &Ui, view: &CanvasView, host: &Lines) {
    let ink = Content::Normal.color(&Tokens::of(ui)).to_egui();
    for (line, text) in host.text.split('\n').enumerate() {
        let at: Pos2 = view.to_screen(Point::new(0.0, line as f64 * LINE_HEIGHT));
        for (col, c) in text.chars().enumerate() {
            let center = at + vec2((col as f64 * CELL_WIDTH + CELL_WIDTH / 2.0) as f32, (LINE_HEIGHT / 2.0) as f32);
            ui.painter().text(center, egui::Align2::CENTER_CENTER, c, FontId::monospace(FONT_SIZE), ink);
        }
    }
}

// --- Character and Paragraph ---

fn character_sample() -> CharacterValues {
    CharacterValues {
        family: Some("Ubuntu".to_owned()),
        style: Some("Light".to_owned()),
        size: Some(24.0),
        leading: Some(Leading::Auto),
        kerning: Some(Kerning::Metrics),
        tracking: None,
        vertical_scale: Some(100.0),
        horizontal_scale: Some(100.0),
        baseline_shift: Some(0.0),
        color: Some(Color32::from_rgb(200, 40, 40)),
        faux_bold: Some(false),
        faux_italic: Some(false),
        caps: Some(Caps::Normal),
        position: Some(BaselinePosition::Normal),
        underline: Some(false),
        strikethrough: Some(false),
        opentype: vec!["liga"],
        language: Some("en-US".to_owned()),
    }
}

fn apply_character(values: &mut CharacterValues, change: CharacterChange) {
    match change {
        CharacterChange::Family(v) => values.family = Some(v),
        CharacterChange::Style(v) => values.style = Some(v),
        CharacterChange::Size(v) => values.size = Some(v),
        CharacterChange::Leading(v) => values.leading = Some(v),
        CharacterChange::Kerning(v) => values.kerning = Some(v),
        CharacterChange::Tracking(v) => values.tracking = Some(v),
        CharacterChange::VerticalScale(v) => values.vertical_scale = Some(v),
        CharacterChange::HorizontalScale(v) => values.horizontal_scale = Some(v),
        CharacterChange::BaselineShift(v) => values.baseline_shift = Some(v),
        CharacterChange::FauxBold(v) => values.faux_bold = Some(v),
        CharacterChange::FauxItalic(v) => values.faux_italic = Some(v),
        CharacterChange::Caps(v) => values.caps = Some(v),
        CharacterChange::Position(v) => values.position = Some(v),
        CharacterChange::Underline(v) => values.underline = Some(v),
        CharacterChange::Strikethrough(v) => values.strikethrough = Some(v),
        CharacterChange::OpenType { tag, on } => {
            values.opentype.retain(|t| *t != tag);
            if on {
                values.opentype.push(tag);
            }
        }
        CharacterChange::Language(v) => values.language = Some(v),
    }
}

/// The Character controls over a 24 pt Ubuntu Light with mixed tracking, Superscript and Subscript
/// and the language list on.
fn character_controls(ui: &mut Ui, _: &mut NoArgs) {
    let shared = state(ui, "character", character_sample);
    let mut values = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let families = ["Ubuntu".to_owned(), "Hack".to_owned()];
    let styles = ["Light".to_owned(), "Regular".to_owned()];
    let languages = [("en-US".to_owned(), "English: USA".to_owned()), ("de-DE".to_owned(), "German".to_owned())];
    let response = CharacterControls::new(&values, &families, &styles).superscript(true).languages(&languages).show(ui);
    for change in response.changes {
        apply_character(&mut values, change);
    }
}

fn apply_paragraph(values: &mut ParagraphValues, change: ParagraphChange) {
    match change {
        ParagraphChange::Align(v) => values.align = Some(v),
        ParagraphChange::Direction(v) => values.direction = Some(v),
        ParagraphChange::StartIndent(v) => values.start_indent = Some(v),
        ParagraphChange::EndIndent(v) => values.end_indent = Some(v),
        ParagraphChange::FirstLineIndent(v) => values.first_line_indent = Some(v),
        ParagraphChange::SpaceBefore(v) => values.space_before = Some(v),
        ParagraphChange::SpaceAfter(v) => values.space_after = Some(v),
        ParagraphChange::Hyphenate(v) => values.hyphenate = Some(v),
    }
}

/// The Paragraph controls on point text: the justify buttons are disabled.
fn paragraph_controls(ui: &mut Ui, _: &mut NoArgs) {
    let shared = state(ui, "paragraph", || ParagraphValues {
        align: Some(TextAlign::Left),
        direction: Some(ParagraphDirection::Automatic),
        start_indent: Some(0.0),
        end_indent: Some(0.0),
        first_line_indent: Some(0.0),
        space_before: Some(0.0),
        space_after: Some(0.0),
        hyphenate: Some(true),
    });
    let mut values = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let response = ParagraphControls::new(&values).point_text(true).show(ui);
    for change in response.changes {
        apply_paragraph(&mut values, change);
    }
}

// --- Text styles ---

/// The Paragraph Styles panel: Basic Paragraph and two styles, the second the style at the type
/// target and overridden.
fn text_styles(ui: &mut Ui, _: &mut NoArgs) {
    struct Panel {
        selected: Option<u64>,
        current: u64,
        overridden: bool,
    }
    let shared = state(ui, "styles", || Panel { selected: Some(1), current: 2, overridden: true });
    let mut guard = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let Panel { selected, current, overridden } = &mut *guard;
    let rows = [(0, "Basic Paragraph"), (1, "Body Copy"), (2, "Heading")].map(|(id, name)| TextStyleRow { id, name: name.to_owned() });
    let response = TextStylesPanel::new(TextStyleKind::Paragraph, &rows, selected).current(Some(*current), *overridden).target(true).show(ui);
    match response.action {
        Some(TextStyleAction::Apply { id, clear_overrides }) => {
            *current = id;
            *overridden = *overridden && !clear_overrides && id == *current;
        }
        Some(TextStyleAction::ClearOverride) => *overridden = false,
        _ => {}
    }
}

// --- Check Spelling ---

const SENTENCE: &str = "Teh quick brwon fox jumps over teh lazy dog.";

struct Document {
    text: String,
    user: UserWords,
}

impl Document {
    fn items(&self) -> Vec<SpellingItem> {
        let dictionary = Dictionary::english();
        dictionary
            .misspellings(&self.text, &self.user)
            .into_iter()
            .map(|m| SpellingItem { object: 1, suggestions: dictionary.suggest(&m.word, SUGGESTION_COUNT), range: m.range, word: m.word })
            .collect()
    }

    fn act(&mut self, action: &SpellingAction) {
        match action {
            SpellingAction::Change { item, to } => self.text.replace_range(item.range.clone(), to),
            SpellingAction::ChangeAll { word, to, .. } => self.text = self.text.replace(word.as_str(), to),
            SpellingAction::Add(word) => {
                self.user.add(word);
            }
            SpellingAction::Relist => {}
        }
    }
}

/// The dialog over the three misspellings of a sample sentence; Change, Change All and Add edit
/// the sentence and list what is left.
fn check_spelling(ui: &mut Ui, _: &mut NoArgs) {
    let fresh = || Document { text: SENTENCE.to_owned(), user: UserWords::default() };
    let shared = state(ui, "spelling-document", fresh);
    let lock = || shared.lock().unwrap_or_else(PoisonError::into_inner);
    {
        let document = lock();
        if document.text != SENTENCE {
            ui.label(document.text.as_str());
        }
    }
    dialog_story(
        ui,
        "spelling",
        || {
            let mut document = lock();
            *document = fresh();
            CheckSpellingDialog::new(document.items())
        },
        |dialog: &mut CheckSpellingDialog| {
            if let Some(action) = dialog.take_action() {
                let mut document = lock();
                document.act(&action);
                dialog.set_items(document.items());
            }
        },
        |()| "Done".to_owned(),
    );
}

/// Opens `make` on a [`DialogHost`] kept in egui memory while the story's dialog is shown
/// (`craft_ui::story::dialog_shown`), runs `tune` on it every pass and shows its answer afterwards.
fn dialog_story<D: StackDialog>(
    ui: &mut Ui,
    name: &str,
    make: impl FnOnce() -> D,
    tune: impl FnOnce(&mut D),
    describe: impl FnOnce(D::Result) -> String,
) {
    struct Dialogs {
        host: DialogHost,
        answer: Option<String>,
        open: bool,
    }
    if !craft_ui::story::dialog_claim(ui.ctx(), name, ui.scope_id()) {
        return;
    }
    let shared = state(ui, name, || Dialogs { host: DialogHost::new(), answer: None, open: true });
    let mut guard = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let Dialogs { host, answer, open } = &mut *guard;
    let ctx: Context = ui.ctx().clone();
    if craft_ui::story::dialog_shown(ui, *open) {
        if !host.is_open::<D>() {
            host.open(make());
            *answer = None;
        }
        if let Some(dialog) = host.get_mut::<D>() {
            tune(dialog);
        }
        host.show(&ctx);
        if let Some(result) = host.take::<D>() {
            *answer = Some(match result {
                DialogResult::Ok(value) => describe(value),
                DialogResult::Cancel => "Cancelled".to_owned(),
            });
            craft_ui::story::dialog_closed(ui, open);
        }
    } else if !craft_ui::story::in_overview(&ctx) && ui.add(Button::new("Show dialog")).clicked() {
        *open = true;
    }
    if let Some(text) = answer {
        ui.label(format!("Result: {text}"));
    }
}
