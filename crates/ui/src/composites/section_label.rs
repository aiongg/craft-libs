//! [`SectionLabel`]: the static caption over a group of controls in a panel.

use egui::{Color32, Label, Response, Ui};

use crate::primitives::Content;
use crate::story::{Args, Level, Story};
use crate::theme::{ToEgui, Tokens, TokensOf, UiTokens, role_format};
use craft_theme::TypeRole;

/// A section label's type role and colour, from [`SectionLabel::style`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SectionLabelStyle {
    /// The text's type role: `label`.
    pub role: TypeRole,
    /// The text: `text.faint`.
    pub color: Color32,
}

/// A static caption that names the group of controls under it (PhotoCraft's `section_label`):
/// `label` text in `text.faint`. It does not collapse and is not a Tab stop; a collapsible section
/// uses [`SectionHeader`](super::SectionHeader).
///
/// ```
/// use craft_ui::composites::SectionLabel;
/// # egui::__run_test_ui(|ui| {
/// SectionLabel::new("Blend mode").show(ui);
/// # });
/// ```
///
/// **AccessKit:** a `Label` with the text.
#[must_use = "show the label with `show`"]
pub struct SectionLabel {
    text: String,
}

impl SectionLabel {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn style(tokens: &UiTokens) -> SectionLabelStyle {
        SectionLabelStyle { role: TypeRole::Label, color: Content::Faint.color(tokens).to_egui() }
    }

    /// Shows the text at the layout's cursor, wrapping at the available width.
    pub fn show(self, ui: &mut Ui) -> Response {
        let tokens = Tokens::of(ui);
        let style = SectionLabel::style(&tokens);
        let format = role_format(&tokens, style.role, style.color);
        ui.add(Label::new(egui::text::LayoutJob::single_section(self.text, format)).selectable(false))
    }
}

/// The components this module has stories for.
pub const COMPONENTS: &[&str] = &["SectionLabel"];

pub(crate) fn stories() -> Vec<Story> {
    let args = || Args::new().with_text("text", "Blend mode");
    vec![Story::new("composites/section-label/default", Level::Composites, "SectionLabel", "Default", args, show_label)]
}

fn show_label(ui: &mut Ui, args: &mut Args) {
    let _ = SectionLabel::new(args.text("text")).show(ui);
}
