use crate::primitives::Id;
use craft_behavior::PressState;
use craft_theme::Flags;
use craft_ui_rt::Rect;

/// Whether a field label names its control or describes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, crate::contract::MemoProps)]
#[non_exhaustive]
pub enum FieldLabelRole {
    /// The label is the control's name.
    #[default]
    Names,
    /// The control keeps its own name and the label describes it.
    Describes,
}

/// The label a container shows beside its body.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FieldLabel {
    /// The label's node id.
    pub id: Id,
    /// The label's rect.
    pub rect: Rect,
    /// The label's press this pass, for a control that scrubs by its label.
    pub press: PressState,
    /// The pointer is over the label this pass.
    pub hovered: bool,
    /// The label's text.
    pub text: String,
    /// Whether it names or describes the controls.
    pub role: FieldLabelRole,
    /// A line that joins the controls' description, not their name.
    pub detail: Option<String>,
    /// The label was clicked or activated this pass: focus goes to its control.
    pub activated: bool,
    /// The label is a link and was clicked this pass: the control's host is told (`Cx::link_activated`), focus stays.
    pub link: bool,
    /// The container shows its control's validity message as its status line (`FooterRegion::status`).
    pub status: bool,
    /// The label shows as a node of its own. A hidden label and one drawn inside its control have none: they name
    /// their control by their text alone.
    pub shown: bool,
}

impl FieldLabel {
    /// A label of `text` at `rect`, the node `id`.
    pub fn new(id: Id, rect: Rect, text: impl Into<String>) -> Self {
        FieldLabel {
            id,
            rect,
            press: PressState::IDLE,
            hovered: false,
            text: text.into(),
            role: FieldLabelRole::Names,
            detail: None,
            activated: false,
            link: false,
            status: false,
            shown: true,
        }
    }

    /// Whether the label shows as a node of its own.
    #[must_use]
    pub fn shown(mut self, shown: bool) -> Self {
        self.shown = shown;
        self
    }

    /// Whether the label names or describes the controls.
    #[must_use]
    pub fn role(mut self, role: FieldLabelRole) -> Self {
        self.role = role;
        self
    }

    /// Adds a detail line to the description of the controls the label covers.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Whether the container shows its control's validity message as its status line.
    #[must_use]
    pub fn status(mut self, status: bool) -> Self {
        self.status = status;
        self
    }

    /// Whether the label was clicked or activated this pass.
    #[must_use]
    pub fn activated(mut self, activated: bool) -> Self {
        self.activated = activated;
        self
    }

    /// Whether the pointer is over the label this pass.
    #[must_use]
    pub fn with_hover(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    /// The label is held down or being dragged.
    pub fn pressed(&self) -> bool {
        self.press.phase.is_press()
    }
}

/// What a merged container adds to its child's node.
pub(crate) struct Merged {
    pub name: String,
    pub description: Option<String>,
    pub flags: Flags,
    pub enabled: bool,
}
