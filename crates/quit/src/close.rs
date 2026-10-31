//! Route an egui window close through the platform quit, and [`CloseGuard`]: the unsaved-changes
//! decision for closing documents, closing all, quitting and Revert.

#[cfg(feature = "ui")]
use egui::{Context, ViewportCommand, ViewportId};

/// Call once per frame at the end of `eframe::App::logic`, after the app's own close decision.
/// Move that decision out of `ui` as well: `logic` runs first, and `ui` does not run while the
/// window is minimized or hidden. See the crate example for keeping a save prompt in app state.
/// When the root window was asked
/// to close (close button, the app's own `ViewportCommand::Close`, the OS) and this platform has
/// its own quit (see [`crate::quit`]), cancels the close and quits that way instead. The window
/// stays for the quit to close.
///
/// Returns whether it took the close over. `false` means there was no close request, the app had
/// already cancelled it this frame, or the platform has no quit of its own, and eframe goes on as
/// it would without this call.
#[cfg(feature = "ui")]
pub fn on_close_request(ctx: &Context) -> bool {
    take_over_close(ctx, crate::quit)
}

/// The id of a Quit menu item that asks the app first.
///
/// An app that builds its own native app menu (muda, any version) gives its Quit item this id,
/// instead of using muda's `PredefinedMenuItem::quit`, which ends the process without asking, and
/// passes its menu events to [`on_menu_event`]. Apps without their own menu use
/// [`crate::install_app_menu`], which does both.
pub const QUIT_MENU_ID: &str = "craft-quit.quit";

/// Handle a native menu event by the id of the item chosen. For [`QUIT_MENU_ID`], asks the root
/// window to close (`ViewportCommand::Close`), so the app's unsaved-changes check runs as it does
/// for the close button and [`on_close_request`] then quits. Returns whether the item was Quit;
/// other items are left to the caller. Safe to call from a menu-event handler between frames:
/// egui keeps the command for the next frame.
#[cfg(feature = "ui")]
pub fn on_menu_event(ctx: &Context, id: &str) -> bool {
    if id != QUIT_MENU_ID {
        return false;
    }
    ctx.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Close);
    true
}

/// [`on_close_request`] with the platform quit passed in. `quit` returns whether it quits.
#[cfg(feature = "ui")]
fn take_over_close(ctx: &Context, quit: impl FnOnce() -> bool) -> bool {
    if ctx.viewport_id() != ViewportId::ROOT || !ctx.input(|i| i.viewport().close_requested()) {
        return false;
    }
    // The app's unsaved-changes prompt (or anything else) is holding the window open.
    if ctx.viewport(|v| v.commands.contains(&ViewportCommand::CancelClose)) {
        return false;
    }
    if !quit() {
        return false;
    }
    ctx.send_viewport_cmd(ViewportCommand::CancelClose);
    true
}

/// Holds a window close while the app asks about unsaved documents. Call in `logic` when the
/// root window was asked to close and `unsaved` documents exist: it sends `CancelClose` and
/// returns `true`, and the app starts a [`CloseGuard`] for [`CloseScope::Quit`]. Without unsaved
/// documents it returns `false` and [`on_close_request`] takes the close as usual.
// pc: crates/ui-egui/src/dialogs/discard_ui.rs:124 `guard_window_close`.
#[cfg(feature = "ui")]
pub fn intercept_close(ctx: &Context, unsaved: bool) -> bool {
    if !unsaved || ctx.viewport_id() != ViewportId::ROOT || !ctx.input(|i| i.viewport().close_requested()) {
        return false;
    }
    ctx.send_viewport_cmd(ViewportCommand::CancelClose);
    ctx.send_viewport_cmd(ViewportCommand::Focus);
    true
}

/// What the user set out to do; it decides the question's wording and what Cancel stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CloseScope {
    /// Close one or more documents (a tab's close box, File › Close, Close All).
    Close,
    /// File › Exit / Quit, the window's close box, the Quit menu item.
    Quit,
    /// File › Revert on one document: "Revert … to the last saved version?".
    Revert,
}

/// The user's answer for one document.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CloseAnswer {
    Save,
    DontSave,
    Cancel,
}

/// What the app does next after an answer or a save.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardStep<D> {
    /// Save `D` (Save As when it has no file); then report [`CloseGuard::saved`] or
    /// [`CloseGuard::save_failed`].
    Save(D),
    /// Close `D` without saving.
    Discard(D),
    /// Ask about the next document: show the prompt for [`CloseGuard::current`].
    Ask(D),
    /// Cancel stopped the whole operation; nothing more closes and a quit does not happen.
    Cancelled,
    /// Every document is answered and closed: finish the operation (close the window, quit).
    Finished,
}

/// The unsaved-changes decision, one document at a time, egui-free. The app keeps one while
/// the operation runs: it asks the current document's question (craft-ui's
/// `UnsavedChangesDialog`), feeds the answer in, saves or discards as told, and finishes or stops
/// on the step it gets back. `D` is the app's document id.
///
/// - Documents are asked in the order given. [`CloseAnswer::Cancel`] at any point ends the
///   operation ([`GuardStep::Cancelled`]); documents already closed stay closed.
/// - "Apply to All" (spec question 3): [`answer_all`](CloseGuard::answer_all) applies one answer
///   to every remaining document, Save included: each is saved in turn, and a cancelled Save As
///   or a failed save stops the operation.
/// - A failed save keeps the document open and stops the operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloseGuard<D> {
    scope: CloseScope,
    pending: std::collections::VecDeque<D>,
    saving: Option<D>,
    apply_to_all: Option<CloseAnswer>,
    cancelled: bool,
}

impl<D: Clone + PartialEq> CloseGuard<D> {
    /// Starts the decision for `modified`, the documents with unsaved changes in asking order.
    /// An empty list is already [`GuardStep::Finished`].
    pub fn new(scope: CloseScope, modified: impl IntoIterator<Item = D>) -> Self {
        Self { scope, pending: modified.into_iter().collect(), saving: None, apply_to_all: None, cancelled: false }
    }

    pub fn scope(&self) -> CloseScope {
        self.scope
    }

    /// The document whose question is open, if any.
    pub fn current(&self) -> Option<&D> {
        self.pending.front()
    }

    /// How many documents still wait, the current one included; the prompt shows "Apply to All"
    /// when it is more than one.
    pub fn remaining(&self) -> usize {
        self.pending.len()
    }

    /// The user answered for the current document.
    // pc: crates/ui-egui/src/dialogs/discard_ui.rs:343 `apply` and :149 `advance`.
    pub fn answer(&mut self, answer: CloseAnswer) -> GuardStep<D> {
        if self.cancelled {
            return GuardStep::Cancelled;
        }
        // A save is still running: the question is not open.
        if let Some(doc) = &self.saving {
            return GuardStep::Save(doc.clone());
        }
        let Some(doc) = self.pending.front().cloned() else { return GuardStep::Finished };
        match answer {
            CloseAnswer::Save => {
                self.saving = Some(doc.clone());
                GuardStep::Save(doc)
            }
            CloseAnswer::DontSave => GuardStep::Discard(doc),
            CloseAnswer::Cancel => self.cancel(),
        }
    }

    /// The user answered with "Apply to All": the same answer for every remaining document.
    // new: Photoshop's Apply to All; PhotoCraft asks one by one.
    pub fn answer_all(&mut self, answer: CloseAnswer) -> GuardStep<D> {
        self.apply_to_all = Some(answer);
        self.answer(answer)
    }

    /// The document from [`GuardStep::Discard`] is closed without saving: the guard moves on.
    // new: the step after a discard; `saved` is the same for a save.
    pub fn discarded(&mut self) -> GuardStep<D> {
        if self.saving.is_none() {
            self.pending.pop_front();
        }
        self.next_step()
    }

    /// The save of the document from [`GuardStep::Save`] finished: it is closed and the guard
    /// moves on.
    // pc: crates/ui-egui/src/dialogs/discard_ui.rs:244 `saved_document`.
    pub fn saved(&mut self) -> GuardStep<D> {
        if let Some(saved) = self.saving.take()
            && self.pending.front() == Some(&saved)
        {
            self.pending.pop_front();
        }
        self.next_step()
    }

    /// The save failed or Save As was cancelled: the document stays open and the operation stops.
    // pc: crates/ui-egui/src/dialogs/discard_ui.rs:210 `save` (the `false` branch).
    pub fn save_failed(&mut self) -> GuardStep<D> {
        self.cancel()
    }

    /// A document closed elsewhere while the guard ran (a script): it is no longer asked about.
    pub fn forget(&mut self, doc: &D) {
        self.pending.retain(|d| d != doc);
    }

    pub fn is_finished(&self) -> bool {
        self.cancelled || (self.pending.is_empty() && self.saving.is_none())
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Ends the operation; the documents not yet answered stay open.
    fn cancel(&mut self) -> GuardStep<D> {
        self.cancelled = true;
        self.pending.clear();
        self.saving = None;
        GuardStep::Cancelled
    }

    /// The step for the front document once the previous one is settled.
    fn next_step(&mut self) -> GuardStep<D> {
        if self.cancelled {
            return GuardStep::Cancelled;
        }
        let Some(doc) = self.pending.front().cloned() else { return GuardStep::Finished };
        match self.apply_to_all {
            None => GuardStep::Ask(doc),
            Some(CloseAnswer::Save) => {
                self.saving = Some(doc.clone());
                GuardStep::Save(doc)
            }
            Some(CloseAnswer::DontSave) => GuardStep::Discard(doc),
            Some(CloseAnswer::Cancel) => self.cancel(),
        }
    }
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    #[test]
    fn each_answer_moves_to_the_next_document_and_cancel_stops_the_rest() {
        let mut guard = CloseGuard::new(CloseScope::Close, [1, 2, 3, 4]);
        assert_eq!(guard.answer(CloseAnswer::Save), GuardStep::Save(1));
        assert_eq!(guard.saved(), GuardStep::Ask(2));
        assert_eq!(guard.answer(CloseAnswer::DontSave), GuardStep::Discard(2));
        assert_eq!(guard.discarded(), GuardStep::Ask(3));
        assert_eq!(guard.answer(CloseAnswer::Cancel), GuardStep::Cancelled);
        assert!(guard.is_cancelled() && guard.is_finished());
        assert_eq!(guard.current(), None);

        let mut guard = CloseGuard::new(CloseScope::Quit, [7]);
        assert_eq!(guard.answer(CloseAnswer::DontSave), GuardStep::Discard(7));
        assert_eq!(guard.discarded(), GuardStep::Finished);
        assert!(guard.is_finished() && !guard.is_cancelled());
        assert_eq!(CloseGuard::<u32>::new(CloseScope::Close, []).saved(), GuardStep::Finished);
    }

    #[test]
    fn apply_to_all_saves_every_document_and_a_failed_save_stops() {
        let mut guard = CloseGuard::new(CloseScope::Quit, [1, 2, 3]);
        assert_eq!(guard.answer_all(CloseAnswer::Save), GuardStep::Save(1));
        assert_eq!(guard.saved(), GuardStep::Save(2));
        assert_eq!(guard.save_failed(), GuardStep::Cancelled);
        assert!(guard.is_cancelled());

        let mut guard = CloseGuard::new(CloseScope::Quit, [1, 2]);
        assert_eq!(guard.answer_all(CloseAnswer::Save), GuardStep::Save(1));
        assert_eq!(guard.saved(), GuardStep::Save(2));
        assert_eq!(guard.saved(), GuardStep::Finished);

        let mut guard = CloseGuard::new(CloseScope::Quit, [1, 2]);
        assert_eq!(guard.answer_all(CloseAnswer::DontSave), GuardStep::Discard(1));
        assert_eq!(guard.discarded(), GuardStep::Discard(2));
        assert_eq!(guard.discarded(), GuardStep::Finished);
    }
}

#[cfg(all(test, feature = "ui"))]
mod tests {
    use std::cell::Cell;

    use egui::{RawInput, ViewportEvent};

    use super::*;

    /// One frame of the root window, with its close button pressed or not. `app` runs before the
    /// take-over, like the app's own unsaved-changes check. Returns whether the close was taken
    /// over, how often the quit ran and the frame's commands for the root window.
    fn frame(close: bool, quits: bool, app: impl Fn(&Context)) -> (bool, u32, Vec<ViewportCommand>) {
        let ctx = Context::default();
        let mut input = RawInput::default();
        if close {
            input.viewports.entry(ViewportId::ROOT).or_default().events.push(ViewportEvent::Close);
        }
        let (taken, quit_calls) = (Cell::new(false), Cell::new(0));
        let mut out = ctx.run_ui(input, |ui| {
            app(ui.ctx());
            taken.set(take_over_close(ui.ctx(), || {
                quit_calls.set(quit_calls.get() + 1);
                quits
            }));
        });
        out.textures_delta.clear(); // not painted
        let commands = out.viewport_output.remove(&ViewportId::ROOT).map(|o| o.commands).unwrap_or_default();
        (taken.get(), quit_calls.get(), commands)
    }

    fn cancels(commands: &[ViewportCommand]) -> bool {
        commands.contains(&ViewportCommand::CancelClose)
    }

    #[test]
    fn intercept_close_holds_the_window_while_documents_are_unsaved() {
        for (unsaved, held) in [(true, true), (false, false)] {
            let result = Cell::new(false);
            let (taken, quit_calls, commands) = frame(true, true, |ctx| result.set(intercept_close(ctx, unsaved)));
            assert_eq!(result.get(), held);
            assert!(cancels(&commands), "either the hold or the quit takes the close");
            assert_eq!(taken, !held);
            assert_eq!(quit_calls, u32::from(!held));
        }
        let held = Cell::new(true);
        frame(false, true, |ctx| held.set(intercept_close(ctx, true)));
        assert!(!held.get(), "no close request, nothing to hold");
    }

    #[test]
    fn a_close_request_quits_and_keeps_the_window() {
        let (taken, quit_calls, commands) = frame(true, true, |_| {});
        assert!(taken);
        assert_eq!(quit_calls, 1);
        assert!(cancels(&commands), "the window stays for the quit to close");
    }

    #[test]
    fn nothing_happens_without_a_close_request() {
        let (taken, quit_calls, commands) = frame(false, true, |_| {});
        assert!(!taken);
        assert_eq!(quit_calls, 0);
        assert!(!cancels(&commands));
    }

    #[test]
    fn a_close_the_app_cancelled_does_not_quit() {
        let (taken, quit_calls, _) = frame(true, true, |ctx| ctx.send_viewport_cmd(ViewportCommand::CancelClose));
        assert!(!taken);
        assert_eq!(quit_calls, 0, "unsaved work asks first");
    }

    /// The root window's commands from one frame run after `before`, which runs between frames
    /// like a native menu-event handler.
    fn commands_after(before: impl FnOnce(&Context)) -> Vec<ViewportCommand> {
        let ctx = Context::default();
        before(&ctx);
        let mut out = ctx.run_ui(RawInput::default(), |_| {});
        out.textures_delta.clear(); // not painted
        out.viewport_output.remove(&ViewportId::ROOT).map(|o| o.commands).unwrap_or_default()
    }

    #[test]
    fn the_quit_menu_item_asks_the_window_to_close() {
        let commands = commands_after(|ctx| assert!(on_menu_event(ctx, QUIT_MENU_ID)));
        assert!(commands.contains(&ViewportCommand::Close), "the app's close handling decides");
    }

    #[test]
    fn quit_from_a_child_viewport_still_asks_the_root_to_close() {
        let ctx = Context::default();
        let child = ViewportId::from_hash_of("document-child");
        let mut input = RawInput { viewport_id: child, ..RawInput::default() };
        input.viewports.entry(child).or_default().parent = Some(ViewportId::ROOT);
        let mut out = ctx.run_ui(input, |ui| {
            assert_eq!(ui.ctx().viewport_id(), child);
            assert!(on_menu_event(ui.ctx(), QUIT_MENU_ID));
        });
        out.textures_delta.clear(); // not painted
        let root_commands = out.viewport_output.get(&ViewportId::ROOT).map(|o| o.commands.as_slice()).unwrap_or_default();
        let child_commands = out.viewport_output.get(&child).map(|o| o.commands.as_slice()).unwrap_or_default();
        assert!(root_commands.contains(&ViewportCommand::Close), "root unsaved handling decides whether the app exits");
        assert!(!child_commands.contains(&ViewportCommand::Close), "Quit must not just close the current child");
    }

    #[test]
    fn other_menu_items_are_left_to_the_app() {
        let commands = commands_after(|ctx| assert!(!on_menu_event(ctx, "edit.copy")));
        assert!(!commands.contains(&ViewportCommand::Close));
    }

    #[test]
    fn without_a_platform_quit_the_window_closes_as_usual() {
        let (taken, quit_calls, commands) = frame(true, false, |_| {});
        assert!(!taken);
        assert_eq!(quit_calls, 1);
        assert!(!cancels(&commands), "eframe closes the window");
    }

    /// eframe uses only the logic callback while minimized; no UI callback runs here.
    fn minimized_logic_frame(unsaved: bool) -> (bool, u32, bool, Vec<ViewportCommand>) {
        let ctx = Context::default();
        let mut input = RawInput::default();
        let root = input.viewports.entry(ViewportId::ROOT).or_default();
        root.minimized = Some(true);
        root.events.push(ViewportEvent::Close);
        let (taken, quit_calls, show_save_prompt) = (Cell::new(false), Cell::new(0), Cell::new(false));
        let mut out = ctx.run_logic(&input, |ctx| {
            if ctx.input(|i| i.viewport().close_requested()) && unsaved {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                show_save_prompt.set(true);
            }
            taken.set(take_over_close(ctx, || {
                quit_calls.set(quit_calls.get() + 1);
                true
            }));
        });
        let commands = out.viewport_commands.remove(&ViewportId::ROOT).unwrap_or_default();
        (taken.get(), quit_calls.get(), show_save_prompt.get(), commands)
    }

    #[test]
    fn minimized_unsaved_close_waits_for_the_prompt_without_running_ui() {
        let (taken, quit_calls, show_save_prompt, commands) = minimized_logic_frame(true);
        assert!(!taken);
        assert_eq!(quit_calls, 0);
        assert!(show_save_prompt, "UI can render the decision stored by logic");
        assert!(cancels(&commands));
    }

    #[test]
    fn minimized_clean_close_takes_over_without_running_ui() {
        let (taken, quit_calls, show_save_prompt, commands) = minimized_logic_frame(false);
        assert!(taken);
        assert_eq!(quit_calls, 1);
        assert!(!show_save_prompt);
        assert!(cancels(&commands), "eframe must not tear down the window before platform quit");
    }
}
