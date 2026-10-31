//! [`TextEditor`] (F14.02, gap 54): typing on a canvas. The type tool's editing session in
//! PhotoCraft, the text box editors in DesignCraft, VectorCraft and FigCraft, a cell or caption
//! typed in place in GridCraft and FilmCraft.
//!
//! The app owns the text, its layout and its undo; the editor owns the selection, the click run,
//! the input-method state and the keys. It reads and changes the text through the app's
//! [`TextEditorHost`], one call per edit, and draws the selection, the caret and the marked text's
//! underline over the canvas. On the canvas tool host (S3-canvas) it runs as
//! [`TextEditorTool`].
//!
//! Offsets are in chars of the host's text, as PhotoCraft's type tool counts them.
//!
//! Input methods (gap 54) go through craft-ui's [`Composer`]: its edits arrive in the order the
//! platform sent them. The marked (preedit) text is written into the host's text with
//! [`EditKind::Composing`] and replaced by every update, so it lays out, wraps and takes the
//! character style exactly as typed text does (PhotoCraft; Photoshop draws it the same way). The
//! editor remembers its range and underlines it, the converted segment thicker. A commit
//! replaces the range with the result; a click or Esc keeps the marked text as typed. After an
//! accepted commit the editor tells the composer (the egui fork's IME receipt, `ImeContextTracker`),
//! and when the text changes under a composition from outside (undo, a panel edit) it asks the
//! platform to reset its composition (the fork's `Context::request_ime_reset` fence) and ignores
//! the IME until the reset completes.
//!
//! ```ignore
//! // Each pass while the layer is being edited, before the app's shortcuts:
//! let r = editor.update(ctx, &mut layer_host);
//! if r.commit { finish_editing(); }
//! // After the layout of this pass, with the canvas painter:
//! editor.paint(&painter, &view, &mut layer_host);
//! editor.finish_pass(ctx, &view, &mut layer_host);
//! ```

#![expect(unused_variables, reason = "the default `style_key` takes no formatting")]

use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::ops::Range;

use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::{CanvasTool, CanvasView, ToolAction, ToolContext, ToolCursor, ToolKey, ToolPointer};
use craft_ui::ime::{Composer, Edit, surrounding_chars};
use craft_ui::theme::ToEgui;
use egui::ime_context::ImeMutationStamp;
use egui::{Color32, Context, CursorIcon, Event, Id, Key, Modifiers, Painter, Rect, Shape, Stroke};

use crate::quotes::{QuoteStyle, smart_quotes};

/// Most clicks in a run that select more: 2 a word, 3 a line, 4 a paragraph, 5 all text.
const MAX_CLICKS: u32 = 5;
/// A click continues a run when it comes within this many seconds and points of the last
/// (egui's own double-click delay and distance).
const MAX_CLICK_DELAY: f64 = 0.3;
const MAX_CLICK_DISTANCE: f32 = 6.0;
/// The caret as PhotoCraft draws it: a white halo, then a black line inside it, so it shows on
/// any image.
const CARET_STROKES: [Stroke; 2] = [Stroke { width: 3.0, color: Color32::WHITE }, Stroke { width: 1.0, color: Color32::BLACK }];
/// Alpha of the selection's accent fill.
const SELECTION_ALPHA: u8 = 110;
/// Points of the marked text's underline, and of the converted segment's.
const PREEDIT_UNDERLINE: f32 = 1.5;
const ACTIVE_UNDERLINE: f32 = 3.0;
/// The caret's half period (Photoshop's rhythm).
const BLINK_SECONDS: f64 = 0.530;

/// The selection: `anchor` stays where it started, `caret` moves. Equal: a caret with nothing
/// selected. Char offsets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextSelection {
    pub caret: usize,
    pub anchor: usize,
}

impl TextSelection {
    /// A caret at `at` with nothing selected.
    pub fn caret_at(at: usize) -> Self {
        TextSelection { caret: at, anchor: at }
    }

    /// The selected chars, start before end.
    pub fn range(self) -> Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }

    pub fn is_empty(self) -> bool {
        self.caret == self.anchor
    }
}

/// The caret as a segment in document units, from the line's ascent to its descent: vertical in
/// horizontal text, horizontal in vertical text, slanted when the layer is rotated or skewed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaretLine {
    pub top: Point,
    pub bottom: Point,
}

/// What kind of edit a [`TextEditorHost::replace`] call is, for the app's history and styling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// Typed text, a line break, or smart-quoted text.
    Typed,
    /// Marked text of a composition replacing the previous marked text. The app keeps it out of
    /// spelling marks and autocorrection; history coalesces it with the commit.
    Composing,
    /// The text a composition committed, replacing its marked text.
    Committed,
    /// Pasted text (no smart quotes).
    Pasted,
    /// Backspace, Delete, Cut, or an input method's delete-surrounding.
    Deleted,
}

/// A character style toggle on the selection (or the insertion style at a caret).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharToggle {
    AllCaps,
    Underline,
    FauxBold,
    FauxItalic,
    Strikethrough,
}

/// A formatting key the editor recognised, for the app to apply to [`TextEditorHost::style_key`]'s
/// selection. Photoshop's keys; ⌘ is Ctrl on Windows and Linux.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StyleKey {
    /// ⇧⌘K All Caps, ⇧⌘U Underline, ⇧⌘B Faux Bold, ⇧⌘I Faux Italic, ⇧⌘/ Strikethrough.
    Toggle(CharToggle),
    /// ⌥⇧⌘H: Hyphenate on the selected paragraphs.
    Hyphenate,
    /// ⌥↑ / ⌥↓: leading by −2 / +2 points (×5 with ⌘).
    Leading(f32),
    /// ⌥⇧↑ / ⌥⇧↓: baseline shift by +2 / −2 points (×5 with ⌘).
    BaselineShift(f32),
    /// ⌥← / ⌥→ with a selection: tracking by −20 / +20 thousandths of an em (×5 with ⌘).
    Tracking(f32),
    /// ⌥← / ⌥→ at a caret: kerning of the pair around the caret, as [`StyleKey::Tracking`]. At a
    /// line or text edge (no pair) nothing happens (Photoshop).
    Kern(f32),
}

/// The app's text as the editor sees it. Implemented over the edited text object (a type layer,
/// a text frame, a cell) for one editing session.
///
/// Geometry is in document units; the editor maps it to the screen with the [`CanvasView`].
/// Layout calls take `&mut self` so the host may cache its layout between them.
pub trait TextEditorHost {
    /// The whole text, including the marked text while composing.
    fn text(&self) -> &str;

    /// Replace the chars `range` with `text`. The host applies its insertion style (the style the
    /// Character panel set at the caret), records history (one step per editing session in
    /// PhotoCraft: "Edit Type Layer") and lays the text out again. `false` when it refused (a
    /// locked layer): the editor leaves its selection as it was.
    fn replace(&mut self, range: Range<usize>, text: &str, kind: EditKind) -> bool;

    /// Whether `at` is inside the edited text's bounds (its box, or its lines for point text): a
    /// press outside ends the session (PhotoCraft `inside_box`).
    fn contains(&mut self, at: Point) -> bool;

    /// The char offset nearest `at`, and the laid-out line it is on.
    fn hit(&mut self, at: Point) -> (usize, usize);

    /// The caret at `offset` on line `line` (`None`: the line the offset starts, which differs
    /// only at a soft wrap). `None` while the text has no layout yet.
    fn caret(&mut self, offset: usize, line: Option<usize>) -> Option<CaretLine>;

    /// The selection's highlight for `range`: one quadrilateral per line piece, corners in
    /// document units, in drawing order (bidi text gives several per line).
    fn selection_quads(&mut self, range: Range<usize>) -> Vec<[Point; 4]>;

    /// Where the marked text's underline runs for `range`: one segment per line piece, at half the
    /// line's descent below its baseline (PhotoCraft), in document units.
    fn underline_segments(&mut self, range: Range<usize>) -> Vec<[Point; 2]>;

    /// The chars of laid-out line `line`, without its line break.
    fn line_range(&mut self, line: usize) -> Range<usize>;

    /// The offset `lines` lines above (negative) or below `offset`, nearest the caret's position
    /// along the line (↑ / ↓; columns in vertical text).
    fn line_step(&mut self, offset: usize, lines: i32) -> usize;

    /// The start or end of the laid-out line holding `offset` (Home / End, ⌘⌫).
    fn line_edge(&mut self, offset: usize, end: bool) -> usize;

    /// Whether the text runs vertically: the arrow keys turn with it (↓ moves along a column).
    fn vertical(&self) -> bool {
        false
    }

    /// Apply a formatting key to `selection`. `false` (the default) when the app has no such
    /// formatting: the key goes on to the app's shortcuts.
    fn style_key(&mut self, key: StyleKey, selection: TextSelection) -> bool {
        false
    }
}

/// What the editor did this pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use = "end the session on `commit` and `cancel`"]
pub struct EditorResponse {
    /// The text changed (typing, a composition, a paste, a delete).
    pub changed: bool,
    /// The selection or caret moved.
    pub selection_changed: bool,
    /// The user committed: ⌘↩, or Esc when Esc commits
    /// ([`TextEditor::esc_commits`], Photoshop's Use ESC Key to Commit Text, on by default). The
    /// app ends the session and keeps the text.
    pub commit: bool,
    /// Esc when it doesn't commit: the app ends the session and undoes it.
    pub cancel: bool,
    /// The editor took keys this pass; the app's shortcuts leave them (the tool host's
    /// `keep_keys`).
    pub took_keys: bool,
}

/// A text editing session on a canvas: selection, click runs, the input method and the keys. The
/// app keeps one per session and drops it when the session ends.
///
/// What the user sees and does:
/// - A caret drawn as PhotoCraft's two strokes (white 3 pt under black 1 pt), so it shows on any
///   image. It blinks 530 ms on, 530 ms off (Photoshop's rhythm), solid while dragging and
///   restarting at each edit or move. The selection is the theme's accent at alpha 110 over each
///   line piece. Marked text has a 1.5 pt accent underline; the converted segment's is 3 pt
///   (craft-ui's `Preedit::active` convention; PhotoCraft draws one weight).
/// - Clicks: one places the caret, Shift-click extends; quick clicks in one spot select more: two
///   a word, three the laid-out line, four the paragraph, five all text, the sixth starts over
///   (Photoshop). Dragging extends by char, and by word or line after a double or triple click.
/// - Keys: ←/→ by char (a selection collapses to its edge), ⌘←/→ by word, ↑/↓ by line, ⌘↑/↓ to
///   the text's start or end, Home/End to the line's edges, Shift extends any of them. ⌫ and ⌦ by
///   char, ⌥ by word, ⌘⌫ to the line start. ⌘A all, ⌘C ⌘X ⌘V the clipboard. Enter a line break;
///   ⌘↩ commits; Esc commits or cancels. The [`StyleKey`]s format. In vertical
///   text the arrows turn with the flow.
/// - Typed straight quotes become the language's marks while [`smart_quotes`](Self::smart_quotes)
///   is set; pasted text is kept as it is. Pasted line endings become `\n`.
/// - Input methods: see the module docs. The candidate window opens at the caret.
#[derive(Debug)]
pub struct TextEditor {
    id: Id,
    selection: TextSelection,
    /// The marked text's chars in the host's text while composing.
    preedit: Option<Range<usize>>,
    /// The converted segment within the marked text, in chars from its start.
    preedit_active: Option<Range<usize>>,
    composer: Composer,
    /// The click run: count (1..=5), time and screen position of the last press.
    clicks: (u32, f64, egui::Pos2),
    /// A drag is extending the selection, by the unit the press selected (the click run's count:
    /// 1 char, 2 word, 3 line, 4 paragraph).
    dragging: Option<u32>,
    /// The chars the press selected, which a drag by word, line or paragraph keeps selected.
    drag_origin: Range<usize>,
    smart_quotes: Option<QuoteStyle>,
    esc_commits: bool,
    /// Egui time of the last edit, move or press: the blink restarts from it.
    blink_start: f64,
    /// The composition ended where no `Context` was at hand (a press, `set_selection`): the next
    /// pass tells the composer.
    composition_ended: bool,
    /// The marked text as the editor last wrote it: the range in the host's text holds exactly
    /// this while the composition is current.
    preedit_text: String,
    /// Length and hash of the host's text as the last pass left it; a different text at the next
    /// pass was changed from outside.
    seen: Option<(usize, u64)>,
}

impl TextEditor {
    /// A session with the caret at `selection`. `id` names it to egui and the input method (one per
    /// edited object: the layer id under the document's).
    pub fn new(id: Id, selection: TextSelection) -> Self {
        TextEditor {
            id,
            selection,
            preedit: None,
            preedit_active: None,
            composer: Composer::new().with_owner(id),
            clicks: (0, 0.0, egui::Pos2::ZERO),
            dragging: None,
            drag_origin: 0..0,
            smart_quotes: None,
            esc_commits: true,
            blink_start: 0.0,
            composition_ended: false,
            preedit_text: String::new(),
            seen: None,
        }
    }

    /// Convert typed straight quotes with `style` (`None`: off). The app sets it from its Use
    /// Smart Quotes preference and the text's language at the caret.
    pub fn smart_quotes(mut self, style: Option<QuoteStyle>) -> Self {
        self.smart_quotes = style;
        self
    }

    /// Whether Esc commits (Photoshop's default) or cancels the session.
    pub fn esc_commits(mut self, on: bool) -> Self {
        self.esc_commits = on;
        self
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn selection(&self) -> TextSelection {
        self.selection
    }

    /// Set the selection (the app's Select All command, a find result). Clamped to the text on the
    /// next pass; a composition in progress ends with its marked text kept.
    // new: PhotoCraft writes `TextEdit::{caret, anchor}` directly (pc: crates/ui-egui/src/tools/type_tool/editing.rs:123 `select_all`).
    pub fn set_selection(&mut self, selection: TextSelection) {
        self.selection = selection;
        self.end_composition_state();
    }

    /// The marked text's chars while composing.
    pub fn preedit(&self) -> Option<Range<usize>> {
        self.preedit.clone()
    }

    /// Run one pass of input: the composer's edits in order, applied through `host` (marked text
    /// written into the text, typed text smart-quoted, keys moving the caret, formatting through
    /// [`TextEditorHost::style_key`], the clipboard through egui). Call before the app's shortcuts,
    /// with the keyboard on the canvas.
    // pc: crates/ui-egui/src/tools/type_tool/keys.rs:51 `handle_keys` (:287 style keys, :329
    // delete, :356 move), ime_edit.rs:263 `ime_update` (marked text in the layer), :345 and :361
    // the delete-surrounding pair, editing.rs:141 `insert`, :171 `insert_typed`, :359
    // `commit_shortcut`, :108 copy, :131 cut.
    pub fn update(&mut self, ctx: &Context, host: &mut impl TextEditorHost) -> EditorResponse {
        if std::mem::take(&mut self.composition_ended) {
            self.composer.discard(ctx);
        }
        // Before the composer reads this pass's events: a reset fence removes them.
        if self.seen.is_some_and(|seen| seen != fingerprint(host.text())) {
            if self.preedit.is_some() && self.preedit_is_current(&*host) {
                self.composer.note_app_change(ctx);
            } else {
                self.text_replaced(ctx, host);
            }
        }
        let edits = self.composer.update(ctx, true);
        let time = ctx.input(|i| i.time);
        let before = self.selection;
        let chars = host.text().chars().count();
        self.selection = TextSelection { caret: self.selection.caret.min(chars), anchor: self.selection.anchor.min(chars) };
        let mut pass = Pass::default();
        // The marked text an ended composition leaves behind until the next edit says whether a
        // commit replaces it.
        let mut ending: Option<Range<usize>> = None;
        for edit in edits {
            if let Some(range) = ending.take() {
                if let Edit::Insert(text) = &edit {
                    let text = text.replace("\r\n", "\n").replace('\r', "\n");
                    let before = self.stamp(&*host, Some(range.clone()));
                    if self.replace(host, range, &text, EditKind::Committed) {
                        let after = self.stamp(&*host, None);
                        self.composer.note_commit(ctx, before, after);
                        pass.changed = true;
                    }
                    pass.took = true;
                    continue;
                }
                if self.replace(host, range, "", EditKind::Composing) {
                    pass.changed = true;
                }
            }
            match edit {
                Edit::Preedit { text, active } => {
                    pass.took = true;
                    ending = self.preedit_edit(host, &text, active, &mut pass);
                }
                Edit::Insert(text) => {
                    pass.took = true;
                    pass.changed |= self.insert_typed(ctx, host, &text);
                }
                Edit::LineBreak { modifiers } => self.key(ctx, host, Key::Enter, modifiers, &mut pass),
                Edit::Key { key, modifiers, .. } => self.key(ctx, host, key, modifiers, &mut pass),
                Edit::Paste(text) => {
                    pass.took = true;
                    pass.pasted = true;
                    pass.changed |= self.insert_text(ctx, host, &text, EditKind::Pasted);
                }
                Edit::Copy => {
                    pass.took = true;
                    pass.clipboard = true;
                    self.copy(ctx, &*host);
                }
                Edit::Cut => {
                    pass.took = true;
                    pass.clipboard = true;
                    pass.changed |= self.cut_selection(ctx, host);
                }
                Edit::DeleteSurrounding { before_chars, after_chars } => {
                    pass.took = true;
                    pass.changed |= self.delete_around(ctx, host, before_chars, after_chars);
                }
                Edit::DeleteSurroundingBytes { before_bytes, after_bytes } => {
                    pass.took = true;
                    pass.changed |= self.delete_around_bytes(ctx, host, before_bytes, after_bytes);
                }
            }
        }
        if let Some(range) = ending
            && self.replace(host, range, "", EditKind::Composing)
        {
            pass.changed = true;
        }
        if pass.changed || self.selection != before {
            self.blink_start = time;
        }
        self.seen = Some(fingerprint(host.text()));
        consume_events(ctx, &pass);
        EditorResponse {
            changed: pass.changed,
            selection_changed: self.selection != before,
            commit: pass.commit,
            cancel: pass.cancel,
            took_keys: pass.took,
        }
    }

    /// Replace the selection with `text` as if typed, without smart quotes: a glyph from the Glyphs
    /// panel, Type › Paste Lorem Ipsum, a suggestion from the context menu over a misspelled word.
    /// The caret ends after it. A composition in progress ends first, its marked text kept.
    /// Edit › Paste from a menu goes through the clipboard instead
    /// (`ViewportCommand::RequestPaste`; the text arrives in the next [`update`](Self::update)).
    // pc: crates/ui-egui/src/panels/type_panels_ui.rs:608 `insert_glyph`;
    // crates/ui-egui/src/tools/type_tool/editing.rs:141 `insert`, :118 `request_paste`.
    pub fn insert(&mut self, ctx: &Context, host: &mut impl TextEditorHost, text: &str, kind: EditKind) -> bool {
        if self.preedit.is_some() || self.composer.is_composing() {
            self.preedit = None;
            self.preedit_active = None;
            self.composer.discard(ctx);
        }
        let changed = self.insert_text(ctx, host, text, kind);
        if changed {
            self.blink_start = ctx.input(|i| i.time);
            self.seen = Some(fingerprint(host.text()));
        }
        changed
    }

    /// Edit › Copy from a menu: the selected chars to the clipboard (`Context::copy_text`).
    /// Nothing with an empty selection.
    // pc: crates/ui-egui/src/tools/type_tool/editing.rs:108 `copy_selection`.
    pub fn copy(&self, ctx: &Context, host: &impl TextEditorHost) {
        let range = self.clamped(host.text().chars().count());
        if !range.is_empty() {
            ctx.copy_text(char_slice(host.text(), &range));
        }
    }

    /// Edit › Cut from a menu: copy, then delete the selection ([`EditKind::Deleted`]).
    // pc: crates/ui-egui/src/tools/type_tool/editing.rs:131 `cut_selection`.
    pub fn cut(&mut self, ctx: &Context, host: &mut impl TextEditorHost) {
        if self.cut_selection(ctx, host) {
            self.blink_start = ctx.input(|i| i.time);
            self.seen = Some(fingerprint(host.text()));
        }
    }

    /// A press on the canvas at `at` (document units, `screen` on screen) at egui time `time`. Inside the text: places
    /// the caret (Shift extends), counts the click run and selects by it, and starts a drag;
    /// returns `true`. Outside: ends a composition with its marked text kept and returns `false`,
    /// so the app ends the session (and may start one on the text under the pointer).
    // pc: crates/ui-egui/src/tools/type_tool/pointer.rs:107 `pointer_down`, :375
    // `next_click_count`, :389 `click_selection`, :429 `multi_click`.
    pub fn press(&mut self, host: &mut impl TextEditorHost, at: Point, screen: egui::Pos2, modifiers: Modifiers, time: f64) -> bool {
        self.end_composition_state();
        if !host.contains(at) {
            self.clicks = (0, time, screen);
            return false;
        }
        let (offset, line) = host.hit(at);
        let chars = host.text().chars().count();
        let offset = offset.min(chars);
        let (previous, previous_time, previous_at) = self.clicks;
        // egui counts a click as part of a run when it is soon enough and near enough.
        let quick = previous > 0 && (0.0..=MAX_CLICK_DELAY).contains(&(time - previous_time)) && screen.distance(previous_at) <= MAX_CLICK_DISTANCE;
        let count = next_click_count(previous, if quick { 2 } else { 1 });
        self.clicks = (count, time, screen);
        self.selection.caret = offset;
        if !modifiers.shift {
            self.selection.anchor = offset;
        }
        self.drag_origin = offset..offset;
        if count >= 2 {
            let line = (count == 3).then(|| host.line_range(line));
            let range = click_selection(host.text(), offset, line, count);
            self.selection = TextSelection { anchor: range.start, caret: range.end };
            self.drag_origin = range;
        }
        self.dragging = Some(count);
        self.blink_start = time;
        true
    }

    /// The pointer moved with the button held after a press inside: extend the selection by the
    /// press's unit.
    // pc: crates/ui-egui/src/tools/type_tool/pointer.rs:171 `pointer_move`.
    pub fn drag(&mut self, host: &mut impl TextEditorHost, at: Point) {
        let Some(count) = self.dragging else { return };
        let (offset, line) = host.hit(at);
        let offset = offset.min(host.text().chars().count());
        if count <= 1 {
            self.selection.caret = offset;
            return;
        }
        let line = (count == 3).then(|| host.line_range(line));
        let range = click_selection(host.text(), offset, line, count);
        let origin = self.drag_origin.clone();
        self.selection = if range.start < origin.start {
            TextSelection { anchor: origin.end, caret: range.start }
        } else {
            TextSelection { anchor: origin.start, caret: range.end.max(origin.end) }
        };
    }

    /// The button came up.
    pub fn release(&mut self) {
        self.dragging = None;
    }

    /// Draw the selection, the caret and the marked text's underline. Run after the text itself.
    // pc: crates/ui-egui/src/tools/type_tool/overlay.rs:7 `draw_overlay` (:86 preedit underline,
    // :101 selection fill, :128 blink, :144 `CARET_STROKES`).
    pub fn paint(&self, painter: &Painter, view: &CanvasView, host: &mut impl TextEditorHost) {
        let ctx = painter.ctx();
        let accent = craft_ui::theme::current(ctx).color.accent.solid.to_egui();
        let screen = |p: Point| view.to_screen(p);
        let chars = host.text().chars().count();
        if let Some(range) = self.preedit.clone().filter(|r| r.end <= chars && !r.is_empty()) {
            let mut underline = |r: Range<usize>, width: f32| {
                for [a, b] in host.underline_segments(r) {
                    painter.line_segment([screen(a), screen(b)], Stroke::new(width, accent));
                }
            };
            underline(range.clone(), PREEDIT_UNDERLINE);
            if let Some(active) = self.preedit_active.clone().filter(|a| !a.is_empty() && a.end <= range.len()) {
                underline(range.start + active.start..range.start + active.end, ACTIVE_UNDERLINE);
            }
        }
        let selected = self.clamped(chars);
        if !selected.is_empty() {
            let fill = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), SELECTION_ALPHA);
            for quad in host.selection_quads(selected) {
                painter.add(Shape::convex_polygon(quad.iter().map(|&p| screen(p)).collect(), fill, Stroke::NONE));
            }
            return;
        }
        let time = ctx.input(|i| i.time);
        let elapsed = (time - self.blink_start).max(0.0);
        ctx.request_repaint_after(std::time::Duration::from_secs_f64((BLINK_SECONDS - elapsed % BLINK_SECONDS).clamp(0.010, BLINK_SECONDS)));
        let visible = self.dragging.is_some() || ((elapsed / BLINK_SECONDS) as i64) % 2 == 0;
        if visible && let Some(CaretLine { top, bottom }) = host.caret(self.selection.caret.min(chars), None) {
            for stroke in CARET_STROKES {
                painter.line_segment([screen(top), screen(bottom)], stroke);
            }
        }
    }

    /// End the pass: place the candidate window at the caret (screen rect from `view`) and give
    /// the composer the committed text around the caret, the marked text left out, for the
    /// platform's surrounding text. Run once per pass after the layout and before the pass ends.
    // pc: crates/ui-egui/src/tools/type_tool/ime_edit.rs:128 `sync_ime`, :222 `ime_geometry`, :86
    // `committed_text`.
    pub fn finish_pass(&mut self, ctx: &Context, view: &CanvasView, host: &mut impl TextEditorHost) {
        // A press or `set_selection` ended the composition in this pass: the composer learns of it
        // before it publishes, so the platform drops its copy in the same pass.
        if std::mem::take(&mut self.composition_ended) {
            self.composer.discard(ctx);
        }
        // An application edit after `update` can have moved the marked text.
        if self.preedit.is_some() && !self.preedit_is_current(&*host) {
            self.text_replaced(ctx, host);
        }
        let text = host.text();
        let at = |c: usize| text.char_indices().nth(c).map_or(text.len(), |(b, _)| b);
        let preedit = self.preedit.clone().map(|r| at(r.start)..at(r.end));
        self.composer.set_committed(text, at(self.selection.caret), at(self.selection.anchor), preedit);
        let caret = self.selection.caret.min(text.chars().count());
        if let Some(CaretLine { top, bottom }) = host.caret(caret, None) {
            self.composer.set_caret(ctx, Rect::from_two_pos(view.to_screen(top), view.to_screen(bottom)).expand(1.0));
        }
    }

    /// The host's text changed outside the editor (undo, redo, a panel edit, another view of the
    /// same text): clamp the selection, drop the marked range, and reset the platform's
    /// composition when one was running.
    // pc: crates/ui-egui/src/tools/type_tool/ime_edit.rs:70 `reconcile_preedit`, ime.rs:346
    // `prepare_reset` and :357 `complete_reset`, ime_edit.rs:117 `note_ime_application_change`.
    pub fn text_replaced(&mut self, ctx: &Context, host: &mut impl TextEditorHost) {
        let chars = host.text().chars().count();
        self.selection = TextSelection { caret: self.selection.caret.min(chars), anchor: self.selection.anchor.min(chars) };
        self.preedit = None;
        self.preedit_active = None;
        self.composition_ended = false;
        self.composer.reset(ctx);
        self.composer.note_app_change(ctx);
        self.seen = Some(fingerprint(host.text()));
    }

    /// End the session's composition before the app commits or switches away: the marked text
    /// stays as typed and the platform drops its copy.
    // pc: crates/ui-egui/src/tools/type_tool/ime.rs:180 `interrupt_incoming`, editing.rs:353 `commit`.
    pub fn end_composition(&mut self, ctx: &Context, _host: &mut impl TextEditorHost) {
        self.preedit = None;
        self.preedit_active = None;
        self.composition_ended = false;
        self.composer.discard(ctx);
    }
}

/// What one pass of edits did.
#[derive(Default)]
struct Pass {
    changed: bool,
    commit: bool,
    cancel: bool,
    /// The editor took keys or text: the app's shortcuts leave them.
    took: bool,
    /// Keys the editor handled, to remove from the input.
    keys: Vec<(Key, Modifiers)>,
    pasted: bool,
    clipboard: bool,
}

/// Removes what the editor took from the pass's input, so the app's shortcuts don't see it again.
fn consume_events(ctx: &Context, pass: &Pass) {
    if !pass.took {
        return;
    }
    let mut keys = pass.keys.clone();
    ctx.input_mut(|i| {
        i.events.retain(|event| match event {
            Event::Key { key, pressed: true, modifiers, .. } => match keys.iter().position(|k| k.0 == *key && k.1 == *modifiers) {
                Some(at) => {
                    keys.remove(at);
                    false
                }
                None => true,
            },
            Event::Text(_) => false,
            Event::Paste(_) => !pass.pasted,
            Event::Copy | Event::Cut => !pass.clipboard,
            _ => true,
        });
    });
}

/// Length and hash of `text`, to tell whether it changed between passes.
fn fingerprint(text: &str) -> (usize, u64) {
    let mut hasher = std::hash::DefaultHasher::new();
    text.hash(&mut hasher);
    (text.len(), hasher.finish())
}

/// The chars of `text` in `range`.
fn char_slice(text: &str, range: &Range<usize>) -> String {
    text.chars().skip(range.start).take(range.len()).collect()
}

/// The chars of the paragraph holding char `c`, without its line break.
fn paragraph_of(chars: &[char], c: usize) -> Range<usize> {
    let is_break = |ch: &char| matches!(ch, '\n' | '\r');
    let start = chars[..c].iter().rposition(is_break).map_or(0, |i| i + 1);
    let end = chars[c..].iter().position(is_break).map_or(chars.len(), |i| c + i);
    start..end
}

impl TextEditor {
    /// The selection as a range within `chars`.
    fn clamped(&self, chars: usize) -> Range<usize> {
        let range = self.selection.range();
        range.start.min(chars)..range.end.min(chars)
    }

    /// Ends the composition in the editor's state, keeping the marked text as typed. The composer
    /// learns of it in the next pass.
    fn end_composition_state(&mut self) {
        if self.preedit.take().is_some() || self.composer.is_composing() {
            self.composition_ended = true;
        }
        self.preedit_active = None;
    }

    /// Whether the marked range still holds the text the composition wrote.
    fn preedit_is_current(&self, host: &impl TextEditorHost) -> bool {
        self.preedit.as_ref().is_some_and(|r| r.end <= host.text().chars().count() && char_slice(host.text(), r) == self.preedit_text)
    }

    /// The committed text and selection as the input method sees them: the marked text, if any,
    /// left out. `preedit` is its range in the host's text now.
    fn stamp(&self, host: &impl TextEditorHost, preedit: Option<Range<usize>>) -> ImeMutationStamp {
        let text = host.text();
        let at = |c: usize| text.char_indices().nth(c).map_or(text.len(), |(b, _)| b);
        ImeMutationStamp::from_committed(text, at(self.selection.caret), at(self.selection.anchor), preedit.map(|r| at(r.start)..at(r.end)))
    }

    /// Replace `range` through the host and put the caret after the new text.
    fn replace(&mut self, host: &mut impl TextEditorHost, range: Range<usize>, text: &str, kind: EditKind) -> bool {
        if !host.replace(range.clone(), text, kind) {
            return false;
        }
        self.selection = TextSelection::caret_at(range.start + text.chars().count());
        true
    }

    /// Replace the selection with `text` (line endings become `\n`). An application edit ends a
    /// live composition.
    fn insert_text(&mut self, ctx: &Context, host: &mut impl TextEditorHost, text: &str, kind: EditKind) -> bool {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let range = self.clamped(host.text().chars().count());
        if range.is_empty() && text.is_empty() {
            return false;
        }
        if !self.replace(host, range, &text, kind) {
            return false;
        }
        if self.preedit.take().is_some() {
            self.preedit_active = None;
            self.composer.discard(ctx);
        }
        self.composer.note_app_change(ctx);
        true
    }

    /// Typed text: straight quotes follow the style, when there is one.
    fn insert_typed(&mut self, ctx: &Context, host: &mut impl TextEditorHost, text: &str) -> bool {
        let Some(style) = self.smart_quotes else {
            return self.insert_text(ctx, host, text, EditKind::Typed);
        };
        let start = self.selection.range().start;
        let before = start.checked_sub(1).and_then(|i| host.text().chars().nth(i));
        let text = smart_quotes(text, before, style);
        self.insert_text(ctx, host, &text, EditKind::Typed)
    }

    /// Copy the selection, then delete it.
    fn cut_selection(&mut self, ctx: &Context, host: &mut impl TextEditorHost) -> bool {
        let range = self.clamped(host.text().chars().count());
        if range.is_empty() {
            return false;
        }
        self.copy(ctx, &*host);
        self.replace(host, range, "", EditKind::Deleted)
    }

    /// New marked text (or its end, with empty `text`). Returns the range of marked text that
    /// ended, for a commit to replace.
    fn preedit_edit(&mut self, host: &mut impl TextEditorHost, text: &str, active: Option<Range<usize>>, pass: &mut Pass) -> Option<Range<usize>> {
        let chars = host.text().chars().count();
        let marked = self.preedit.take().filter(|r| r.end <= chars);
        let active = active.filter(|a| a.end <= text.chars().count());
        if text.is_empty() {
            self.preedit_active = None;
            return marked;
        }
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let range = marked.unwrap_or_else(|| self.clamped(chars));
        if self.replace(host, range.clone(), &text, EditKind::Composing) {
            self.preedit = Some(range.start..range.start + text.chars().count());
            self.preedit_text = text;
            self.preedit_active = active;
            pass.changed = true;
        }
        None
    }

    /// An input method's delete-surrounding: the chars around the marked text, or the selection,
    /// as counts of chars.
    fn delete_around(&mut self, ctx: &Context, host: &mut impl TextEditorHost, before: usize, after: usize) -> bool {
        let (start, end) = self.active_range(host);
        self.delete_runs(ctx, host, start.saturating_sub(before), start, end, end.saturating_add(after))
    }

    /// As [`Self::delete_around`], counted in UTF-8 bytes; a count that ends inside a char leaves it.
    fn delete_around_bytes(&mut self, ctx: &Context, host: &mut impl TextEditorHost, before: usize, after: usize) -> bool {
        let (start, end) = self.active_range(host);
        let text = host.text();
        let head: String = text.chars().take(start).collect();
        let tail: String = text.chars().skip(end).collect();
        let (before, after) = surrounding_chars(&head, &tail, before, after);
        self.delete_runs(ctx, host, start - before, start, end, end + after)
    }

    /// The marked text, else the selection.
    fn active_range(&self, host: &impl TextEditorHost) -> (usize, usize) {
        let chars = host.text().chars().count();
        let range = self.preedit.clone().filter(|r| r.end <= chars).unwrap_or_else(|| self.clamped(chars));
        (range.start, range.end)
    }

    /// Delete `from..start` and `end..to`, keeping `start..end` (and the caret in it).
    fn delete_runs(&mut self, ctx: &Context, host: &mut impl TextEditorHost, from: usize, start: usize, end: usize, to: usize) -> bool {
        let to = to.min(host.text().chars().count());
        if from >= start && end >= to {
            return false;
        }
        let before = self.stamp(&*host, self.preedit.clone());
        // The trailing run first, so the leading run's offsets stay valid.
        if end < to && !host.replace(end..to, "", EditKind::Deleted) {
            return false;
        }
        if from < start && !host.replace(from..start, "", EditKind::Deleted) {
            return false;
        }
        let shift = start - from;
        self.selection = TextSelection { caret: self.selection.caret.saturating_sub(shift), anchor: self.selection.anchor.saturating_sub(shift) };
        if let Some(r) = self.preedit.as_mut() {
            *r = r.start.saturating_sub(shift)..r.end.saturating_sub(shift);
        }
        let after = self.stamp(&*host, self.preedit.clone());
        self.composer.note_commit(ctx, before, after);
        true
    }

    /// Move the caret to `to` (clamped), extending the selection with `extend`.
    fn move_caret(&mut self, to: usize, chars: usize, extend: bool) {
        self.selection.caret = to.min(chars);
        if !extend {
            self.selection.anchor = self.selection.caret;
        }
    }

    /// One key press. Chords with ⌘ the editor has no use for stay the app's.
    fn key(&mut self, ctx: &Context, host: &mut impl TextEditorHost, raw: Key, modifiers: Modifiers, pass: &mut Pass) {
        let text = host.text().to_owned();
        let chars = text.chars().count();
        self.selection = TextSelection { caret: self.selection.caret.min(chars), anchor: self.selection.anchor.min(chars) };
        let key = flow_key(raw, host.vertical());
        let handled = self.style_key(host, key, modifiers, &text)
            || self.delete_key(host, key, modifiers, &text, pass)
            || self.move_key(host, key, modifiers, &text);
        if handled {
            pass.took = true;
            pass.keys.push((raw, modifiers));
            return;
        }
        let taken = match key {
            Key::Enter if modifiers.command => {
                // The input method owns confirmation until the marked text ends.
                pass.commit |= self.preedit.is_none();
                true
            }
            Key::Enter => {
                pass.changed |= self.insert_text(ctx, host, "\n", EditKind::Typed);
                true
            }
            Key::Tab if !modifiers.command && !modifiers.alt && !modifiers.shift => {
                // egui queues focus navigation before this runs; consuming the key alone would
                // let the menu bar take focus after the first Tab.
                ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
                pass.changed |= self.insert_text(ctx, host, "\t", EditKind::Typed);
                true
            }
            Key::Escape => {
                if self.esc_commits {
                    pass.commit = true;
                } else {
                    pass.cancel = true;
                }
                true
            }
            Key::A if modifiers.command => {
                self.selection = TextSelection { anchor: 0, caret: chars };
                true
            }
            _ => !modifiers.command,
        };
        if taken {
            pass.took = true;
            pass.keys.push((raw, modifiers));
        }
    }

    /// Photoshop's formatting keys, through the host.
    fn style_key(&mut self, host: &mut impl TextEditorHost, key: Key, modifiers: Modifiers, text: &str) -> bool {
        let scale = if modifiers.command { 5.0 } else { 1.0 };
        let selection = self.selection;
        let style = match key {
            Key::K | Key::U | Key::B | Key::I | Key::Slash | Key::Questionmark if modifiers.command && modifiers.shift && !modifiers.alt => {
                StyleKey::Toggle(match key {
                    Key::K => CharToggle::AllCaps,
                    Key::U => CharToggle::Underline,
                    Key::B => CharToggle::FauxBold,
                    Key::I => CharToggle::FauxItalic,
                    _ => CharToggle::Strikethrough,
                })
            }
            Key::H if modifiers.command && modifiers.shift && modifiers.alt => StyleKey::Hyphenate,
            Key::ArrowUp | Key::ArrowDown if modifiers.alt => {
                let up = key == Key::ArrowUp;
                if modifiers.shift {
                    StyleKey::BaselineShift(2.0 * scale * if up { 1.0 } else { -1.0 })
                } else {
                    StyleKey::Leading(2.0 * scale * if up { -1.0 } else { 1.0 })
                }
            }
            Key::ArrowLeft | Key::ArrowRight if modifiers.alt && !modifiers.shift => {
                let by = 20.0 * scale * if key == Key::ArrowRight { 1.0 } else { -1.0 };
                if !selection.is_empty() {
                    StyleKey::Tracking(by)
                } else {
                    // Kerning needs a pair on one line: a text or paragraph edge has none.
                    let before = selection.caret.checked_sub(1).and_then(|i| text.chars().nth(i));
                    let after = text.chars().nth(selection.caret);
                    if before.is_none_or(|c| c == '\n') || after.is_none_or(|c| c == '\n') {
                        return true;
                    }
                    StyleKey::Kern(by)
                }
            }
            _ => return false,
        };
        host.style_key(style, selection)
    }

    fn delete_key(&mut self, host: &mut impl TextEditorHost, key: Key, modifiers: Modifiers, text: &str, pass: &mut Pass) -> bool {
        if !matches!(key, Key::Backspace | Key::Delete) {
            return false;
        }
        let chars = text.chars().count();
        let range = self.clamped(chars);
        let range = if range.is_empty() {
            let at = range.start;
            let forward = key == Key::Delete;
            let to = match (forward, modifiers.alt, modifiers.command) {
                (false, _, true) => host.line_edge(at, false),
                (false, true, _) => word_boundary(text, at, false),
                (false, _, _) => at.saturating_sub(1),
                (true, true, _) => word_boundary(text, at, true),
                (true, _, _) => (at + 1).min(chars),
            };
            to.min(at)..to.max(at)
        } else {
            range
        };
        if !range.is_empty() {
            pass.changed |= self.replace(host, range, "", EditKind::Deleted);
        }
        true
    }

    fn move_key(&mut self, host: &mut impl TextEditorHost, key: Key, modifiers: Modifiers, text: &str) -> bool {
        let chars = text.chars().count();
        let caret = self.selection.caret;
        let range = self.selection.range();
        let to = match key {
            Key::ArrowLeft | Key::ArrowRight => {
                let forward = key == Key::ArrowRight;
                if modifiers.command || modifiers.alt {
                    word_boundary(text, caret, forward)
                } else if !range.is_empty() && !modifiers.shift {
                    if forward { range.end } else { range.start }
                } else if forward {
                    caret + 1
                } else {
                    caret.saturating_sub(1)
                }
            }
            Key::ArrowUp | Key::ArrowDown => {
                let up = key == Key::ArrowUp;
                if modifiers.command { if up { 0 } else { chars } } else { host.line_step(caret, if up { -1 } else { 1 }) }
            }
            Key::Home | Key::End => host.line_edge(caret, key == Key::End),
            _ => return false,
        };
        self.move_caret(to, chars, modifiers.shift);
        true
    }
}

/// The selection a run of `count` quick clicks makes at char `at`: 1 a caret, 2 the word (letters
/// and digits) around it, 3 the laid-out line `line` (`host.line_range`), 4 the paragraph up to
/// its line break, 5 all of `text`. Offsets past the text are clamped.
// pc: crates/ui-egui/src/tools/type_tool/pointer.rs:389 `click_selection`, :417 `paragraph_of`.
pub fn click_selection(text: &str, at: usize, line: Option<Range<usize>>, count: u32) -> Range<usize> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let c = at.min(n);
    match count {
        2 => {
            let word = |i: usize| chars.get(i).is_some_and(|ch| ch.is_alphanumeric());
            let (mut a, mut b) = (c, c);
            while a > 0 && word(a - 1) {
                a -= 1;
            }
            while word(b) {
                b += 1;
            }
            a..b
        }
        3 => line.map_or_else(|| paragraph_of(&chars, c), |l| l.start.min(n)..l.end.min(n)),
        4 => paragraph_of(&chars, c),
        5 => 0..n,
        _ => c..c,
    }
}

/// The count of a click in a run: `egui_count` is egui's own (1, 2 or 3, after its delay and
/// distance checks), `prev` the previous click's count (0 for none). A slow or distant click
/// starts a new run; after five the run starts over.
// pc: crates/ui-egui/src/tools/type_tool/pointer.rs:375 `next_click_count`.
pub fn next_click_count(prev: u32, egui_count: u32) -> u32 {
    match (egui_count, prev) {
        (0 | 1, _) => 1,
        (n, 0) => n.min(MAX_CLICKS),
        (_, p) => p % MAX_CLICKS + 1,
    }
}

/// The next word edge from char `from`: forward skips non-word chars then word chars (letters and
/// digits), backward the same leftward in logical order.
// pc: crates/text/src/layout/navigate.rs:42 `word_boundary`.
pub fn word_boundary(text: &str, from: usize, forward: bool) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut i = from.min(chars.len());
    if forward {
        while i < chars.len() && !chars[i].is_alphanumeric() {
            i += 1;
        }
        while i < chars.len() && chars[i].is_alphanumeric() {
            i += 1;
        }
    } else {
        while i > 0 && !chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        while i > 0 && chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
    }
    i
}

/// The arrow key as the logical direction it means in vertical text: ↓ next char, ↑ previous,
/// ← next column, → previous column (right-to-left columns). Other keys and horizontal text
/// unchanged.
// pc: crates/ui-egui/src/tools/type_tool/keys.rs:29 `flow_key`.
pub fn flow_key(key: Key, vertical: bool) -> Key {
    if !vertical {
        return key;
    }
    match key {
        Key::ArrowUp => Key::ArrowLeft,
        Key::ArrowDown => Key::ArrowRight,
        Key::ArrowLeft => Key::ArrowDown,
        Key::ArrowRight => Key::ArrowUp,
        k => k,
    }
}

/// [`TextEditor`] on the canvas tool host, for apps whose type tool is a `CanvasTool`: the host
/// delivers presses, drags and keys in document units, and the tool host's claim keeps the
/// overlays (guides, a transform box) ahead of it. The app builds it per pass over its editor and
/// host.
///
/// - Press inside the text: the editor's [`press`](TextEditor::press), the gesture claimed. Press
///   outside: the session's composition ends and the press is ignored, so the app's own handling
///   ends the session or starts one on the text under the pointer (Photoshop: a click outside
///   commits).
/// - Drag and release: the editor's. Cancel (Esc during a drag, a tool switch): the drag ends, the
///   selection stays.
/// - Keys: taken while editing (the tool host's `keep_keys`), so the app's single-key tool
///   shortcuts don't fire while typing.
/// - Cursor: the I-beam (the vertical I-beam over vertical text); paint: the editor's paint.
/// - No snapping; autoscroll on, so a selection drag past the edge scrolls.
///
/// The app runs [`TextEditor::update`] before the tool host in a pass (it reads the keys and the
/// input method, and a press in the same pass counts on it) and [`TextEditor::finish_pass`] after
/// it; the host has no per-pass hook for a tool.
///
/// The paragraph box's handles are not the editor's: the app draws them with the transform box
/// before the tool host runs, so their presses never reach it.
pub struct TextEditorTool<'a, H: TextEditorHost> {
    pub editor: &'a mut TextEditor,
    // `CanvasTool::paint` takes `&self` and the host's layout calls take `&mut self`.
    host: RefCell<&'a mut H>,
}

impl<'a, H: TextEditorHost> TextEditorTool<'a, H> {
    pub fn new(editor: &'a mut TextEditor, host: &'a mut H) -> Self {
        TextEditorTool { editor, host: RefCell::new(host) }
    }
}

impl<H: TextEditorHost> CanvasTool for TextEditorTool<'_, H> {
    fn cursor(&self, _cx: &ToolContext<'_>) -> ToolCursor {
        let vertical = self.host.borrow().vertical();
        ToolCursor::System(if vertical { CursorIcon::VerticalText } else { CursorIcon::Text })
    }

    fn press(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) -> ToolAction {
        if self.editor.press(*self.host.get_mut(), pointer.doc, pointer.screen, pointer.modifiers, pointer.time) {
            ToolAction::Claimed
        } else {
            ToolAction::Ignored
        }
    }

    fn drag(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.editor.drag(*self.host.get_mut(), pointer.doc);
    }

    fn release(&mut self, _pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        self.editor.release();
    }

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        self.editor.release();
    }

    /// Keys the editor took are already out of the input; what is left are the ⌘ chords it does
    /// not know, which stay for the app.
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        cx.keep_keys();
        !key.modifiers.command
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        self.editor.paint(painter, cx.view, *self.host.borrow_mut());
    }
}
