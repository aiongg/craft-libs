//! `Alert` (24.4): a notice with one OK answer, a configuration of `Prompt`.

use super::prompt::{Prompt, PromptOutput, Reply, buttons};
use super::words::PromptLabels;
use crate::contract::Bind;
use crate::runtime::env::{Env, Label};
use crate::runtime::windows::os_of;
use crate::sections::DialogRole;
use craft_theme::Tone;
use egui::Ui;

/// A modal notice with one OK button: a prompt whose only answer is OK, which OK, Enter, Escape and the close
/// action all give.
///
/// The open state is the `opened` binding, else the alert's own, which starts open.
#[derive(crate::contract::Pattern, crate::contract::Props)]
pub struct Alert<'a> {
    title: Label,
    message: Label,
    /// The label of the OK button; "OK" in the language of the scope without it.
    #[prop(into)]
    ok_label: Option<Label>,
    tone: Option<(Tone, Label)>,
    details: Option<(Label, Label)>,
    /// Binds the open state to `open`.
    #[prop(name = "opened")]
    open: Option<Bind<'a, bool>>,
}

impl<'a> Alert<'a> {
    /// An alert titled `title` with `message` and one OK button.
    pub fn new(title: impl Into<Label>, message: impl Into<Label>) -> Self {
        Alert { title: title.into(), message: message.into(), ok_label: None, tone: None, details: None, open: None }
    }

    /// A line under the message in the colour of `tone`, with the tone's icon.
    #[must_use]
    pub fn tone(mut self, tone: Tone, text: impl Into<Label>) -> Self {
        self.tone = Some((tone, text.into()));
        self
    }

    /// A collapsible section under the message; `label` names its header ("Details").
    #[must_use]
    pub fn details(mut self, label: impl Into<Label>, text: impl Into<Label>) -> Self {
        self.details = Some((label.into(), text.into()));
        self
    }

    /// Shows the alert; `picked` is `Some(0)` the pass it closes.
    pub fn show(self, ui: &mut Ui) -> PromptOutput {
        let Alert { title, message, ok_label, tone, details, open } = self;
        let ok = ok_label.unwrap_or_else(|| Env::of(ui).words::<PromptLabels>().ok.clone());
        let mut prompt = Prompt::new(title, message).answers(buttons(&[Reply::new(ok, DialogRole::Accept, None)], os_of(ui.ctx())));
        if let Some((tone, text)) = tone {
            prompt = prompt.tone(tone, text);
        }
        if let Some((label, text)) = details {
            prompt = prompt.details(label, text);
        }
        if let Some(open) = open {
            prompt = prompt.opened(open);
        }
        prompt.show(ui)
    }
}
