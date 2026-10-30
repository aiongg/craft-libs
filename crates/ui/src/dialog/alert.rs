//! [`Alert`]: a notice with one button.

use egui::accesskit::Role;
use egui::{Context, Event, Id, Key};

use super::dialog::{Dialog, InitialFocus};
use super::footer::Footer;
use super::prompt::{load_state, store_state};
use super::size::Width;
use super::stack::{DialogStack, dialog_id};

/// A modal notice with one OK button. Its AccessKit node is an alert dialog described by the
/// message.
///
/// ```
/// # use craft_ui::dialog::Alert;
/// # egui::__run_test_ctx(|ctx| {
/// if Alert::new("about", "About", "Version 1.0").show(ctx) { /* closed */ }
/// # });
/// ```
///
/// - Show it every pass until it answers. OK, Esc, the close box and Enter close it.
/// - Enter closes it only for a press made after the alert's first pass, and never for a key
///   repeat: the Enter that raised it, still repeating, must not close it at once.
/// - It is centred, 420 points wide, and the message wraps; OK takes the focus on open.
#[must_use = "show the alert with `show`"]
pub struct Alert<'a> {
    id: Id,
    title: String,
    message: String,
    ok_label: String,
    stack: Option<&'a DialogStack>,
}

impl<'a> Alert<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, title: impl Into<String>, message: impl Into<String>) -> Self {
        Self { id: dialog_id(id), title: title.into(), message: message.into(), ok_label: "OK".to_owned(), stack: None }
    }

    /// The translated label of the button; "OK" by default.
    pub fn ok_label(mut self, label: impl Into<String>) -> Self {
        self.ok_label = label.into();
        self
    }

    /// Makes the alert part of `stack`, as [`Dialog::stack`] does.
    pub fn stack(mut self, stack: &'a DialogStack) -> Self {
        self.stack = Some(stack);
        self
    }

    /// The id a [`DialogStack`] knows the alert by.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Shows the alert; true the pass it is closed.
    pub fn show(self, ctx: &Context) -> bool {
        let Self { id, title, message, ok_label, stack } = self;
        let key = id.with("alert");
        let (mut state, first) = load_state(ctx, key);
        let opening = first == ctx.cumulative_frame_nr();
        // The focused button would answer a repeating Enter by itself, and the Enter that raised
        // the alert may still be in this pass's input.
        ctx.input_mut(|i| {
            i.events.retain(|e| !matches!(e, Event::Key { key: Key::Enter, pressed: true, repeat, .. } if *repeat || opening));
        });
        let mut dialog = Dialog::with_id(id, title)
            .width(Width::Fixed(420.0))
            .footer(Footer::standard().ok(ok_label).without_cancel())
            .initial_focus(InitialFocus::DefaultButton)
            .role(Role::AlertDialog)
            .description(message.clone());
        if let Some(stack) = stack {
            dialog = dialog.stack(stack);
        }
        let shown = dialog.show(ctx, &mut state, &mut (), |ui, ()| {
            ui.label(message);
        });
        store_state(ctx, key, state, first);
        shown.outcome.is_some_and(|outcome| outcome.closes())
    }
}
