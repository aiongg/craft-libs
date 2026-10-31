//! Temporary tools, as in Photoshop: tools in effect only while a key is held.
//!
//! - [`MomentaryKeys`]: Space is the Hand while held, Cmd+Space (Ctrl+Space off the Mac) Zoom In,
//!   Alt+Space Zoom Out; the keys work in any order, and Alt added to a held Zoom In zooms out.
//! - [`SpringTool`]: a tool letter held for [`SpringTool::HOLD`] seconds, or used on the canvas
//!   while it is down, switches tools only until it comes up.
//!
//! The current tool never changes for a momentary key: the app passes the temporary tool to
//! [`ToolHost::show`](super::ToolHost::show) instead of the selected one, and a drag that
//! started with it finishes with it ([`ToolHost::in_gesture`](super::ToolHost::in_gesture)).
//! Space during a marquee, lasso or shape drag repositions it: that is the tool's own key (its
//! [`CanvasTool::key`](super::CanvasTool::key)), not a momentary tool. PhotoCraft's Cmd-held Move
//! tool and the Brush's erase key are tool behaviour (step 7), not here.

use egui::{Context, Id, InputState, Key, KeyboardShortcut, Modifiers};

/// Is `sc` held: its key down with at least its modifiers (more may be held, so Shift still
/// squares a marquee while Space is down).
fn shortcut_held(input: &InputState, sc: &KeyboardShortcut) -> bool {
    let (want, m) = (sc.modifiers, input.modifiers);
    input.key_down(sc.logical_key)
        && (!want.alt || m.alt)
        && (!want.shift || m.shift)
        && (!want.command || m.command)
        && (!want.ctrl || m.ctrl)
        && (!want.mac_cmd || m.mac_cmd)
}

/// A tool in effect only while its keys are held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MomentaryTool {
    Hand,
    ZoomIn,
    ZoomOut,
}

impl MomentaryTool {
    /// The keymap id of the binding (Edit › Keyboard Shortcuts › Tools › Temporary).
    // pc: crates/ui-egui/src/input/hold_keys.rs:48 `Temporary::id`.
    pub fn command_id(self) -> &'static str {
        match self {
            MomentaryTool::Hand => "tools.temporary.hand",
            MomentaryTool::ZoomIn => "tools.temporary.zoomIn",
            MomentaryTool::ZoomOut => "tools.temporary.zoomOut",
        }
    }
}

/// The momentary keys and their bindings. The bindings are the app's keymap entries for
/// [`MomentaryTool::command_id`]; [`Default`] is Photoshop's.
///
/// ```no_run
/// use craft_ui::canvas::{MomentaryKeys, MomentaryTool};
/// # fn demo(ctx: &egui::Context, canvas: egui::Id, drawing: bool) {
/// let keys = MomentaryKeys::default();
/// match keys.update(ctx, canvas, drawing) {
///     Some(MomentaryTool::Hand) => { /* show the Hand tool this pass */ }
///     Some(MomentaryTool::ZoomIn | MomentaryTool::ZoomOut) => { /* the Zoom tool, in or out */ }
///     None => { /* the selected tool */ }
/// }
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MomentaryKeys {
    pub hand: KeyboardShortcut,
    pub zoom_in: KeyboardShortcut,
    pub zoom_out: KeyboardShortcut,
}

impl Default for MomentaryKeys {
    /// Space; Cmd+Space (Ctrl+Space off the Mac); Alt+Space.
    // pc: crates/engine/src/prefs/shortcuts.rs:115 `TEMPORARY_TOOLS`.
    fn default() -> Self {
        Self {
            hand: KeyboardShortcut::new(Modifiers::NONE, Key::Space),
            zoom_in: KeyboardShortcut::new(Modifiers::COMMAND, Key::Space),
            zoom_out: KeyboardShortcut::new(Modifiers::ALT, Key::Space),
        }
    }
}

impl MomentaryKeys {
    /// The temporary tool the keys held in `input` ask for, in any order of pressing (Space then
    /// Cmd, or Cmd then Space). Of the bindings whose key is down and whose modifiers are all held,
    /// the one with the most modifiers wins (Cmd+Space beats Space); Alt held with Zoom In turns it
    /// into Zoom Out unless Alt is part of Zoom In's own binding (Cmd+Alt+Space zooms out). Pure;
    /// ignores whether a text field has the keys.
    // pc: crates/ui-egui/src/input/hold_keys.rs:176 `held_tool`, :163 `held`.
    pub fn held(&self, input: &InputState) -> Option<MomentaryTool> {
        let count = |sc: &KeyboardShortcut| {
            let m = sc.modifiers;
            u8::from(m.alt) + u8::from(m.shift) + u8::from(m.command || m.mac_cmd) + u8::from(m.ctrl && !m.command)
        };
        let mut best: Option<(u8, MomentaryTool, KeyboardShortcut)> = None;
        for (tool, sc) in [(MomentaryTool::Hand, self.hand), (MomentaryTool::ZoomIn, self.zoom_in), (MomentaryTool::ZoomOut, self.zoom_out)] {
            if shortcut_held(input, &sc) && best.is_none_or(|(n, ..)| count(&sc) > n) {
                best = Some((count(&sc), tool, sc));
            }
        }
        best.map(
            |(_, tool, sc)| {
                if tool == MomentaryTool::ZoomIn && input.modifiers.alt && !sc.modifiers.alt { MomentaryTool::ZoomOut } else { tool }
            },
        )
    }

    /// The temporary tool for this pass on the canvas `canvas`: [`held`](Self::held) while no
    /// text field has the keyboard, except while `drawing` (a marquee, lasso or shape is being
    /// dragged: Space repositions it instead). A temporary tool in effect when the primary button
    /// went down stays while it is down and on the pass it is released (the Move tool's release
    /// commits the move), even after its keys come up. The latch lives in egui memory under
    /// `canvas`.
    // pc: crates/ui-egui/src/input/hold_keys.rs:240 `for_frame`, :232 `latch_id`.
    pub fn update(&self, ctx: &Context, canvas: Id, drawing: bool) -> Option<MomentaryTool> {
        let (down, released) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.primary_released()));
        let latch = canvas.with("momentary-latch");
        let latched = ctx.data(|d| d.get_temp::<Option<MomentaryTool>>(latch)).flatten();
        let tool = if (down || released) && latched.is_some() {
            latched
        } else if drawing || ctx.text_edit_focused() {
            None
        } else {
            ctx.input(|i| self.held(i))
        };
        ctx.data_mut(|d| d.insert_temp(latch, if down { tool } else { None }));
        tool
    }
}

/// Spring-loaded tool keys: a tool letter switches tools when pressed, as always; held for
/// [`HOLD`](Self::HOLD) seconds or longer, or with the canvas used while it is down, the switch
/// was temporary, and releasing the letter gives back the tool in effect before the press. A
/// quick tap with no canvas use stays a permanent switch.
///
/// - Shift + letter (cycling a group) and Alt + letter never spring; pressing the current
///   group's own letter cycles the group and does not spring.
/// - A drag still running when the letter comes up finishes with the temporary tool; the
///   previous tool returns once the button is up.
/// - Another letter pressed while one is held wins; releasing every held letter returns the
///   original tool. If the tool changes any other way meanwhile (toolbar, menu, automation),
///   that choice stands.
///
/// `T` is the app's tool id.
#[derive(Clone, Debug, PartialEq)]
pub struct SpringTool<T> {
    spring: Option<Spring<T>>,
}

#[derive(Clone, Debug, PartialEq)]
struct Spring<T> {
    previous: T,
    tool: T,
    keys: Vec<Key>,
    since: f64,
    used: bool,
    released: bool,
}

impl<T> Default for SpringTool<T> {
    fn default() -> Self {
        Self { spring: None }
    }
}

impl<T: Clone + PartialEq> SpringTool<T> {
    /// Seconds a held letter takes to make the switch temporary.
    pub const HOLD: f64 = 0.3;

    /// A tool letter `key` was pressed at `now`, switching from `before` to `picked`. `plain` is
    /// false for Shift or Alt with the letter, or the current group's own letter (no spring).
    // pc: crates/ui-egui/src/input/spring_tools.rs:65 `pressed`.
    pub fn pressed(&mut self, key: Key, before: T, picked: T, plain: bool, now: f64) {
        let active = self.spring.as_ref().is_some_and(|s| !s.released);
        if !plain {
            self.spring = None;
        } else if active {
            if let Some(s) = self.spring.as_mut() {
                s.keys.retain(|k| *k != key);
                s.keys.push(key);
                s.used = true;
                s.tool = picked;
            }
        } else {
            self.spring = Some(Spring { previous: before, tool: picked, keys: vec![key], since: now, used: false, released: false });
        }
    }

    /// The canvas was pressed while a letter is held: the switch is temporary however short the
    /// hold.
    // pc: crates/ui-egui/src/input/spring_tools.rs:86 `note_canvas_press`.
    pub fn canvas_used(&mut self) {
        if let Some(s) = self.spring.as_mut().filter(|s| !s.released) {
            s.used = true;
        }
    }

    /// Once a pass: the tool to switch back to now, if every held letter is up, the switch was
    /// temporary, `current` is still the sprung tool and no gesture runs (`in_gesture`).
    // pc: crates/ui-egui/src/input/spring_tools.rs:94 `tick`.
    pub fn tick(&mut self, input: &InputState, current: &T, in_gesture: bool) -> Option<T> {
        let mut s = self.spring.take()?;
        if *current != s.tool {
            return None;
        }
        if !s.released {
            if in_gesture {
                s.used = true;
            }
            s.keys.retain(|k| input.key_down(*k));
            if !s.keys.is_empty() {
                self.spring = Some(s);
                return None;
            }
            if !s.used && input.time - s.since < Self::HOLD {
                return None;
            }
            s.released = true;
        }
        if in_gesture || input.pointer.any_down() {
            self.spring = Some(s);
            return None;
        }
        Some(s.previous)
    }

    /// The tool a release would give back, while a spring is pending (for automation and tests).
    pub fn previous(&self) -> Option<&T> {
        self.spring.as_ref().map(|s| &s.previous)
    }
}
