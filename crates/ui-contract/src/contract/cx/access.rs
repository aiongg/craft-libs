//! The `Accessible` trait: live announcements and the enclosing field label.

use super::context::{Cx, CxCore};
use crate::contract::Component;
use crate::runtime::access::scope::FieldLabelRole;
use crate::runtime::access::{FieldLabel, Politeness};

/// Live announcements and the enclosing field label (chapter 12).
pub trait Accessible {
    /// Announces transient text: the node's access entry carries it as a live status after the run.
    fn announce(&mut self, text: impl Into<String>, politeness: Politeness);

    /// The label of the enclosing field container, when the component sits in one.
    fn field_label(&self) -> Option<FieldLabel>;
}

impl Accessible for CxCore<'_> {
    fn announce(&mut self, text: impl Into<String>, politeness: Politeness) {
        self.run.announcements.push((text.into(), politeness));
    }

    fn field_label(&self) -> Option<FieldLabel> {
        self.env.field_label().cloned()
    }
}

/// Names or describes the control `core` runs by the side label of its label scope: the label's node when it shows
/// one, else its text. The driver applies it to the access node of each run, after the component's `access` hook, so
/// a control with a name of its own keeps it where the label only describes.
pub(crate) fn apply_field_label(core: &CxCore<'_>, node: &mut crate::runtime::access::Access) {
    let Some(label) = core.env.field_label() else { return };
    match label.role {
        FieldLabelRole::Names if label.shown => {
            node.labelled_by(label.id);
        }
        FieldLabelRole::Names => {
            node.name(crate::runtime::access::Name::Text(label.text.clone().into()));
        }
        FieldLabelRole::Describes if label.shown => {
            node.described_by(label.id);
        }
        FieldLabelRole::Describes => {
            node.description(label.text.clone());
        }
    }
    if let Some(detail) = &label.detail {
        node.description(detail.clone());
    }
}

impl<C: Component> Accessible for Cx<'_, C> {
    fn announce(&mut self, text: impl Into<String>, politeness: Politeness) {
        self.core.announce(text, politeness);
    }

    fn field_label(&self) -> Option<FieldLabel> {
        self.core.field_label()
    }
}
