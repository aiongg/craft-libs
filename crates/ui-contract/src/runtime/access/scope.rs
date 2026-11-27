use crate::primitives::Id;
use craft_behavior::PressState;
use craft_theme::Flags;
use craft_ui_rt::{BuildCx, Rect, State};

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
    /// The rect the label offers its control as part of the control's hit target: the label (without its reset dot)
    /// of a leaf label step whose child draws only its mark (`Component::LABELLED_AFTER`).
    pub(crate) claim: Option<Rect>,
    /// A double click on the label's name or a click on its reset dot asked the control to reset this pass.
    pub(crate) reset: bool,
    /// The cell the control writes what its label shows of it (`ScopeFacts`).
    pub(crate) facts: Option<State<ScopeFacts>>,
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
            claim: None,
            reset: false,
            facts: None,
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

    /// Whether a press on the label follows its link (`LabelPress::Link`) this pass.
    #[must_use]
    pub fn linked(mut self, link: bool) -> Self {
        self.link = link;
        self
    }

    /// The label's press this pass, which its control shows as its own.
    #[must_use]
    pub fn with_press(mut self, press: PressState) -> Self {
        self.press = press;
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

/// What a control tells the label that names it: its reset (10.2) and whether it scrubs by the label. The control's run
/// writes it, through the cell its `FieldLabel` carries, when its build ends; the label, shown before the control,
/// reads the cell tracked, so it shows again in the same frame when a fact changes and never when none does.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ScopeFacts {
    pub reset: Option<ResetFact>,
    pub scrub: bool,
}

/// What a control tells its label scope about its reset (10.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResetFact {
    /// The control that takes `Intent::Reset`.
    pub control: Id,
    /// The value differs from the reset value.
    pub differs: bool,
    /// The reset value as the control words it (`Component::word_value`); `None` names no default.
    pub default: Option<String>,
}

/// The facts cell of the label scope `scope`, kept by the node whose build shows the label, and its facts, read
/// tracked for that node.
pub(crate) fn scope_facts(cx: &mut BuildCx<'_>, scope: Id) -> (State<ScopeFacts>, ScopeFacts) {
    let cell = cx.slot(("label_scope", scope.value()), ScopeFacts::default);
    (cell, cell.get(cx))
}

/// Writes `change` into the facts of the label scope that names the running control, when its build ends and only
/// when it changes them. A control no label names writes nothing.
fn note(core: &crate::contract::CxCore<'_>, change: impl FnOnce(&mut ScopeFacts) + 'static) {
    let Some(cell) = core.env.field_label().and_then(|label| label.facts) else { return };
    if core.run.measuring {
        return;
    }
    crate::contract::context::log_build_read(move |cx| {
        use craft_ui_rt::Writes as _;
        let mut facts = cell.with(cx.store_mut(), ScopeFacts::clone);
        change(&mut facts);
        cell.set_if_changed(cx, facts);
    });
}

/// The running control's reset, for the label that names it: `None` when it has no reset value.
pub(crate) fn note_reset(core: &crate::contract::CxCore<'_>, reset: Option<ResetFact>) {
    note(core, move |facts| facts.reset = reset);
}

/// The running control scrubs by the label that names it (`Component::scrub` with a label source): the label senses
/// the drag from its next run on.
pub(crate) fn note_scrub(core: &crate::contract::CxCore<'_>) {
    note(core, |facts| facts.scrub = true);
}
