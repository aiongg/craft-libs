//! `OpeningCard`: the card shown in the document area while a file opens.

use crate::contract::Pattern;
use crate::runtime::env::Label;
use egui::{Response, Ui};

/// A centred card: "Opening {name}…" elided on one line with the full text on hover, the muted message, a progress bar
/// (indeterminate without a fraction), the percentage at the start and Cancel at the end of the last row. Job UI
/// only: the fraction and the cancel answer come from the app's engine job.
#[must_use = "show the card with `show`"]
#[derive(Pattern)]
pub struct OpeningCard {
    #[allow(dead_code)] // P3(top-w5): tp.T31
    name: Label,
    #[allow(dead_code)] // P3(top-w5): tp.T31
    message: Option<Label>,
    #[allow(dead_code)] // P3(top-w5): tp.T31
    fraction: Option<f32>,
    #[allow(dead_code)] // P3(top-w5): tp.T31
    cancel: Option<Label>,
}

/// What an [`OpeningCard`] reports.
#[derive(Debug)]
pub struct OpeningCardOutput {
    /// The card's response.
    pub response: Response,
    /// Cancel was picked in this pass.
    pub cancelled: bool,
}

impl OpeningCard {
    /// A card for the file `name`.
    pub fn new(name: impl Into<Label>) -> Self {
        OpeningCard { name: name.into(), message: None, fraction: None, cancel: None }
    }

    /// The muted message under the title.
    #[must_use]
    pub fn message(mut self, message: impl Into<Label>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// How far the job is, 0 to 1; `None` shows an indeterminate bar.
    #[must_use]
    pub fn fraction(mut self, fraction: Option<f32>) -> Self {
        self.fraction = fraction;
        self
    }

    /// The Cancel button's text; without it the card has no Cancel.
    #[must_use]
    pub fn cancel(mut self, cancel: impl Into<Label>) -> Self {
        self.cancel = Some(cancel.into());
        self
    }

    /// Shows the card centred in `ui`.
    pub fn show(self, ui: &mut Ui) -> OpeningCardOutput {
        let _ = ui;
        todo!("tp.T31")
    }
}
