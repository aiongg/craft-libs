//! Key focus: egui decides who gets keys, and egui key focus on the panel's widget is the rt
//! window's key focus.
//!
//! - A pointer click in the panel does not take egui focus (egui gives none on click), so the
//!   app's tool keys keep working, as with the egui rows.
//! - Tab into the panel takes it (the panel's widget is focusable) and enters the rt window at its
//!   first stop (last with Shift); Tab past its last stop leaves for egui's next widget.
//! - A text input taking rt focus (the rename field) asks for it through
//!   [`craft_ui_rt::WindowCommand::Focus`], and gives it back to egui when its IME session ends.
//! - While the panel has focus, egui's own Tab, arrow and Escape focus moves are off; an Escape
//!   the rt window does not use gives the focus up, as egui's widgets do.

use craft_ui_rt::{FocusExit, Key, KeyInput, Mods, PlatformEvent, Runtime, WindowEvent, WindowId};
use egui::{EventFilter, FocusDirection};

use crate::host::EguiHost;

/// Keys egui does not use to move focus away from the panel.
const LOCK: EventFilter = EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true };

pub(crate) struct FocusMirror {
    /// What the rt window was last told; it starts with key focus.
    told: bool,
    /// Focus was asked for on the window's behalf (a text input) and not yet handed back.
    for_text: bool,
}

impl Default for FocusMirror {
    fn default() -> FocusMirror {
        FocusMirror { told: true, for_text: false }
    }
}

impl FocusMirror {
    /// Before the step: tells the rt window when egui focus came or went, and returns whether the
    /// panel has it.
    pub fn before(&mut self, ctx: &egui::Context, id: egui::Id, window: WindowId, runtime: &mut Runtime, queue: &mut Vec<PlatformEvent>) -> bool {
        let focused = ctx.memory(|m| m.has_focus(id));
        if focused != self.told {
            self.told = focused;
            queue.push(PlatformEvent::Window(window, WindowEvent::Focus(focused)));
            if focused && !self.for_text && runtime.focused(window).is_none() {
                // egui's keyboard navigation brought focus in.
                let back = ctx.input(|i| i.modifiers.shift);
                runtime.move_focus(window, !back);
            }
            if !focused {
                self.for_text = false;
            }
        }
        if focused {
            ctx.memory_mut(|m| m.set_focus_lock_filter(id, LOCK));
        }
        focused
    }

    /// After the step: Tab moves out of the window, focus asked for by a text input, and focus
    /// given up by Escape or by the text input's end.
    pub fn after(&mut self, ctx: &egui::Context, id: egui::Id, window: WindowId, runtime: &mut Runtime, host: &mut EguiHost, unhandled: &[KeyInput]) {
        for exit in runtime.take_focus_exits(window) {
            let direction = match exit {
                FocusExit::Next => FocusDirection::Next,
                FocusExit::Previous => FocusDirection::Previous,
            };
            ctx.memory_mut(|m| m.move_focus(direction));
        }
        if std::mem::take(&mut host.focus_request) && !self.told {
            self.for_text = true;
            ctx.memory_mut(|m| m.request_focus(id));
            // The rt window learns it has focus in the next pass.
            ctx.request_repaint();
        }
        let escape = unhandled.iter().any(|k| k.down && k.key == Key::Escape && k.mods == Mods::NONE);
        let text_ended = self.for_text && self.told && host.ime.is_none();
        if self.told && (escape || text_ended) {
            self.for_text = false;
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
    }
}
