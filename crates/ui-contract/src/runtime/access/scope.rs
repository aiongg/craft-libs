use crate::primitives::Id;
use craft_behavior::PressState;
use craft_theme::Flags;
use craft_ui_rt::{BuildCx, Rect, State, Vec2};

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
    /// The control the label names: it shows the label's press and hover as its own (`FieldLabel::names`).
    pub(crate) control: Id,
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
    /// A double click on the label's name or a click on its reset dot asked the control to reset (`ask_reset`).
    pub(crate) reset: Option<ResetAsk>,
    /// The cell the control writes what its label shows of it (`ScopeFacts`).
    pub(crate) facts: Option<State<ScopeFacts>>,
    /// The cell the controls under a container's label write their validity message to (`note_message`); the
    /// container's footer reads it, so a message change reruns the footer alone.
    pub(crate) message: Option<State<Option<FieldMessage>>>,
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
            control: Id::NULL,
            activated: false,
            link: false,
            status: false,
            shown: true,
            claim: None,
            reset: None,
            facts: None,
            message: None,
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

    /// The label names the control `control` and the pointer is over it or presses it.
    pub(crate) fn hover_of(&self, control: Id) -> (bool, bool) {
        let own = self.control == control;
        (own && self.hovered, own && self.pressed())
    }

    /// The label is held down or being dragged.
    pub fn pressed(&self) -> bool {
        self.press.phase.is_press()
    }
}

/// One reset the label asked of its control. The label keeps it in its env until it runs again, so the control tells a
/// reset it has run from one it has not by the serial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResetAsk {
    /// The control that registered the reset (`ResetFact::control`).
    pub control: Id,
    /// Counts the resets asked in the label scope, so each ask differs from the one before.
    pub serial: u64,
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
    /// The child of the control whose stop a click on the label focuses, in place of the control's own
    /// (`PaintUi::label_target`).
    pub target: Option<Id>,
    /// Where the control's first line sits, from its top, as its last paint left it; `None` before its first paint.
    pub line: Option<ControlLine>,
}

/// The line a side label aligns to, measured from the control's top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ControlLine {
    /// The control's first text baseline.
    Baseline(f32),
    /// The centre line of a control that paints no text; the label's text is centred on it.
    Centre(f32),
}

impl ControlLine {
    /// The baseline, from the control's top, a label of size `label` with baseline `label_baseline` lines up with.
    pub(crate) fn baseline(self, label: Vec2, label_baseline: f32) -> f32 {
        match self {
            ControlLine::Baseline(baseline) => baseline,
            ControlLine::Centre(centre) => centre + label_baseline - label.y / 2.0,
        }
    }
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
    if !core.run.measuring {
        note_in(core.env.field_label(), change);
    }
}

fn note_in(label: Option<&FieldLabel>, change: impl FnOnce(&mut ScopeFacts) + 'static) {
    let Some(cell) = label.and_then(|label| label.facts) else { return };
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

/// Names `target`, a child of the control the label `label` names, as the one a click on the label focuses.
pub(crate) fn note_label_target(label: Option<&FieldLabel>, target: Id) {
    note_in(label, move |facts| facts.target = Some(target));
}

/// The running control's first line, from its top, as its paint left it: the first text baseline, or the centre of a
/// control that painted none. Written after the paint, so the label places itself by it from the next run on; the
/// facts cell changes only when the line moves.
pub(crate) fn note_line(core: &crate::contract::CxCore<'_>) {
    let line = match core.paint.borrow().baseline {
        Some(y) => ControlLine::Baseline(y - core.rect.min.y),
        None => ControlLine::Centre(core.rect.height() / 2.0),
    };
    note(core, move |facts| facts.line = Some(line));
}

/// The validity message of the first control under a container's label that reported one, and the tone it shows in.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FieldMessage {
    /// The control that reported it; it alone replaces or clears it.
    pub control: Id,
    pub message: crate::runtime::env::Message,
    pub tone: craft_theme::variant::Tone,
}

/// The message cell of the container `container`'s label scope, kept by the container's node.
pub(crate) fn message_cell(cx: &mut BuildCx<'_>, container: Id) -> State<Option<FieldMessage>> {
    cx.slot(("label_message", container.value()), || None)
}

/// Writes the running control's validity message (`validity_of`) into the message cell of the container label above
/// it when its build ends, only when it changes the cell: the first control that reports a message holds the cell
/// until it reports none. Returns the message as text when no container shows it as its status line
/// (`FooterRegion::status`): the control shows it itself, in its tooltip.
pub(crate) fn note_message(core: &crate::contract::CxCore<'_>, validity: Option<(&crate::runtime::env::Message, bool)>) -> Option<String> {
    let label = core.env.field_label();
    let cell = label.and_then(|label| label.message);
    if !core.run.measuring
        && let Some(cell) = cell
    {
        let id = core.id;
        let mine = validity.map(|(message, warns)| FieldMessage { control: id, message: message.clone(), tone: message.effective_tone(warns) });
        crate::contract::context::log_build_read(move |cx| {
            use craft_ui_rt::Writes as _;
            let held = cell.with(cx.store_mut(), |held| held.as_ref().map(|held| held.control));
            match (held, mine) {
                (Some(holder), _) if holder != id => {}
                (_, mine) => cell.set_if_changed(cx, mine),
            }
        });
    }
    let shown_by_scope = cell.is_some() && label.is_some_and(|label| label.status);
    let (message, _) = validity.filter(|_| !shown_by_scope)?;
    Some(core.env.resolve(&message.text).as_str().to_owned())
}

/// How many resets the label scope `scope` has asked; kept for the session, so a label that shows again carries on
/// counting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AskedResets(u64);

/// The last reset ask a control ran.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RanReset(u64);

/// A reset the label of scope `scope` asks of `control`, a click on its dot or a double click on its name.
pub(crate) fn ask_reset(ctx: &crate::contract::Context, scope: Id, control: Id) -> ResetAsk {
    let serial = crate::runtime::memory::with_session::<AskedResets, _>(ctx, crate::runtime::memory::sub(scope, "resets"), |asked| {
        asked.0 += 1;
        asked.0
    });
    ResetAsk { control, serial }
}

/// Queues the whole-binding `Intent::Reset` on the running control when its label asked it a reset the control has not
/// run yet, once per ask: a run for another reason finds the same ask in the env and queues nothing.
pub(crate) fn push_label_reset(core: &mut crate::contract::CxCore<'_>) {
    if core.run.measuring {
        return;
    }
    let Some(ask) = core.env.field_label().and_then(|label| label.reset).filter(|ask| ask.control == core.id) else { return };
    let key = crate::runtime::memory::sub(core.id, "reset-ran");
    let fresh =
        crate::runtime::memory::with_session::<RanReset, _>(&core.run.context, key, |ran| std::mem::replace(&mut ran.0, ask.serial) != ask.serial);
    if fresh {
        core.intents.push_whole_reset();
    }
}
