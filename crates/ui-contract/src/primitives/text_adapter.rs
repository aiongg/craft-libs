//! The text adapter: runs ui-rt's text editing engine (`craft_ui_rt::text_edit`) for a text part inside its
//! component's run. It applies the run's queued input in arrival order (the IME engine's preedits and commits, the key
//! stage's typing, moves, deletes and edit commands, pasted text, the pointer on the part), lays out the shown text,
//! records the engine's paint for the node and what happened for the pass's typing run (18.2).

mod completion;

use crate::contract::CxCore;
use crate::contract::rt::node::NodeInput;
use crate::primitives::{Services, SubmitKey, TabKey, TextLayout, TextOptions, TextRun, Wrap};
use crate::runtime::access::text::{Line, Run};
use crate::runtime::focus::{EditSet, KeepKeys, KeyUse};
use crate::runtime::motion::Animate as _;
use crate::style::Span;
use craft_behavior::{EditCommand, EditorInput, Granularity, Intent, NavMove, PartKey, PointerButton, TextPos};
use craft_keymap::Mods;
use craft_ui_rt::text_edit::{EditConfig, EditLayout, EditLook, EditPaint, Geometry, TextEdit};
use craft_ui_rt::{Gesture, Pos2, Rect, Vec2};
use egui::Color32;
use std::any::Any;
use std::sync::Arc;

/// The caret's width in points: the stroke the fields have always drawn.
const CARET_WIDTH: f32 = 2.0;

/// How a typing run ended, as the adapter saw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum End {
    /// Enter, or the `TextOptions::submit_key` chord in a multi-line field that submits.
    Submit,
    /// A commit-on character was typed.
    Separator,
    /// Focus left the part.
    Blur,
    /// Escape.
    Escape,
    /// The component put the text back as it was before the run (`Cx::set_draft` with `None`): the run ends as
    /// Escape ends it, but the Escape is not the field's.
    Revert,
    /// The draft becomes the codec's normal form of its value and is selected; nothing applies and the run stays
    /// open. `refuses` for a Tab on a `TabKey::Rewrites` part, which alerts; a press that holds the draft does not.
    Rewrite {
        /// The rewrite alerts.
        refuses: bool,
    },
}

/// A change of the draft the component asked for, applied in the next pass (`Cx::set_draft`).
#[derive(Clone, Debug)]
pub(crate) enum DraftChange {
    /// The draft becomes this text, as a change of the typing run.
    Text(String),
    /// The run ends with the text from before it.
    Revert,
}

/// The enclosing focus scope's Escape rules, which a dialog or palette provides to the nodes it holds
/// (`craft_ui_rt::BuildCx::provide`); outside one every rule is off.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScopeEscape {
    /// Escape reaches a dialog that closes on it.
    pub closes: bool,
    /// The dialog commits typing on Escape (`FocusPolicy::escape_keeps_typing`).
    pub keeps_typing: bool,
    /// The dialog is modal: a draft leaves Escape to its Cancel.
    pub modal: bool,
}

/// What the adapter keeps per text part between runs, in the component's node memory.
#[derive(Clone, Default)]
pub(crate) struct TextState {
    pub draft: Option<String>,
    /// The value when the run began (a `K::Value`).
    pub start: Option<Arc<dyn Any + Send + Sync>>,
    /// The value the run last showed or applied (a `K::Value`): a different value at the next run was set outside
    /// the run, which ends it.
    pub seen: Option<Arc<dyn Any + Send + Sync>>,
    /// The draft changed since the typing run last read it.
    pub dirty: bool,
    pub end: Option<End>,
    /// The modifiers of the Enter that ended the run with `End::Submit`.
    pub submit_mods: Mods,
    pub invalid: bool,
    /// The text a commit left, shown for the run after it.
    pub normalised: Option<String>,
    /// The editing engine: caret, selection, composition, scroll and undo steps.
    pub edit: TextEdit,
    /// Last run's layout, where it sat and the view it showed in: the pointer, row moves and the access mapping
    /// read it.
    pub placed: Option<Placed>,
    /// The rows of last run's text, for mapping an AccessKit selection back to bytes; kept only while the access
    /// tree is on.
    pub rows: Option<Rows>,
    pub caret: Option<usize>,
    pub focused: bool,
    /// Where the held pointer last was (window coordinates) while a drag selects: a build without pointer input
    /// scrolls on from it while it is past the view's edge.
    pub drag_at: Option<Pos2>,
    pub completions: completion::Completions,
    /// The draft change `Cx::set_draft` asked for, applied at the start of the next run.
    pub change: Option<DraftChange>,
    /// `Cx::select_all_text` asked for the whole text to be selected at the start of the next run.
    pub select_all: bool,
    /// Focus arrived on a `select_on_focus` part with a pointer press that is still down: the press keeps the caret or
    /// range the pointer makes, and its release as a plain click selects the whole text, as an address bar does.
    pub select_on_click: bool,
    /// The selection, `(caret, anchor)` in bytes, the text part had when its edit menu, or a menu bar or menu the
    /// keyboard goes back to it from, took the focus: the menu's command acts on it once the focus is back.
    pub held_selection: Option<(usize, usize)>,
}

/// A laid-out text part: the engine's layout, the unscrolled text origin and the view, in window coordinates.
#[derive(Clone)]
pub(crate) struct Placed {
    pub layout: Arc<EditLayout>,
    pub origin: Pos2,
    pub view: Rect,
}

impl Placed {
    fn geometry(&self, edit: &TextEdit) -> Geometry<'_> {
        Geometry { layout: &self.layout, view: self.view, origin: self.origin, scroll: edit.scroll() }
    }
}

/// The keys the text part keeps, from its options and whether it holds a draft (`drafting`).
pub(crate) fn keys_of(base: KeyUse, options: &TextOptions, drafting: bool) -> KeyUse {
    // A part whose Tab inserts or reports keeps Tab, so the focus service never moves focus on it. A part whose Tab
    // rewrites keeps it only while it holds a draft to rewrite; with nothing typed, Tab moves focus on.
    let keeps_tab = match options.tab {
        TabKey::Moves => false,
        TabKey::Rewrites => drafting,
        TabKey::Inserts | TabKey::Reports => true,
    };
    let keys = if keeps_tab {
        base.with_keep(KeepKeys::Tab)
    } else if options.multiline() {
        base.with_keep(KeepKeys::BlockArrows)
    } else {
        base
    };
    let keys = keys.with_value(base.value || options.steps_value);
    let keys = if options.holds_drafts { keys.with_holds_drafts() } else { keys };
    // A multi-line part makes a plain Enter a line break.
    if options.submits || options.multiline() { keys.with_submits() } else { keys }
}

/// `keys`, the text part's, with the edits the engine cannot run now inactive: with no selection Cut, Copy and Delete,
/// and Undo and Redo while it has no step for them, so their keys and menu picks still reach the part and the edit
/// menu shows them disabled. An Undo or Redo with no step goes on to the app command. The selection is the one the
/// part acts on: the one it holds for its edit menu or a menu bar, else the engine's.
pub(crate) fn with_idle(state: &TextState, keys: KeyUse) -> KeyUse {
    let selected = state.held_selection.map_or_else(|| !state.edit.selection().is_empty(), |(caret, anchor)| caret != anchor);
    let (undo, redo) = state.edit.history();
    let idle = [
        (EditCommand::Cut, !selected),
        (EditCommand::Copy, !selected),
        (EditCommand::Delete, !selected),
        (EditCommand::Undo, !undo),
        (EditCommand::Redo, !redo),
    ]
    .into_iter()
    .filter(|(command, idle)| *idle && keys.edits.contains(*command))
    .map(|(command, _)| command);
    keys.with_inactive(idle.fold(keys.inactive, EditSet::with))
}

/// What the text step hands the adapter for one run.
pub(crate) struct AdapterIn<'a> {
    pub services: &'a dyn Services,
    pub key: PartKey,
    /// The text part's ui-rt part id, which its presses carry.
    pub part_id: u64,
    pub rect: Rect,
    pub shown: &'a str,
    pub options: &'a TextOptions,
    pub keys: KeyUse,
    pub read_only: bool,
    pub enabled: bool,
    /// The part takes the pointer: presses, drags and clicks. A part whose component scrubs by drag leaves the
    /// pointer to the component until the field is in edit.
    pub pointer: bool,
    pub rtl: bool,
    /// The text part holds key focus.
    pub focused: bool,
    /// The text part's own edit menu is open and holds the focus: losing the focus to it ends no typing run, and
    /// getting it back is no new focus.
    pub held: bool,
    /// The part keeps the selection it had for the surface that holds the focus.
    pub keeps_selection: bool,
    pub ink: Color32,
    /// A list the component opened on the text is shown (a dropdown, the completions): Escape is the list's, except
    /// in a clearing field that still shows text.
    pub list_open: bool,
    pub placeholder: Option<&'a str>,
    /// The width of the preedit underline.
    pub underline: f32,
    /// The app placed the caret this run ([`place_caret`]): the part scrolls it into view.
    pub caret_placed: bool,
    /// The enclosing scope's Escape rules.
    pub scope: ScopeEscape,
}

pub(crate) struct AdapterOut {
    pub layout: TextLayout,
    pub focused: bool,
    /// A `TabKey::Reports` part took Tab (`Some(true)`) or Shift+Tab (`Some(false)`) this run.
    pub tab: Option<bool>,
    /// A primary click on the text ended this run.
    pub clicked: bool,
    /// The window y of the first row's baseline; `None` while the text is empty.
    pub baseline: Option<f32>,
    /// What the node paints for the part until its next run.
    pub paint: EditPaint,
    /// The caret in window coordinates, while focused.
    pub caret: Option<Rect>,
    /// Undo and Redo the engine had no step for: the text step fires the app's commands for them.
    pub unhandled: Vec<EditCommand>,
}

/// The rows of a laid-out text as AccessKit lines, with the byte where each starts in the text.
#[derive(Clone)]
pub(crate) struct Rows {
    pub lines: Vec<Line>,
    starts: Vec<usize>,
}

impl Rows {
    /// The rows of `layout` over `text`, placed from `origin`. A newline between two rows belongs to neither.
    pub(crate) fn of(layout: &TextLayout, text: &str, origin: Pos2) -> Rows {
        let (mut lines, mut starts, mut at) = (Vec::new(), Vec::new(), 0);
        for (row, rect, runs) in layout.row_lines() {
            starts.push(at);
            at += row.len();
            if text.get(at..).is_some_and(|rest| rest.starts_with('\n')) {
                at += 1;
            }
            let runs = runs.into_iter().map(|run| Run { bytes: run.bytes, rtl: run.rtl, positions: run.positions, widths: run.widths }).collect();
            lines.push(Line { text: row, rect: rect.translate(origin.to_vec2()), runs });
        }
        Rows { lines, starts }
    }

    /// The position of byte `byte` of the text.
    pub(crate) fn pos(&self, byte: usize) -> TextPos {
        let line = self.starts.iter().rposition(|start| *start <= byte).unwrap_or(0);
        TextPos::new(line, byte.saturating_sub(self.starts.get(line).copied().unwrap_or(0)))
    }

    /// The byte of `pos` in the text.
    pub(crate) fn byte(&self, pos: TextPos) -> usize {
        self.starts.get(pos.line).copied().unwrap_or(0) + pos.byte
    }
}

pub(super) fn byte_of(text: &str, chars: usize) -> usize {
    text.char_indices().nth(chars).map_or(text.len(), |(byte, _)| byte)
}

/// The byte offset `bound` moved onto the character boundary at or before it, inside `text`.
fn boundary(text: &str, bound: usize) -> usize {
    let mut at = bound.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// Places the bound caret `bound` (a byte offset into `text`) when it differs from the engine's caret: the part
/// reported its caret through the binding last run, so a different bound value is one the app wrote. The selection
/// collapses there. Returns whether it moved the caret.
pub(crate) fn place_caret(state: &mut TextState, text: &str, bound: Option<usize>) -> bool {
    let Some(bound) = bound else { return false };
    let at = boundary(text, bound);
    if state.edit.caret() == at && state.edit.anchor() == at {
        return false;
    }
    state.edit.select(text, at, at);
    true
}

/// Select All or Copy on the engine `edit` over read-only `text`: the edit commands read-only text takes. A Cut that
/// reaches it copies, as read-only text removes nothing.
pub(crate) fn select_or_copy(core: &CxCore<'_>, edit: &mut TextEdit, text: &str, command: EditCommand) {
    match command {
        EditCommand::SelectAll => edit.select_all(text),
        EditCommand::Copy | EditCommand::Cut if !edit.selection().is_empty() => core.context().copy_text(text[edit.selection()].to_owned()),
        _ => {}
    }
}

/// The modifiers of the Enter chord that breaks the line under `submit_key`: under `Enter` the line break is
/// Shift+Enter and a plain Enter submits; under `CommandEnter` a plain Enter breaks the line.
fn breaks_line(submit_key: SubmitKey, submits: bool, shift: bool, force: bool) -> bool {
    match submit_key {
        _ if !submits => !force,
        SubmitKey::Enter => shift,
        SubmitKey::CommandEnter => !force,
    }
}

/// What the queue walk of one run found besides the engine's edits.
#[derive(Default)]
struct Walked {
    edited: bool,
    separated: bool,
    stepped: bool,
    /// The submitting Enter's modifiers.
    submitted: Option<Mods>,
    tab: Option<bool>,
    clicked: bool,
    /// The pointer moved or pressed on the part: the caret follows it into view.
    pointed: bool,
    /// Undo and Redo the engine had no step for, for the app's commands.
    unhandled: Vec<EditCommand>,
}

/// Applies the run's queued input to `text` in arrival order.
fn walk(core: &mut CxCore<'_>, state: &mut TextState, input: &AdapterIn<'_>, text: &mut String, config: &EditConfig) -> Walked {
    let mut out = Walked::default();
    let options = input.options;
    let node_origin = core.rect.min.to_vec2();
    let keep = input.keys.keep;
    let queue: Vec<NodeInput> = core.run.input.to_vec();
    for (place, item) in queue.into_iter().enumerate() {
        let editor = match item {
            NodeInput::Ime(ime) => Some(ime),
            NodeInput::Paste(pasted) if input.focused && !input.read_only => {
                let pasted: &str = &pasted;
                let applied = if options.whole_paste.as_ref().is_some_and(|whole| whole(pasted)) {
                    state.edit.select_all(text);
                    state.edit.paste(text, pasted.trim(), config)
                } else {
                    state.edit.paste(text, pasted, config)
                };
                out.edited |= applied.text_changed;
                None
            }
            NodeInput::Intent { intent, shift, .. } if input.focused => match intent {
                Intent::Submit | Intent::SubmitForce if state.edit.preedit().is_none() => {
                    let force = matches!(intent, Intent::SubmitForce);
                    if options.multiline() && breaks_line(options.submit_key, options.submits, shift, force) {
                        core.intents.take_editor_at(place, keep);
                        Some(EditorInput::newline(shift))
                    } else if !options.multiline() || options.submits {
                        if core.intents.take_editor_at(place, keep).is_some() || force {
                            out.submitted = Some(Mods { shift, cmd: force, ..Mods::NONE });
                        }
                        None
                    } else {
                        None
                    }
                }
                Intent::Tab | Intent::BackTab if state.edit.preedit().is_none() => {
                    let back = matches!(intent, Intent::BackTab);
                    match options.tab {
                        TabKey::Rewrites if !input.read_only && state.draft.is_some() => {
                            if core.intents.take_editor_at(place, keep).is_some() {
                                state.end = Some(End::Rewrite { refuses: true });
                            }
                            None
                        }
                        TabKey::Reports => {
                            core.intents.take_editor_at(place, keep);
                            out.tab = Some(!back);
                            None
                        }
                        TabKey::Inserts if !back => core.intents.take_editor_at(place, keep).map(|_| EditorInput::Text(Arc::from("\t"))),
                        _ => None,
                    }
                }
                Intent::Text(_) | Intent::Move(..) | Intent::Delete { .. } | Intent::Edit(_) => core.intents.take_editor_at(place, keep),
                _ => None,
            },
            // A wheel over a multi-line part scrolls its text; the caret stays where it is.
            NodeInput::Wheel(wheel) if config.multiline && input.pointer && !core.run.wheel_taken.get() => {
                if let Some(placed) = state.placed.clone()
                    && state.edit.scroll_by(-wheel.scroll, &placed.geometry(&state.edit))
                {
                    core.run.wheel_taken.set(true);
                }
                None
            }
            NodeInput::Pointer { gesture, pos, button, mods, part, .. } if input.pointer && part == input.part_id => {
                let at = pos + node_origin;
                let Some(placed) = state.placed.clone() else { continue };
                match gesture {
                    Gesture::Down { clicks } => {
                        let selection = state.edit.selection();
                        // A secondary press inside the selection keeps it for the edit menu it opens, as native
                        // fields do.
                        let inside = {
                            let hit = placed.layout.to_text(placed.geometry(&state.edit).hit(at));
                            !selection.is_empty() && (selection.start..=selection.end).contains(&hit)
                        };
                        if button == PointerButton::Primary || !inside {
                            state.edit.press(text, &placed.geometry(&state.edit), at, clicks, mods.shift && button == PointerButton::Primary);
                            out.pointed = true;
                        }
                    }
                    Gesture::DragStart { .. } | Gesture::Drag { .. } => {
                        state.edit.drag(text, &placed.geometry(&state.edit), at, config);
                        state.drag_at = Some(at);
                        out.pointed = true;
                    }
                    Gesture::Up { click, .. } => {
                        state.edit.release();
                        state.drag_at = None;
                        out.clicked |= click && button == PointerButton::Primary;
                    }
                    Gesture::Cancel(_) => {
                        state.edit.release();
                        state.drag_at = None;
                    }
                    Gesture::LongPress => {}
                }
                None
            }
            _ => None,
        };
        let Some(editor) = editor else { continue };
        // An AccessKit selection names rows and bytes of last run's text.
        if let EditorInput::Select { anchor, focus, .. } = &editor {
            if let Some(rows) = &state.rows {
                let (anchor, focus) = (rows.byte(*anchor), rows.byte(*focus));
                state.edit.select(text, anchor, focus);
            }
            continue;
        }
        // A commit-on character ends the run instead of being typed.
        if let EditorInput::Text(typed) = &editor
            && !input.read_only
            && state.edit.preedit().is_none()
            && typed.chars().next().is_some_and(|c| options.commit_on.contains(&c))
        {
            out.separated = true;
            continue;
        }
        if let EditorInput::Move(mv, mods) = &editor {
            // An Arrow or Page key on a stepping field with an unapplied draft applies the draft first.
            let steps = matches!(mv, NavMove::Up | NavMove::Down | NavMove::PageUp | NavMove::PageDown);
            if steps && options.steps_value && !options.multiline() && !input.read_only && state.draft.is_some() {
                out.stepped = true;
            }
            // `TextOptions::arrows_from_end`: a plain Left or Right over the whole text starts from its end.
            let whole = state.edit.selection() == (0..text.len());
            if options.arrows_from_end && !mods.extend && mods.by == Granularity::Unit && matches!(mv, NavMove::Back | NavMove::Forward) && whole {
                state.edit.select(text, text.len(), text.len());
            }
        }
        let geometry = state.placed.clone();
        let applied = state.edit.apply(text, &editor, config, geometry.as_ref().map(|p| p.geometry(&state.edit)).as_ref());
        out.edited |= applied.text_changed;
        if let Some(copy) = applied.copy {
            core.context().copy_text(copy);
        }
        if applied.paste {
            core.context().request_paste();
        }
        out.unhandled.extend(applied.unhandled);
    }
    // A drag held past the view's edge scrolls on while the pointer stays out.
    if state.edit.selecting()
        && let (Some(at), Some(placed)) = (state.drag_at, state.placed.clone())
        && !out.pointed
        && !placed.view.contains(at)
    {
        state.edit.drag(text, &placed.geometry(&state.edit), at, config);
        out.pointed = true;
    }
    out
}

/// Runs the text part for one run.
pub(crate) fn run(core: &mut CxCore<'_>, state: &mut TextState, input: AdapterIn<'_>) -> AdapterOut {
    let focused_before = input.focused;
    if input.keeps_selection {
        if state.held_selection.is_none() {
            state.held_selection = Some((state.edit.caret(), state.edit.anchor()));
        }
    } else if input.focused {
        if let Some((caret, anchor)) = state.held_selection.take() {
            let text = state.draft.clone().unwrap_or_else(|| input.shown.to_owned());
            state.edit.select(&text, anchor, caret);
        }
    } else if !state.focused {
        // The menu closed a run ago and the focus went elsewhere: nothing comes back to the held selection.
        state.held_selection = None;
    }
    let options = input.options;
    let config =
        EditConfig { multiline: options.multiline(), mask: options.mask, limit: options.limit, read_only: input.read_only, os: core.context().os() };
    // An idle part shows its resting text, if it has one; focus (or the edit menu holding it) and any draft show the
    // value's own text. A run that edits an idle part's text edits the value's own text.
    let edits_idle = options.replace_selection.is_some() || state.change.is_some();
    let resting = options.resting.as_deref().filter(|_| !input.focused && !input.held && !edits_idle);
    let mut base =
        state.draft.clone().or_else(|| state.normalised.take()).or_else(|| resting.map(str::to_owned)).unwrap_or_else(|| input.shown.to_owned());
    if state.draft.is_none() && !input.focused {
        // The text is the value's: the engine's undo steps start again from it.
        state.edit.reset(&base);
    }
    let mut edited = false;
    if let Some(insert) = &options.replace_selection {
        state.edit.replace_range(&mut base, state.edit.selection(), insert);
        edited = true;
    }
    let walked = walk(core, state, &input, &mut base, &config);
    edited |= walked.edited;
    if let Some((range, insert)) = state.completions.take(&base) {
        state.edit.replace_range(&mut base, range, &insert);
        edited = true;
    }
    match state.change.take() {
        Some(DraftChange::Text(text)) => {
            state.edit.replace_all(&mut base, &text);
            edited = true;
        }
        Some(DraftChange::Revert) => state.end = Some(End::Revert),
        None => {}
    }
    escape(core, state, &input, &base, focused_before);
    let env = core.env.to_env();
    let tokens = &env.tokens;
    // Lay out the shown text: masked, with the composition at the caret.
    let shown = state.edit.shown(&base, &config);
    let wrap = if config.multiline { Wrap::Wrap } else { Wrap::Line };
    let run_of = |text: &str| {
        TextRun::spans(vec![Span { mono: options.mono, ..Span::new(text.to_owned()) }]).role(options.role).wrap(wrap).align(if options.centred {
            crate::primitives::HAlign::Center
        } else if options.end_aligned {
            crate::primitives::HAlign::End
        } else {
            crate::primitives::HAlign::Start
        })
    };
    let fonts = input.services.fonts();
    let text_env = super::text::TextEnv { tokens, direction: env.direction, ink: input.ink, fonts };
    let layout = run_of(&shown.text).layout(input.services, &text_env, input.rect.width());
    let edit_layout = Arc::new(EditLayout::new(layout.rt().clone(), layout.metrics(), &shown));
    // A single line sits on the baseline of its control's line, whatever its face; several lines start at the top.
    let size = layout.size();
    let free = super::text::free_room_before(
        if options.centred {
            crate::primitives::HAlign::Center
        } else if options.end_aligned {
            crate::primitives::HAlign::End
        } else {
            crate::primitives::HAlign::Start
        },
        input.rtl,
    );
    let x = input.rect.min.x + ((input.rect.width() - size.x) * free).max(0.0);
    let y = if config.multiline {
        input.rect.min.y
    } else {
        super::text::line_baseline(input.services, &text_env, options.role, input.rect.center().y) - layout.first_baseline
    };
    let placed = Placed { layout: edit_layout, origin: Pos2::new(x, y), view: input.rect };
    // The caret is revealed only when it moved, text changed or the app placed it, so a wheel that scrolled it out
    // of view stays where it put the text.
    let moved = Some(state.edit.caret()) != state.caret;
    if input.caret_placed || edited || moved || walked.pointed {
        state.edit.reveal(&placed.geometry(&state.edit));
    } else {
        state.edit.scroll_by(Vec2::ZERO, &placed.geometry(&state.edit));
    }
    if state.edit.selecting() && state.drag_at.is_some_and(|at| !input.rect.contains(at)) {
        core.wake_at(core.now());
    }
    let now = core.now();
    if edited || moved {
        state.edit.moved(now);
    }
    state.caret = Some(state.edit.caret());
    // The placeholder shows while the text is empty and nothing composes; it is the faint text colour, the muted one
    // on a quiet field and the disabled one when the field is off.
    let colors = &tokens.color.text;
    let placeholder = input.placeholder.filter(|_| shown.text.is_empty()).map(|hint| {
        let color = match (input.enabled, options.look == crate::style::FieldLook::Quiet) {
            (false, _) => colors.disabled,
            (true, true) => colors.muted,
            (true, false) => colors.faint,
        };
        run_of(hint).color(crate::style::StyleColor(crate::style::rgba_color(color))).layout(input.services, &text_env, input.rect.width())
    });
    let look = EditLook {
        selection: tokens.color.accent.soft,
        caret: colors.normal,
        caret_width: CARET_WIDTH,
        underline: craft_ui_rt::Rgba(input.ink.to_srgba_unmultiplied()),
        underline_width: input.underline,
        focused: input.focused,
        blinks: crate::runtime::motion::blinks(&core.env.motion()),
    };
    let geometry = placed.geometry(&state.edit);
    let paint = state.edit.paint(placeholder.as_ref().map_or_else(|| layout.rt(), |hint| hint.rt()), &geometry, look);
    let caret = paint.caret.map(|caret| caret.translate(paint.origin.to_vec2()));
    state.placed = Some(placed);
    let typing = input.focused && state.edit.preedit().is_none() && !input.read_only;
    if edited {
        state.draft = Some(base.clone());
        state.dirty = true;
    }
    let at = state.draft.as_deref().filter(|_| typing).map(|text| (text, text[..boundary(text, state.edit.caret())].chars().count()));
    state.completions.refresh(options.complete.as_deref(), at);
    let lost_focus = focused_before && !input.focused;
    if let Some(mods) = walked.submitted {
        state.end = Some(End::Submit);
        state.submit_mods = mods;
    } else if walked.separated || (walked.stepped && state.end.is_none()) {
        state.end = Some(End::Separator);
    } else if (lost_focus || (state.focused && !input.focused)) && !input.held && state.draft.is_some() && state.end.is_none() {
        state.end = Some(End::Blur);
    }
    state.focused = input.focused || input.held;
    let baseline = (!shown.text.is_empty()).then_some(y + layout.first_baseline);
    AdapterOut { layout, focused: input.focused, tab: walked.tab, clicked: walked.clicked, baseline, paint, caret, unhandled: walked.unhandled }
}

/// The Escape rules of a text part. A clearing field takes Escape while it shows text, with or without a draft, and
/// passes it on once it is empty: a list open on it then gets the Escape that finds the field empty and closes. A draft
/// in a dialog whose Escape closes it does not take the Escape: the one Escape cancels the dialog and the typing with
/// it. Outside dialogs Escape reverts the draft first. In a modal dialog a draft leaves Escape to the dialog's Cancel.
/// A field that keeps its text on Escape takes every Escape while editable and hands it on after committing.
fn escape(core: &mut CxCore<'_>, state: &mut TextState, input: &AdapterIn<'_>, base: &str, focused_before: bool) {
    let options = input.options;
    if !focused_before || !core.intents.untaken(&Intent::Cancel) {
        return;
    }
    let composing = state.edit.preedit().is_some();
    let keep_on_escape = options.keep_on_escape && !options.clear_on_escape && input.enabled && !input.read_only;
    let closes_dialog = !keep_on_escape && !composing && !options.clear_on_escape && input.scope.closes;
    let dialog_escape = closes_dialog && state.draft.is_some();
    if dialog_escape {
        // The draft ends as for Escape (its typing is dropped), and the Escape stays for the dialog. A dialog that keeps
        // typing on Escape commits the draft as leaving the field does.
        state.end = Some(if input.scope.keeps_typing { End::Blur } else { End::Escape });
    }
    let draft_takes = state.draft.is_some() && !dialog_escape && !input.scope.modal;
    // With `RootEnv::escape_leaves_fields` a field takes Escape with nothing to revert too, and gives up the focus;
    // in a dialog that Escape closes the Escape stays the dialog's and the field keeps the focus.
    let leaves = !options.clear_on_escape && !input.scope.closes && core.env.to_env().escape_leaves_fields;
    let takes_escape = keep_on_escape || if options.clear_on_escape { !base.is_empty() || composing } else { draft_takes || composing || leaves };
    let list_first = input.list_open && !options.clear_on_escape;
    if takes_escape && !state.completions.open && !list_first && core.intents.take(Intent::Cancel) {
        // Escape over a composition alone ends the composition and leaves the text.
        if state.draft.is_some() || !composing {
            state.end = Some(End::Escape);
            if leaves {
                core.surrender_focus();
            }
        }
        state.edit.end_preedit();
    }
}
