//! Nudging canvas objects with the arrow keys: a selection outline, a guide, a transform box or
//! an anchor (PhotoCraft: 1 pixel, Shift 10, along the document axes).
//!
//! Each press is one [`EditEvent::Set`](crate::edit::EditEvent::Set); the host bundles a run of
//! presses into one undo step (PhotoCraft's `nudge_bundle`).
//!
//! Three ways in, one parser:
//! - overlays drawn with a `Ui` call [`read`] or [`read_with_alt`];
//! - tools on the [`ToolHost`](crate::canvas::ToolHost) turn the [`ToolKey`] they were given into a
//!   [`Nudge`] with [`NudgeKeys::from_key`];
//! - an app that dispatches its shortcuts at context level (PhotoCraft's Move tool arrows, ⌘-arrows
//!   from the temporary Move tool) calls [`NudgeKeys::read`].

use egui::{Context, Event, Key, LayerId, Modifiers, Ui};
use kurbo::Vec2;

use crate::canvas::ToolKey;
use crate::focus::key_owner::{KeyOwner, OwnerGate, owner};

/// The move of one press, in document units (PhotoCraft's).
pub const STEP: f64 = 1.0;
/// The move of one press with Shift held.
pub const BIG_STEP: f64 = 10.0;

/// One arrow key press read by [`NudgeKeys`]: the move in document units along the document axes
/// and the modifiers that chose it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nudge {
    /// Right +x, Down +y; `step` or, with Shift, `big_step` long. Right to left does not swap Left
    /// and Right: the document doesn't mirror.
    pub by: Vec2,
    /// Shift was held: `by` is the big step.
    pub shift: bool,
    /// Alt was held (read only with [`NudgeKeys::alt`]): the Move tool duplicates first, a path's
    /// direction handle moves without its partner.
    pub alt: bool,
}

/// Which arrow presses are nudges and how far they move: the context-level and tool-key reader.
///
/// ```no_run
/// use craft_ui::behavior::nudge::NudgeKeys;
/// use egui::Modifiers;
/// # fn demo(ctx: &egui::Context, cmd_is_move_tool: bool) {
/// // ⌘-arrows (Ctrl elsewhere) from the temporary Move tool, Alt to copy first.
/// let base = if cmd_is_move_tool { Modifiers::COMMAND } else { Modifiers::NONE };
/// if let Some(n) = NudgeKeys::new().alt(true).base(base).read(ctx) {
///     let _ = (n.by, n.alt); // app.nudge(n.by, n.alt)
/// }
/// # }
/// ```
///
/// - A press with exactly `base`, plus Shift or (with [`alt`](Self::alt)) Alt, is read; any other
///   modifier, and ⌘ or Ctrl when `base` doesn't hold it, is not. Written as exact matches
///   (`Modifiers::matches_exact`), so no pattern order decides (PhotoCraft's `consume_key(base |
///   mods, key)` loop, most specific first).
/// - Nothing is read while a text field has focus, a popup or menu is open, or a modal layer is
///   above [`layer`](Self::layer) (default: the background layer), as [`reads`] decides for a `Ui`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NudgeKeys {
    step: f64,
    big_step: f64,
    base: Modifiers,
    alt: bool,
    layer: LayerId,
}

impl Default for NudgeKeys {
    fn default() -> Self {
        Self::new()
    }
}

impl NudgeKeys {
    /// Plain and Shift arrows, [`STEP`] and [`BIG_STEP`], no base modifier, Alt not read, the
    /// background layer.
    pub fn new() -> Self {
        Self { step: STEP, big_step: BIG_STEP, base: Modifiers::NONE, alt: false, layer: LayerId::background() }
    }

    /// The move of a plain press and of a Shift press, in document units.
    pub fn steps(mut self, step: f64, big_step: f64) -> Self {
        self.step = step;
        self.big_step = big_step;
        self
    }

    /// The modifier every read press holds: `Modifiers::COMMAND` (⌘ on macOS, Ctrl elsewhere)
    /// where ⌘ makes the temporary Move tool (PhotoCraft's `hold_keys::cmd_nudges`). Default:
    /// none, and then a press with ⌘ or Ctrl is not read.
    pub fn base(mut self, base: Modifiers) -> Self {
        self.base = base;
        self
    }

    /// Whether Alt and Alt+Shift presses are read too ([`Nudge::alt`]). Default: off, and an Alt
    /// press is left to others (an Alt-arrow kerns type).
    pub fn alt(mut self, on: bool) -> Self {
        self.alt = on;
        self
    }

    /// The layer the canvas is drawn on: a modal layer above it blocks [`read`](Self::read).
    /// Default: [`LayerId::background`].
    pub fn layer(mut self, layer: LayerId) -> Self {
        self.layer = layer;
        self
    }

    /// Consumes the first arrow press of this pass that these keys read and returns its
    /// [`Nudge`]; `None` (and nothing consumed) when no press qualifies or the keys are not free:
    /// a text field has focus, a popup or menu is open, or a modal layer is above
    /// [`layer`](Self::layer). Right +x and Down +y in every text direction.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn read(&self, ctx: &Context) -> Option<Nudge> {
        // pc: crates/ui-egui/src/tools/move_mods.rs:200 `arrow_keys` (`consume_key(base | mods, key)`
        // over SHIFT|ALT, SHIFT, ALT, NONE; 10 with Shift, else 1); the owner check is this module's
        // `reads` with `ui.layer_id()` replaced by `self.layer`.
        todo!()
    }

    /// The [`Nudge`] a key the tool host passed on means, by the same rules as
    /// [`read`](Self::read), without consuming anything: the host already gave the key to one
    /// tool. A release (`pressed` false) is `None`; a repeat is a nudge. The owner check is the
    /// host's (it passes keys only while the canvas has the keyboard).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn from_key(&self, key: &ToolKey) -> Option<Nudge> {
        // new: the parser `read` uses, applied to `ToolKey { key, modifiers, pressed, .. }`.
        todo!()
    }
}

/// Consumes one arrow key press, plain or with Shift only, and returns the move in document units
/// along the document axes: Right +x, Down +y, `step` or with Shift `big_step`. Right to left does
/// not swap Left and Right: the document doesn't mirror.
///
/// Reads nothing while a text field has focus, a popup is open or a modal layer is above `ui`.
/// Equals `NudgeKeys::new().steps(step, big_step).layer(ui.layer_id())` read from `ui`'s context.
pub fn read(ui: &Ui, step: f64, big_step: f64) -> Option<Vec2> {
    if !reads(ui) {
        return None;
    }
    ui.input_mut(|i| {
        let at = i.events.iter().position(|e| arrow(e).is_some())?;
        let (dir, shift) = arrow(&i.events[at])?;
        i.events.remove(at);
        Some(dir * if shift { big_step } else { step })
    })
}

/// [`read`] that also takes Alt and Alt+Shift, and reports whether Alt was held: an arrow with
/// Alt moves a path's direction handle alone, without its partner (PhotoCraft's ⌥-drag).
pub fn read_with_alt(ui: &Ui, step: f64, big_step: f64) -> Option<(Vec2, bool)> {
    if !reads(ui) {
        return None;
    }
    ui.input_mut(|i| {
        let at = i.events.iter().position(|e| arrow_with_alt(e).is_some())?;
        let (dir, shift, alt) = arrow_with_alt(&i.events[at])?;
        i.events.remove(at);
        Some((dir * if shift { big_step } else { step }, alt))
    })
}

/// Whether the canvas's keys are free: no text field has focus, no popup is open and no modal layer
/// is above `ui`.
pub fn reads(ui: &Ui) -> bool {
    let ctx = ui.ctx();
    // A modal layer makes the owner `Dialog`; the layer check then tells the dialog's own ui from the canvas under it.
    let free = matches!(owner(ctx, &OwnerGate::default()), KeyOwner::App | KeyOwner::Widget | KeyOwner::Dialog);
    free && ctx.memory(|m| m.allows_interaction(ui.layer_id()))
}

/// An arrow key press plain or with Shift: its unit direction and whether Shift is held.
fn arrow(event: &Event) -> Option<(Vec2, bool)> {
    match arrow_with_alt(event)? {
        (dir, shift, false) => Some((dir, shift)),
        _ => None,
    }
}

/// An arrow key press with no modifier but Shift and Alt: its unit direction, whether Shift is
/// held and whether Alt is.
fn arrow_with_alt(event: &Event) -> Option<(Vec2, bool, bool)> {
    let Event::Key { key, pressed: true, modifiers, .. } = event else { return None };
    let dir = match key {
        Key::ArrowLeft => Vec2::new(-1.0, 0.0),
        Key::ArrowRight => Vec2::new(1.0, 0.0),
        Key::ArrowUp => Vec2::new(0.0, -1.0),
        Key::ArrowDown => Vec2::new(0.0, 1.0),
        _ => return None,
    };
    [Modifiers::NONE, Modifiers::SHIFT, Modifiers::ALT, Modifiers::ALT | Modifiers::SHIFT]
        .into_iter()
        .find(|&m| modifiers.matches_exact(m))
        .map(|m| (dir, m.shift, m.alt))
}
