//! Input methods for text typed onto canvases.
//!
//! egui's `TextEdit` handles its own composition. A canvas that takes text (a type tool, a cell
//! being typed into, a caption on a timeline) uses one [`Composer`] instead:
//!
//! ```no_run
//! # fn insert(_: &str) {} fn press(_: egui::Key, _: egui::Modifiers) {}
//! # fn caret_rect() -> egui::Rect { egui::Rect::ZERO }
//! # let ctx = egui::Context::default(); let mut composer = craft_ui::ime::Composer::new();
//! # let editing = true;
//! use craft_ui::ime::Edit;
//! // Every pass, before shortcuts and other keyboard handling:
//! for edit in composer.update(&ctx, editing) {
//!     match edit {
//!         Edit::Insert(text) => insert(&text),
//!         Edit::Key { key, modifiers, .. } => press(key, modifiers),
//!         _ => {}
//!     }
//! }
//! // After laying out the text, in screen points:
//! composer.set_caret(&ctx, caret_rect());
//! // Paint `composer.preedit()` at the caret, underlined.
//! ```
//!
//! The composer covers what the apps each handled on their own:
//!
//! - Events stay in the order the platform sent them, so a Telex Backspace that replaces a
//!   letter lands between the right two inserts.
//! - Keys, text and clipboard events belong to the IME while it composes. Enter, Escape,
//!   Backspace, Delete and Space that end a composition are its too, once: Enter that confirms a
//!   word doesn't also start a line or submit, Backspace that empties the marked text doesn't
//!   delete the character before it, and Escape that cancels doesn't leave the editor.
//! - A bare line break committed by the IME confirms the marked text instead of breaking the line.
//! - Empty preedits outside a composition (Wayland sends them on every state change) are ignored.
//! - A click while composing keeps the marked text as typed and tells the IME to drop its copy;
//!   a late commit of the same text is dropped. Giving up the keyboard does the same.
//! - Text that repeats the commit just made (Apple Korean's Space) is typed once.
//! - Cmd/Ctrl chords don't type their letter if the host sends it; Alt and AltGr still type.
//! - The candidate window is placed at the caret, clamped into the window.
//!
//! Platform bugs below egui (lost commits on macOS, the Korean first key, X11 focus) belong in
//! winit and are fixed there; the composer doesn't work around them.

mod composer;

pub use composer::{Composer, Edit, Preedit, Processed};

#[cfg(test)]
mod tests;
