//! The editor over an in-memory host (monospace layout: one char per unit, lines at `\n`) in a
//! harness.

use std::ops::Range;

use craft_ui::canvas::kurbo::Point;
use egui::{Context, Event, Key, Modifiers, RawInput, Rect, pos2, vec2};

use super::editor::*;
use crate::quotes::QuoteStyle;
use craft_ui::canvas::CanvasView;

/// A text of lines at `\n`; char `c` of a line sits at x = c, line `l` at y = l.
#[derive(Default)]
struct Host {
    text: String,
    vertical: bool,
    replaced: Vec<(Range<usize>, String, EditKind)>,
    styled: Vec<(StyleKey, TextSelection)>,
    /// `style_key` declines every key, as the trait default does.
    declines: bool,
}

impl Host {
    fn new(text: &str) -> Self {
        Host { text: text.into(), ..Host::default() }
    }

    fn starts(&self) -> Vec<usize> {
        let mut starts = vec![0];
        starts.extend(self.text.chars().enumerate().filter(|(_, c)| *c == '\n').map(|(i, _)| i + 1));
        starts
    }

    fn line_of(&self, offset: usize) -> usize {
        self.starts().iter().rposition(|&s| s <= offset).unwrap_or(0)
    }

    fn chars(&self) -> usize {
        self.text.chars().count()
    }
}

fn byte(text: &str, chars: usize) -> usize {
    text.char_indices().nth(chars).map_or(text.len(), |(i, _)| i)
}

impl TextEditorHost for Host {
    fn text(&self) -> &str {
        &self.text
    }

    fn replace(&mut self, range: Range<usize>, text: &str, kind: EditKind) -> bool {
        let (a, b) = (byte(&self.text, range.start), byte(&self.text, range.end));
        self.text.replace_range(a..b, text);
        self.replaced.push((range, text.into(), kind));
        true
    }

    fn contains(&mut self, at: Point) -> bool {
        at.x >= -1.0 && at.y >= -1.0 && at.y <= self.starts().len() as f64
    }

    fn hit(&mut self, at: Point) -> (usize, usize) {
        let starts = self.starts();
        let line = (at.y.floor().max(0.0) as usize).min(starts.len() - 1);
        let end = self.line_range(line).end;
        ((starts[line] + at.x.round().max(0.0) as usize).min(end), line)
    }

    fn caret(&mut self, offset: usize, line: Option<usize>) -> Option<CaretLine> {
        let line = line.unwrap_or_else(|| self.line_of(offset));
        let x = (offset - self.starts()[line]) as f64;
        Some(CaretLine { top: Point::new(x, line as f64), bottom: Point::new(x, line as f64 + 1.0) })
    }

    fn selection_quads(&mut self, _: Range<usize>) -> Vec<[Point; 4]> {
        Vec::new()
    }

    fn underline_segments(&mut self, _: Range<usize>) -> Vec<[Point; 2]> {
        Vec::new()
    }

    fn line_range(&mut self, line: usize) -> Range<usize> {
        let starts = self.starts();
        let start = starts[line];
        let end = starts.get(line + 1).map_or(self.chars(), |next| next - 1);
        start..end
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
        let range = self.line_range(line as usize);
        (range.start + offset - starts[self.line_of(offset)]).min(range.end)
    }

    fn line_edge(&mut self, offset: usize, end: bool) -> usize {
        let range = self.line_range(self.line_of(offset));
        if end { range.end } else { range.start }
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn style_key(&mut self, key: StyleKey, selection: TextSelection) -> bool {
        self.styled.push((key, selection));
        !self.declines
    }
}

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
}

/// ⌘ is Ctrl off macOS; the editor reads `Modifiers::command`, so tests set both.
const COMMAND: Modifiers = Modifiers { command: true, ctrl: true, ..Modifiers::NONE };

fn with(m: Modifiers, f: impl FnOnce(&mut Modifiers)) -> Modifiers {
    let mut m = m;
    f(&mut m);
    m
}

fn run(ctx: &Context, raw: RawInput, f: impl FnOnce(&Context)) -> egui::FullOutput {
    let mut f = Some(f);
    let mut out = ctx.run_ui(raw, |ui| f.take().expect("one pass")(ui.ctx()));
    out.textures_delta.clear();
    out
}

/// One pass of `events` at `time`.
fn pass(ctx: &Context, editor: &mut TextEditor, host: &mut Host, time: f64, events: Vec<Event>) -> EditorResponse {
    let raw = RawInput { time: Some(time), events, screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))), ..Default::default() };
    let mut response = EditorResponse::default();
    run(ctx, raw, |ctx| response = editor.update(ctx, host));
    response
}

fn setup(text: &str, at: usize) -> (Context, TextEditor, Host) {
    (Context::default(), TextEditor::new(egui::Id::unique("editor"), TextSelection::caret_at(at)), Host::new(text))
}

fn keys(events: &[(Key, Modifiers)]) -> Vec<Event> {
    events.iter().map(|&(k, m)| key(k, m)).collect()
}

/// A press at the char (`x`, `y`) of the host's layout, on screen at the same point.
fn click(editor: &mut TextEditor, host: &mut Host, x: f64, y: f64, shift: bool, time: f64) -> bool {
    editor.press(host, Point::new(x, y), pos2(x as f32 * 10.0, y as f32 * 10.0), Modifiers { shift, ..Modifiers::NONE }, time)
}

// Typed letters go in at the caret through `replace(.., Typed)`; with a selection they replace it.
#[test]
fn typing_inserts_at_the_caret_and_replaces_the_selection() {
    let (ctx, mut editor, mut host) = setup("abcd", 2);
    let r = pass(&ctx, &mut editor, &mut host, 0.0, vec![Event::Text("x".into())]);
    assert!(r.changed && r.took_keys);
    assert_eq!((host.text.as_str(), editor.selection()), ("abxcd", TextSelection::caret_at(3)));
    assert_eq!(host.replaced.last(), Some(&(2..2, "x".to_string(), EditKind::Typed)));

    editor.set_selection(TextSelection { anchor: 1, caret: 4 });
    let _ = pass(&ctx, &mut editor, &mut host, 1.0, vec![Event::Text("Z".into())]);
    assert_eq!((host.text.as_str(), editor.selection()), ("aZd", TextSelection::caret_at(2)));
    assert_eq!(host.replaced.last(), Some(&(1..4, "Z".to_string(), EditKind::Typed)));
}

// With `smart_quotes(Some(ENGLISH))` a typed `"` after a space is “; a pasted `"` stays straight.
#[test]
fn typed_quotes_follow_the_style_and_pasted_text_keeps_them() {
    let (ctx, editor, mut host) = setup("say ", 4);
    let mut editor = editor.smart_quotes(Some(QuoteStyle::ENGLISH));
    let _ = pass(&ctx, &mut editor, &mut host, 0.0, vec![Event::Text("\"".into())]);
    assert_eq!(host.text, "say \u{201C}");
    let _ = pass(&ctx, &mut editor, &mut host, 1.0, vec![Event::Text("hi\"".into())]);
    assert_eq!(host.text, "say \u{201C}hi\u{201D}");
    let _ = pass(&ctx, &mut editor, &mut host, 2.0, vec![Event::Paste("\"p\"\r\n".into())]);
    assert_eq!(host.text, "say \u{201C}hi\u{201D}\"p\"\n");
    assert_eq!(host.replaced.last().map(|r| r.2), Some(EditKind::Pasted));
}

// ← → by char, ⌘← → by word, ↑ ↓ through `line_step`, Home/End through `line_edge`; Shift extends.
#[test]
fn arrows_words_lines_and_shift_extend() {
    let (ctx, mut editor, mut host) = setup("one two\nthree four", 0);
    let mut step = |editor: &mut TextEditor, k: Key, m: Modifiers| {
        let r = pass(&ctx, editor, &mut host, 0.0, vec![key(k, m)]);
        assert!(r.took_keys);
        editor.selection()
    };
    let sel = |anchor, caret| TextSelection { anchor, caret };
    assert_eq!(step(&mut editor, Key::ArrowRight, Modifiers::NONE), sel(1, 1));
    assert_eq!(step(&mut editor, Key::ArrowRight, COMMAND), sel(3, 3));
    assert_eq!(step(&mut editor, Key::ArrowRight, COMMAND), sel(7, 7));
    assert_eq!(step(&mut editor, Key::ArrowDown, Modifiers::NONE), sel(15, 15));
    assert_eq!(step(&mut editor, Key::Home, Modifiers::NONE), sel(8, 8));
    assert_eq!(step(&mut editor, Key::End, Modifiers::NONE), sel(18, 18));
    assert_eq!(step(&mut editor, Key::ArrowUp, Modifiers::NONE), sel(7, 7));
    assert_eq!(step(&mut editor, Key::ArrowLeft, COMMAND), sel(4, 4));
    assert_eq!(step(&mut editor, Key::ArrowLeft, Modifiers::SHIFT), sel(4, 3));
    assert_eq!(step(&mut editor, Key::ArrowLeft, with(COMMAND, |m| m.shift = true)), sel(4, 0));
    // A selection collapses to its edge without Shift.
    assert_eq!(step(&mut editor, Key::ArrowRight, Modifiers::NONE), sel(4, 4));
    assert_eq!(step(&mut editor, Key::ArrowDown, COMMAND), sel(18, 18));
    assert_eq!(step(&mut editor, Key::ArrowUp, with(COMMAND, |m| m.shift = true)), sel(18, 0));
}

// ⌫ deletes a char, ⌥⌫ a word, ⌘⌫ to the line start; ⌦ and ⌥⌦ forward.
#[test]
fn backspace_and_delete_by_char_word_and_line() {
    let (ctx, mut editor, mut host) = setup("one two\nthree four", 13);
    let press = |editor: &mut TextEditor, host: &mut Host, k: Key, m: Modifiers| {
        let r = pass(&ctx, editor, host, 0.0, vec![key(k, m)]);
        assert!(r.changed);
        (host.text.clone(), editor.selection().caret)
    };
    assert_eq!(press(&mut editor, &mut host, Key::Backspace, Modifiers::NONE), ("one two\nthre four".into(), 12));
    assert_eq!(press(&mut editor, &mut host, Key::Backspace, Modifiers::ALT), ("one two\n four".into(), 8));
    assert_eq!(press(&mut editor, &mut host, Key::Delete, Modifiers::NONE), ("one two\nfour".into(), 8));
    assert_eq!(press(&mut editor, &mut host, Key::Delete, Modifiers::ALT), ("one two\n".into(), 8));
    editor.set_selection(TextSelection::caret_at(6));
    assert_eq!(press(&mut editor, &mut host, Key::Backspace, COMMAND), ("o\n".into(), 0));
    // A selection is deleted whichever key.
    editor.set_selection(TextSelection { anchor: 0, caret: 1 });
    assert_eq!(press(&mut editor, &mut host, Key::Delete, Modifiers::NONE), ("\n".into(), 0));
    assert_eq!(host.replaced.last().map(|r| r.2), Some(EditKind::Deleted));
    // Nothing before the start of the text.
    let r = pass(&ctx, &mut editor, &mut host, 1.0, vec![key(Key::Backspace, Modifiers::NONE)]);
    assert!(!r.changed && r.took_keys);
}

// Two to five quick clicks select the word, line, paragraph and all; the sixth places a caret.
#[test]
fn click_runs_select_word_line_paragraph_all() {
    let (_, mut editor, mut host) = setup("one two\nthree four", 0);
    let clicks = |editor: &mut TextEditor, host: &mut Host, time: f64| {
        assert!(click(editor, host, 3.0, 1.0, false, time));
        editor.release();
        let s = editor.selection();
        (s.anchor, s.caret)
    };
    assert_eq!(clicks(&mut editor, &mut host, 0.0), (11, 11));
    assert_eq!(clicks(&mut editor, &mut host, 0.1), (8, 13));
    assert_eq!(clicks(&mut editor, &mut host, 0.2), (8, 18));
    assert_eq!(clicks(&mut editor, &mut host, 0.3), (8, 18));
    assert_eq!(clicks(&mut editor, &mut host, 0.4), (0, 18));
    assert_eq!(clicks(&mut editor, &mut host, 0.5), (11, 11));
    // A slow click starts a new run; so does a distant one.
    assert_eq!(clicks(&mut editor, &mut host, 5.0), (11, 11));
    assert!(click(&mut editor, &mut host, 1.0, 0.0, false, 5.1));
    assert_eq!(editor.selection(), TextSelection::caret_at(1));
    // Shift extends from the anchor; a drag by word keeps the pressed word.
    assert!(click(&mut editor, &mut host, 5.0, 0.0, true, 9.0));
    assert_eq!(editor.selection(), TextSelection { anchor: 1, caret: 5 });
    editor.release();
    assert!(click(&mut editor, &mut host, 5.0, 0.0, false, 20.0));
    assert!(click(&mut editor, &mut host, 5.0, 0.0, false, 20.1));
    assert_eq!(editor.selection(), TextSelection { anchor: 4, caret: 7 });
    editor.drag(&mut host, Point::new(9.0, 1.0));
    assert_eq!(editor.selection(), TextSelection { anchor: 4, caret: 18 });
    editor.drag(&mut host, Point::new(0.0, 0.0));
    assert_eq!(editor.selection(), TextSelection { anchor: 7, caret: 0 });
    editor.release();
    editor.drag(&mut host, Point::new(9.0, 1.0));
    assert_eq!(editor.selection(), TextSelection { anchor: 7, caret: 0 });
    // A press outside the text is not the editor's.
    assert!(!click(&mut editor, &mut host, 5.0, 9.0, false, 30.0));
}

// ⇧⌘B is `Toggle(FauxBold)`; ⌥→ at a caret is `Kern(20.0)`, with a selection `Tracking(20.0)`, ⌘ makes it 100.
#[test]
fn style_keys_reach_the_host() {
    let (ctx, mut editor, mut host) = setup("ab\ncd", 1);
    let shift_command = with(COMMAND, |m| m.shift = true);
    let alt_command = with(COMMAND, |m| m.alt = true);
    let send = |editor: &mut TextEditor, host: &mut Host, k: Key, m: Modifiers| {
        let r = pass(&ctx, editor, host, 0.0, vec![key(k, m)]);
        assert!(r.took_keys && !r.changed);
        host.styled.last().map(|s| s.0)
    };
    assert_eq!(send(&mut editor, &mut host, Key::B, shift_command), Some(StyleKey::Toggle(CharToggle::FauxBold)));
    assert_eq!(send(&mut editor, &mut host, Key::Slash, shift_command), Some(StyleKey::Toggle(CharToggle::Strikethrough)));
    assert_eq!(send(&mut editor, &mut host, Key::H, with(alt_command, |m| m.shift = true)), Some(StyleKey::Hyphenate));
    assert_eq!(send(&mut editor, &mut host, Key::ArrowRight, Modifiers::ALT), Some(StyleKey::Kern(20.0)));
    assert_eq!(send(&mut editor, &mut host, Key::ArrowLeft, alt_command), Some(StyleKey::Kern(-100.0)));
    assert_eq!(send(&mut editor, &mut host, Key::ArrowUp, Modifiers::ALT), Some(StyleKey::Leading(-2.0)));
    assert_eq!(send(&mut editor, &mut host, Key::ArrowDown, alt_command), Some(StyleKey::Leading(10.0)));
    assert_eq!(send(&mut editor, &mut host, Key::ArrowUp, with(Modifiers::ALT, |m| m.shift = true)), Some(StyleKey::BaselineShift(2.0)));
    // No pair at a paragraph edge: the host is still asked and decides (here it takes the key).
    editor.set_selection(TextSelection::caret_at(2));
    let before = host.styled.len();
    let r = pass(&ctx, &mut editor, &mut host, 1.0, keys(&[(Key::ArrowRight, Modifiers::ALT)]));
    assert!(r.took_keys);
    assert_eq!(host.styled.len(), before + 1);
    assert_eq!(host.styled.last(), Some(&(StyleKey::Kern(20.0), TextSelection::caret_at(2))));
    editor.set_selection(TextSelection { anchor: 0, caret: 2 });
    assert_eq!(send(&mut editor, &mut host, Key::ArrowRight, Modifiers::ALT), Some(StyleKey::Tracking(20.0)));
    assert_eq!(host.styled.last().map(|s| s.1), Some(TextSelection { anchor: 0, caret: 2 }));
    assert_eq!(send(&mut editor, &mut host, Key::ArrowLeft, alt_command), Some(StyleKey::Tracking(-100.0)));
}

// Esc sets `commit` by default and `cancel` with `esc_commits(false)`; ⌘↩ commits.
#[test]
fn escape_commits_or_cancels() {
    let (ctx, mut editor, mut host) = setup("ab", 1);
    let r = pass(&ctx, &mut editor, &mut host, 0.0, keys(&[(Key::Escape, Modifiers::NONE)]));
    assert!(r.commit && !r.cancel);
    let r = pass(&ctx, &mut editor, &mut host, 1.0, keys(&[(Key::Enter, COMMAND)]));
    assert!(r.commit && host.text == "ab");
    let r = pass(&ctx, &mut editor, &mut host, 2.0, keys(&[(Key::Enter, Modifiers::NONE)]));
    assert!(!r.commit && r.changed && host.text == "a\nb");

    let mut editor = editor.esc_commits(false);
    let r = pass(&ctx, &mut editor, &mut host, 3.0, keys(&[(Key::Escape, Modifiers::NONE)]));
    assert!(r.cancel && !r.commit);
}

// In vertical text ↓ moves to the next char and ← to the next column.
#[test]
fn vertical_text_turns_the_arrows() {
    assert_eq!(flow_key(Key::ArrowDown, true), Key::ArrowRight);
    assert_eq!(flow_key(Key::ArrowLeft, true), Key::ArrowDown);
    assert_eq!(flow_key(Key::ArrowDown, false), Key::ArrowDown);
    assert_eq!(flow_key(Key::Home, true), Key::Home);
    let (ctx, mut editor, mut host) = setup("abc\ndef", 1);
    host.vertical = true;
    let _ = pass(&ctx, &mut editor, &mut host, 0.0, keys(&[(Key::ArrowDown, Modifiers::NONE)]));
    assert_eq!(editor.selection().caret, 2);
    let _ = pass(&ctx, &mut editor, &mut host, 1.0, keys(&[(Key::ArrowLeft, Modifiers::NONE)]));
    assert_eq!(editor.selection().caret, 6);
    let _ = pass(&ctx, &mut editor, &mut host, 2.0, keys(&[(Key::ArrowRight, Modifiers::NONE)]));
    assert_eq!(editor.selection().caret, 2);
    let _ = pass(&ctx, &mut editor, &mut host, 3.0, keys(&[(Key::ArrowUp, Modifiers::NONE)]));
    assert_eq!(editor.selection().caret, 1);
}

// `insert` puts a glyph at the selection with no smart quotes and moves the caret after it; `cut`
// copies the selection to the clipboard output and deletes it.
#[test]
fn menu_insert_copy_and_cut() {
    let (ctx, editor, mut host) = setup("abcd", 0);
    let mut editor = editor.smart_quotes(Some(QuoteStyle::ENGLISH));
    editor.set_selection(TextSelection { anchor: 1, caret: 3 });
    run(&ctx, RawInput::default(), |ctx| assert!(editor.insert(ctx, &mut host, "\"", EditKind::Typed)));
    assert_eq!((host.text.as_str(), editor.selection()), ("a\"d", TextSelection::caret_at(2)));
    editor.set_selection(TextSelection { anchor: 0, caret: 2 });
    let out = run(&ctx, RawInput::default(), |ctx| editor.copy(ctx, &host));
    let copied = out.platform_output.commands.iter().find_map(|c| if let egui::OutputCommand::CopyText(t) = c { Some(t.clone()) } else { None });
    assert_eq!(copied.as_deref(), Some("a\""));
    let out = run(&ctx, RawInput::default(), |ctx| editor.cut(ctx, &mut host));
    let cut = out.platform_output.commands.iter().find_map(|c| if let egui::OutputCommand::CopyText(t) = c { Some(t.clone()) } else { None });
    assert_eq!((cut.as_deref(), host.text.as_str(), editor.selection()), (Some("a\""), "d", TextSelection::caret_at(0)));
    assert_eq!(host.replaced.last().map(|r| r.2), Some(EditKind::Deleted));
    // Nothing selected: nothing copied.
    let out = run(&ctx, RawInput::default(), |ctx| editor.copy(ctx, &host));
    assert!(out.platform_output.commands.is_empty());
}

fn preedit(text: &str) -> Event {
    Event::Ime(egui::ImeEvent::Preedit { text: text.into(), active_range_chars: None })
}

fn commit(text: &str) -> Event {
    Event::Ime(egui::ImeEvent::Commit(text.into()))
}

// Preedit "k", then "か": the host text holds "か" at the caret (kind Composing); commit "課" replaces it (Committed).
#[test]
fn marked_text_is_written_into_the_text_and_replaced() {
    let (ctx, mut editor, mut host) = setup("ab", 1);
    let r = pass(&ctx, &mut editor, &mut host, 0.0, vec![preedit("k")]);
    assert!(r.changed && host.text == "akb" && editor.preedit() == Some(1..2));
    let _ = pass(&ctx, &mut editor, &mut host, 1.0, vec![preedit("か")]);
    assert_eq!((host.text.as_str(), editor.preedit(), editor.selection()), ("aかb", Some(1..2), TextSelection::caret_at(2)));
    assert_eq!(host.replaced.last().map(|r| r.2), Some(EditKind::Composing));
    // The commit arrives after the marked text ended.
    let _ = pass(&ctx, &mut editor, &mut host, 2.0, vec![preedit(""), commit("課")]);
    assert_eq!((host.text.as_str(), editor.preedit(), editor.selection()), ("a課b", None, TextSelection::caret_at(2)));
    assert_eq!(host.replaced.last(), Some(&(1..2, "課".to_string(), EditKind::Committed)));
}

// A press outside while composing keeps the marked text as typed and returns false.
#[test]
fn click_away_keeps_the_marked_text() {
    let (ctx, mut editor, mut host) = setup("ab", 1);
    let _ = pass(&ctx, &mut editor, &mut host, 0.0, vec![preedit("か")]);
    assert!(!click(&mut editor, &mut host, 0.0, 50.0, false, 1.0));
    assert!(editor.preedit().is_none() && host.text == "aかb");
    // The pass ends with the composer told; the next marked text starts after the kept one.
    let view = CanvasView::new(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0)), Point::new(0.0, 0.0), 10.0);
    let raw = RawInput { time: Some(1.0), screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))), ..Default::default() };
    run(&ctx, raw, |ctx| {
        let _ = editor.update(ctx, &mut host);
        let _ = editor.finish_pass(ctx, &view, &mut host);
    });
    let _ = pass(&ctx, &mut editor, &mut host, 2.0, vec![preedit("き")]);
    assert_eq!((host.text.as_str(), editor.preedit()), ("aかきb", Some(2..3)));
}

// `text_replaced` after the host undid the marked text clamps the selection and drops the preedit range.
#[test]
fn external_change_drops_the_marked_range() {
    let (ctx, mut editor, mut host) = setup("ab", 1);
    let _ = pass(&ctx, &mut editor, &mut host, 0.0, vec![preedit("かな")]);
    assert_eq!(editor.preedit(), Some(1..3));
    host.text = "ab".into();
    run(&ctx, RawInput::default(), |ctx| editor.text_replaced(ctx, &mut host));
    assert!(editor.preedit().is_none());
    assert_eq!(editor.selection(), TextSelection::caret_at(2));
    // The platform's composition is fenced: its next marked text is ignored.
    let r = pass(&ctx, &mut editor, &mut host, 1.0, vec![preedit("き")]);
    assert!(!r.changed && host.text == "ab" && editor.preedit().is_none());
}

// On the tool host a press inside the text is claimed and places the caret; a press outside is
// left to the app (`ignored_press`) and starts no gesture; a drag extends the selection.
#[test]
fn tool_host_claims_inside_and_ignores_outside() {
    use craft_ui::canvas::{ToolGesture, ToolHost, ToolHostResponse, ToolInput};

    let ctx = Context::default();
    let mut editor = TextEditor::new(egui::Id::unique("editor"), TextSelection::caret_at(0));
    let mut host = Host::new("hello\nworld");
    let view = CanvasView::new(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0);
    let mut time = 0.0;
    let mut tool_id = egui::Id::NULL;
    let mut step = |editor: &mut TextEditor, host: &mut Host, inputs: &[ToolInput]| -> ToolHostResponse {
        time += 1.0;
        for input in inputs {
            ToolHost::inject(&ctx, tool_id, *input);
        }
        let mut response = None;
        let mut out = ctx.run_ui(RawInput { time: Some(time), screen_rect: Some(view.rect), ..Default::default() }, |ui| {
            let canvas = ui.interact(view.rect, ui.scope_id().with("canvas"), egui::Sense::click_and_drag());
            let mut tool = TextEditorTool::new(editor, host);
            response = Some(ToolHost::new("doc").show(ui, &view, &canvas, &mut tool));
        });
        out.textures_delta.clear();
        let response = response.expect("one pass");
        tool_id = response.id;
        response
    };
    // The first pass gives the host's id, which injected input is queued under.
    let _ = step(&mut editor, &mut host, &[]);
    let m = Modifiers::NONE;
    let at = |x: f64, y: f64| Point::new(x, y);
    let down = |p: Point| ToolInput::Down { doc: p, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers: m };
    let mv = |p: Point| ToolInput::Move { doc: p, pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers: m };
    let up = |p: Point| ToolInput::Up { doc: p, modifiers: m };

    // The host maps document points to screen points one to one here, so `Host::contains` sees
    // the line numbers as y.
    let r = step(&mut editor, &mut host, &[down(at(3.0, 0.5))]);
    assert_eq!(r.gesture, Some(ToolGesture::Began));
    assert!(r.ignored_press.is_none());
    assert_eq!(editor.selection(), TextSelection::caret_at(3));
    let r = step(&mut editor, &mut host, &[mv(at(5.0, 0.5))]);
    assert!(r.gesture.is_some());
    assert_eq!(editor.selection(), TextSelection { anchor: 3, caret: 5 });
    let _ = step(&mut editor, &mut host, &[up(at(5.0, 0.5))]);

    let r = step(&mut editor, &mut host, &[down(at(3.0, 9.0))]);
    assert_eq!(r.gesture, None);
    assert!(r.ignored_press.is_some());
    assert_eq!(editor.selection(), TextSelection { anchor: 3, caret: 5 });
}

fn screen() -> Rect {
    Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))
}

fn primary(pressed: bool) -> Event {
    Event::PointerButton { pos: pos2(0.0, 5.0), button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn view() -> CanvasView {
    CanvasView::new(screen(), Point::new(0.0, 0.0), 10.0)
}

// Typed "x", a press, typed "y" in one pass: "x" goes in at the old caret, the press moves the
// caret, "y" goes in at the new one, all in this pass.
#[test]
fn events_after_a_press_apply_at_the_new_caret() {
    let (ctx, mut editor, mut host) = setup("ab", 2);
    let raw = RawInput {
        time: Some(0.0),
        screen_rect: Some(screen()),
        events: vec![Event::Text("x".into()), primary(true), Event::Text("y".into())],
        ..Default::default()
    };
    let mut late = EditorResponse::default();
    run(&ctx, raw, |ctx| {
        let r = editor.update(ctx, &mut host);
        assert!(r.changed);
        assert_eq!(host.text, "abx", "the text before the press applies at the old caret");
        assert_eq!(ctx.input(|i| i.events.len()), 2, "the press and the text after it wait in the input");
        assert!(click(&mut editor, &mut host, 0.0, 0.0, false, 0.0));
        assert_eq!((host.text.as_str(), editor.selection()), ("yabx", TextSelection::caret_at(1)));
        assert_eq!(ctx.input(|i| i.events.len()), 1, "the text is taken; the press stays for the app");
        late = editor.finish_pass(ctx, &view(), &mut host);
    });
    assert!(late.changed && late.took_keys);
    assert_eq!(host.replaced.last(), Some(&(0..0, "y".to_string(), EditKind::Typed)));
}

// A press the editor never sees (an overlay took it, or it fell outside): what came after it
// applies at the caret as it is, at the end of the pass.
#[test]
fn events_after_a_press_the_editor_did_not_get_apply_in_finish_pass() {
    let (ctx, mut editor, mut host) = setup("ab", 1);
    let raw = RawInput { time: Some(0.0), screen_rect: Some(screen()), events: vec![primary(true), Event::Text("z".into())], ..Default::default() };
    run(&ctx, raw, |ctx| {
        let r = editor.update(ctx, &mut host);
        assert!(!r.changed && host.text == "ab");
        let late = editor.finish_pass(ctx, &view(), &mut host);
        assert!(late.changed);
    });
    assert_eq!(host.text, "azb");
}

// Another widget with the keyboard (a list, a field in a panel) keeps its keys and text; marked
// text is kept as typed; the canvas's own focus does not count.
#[test]
fn the_editor_stands_down_while_another_widget_has_the_keyboard() {
    let canvas = egui::Id::unique("canvas");
    let other = egui::Id::unique("list");
    let (ctx, editor, mut host) = setup("ab", 1);
    let mut editor = editor.canvas(canvas);
    let _ = pass(&ctx, &mut editor, &mut host, 0.0, vec![preedit("か")]);
    assert_eq!((host.text.as_str(), editor.preedit()), ("aかb", Some(1..2)));

    run(&ctx, RawInput { time: Some(1.0), screen_rect: Some(screen()), ..Default::default() }, |ctx| ctx.memory_mut(|m| m.request_focus(other)));
    let enter = key(Key::Enter, Modifiers::NONE);
    let raw = RawInput { time: Some(2.0), screen_rect: Some(screen()), events: vec![enter, Event::Text("q".into())], ..Default::default() };
    run(&ctx, raw, |ctx| {
        assert_eq!(ctx.memory(|m| m.focused()), Some(other));
        let r = editor.update(ctx, &mut host);
        assert!(!r.took_keys && !r.commit);
        assert_eq!(ctx.input(|i| i.events.len()), 2, "the list keeps Enter and the text");
    });
    assert_eq!(host.text, "aかb", "no line break, no q; the marked text stays as typed");
    assert!(editor.preedit().is_none());

    run(&ctx, RawInput { time: Some(3.0), screen_rect: Some(screen()), ..Default::default() }, |ctx| ctx.memory_mut(|m| m.request_focus(canvas)));
    let r = pass(&ctx, &mut editor, &mut host, 4.0, vec![Event::Text("q".into())]);
    assert!(r.changed && host.text == "aかqb", "the canvas with the focus types: {}", host.text);
}

// A window without focus leaves the keys alone.
#[test]
fn an_unfocused_window_leaves_the_keys() {
    let (ctx, mut editor, mut host) = setup("ab", 1);
    let raw = RawInput { time: Some(0.0), screen_rect: Some(screen()), focused: false, events: vec![Event::Text("x".into())], ..Default::default() };
    run(&ctx, raw, |ctx| {
        let r = editor.update(ctx, &mut host);
        assert!(!r.took_keys && ctx.input(|i| i.events.len()) == 1);
    });
    assert_eq!(host.text, "ab");
}

// A key release without ⌘ is consumed with the press; a ⌘ release stays for the app.
#[test]
fn plain_key_releases_are_consumed() {
    let (ctx, mut editor, mut host) = setup("ab", 1);
    let release = |m: Modifiers| Event::Key { key: Key::ArrowRight, physical_key: None, pressed: false, repeat: false, modifiers: m };
    let raw =
        RawInput { time: Some(0.0), screen_rect: Some(screen()), events: vec![release(Modifiers::NONE), release(COMMAND)], ..Default::default() };
    run(&ctx, raw, |ctx| {
        let _ = editor.update(ctx, &mut host);
        assert_eq!(ctx.input(|i| i.events.len()), 1);
    });
}

// ⌥→ at the end of the text goes to the host as a kern; a host without formatting gets the word move.
#[test]
fn alt_arrows_at_the_text_edge_follow_the_host() {
    let alt = Modifiers { alt: true, ..Modifiers::NONE };
    let (ctx, mut editor, mut host) = setup("ab", 2);
    let r = pass(&ctx, &mut editor, &mut host, 0.0, keys(&[(Key::ArrowRight, alt)]));
    assert!(r.took_keys);
    assert_eq!(host.styled, [(StyleKey::Kern(20.0), TextSelection::caret_at(2))]);

    let (ctx, mut editor, mut host) = setup("ab cd", 5);
    host.declines = true;
    let _ = pass(&ctx, &mut editor, &mut host, 0.0, keys(&[(Key::ArrowLeft, alt)]));
    assert_eq!(editor.selection(), TextSelection::caret_at(3), "the word move");
}

// An input method's delete-surrounding around a selection, counted in chars and in bytes (a count
// that ends inside a char leaves it).
#[test]
fn delete_surrounding_counts_chars_and_bytes() {
    let around = |before_chars, after_chars| Event::Ime(egui::ImeEvent::DeleteSurrounding { before_chars, after_chars });
    let bytes = |before_bytes, after_bytes| Event::Ime(egui::ImeEvent::DeleteSurroundingBytes { before_bytes, after_bytes });
    let (ctx, mut editor, mut host) = setup("abcdef", 4);
    editor.set_selection(TextSelection { anchor: 2, caret: 4 });
    let r = pass(&ctx, &mut editor, &mut host, 0.0, vec![around(1, 1)]);
    assert!(r.changed);
    assert_eq!((host.text.as_str(), editor.selection()), ("acdf", TextSelection { anchor: 1, caret: 3 }));
    assert!(host.replaced.iter().all(|r| r.2 == EditKind::Deleted));

    // "é" is 2 bytes and "日" 3: two bytes before the caret end inside 日 and leave it.
    let (ctx, mut editor, mut host) = setup("aé日bc", 3);
    let _ = pass(&ctx, &mut editor, &mut host, 0.0, vec![bytes(2, 0)]);
    assert_eq!(host.text, "aé日bc");
    let _ = pass(&ctx, &mut editor, &mut host, 1.0, vec![bytes(5, 1)]);
    assert_eq!((host.text.as_str(), editor.selection()), ("ac", TextSelection::caret_at(1)));
}

// A press inside while composing keeps the marked text as typed; the input method's late commit
// of the same text, a moment later, is dropped.
#[test]
fn a_press_while_composing_drops_the_late_duplicate_commit() {
    let (ctx, mut editor, mut host) = setup("ab", 1);
    let _ = pass(&ctx, &mut editor, &mut host, 0.0, vec![preedit("か")]);
    assert!(click(&mut editor, &mut host, 0.0, 0.0, false, 1.0));
    assert_eq!((host.text.as_str(), editor.selection(), editor.preedit()), ("aかb", TextSelection::caret_at(0), None));
    let _ = pass(&ctx, &mut editor, &mut host, 1.1, vec![commit("か")]);
    assert_eq!(host.text, "aかb", "the duplicate commit is not typed again");
}
