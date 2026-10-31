//! What PhotoCraft's own key handling does that the keymap must match before PhotoCraft drops it
//! (gaps 43, 44, 85): the Return glyph, modifier-only hints, Delete and Backspace as one key,
//! layout position fallback, tools sharing a key, unparseable override texts, and the clipboard
//! pairing as a reader of its own.
//!
//! "Main shortcut removed, alternates stay" (gap 84) needs nothing new: [`Keymap::remove`] on the
//! first binding writes an override with the others.

#![expect(unused_variables, unused_mut, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::chord::{Chord, Mods, Platform};
use crate::command::CommandRef;
use crate::keymap::Keymap;
use crate::layout::Layout;

/// The glyph a Mac shows for the Return key. Apple's menus show ↩ (`Key::Enter` today shows ⏎);
/// which one the suite uses waits for the owner's Mac check (gap 43).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ReturnGlyph {
    /// ↩, as AppKit draws a menu item's Return key equivalent.
    #[default]
    Return,
    /// ⏎.
    Enter,
}

impl ReturnGlyph {
    pub fn symbol(self) -> &'static str {
        todo!()
    }
}

impl Chord {
    /// [`Chord::display`] with `glyph` for Return on macOS; the same as `display` elsewhere.
    // new: gap 43; crate::chord.rs:265 `mac_symbol` maps `Key::Enter` to ⏎.
    pub fn display_with_return(&self, platform: Platform, glyph: ReturnGlyph) -> String {
        todo!()
    }
}

impl Mods {
    /// The held modifiers alone, for a hint while they are down ("Alt: subtract from selection"):
    /// `⇧⌥` on macOS (in Apple's order ⌃⌥⇧⌘), `Shift+Alt` elsewhere (Ctrl, Shift, Alt order as
    /// in [`Chord::display`]). Empty when none is held.
    // pc: crates/ui-egui/src/input/shortcuts.rs:50 `pretty` (`pretty("Shift+Alt")` is "⇧⌥" on
    // macOS, "Shift+Alt" elsewhere; input/shortcuts/tests.rs:225), used by the options bar's
    // selection-mode tooltips (panels/options_bar.rs:272).
    pub fn display(self, platform: Platform) -> String {
        todo!()
    }
}

impl Keymap {
    /// Whether tools of different groups may share a chord ([`Keymap::conflicts`] reports no
    /// conflict between two tools; a press cycles through them in catalog order). Off by default;
    /// PhotoCraft turns it on, as Photoshop allows it.
    // new: gap 85; PhotoCraft's `bindings()` table lets tools share keys.
    pub fn with_shared_tool_keys(mut self, on: bool) -> Keymap {
        todo!()
    }

    pub fn shares_tool_keys(&self) -> bool {
        todo!()
    }

    /// The override texts for `command` that don't parse, as the user wrote them, so the dialog
    /// shows them (struck out, with the parse error as tooltip) instead of the default it falls
    /// back to. Empty when every text parses.
    // new: gap 85; PhotoCraft showed the raw text.
    pub fn unparsed_texts(&self, command: &CommandRef) -> Vec<&str> {
        todo!()
    }
}

impl Layout {
    /// Whether a binding on a key the layout doesn't type unshifted (`[` and `]` on a German
    /// layout, where those positions type `ü` and `+`) also matches the key at its US position.
    /// On by default for `[`, `]`, `;`, `'`, `\`, `,`, `.`, `/`, `` ` ``, `-` and `=`.
    // pc: crates/ui-egui/src/input/layout_keys.rs:1-12 (module doc: a per-layout table, then the
    // fallback that the keys at the US `[` / `]` positions act as `[` / `]` when they type
    // something else). PhotoCraft applies it to the brush keys `[`, `]` and ⇧+ / ⇧−; the
    // library's wider key list is new.
    pub fn with_position_fallback(mut self, on: bool) -> Layout {
        todo!()
    }
}

#[cfg(feature = "ui")]
impl crate::egui::Dispatcher {
    /// Whether Delete and Backspace match each other's bindings: a binding on Delete also runs
    /// from Backspace (the Mac's "delete" key), and Backspace from Delete (⌥⌫ and ⌥Delete both
    /// fill on Windows). On by default.
    // pc: crates/ui-egui/src/input/shortcuts.rs:141-146 `key_matches`.
    pub fn with_delete_equivalence(mut self, on: bool) -> crate::egui::Dispatcher {
        todo!()
    }
}

/// A clipboard chord read from a frame's events: the press and the Copy, Cut or Paste event it
/// caused, paired.
#[cfg(feature = "ui")]
#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardPress {
    /// The chord pressed (Cmd+V, Shift+Insert).
    pub chord: Chord,
    /// The event index of the press, when the integration sent one (the web build does; the
    /// desktop sends only the clipboard event).
    pub press: Option<usize>,
    /// The event index of the Copy, Cut or Paste event.
    pub event: Option<usize>,
    /// A Paste event's text.
    pub pasted: Option<String>,
}

/// The frame's clipboard chords, paired with their events as [`crate::egui::Dispatcher`] pairs
/// them, for an app that handles copy and paste outside the dispatcher (label selection, a
/// swallowed ⌘V that pasted into nothing, Linux's Shift+Insert). Reads only; removes nothing.
// pc: crates/ui-egui/src/input/shortcuts.rs:286 `clipboard_keys` (doc :267-285: a clipboard
// event right after its own ⌘ press is that press, #2638; a V release whose press never arrived,
// after ⌘ was held and before any other key, is a swallowed ⌘V, #6329; Ctrl+Insert copies and
// Shift+Insert pastes, mapped by hand on Linux). The pairing is egui.rs `clipboard_event_follows`,
// shared.
#[cfg(feature = "ui")]
pub fn clipboard_presses(input: &egui::InputState, layout: &Layout, platform: Platform) -> Vec<ClipboardPress> {
    todo!()
}
