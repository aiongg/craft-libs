//! [`Prompt`]: a question with answers and accelerator letters, and [`accelerator_labels`].

use egui::{Context, Id, Key, Modifiers};

use super::buttons::{ButtonRole, FooterButton, Platform};
use super::dialog::{Dialog, InitialFocus, Outcome};
use super::footer::Footer;
use super::size::Width;
use super::stack::{DialogStack, dialog_id};
use super::state::DialogState;

/// The labels of one prompt's buttons with their keys in the same style: `(Y)es` when every label
/// starts with its key (case-insensitive), otherwise the key is appended to every label
/// (`Ja (Y)`, `Nein (N)`), so one prompt never mixes the two. An answer without a key keeps its
/// text. macOS shows plain labels: the keys still work, the letters are hidden.
pub fn accelerator_labels(answers: &[(String, Option<Key>)], platform: Platform) -> Vec<String> {
    let starts_with_key = |label: &str, key: Key| label.chars().next().is_some_and(|c| c.to_string().eq_ignore_ascii_case(key.name()));
    let inline = answers.iter().all(|(label, key)| key.is_none_or(|k| starts_with_key(label, k)));
    answers
        .iter()
        .map(|(label, key)| match key {
            Some(key) if platform == Platform::Other => match label.chars().next() {
                Some(first) if inline => format!("({first}){}", &label[first.len_utf8()..]),
                _ => format!("{label} ({})", key.name()),
            },
            _ => label.clone(),
        })
        .collect()
}

/// The role Esc and the close box answer with: the host's choice, else Cancel, else the first
/// Alternate, else the Default.
fn escape_role(roles: &[ButtonRole], chosen: Option<ButtonRole>) -> ButtonRole {
    chosen
        .filter(|role| roles.contains(role))
        .or_else(|| [ButtonRole::Cancel, ButtonRole::Alternate, ButtonRole::Default].into_iter().find(|role| roles.contains(role)))
        .unwrap_or(ButtonRole::Cancel)
}

/// The state of a prompt or alert the host does not hold: kept in the context under `id`, and
/// started afresh when the host skipped a frame without showing it.
pub(super) fn load_state(ctx: &Context, id: Id) -> (DialogState, u64) {
    let now = ctx.cumulative_frame_nr();
    match ctx.data(|d| d.get_temp::<(DialogState, u64, u64)>(id)) {
        Some((state, first, last)) if last + 1 >= now => (state, first),
        _ => (DialogState::new(), now),
    }
}

pub(super) fn store_state(ctx: &Context, id: Id, state: DialogState, first: u64) {
    let now = ctx.cumulative_frame_nr();
    ctx.data_mut(|d| {
        if state.is_open() {
            d.insert_temp(id, (state, first, now));
        } else {
            d.remove::<(DialogState, u64, u64)>(id);
        }
    });
}

struct Answer {
    role: ButtonRole,
    label: String,
    key: Option<Key>,
}

/// A modal question. The host passes translated labels and keys; the wording of the platform
/// (Yes / No / Cancel on Windows, Save / Don't Save / Cancel on macOS) is its choice.
///
/// ```
/// # use craft_ui::dialog::{ButtonRole, Prompt};
/// # egui::__run_test_ctx(|ctx| {
/// let answer = Prompt::new("unsaved", "Unsaved changes", "Do you want to save the changes?")
///     .answer(ButtonRole::Default, "Yes", Some(egui::Key::Y))
///     .answer(ButtonRole::Alternate, "No", Some(egui::Key::N))
///     .answer(ButtonRole::Cancel, "Cancel", Some(egui::Key::C))
///     .show(ctx);
/// # let _ = answer;
/// # });
/// ```
///
/// - Show it every pass until it answers. It keeps its own state, so there is nothing to hold.
/// - The Default answer takes the focus on open and answers Enter unless another button has the
///   focus. Esc and the close box answer the [`escape`](Prompt::escape) role: Cancel when there
///   is one, else the first Alternate.
/// - The answers' keys are plain keys with no modifier, read only while the prompt is the top
///   modal and no popup is open. They are drawn in the labels on Windows and Linux
///   ([`accelerator_labels`]) and are the buttons' AccessKit keyboard shortcuts.
/// - It is centred, 420 points wide, and the message wraps. A role appears once; two Alternates
///   are not told apart in the answer.
#[must_use = "show the prompt with `show`"]
pub struct Prompt<'a> {
    id: Id,
    title: String,
    message: String,
    answers: Vec<Answer>,
    escape: Option<ButtonRole>,
    stack: Option<&'a DialogStack>,
}

impl<'a> Prompt<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, title: impl Into<String>, message: impl Into<String>) -> Self {
        Self { id: dialog_id(id), title: title.into(), message: message.into(), answers: Vec::new(), escape: None, stack: None }
    }

    /// Adds an answer: its role (which decides its place and look), translated label and key.
    pub fn answer(mut self, role: ButtonRole, label: impl Into<String>, key: Option<Key>) -> Self {
        debug_assert!(self.answers.iter().all(|a| a.role != role), "craft-ui: a prompt answers each role once");
        self.answers.push(Answer { role, label: label.into(), key });
        self
    }

    /// The answer Esc and the close box give; Cancel when the prompt has one.
    pub fn escape(mut self, role: ButtonRole) -> Self {
        self.escape = Some(role);
        self
    }

    /// Makes the prompt part of `stack`, as [`Dialog::stack`] does.
    pub fn stack(mut self, stack: &'a DialogStack) -> Self {
        self.stack = Some(stack);
        self
    }

    /// The id a [`DialogStack`] knows the prompt by.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Shows the prompt; the role of the answer the pass it is given.
    pub fn show(self, ctx: &Context) -> Option<ButtonRole> {
        let Self { id, title, message, answers, escape, stack } = self;
        let roles: Vec<ButtonRole> = answers.iter().map(|a| a.role).collect();
        let escape = escape_role(&roles, escape);
        let labels = accelerator_labels(&answers.iter().map(|a| (a.label.clone(), a.key)).collect::<Vec<_>>(), Platform::of(ctx));
        let keys: Vec<(ButtonRole, Option<Key>)> = answers.iter().map(|a| (a.role, a.key)).collect();

        let (mut state, first) = load_state(ctx, id.with("prompt"));
        let buttons: Vec<FooterButton> = answers.iter().zip(labels).map(|(a, label)| FooterButton::new(a.role, label)).collect();
        let shortcuts: Vec<Option<Key>> = answers.iter().map(|a| a.key).collect();
        let footer = Footer::custom(move |ui, fx| {
            let row = fx.buttons(ui, buttons);
            for (response, key) in row.responses.iter().zip(&shortcuts) {
                if let Some(key) = key {
                    ui.ctx().accesskit_node_builder(response.id, |node| node.set_keyboard_shortcut(key.name()));
                }
            }
        });
        let mut dialog = Dialog::with_id(id, title).width(Width::Fixed(420.0)).footer(footer).initial_focus(InitialFocus::DefaultButton).answers();
        if let Some(stack) = stack {
            dialog = dialog.stack(stack);
        }
        let shown = dialog.show(ctx, &mut state, &mut (), |ui, ()| {
            ui.label(message);
        });
        let mut answer = match shown.outcome {
            Some(Outcome::Answer(role)) => Some(role),
            Some(Outcome::Cancel | Outcome::Close) => Some(escape),
            _ => None,
        };
        if answer.is_none() && shown.active {
            answer = ctx.input_mut(|i| keys.iter().find(|(_, key)| key.is_some_and(|k| i.consume_key(Modifiers::NONE, k))).map(|(role, _)| *role));
            if answer.is_some() {
                if let Some(focus) = state.return_focus() {
                    ctx.memory_mut(|m| m.request_focus(focus));
                }
                state.close();
            }
        }
        store_state(ctx, id.with("prompt"), state, first);
        answer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answers(items: &[(&str, Option<Key>)]) -> Vec<(String, Option<Key>)> {
        items.iter().map(|(label, key)| ((*label).to_owned(), *key)).collect()
    }

    #[test]
    fn labels_show_the_key_in_one_style_per_prompt() {
        let english = answers(&[("Yes", Some(Key::Y)), ("No", Some(Key::N)), ("Cancel", Some(Key::C))]);
        assert_eq!(accelerator_labels(&english, Platform::Other), ["(Y)es", "(N)o", "(C)ancel"]);
        let german = answers(&[("Ja", Some(Key::Y)), ("Nein", Some(Key::N)), ("Abbrechen", Some(Key::C))]);
        assert_eq!(accelerator_labels(&german, Platform::Other), ["Ja (Y)", "Nein (N)", "Abbrechen (C)"]);
        let save = answers(&[("Don't Save", Some(Key::D)), ("Cancel", Some(Key::C)), ("Save", Some(Key::S))]);
        assert_eq!(accelerator_labels(&save, Platform::Other), ["(D)on't Save", "(C)ancel", "(S)ave"]);
    }

    #[test]
    fn macos_hides_the_letters_and_a_keyless_answer_keeps_its_text() {
        let save = answers(&[("Don't Save", Some(Key::D)), ("Cancel", Some(Key::C))]);
        assert_eq!(accelerator_labels(&save, Platform::Mac), ["Don't Save", "Cancel"]);
        let mixed = answers(&[("Retry", Some(Key::R)), ("Skip", None)]);
        assert_eq!(accelerator_labels(&mixed, Platform::Other), ["(R)etry", "Skip"]);
        let wide = answers(&[("Écraser", Some(Key::E)), ("Annuler", Some(Key::C))]);
        assert_eq!(accelerator_labels(&wide, Platform::Other), ["Écraser (E)", "Annuler (C)"]);
    }

    #[test]
    fn esc_answers_cancel_else_the_first_alternate() {
        use ButtonRole::{Alternate, Cancel, Default};
        assert_eq!(escape_role(&[Default, Alternate, Cancel], None), Cancel);
        assert_eq!(escape_role(&[Default, Alternate], None), Alternate);
        assert_eq!(escape_role(&[Default, Alternate, Cancel], Some(Alternate)), Alternate);
        assert_eq!(escape_role(&[Default, Cancel], Some(Alternate)), Cancel, "a role the prompt lacks is ignored");
        assert_eq!(escape_role(&[Default], None), Default);
    }
}
