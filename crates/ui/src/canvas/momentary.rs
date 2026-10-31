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
//! [`CanvasTool::key`](super::CanvasTool::key)), not a momentary tool.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Context, Id, InputState, Key, KeyboardShortcut, Modifiers};

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
/// # fn demo(ctx: &egui::Context, host: egui::Id, in_gesture: bool) {
/// let keys = MomentaryKeys::default();
/// match keys.update(ctx, host, in_gesture) {
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
    // pc: crates/engine/src/prefs.rs `TEMPORARY_TOOLS`.
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
    /// Cmd, or Cmd then Space): Zoom Out wins over Zoom In when Alt is added, Zoom In over the
    /// Hand. Pure; ignores whether a text field has the keys.
    // pc: crates/ui-egui/src/input/hold_keys.rs:163 `held`, `active`.
    pub fn held(&self, input: &InputState) -> Option<MomentaryTool> {
        todo!()
    }

    /// The temporary tool for this pass on the canvas whose tool host is `host`: the held keys
    /// while no text field has the keyboard, latched while `in_gesture` (a drag started with the
    /// Hand finishes with the Hand even after Space comes up; a drag started with the selected
    /// tool ignores Space pressed during it). Consumes Space so a focused button does not click.
    // pc: crates/ui-egui/src/input/hold_keys.rs `update` (latch), canvas.rs temporary tool.
    pub fn update(&self, ctx: &Context, host: Id, in_gesture: bool) -> Option<MomentaryTool> {
        todo!()
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
        todo!()
    }

    /// The canvas was pressed while a letter is held: the switch is temporary however short the
    /// hold.
    // pc: crates/ui-egui/src/input/spring_tools.rs:86 `note_canvas_press`.
    pub fn canvas_used(&mut self) {
        todo!()
    }

    /// Once a pass: the tool to switch back to now, if every held letter is up, the switch was
    /// temporary, `current` is still the sprung tool and no gesture runs (`in_gesture`).
    // pc: crates/ui-egui/src/input/spring_tools.rs:94 `tick`.
    pub fn tick(&mut self, input: &InputState, current: &T, in_gesture: bool) -> Option<T> {
        todo!()
    }

    /// The tool a release would give back, while a spring is pending (for automation and tests).
    pub fn previous(&self) -> Option<&T> {
        self.spring.as_ref().map(|s| &s.previous)
    }
}
