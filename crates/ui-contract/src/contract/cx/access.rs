//! The `Accessible` trait: live announcements and the enclosing field label.

use super::context::{Cx, CxCore};
use crate::contract::Component;
use crate::runtime::access::{self, FieldLabel, Politeness};

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
        access::scope::field_label(self.context())
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
