use std::ops::Range;

use egui::{Context, Event, IMEPurpose, ImeEvent, Key, Modifiers, Rect, output::IMEOutput};

/// Seconds after a composition ends in which the IME's own confirm, cancel and edit keys
/// (Enter, Escape, Backspace, Delete, Space) are still taken as the IME's. Platforms that forward
/// the confirming key do so in the same batch of events; a person's next Enter comes much later.
const GUARD_SECS: f64 = 0.05;

/// Seconds after the composer kept marked text itself (a click away, the canvas giving up the
/// keyboard) in which the IME's own commit of that same text is a duplicate.
const KEPT_SECS: f64 = 0.5;

/// One step for the canvas's text, in the order the platform sent it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    /// The marked text changed. Empty text ends the composition; an [`Edit::Insert`] of the
    /// committed text, if any, follows. `active` is the segment being converted, in chars.
    Preedit {
        text: String,
        active: Option<Range<usize>>,
    },
    /// Text to insert at the caret: typed, committed by the IME, or marked text kept as typed.
    Insert(String),
    /// A key press for the editor (caret movement, Backspace, Enter, Escape, shortcuts). Keys the
    /// IME is using never appear here.
    Key {
        key: Key,
        modifiers: Modifiers,
        repeat: bool,
    },
    Paste(String),
    Copy,
    Cut,
    /// Delete chars around the caret (the web text agent's corrections, mobile Backspace).
    DeleteSurrounding {
        before_chars: usize,
        after_chars: usize,
    },
}

/// The marked text, for painting at the caret with an underline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preedit<'a> {
    pub text: &'a str,
    /// The segment being converted (Japanese, Chinese phrase input), in chars. Draw it with a
    /// thicker underline. `None` when the IME doesn't say.
    pub active: Option<&'a Range<usize>>,
}

/// What [`Composer::process`] made of a pass's events.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Processed {
    pub edits: Vec<Edit>,
    /// The input events with those the IME used removed, for the app's other keyboard handling.
    pub events: Vec<Event>,
}

/// One composer for text typed onto a canvas, outside egui's `TextEdit`.
///
/// It turns a pass's input events into ordered [`Edit`]s, keeps the marked (preedit) text, and
/// tells the platform where the caret is so the candidate window opens there.
#[derive(Clone, Debug, Default)]
pub struct Composer {
    preedit: String,
    active: Option<Range<usize>>,
    /// When the last composition ended, for [`GUARD_SECS`].
    ended_at: Option<f64>,
    /// Marked text the composer kept itself, and when, for [`KEPT_SECS`].
    kept: Option<(String, f64)>,
    /// This pass's last commit since the last typed text, to drop the same text typed again.
    last_commit: Option<String>,
    modifiers: Modifiers,
    caret: Option<Rect>,
    purpose: IMEPurpose,
    interrupt: bool,
    on: bool,
}

impl Composer {
    pub fn new() -> Self {
        Self::default()
    }

    /// `IMEPurpose::Password` or `Terminal` for canvases that need them.
    pub fn with_purpose(mut self, purpose: IMEPurpose) -> Self {
        self.purpose = purpose;
        self
    }

    pub fn is_composing(&self) -> bool {
        !self.preedit.is_empty()
    }

    pub fn preedit(&self) -> Option<Preedit<'_>> {
        self.is_composing().then(|| Preedit { text: &self.preedit, active: self.active.as_ref() })
    }

    /// Call once per pass, before the app's other keyboard handling, with whether the canvas
    /// has a text caret. Returns the pass's edits and removes the events the IME used from
    /// `ctx`'s input, so shortcuts don't see the Space that converted a word or the Enter that
    /// confirmed it.
    ///
    /// When `editing` is false, or an egui text field has the keyboard, marked text is kept as
    /// typed (an empty [`Edit::Preedit`], then an [`Edit::Insert`]) and the IME turns off.
    pub fn update(&mut self, ctx: &Context, editing: bool) -> Vec<Edit> {
        if !editing || ctx.text_edit_focused() {
            self.on = false;
            let time = ctx.input(|i| i.time);
            let kept = self.end_at(time);
            // Turning the IME off drops its marked text; there is nothing to interrupt.
            self.interrupt = false;
            return kept.map(|text| vec![Edit::Preedit { text: String::new(), active: None }, Edit::Insert(text)]).unwrap_or_default();
        }
        self.on = true;
        let (events, time) = ctx.input(|i| (i.events.clone(), i.time));
        let processed = self.process(&events, time);
        if processed.events.len() != events.len() {
            ctx.input_mut(|i| i.events = processed.events);
        }
        self.write_output(ctx);
        processed.edits
    }

    /// The caret in screen points (`ctx.layer_transform_to_global(layer) * rect` for a canvas
    /// on a transformed layer), after this pass's edits moved it. Put it at the start of the
    /// marked text, or of its active segment, so candidates don't cover what is being typed.
    ///
    /// Call it after [`Self::update`] in every pass that edits; until the first call the
    /// candidate window has no position.
    pub fn set_caret(&mut self, ctx: &Context, caret: Rect) {
        self.caret = Some(caret_area(caret, ctx.content_rect()));
        if self.on {
            self.write_output(ctx);
        }
    }

    /// Keeps marked text as typed and tells the IME to drop its copy, for an app ending its
    /// text editing itself (a tool change, a command, Escape handled by the app). Returns the
    /// text to insert in place of the marked text before the editor closes.
    pub fn end(&mut self, ctx: &Context) -> Option<String> {
        let time = ctx.input(|i| i.time);
        let kept = self.end_at(time);
        if kept.is_some() && self.on {
            self.write_output(ctx);
        }
        kept
    }

    /// The state machine behind [`Self::update`], for apps that read events elsewhere and for
    /// tests. `time` is the pass's input time in seconds.
    pub fn process(&mut self, events: &[Event], time: f64) -> Processed {
        let mut out = Processed::default();
        // Ends of composition in this pass guard the IME's keys regardless of time.
        let mut ended_this_pass = false;
        self.last_commit = None;
        if self.kept.as_ref().is_some_and(|(_, at)| time - at > KEPT_SECS) {
            self.kept = None;
        }
        for event in events {
            let keep = match event {
                Event::Ime(ime) => {
                    self.ime(ime, time, &mut ended_this_pass, &mut out.edits);
                    false
                }
                Event::Key { key, pressed, repeat, modifiers, .. } => {
                    self.modifiers = *modifiers;
                    if !*pressed {
                        true
                    } else if self.is_composing() {
                        false
                    } else if is_ime_key(*key) && (ended_this_pass || self.ended_at.is_some_and(|at| time - at <= GUARD_SECS)) {
                        // One forwarded key per composition end: a second Enter is the user's.
                        self.ended_at = None;
                        ended_this_pass = false;
                        false
                    } else {
                        self.kept = None;
                        out.edits.push(Edit::Key { key: *key, modifiers: *modifiers, repeat: *repeat });
                        true
                    }
                }
                Event::Text(text) => {
                    // The key that produced a commit can arrive as text too (Apple Korean's Space).
                    let duplicate = self.last_commit.take().is_some_and(|c| c == *text);
                    if self.is_composing() || duplicate || is_line_break(text) || is_shortcut_chord(self.modifiers) {
                        false
                    } else {
                        self.kept = None;
                        out.edits.push(Edit::Insert(text.clone()));
                        true
                    }
                }
                Event::Paste(_) | Event::Copy | Event::Cut if self.is_composing() => false,
                Event::Paste(text) => {
                    out.edits.push(Edit::Paste(text.clone()));
                    true
                }
                Event::Copy => {
                    out.edits.push(Edit::Copy);
                    true
                }
                Event::Cut => {
                    out.edits.push(Edit::Cut);
                    true
                }
                Event::ModifiersChanged(modifiers) => {
                    self.modifiers = *modifiers;
                    true
                }
                Event::PointerButton { pressed: true, .. } => {
                    // A click while composing keeps the marked text where it is, then the app
                    // handles the click (moving the caret or ending the edit).
                    if let Some(text) = self.end_at(time) {
                        out.edits.push(Edit::Preedit { text: String::new(), active: None });
                        out.edits.push(Edit::Insert(text));
                    }
                    true
                }
                _ => true,
            };
            if keep {
                out.events.push(event.clone());
            }
        }
        out
    }

    fn ime(&mut self, ime: &ImeEvent, time: f64, ended_this_pass: &mut bool, edits: &mut Vec<Edit>) {
        match ime {
            ImeEvent::Preedit { text, .. } if text.is_empty() => {
                // Wayland input methods send empty preedits on every state change and after
                // each commit; only one that ends a composition means anything.
                if self.is_composing() {
                    self.take_preedit(time, edits);
                    *ended_this_pass = true;
                }
            }
            ImeEvent::Preedit { text, active_range_chars } => {
                self.kept = None;
                self.last_commit = None;
                self.preedit.clone_from(text);
                self.active = active_range_chars.clone().filter(|r| r.end <= text.chars().count());
                edits.push(Edit::Preedit { text: text.clone(), active: self.active.clone() });
            }
            // A bare line break confirms the composition (Enter with some IMEs): the marked text
            // stays as typed, not a new line.
            ImeEvent::Commit(text) if is_line_break(text) => {
                if let Some(kept) = self.take_preedit(time, edits) {
                    edits.push(Edit::Insert(kept));
                    *ended_this_pass = true;
                }
            }
            ImeEvent::Commit(text) => {
                let was_composing = self.take_preedit(time, edits).is_some();
                if self.kept.take().is_some_and(|(kept, _)| kept == *text) {
                    return;
                }
                if text.is_empty() {
                    *ended_this_pass |= was_composing;
                    return;
                }
                // A commit with no composition (the emoji picker, a dictated phrase, Korean
                // Space with no syllable) is still typed text, and may be followed by the key
                // that produced it.
                *ended_this_pass = true;
                self.ended_at = Some(time);
                self.last_commit = Some(text.clone());
                edits.push(Edit::Insert(text.clone()));
            }
            ImeEvent::DeleteSurrounding { before_chars, after_chars } => {
                if !self.is_composing() {
                    edits.push(Edit::DeleteSurrounding { before_chars: *before_chars, after_chars: *after_chars });
                }
            }
            #[expect(deprecated)]
            ImeEvent::Enabled | ImeEvent::Disabled => {}
        }
    }

    fn take_preedit(&mut self, time: f64, edits: &mut Vec<Edit>) -> Option<String> {
        if !self.is_composing() {
            return None;
        }
        self.active = None;
        self.ended_at = Some(time);
        edits.push(Edit::Preedit { text: String::new(), active: None });
        Some(std::mem::take(&mut self.preedit))
    }

    /// Keeps marked text as typed, remembering it so the IME's late commit of it is dropped.
    fn end_at(&mut self, time: f64) -> Option<String> {
        if !self.is_composing() {
            return None;
        }
        let text = std::mem::take(&mut self.preedit);
        self.active = None;
        self.interrupt = true;
        self.kept = Some((text.clone(), time));
        Some(text)
    }

    fn write_output(&mut self, ctx: &Context) {
        let interrupt = std::mem::take(&mut self.interrupt);
        let rect = self.caret.unwrap_or(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0)));
        // egui-winit turns the IME on only in passes that set `ime`.
        ctx.output_mut(|o| {
            // A later write in the same pass (the caret after edits) keeps the interrupt.
            let interrupt = interrupt || o.ime.is_some_and(|ime| ime.should_interrupt_composition);
            o.ime = Some(IMEOutput { purpose: self.purpose, rect, cursor_rect: rect, should_interrupt_composition: interrupt });
        });
    }
}

/// Keys an IME uses to confirm, cancel or edit a composition.
fn is_ime_key(key: Key) -> bool {
    matches!(key, Key::Enter | Key::Escape | Key::Backspace | Key::Delete | Key::Space)
}

fn is_line_break(text: &str) -> bool {
    text == "\n" || text == "\r" || text == "\r\n"
}

/// Cmd/Ctrl chords are shortcuts: a host that still sends their letter as text doesn't type it.
/// Alt is left alone: Option on macOS and AltGr (Ctrl+Alt) on Windows type characters.
fn is_shortcut_chord(m: Modifiers) -> bool {
    (m.command || m.ctrl || m.mac_cmd) && !m.alt
}

/// A caret IMEs can place a window at: at least a point wide and tall (some ignore an empty
/// rectangle), and inside the window, so a caret scrolled off the canvas doesn't put the
/// candidates off screen.
fn caret_area(caret: Rect, screen: Rect) -> Rect {
    if !caret.is_finite() || !screen.is_positive() {
        return caret;
    }
    let size = caret.size().max(egui::vec2(1.0, 1.0)).min(screen.size());
    let min = caret.min.clamp(screen.min, screen.max - size);
    Rect::from_min_size(min, size)
}
